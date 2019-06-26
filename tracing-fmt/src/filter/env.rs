use regex::Regex;
use tracing_core::{subscriber::Interest, Level, Metadata};
use {filter::Filter, span::Context};

use std::{cmp::Ordering, env, error::Error, fmt, str::FromStr};

pub const DEFAULT_FILTER_ENV: &str = "RUST_LOG";

#[derive(Debug)]
pub struct EnvFilter {
    directives: Vec<Directive>,
    max_level: LevelFilter,
    includes_span_directive: bool,
}

#[derive(Debug)]
pub struct FromEnvError {
    kind: ErrorKind,
}

#[derive(Debug)]
pub struct ParseError {
    directive: String,
}

#[derive(Debug)]
struct Directive {
    target: Option<String>,
    in_span: Option<String>,
    // TODO: this can probably be a `SmallVec` someday, since a span won't have
    // over 32 fields.
    fields: Vec<String>,
    level: LevelFilter,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum LevelFilter {
    Off,
    Level(Level),
}

#[derive(Debug)]
enum ErrorKind {
    Parse(ParseError),
    Env(env::VarError),
}

// ===== impl EnvFilter =====

impl EnvFilter {
    /// Returns a new `EnvFilter` from the value of the `RUST_LOG` environment
    /// variable, ignoring any invalid filter directives.
    pub fn from_default_env() -> Self {
        Self::from_env(DEFAULT_FILTER_ENV)
    }

    /// Returns a new `EnvFilter` from the value of the given environment
    /// variable, ignoring any invalid filter directives.
    pub fn from_env<A: AsRef<str>>(env: A) -> Self {
        env::var(env.as_ref())
            .map(|ref var| Self::new(var))
            .unwrap_or_default()
    }

    /// Returns a new `EnvFilter` from the directives in the given string,
    /// ignoring any that are invalid.
    pub fn new<S: AsRef<str>>(dirs: S) -> Self {
        Self::new2(parse_directives(dirs.as_ref()))
    }

    /// Returns a new `EnvFilter` from the directives in the given string,
    /// or an error if any are invalid.
    pub fn try_new<S: AsRef<str>>(dirs: S) -> Result<Self, ParseError> {
        Ok(Self::new2(try_parse_directives(dirs.as_ref())?))
    }

    /// Returns a new `EnvFilter` from the value of the `RUST_LOG` environment
    /// variable, or an error if the environment variable contains any invalid
    /// filter directives.
    pub fn try_from_default_env() -> Result<Self, FromEnvError> {
        Self::try_from_env(DEFAULT_FILTER_ENV)
    }

    /// Returns a new `EnvFilter` from the value of the given environment
    /// variable, or an error if the environment variable is unset or contains
    /// any invalid filter directives.
    pub fn try_from_env<A: AsRef<str>>(env: A) -> Result<Self, FromEnvError> {
        env::var(env.as_ref())?.parse().map_err(Into::into)
    }

    fn new2(mut directives: Vec<Directive>) -> Self {
        if directives.is_empty() {
            directives.push(Directive::default());
        } else {
            directives.sort_by_key(Directive::len);
        }

        let max_level = directives
            .iter()
            .map(|directive| &directive.level)
            .max()
            .cloned()
            .unwrap_or(LevelFilter::Level(Level::ERROR));

        let includes_span_directive = directives
            .iter()
            .any(|directive| directive.in_span.is_some());

        EnvFilter {
            directives,
            max_level,
            includes_span_directive,
        }
    }

    fn directives_for<'a>(
        &'a self,
        metadata: &'a Metadata<'a>,
    ) -> impl Iterator<Item = &'a Directive> + 'a {
        let target = metadata.target();
        let name = metadata.name();
        self.directives
            .iter()
            .rev()
            .filter_map(move |d| match d.target.as_ref() {
                None => Some(d),
                Some(t) if target.starts_with(t) => Some(d),
                _ => d
                    .in_span
                    .as_ref()
                    .and_then(|span| if span == name { Some(d) } else { None }),
            })
    }
}

impl<S> From<S> for EnvFilter
where
    S: AsRef<str>,
{
    fn from(s: S) -> Self {
        Self::new(s)
    }
}

