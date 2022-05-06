//! # tracing-serde
//!
//! An adapter for serializing [`tracing`] types using [`serde`].
//!
//! [![Documentation][docs-badge]][docs-url]
//! [![Documentation (master)][docs-master-badge]][docs-master-url]
//!
//! [docs-badge]: https://docs.rs/tracing-serde/badge.svg
//! [docs-url]: crate
//! [docs-master-badge]: https://img.shields.io/badge/docs-master-blue
//! [docs-master-url]: https://tracing-rs.netlify.com/tracing_serde
//!
//! ## Overview
//!
//! [`tracing`] is a framework for instrumenting Rust programs to collect
//! scoped, structured, and async-aware diagnostics.`tracing-serde` enables
//! serializing `tracing` types using [`serde`].
//!
//! Traditional logging is based on human-readable text messages.
//! `tracing` gives us machine-readable structured diagnostic
//! information. This lets us interact with diagnostic data
//! programmatically. With `tracing-serde`, you can implement a
//! `Collector` to serialize your `tracing` types and make use of the
//! existing ecosystem of `serde` serializers to talk with distributed
//! tracing systems.
//!
//! Serializing diagnostic information allows us to do more with our logged
//! values. For instance, when working with logging data in JSON gives us
//! pretty-print when we're debugging in development and you can emit JSON
//! and tracing data to monitor your services in production.
//!
//! The `tracing` crate provides the APIs necessary for instrumenting
//! libraries and applications to emit trace data.
//!
//! *Compiler support: [requires `rustc` 1.49+][msrv]*
//!
//! [msrv]: #supported-rust-versions
//!
//! ## Usage
//!
//! First, add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! tracing = "0.1"
//! tracing-serde = "0.1"
//! ```
//!
//! Next, add this to your crate:
//!
//! ```rust
//! use tracing_serde::AsSerde;
//! ```
//!
//! Please read the [`tracing` documentation](https://docs.rs/tracing/latest/tracing/index.html)
//! for more information on how to create trace data.
//!
//! This crate provides the `as_serde` function, via the `AsSerde` trait,
//! which enables serializing the `Attributes`, `Event`, `Id`, `Metadata`,
//! and `Record` `tracing` values.
//!
//! For the full example, please see the [examples](../examples) folder.
//!
//! Implement a `Collector` to format the serialization of `tracing`
//! types how you'd like.
//!
//! ```rust
//! # use tracing_core::{Collect, Metadata, Event};
//! # use tracing_core::span::{Attributes, Current, Id, Record};
//! # use std::sync::atomic::{AtomicUsize, Ordering};
//! use tracing_serde::AsSerde;
//! use serde_json::json;
//!
//! pub struct JsonSubscriber {
//!     next_id: AtomicUsize, // you need to assign span IDs, so you need a counter
//! }
//!
//! impl Collect for JsonSubscriber {
//!
//!     fn new_span(&self, attrs: &Attributes<'_>) -> Id {
//!         let id = self.next_id.fetch_add(1, Ordering::Relaxed);
//!         let id = Id::from_u64(id as u64);
//!         let json = json!({
//!         "new_span": {
//!             "attributes": attrs.as_serde(),
//!             "id": id.as_serde(),
//!         }});
//!         println!("{}", json);
//!         id
//!     }
//!
//!     fn event(&self, event: &Event<'_>) {
//!         let json = json!({
//!            "event": event.as_serde(),
//!         });
//!         println!("{}", json);
//!     }
//!
//!     // ...
//!     # fn enabled(&self, _: &Metadata<'_>) -> bool { false }
//!     # fn enter(&self, _: &Id) {}
//!     # fn exit(&self, _: &Id) {}
//!     # fn record(&self, _: &Id, _: &Record<'_>) {}
//!     # fn record_follows_from(&self, _: &Id, _: &Id) {}
//!     # fn current_span(&self) -> Current { Current::unknown() }
//! }
//! ```
//!
//! After you implement your `Collector`, you can use your `tracing`
//! subscriber (`JsonSubscriber` in the above example) to record serialized
//! trace data.
//!
//! ##  Crate Feature Flags
//!
//! The following crate feature flags are available:
//!
//! * `std`: Depend on the Rust standard library (enabled by default).
//!
//!   `no_std` users may disable this feature with `default-features = false`:
//!
//!   ```toml
//!   [dependencies]
//!   tracing-serde = { version = "0.2", default-features = false }
//!   ```
//
//!   **Note**:`tracing-serde`'s `no_std` support requires `liballoc`.
//!
//! ## Supported Rust Versions
//!
//! Tracing is built against the latest stable release. The minimum supported
//! version is 1.49. The current Tracing version is not guaranteed to build on
//! Rust versions earlier than the minimum supported version.
//!
//! Tracing follows the same compiler support policies as the rest of the Tokio
//! project. The current stable Rust compiler and the three most recent minor
//! versions before it will always be supported. For example, if the current
//! stable compiler version is 1.45, the minimum supported version will not be
//! increased past 1.42, three minor versions prior. Increasing the minimum
//! supported compiler version is not considered a semver breaking change as
//! long as doing so complies with this policy.
//!
//! [`tracing`]: https://crates.io/crates/tracing
//! [`serde`]: https://crates.io/crates/serde
#![doc(html_root_url = "https://docs.rs/tracing-serde/0.1.2")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/logo-type.png",
    html_favicon_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/favicon.ico",
    issue_tracker_base_url = "https://github.com/tokio-rs/tracing/issues/"
)]
// #![warn(
//     missing_debug_implementations,
//     // missing_docs, // TODO: add documentation
//     rust_2018_idioms,
//     unreachable_pub,
//     bad_style,
//     const_err,
//     dead_code,
//     improper_ctypes,
//     non_shorthand_field_patterns,
//     no_mangle_generic_items,
//     overflowing_literals,
//     path_statements,
//     patterns_in_fns_without_body,
//     private_in_public,
//     unconditional_recursion,
//     unused,
//     unused_allocation,
//     unused_comparisons,
//     unused_parens,
//     while_true
// )]
// Support using tracing-serde without the standard library!
#![cfg_attr(not(feature = "std"), no_std)]

