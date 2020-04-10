use crate::non_blocking::NonBlocking;
use tracing_subscriber::fmt::MakeWriter;

mod inner;
pub mod non_blocking;
pub mod rolling;
mod worker;

pub fn non_blocking<T: MakeWriter + Send + Sync + 'static>(writer: T) -> NonBlocking {
    NonBlocking::new(writer)
}
