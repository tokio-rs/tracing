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
                    method = ?req.method(),
                    uri = ?req.uri(),
                )
            }
        )+
    }
}

make_req_fns! {
    info_request, tracing::Level::INFO,
    warn_request, tracing::Level::WARN,
    error_request, tracing::Level::ERROR
}

#[inline]
pub fn debug_request<A>(req: &http::Request<A>) -> tracing::Span {
    tracing::span!(
        tracing::Level::DEBUG,
        "request",
        method = ?req.method(),
        uri = ?req.uri(),
        version = ?req.version(),
    )
}

#[inline]
pub fn trace_request<A>(req: &http::Request<A>) -> tracing::Span {
    tracing::span!(
        tracing::Level::TRACE,
        "request",
        method = ?req.method(),
        uri = ?req.uri(),
        version = ?req.version(),
        headers = ?req.headers(),
    )
}
