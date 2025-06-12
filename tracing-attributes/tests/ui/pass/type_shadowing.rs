type Foo = ();
enum Bar {
    Foo,
}

#[tracing::instrument]
fn this_is_fine() -> Foo {
    // glob import imports Bar::Foo, shadowing Foo
    use Bar::*;
}

fn main() {}
