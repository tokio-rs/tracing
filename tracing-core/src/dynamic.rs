//! Support for runtime injected, dynamically resolved metadata.
//!
//! # Limitations
//!
//! Dynamically resolved metadata is dynamically resolved. This means that
//! anything looking at the static metadata before the dynamic metadata is
//! injected will see the placeholder metadata.
//!
//! [`Collect::register_callsite`] necessarily only sees the static placeholder
//! metadata. Generally, any place that deals in `Metadata<'static>` sees the
//! placeholders. This means that any decisions made at this point may be wrong
//! if the placeholder metadata causes the collector to not register interest.
//  XXX: Can/should we mitigate this by not respecting register_callsite?
//!
//! It's not sufficient to observe a magic field name and assume that dynamic
//! metadata is on its way later. It is perfectly acceptable to provide space
//! for dynamic injection and then not use it.
//  Due to limitations in the macro, this is almost always the case.
//!
//! For the most part, this is handled transparently. For example,
//! [`Event::record`] will only visit fields not used in this manner. If you
//! want the magic fields, you need to use [`RecordFields::record_prenormal`].
//! However, becuase spans are registered piecewise, it's still possible to see
//! magic fields there.
//  XXX: We need *more tests* for this, and ideally, magic fields *should* be
//  completely hidden unless you specifically ask for them. The hedging here is
//  solely due to me not being sure, due to the lack of new tests to cover this.
//!
//! [`Collect::new_span`]: crate::Collect::new_span
//! [`Collect::register_callsite`]: crate::Collect::register_callsite
//! [`Event::record`]: crate::Event::record
//! [`RecordFields::record_prenormal`]: crate::field::RecordFields::record_prenormal
//!
//! # Strategy
//!
//! In order to deliver the dynamic metadata, a magic reserved name is used:
//! [`MAGIC_NAME`]. If (and only if) the metadata's name is this identifier,
//! special field names are recognized and patched on top of the static
//! metadata. The fields must be the leading prefix of the field set and must
//! appear in the given order:
//!
//! - [`MAGIC_FIELD_NAME`]: Override [`Metadata::name`]. Record as `&str`.
//! - [`MAGIC_FIELD_TARGET`]: Override [`Metadata::target`]. Record as `&str`.
//!
//!   <div class="example-wrap" style="display:inline-block">
//!   <pre class="ignore" style="white-space:normal;font:inherit;">
//!
//!   **Note**: Although it is possible to override the target, it is generally
//!   advisable to make the initial target as accurate as possible, as static
//!   filtering is done with the static metadata's target.
//!   </pre></div>
//!
//! - [`MAGIC_FIELD_LEVEL`]: Override [`Metadata::level`]. Record as `&str`.
//!
//!   <div class="example-wrap" style="display:inline-block">
//!   <pre class="ignore" style="white-space:normal;font:inherit;">
//!
//!   **Note**: Although it is possible to override the level, it is generally
//!   advisable to make the initial target as accurate as possible, as static
//!   filtering is done with the static metadata's level.
//!   </pre></div>
//!
//! - [`MAGIC_FIELD_FILE`]: Override [`Metadata::file`]. Record as `&str`.
//! - [`MAGIC_FIELD_LINE`]: Override [`Metadata::line`]. Record as `u64`.
//! - [`MAGIC_FIELD_MODULE_PATH`]: Override [`Metadata::module_path`]. Record as `&str`.

use core::convert::TryInto;

use crate::{
    field::{self, FieldSet},
    metadata::Kind,
    Level, Metadata,
};

macro_rules! magic_prefix {
    () => {
        "[\u{FDD2}\u{FDEC}]"
    };
}

