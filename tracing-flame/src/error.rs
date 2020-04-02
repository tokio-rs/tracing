use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Error {
    inner: Kind,
}

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
        fmt::Display::fmt(&self.inner, f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.inner {
            Kind::CreateFile { ref source, .. } => Some(source),
            Kind::FlushFile(ref source) => Some(source),
            Kind::LockError => None,
        }
    }
}

impl<E> From<E> for Error
where
    E: Into<Kind>,
{
    fn from(err: E) -> Self {
        let inner = err.into();
        Self { inner }
    }
}

#[derive(Debug)]
pub enum Kind {
    CreateFile {
        source: std::io::Error,
        path: PathBuf,
    },
    FlushFile(std::io::Error),
    LockError,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateFile { path, .. } => {
                write!(f, "cannot create output file. path={}", path.display())
            }
            Self::FlushFile { .. } => write!(f, "cannot flush output buffer"),
            Self::LockError => write!(
                f,
                "encountered poison error when acquiring lock on output buffer"
            ),
        }
    }
}
