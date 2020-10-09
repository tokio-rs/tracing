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
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter("attrs_args=trace")
        .finish();

    tracing::collector::with_default(subscriber, || {
        let n = 5;
        let sequence = fibonacci_seq(n);
        info!("The first {} fibonacci numbers are {:?}", n, sequence);
    })
}