/// The magic name that specifies your metadata should be dynamically injected
/// from the contents of specifically named attatched fields.
///
/// We use two [noncharacters] to specify the magic string. This gives 10 bits
/// of uniqueness space and uses explicitly for internal use codepoints.
/// Noncharacters should never show up in normal text use.
///
/// [noncharacters]: http://www.unicode.org/faq/private_use.html#nonchar1
pub const MAGIC_NAME: &str = magic_prefix!();
/// The magic runtime metadata field for the name.
pub const MAGIC_FIELD_NAME: &str = concat!(magic_prefix!(), " name");
/// The magic runtime metadata field for the target.
pub const MAGIC_FIELD_TARGET: &str = concat!(magic_prefix!(), " target");
/// The magic runtime metadata field for the level.
pub const MAGIC_FIELD_LEVEL: &str = concat!(magic_prefix!(), " level");
/// The magic runtime metadata field for the file.
pub const MAGIC_FIELD_FILE: &str = concat!(magic_prefix!(), " file");
/// The magic runtime metadata field for the line.
pub const MAGIC_FIELD_LINE: &str = concat!(magic_prefix!(), " line");
/// The magic runtime metadata field for the module path.
pub const MAGIC_FIELD_MODULE_PATH: &str = concat!(magic_prefix!(), " module_path");

#[derive(Default)]
pub(crate) struct MagicFields {
    name: Option<field::Field>,
    target: Option<field::Field>,
    level: Option<field::Field>,
    file: Option<field::Field>,
    line: Option<field::Field>,
    module_path: Option<field::Field>,
    field_count: usize,
}

impl MagicFields {
    pub(crate) fn new(fields: &FieldSet) -> Self {
        let mut fields = fields.iter().peekable();
        let mut magic = Self::default();

        let _: Option<()> = (|| {
            if fields.peek()?.name() == MAGIC_FIELD_NAME {
                magic.name = fields.next();
                magic.field_count += 1;
            }
            if fields.peek()?.name() == MAGIC_FIELD_TARGET {
                magic.target = fields.next();
                magic.field_count += 1;
            }
            if fields.peek()?.name() == MAGIC_FIELD_LEVEL {
                magic.level = fields.next();
                magic.field_count += 1;
            }
            if fields.peek()?.name() == MAGIC_FIELD_FILE {
                magic.file = fields.next();
                magic.field_count += 1;
            }
            if fields.peek()?.name() == MAGIC_FIELD_LINE {
                magic.line = fields.next();
                magic.field_count += 1;
            }
            if fields.peek()?.name() == MAGIC_FIELD_MODULE_PATH {
                magic.module_path = fields.next();
                magic.field_count += 1;
            }

            Some(())
        })();

        magic
    }

    pub(crate) fn count(&self) -> usize {
        self.field_count
    }
}

impl Metadata<'_> {
    pub(crate) fn normalized<'a>(
        &'a self,
        fields: &dyn field::RecordFields<'a>,
    ) -> Option<Metadata<'a>> {
        if !self.is_dynamic() {
            return None;
        }

        let mut visitor = MagicVisitor {
            name: self.name(),
            target: self.target(),
            level: *self.level(),
            file: self.file(),
            line: self.line(),
            module_path: self.module_path(),
            fields: self.magic_fields(),
        };

        fields.record_prenormal(&mut visitor);
        return Some(Metadata::new(
            visitor.name,
            visitor.target,
            visitor.level,
            visitor.file,
            visitor.line,
            visitor.module_path,
            self.fields(),
            Kind::EVENT,
        ));
    }

    /// Check if this metadata contains any [dynamically resolved metadata][crate::dynamic],
    /// and thus needs [normalization][Self::normalized].
    pub(crate) fn is_dynamic(&self) -> bool {
        self.name() == MAGIC_NAME
    }
}

struct MagicVisitor<'a> {
    name: &'a str,
    target: &'a str,
    level: Level,
    file: Option<&'a str>,
    line: Option<u32>,
    module_path: Option<&'a str>,
    fields: MagicFields,
}

