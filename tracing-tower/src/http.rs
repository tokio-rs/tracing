use http;
use tracing;

macro_rules! make_req_fns {
    ($($name:ident, $level:expr),+) => {
        $(
            #[inline]
            pub fn $name<A>(req: &http::Request<A>) -> tracing::Span {
                tracing::span!(
                    $level,
                    "request",
                    request.method = ?req.method(),
                    request.uri = ?req.uri(),
                    request.version = ?req.version(),
                    headers = ?req.headers()
                )
            }
        )+
    }
}

make_req_fns! {
    trace_request, tracing::Level::TRACE,
    debug_request, tracing::Level::DEBUG,
    info_request, tracing::Level::INFO,
    warn_request, tracing::Level::WARN,
    error_request, tracing::Level::ERROR
}
