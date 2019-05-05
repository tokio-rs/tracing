#[macro_use]
extern crate tokio_trace;
#[macro_use]
extern crate tokio_trace_proc_macros;
extern crate env_logger;
extern crate tokio_trace_fmt;

#[trace]
fn nth_fibonacci(n: u64) -> u64 {
    if n == 0 || n == 1 {
        debug!("Base case");
        1
    } else {
        debug!("Recursing");
        nth_fibonacci(n - 1) + nth_fibonacci(n - 2)
    }
}

#[trace]
fn fibonacci_seq(to: u64) -> Vec<u64> {
    let mut sequence = vec![];

    for n in 0..=to {
        debug!("Pushing {n} fibonacci", n = n);
        sequence.push(nth_fibonacci(n));
    }

    sequence
}

fn main() {
    std::env::set_var("RUST_LOG", "trace");
    env_logger::Builder::new().parse("trace").init();
    let subscriber = tokio_trace_fmt::FmtSubscriber::builder().full().finish();

    tokio_trace::subscriber::with_default(subscriber, || {
        let n: u64 = 5;
        let sequence = fibonacci_seq(n);
        info!("The first {} fibonacci numbers are {:?}", n, sequence);
    })
}
