use crate::non_blocking::NonBlocking;
use std::io::Write;

mod inner;
pub mod non_blocking;
pub mod rolling;
mod worker;

pub fn non_blocking<T: Write + Send + Sync + 'static>(writer: T) -> NonBlocking {
    NonBlocking::new(writer)
}