impl FromStr for EnvFilter {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_new(s)
    }
}

impl Default for EnvFilter {
    fn default() -> Self {
        Self::new2(Default::default())
    }
}

impl<N> Filter<N> for EnvFilter {
    fn callsite_enabled(&self, metadata: &Metadata, _: &Context<N>) -> Interest {
        if !self.includes_span_directive && self.max_level < *metadata.level() {
            return Interest::never();
        }

        let mut interest = Interest::never();
        for directive in self.directives_for(metadata) {
            let accepts_level = directive.level >= *metadata.level();
            match directive.in_span.as_ref() {
                // The directive applies to anything within the span described
                // by this metadata. The span must always be enabled.
                Some(span) if span == metadata.name() => return Interest::always(),

                // The directive may accept this metadata, but
                // only within a particular span. Keep searching
                // to see if there's one that always applies.
                Some(_) if accepts_level => interest = Interest::sometimes(),

                // The directive doesn't care about the current span, and the
                // levels are compatible. We are always interested.
                None if accepts_level => return Interest::always(),

                _ => continue,
            }
        }

        interest
    }

    fn enabled<'a>(&self, metadata: &Metadata, ctx: &Context<'a, N>) -> bool {
        for directive in self.directives_for(metadata) {
            let accepts_level = directive.level >= *metadata.level();
            match directive.in_span.as_ref() {
                // The directive applies to anything within the span described
                // by this metadata. The span must always be enabled.
                Some(span) if span == metadata.name() => return true,

                // The directive doesn't care about the current span, and the
                // levels are compatible. We are always interested.
                None if accepts_level => return true,

                // The directive only applies if we're in a particular span;
                // check if we're currently in that span.
                Some(desired) if accepts_level => {
                    // Are we within the desired span?
                    let in_span = ctx
                        .visit_spans(|_, span| {
                            if span.name() == desired {
                                // If there are no fields, then let's exit.
                                // if there are fields and none of them match
                                // try the next span
                                if directive.fields.is_empty()
                                    || directive
                                        .fields
                                        .iter()
                                        .any(|field| span.fields().contains(field))
                                {
                                    // Return `Err` to short-circuit the span visitation.
                                    Err(())
                                } else {
                                    Ok(())
                                }
                            } else {
                                Ok(())
                            }
                        })
                        .is_err();

                    if in_span {
                        return true;
                    }
                }

                _ => continue,
            }
        }

        false
    }
}

fn parse_directives(spec: &str) -> Vec<Directive> {
    spec.split(',')
        .filter_map(|dir| {
            Directive::parse(dir).or_else(|| {
                eprintln!("ignoring invalid filter directive '{}'", dir);
                None
            })
        })
        .collect()
}

fn try_parse_directives(spec: &str) -> Result<Vec<Directive>, ParseError> {
    spec.split(',')
        .map(|dir| Directive::parse(dir).ok_or_else(|| ParseError::new(dir)))
        .collect()
}

impl fmt::Display for EnvFilter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut directives = self.directives.iter();
        if let Some(d) = directives.next() {
            write!(f, "{}", d)?;
            for d in directives {
                write!(f, ",{}", d)?;
            }
        }
        Ok(())
    }
}

// ===== impl Directive =====

impl Directive {
    fn len(&self) -> usize {
        self.target
            .as_ref()
            .map(String::len)
            .or_else(|| self.in_span.as_ref().map(String::len))
            .unwrap_or(0)
    }

