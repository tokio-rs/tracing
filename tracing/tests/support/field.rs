use tracing::{
    callsite,
    callsite::Callsite,
    field::{self, Field, Value, Visit},
    metadata::Kind,
};

use std::{
    any::{self, Any, TypeId},
    collections::HashMap,
    fmt,
};

#[derive(Default, Debug, Eq, PartialEq)]
pub struct Expect {
    fields: HashMap<String, MockValue>,
    only: bool,
}

#[derive(Debug)]
pub struct MockField {
    name: String,
    value: MockValue,
}

#[derive(Debug, Eq, PartialEq)]
pub struct MockValue {
    value: MockValueKind,
    downcasts_to: Option<Downcasts>,
}

struct Downcasts {
    check: Box<dyn for<'a> Fn(&'a Value<'a>) -> bool + Send + Sync>,
    name: &'static str,
}

#[derive(Debug, Eq, PartialEq)]
enum MockValueKind {
    I64(i64),
    U64(u64),
    Bool(bool),
    Str(String),
    Debug(String),
    Display(String),
    Any,
}

pub fn mock<K>(name: K) -> MockField
where
    String: From<K>,
{
    MockField {
        name: name.into(),
        value: MockValue {
            value: MockValueKind::Any,
            downcasts_to: None,
        },
    }
}

impl MockField {
    /// Expect a field with the given name and value.
    pub fn with_value(self, value: impl Into<MockValue>) -> Self {
        Self {
            value: value.into(),
            ..self
        }
    }

    pub fn and(self, other: MockField) -> Expect {
        Expect {
            fields: HashMap::new(),
            only: false,
        }
        .and(self)
        .and(other)
    }

    pub fn only(self) -> Expect {
        Expect {
            fields: HashMap::new(),
            only: true,
        }
        .and(self)
    }
}

impl Into<Expect> for MockField {
    fn into(self) -> Expect {
        Expect {
            fields: HashMap::new(),
            only: false,
        }
        .and(self)
    }
}

impl Expect {
    pub fn and(mut self, field: MockField) -> Self {
        self.fields.insert(field.name, field.value);
        self
    }

    /// Indicates that no fields other than those specified should be expected.
    pub fn only(self) -> Self {
        Self { only: true, ..self }
    }

    pub(crate) fn compare_or_panic(&mut self, name: &str, value: &Value<'_>, ctx: &str) {
        match self.fields.remove(name) {
            Some(mock) => {
                match &mock.value {
                    MockValueKind::Any => {}
                    expected => assert!(
                        expected == value,
                        "\nexpected {} to contain:\n\t`{}{}`\nbut got:\n\t`{}{}`",
                        ctx,
                        name,
                        mock,
                        name,
                        value
                    ),
                };
                if let Some(downcasts) = mock.downcasts_to {
                    downcasts.check(value, ctx)
                }
            }
            None if self.only => panic!(
                "\nexpected {} to contain only:\n\t`{}`\nbut got:\n\t`{}{}`",
                ctx, self, name, value
            ),
            _ => {}
        }
    }

    // pub fn checker(&mut self, ctx: String) -> CheckVisitor<'_> {
    //     CheckVisitor { expect: self, ctx }
    // }

    pub(crate) fn assert_done(&self, ctx: &str) {
        assert!(self.fields.is_empty(), "{}missing {}", self, ctx)
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

impl fmt::Display for MockValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.value {
            MockValueKind::I64(v) => write!(f, "i64 = {:?}", v),
            MockValueKind::U64(v) => write!(f, "u64 = {:?}", v),
            MockValueKind::Bool(v) => write!(f, "bool = {:?}", v),
            MockValueKind::Str(ref v) => write!(f, "&str = {:?}", v),
            MockValueKind::Debug(ref v) => write!(f, "&fmt::Debug = {}", v),
            MockValueKind::Display(ref v) => write!(f, "&fmt::Display = {}", v),
            MockValueKind::Any => write!(f, "_ = _"),
        }
    }
}

impl PartialEq<Value<'_>> for MockValueKind {
    fn eq(&self, actual: &Value<'_>) -> bool {
        match self {
            MockValueKind::I64(expected) => actual.as_i64().iter().any(|actual| expected == actual),
            MockValueKind::U64(expected) => actual.as_u64().iter().any(|actual| expected == actual),
            MockValueKind::Bool(expected) => {
                actual.as_bool().iter().any(|actual| expected == actual)
            }
            MockValueKind::Str(expected) => actual.as_str().iter().any(|actual| expected == actual),
            MockValueKind::Debug(expected) => expected == &format!("{:?}", actual),
            MockValueKind::Display(expected) => actual
                .as_display()
                .iter()
                .any(|actual| expected == &format!("{}", actual)),
            MockValueKind::Any => true,
        }
    }
}

impl From<i64> for MockValue {
    fn from(v: i64) -> Self {
        Self {
            value: MockValueKind::I64(v),
            downcasts_to: Some(Downcasts::to::<i64>()),
        }
    }
}

