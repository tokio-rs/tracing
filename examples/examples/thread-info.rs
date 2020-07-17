#![deny(rust_2018_idioms)]
/// This is a example showing how thread info can be displayed when
/// formatting events with `tracing_subscriber::fmt`. This is useful
/// as `tracing` spans can be entered by multicple threads concurrently,
/// or move across threads freely.
///
/// You can run this example by running the following command in a terminal
///
/// ```
/// cargo run --example thread-info
/// ```
///
/// Example output:
///
/// ```not_rust
/// Jul 17 00:38:07.177  INFO ThreadId(02) thread_info: i=9
/// Jul 17 00:38:07.177  INFO            thread 1 ThreadId(03) thread_info: i=9
/// Jul 17 00:38:07.177  INFO large name thread 2 ThreadId(04) thread_info: i=9
/// ```
use std::thread;
use std::time::Duration;
use tracing::info;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        // enable thread id to be emitted
        .with_thread_ids(true)
        // enabled thread name to be emitted
        .with_thread_names(true)
        .init();

    let do_work = || {
        for i in 1..10 {
            info!(i);
            thread::sleep(Duration::from_millis(1));
        }
    };

    let thread_with_no_name = thread::spawn(do_work);
    let thread_one = thread::Builder::new()
        .name("thread 1".to_string())
        .spawn(do_work)
        .expect("could not spawn a new thread");
    let thread_two = thread::Builder::new()
        .name("large name thread 2".to_string())
        .spawn(do_work)
        .expect("could not spawn a new thread");

    thread_with_no_name
        .join()
        .expect("could not wait for a thread");
    thread_one.join().expect("could not wait for a thread");
    thread_two.join().expect("could not wait for a thread");
}
