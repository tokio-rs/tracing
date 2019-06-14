use std::fmt;

use serde::{
    ser::{SerializeMap, SerializeSeq, SerializeStruct, Serializer},
    Serialize,
};

use tokio_trace_core::{
    event::Event,
    field::{Field, FieldSet, Visit},
    metadata::{Level, Metadata},
    span::{Attributes, Id, Record},
};

#[derive(Debug)]
pub struct SerializeField(Field);

impl Serialize for SerializeField {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0.name())
    }
}

#[derive(Debug)]
pub struct SerializeFieldSet<'a>(&'a FieldSet);

impl<'a> Serialize for SerializeFieldSet<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for element in self.0 {
            seq.serialize_element(element.name())?;
        }
        seq.end()
    }
}

#[derive(Debug)]
pub struct SerializeLevel<'a>(&'a Level);

impl<'a> Serialize for SerializeLevel<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.0 == &Level::ERROR {
            serializer.serialize_str("ERROR")
        } else if self.0 == &Level::WARN {
            serializer.serialize_str("WARN")
        } else if self.0 == &Level::INFO {
            serializer.serialize_str("INFO")
        } else if self.0 == &Level::DEBUG {
            serializer.serialize_str("DEBUG")
        } else if self.0 == &Level::TRACE {
            serializer.serialize_str("TRACE")
        } else {
            unreachable!()
        }
    }
}

#[derive(Debug)]
pub struct SerializeId<'a>(&'a Id);

impl<'a> Serialize for SerializeId<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Id", 1)?;
        state.serialize_field("u64", &self.0.into_u64())?;
        state.end()
    }
}

#[derive(Debug)]
pub struct SerializeMetadata<'a>(&'a Metadata<'a>);

impl<'a> Serialize for SerializeMetadata<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Metadata", 9)?;
        state.serialize_field("name", self.0.name())?;
        state.serialize_field("target", self.0.target())?;
        state.serialize_field("level", &SerializeLevel(self.0.level()))?;
        state.serialize_field("module_path", &self.0.module_path())?;
        state.serialize_field("file", &self.0.file())?;
        state.serialize_field("line", &self.0.line())?;
        state.serialize_field("fields", &SerializeFieldSet(self.0.fields()))?;
        state.serialize_field("field_names", &self.0.is_span())?;
        state.serialize_field("callsite", &self.0.is_event())?;
        state.end()
    }
}

/// Implements `serde::Serialize` to write `Event` data to a serializer.
#[derive(Debug)]
pub struct SerializeEvent<'a>(&'a Event<'a>);

impl<'a> Serialize for SerializeEvent<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let serializer = serializer.serialize_struct("Event", 2)?;
        let mut visitor = SerdeStructVisitor {
            serializer,
            state: Ok(()),
        };
        self.0.record(&mut visitor);
        visitor.finish()
    }
}

/// A Serde visitor to pull `Attributes` data out of a serialized stream
#[derive(Debug)]
pub struct SerializeAttributes<'a>(&'a Attributes<'a>);

impl<'a> Serialize for SerializeAttributes<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_struct("Attributes", 3)?;
        serializer.serialize_field("metadata", &SerializeMetadata(self.0.metadata()))?;
        serializer.serialize_field("parent", &self.0.parent().map(SerializeId))?;
        serializer.serialize_field("is_root", &self.0.is_root())?;

        let mut visitor = SerdeStructVisitor {
            serializer,
            state: Ok(()),
        };
        self.0.record(&mut visitor);
        visitor.finish()
    }
}

/// A Serde visitor to pull `Record` data out of a serialized stream
#[derive(Debug)]
pub struct SerializeRecord<'a>(&'a Record<'a>);

impl<'a> Serialize for SerializeRecord<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let serializer = serializer.serialize_map(None)?;
        let mut visitor = SerdeMapVisitor {
            serializer,
            state: Ok(()),
        };
        self.0.record(&mut visitor);
        visitor.finish()
    }
}

struct SerdeMapVisitor<S: SerializeMap> {
    serializer: S,
    state: Result<(), S::Error>,
}

