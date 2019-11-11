use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::info;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{fmt::format::Format, fmt::time::ChronoUtc, fmt::Subscriber};

struct RollingFileAppender<F> {
    file: Arc<Mutex<File>>,
    should_rotate: Arc<F>,
}

impl<F> MakeWriter for RollingFileAppender<F> {
    type Writer = Self;
    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}

impl<F> RollingFileAppender<F>
where
    F: for<'a> Fn(&'a Path) -> bool,
{
    fn try_new(should_rotate: F) -> Result<Self, io::Error> {
        let file = File::create("foo.txt")?;
        let appender = RollingFileAppender {
            file: Arc::new(Mutex::new(file)),
            should_rotate: Arc::new(should_rotate),
        };
        Ok(appender)
    }
}

impl<F> Write for RollingFileAppender<F> {
    fn write(&mut self, bytes: &[u8]) -> Result<usize, io::Error> {
        let mut file = self.file.lock().expect("Mutex poisoned");
        file.write(bytes)
    }
    fn flush(&mut self) -> Result<(), io::Error> {
        let mut file = self.file.lock().expect("Mutex poisoned");
        file.flush()
    }
}

impl<F> Clone for RollingFileAppender<F> {
    fn clone(&self) -> Self {
        Self {
            file: self.file.clone(),
            should_rotate: self.should_rotate.clone(),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let appender = RollingFileAppender::try_new(|_| false)?;

    let format = Format::default()
        .with_timer(ChronoUtc::rfc3339())
        .with_ansi(false)
        .with_target(false)
        .json();
    let subscriber = Subscriber::builder()
        .with_writer(appender)
        .on_event(format)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");

    info!("preparing to shave {} yaks", 3);

    Ok(())
}
