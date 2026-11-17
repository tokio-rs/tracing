// Creating symbolic links on Windows may require elevated privileges.
#![cfg(unix)]

use std::{fs, io::Write, path::Path};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

fn assert_latest_points_to_log(directory: &Path, filename: &str, latest: &str) {
    let mut appender = RollingFileAppender::builder()
        .rotation(Rotation::NEVER)
        .filename_prefix(filename)
        .latest_symlink(latest)
        .build(directory)
        .expect("failed to create appender");
    appender.write_all(b"log entry\n").unwrap();
    appender.flush().unwrap();

    let latest = directory.join(latest);
    assert_eq!(fs::read_to_string(&latest).unwrap(), "log entry\n");
    assert_eq!(
        fs::canonicalize(latest).unwrap(),
        fs::canonicalize(directory.join(filename)).unwrap()
    );
}

#[test]
fn latest_symlink_with_relative_directory() {
    let directory = tempfile::tempdir_in(".").unwrap();
    let relative = Path::new(directory.path().file_name().unwrap());

    assert_latest_points_to_log(relative, "first.log", "latest.log");
    assert_latest_points_to_log(relative, "second.log", "latest.log");
}

#[test]
fn latest_symlink_with_relative_directory_and_nested_paths() {
    let directory = tempfile::tempdir_in(".").unwrap();
    let relative = Path::new(directory.path().file_name().unwrap());
    fs::create_dir(relative.join("links")).unwrap();

    assert_latest_points_to_log(relative, "nested/app.log", "links/latest.log");
}

#[test]
fn latest_symlink_with_absolute_directory() {
    let directory = tempfile::tempdir().unwrap();

    assert_latest_points_to_log(directory.path(), "app.log", "latest.log");
}
