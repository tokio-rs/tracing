use tracing_core::Field;
use std::fmt::Debug;

pub trait FieldMatcher: Debug {
    fn matches_field(&self, field: &Field) -> bool;
}

#[derive(Debug)]
pub struct ExactFieldMatcher {
    name: String,
}

impl ExactFieldMatcher {
    pub fn new(name: String) -> Self {
        Self { name, }
    }
}

impl FieldMatcher for ExactFieldMatcher {
    fn matches_field(&self, field: &Field) -> bool {
        field.name() == self.name
    }
}
