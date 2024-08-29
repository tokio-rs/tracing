#[cfg(feature = "futures-01")]
mod futures_01;

#[cfg(feature = "futures-03")]
mod futures_03;
#[allow(unreachable_pub, unused_imports)]
#[cfg(feature = "futures-03")]
pub use futures_03::*;
