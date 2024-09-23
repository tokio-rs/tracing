#![cfg(target_os = "linux")]

use std::collections::HashMap;
use std::process::Command;
use std::time::Duration;

use serde::Deserialize;
use tracing::{debug, error, info, info_span, trace, warn};
use tracing_journald::{Priority, PriorityMappings, Subscriber};
use tracing_subscriber::subscribe::CollectExt;
use tracing_subscriber::Registry;

fn journalctl_version() -> std::io::Result<String> {
    let output = Command::new("journalctl").arg("--version").output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn with_journald(f: impl FnOnce()) {
    with_journald_subscriber(
        Subscriber::new()
            .unwrap()
            .with_field_prefix(None)
            .with_priority_mappings(PriorityMappings {
                trace: Priority::Informational,
                ..PriorityMappings::new()
            }),
        f,
    )
}

fn with_journald_subscriber(subscriber: Subscriber, f: impl FnOnce()) {
    match journalctl_version() {
        Ok(_) => {
            let sub = Registry::default().with(subscriber);
            tracing::collect::with_default(sub, f);
        }
        Err(error) => eprintln!(
            "SKIPPING TEST: journalctl --version failed with error: {}",
            error
        ),
    }
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(untagged)]
enum Field {
    Text(String),
    Array(Vec<String>),
    Binary(Vec<u8>),
}

impl Field {
    fn as_array(&self) -> Option<&[String]> {
        match self {
            Field::Text(_) => None,
            Field::Binary(_) => None,
            Field::Array(v) => Some(v),
        }
    }

    fn as_text(&self) -> Option<&str> {
        match self {
            Field::Text(v) => Some(v.as_str()),
            Field::Binary(_) => None,
            Field::Array(_) => None,
        }
    }
}

// Convenience impls to compare fields against strings and bytes with assert_eq!
impl PartialEq<&str> for Field {
    fn eq(&self, other: &&str) -> bool {
        match self {
            Field::Text(s) => s == other,
            Field::Binary(_) => false,
            Field::Array(_) => false,
        }
    }
}

impl PartialEq<[u8]> for Field {
    fn eq(&self, other: &[u8]) -> bool {
        match self {
            Field::Text(s) => s.as_bytes() == other,
            Field::Binary(data) => data == other,
            Field::Array(_) => false,
        }
    }
}

impl PartialEq<Vec<&str>> for Field {
    fn eq(&self, other: &Vec<&str>) -> bool {
        match self {
            Field::Text(_) => false,
            Field::Binary(_) => false,
            Field::Array(data) => data == other,
        }
    }
}

/// Retry `f` 30 times 100ms apart, i.e. a total of three seconds.
///
/// When `f` returns an error wait 100ms and try it again, up to ten times.
/// If the last attempt failed return the error returned by that attempt.
///
/// If `f` returns Ok immediately return the result.
fn retry<T, E>(f: impl Fn() -> Result<T, E>) -> Result<T, E> {
    let attempts = 30;
    let interval = Duration::from_millis(100);
    for attempt in (0..attempts).rev() {
        match f() {
            Ok(result) => return Ok(result),
            Err(e) if attempt == 0 => return Err(e),
            Err(_) => std::thread::sleep(interval),
        }
    }
    unreachable!()
}

/// Read from journal with `journalctl`.
///
/// `test_name` is a string to match in the `TEST_NAME` field
/// of the `journalctl` call, to make sure to only select journal entries
/// originating from and relevant to the current test.
///
/// Additionally filter by the `_PID` field with the PID of this
/// test process, to make sure this method only reads journal entries
/// created by this test process.
fn read_from_journal(test_name: &str) -> Vec<HashMap<String, Field>> {
    let stdout = String::from_utf8(
        Command::new("journalctl")
            // We pass --all to circumvent journalctl's default limit of 4096 bytes for field values
            .args(["--user", "--output=json", "--all"])
            // Filter by the PID of the current test process
            .arg(format!("_PID={}", std::process::id()))
            .arg(format!("TEST_NAME={}", test_name))
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();

    stdout
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

/// Read exactly one line from journal for the given test name.
///
/// Try to read lines for `testname` from journal, and `retry()` if the wasn't
/// _exactly_ one matching line.
fn retry_read_one_line_from_journal(testname: &str) -> HashMap<String, Field> {
    retry(|| {
        let mut messages = read_from_journal(testname);
        if messages.len() == 1 {
            Ok(messages.pop().unwrap())
        } else {
            Err(format!(
                "one messages expected, got {} messages",
                messages.len()
            ))
        }
    })
    .unwrap()
}

#[test]
fn simple_message() {
    with_journald(|| {
        info!(test.name = "simple_message", "Hello World");

        let message = retry_read_one_line_from_journal("simple_message");
        assert_eq!(message["MESSAGE"], "Hello World");
        assert_eq!(message["PRIORITY"], "5");
    });
}

#[test]
fn custom_priorities() {
    fn check_message(level: &str, priority: &str) {
        let entry = retry_read_one_line_from_journal(&format!("custom_priority.{}", level));
        assert_eq!(entry["MESSAGE"], format!("hello {}", level).as_str());
        assert_eq!(entry["PRIORITY"], priority);
    }

    let priorities = PriorityMappings {
        error: Priority::Critical,
        warn: Priority::Error,
        info: Priority::Warning,
        debug: Priority::Notice,
        trace: Priority::Informational,
    };
    let subscriber = Subscriber::new()
        .unwrap()
        .with_field_prefix(None)
        .with_priority_mappings(priorities);
    let test = || {
        trace!(test.name = "custom_priority.trace", "hello trace");
        check_message("trace", "6");
        debug!(test.name = "custom_priority.debug", "hello debug");
        check_message("debug", "5");
        info!(test.name = "custom_priority.info", "hello info");
        check_message("info", "4");
        warn!(test.name = "custom_priority.warn", "hello warn");
        check_message("warn", "3");
        error!(test.name = "custom_priority.error", "hello error");
        check_message("error", "2");
    };

    with_journald_subscriber(subscriber, test);
}

#[test]
fn multiline_message() {
    with_journald(|| {
        warn!(test.name = "multiline_message", "Hello\nMultiline\nWorld");

        let message = retry_read_one_line_from_journal("multiline_message");
        assert_eq!(message["MESSAGE"], "Hello\nMultiline\nWorld");
        assert_eq!(message["PRIORITY"], "4");
    });
}

#[test]
fn multiline_message_trailing_newline() {
    with_journald(|| {
        error!(
            test.name = "multiline_message_trailing_newline",
            "A trailing newline\n"
        );

        let message = retry_read_one_line_from_journal("multiline_message_trailing_newline");
        assert_eq!(message["MESSAGE"], "A trailing newline\n");
        assert_eq!(message["PRIORITY"], "3");
    });
}

#[test]
fn internal_null_byte() {
    with_journald(|| {
        debug!(test.name = "internal_null_byte", "An internal\x00byte");

        let message = retry_read_one_line_from_journal("internal_null_byte");
        assert_eq!(message["MESSAGE"], b"An internal\x00byte"[..]);
        assert_eq!(message["PRIORITY"], "6");
    });
}

#[test]
fn large_message() {
    let large_string = "b".repeat(512_000);
    with_journald(|| {
        debug!(test.name = "large_message", "Message: {}", large_string);

        let message = retry_read_one_line_from_journal("large_message");
        assert_eq!(
            message["MESSAGE"],
            format!("Message: {}", large_string).as_str()
        );
        assert_eq!(message["PRIORITY"], "6");
    });
}

#[test]
fn simple_metadata() {
    let sub = Subscriber::new()
        .unwrap()
        .with_field_prefix(None)
        .with_syslog_identifier("test_ident".to_string());
    with_journald_subscriber(sub, || {
        info!(test.name = "simple_metadata", "Hello World");

        let message = retry_read_one_line_from_journal("simple_metadata");
        assert_eq!(message["MESSAGE"], "Hello World");
        assert_eq!(message["PRIORITY"], "5");
        assert_eq!(message["TARGET"], "journal");
        assert_eq!(message["SYSLOG_IDENTIFIER"], "test_ident");
        assert!(message["CODE_FILE"].as_text().is_some());
        assert!(message["CODE_LINE"].as_text().is_some());
    });
}

#[test]
fn journal_fields() {
    let sub = Subscriber::new()
        .unwrap()
        .with_field_prefix(None)
        .with_custom_fields([("SYSLOG_FACILITY", "17")])
        .with_custom_fields([("ABC", "dEf"), ("XYZ", "123")]);
    with_journald_subscriber(sub, || {
        info!(test.name = "journal_fields", "Hello World");

        let message = retry_read_one_line_from_journal("journal_fields");
        assert_eq!(message["MESSAGE"], "Hello World");
        assert_eq!(message["PRIORITY"], "5");
        assert_eq!(message["TARGET"], "journal");
        assert_eq!(message["SYSLOG_FACILITY"], "17");
        assert_eq!(message["ABC"], "dEf");
        assert_eq!(message["XYZ"], "123");
        assert!(message["CODE_FILE"].as_text().is_some());
        assert!(message["CODE_LINE"].as_text().is_some());
    });
}

#[test]
fn span_metadata() {
    with_journald(|| {
        let s1 = info_span!("span1", span_field1 = "foo1");
        let _g1 = s1.enter();

        info!(test.name = "span_metadata", "Hello World");

        let message = retry_read_one_line_from_journal("span_metadata");
        assert_eq!(message["MESSAGE"], "Hello World");
        assert_eq!(message["PRIORITY"], "5");
        assert_eq!(message["TARGET"], "journal");

        assert_eq!(message["SPAN_FIELD1"].as_text(), Some("foo1"));
        assert_eq!(message["SPAN_NAME"].as_text(), Some("span1"));

        assert!(message["CODE_FILE"].as_text().is_some());
        assert!(message["CODE_LINE"].as_text().is_some());

        assert!(message["SPAN_CODE_FILE"].as_text().is_some());
        assert!(message["SPAN_CODE_LINE"].as_text().is_some());
    });
}

#[test]
fn multiple_spans_metadata() {
    with_journald(|| {
        let s1 = info_span!("span1", span_field1 = "foo1");
        let _g1 = s1.enter();
        let s2 = info_span!("span2", span_field1 = "foo2");
        let _g2 = s2.enter();

        info!(test.name = "multiple_spans_metadata", "Hello World");

        let message = retry_read_one_line_from_journal("multiple_spans_metadata");
        assert_eq!(message["MESSAGE"], "Hello World");
        assert_eq!(message["PRIORITY"], "5");
        assert_eq!(message["TARGET"], "journal");

        assert_eq!(message["SPAN_FIELD1"], vec!["foo1", "foo2"]);
        assert_eq!(message["SPAN_NAME"], vec!["span1", "span2"]);

        assert!(message["CODE_FILE"].as_text().is_some());
        assert!(message["CODE_LINE"].as_text().is_some());

        assert!(message.contains_key("SPAN_CODE_FILE"));
        assert_eq!(message["SPAN_CODE_LINE"].as_array().unwrap().len(), 2);
    });
}

#[test]
fn spans_field_collision() {
    with_journald(|| {
        let s1 = info_span!("span1", span_field = "foo1");
        let _g1 = s1.enter();
        let s2 = info_span!("span2", span_field = "foo2");
        let _g2 = s2.enter();

        info!(
            test.name = "spans_field_collision",
            span_field = "foo3",
            "Hello World"
        );

        let message = retry_read_one_line_from_journal("spans_field_collision");
        assert_eq!(message["MESSAGE"], "Hello World");
        assert_eq!(message["SPAN_NAME"], vec!["span1", "span2"]);

        assert_eq!(message["SPAN_FIELD"], vec!["foo1", "foo2", "foo3"]);
    });
}
