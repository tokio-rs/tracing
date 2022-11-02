use tracing_core::Level;

use super::{
    encrypter::{DefaultEncrypter, Encrypter},
    EncrypterLayer,
};

/// encrypter builder
#[derive(Debug)]
pub struct EncrypterLayerBuilder {
    module_rules: Vec<(String, Level)>,
}

impl EncrypterLayerBuilder {
    /// new a builder
    pub fn new() -> Self {
        EncrypterLayerBuilder {
            module_rules: vec![],
        }
    }
    /// adding encrypt rules
    pub fn add_rule(mut self, rule: &str) -> Self {
        let rule: Vec<&str> = rule.split('=').collect();
        if rule.len() < 2 {
            panic!("rule format error");
        }
        self.module_rules
            .push((rule[0].to_string(), rule[1].parse().expect("can't parse level")));
        Self {
            module_rules: self.module_rules,
        }
    }
    /// build to layer
    pub fn build(self, encrypter: Box<dyn Encrypter + Send + Sync>) -> EncrypterLayer {
        EncrypterLayer {
            module_rules: self.module_rules,
            encrypter,
        }
    }
    /// use default encrypter to build
    pub fn default(self) -> EncrypterLayer {
        EncrypterLayer {
            module_rules: self.module_rules,
            encrypter: Box::new(DefaultEncrypter),
        }
    }
}

#[test]
fn test_builder() {
    use super::encrypter::TestEncrypter;
    let _ = EncrypterLayerBuilder::new()
        .add_rule("xxx=off")
        .add_rule("xxx=off")
        .build(Box::new(TestEncrypter));
}
