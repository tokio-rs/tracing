use std::fmt;
use std::path::PathBuf;

/// The error type for `tracing-flame`
#[derive(Debug)]
pub struct Error(pub(crate) Kind);

impl Error {
    pub(crate) fn report(&self) {
        let current_error: &dyn std::error::Error = self;
        let mut current_error = Some(current_error);
        let mut ind = 0;

        eprintln!("Error:");

        while let Some(error) = current_error {
            eprintln!("    {}: {}", ind, error);
            ind += 1;
            current_error = error.source();
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.0 {
            Kind::CreateFile { ref source, .. } => Some(source),
            Kind::FlushFile(ref source) => Some(source),
        }
    }
}

#[derive(Debug)]
pub(crate) enum Kind {
    CreateFile {
        source: std::io::Error,
        path: PathBuf,
    },
    FlushFile(std::io::Error),
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateFile { path, .. } => {
                write!(f, "cannot create output file. path={}", path.display())
            }
            Self::FlushFile { .. } => write!(f, "cannot flush output buffer"),
        }
    }
}
