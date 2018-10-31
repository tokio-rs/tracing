#![feature(test)]
#[macro_use]
extern crate tokio_trace;
extern crate test;
use test::Bencher;

#[bench]
fn bench_span_no_subscriber(b: &mut Bencher) {
    let n = test::black_box(1);
    b.iter(|| {
        (0..n).fold(0, |old, new| {
            span!("span");
            old ^ new
        })
    });
}

#[bench]
fn bench_no_span_no_subscriber(b: &mut Bencher) {
    let n = test::black_box(1);
    b.iter(|| (0..n).fold(0, |new, old| old ^ new));
}

#[bench]
fn bench_1_atomic_load(b: &mut Bencher) {
    // This is just included as a baseline.
    let n = test::black_box(1);
    use std::sync::atomic::{AtomicUsize, Ordering};
    let foo = AtomicUsize::new(1);
    b.iter(|| (0..n).fold(0, |new, _old| foo.load(Ordering::Relaxed) ^ new));
}
