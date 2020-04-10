use crate::non_blocking::{NonBlockingWriter, NonBlocking};
use tracing_subscriber::fmt::MakeWriter;

mod inner;
mod non_blocking;
pub mod rolling;
mod worker;

pub fn non_blocking<T: MakeWriter + Send + Sync + 'static>(writer: T) -> NonBlocking {
    NonBlocking::new(writer)
}