use core::fmt;
use core::num::NonZeroU64;
use std::fmt::Arguments;

use serde::{
    ser::{SerializeMap, SerializeSeq, SerializeStruct, SerializeTupleStruct, Serializer},
    Serialize,
    Deserialize,
};

use tracing_core::{
    event::Event,
    field::{Field, FieldSet, Visit},
    metadata::{Level, Metadata},
    span::{Attributes, Id, Record},
};

pub mod fields;


#[derive(Debug, Serialize, Deserialize)]
pub struct SerializeId {
    id: NonZeroU64,
}

#[repr(usize)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum SerializeLevel {
    /// The "trace" level.
    ///
    /// Designates very low priority, often extremely verbose, information.
    Trace = 0,
    /// The "debug" level.
    ///
    /// Designates lower priority information.
    Debug = 1,
    /// The "info" level.
    ///
    /// Designates useful information.
    Info = 2,
    /// The "warn" level.
    ///
    /// Designates hazardous situations.
    Warn = 3,
    /// The "error" level.
    ///
    /// Designates very serious errors.
    Error = 4,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SerializeField<'a> {
    name: &'a str,
}

#[derive(Deserialize)]
#[serde(from = "Vec<&'a str>")]
pub enum SerializeFieldSet<'a> {
    Ser(&'a FieldSet),
    // TODO - How to handle this on nostd? Optional Heapless?
    #[serde(borrow)]
    De(Vec<&'a str>),
}

impl<'a> Serialize for SerializeFieldSet<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let SerializeFieldSet::Ser(sfs) = self {
            let mut seq = serializer.serialize_seq(Some(sfs.len()))?;
            for element in sfs.iter() {
                seq.serialize_element(element.name())?;
            }
            seq.end()
        } else {
            todo!()
        }
    }
}

