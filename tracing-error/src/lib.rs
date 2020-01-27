mod backtrace;
mod layer;

pub use self::backtrace::SpanTrace;
pub use self::layer::ErrorLayer;

#[macro_export]
macro_rules! try_bool {
    ($e:expr, $dest:ident) => {{
        let ret = $e.unwrap_or_else(|e| $dest = Some(e));
        if $dest.is_some() {
            return false;
        }
        ret
    }};
}