    fn parse(from: &str) -> Option<Self> {
        lazy_static! {
            static ref DIRECTIVE_RE: Regex = Regex::new(
                r"(?x)
                ^(?P<global_level>trace|TRACE|debug|DEBUG|info|INFO|warn|WARN|error|ERROR|off|OFF[0-5])$ |
                ^
                (?: # target name or span name
                    (?P<target>[\w:]+)|(?P<span>\[[^\]]*\])
                ){1,2}
                (?: # level or nothing
                    =(?P<level>trace|TRACE|debug|DEBUG|info|INFO|warn|WARN|error|ERROR|off|OFF[0-5])?
                )?
                $
                "
            )
            .unwrap();
            static ref SPAN_PART_RE: Regex =
                Regex::new(r#"(?P<name>\w+)?(?:\{(?P<fields>[^\}]*)\})?"#).unwrap();
            static ref FIELD_FILTER_RE: Regex =
                Regex::new(r#"([\w_0-9]+(?:=(?:[\w0-9]+|".+"))?)(?: |$)"#).unwrap();
        }

        fn parse_level(from: &str) -> Option<LevelFilter> {
            // TODO: maybe this whole function ought to be replaced by a
            // `FromStr` impl for `Level` in `tracing_core`...?
            from.parse::<usize>()
                .ok()
                .and_then(|num| match num {
                    0 => Some(LevelFilter::Off),
                    1 => Some(LevelFilter::Level(Level::ERROR)),
                    2 => Some(LevelFilter::Level(Level::WARN)),
                    3 => Some(LevelFilter::Level(Level::INFO)),
                    4 => Some(LevelFilter::Level(Level::DEBUG)),
                    5 => Some(LevelFilter::Level(Level::TRACE)),
                    _ => None,
                })
                .or_else(|| match from {
                    "" => Some(LevelFilter::Level(Level::ERROR)),
                    s if s.eq_ignore_ascii_case("error") => Some(LevelFilter::Level(Level::ERROR)),
                    s if s.eq_ignore_ascii_case("warn") => Some(LevelFilter::Level(Level::WARN)),
                    s if s.eq_ignore_ascii_case("info") => Some(LevelFilter::Level(Level::INFO)),
                    s if s.eq_ignore_ascii_case("debug") => Some(LevelFilter::Level(Level::DEBUG)),
                    s if s.eq_ignore_ascii_case("trace") => Some(LevelFilter::Level(Level::TRACE)),
                    s if s.eq_ignore_ascii_case("off") => Some(LevelFilter::Off),
                    _ => None,
                })
        }

        let caps = DIRECTIVE_RE.captures(from)?;

        if let Some(level) = caps
            .name("global_level")
            .and_then(|c| parse_level(c.as_str()))
        {
            return Some(Directive {
                level,
                ..Default::default()
            });
        }

        let target = caps.name("target").and_then(|c| {
            let s = c.as_str();
            if parse_level(s).is_some() {
                None
            } else {
                Some(s.to_owned())
            }
        });

        let (in_span, fields) = caps
            .name("span")
            .and_then(|cap| {
                let cap = cap.as_str().trim_matches(|c| c == '[' || c == ']');
                let caps = SPAN_PART_RE.captures(cap)?;
                let span = caps.name("name").map(|c| c.as_str().to_owned());
                let fields = caps
                    .name("fields")
                    .map(|c| {
                        FIELD_FILTER_RE
                            .find_iter(c.as_str())
                            .map(|c| c.as_str().trim().to_owned())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_else(Vec::new);
                Some((span, fields))
            })
            .unwrap_or_else(|| (None, Vec::new()));

        let level = caps
            .name("level")
            .and_then(|c| parse_level(c.as_str()))
            .unwrap_or(LevelFilter::Level(Level::ERROR));

        Some(Directive {
            level,
            target,
            in_span,
            fields,
        })
    }
}

impl Default for Directive {
    fn default() -> Self {
        Self {
            level: LevelFilter::Level(Level::ERROR),
            target: None,
            in_span: None,
            fields: Vec::new(),
        }
    }
}

impl fmt::Display for Directive {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut write_equals = false;

        if let Some(ref tgt) = self.target {
            write!(f, "{}", tgt)?;
            write_equals = true;
        }

        if self.in_span.is_some() || !self.fields.is_empty() {
            "[".fmt(f)?;
            if let Some(ref span) = self.in_span {
                write!(f, "{}", span)?;
            }

            let mut fields = self.fields.iter();
            if let Some(field) = fields.next() {
                write!(f, "{{{}", field)?;
                for field in fields {
                    write!(f, " {}", field)?;
                }
                "}".fmt(f)?;
            }
            "]".fmt(f)?;
            write_equals = true;
        }

        if write_equals {
            "=".fmt(f)?;
        }

        self.level.fmt(f)
    }
}

// ===== impl FromEnvError =====

impl From<ParseError> for FromEnvError {
    fn from(p: ParseError) -> Self {
        Self {
            kind: ErrorKind::Parse(p),
        }
    }
}

impl From<env::VarError> for FromEnvError {
    fn from(v: env::VarError) -> Self {
        Self {
            kind: ErrorKind::Env(v),
        }
    }
}

impl fmt::Display for FromEnvError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            ErrorKind::Parse(ref p) => p.fmt(f),
            ErrorKind::Env(ref e) => e.fmt(f),
        }
    }
}

