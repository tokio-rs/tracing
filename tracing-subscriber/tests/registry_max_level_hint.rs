use tracing_subscriber::filter::LevelFilter;

#[test]
fn registry_sets_max_level_hint() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::subscriber())
        .with(LevelFilter::DEBUG)
        .init();
    assert_eq!(LevelFilter::current(), LevelFilter::DEBUG);
}