impl<'a> From<Vec<&'a str>> for SerializeFieldSet<'a> {
    fn from(other: Vec<&'a str>) -> Self {
        SerializeFieldSet::De(other)
    }
}

#[derive(Serialize, Deserialize)]
pub struct SerializeMetadata<'a> {
    name: &'a str,
    target: &'a str,
    level: SerializeLevel,
    module_path: Option<&'a str>,
    file: Option<&'a str>,
    line: Option<u32>,
    fields: SerializeFieldSet<'a>,
    is_span: bool,
    is_event: bool,
}

#[derive(Deserialize)]
pub struct DeserEventRecord<'a> {
    #[serde(borrow)]
    field: SerializeField<'a>,
    record: RecordValueSetItem<'a>,
}

#[derive(Serialize, Deserialize)]
pub enum RecordValueSetItem<'a> {
    Debug(DebugRecord<'a>),
    Str(&'a str),
    F64(f64),
    I64(i64),
    U64(u64),
    Bool(bool),
}

#[derive(Deserialize)]
#[serde(from = "&'a str")]
pub enum DebugRecord<'a> {
    Ser(&'a Arguments<'a>),
    De(&'a str),
}

impl<'a> From<&'a str> for DebugRecord<'a> {
    fn from(other: &'a str) -> Self {
        Self::De(other)
    }
}

impl<'a> Serialize for DebugRecord<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            DebugRecord::Ser(args) => {
                serializer.serialize_newtype_variant(
                    "DebugRecord",
                    1,
                    "De",
                    args
                )
            }
            DebugRecord::De(msg) => {
                serializer.serialize_newtype_variant(
                    "DebugRecord",
                    1,
                    "De",
                    msg
                )
            }
        }
    }
}


/// Implements `serde::Serialize` to write `Attributes` data to a serializer.
#[derive(Serialize, Deserialize)]
pub struct SerializeAttributes<'a> {
    #[serde(borrow)]
    metadata: SerializeMetadata<'a>,
    parent: Option<SerializeId>,
    is_root: bool,
}

/// Implements `tracing_core::field::Visit` for some `serde::ser::SerializeMap`.
#[derive(Debug)]
pub struct SerdeSeqVisitor<S: SerializeSeq> {
    serializer: S,
    state: Result<(), S::Error>,
}

impl<S> SerdeSeqVisitor<S>
where
    S: SerializeSeq,
{
    /// Create a new map visitor.
    pub fn new(serializer: S) -> Self {
        Self {
            serializer,
            state: Ok(()),
        }
    }

    /// Completes serializing the visited object, returning `Ok(())` if all
    /// fields were serialized correctly, or `Error(S::Error)` if a field could
    /// not be serialized.
    pub fn finish(self) -> Result<S::Ok, S::Error> {
        self.state?;
        self.serializer.end()
    }

    /// Completes serializing the visited object, returning ownership of the underlying serializer
    /// if all fields were serialized correctly, or `Err(S::Error)` if a field could not be
    /// serialized.
    pub fn take_serializer(self) -> Result<S, S::Error> {
        self.state?;
        Ok(self.serializer)
    }
}

