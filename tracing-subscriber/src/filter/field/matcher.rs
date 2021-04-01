use std::fmt::Debug;
use tracing_core::Field;

pub trait FieldMatcher: Debug {
    fn matches_field(&self, field: &Field) -> bool;
}

#[derive(Debug)]
pub struct ExactFieldMatcher {
    name: String,
}

impl ExactFieldMatcher {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl FieldMatcher for ExactFieldMatcher {
    fn matches_field(&self, field: &Field) -> bool {
        field.name() == self.name
    }
}