impl<'a> field::Visit<'a> for MagicVisitor<'a> {
    fn record_u64(&mut self, field: &crate::Field, value: u64) {
        if Some(field) == self.fields.line.as_ref() {
            self.line = Some(value.try_into().unwrap_or(u32::MAX));
        }
    }

    fn record_str(&mut self, field: &crate::Field, value: &'a str) {
        if Some(field) == self.fields.name.as_ref() {
            self.name = value;
        } else if Some(field) == self.fields.target.as_ref() {
            self.target = value;
        } else if Some(field) == self.fields.level.as_ref() {
            self.level = value.parse().unwrap_or(self.level);
        } else if Some(field) == self.fields.file.as_ref() {
            self.file = Some(value);
        } else if Some(field) == self.fields.module_path.as_ref() {
            self.module_path = Some(value);
        }
    }

    fn record_debug(&mut self, _: &crate::Field, _: &dyn core::fmt::Debug) {}
}

#[doc(hidden)]
#[macro_export]
macro_rules! __tracing_core_tts_if {
    (if ($($yes:tt)+) { $($then:tt)* } else { $($else:tt)* }) => ($($then)*);
    (if (           ) { $($then:tt)* } else { $($else:tt)* }) => ($($else)*);
}

#[doc(hidden)]
#[macro_export]
macro_rules! __tracing_core_ensure_safe_name {
    (name) => {
        ::core::compile_error!(
            "found field `name` out of order; if this is what you meant, use `r#name`"
        )
    };
    (target) => {
        ::core::compile_error!(
            "found field `target` out of order; if this is what you meant, use `r#target`"
        )
    };
    (level) => {
        ::core::compile_error!(
            "found field `level` out of order; if this is what you meant, use `r#level`"
        )
    };
    (file) => {
        ::core::compile_error!(
            "found field `file` out of order; if this is what you meant, use `r#file`"
        )
    };
    (line) => {
        ::core::compile_error!(
            "found field `line` out of order; if this is what you meant, use `r#line`"
        )
    };
    (module) => {
        ::core::compile_error!(
            "found field `module` out of order; if this is what you meant, use `r#module`"
        )
    };
    ($tt:tt) => {
        ()
    };
}

