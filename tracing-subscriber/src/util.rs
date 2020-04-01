use tracing_core::dispatcher::{self, Dispatch};
pub trait SubscriberInitExt
where
    Self: Into<Dispatch>,
{
    fn set_default(self) -> dispatcher::DefaultGuard {
        #[cfg(feature = "tracing-log")]
        let _ = tracing_log::LogTracer::init();

        dispatcher::set_default(&self.into())
    }

    fn try_init(self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        #[cfg(feature = "tracing-log")]
        tracing_log::LogTracer::init().map_err(Box::new)?;

        dispatcher::set_global_default(self.into())?;

        Ok(())
    }

    fn init(self) {
        self.try_init()
            .expect("failed to set global default subscriber")
    }
}

impl<T> SubscriberInitExt for T where T: Into<Dispatch> {}
