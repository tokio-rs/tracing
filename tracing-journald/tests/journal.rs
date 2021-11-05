#![cfg(target_os = "linux")]

use std::collections::HashMap;
use std::process::Command;

use serde::Deserialize;
use tracing::{debug, error, info, warn};
use tracing_journald::Subscriber;
use tracing_subscriber::subscribe::CollectExt;
use tracing_subscriber::Registry;

fn journalctl_version() -> std::io::Result<String> {
    let output = Command::new("journalctl").arg("--version").output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn with_journald(f: impl FnOnce()) {
    match journalctl_version() {
        Ok(_) => {
            let sub = Registry::default().with(Subscriber::new().unwrap().with_field_prefix(None));
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
    Binary(Vec<u8>),
}

// Convenience impls to compare fields against strings and bytes with assert_eq!
impl PartialEq<&str> for Field {
    fn eq(&self, other: &&str) -> bool {
        match self {
            Field::Text(s) => s == other,
            Field::Binary(_) => false,
        }
    }
}

impl PartialEq<[u8]> for Field {
    fn eq(&self, other: &[u8]) -> bool {
        match self {
            Field::Text(s) => s.as_bytes() == other,
            Field::Binary(data) => data == other,
        }
    }
}

/// Read from journal with `journalctl`.
///
/// `test_name` is the randomized name of the test being run, and gets
/// added as `TEST_NAME` match to the `journalctl` call, to make sure to
/// only select journal entries originating from and relevant to the
/// current test.
fn read_from_journal(test_name: &str) -> Vec<HashMap<String, Field>> {
    // Retry the `journalctl` command up to 5 times, in case there's some
    // timing-related flakiness.
    for retry in (0..5).rev() {
        eprintln!("running `journalctl` ({} tries remaining)...", retry);

        let stdout = String::from_utf8(
            Command::new("journalctl")
                .args(&["--user", "--output=json"])
                // Filter by the PID of the current test process
                .arg(format!("_PID={}", std::process::id()))
                // tracing-journald logs strings in their debug representation
                .arg(format!("TEST_NAME={:?}", test_name))
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap();

        let fields: Vec<HashMap<_, _>> = stdout
            .lines()
            .map(|l| {
                dbg!(l);
                serde_json::from_str(l).unwrap()
            })
            .collect();

        if !fields.is_empty() {
            return fields;
        }
    }

    Vec::new()
}

#[test]
fn simple_message() {
    with_journald(|| {
        info!(test.name = "simple_message", "Hello World");

        let messages = read_from_journal("simple_message");
        assert_eq!(messages.len(), 1);

        let message = &messages[0];
        assert_eq!(message["MESSAGE"], "Hello World");
        assert_eq!(message["PRIORITY"], "5");
    });
}

#[test]
fn multiline_message() {
    with_journald(|| {
        warn!(test.name = "multiline_message", "Hello\nMultiline\nWorld");

        let messages = read_from_journal("multiline_message");
        assert_eq!(messages.len(), 1);

        let message = &messages[0];
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

        let messages = read_from_journal("multiline_message_trailing_newline");
        assert_eq!(messages.len(), 1);

        let message = &messages[0];
        assert_eq!(message["MESSAGE"], "A trailing newline\n");
        assert_eq!(message["PRIORITY"], "3");
    });
}

#[test]
fn internal_null_byte() {
    with_journald(|| {
        debug!(test.name = "internal_null_byte", "An internal\x00byte");

        let messages = read_from_journal("internal_null_byte");
        assert_eq!(messages.len(), 1);

        let message = &messages[0];
        assert_eq!(message["MESSAGE"], b"An internal\x00byte"[..]);
        assert_eq!(message["PRIORITY"], "6");
    });
}
