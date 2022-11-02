//! An example demonstrating how `fmt::Encrypter` can write to multiple
//! destinations (in this instance, `stdout` and a file) simultaneously.

#[path = "fmt/yak_shave.rs"]
mod yak_shave;

use std::time::Duration;

use tracing_subscriber::{encrypter::builder::EncrypterLayerBuilder, EnvFilter};

#[tokio::main]
async fn main() {
    log();
    tracing::info!("321321321");
    log::info!("dsadas");
    tokio::spawn(license_client::init(
        "http://192.168.200.138:3031/check-license".to_string(),
        "/Users/_naokiararaki/Code/license-test/client-license".to_string(),
    ));
    loop {
        // println!("dsadasd");
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

fn log() {
    let enc = EncrypterLayerBuilder::new()
        .add_rule("license=error")
        .add_rule("hyper=")
        .default();

    tracing_subscriber::fmt()
        .with_encrypter(enc)
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::TRACE.into()))
        .init();

    // let number_of_yaks = 3;
    // // this creates a new event, outside of any spans.
    // tracing::info!(number_of_yaks, "preparing to shave yaks");

    // let number_shaved = yak_shave::shave_all(number_of_yaks);
    // tracing::info!(
    //     all_yaks_shaved = number_shaved == number_of_yaks,
    //     "yak shaving completed."
    // );

}
