use tracing_macros::fatal;

fn fibonacci(n: i32) -> i32 {
    match n {
        0 => 0,
        1 => 1,
        n if n < 0 => fatal!("illegal input {}", n),
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() {
    let collector = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .finish();

    tracing::collect::with_default(collector, || {
        fibonacci(4);
        fibonacci(-1);
    });
}
