#![deny(rust_2018_idioms)]

use tracing::{debug, info};
use tracing_attributes::instrument;

#[instrument]
fn nth_fibonacci(n: u64) -> u64 {
    if n == 0 || n == 1 {
        debug!("Base case");
        1
    } else {
        debug!("Recursing");
        nth_fibonacci(n - 1) + nth_fibonacci(n - 2)
    }
}

#[instrument]
fn fibonacci_seq(to: u64) -> Vec<u64> {
    let mut sequence = vec![];

    for n in 0..=to {
        debug!("Pushing {n} fibonacci", n = n);
        sequence.push(nth_fibonacci(n));
    }

    sequence
}

fn main() {
    use tracing_subscriber::{fmt, Filter};
    let subscriber = fmt::Subscriber::builder()
        .with_filter(Filter::from("args=trace"))
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        let n = 5;
        let sequence = fibonacci_seq(n);
        info!("The first {} fibonacci numbers are {:?}", n, sequence);
    })
}
