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
        request.method = ?req.method(),
        request.uri = ?req.uri(),
        request.version = ?req.version(),
    )
}

#[inline]
pub fn trace_request<A>(req: &http::Request<A>) -> tracing::Span {
    tracing::span!(
        tracing::Level::TRACE,
        "request",
        request.method = ?req.method(),
        request.uri = ?req.uri(),
        request.version = ?req.version(),
        request.headers = ?req.headers(),
    )
}
