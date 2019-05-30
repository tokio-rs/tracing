use std::fmt;

use serde::{
    Serialize,
    ser::{
        SerializeMap,
        SerializeStruct,
        Serializer,
    },
};

use tokio_trace::{
    event::Event,
    field::{Field, Visit},
    metadata::Metadata,
    span::{Attributes, Record},
};

#[derive(Debug)]
pub struct FieldVisitor(pub (crate) Field);

impl Serialize for FieldVisitor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        let mut state = serializer.serialize_struct("Field", 1)?;
        state.serialize_field("name", self.0.name())?;
        state.end()
    }
}

#[derive(Debug)]
pub struct SerializeMetadata<'a>(pub (crate) &'a Metadata<'a>);

impl<'a> Serialize for SerializeMetadata<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        let mut state = serializer.serialize_struct("Metadata", 2)?;
        state.serialize_field("name", self.0.name)?;
        state.serialize_field("target", self.0.target)?;
        state.end()
    }
}

/// A Serde visitor to pull `Event` data out of a serialized stream
#[derive(Debug)]
pub struct SerializeEvent<'a>(pub (crate) Event<'a>);

impl<'a> Serialize for SerializeEvent<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        let mut state = serializer.serialize_struct("SerializeEvent", 2)?;
        state.serialize_field("metadata", &SerializeMetadata(self.0.metadata()))?;
        state.end()
    }
}

/// A Serde visitor to pull `Attributes` data out of a serialized stream
#[derive(Debug)]
pub struct SerializeAttributes<'a>(pub (crate) Attributes<'a>);

impl<'a> Serialize for SerializeAttributes<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        let mut state = serializer.serialize_struct("SerializeAttributes", 3)?;
        state.serialize_field("metadata", &SerializeMetadata(self.0.metadata()))?;
        if let Some(id) = self.0.parent() {
            // TODO this probably isn't how we want it
            state.serialize_field("parent", &format!("Explicit: {:?}", id))?;
        }

        else if self.0.is_root() {
            state.serialize_field("parent", "Root")?;
        }

        else {
            state.serialize_field("parent", "Current")?;
        }

        state.end()
    }
}

/// A Serde visitor to pull `Record` data out of a serialized stream
#[derive(Debug)]
pub struct RecordValues<'a>(pub (crate) Record<'a>);

impl<'a> Serialize for RecordValues<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        let our_state = serializer.serialize_map(Some(1))?;
        let mut visitor = SerdeVisitor {
                            serializer: our_state,
                            state: Ok(()),
                          };
        self.0.record(&mut visitor);
        visitor.finish()
    }
}

struct SerdeVisitor<S: SerializeMap> {
    serializer: S,
    state: Result<(), S::Error>
}

impl<S> Visit for SerdeVisitor<S> where S: SerializeMap {
    fn record_bool(&mut self, field: &Field, value: bool) {
        // If previous fields serialized successfully, continue serializing,
        // otherwise, short-circuit and do nothing.
        if self.state.is_ok() {
            self.state = self.serializer
                .serialize_entry(field.name(), &value)
                .map(|_| ());
        }
    }

    fn record_debug(&mut self, field: &Field, value: &fmt::Debug) {
        if self.state.is_ok() {
            self.state = self.serializer
                .serialize_entry(field.name(), &format!("{:?}", value))
                .map(|_| ());
        }
    }
}

impl<S: SerializeMap> SerdeVisitor<S> {

    /// Completes serializing the visited object, returning `Ok(())` if all
    /// fields were serialized correctly, or `Error(S::Error)` if a field could
    /// not be serialized.
    fn finish(self) -> Result<S::Ok, S::Error> {
        self.state?;
        self.serializer.end()
    }
}
