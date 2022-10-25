use tracing::subscriber::with_default;
use tracing_mock::{event, field, span, subscriber};

fn prepare_yak_shaving() {
    tracing::info!("preparing to shave yaks");
}

// If this test gets updated, the `tracing-mock` README.md must be updated too.
#[test]
fn prepare_yak_shaving_traced() {
    let (subscriber, handle) = subscriber::mock()
        .event(event::mock().with_fields(field::msg("preparing to shave yaks")))
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        prepare_yak_shaving();
    });

    handle.assert_finished();
}

mod yak_shave {
    pub fn shave_all(number_of_yaks: u32) -> u32 {
        number_of_yaks
    }
}

#[tracing::instrument]
fn yak_shaving(number_of_yaks: u32) {
    tracing::info!(number_of_yaks, "preparing to shave yaks");

    let number_shaved = yak_shave::shave_all(number_of_yaks);
    tracing::info!(
        all_yaks_shaved = number_shaved == number_of_yaks,
        "yak shaving completed."
    );
}

// If this test gets updated, the `tracing-mock` README.md must be updated too.
#[test]
fn yak_shaving_traced() {
    let yak_count: u32 = 3;
    let span = span::mock().named("yak_shaving");

    let (subscriber, handle) = subscriber::mock()
        .new_span(
            span.clone()
                .with_field(field::mock("number_of_yaks").with_value(&yak_count).only()),
        )
        .enter(span.clone())
        .event(
            event::mock().with_fields(
                field::mock("number_of_yaks")
                    .with_value(&yak_count)
                    .and(field::msg("preparing to shave yaks"))
                    .only(),
            ),
        )
        .event(
            event::mock().with_fields(
                field::mock("all_yaks_shaved")
                    .with_value(&true)
                    .and(field::msg("yak shaving completed."))
                    .only(),
            ),
        )
        .exit(span)
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        yak_shaving(yak_count);
    });

    handle.assert_finished();
}
