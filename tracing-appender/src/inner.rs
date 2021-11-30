use crate::sync::{RwLock, RwLockReadGuard};
use std::{
    fmt::Debug,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::rolling::Rotation;
use time::OffsetDateTime;

#[derive(Debug)]
pub(crate) struct InnerAppender {
    state: State,
    writer: RwLock<File>,
}

#[derive(Debug)]
struct State {
    log_directory: String,
    log_filename_prefix: String,
    rotation: Rotation,
    next_date: AtomicUsize,
}

#[derive(Debug)]
pub struct RollingWriter<'a>(RwLockReadGuard<'a, File>);

impl io::Write for InnerAppender {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let now = OffsetDateTime::now_utc();
        let writer = self.writer.get_mut();
        if self.state.should_rollover(now) {
            let _did_cas = self.state.advance_date(now);
            debug_assert!(_did_cas, "if we have &mut access to the appender, no other thread can have advanced the timestamp...");
            self.state.refresh_writer(now, writer);
        }
        writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.get_mut().flush()
    }
}

impl io::Write for RollingWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (&*self.0).write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        (&*self.0).flush()
    }
}

impl InnerAppender {
    pub(crate) fn new(
        log_directory: &Path,
        log_filename_prefix: &Path,
        rotation: Rotation,
        now: OffsetDateTime,
    ) -> io::Result<Self> {
        let log_directory = log_directory.to_str().unwrap();
        let log_filename_prefix = log_filename_prefix.to_str().unwrap();

        let filename = rotation.join_date(log_filename_prefix, &now);
        let next_date = rotation.next_date(&now);

        Ok(InnerAppender {
            state: State {
                log_directory: log_directory.to_string(),
                log_filename_prefix: log_filename_prefix.to_string(),
                next_date: AtomicUsize::new(
                    next_date
                        .map(|date| date.unix_timestamp() as usize)
                        .unwrap_or(0),
                ),
                rotation,
            },
            writer: RwLock::new(create_writer(log_directory, &filename)?),
        })
    }

    pub(crate) fn make_writer(&self) -> RollingWriter<'_> {
        let now = OffsetDateTime::now_utc();

        // Should we try to roll over the log file?
        if self.state.should_rollover(now) {
            // Did we get the right to lock the file? If not, another thread
            // did it and we can just make a writer.
            if self.state.advance_date(now) {
                self.state.refresh_writer(now, &mut *self.writer.write());
            }
        }
        RollingWriter(self.writer.read())
    }
}

impl State {
    fn refresh_writer(&self, now: OffsetDateTime, file: &mut File) {
        debug_assert!(self.should_rollover(now));

        let filename = self.rotation.join_date(&self.log_filename_prefix, &now);

        match create_writer(&self.log_directory, &filename) {
            Ok(new_file) => {
                if let Err(err) = file.flush() {
                    eprintln!("Couldn't flush previous writer: {}", err);
                }
                *file = new_file;
            }
            Err(err) => eprintln!("Couldn't create writer for logs: {}", err),
        }
    }

    fn should_rollover(&self, date: OffsetDateTime) -> bool {
        // the `None` case means that the `InnerAppender` *never* rotates log files.
        let next_date = self.next_date.load(Ordering::Acquire);
        if next_date == 0 {
            return false;
        }
        date.unix_timestamp() as usize >= next_date
    }

    fn advance_date(&self, now: OffsetDateTime) -> bool {
        let next_date = self
            .rotation
            .next_date(&now)
            .map(|date| date.unix_timestamp() as usize)
            .unwrap_or(0);
        self.next_date
            .compare_exchange(
                now.unix_timestamp() as usize,
                next_date,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }
}

fn create_writer(directory: &str, filename: &str) -> io::Result<File> {
    let path = Path::new(directory).join(filename);
    let mut open_options = OpenOptions::new();
    open_options.append(true).create(true);

    let new_file = open_options.open(path.as_path());
    if new_file.is_err() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            return open_options.open(path);
        }
    }

    new_file
}
