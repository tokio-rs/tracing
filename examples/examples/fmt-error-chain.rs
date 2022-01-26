//! This example demonstrates using the `tracing-error` crate's `SpanTrace` type
//! to attach a trace context to a custom error type.
#![deny(rust_2018_idioms)]
#![feature(provide_any)]
use std::any::Requisition;
use std::fmt;
use std::{error::Error, path::Path};
use tracing::{error, info};
use tracing_error::{ErrorSubscriber, SpanTrace};
use tracing_subscriber::prelude::*;

#[derive(Debug)]
struct FileError {
    context: SpanTrace,
}

impl FileError {
    fn new() -> Self {
        Self {
            context: SpanTrace::capture(),
        }
    }
}

impl Error for FileError {
    fn provide<'a>(&'a self, mut req: Requisition<'a, '_>) {
        req.provide_ref(&self.context)
           .provide_ref::<tracing::Span>(self.context.as_ref());
    }
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("file does not exist")
    }
}

#[tracing::instrument]
fn read_file(path: &Path) -> Result<String, FileError> {
    Err(FileError::new())
}

#[derive(Debug)]
struct ConfigError {
    source: FileError,
}

impl From<FileError> for ConfigError {
    fn from(source: FileError) -> Self {
        Self { source }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("config file cannot be loaded")
    }
}

#[tracing::instrument]
fn load_config() -> Result<String, ConfigError> {
    let path = Path::new("my_config");
    let config = read_file(&path)?;
    Ok(config)
}

struct App {
    // Imagine this is actually something we deserialize with serde
    config: String,
}

impl App {
    fn run() -> Result<(), AppError> {
        let this = Self::init()?;
        this.start()
    }

    fn init() -> Result<Self, ConfigError> {
        let config = load_config()?;
        Ok(Self { config })
    }

    fn start(&self) -> Result<(), AppError> {
        // Pretend our actual application logic all exists here
        info!("Loaded config: {}", self.config);
        Ok(())
    }
}

#[derive(Debug)]
struct AppError {
    source: ConfigError,
}

impl From<ConfigError> for AppError {
    fn from(source: ConfigError) -> Self {
        Self { source }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("config invalid")
    }
}

#[tracing::instrument]
fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::subscriber())
        // The `ErrorSubscriber` subscriber layer enables the use of `SpanTrace`.
        .with(ErrorSubscriber::default())
        .init();

    if let Err(e) = App::run() {
        error!(
            error = &e as &(dyn Error + 'static),
            "App exited unsuccessfully"
        );
    }
}
