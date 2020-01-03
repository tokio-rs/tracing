#![deny(rust_2018_idioms)]
use tracing::{debug, error, info, span, trace, warn, Level};

use std::{error::Error, fmt};

#[tracing::instrument]
pub fn shave(yak: usize) -> Result<(), Box<dyn Error + 'static>> {
    debug!(
        message = "hello! I'm gonna shave a yak.",
        excitement = "yay!"
    );
    if yak == 3 {
        warn!(target: "yak_events", "could not locate yak!");
        return Err(ShaveError::new(yak, YakError::new("could not locate yak")).into());
    } else {
        trace!(target: "yak_events", "yak shaved successfully");
    }
    Ok(())
}

pub fn shave_all(yaks: usize) -> usize {
    let span = span!(Level::TRACE, "shaving_yaks", yaks_to_shave = yaks);
    let _enter = span.enter();

    info!("shaving yaks");

    let mut num_shaved = 0;
    for yak in 1..=yaks {
        let res = shave(yak);
        trace!(target: "yak_events", yak, shaved = res.is_ok());

        if let Err(ref error) = res {
            error!(
                message = "failed to shave yak!",
                yak,
                error = error.as_ref()
            );
        } else {
            num_shaved += 1;
        }

        trace!(target: "yak_events", yaks_shaved = num_shaved);
    }

    num_shaved
}

#[derive(Debug)]
struct ShaveError {
    source: Box<dyn Error + 'static>,
    yak: usize,
}

impl fmt::Display for ShaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "shaving yak #{} failed!", self.yak)
    }
}

impl Error for ShaveError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}

impl ShaveError {
    fn new(yak: usize, source: impl Into<Box<dyn Error + 'static>>) -> Self {
        Self {
            source: source.into(),
            yak,
        }
    }
}

#[derive(Debug)]
struct YakError {
    description: &'static str,
}

impl fmt::Display for YakError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}

impl Error for YakError {}

impl YakError {
    fn new(description: &'static str) -> Self {
        Self { description }
    }
}
