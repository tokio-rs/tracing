use std::{error::Error, fmt};
use tracing::{debug, error, info, span, warn, Level};

#[tracing::instrument]
pub fn shave(yak: usize) -> Result<(), Box<dyn Error + 'static>> {
    debug!(excitement = "yay!", "hello! I'm gonna shave a yak.");
    if yak == 3 {
        warn!("could not locate yak!");
        return Err(ShaveError::new(yak).into());
    } else {
        debug!("yak shaved successfully");
    }
    Ok(())
}

pub fn shave_all(yaks: usize) -> usize {
    let span = span!(Level::TRACE, "shaving_yaks", yaks);
    let _enter = span.enter();

    info!("shaving yaks");

    let mut yaks_shaved = 0;
    for yak in 1..=yaks {
        let res = shave(yak);
        debug!(yak, shaved = res.is_ok());

        if let Err(ref error) = res {
            error!(yak, error = error.as_ref(), "failed to shave yak!");
        } else {
            yaks_shaved += 1;
        }
        debug!(yaks_shaved);
    }

    yaks_shaved
}

#[derive(Debug)]
enum ShaveError {
    MissingYak { yak: usize },
}

impl fmt::Display for ShaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShaveError::MissingYak { yak } => write!(f, "shaving yak #{} failed!", yak),
        }
    }
}

impl Error for ShaveError {}

impl ShaveError {
    fn new(yak: usize) -> Self {
        ShaveError::MissingYak { yak }
    }
}
