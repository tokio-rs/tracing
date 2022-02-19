use clippy_utils::diagnostics::span_lint_and_note;
use clippy_utils::match_def_path;
use rustc_hir::def_id::DefId;
use rustc_hir::{AsyncGeneratorKind, Body, BodyId, GeneratorKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::GeneratorInteriorTypeCause;
use rustc_session::{declare_lint, declare_lint_pass};
use rustc_span::Span;

declare_lint! {
    /// ### What it does
    /// Checks for calls to await while holding a
    /// `tracing` span's `Entered` or `EnteredSpan` guards.
    ///
    /// ### Why is this bad?
    /// The guards created by `tracing::Span::enter()` or `tracing::Span::entered()`
    /// across `.await` points will result in incorrect traces. This occurs when
    /// an async function or async block yields at an .await point, the current
    /// scope is exited, but values in that scope are not dropped (because
    /// the async block will eventually resume execution from that await point).
    /// This means that another task will begin executing while remaining in the entered span.
    ///
    /// ### Known problems
    /// Will report false positive for explicitly dropped refs ([#6353](https://github.com/rust-lang/rust-clippy/issues/6353)).
    ///
    /// ### Example
    /// ```rust,ignore
    /// use tracing::{span, Level};
    ///
    /// async fn foo() {
    ///     let span = span!(Level::INFO, "foo");
    ///
    ///     THIS WILL RESULT IN INCORRECT TRACES
    ///     let _enter = span.enter();
    ///     bar().await;
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```rust,ignore
    /// use tracing::{span, Level}
    ///
    /// async fn foo() {
    ///     let span = span!(Level::INFO, "foo");
    ///
    ///     let some_value = span.in_scope(|| {
    ///         // run some synchronous code inside the span...
    ///     });
    ///
    ///     // This is okay! The span has already been exited before we reach
    ///     // the await point.
    ///     bar(some_value).await;
    /// }
    /// ```
    ///
    /// Or use:
    ///
    /// ```rust,ignore
    /// use tracing::{span, Level, Instrument};
    ///
    /// async fn foo() {
    ///     let span = span!(Level::INFO, "foo");
    ///     async move {
    ///         // This is correct! If we yield here, the span will be exited,
    ///         // and re-entered when we resume.
    ///         bar().await;
    ///     }.instrument(span) // instrument the async block with the span...
    ///     .await // ...and await it.
    /// }
    /// ```
    pub AWAIT_HOLDING_SPAN_GUARD,
    Warn,
    "Inside an async function, holding a Span guard while calling await"
}

declare_lint_pass!(AwaitHoldingSpanGuard => [AWAIT_HOLDING_SPAN_GUARD]);

pub const TRACING_SPAN_ENTER_GUARD: [&str; 3] = ["tracing", "span", "Entered"];
pub const TRACING_SPAN_ENTERED_GUARD: [&str; 3] = ["tracing", "span", "EnteredSpan"];

impl LateLintPass<'_> for AwaitHoldingSpanGuard {
    fn check_body(&mut self, cx: &LateContext<'_>, body: &'_ Body<'_>) {
        use AsyncGeneratorKind::{Block, Closure, Fn};
        if let Some(GeneratorKind::Async(Block | Closure | Fn)) = body.generator_kind {
            let body_id = BodyId {
                hir_id: body.value.hir_id,
            };
            let typeck_results = cx.tcx.typeck_body(body_id);
            check_interior_types(
                cx,
                typeck_results
                    .generator_interior_types
                    .as_ref()
                    .skip_binder(),
                body.value.span,
            );
        }
    }
}

fn check_interior_types(
    cx: &LateContext<'_>,
    ty_causes: &[GeneratorInteriorTypeCause<'_>],
    span: Span,
) {
    for ty_cause in ty_causes {
        if let rustc_middle::ty::Adt(adt, _) = ty_cause.ty.kind() {
            if is_tracing_span_guard(cx, adt.did) {
                span_lint_and_note(
                    cx,
                    AWAIT_HOLDING_SPAN_GUARD,
                    ty_cause.span,
                    "this Span guard is held across an 'await' point. Consider using the `.instrument()` combinator or the `.in_scope()` method instead",
                    ty_cause.scope_span.or(Some(span)),
                    "these are all the await points this ref is held through",
                );
            }
        }
    }
}

fn is_tracing_span_guard(cx: &LateContext<'_>, def_id: DefId) -> bool {
    match_def_path(cx, def_id, &TRACING_SPAN_ENTER_GUARD)
        || match_def_path(cx, def_id, &TRACING_SPAN_ENTERED_GUARD)
}