impl<S> Visit for SerdeMapVisitor<S>
where
    S: SerializeMap,
{
    fn record_bool(&mut self, field: &Field, value: bool) {
        // If previous fields serialized successfully, continue serializing,
        // otherwise, short-circuit and do nothing.
        if self.state.is_ok() {
            self.state = self.serializer.serialize_entry(field.name(), &value)
        }
    }

    fn record_debug(&mut self, field: &Field, value: &fmt::Debug) {
        if self.state.is_ok() {
            self.state = self
                .serializer
                .serialize_entry(field.name(), &format_args!("{:?}", value))
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        if self.state.is_ok() {
            self.state = self.serializer.serialize_entry(field.name(), &value)
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        if self.state.is_ok() {
            self.state = self.serializer.serialize_entry(field.name(), &value)
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if self.state.is_ok() {
            self.state = self.serializer.serialize_entry(field.name(), &value)
        }
    }
}

impl<S: SerializeMap> SerdeMapVisitor<S> {
    /// Completes serializing the visited object, returning `Ok(())` if all
    /// fields were serialized correctly, or `Error(S::Error)` if a field could
    /// not be serialized.
    fn finish(self) -> Result<S::Ok, S::Error> {
        self.state?;
        self.serializer.end()
    }
}

struct SerdeStructVisitor<S: SerializeStruct> {
    serializer: S,
    state: Result<(), S::Error>,
}

impl<S> Visit for SerdeStructVisitor<S>
where
    S: SerializeStruct,
{
    fn record_bool(&mut self, field: &Field, value: bool) {
        // If previous fields serialized successfully, continue serializing,
        // otherwise, short-circuit and do nothing.
        if self.state.is_ok() {
            self.state = self.serializer.serialize_field(field.name(), &value)
        }
    }

    fn record_debug(&mut self, field: &Field, value: &fmt::Debug) {
        if self.state.is_ok() {
            self.state = self
                .serializer
                .serialize_field(field.name(), &format_args!("{:?}", value))
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        if self.state.is_ok() {
            self.state = self.serializer.serialize_field(field.name(), &value)
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        if self.state.is_ok() {
            self.state = self.serializer.serialize_field(field.name(), &value)
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if self.state.is_ok() {
            self.state = self.serializer.serialize_field(field.name(), &value)
        }
    }
}

impl<S: SerializeStruct> SerdeStructVisitor<S> {
    /// Completes serializing the visited object, returning `Ok(())` if all
    /// fields were serialized correctly, or `Error(S::Error)` if a field could
    /// not be serialized.
    fn finish(self) -> Result<S::Ok, S::Error> {
        self.state?;
        self.serializer.end()
    }
}

/// `AsSerde` is a trait that provides the `as_serde` function to types in
/// `tokio-trace` to allow users to serialize their values.
pub trait AsSerde<'a> {
    type Serializable: serde::Serialize + self::sealed::Sealed + 'a;

    /// `as_serde` borrows a `tokio-trace` value and returns the serialized value.
    fn as_serde(&'a self) -> Self::Serializable;
}

impl<'a> AsSerde<'a> for tokio_trace_core::Metadata<'a> {
    type Serializable = SerializeMetadata<'a>;

    fn as_serde(&'a self) -> Self::Serializable {
        SerializeMetadata(self)
    }
}

impl<'a> AsSerde<'a> for tokio_trace_core::Event<'a> {
    type Serializable = SerializeEvent<'a>;

    fn as_serde(&'a self) -> Self::Serializable {
        SerializeEvent(self)
    }
}

impl<'a> AsSerde<'a> for tokio_trace_core::span::Attributes<'a> {
    type Serializable = SerializeAttributes<'a>;

    fn as_serde(&'a self) -> Self::Serializable {
        SerializeAttributes(self)
    }
}

impl<'a> AsSerde<'a> for tokio_trace_core::span::Id {
    type Serializable = SerializeId<'a>;

    fn as_serde(&'a self) -> Self::Serializable {
        SerializeId(self)
    }
}

impl<'a> AsSerde<'a> for tokio_trace_core::span::Record<'a> {
    type Serializable = SerializeRecord<'a>;

    fn as_serde(&'a self) -> Self::Serializable {
        SerializeRecord(self)
    }
}

impl<'a> self::sealed::Sealed for SerializeMetadata<'a> {}

impl<'a> self::sealed::Sealed for SerializeEvent<'a> {}

impl<'a> self::sealed::Sealed for SerializeAttributes<'a> {}

impl<'a> self::sealed::Sealed for SerializeId<'a> {}

impl<'a> self::sealed::Sealed for SerializeRecord<'a> {}
mod sealed {
    pub trait Sealed {}
}
