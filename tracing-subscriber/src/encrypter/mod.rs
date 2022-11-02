use self::encrypter::Encrypter;
use tracing_core::{field::Visit, Event, Level};

pub mod builder;
pub mod encrypter;

pub struct EncrypterLayer {
    module_rules: Vec<(String, Level)>,
    encrypter: Box<dyn Encrypter + Send + Sync>,
}

impl std::fmt::Debug for EncrypterLayer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "module_rules: {:?}", self.module_rules)
    }
}

#[derive(Debug)]
struct EncrypterVisitor {
    module_path: String,
}

impl Visit for EncrypterVisitor {
    fn record_str(&mut self, field: &tracing_core::Field, value: &str) {
        if field.name() == "log.module_path" {
            self.module_path = value.to_string();
        }
    }
    fn record_debug(&mut self, _field: &tracing_core::Field, _value: &dyn core::fmt::Debug) {}
}

impl EncrypterLayer {
    /// filter and encrypt the msg
    pub fn filter_enc(&self, buf: &mut String, event: &Event<'_>) {
        let module = match event.metadata().module_path() {
            Some(module) => module.to_string(),
            _ => {
                let mut visitor = EncrypterVisitor {
                    module_path: String::new(),
                };
                event.record(&mut visitor);
                visitor.module_path
            }
        };
        let level = event.metadata().level();
        for r in self.module_rules.clone() {
            if module.starts_with(r.0.as_str()) && *level <= r.1 {
                *buf = self.encrypter.encrypt(buf.to_string());
                buf.push('\n');
                return;
            }
        }
    }
}
