use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::thread;
use tracing::{span, Level, Span};

/// A future that yields once, simulating an async operation that causes
/// the runtime to potentially reschedule the task on a different thread.
struct YieldOnce {
    has_been_polled: bool,
}

impl Future for YieldOnce {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.has_been_polled {
            Poll::Ready(())
        } else {
            self.has_been_polled = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

/// Create a no-op waker for manual polling
fn noop_waker() -> Waker {
    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        |_| RawWaker::new(std::ptr::null(), &VTABLE),
        |_| {},
        |_| {},
        |_| {},
    );
    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
}

#[test]
fn clone_span_of_closed_parent_should_not_panic() {
    let subscriber = tracing_subscriber::registry();
    tracing::subscriber::with_default(subscriber, || {
        let span = span!(Level::INFO, "parent");
        let guard = span.enter();

        std::mem::forget(guard);
        drop(span);

        // This should NOT panic
        let _child = span!(Level::INFO, "child");
    });
}

fn mk_span_dropped_on_different_thread() {
    // Channel for sending the future from main thread to worker thread
    let (send_future, recv_future) = mpsc::channel::<Pin<Box<dyn Future<Output = ()> + Send>>>();
    let (send_done, recv_done) = mpsc::channel::<()>();

    // Use set_global_default so all threads share the same subscriber
    let subscriber = tracing_subscriber::registry();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let mut future = Box::pin(async {
        let span = span!(Level::INFO, "parent_span");
        let _guard = span.enter();

        YieldOnce { has_been_polled: false }.await;

        // Here we resume on a different thread, dropping the guard on that thread
    });

    // Poll once on main thread
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    match future.as_mut().poll(&mut cx) {
        Poll::Pending => {
            // Now send to worker thread which will complete the future
            send_future.send(future).unwrap();
        }
        Poll::Ready(()) => panic!("Future should have yielded"),
    }

    // Worker thread, which receives the partially-polled future and completes it
    let worker = thread::spawn(move || {
        let mut future = recv_future.recv().unwrap();
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(()) => {}
            Poll::Pending => panic!("Future should have completed"),
        }
        // Signal main thread that we're done so it can try to create a child span
        send_done.send(()).unwrap();
    });

    // Wait for worker to finish
    recv_done.recv().unwrap();
    worker.join().unwrap();
}

#[test]
fn span_macro_after_cross_thread_span_should_not_panic() {
    mk_span_dropped_on_different_thread();

    // This should NOT panic
    let _child = span!(Level::INFO, "child_span");
}

// FIXME: This test still fails

// #[test]
// fn current_span_after_cross_thread_span_should_not_panic() {
//     mk_span_dropped_on_different_thread();

//     // This should NOT panic
//     Span::current();
// }