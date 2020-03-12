use std::path::PathBuf;
use std::fmt;

#[derive(Debug)]
pub struct Error {
    inner: Kind,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.inner {
            Kind::IO{ ref source, .. } => Some(source),
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
    IO {
        source: std::io::Error,
        path: PathBuf,
    },
}


impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IO { path, .. } => write!(f, "cannot create output file. path={}", path.display()),
        }
    }
}