impl<S> Visit for SerdeSeqVisitor<S>
where
    S: SerializeSeq,
{
    fn record_bool(&mut self, field: &Field, value: bool) {
        // If previous fields serialized successfully, continue serializing,
        // otherwise, short-circuit and do nothing.
        if self.state.is_ok() {
            // self.state = self.serializer.serialize_element(&DeserEventRecord {
            //     field: SerializeField { name: field.name() },
            //     record: DeserRecordValueSetItem::Bool(value),
            // })
            todo!()
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if self.state.is_ok() {
            // let res = || {
            //     let mut serializer = self.serializer.serialize_struct("DeserEventRecord", 2)?;
            //     serializer.serialize_field("field", &SerializeField { name: field.name() })?;
            //     // serializer.serialize_field("record", value)
            //     let mut serializer = serializer.serialize_tuple_variant("record", 0, "Debug", 1)?;
            //     serializer.serialize_field(&format_args!("{:?}", value))
            // };
            // self.state = res();
            todo!()

            // self.state = self.serializer.serialize_element(&DeserEventRecord {
            //     field: SerializeField { name: field.name() },
            //     record: Some(DeserRecordValueSetItem::Debug(&format_args!("{:?}", value))),
            // })
        }
    }

    // fn record_u64(&mut self, field: &Field, value: u64) {
    //     // if self.state.is_ok() {
    //     //     self.state = self.serializer.serialize_entry(field.name(), &value)
    //     // }
    // }

    // fn record_i64(&mut self, field: &Field, value: i64) {
    //     // if self.state.is_ok() {
    //     //     self.state = self.serializer.serialize_entry(field.name(), &value)
    //     // }
    // }

    // fn record_f64(&mut self, field: &Field, value: f64) {
    //     if self.state.is_ok() {
    //         self.state = self.serializer.serialize_entry(field.name(), &value)
    //     }
    // }

    // fn record_str(&mut self, field: &Field, value: &str) {
    //     if self.state.is_ok() {
    //         self.state = self.serializer.serialize_entry(field.name(), &value)
    //     }
    // }
}

// ------------------------------------ LINE OF WORK ------------------------------------

#[derive(Deserialize)]
#[serde(from = "Vec<DeserEventRecord<'a>>")]
pub enum SerializeEventRecordFields<'a> {
    #[serde(borrow)]
    Ser(&'a Event<'a>),
    De(Vec<DeserEventRecord<'a>>),
}

impl<'a> From<Vec<DeserEventRecord<'a>>> for SerializeEventRecordFields<'a> {
    fn from(other: Vec<DeserEventRecord<'a>>) -> Self {
        Self::De(other)
    }
}

#[derive(Serialize, Deserialize)]
pub struct SerializeEvent<'a> {
    #[serde(borrow)]
    fields: SerializeEventRecordFields<'a>,
    metadata: SerializeMetadata<'a>,
    parent: Option<SerializeId>,
}

impl<'a> Serialize for SerializeEventRecordFields<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let SerializeEventRecordFields::Ser(serf) = self {
            let mut ctr = VisitCounter { ct: 0 };
            serf.record(&mut ctr);
            let items = ctr.ct;
            let mut serializer = serializer.serialize_seq(Some(items));
            todo!()
        } else {
            todo!()
        }
        // let mut serializer = serializer.serialize_struct("Event", 2)?;
        // serializer.serialize_field("metadata", &self.0.metadata().as_serde())?;
        // let mut visitor = SerdeStructVisitor {
        //     serializer,
        //     state: Ok(()),
        // };
        // self.0.record(&mut visitor);
        // visitor.finish()
    }
}

/// Implements `serde::Serialize` to write `Record` data to a serializer.
#[derive(Debug)]
pub struct SerializeRecord<'a>(&'a Record<'a>);

impl<'a> Serialize for SerializeRecord<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let serializer = serializer.serialize_map(None)?;
        let mut visitor = SerdeMapVisitor::new(serializer);
        self.0.record(&mut visitor);
        visitor.finish()
    }
}

/// Implements `tracing_core::field::Visit` for some `serde::ser::SerializeMap`.
#[derive(Debug)]
pub struct SerdeMapVisitor<S: SerializeMap> {
    serializer: S,
    state: Result<(), S::Error>,
}