/// Construct [`Metadata`] used for [dynamic resolution][crate::dynamic].
///
/// # Examples
///
/// Construct a minimal dynamic metadata, where the fields `target` and `level`
/// and injectable. The value provided in this invocation is the default.
///
/// ```rust
/// # use tracing_core::{dynamic_metadata, Level, Metadata};
/// static DYN_META: &Metadata<'_> = dynamic_metadata! {
///     target: "crate",
///     level: Level::INFO,
/// };
/// ```
///
/// <div class="example-wrap" style="display:inline-block">
/// <pre class="ignore" style="white-space:normal;font:inherit;">
///
/// **Note**: Although it is possible to override the target and level, it is
/// generally advisable to make the target as accurate as statically possible,
/// and to not override the level. Static filtering is done with the static
/// metadata through [`Collect::register_callsite`], and if a span/event is
/// filtered out there, the collector will never see the dynamic override.
///
/// [`Collect::register_callsite`]: crate::Collect::register_callsite
///
/// </pre></div>
///
/// <div class="example-wrap" style="display:inline-block">
/// <pre class="ignore" style="white-space:normal;font:inherit;">
///
/// **Note**: While technically a custom field, `message` is by convention
/// always included on events; this is the field which holds the actual
/// human-formatted log message. **If you don't specify the field here, you
/// won't be able to include it when you emit an event with this metadata!**
///
/// </pre></div>
///
/// Construct a metadata that allows overriding all special fields and that
/// carries a message.
///
/// ```rust
/// # use tracing_core::{dynamic_metadata, Level, Metadata};
/// static DYN_META: &Metadata<'_> = dynamic_metadata! {
///     name,
///     target: "crate",
///     level: Level::INFO,
///     file,
///     line,
///     module,
///     message,
/// };
/// ```
///
/// <div class="example-wrap" style="display:inline-block">
/// <pre class="ignore" style="white-space:normal;font:inherit;">
///
/// **Note**: `file`, `line`, and `module` can also have defaults specified.
/// `name` cannot have a default specified; the static value is always set to
/// [`MAGIC_NAME`].
///
/// </pre></div>
///
/// <div class="example-wrap" style="display:inline-block">
/// <pre class="ignore" style="white-space:normal;font:inherit;">
///
/// **Note**: The order of the metadata fields is semantic. If you don't get the
/// order correct, compilation will fail with an error noting which identifier
/// was out of order. If you do intend to have a real field with this name, you
/// can use a raw identifier with `r#`. (This is stripped by most collectors.)
///
/// </pre></div>
///
/// # Syntax
///
/// ```
/// # macro_rules! m {{
/// $(name,)?
/// target: $target:expr,
/// level: $level:expr,
/// $(file $(: $file:expr)? ,)?
/// $(line $(: $line:expr)? ,)?
/// $(module $(: $module:expr)? ,)?
/// $($field:ident,)*
/// # } => {}}
/// ```
#[macro_export]
macro_rules! dynamic_metadata {
    // === base case ===
    {@{
        $(name: $name:expr,)?
        $(target: $target:expr,)?
        $(level: $level:expr,)?
        $(file: $file:expr,)?
        $(line: $line:expr,)?
        $(module: $module:expr,)?
        ;
        $($field_name:ident),* $(,)?
    }} => {{
        struct Callsite;
        static CALLSITE: Callsite = Callsite;
        static META: $crate::Metadata<'static> = $crate::Metadata::new(
            $crate::dynamic::MAGIC_NAME,
            $crate::__tracing_core_tts_if! {
                if ($($target)?) {
                    $($target)?
                } else {
                    ::core::compile_error!("dynamic_metadata requires a specified target")
                }
            },
            $crate::__tracing_core_tts_if! {
                if ($($level)?) {
                    $($level)?
                } else {
                    ::core::compile_error!("dynamic_metadata requires a specified level")
                }
            },
            $crate::__tracing_core_tts_if! {
                if ($($file)?) {
                    $($file)?
                } else {
                    ::core::option::Option::None
                }
            },
            $crate::__tracing_core_tts_if! {
                if ($($line)?) {
                    $($line)?
                } else {
                    ::core::option::Option::None
                }
            },
            $crate::__tracing_core_tts_if! {
                if ($($module)?) {
                    $($module)?
                } else {
                    ::core::option::Option::None
                }
            },
            $crate::field::FieldSet::new(
                &[
                    // TODO(cad97): This doesn't allow specifying a meta field
                    // default but not adding in the magic field to the
                    // fieldset, and while that isn't actively *wrong* (as
                    // without the field, the default is used), it definitely
                    // feels suboptimal. Ideally, there should be some way to
                    // specify that a default will not be dynamically overriden.
                    $({let _ = $name;   $crate::dynamic::MAGIC_FIELD_NAME},)?
                    $({let _ = $target; $crate::dynamic::MAGIC_FIELD_TARGET},)?
                    $({let _ = $level;  $crate::dynamic::MAGIC_FIELD_LEVEL},)?
                    $({let _ = $file;   $crate::dynamic::MAGIC_FIELD_FILE},)?
                    $({let _ = $line;   $crate::dynamic::MAGIC_FIELD_LINE},)?
                    $({let _ = $module; $crate::dynamic::MAGIC_FIELD_MODULE_PATH},)?
                    $({$crate::__tracing_core_ensure_safe_name!($field_name); ::core::stringify!($field_name)}),*
                ],
                $crate::identify_callsite!(&CALLSITE)
            ),
            $crate::metadata::Kind::EVENT,
        );
        impl $crate::Callsite for Callsite {
            fn set_interest(&self, _: $crate::collect::Interest) {}
            fn metadata(&self) -> &'static $crate::Metadata<'static> {
                &META
            }
        }
        &META
    }};

    // === recursive case (more tts) ===
    {
        @{
        }
        name
        $(, $($rest:tt)*)?
    } => { $crate::dynamic_metadata! {
        @{
            name: "placeholder to be matched",
        }
        $($($rest)*)?
    } };

    {
        @{
            $(name: $name:expr,)?
        }
        target $(: $target:expr)?
        $(, $($rest:tt)*)?
    } => { $crate::dynamic_metadata! {
        @{
            $(name: $name,)?
            $(target: $target,)?
        }
        $($($rest)*)?
    }};

    {
        @{
            $(name: $name:expr,)?
            $(target: $target:expr,)?
        }
        level $(: $level:expr)?
        $(, $($rest:tt)*)?
    } => { $crate::dynamic_metadata! {
        @{
            $(name: $name,)?
            $(target: $target,)?
            $(level: $level,)?
        }
        $($($rest)*)?
    }};

    {
        @{
            $(name: $name:expr,)?
            $(target: $target:expr,)?
            $(level: $level:expr,)?
        }
        file $(: $file:expr)?
        $(, $($rest:tt)*)?
    } => { $crate::dynamic_metadata! {
        @{
            $(name: $name,)?
            $(target: $target,)?
            $(level: $level,)?
            file: $crate::__tracing_core_tts_if! {
                if ($($file)?) {
                    ::core::option::Option::Some($($file)?)
                } else {
                    ::core::option::Option::<&str>::None
                }
            },
        }
        $($($rest)*)?
    }};

    {
        @{
            $(name: $name:expr,)?
            $(target: $target:expr,)?
            $(level: $level:expr,)?
            $(file: $file:expr,)?
        }
        line $(: $line:expr)?
        $(, $($rest:tt)*)?
    } => { $crate::dynamic_metadata! {
        @{
            $(name: $name,)?
            $(target: $target,)?
            $(level: $level,)?
            $(file: $file,)?
            line: $crate::__tracing_core_tts_if! {
                if ($($line)?) {
                    ::core::option::Option::Some($($line)?)
                } else {
                    ::core::option::Option::<u32>::None
                }
            },
        }
        $($($rest)*)?
    }};

    {
        @{
            $(name: $name:expr,)?
            $(target: $target:expr,)?
            $(level: $level:expr,)?
            $(file: $file:expr,)?
            $(line: $line:expr,)?
        }
        module $(: $module:expr)?
        $(, $($rest:tt)*)?
    } => { $crate::dynamic_metadata! {
        @{
            $(name: $name,)?
            $(target: $target,)?
            $(level: $level,)?
            $(file: $file,)?
            $(line: $line,)?
            module: $crate::__tracing_core_tts_if! {
                if ($($module)?) {
                    ::core::option::Option::Some($($module)?)
                } else {
                    ::core::option::Option::<&str>::None
                }
            },
        }
        $($($rest)*)?
    }};

    {
        @{
            $(name: $name:expr,)?
            $(target: $target:expr,)?
            $(level: $level:expr,)?
            $(file: $file:expr,)?
            $(line: $line:expr,)?
            $(module: $module:expr,)?
        }
        $($field_name:ident),* $(,)?
    } => { $crate::dynamic_metadata! {
        @{
            $(name: $name,)?
            $(target: $target,)?
            $(level: $level,)?
            $(file: $file,)?
            $(line: $line,)?
            $(module: $module,)?
            ;
            $($field_name,)*
        }
    }};

    // === entry ===
    { $first:ident $($tt:tt)* } => { $crate::dynamic_metadata! { @{} $first $($tt)* } };
}

#[cfg(test)]
mod tests {
    static _1: &crate::Metadata<'_> = dynamic_metadata! {
        target: "tracing_core",
        level: crate::Level::INFO,
        message,
    };

    static _2: &crate::Metadata<'_> = dynamic_metadata! {
        name,
        target: "tracing_core",
        level: crate::Level::INFO,
        file,
        line,
        module,
        message,
    };
}
