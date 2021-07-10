use std::{error::Error, fmt::Display};
use tracing::{debug, error, info, span, trace, warn, Level};

#[derive(Debug)]
enum OutOfSpaceError {
    OutOfCash,
}

impl Error for OutOfSpaceError {}

impl Display for OutOfSpaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutOfSpaceError::OutOfCash => f.write_str("out of cash"),
        }
    }
}

#[derive(Debug)]
enum MissingYakError {
    OutOfSpace { source: OutOfSpaceError },
}

impl Error for MissingYakError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            MissingYakError::OutOfSpace { source } => Some(source),
        }
    }
}

impl Display for MissingYakError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MissingYakError::OutOfSpace { .. } => f.write_str("out of space"),
        }
    }
}

#[derive(Debug)]
enum YakError {
    MissingYak { source: MissingYakError },
}

impl Error for YakError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            YakError::MissingYak { source } => Some(source),
        }
    }
}

impl Display for YakError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            YakError::MissingYak { .. } => f.write_str("missing yak"),
        }
    }
}

// the `#[tracing::instrument]` attribute creates and enters a span
// every time the instrumented function is called. The span is named after the
// the function or method. Paramaters passed to the function are recorded as fields.
#[tracing::instrument]
pub fn shave(yak: usize) -> Result<(), Box<dyn Error + 'static>> {
    // this creates an event at the TRACE log level with two fields:
    // - `excitement`, with the key "excitement" and the value "yay!"
    // - `message`, with the key "message" and the value "hello! I'm gonna shave a yak."
    //
    // unlike other fields, `message`'s shorthand initialization is just the string itself.
    trace!(excitement = "yay!", "hello! I'm gonna shave a yak");
    if yak == 3 {
        warn!("could not locate yak");
        return Err(YakError::MissingYak {
            source: MissingYakError::OutOfSpace {
                source: OutOfSpaceError::OutOfCash,
            },
        }
        .into());
    } else {
        trace!("yak shaved successfully");
    }
    Ok(())
}

pub fn shave_all(yaks: usize) -> usize {
    // Constructs a new span named "shaving_yaks" at the INFO level,
    // and a field whose key is "yaks". This is equivalent to writing:
    //
    // let span = span!(Level::INFO, "shaving_yaks", yaks = yaks);
    //
    // local variables (`yaks`) can be used as field values
    // without an assignment, similar to struct initializers.
    let span = span!(Level::INFO, "shaving_yaks", yaks);
    let _enter = span.enter();

    info!("shaving yaks");

    let mut yaks_shaved = 0;
    for yak in 1..=yaks {
        let res = shave(yak);
        debug!(target: "yak_events", yak, shaved = res.is_ok());

        if let Err(ref error) = res {
            // Like spans, events can also use the field initialization shorthand.
            // In this instance, `yak` is the field being initalized.
            error!(yak, error = error.as_ref(), "failed to shave yak");
        } else {
            yaks_shaved += 1;
        }
        trace!(yaks_shaved);
    }

    yaks_shaved
}
