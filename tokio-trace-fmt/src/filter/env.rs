use regex::Regex;
use tokio_trace_core::{subscriber::Interest, Level, Metadata};
use {filter::Filter, span::Context};

use std::{cmp::Ordering, env};

pub const DEFAULT_FILTER_ENV: &str = "RUST_LOG";

#[derive(Debug)]
pub struct EnvFilter {
    directives: Vec<Directive>,
    max_level: LevelFilter,
    includes_span_directive: bool,
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

// ===== impl EnvFilter =====

impl EnvFilter {
    pub fn from_default_env() -> Self {
        Self::from_env(DEFAULT_FILTER_ENV)
    }

    pub fn from_env<A: AsRef<str>>(env: A) -> Self {
        let directives = env::var(env.as_ref())
            .map(|ref var| parse_directives(var))
            .unwrap_or_default();
        Self::new(directives)
    }

    fn new(mut directives: Vec<Directive>) -> Self {
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

impl<A> From<A> for EnvFilter
where
    A: AsRef<str>,
{
    fn from(env: A) -> Self {
        Self::new(parse_directives(env.as_ref()))
    }
}

impl<N> Filter<N> for EnvFilter {
    fn callsite_enabled(&self, metadata: &Metadata, _: &Context<N>) -> Interest {
        if !self.includes_span_directive && self.max_level <= *metadata.level() {
            return Interest::never();
        }

        let mut interest = Interest::never();
        for directive in self.directives_for(metadata) {
            let accepts_level = directive.level > *metadata.level();
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
            let accepts_level = directive.level > *metadata.level();
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
                eprintln!("ignoring invalid log directive '{}'", dir);
                None
            })
        })
        .collect()
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
            // `FromStr` impl for `Level` in `tokio_trace_core`...?
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

#[cfg(test)]
mod tests {
    use super::*;
    use default::NewRecorder;
    use span::*;
    use tokio_trace_core::*;

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

}