impl Error for FromEnvError {
    fn description(&self) -> &str {
        match self.kind {
            ErrorKind::Parse(ref p) => p.description(),
            ErrorKind::Env(ref e) => e.description(),
        }
    }

    #[allow(deprecated)] // for compatibility with minimum Rust version 1.26.0
    fn cause(&self) -> Option<&dyn Error> {
        match self.kind {
            ErrorKind::Parse(ref p) => Some(p),
            ErrorKind::Env(ref e) => Some(e),
        }
    }
}

// ===== impl ParseError =====

impl ParseError {
    fn new(directive: &str) -> Self {
        ParseError {
            directive: directive.to_string(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid filter directive '{}'", self.directive)
    }
}

impl Error for ParseError {
    fn description(&self) -> &str {
        "invalid filter directive"
    }
}

// ===== impl LevelFilter =====

impl PartialEq<Level> for LevelFilter {
    fn eq(&self, other: &Level) -> bool {
        match self {
            LevelFilter::Off => false,
            LevelFilter::Level(l) => l == other,
        }
    }
}

impl PartialOrd<Level> for LevelFilter {
    fn partial_cmp(&self, other: &Level) -> Option<Ordering> {
        match self {
            LevelFilter::Off => Some(Ordering::Less),
            LevelFilter::Level(l) => l.partial_cmp(other),
        }
    }
}

impl fmt::Display for LevelFilter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LevelFilter::Off => f.pad("off"),
            LevelFilter::Level(Level::ERROR) => f.pad("error"),
            LevelFilter::Level(Level::WARN) => f.pad("warn"),
            LevelFilter::Level(Level::INFO) => f.pad("info"),
            LevelFilter::Level(Level::DEBUG) => f.pad("debug"),
            LevelFilter::Level(Level::TRACE) => f.pad("trace"),
            LevelFilter::Level(_) => f.pad("???"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use default::NewRecorder;
    use span::*;
    use tracing_core::*;

    struct Cs;

    impl Callsite for Cs {
        fn set_interest(&self, _interest: Interest) {}
        fn metadata(&self) -> &Metadata {
            unimplemented!()
        }
    }

    #[test]
    fn callsite_enabled_no_span_directive() {
        let filter = EnvFilter::from("app=debug");
        let store = Store::with_capacity(1);
        let ctx = Context::new(&store, &NewRecorder);
        let meta = Metadata::new(
            "mySpan",
            "app",
            Level::TRACE,
            None,
            None,
            None,
            &[],
            &Cs,
            Kind::SPAN,
        );

        let interest = filter.callsite_enabled(&meta, &ctx);
        assert!(interest.is_never());
    }

    #[test]
    fn callsite_off() {
        let filter = EnvFilter::from("app=off");
        let store = Store::with_capacity(1);
        let ctx = Context::new(&store, &NewRecorder);
        let meta = Metadata::new(
            "mySpan",
            "app",
            Level::ERROR,
            None,
            None,
            None,
            &[],
            &Cs,
            Kind::SPAN,
        );

        let interest = filter.callsite_enabled(&meta, &ctx);
        assert!(interest.is_never());
    }

    #[test]
    fn callsite_enabled_includes_span_directive() {
        let filter = EnvFilter::from("app[mySpan]=debug");
        let store = Store::with_capacity(1);
        let ctx = Context::new(&store, &NewRecorder);
        let meta = Metadata::new(
            "mySpan",
            "app",
            Level::TRACE,
            None,
            None,
            None,
            &[],
            &Cs,
            Kind::SPAN,
        );

        let interest = filter.callsite_enabled(&meta, &ctx);
        assert!(interest.is_always());
    }

    #[test]
    fn callsite_enabled_includes_span_directive_field() {
        let filter = EnvFilter::from("app[mySpan{field=\"value\"}]=debug");
        let store = Store::with_capacity(1);
        let ctx = Context::new(&store, &NewRecorder);
        let meta = Metadata::new(
            "mySpan",
            "app",
            Level::TRACE,
            None,
            None,
            None,
            &["field=\"value\""],
            &Cs,
            Kind::SPAN,
        );

        let interest = filter.callsite_enabled(&meta, &ctx);
        assert!(interest.is_always());
    }

    #[test]
    fn callsite_disabled_includes_directive_field() {
        let filter = EnvFilter::from("app[{field=\"novalue\"}]=debug");
        let store = Store::with_capacity(1);
        let ctx = Context::new(&store, &NewRecorder);
        let meta = Metadata::new(
            "mySpan",
            "app",
            Level::TRACE,
            None,
            None,
            None,
            &["field=\"value\""],
            &Cs,
            Kind::SPAN,
        );

        let interest = filter.callsite_enabled(&meta, &ctx);
        assert!(interest.is_never());
    }

    #[test]
    fn callsite_disabled_includes_directive_field_no_value() {
        let filter = EnvFilter::from("app[mySpan{field}]=debug");
        let store = Store::with_capacity(1);
        let ctx = Context::new(&store, &NewRecorder);
        let meta = Metadata::new(
            "mySpan",
            "app",
            Level::TRACE,
            None,
            None,
            None,
            &["field=\"value\""],
            &Cs,
            Kind::SPAN,
        );

        let interest = filter.callsite_enabled(&meta, &ctx);
        assert!(interest.is_always());
    }

    #[test]
    fn callsite_enabled_includes_span_directive_multiple_fields() {
        let filter = EnvFilter::from("app[mySpan{field=\"value\" field2=2}]=debug");
        let store = Store::with_capacity(1);
        let ctx = Context::new(&store, &NewRecorder);
        let meta = Metadata::new(
            "mySpan",
            "app",
            Level::TRACE,
            None,
            None,
            None,
            &["field=\"value\""],
            &Cs,
            Kind::SPAN,
        );

        let interest = filter.callsite_enabled(&meta, &ctx);
        assert!(interest.is_always());
    }

    #[test]
    fn parse_directives_valid() {
        let dirs = parse_directives("crate1::mod1=error,crate1::mod2,crate2=debug,crate3=off");
        assert_eq!(dirs.len(), 4, "\ngot: {:?}", dirs);
        assert_eq!(dirs[0].target, Some("crate1::mod1".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::Level(Level::ERROR));
        assert_eq!(dirs[0].in_span, None);

        assert_eq!(dirs[1].target, Some("crate1::mod2".to_string()));
        assert_eq!(dirs[1].level, LevelFilter::Level(Level::ERROR));
        assert_eq!(dirs[1].in_span, None);

        assert_eq!(dirs[2].target, Some("crate2".to_string()));
        assert_eq!(dirs[2].level, LevelFilter::Level(Level::DEBUG));
        assert_eq!(dirs[2].in_span, None);

        assert_eq!(dirs[3].target, Some("crate3".to_string()));
        assert_eq!(dirs[3].level, LevelFilter::Off);
        assert_eq!(dirs[3].in_span, None);
    }

    #[test]
    fn parse_directives_invalid_crate() {
        // test parse_directives with multiple = in specification
        let dirs = parse_directives("crate1::mod1=warn=info,crate2=debug");
        assert_eq!(dirs.len(), 1, "\ngot: {:?}", dirs);
        assert_eq!(dirs[0].target, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::Level(Level::DEBUG));
        assert_eq!(dirs[0].in_span, None);
    }

    #[test]
    fn parse_directives_invalid_level() {
        // test parse_directives with 'noNumber' as log level
        let dirs = parse_directives("crate1::mod1=noNumber,crate2=debug");
        assert_eq!(dirs.len(), 1, "\ngot: {:?}", dirs);
        assert_eq!(dirs[0].target, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::Level(Level::DEBUG));
        assert_eq!(dirs[0].in_span, None);
    }

    #[test]
    fn parse_directives_string_level() {
        // test parse_directives with 'warn' as log level
        let dirs = parse_directives("crate1::mod1=wrong,crate2=warn");
        assert_eq!(dirs.len(), 1, "\ngot: {:?}", dirs);
        assert_eq!(dirs[0].target, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::Level(Level::WARN));
        assert_eq!(dirs[0].in_span, None);
    }

    #[test]
    fn parse_directives_empty_level() {
        // test parse_directives with '' as log level
        let dirs = parse_directives("crate1::mod1=wrong,crate2=");
        assert_eq!(dirs.len(), 1, "\ngot: {:?}", dirs);
        assert_eq!(dirs[0].target, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::Level(Level::ERROR));
        assert_eq!(dirs[0].in_span, None);
    }

    #[test]
    fn parse_directives_global() {
        // test parse_directives with no crate
        let dirs = parse_directives("warn,crate2=debug");
        assert_eq!(dirs.len(), 2, "\ngot: {:?}", dirs);
        assert_eq!(dirs[0].target, None);
        assert_eq!(dirs[0].level, LevelFilter::Level(Level::WARN));
        assert_eq!(dirs[1].in_span, None);

        assert_eq!(dirs[1].target, Some("crate2".to_string()));
        assert_eq!(dirs[1].level, LevelFilter::Level(Level::DEBUG));
        assert_eq!(dirs[1].in_span, None);
    }

    #[test]
    fn parse_directives_valid_with_spans() {
        let dirs = parse_directives("crate1::mod1[foo]=error,crate1::mod2[bar],crate2[baz]=debug");
        assert_eq!(dirs.len(), 3, "\ngot: {:?}", dirs);
        assert_eq!(dirs[0].target, Some("crate1::mod1".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::Level(Level::ERROR));
        assert_eq!(dirs[0].in_span, Some("foo".to_string()));

        assert_eq!(dirs[1].target, Some("crate1::mod2".to_string()));
        assert_eq!(dirs[1].level, LevelFilter::Level(Level::ERROR));
        assert_eq!(dirs[1].in_span, Some("bar".to_string()));

        assert_eq!(dirs[2].target, Some("crate2".to_string()));
        assert_eq!(dirs[2].level, LevelFilter::Level(Level::DEBUG));
        assert_eq!(dirs[2].in_span, Some("baz".to_string()));
    }

    #[test]
    fn parse_directives_with_fields() {
        let dirs = parse_directives(
            "[span1{foo=1}]=error,[span2{bar=2 baz=false}],crate2[{quux=\"quuux\"}]=debug",
        );
        assert_eq!(dirs.len(), 3, "\ngot: {:?}", dirs);
        assert_eq!(dirs[0].target, None);
        assert_eq!(dirs[0].level, LevelFilter::Level(Level::ERROR));
        assert_eq!(dirs[0].in_span, Some("span1".to_string()));
        assert_eq!(&dirs[0].fields[..], &["foo=1"]);

        assert_eq!(dirs[1].target, None);
        assert_eq!(dirs[1].level, LevelFilter::Level(Level::ERROR));
        assert_eq!(dirs[1].in_span, Some("span2".to_string()));
        assert_eq!(&dirs[1].fields[..], &["bar=2", "baz=false"]);

        assert_eq!(dirs[2].target, Some("crate2".to_string()));
        assert_eq!(dirs[2].level, LevelFilter::Level(Level::DEBUG));
        assert_eq!(dirs[2].in_span, None);
        assert_eq!(&dirs[2].fields[..], &["quux=\"quuux\""]);
    }

    #[test]
    fn roundtrip() {
        let f1: EnvFilter =
            "[span1{foo=1}]=error,[span2{bar=2 baz=false}],crate2[{quux=\"quuux\"}]=debug"
                .parse()
                .unwrap();
        let _: EnvFilter = format!("{}", f1).parse().unwrap();
    }

}
