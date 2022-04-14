use std::future::{self, Future};
#[test]
fn async_fn_is_send() {
    async fn some_async_fn() {
        tracing::info!("{}", future::ready("test").await);
    }

    assert_send(some_async_fn())
}

#[test]
fn async_block_is_send() {
    assert_send(async {
        tracing::info!("{}", future::ready("test").await);
    })
}


fn assert_send<F: Future + Send>(_f: F) {}