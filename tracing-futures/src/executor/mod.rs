#[cfg(feature = "futures-01")]
mod futures_01;
#[cfg(feature = "futures-01")]
pub use self::futures_01::*;
#[cfg(feature = "std-future")]
mod std_future;
#[cfg(feature = "std-future")]
pub use self::std_future::*;
