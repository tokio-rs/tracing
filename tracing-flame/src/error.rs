use displaydoc::Display;
use std::path::PathBuf;

#[derive(Display, Debug, thiserror::Error)]
pub enum Error {
    /// Encountered an io error creating file: {path}
    IO {
        source: std::io::Error,
        path: PathBuf,
    },
}