impl From<i32> for MockValue {
    fn from(v: i32) -> Self {
        Self {
            value: MockValueKind::I64(v as i64),
            downcasts_to: Some(Downcasts::to::<i64>()),
        }
    }
}

impl From<u64> for MockValue {
    fn from(v: u64) -> Self {
        Self {
            value: MockValueKind::U64(v),
            downcasts_to: Some(Downcasts::to::<u64>()),
        }
    }
}

impl From<usize> for MockValue {
    fn from(v: usize) -> Self {
        Self {
            value: MockValueKind::U64(v as u64),
            downcasts_to: Some(Downcasts::to::<u64>()),
        }
    }
}

impl From<bool> for MockValue {
    fn from(v: bool) -> Self {
        Self {
            value: MockValueKind::Bool(v),
            downcasts_to: Some(Downcasts::to::<bool>()),
        }
    }
}

impl From<&'_ str> for MockValue {
    fn from(v: &str) -> Self {
        Self {
            value: MockValueKind::Str(v.to_owned()),
            downcasts_to: None,
        }
    }
}

impl From<&'_ dyn fmt::Debug> for MockValue {
    fn from(v: &dyn fmt::Debug) -> Self {
        Self {
            value: MockValueKind::Debug(format!("{:?}", v)),
            downcasts_to: None,
        }
    }
}

impl From<&'_ dyn fmt::Display> for MockValue {
    fn from(v: &dyn fmt::Display) -> Self {
        Self {
            value: MockValueKind::Display(format!("{}", v)),
            downcasts_to: None,
        }
    }
}
impl From<fmt::Arguments<'_>> for MockValue {
    fn from(v: fmt::Arguments<'_>) -> Self {
        Self {
            value: MockValueKind::Debug(v.to_string()),
            downcasts_to: None,
        }
    }
}
// pub struct CheckVisitor<'a> {
//     expect: &'a mut Expect,
//     ctx: String,
// }

// impl<'a> Visit for CheckVisitor<'a> {
//     fn record_i64(&mut self, field: &Field, value: i64) {
//         self.expect
//             .compare_or_panic(field.name(), &value, &self.ctx[..])
//     }

//     fn record_u64(&mut self, field: &Field, value: u64) {
//         self.expect
//             .compare_or_panic(field.name(), &value, &self.ctx[..])
//     }

//     fn record_bool(&mut self, field: &Field, value: bool) {
//         self.expect
//             .compare_or_panic(field.name(), &value, &self.ctx[..])
//     }

//     fn record_str(&mut self, field: &Field, value: &str) {
//         self.expect
//             .compare_or_panic(field.name(), &value, &self.ctx[..])
//     }

//     fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
//         self.expect
//             .compare_or_panic(field.name(), &field::debug(value), &self.ctx)
//     }
// }

// impl<'a> CheckVisitor<'a> {
//     pub fn finish(self) {
//         assert!(
//             self.expect.fields.is_empty(),
//             "{}missing {}",
//             self.expect,
//             self.ctx
//         );
//     }
// }

// impl<'a> From<&'a dyn Value> for MockValue {
//     fn from(value: &'a dyn Value) -> Self {
//         struct MockValueBuilder {
//             value: Option<MockValue>,
//         }

//         impl Visit for MockValueBuilder {
//             fn record_i64(&mut self, _: &Field, value: i64) {
//                 self.value = Some(MockValue::I64(value));
//             }

//             fn record_u64(&mut self, _: &Field, value: u64) {
//                 self.value = Some(MockValue::U64(value));
//             }

//             fn record_bool(&mut self, _: &Field, value: bool) {
//                 self.value = Some(MockValue::Bool(value));
//             }

//             fn record_str(&mut self, _: &Field, value: &str) {
//                 self.value = Some(MockValue::Str(value.to_owned()));
//             }

//             fn record_debug(&mut self, _: &Field, value: &dyn fmt::Debug) {
//                 self.value = Some(MockValue::Debug(format!("{:?}", value)));
//             }
//         }

//         let fake_field = callsite!(name: "fake", kind: Kind::EVENT, fields: fake_field)
//             .metadata()
//             .fields()
//             .field("fake_field")
//             .unwrap();
//         let mut builder = MockValueBuilder { value: None };
//         value.record(&fake_field, &mut builder);
//         builder
//             .value
//             .expect("finish called before a value was recorded")
//     }
// }

impl fmt::Display for Expect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fields ")?;
        let entries = self
            .fields
            .iter()
            .map(|(k, v)| (field::display(k), field::display(v)));
        f.debug_map().entries(entries).finish()
    }
}

impl Downcasts {
    fn to<T: Any>() -> Self {
        Self {
            check: Box::new(|value| value.downcast_ref::<T>().is_some()),
            name: any::type_name::<T>(),
        }
    }

    fn check(&self, value: &Value<'_>, ctx: &str) {
        assert!(
            (self.check)(value),
            "expected {} to downcast to {}, but got {:?}",
            ctx,
            self.name,
            value,
        )
    }
}

impl fmt::Debug for Downcasts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Downcasts")
            .field("to", &format_args!("{}", self.name))
            .finish()
    }
}

impl PartialEq for Downcasts {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Downcasts {}
