use proc_macro_hack::proc_macro_hack;

#[proc_macro_hack]
pub use tracing_macro_impls::debug;
#[proc_macro_hack]
pub use tracing_macro_impls::error;
#[proc_macro_hack]
pub use tracing_macro_impls::event;
#[proc_macro_hack]
pub use tracing_macro_impls::info;
#[proc_macro_hack]
pub use tracing_macro_impls::trace;
#[proc_macro_hack]
pub use tracing_macro_impls::warn;