impl<S> SerdeMapVisitor<S>
where
    S: SerializeMap,
{
    /// Create a new map visitor.
    pub fn new(serializer: S) -> Self {
        Self {
            serializer,
            state: Ok(()),
        }
    }

    /// Completes serializing the visited object, returning `Ok(())` if all
    /// fields were serialized correctly, or `Error(S::Error)` if a field could
    /// not be serialized.
    pub fn finish(self) -> Result<S::Ok, S::Error> {
        self.state?;
        self.serializer.end()
    }

    /// Completes serializing the visited object, returning ownership of the underlying serializer
    /// if all fields were serialized correctly, or `Err(S::Error)` if a field could not be
    /// serialized.
    pub fn take_serializer(self) -> Result<S, S::Error> {
        self.state?;
        Ok(self.serializer)
    }
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

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
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

    fn record_f64(&mut self, field: &Field, value: f64) {
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

/// Implements `tracing_core::field::Visit` for some `serde::ser::SerializeStruct`.
#[derive(Debug)]
pub struct SerdeStructVisitor<S: SerializeStruct> {
    serializer: S,
    state: Result<(), S::Error>,
}

struct VisitCounter {
    ct: usize,
}

impl Visit for VisitCounter {
    #[inline(always)]
    fn record_debug(&mut self, _field: &Field, _value: &dyn fmt::Debug) {
        self.ct += 1;
    }
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

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
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

    fn record_f64(&mut self, field: &Field, value: f64) {
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
    pub fn finish(self) -> Result<S::Ok, S::Error> {
        self.state?;
        self.serializer.end()
    }
}

pub trait AsSerde<'a>: self::sealed::Sealed {
    type Serializable: serde::Serialize + 'a;

    /// `as_serde` borrows a `tracing` value and returns the serialized value.
    fn as_serde(&'a self) -> Self::Serializable;
}

impl<'a> AsSerde<'a> for tracing_core::Metadata<'a> {
    type Serializable = SerializeMetadata<'a>;

    fn as_serde(&'a self) -> Self::Serializable {
        SerializeMetadata {
            name: self.name(),
            target: self.target(),
            level: self.level().as_serde(),
            module_path: self.module_path(),
            file: self.file(),
            line: self.line(),
            fields: SerializeFieldSet::Ser(self.fields()),
            is_span: self.is_span(),
            is_event: self.is_event(),
        }
    }
}

impl<'a> AsSerde<'a> for tracing_core::Event<'a> {
    type Serializable = SerializeEvent<'a>;

    fn as_serde(&'a self) -> Self::Serializable {
        SerializeEvent {
            fields: SerializeEventRecordFields::Ser(self),
            metadata: self.metadata().as_serde(),
            parent: self.parent().map(|p| p.as_serde()),
        }
    }
}

impl<'a> AsSerde<'a> for tracing_core::span::Attributes<'a> {
    type Serializable = SerializeAttributes<'a>;

    fn as_serde(&'a self) -> Self::Serializable {
        SerializeAttributes {
            metadata: self.metadata().as_serde(),
            parent: self.parent().map(|p| p.as_serde()),
            is_root: self.is_root(),
        }
    }
}

impl<'a> AsSerde<'a> for tracing_core::span::Id {
    type Serializable = SerializeId;

    fn as_serde(&'a self) -> Self::Serializable {
        SerializeId {
            id: self.into_non_zero_u64(),
        }
    }
}

impl<'a> AsSerde<'a> for tracing_core::span::Record<'a> {
    type Serializable = SerializeRecord<'a>;

    fn as_serde(&'a self) -> Self::Serializable {
        SerializeRecord(self)
    }
}

impl<'a> AsSerde<'a> for Level {
    type Serializable = SerializeLevel;

    fn as_serde(&'a self) -> Self::Serializable {
        match self {
            &Level::ERROR => SerializeLevel::Error,
            &Level::WARN => SerializeLevel::Warn,
            &Level::INFO => SerializeLevel::Info,
            &Level::DEBUG => SerializeLevel::Debug,
            &Level::TRACE => SerializeLevel::Trace,
        }
    }
}

impl<'a> self::sealed::Sealed for Event<'a> {}

impl<'a> self::sealed::Sealed for Attributes<'a> {}

impl self::sealed::Sealed for Id {}

impl self::sealed::Sealed for Level {}

impl<'a> self::sealed::Sealed for Record<'a> {}

impl<'a> self::sealed::Sealed for Metadata<'a> {}

mod sealed {
    pub trait Sealed {}
}
