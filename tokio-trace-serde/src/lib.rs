use std::fmt::{self, Debug, Formatter};

use serde::{
    Serialize,
    ser::{
        SerializeMap,
        SerializeSeq,
        SerializeStruct,
        SerializeTupleVariant,
        Serializer,
    },
};

use tokio_trace::{
    event::Event,
    field::{Field, FieldSet, Visit},
    metadata::Metadata,
    span::{Attributes, Id, Parent, Record},
};

#[derive(Debug)]
pub struct FieldVisitor(pub (crate) Field);

impl Serialize for FieldVisitor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        // i: usize
        // fields: FieldSet
        let mut state = serializer.serialize_struct("Field", 2)?;
        state.serialize_field("i", &self.0.callsite())?;
        state.serialize_field("fields", &FieldSetVisitor(self.0.fields))?;
        state.end()
    }
}

#[derive(Debug)]
pub struct FieldSetVisitor(pub (crate) FieldSet);

impl Serialize for FieldSetVisitor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        // pub names: [&'static str]
        // pub callsite: callsite::Identifier
        //self.callsite.metadata()
        let mut state = serializer.serialize_struct("FieldSet", 2)?;
        let mut seq = serializer.serialize_seq(Some(self.0.names.len()))?;
        for e in self.0.names {
            seq.serialize_element(e)?;
        }
        seq.end();
        state.serialize_field("names", &seq)?;
        let metadata = self.0.callsite.metadata();
        state.serialize_field("callsite", metadata)?;
        state.end()
    }
}

#[derive(Debug)]
pub struct SerializeMetadata<'a>(pub (crate) Metadata<'a>);

impl<'a> Serialize for SerializeMetadata<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        // name: &'static str
        // target: &'a str
        let mut state = serializer.serialize_struct("Metadata", 2)?;
        state.serialize_field("name", self.0.name)?;
        state.serialize_field("target", self.0.target)?;
        state.end()
    }
}

/// A Serde visitor to pull `Event` data out of a serialized stream
#[derive(Debug)]
pub struct SerializeEvent<'a>(pub (crate) Event<'a>);

impl<'a> SerializeEvent<'a> {
    fn new(event: Event<'a>) -> Self {
        SerializeEvent(event)
    }
}

impl<'a> Serialize for SerializeEvent<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        // fields: &'a ValueSet <'a>
        // metadata: &'a Metadata <'a>
        let mut state = serializer.serialize_struct("SerializeEvent", 2)?;
        //state.serialize_field("fields", &ValueSetVisitor(self.0.fields()))?;
        state.serialize_field("metadata", &SerializeMetadata(*self.0.metadata()))?;
        state.end()
    }
}

impl<'a> Visit for SerializeEvent<'a> {
    fn record_debug(&mut self, field: &Field, value: &fmt::Debug) {
        //self.0.field(field.name(), value);
        unimplemented!();
    }
}

/// A Serde visitor to pull `Attributes` data out of a serialized stream
#[derive(Debug)]
pub struct SerializeAttributes<'a>(pub (crate) Attributes<'a>);

impl<'a> SerializeAttributes<'a> {
    fn new(attributes: Attributes<'a>) -> Self {
        SerializeAttributes(attributes)
    }
}

impl<'a> Serialize for SerializeAttributes<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        // metadata
        // values
        // parent: Parent
        let mut state = serializer.serialize_struct("SerializeAttributes", 3)?;
        state.serialize_field("metadata", &SerializeMetadata(*self.0.metadata()))?;
        //state.serialize_field("values", &ValueSetVisitor(*self.0.values()))?;
        match self.0.parent() {
            Root => {
                state.serialize_field("parent", "Root")?;
            }
            Current => {
                state.serialize_field("parent", "Current")?;
            }
            Explicit(ref id) => {
                state.serialize_field("parent", &format!("Explicit: {}", id))?;
            }
        }
        state.end()
    }
}

impl<'a> Visit for SerializeAttributes<'a> {
    fn record_debug(&mut self, field: &Field, value: &fmt::Debug) {
        //self.0.field(field.name(), value);
        unimplemented!();
    }
}

/// A Serde visitor to pull `Record` data out of a serialized stream
#[derive(Debug)]
pub struct RecordValues<'a>(pub (crate) Record<'a>);

impl<'a> RecordValues<'a> {
    fn new(record: Record<'a>) -> Self {
        RecordValues(record)
    }
}

impl<'a> Serialize for RecordValues<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        // values: &'a field::ValueSet<'a>
        let mut state = serializer.serialize_struct("RecordValues", 1)?;
        state.serialize_field("values", &format!("{}", self.0.record(SerdeVisitor(serializer))))?;
        state.end()
    }
}

impl<'a> Visit for RecordValues<'a> {
    fn record_debug(&mut self, field: &Field, value: &fmt::Debug) {
        //self.0.record(field.name(), value);
        unimplemented!();
    }
}

struct SerdeVisitor<S>(S) where S: Serializer;

impl<S> Visit for SerdeVisitor<S> where S: Serializer {
    fn record_debug(&mut self, field: &Field, value: &fmt::Debug) {
        self.0.serialize_field(field.name(), &format!("{}", value));
    }
}

// Then, we could have newtypes that wrap tokio-trace-core's Event, Attributes, and Record types
// and implement the Serialize trait from serde by wrapping the passed in serializer with the
// newtype that implements Visit and using it to record the fields on the event or span. We should
// probably also serialize span IDs and metadata as well. I think we would probably want to
// serialize the metadata using serialize_struct and the fields using serialize_map.

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
