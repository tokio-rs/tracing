use tracing::trace_return;

#[trace_return(ret)]
const fn answer() -> i32 {
    42
}

fn main() {}
