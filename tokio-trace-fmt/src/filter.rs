use span;

use tokio_trace_core::{subscriber::Interest, Level, Metadata};

use std::env;

pub const DEFAULT_FILTER_ENV: &'static str = "RUST_LOG";

pub trait Filter {
    fn callsite_enabled(&self, metadata: &Metadata, ctx: &span::Context) -> Interest {
        if self.enabled(metadata, ctx) {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    fn enabled(&self, metadata: &Metadata, ctx: &span::Context) -> bool;
}

#[derive(Debug)]
pub struct EnvFilter {
    directives: Vec<Directive>,
    max_level: Level,
}

#[derive(Debug)]
struct Directive {
    target: Option<String>,
    in_span: Option<String>,
    level: Level,
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
            .unwrap_or(Level::ERROR);

        EnvFilter {
            directives,
            max_level,
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

impl Filter for EnvFilter {
    fn callsite_enabled(&self, metadata: &Metadata, _: &span::Context) -> Interest {
        if metadata.level() > &self.max_level {
            return Interest::never();
        }

        let mut interest = Interest::never();
        for directive in self.directives_for(metadata) {
            let accepts_level = metadata.level() <= &directive.level;
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

    fn enabled(&self, metadata: &Metadata, ctx: &span::Context) -> bool {
        for directive in self.directives_for(metadata) {
            let accepts_level = metadata.level() <= &directive.level;
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
                                // Return `Err` to short-circuit the span visitation.
                                Err(())
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
    // N.B. that this is based pretty closely on the `env_logger` crate,
    // since we want to accept a superset of their syntax. Refer to
    // https://github.com/sebasmagri/env_logger/blob/master/src/filter/mod.rs

    let mut dirs = Vec::new();

    for dir in spec.split(',') {
        if let Some(dir) = Directive::parse(dir) {
            dirs.push(dir);
        } else {
            eprintln!("ignoring invalid log directive '{}'", dir);
        }
    }

    dirs
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
        fn parse_level(from: &str) -> Option<Level> {
            // TODO: maybe this whole function ought to be replaced by a
            // `FromStr` impl for `Level` in `tokio_trace_core`...?
            from.parse::<usize>()
                .ok()
                .and_then(|num| match num {
                    1 => Some(Level::ERROR),
                    2 => Some(Level::WARN),
                    3 => Some(Level::INFO),
                    4 => Some(Level::DEBUG),
                    5 => Some(Level::TRACE),
                    _ => None,
                })
                .or_else(|| match from {
                    "" => Some(Level::ERROR),
                    s if s.eq_ignore_ascii_case("error") => Some(Level::ERROR),
                    s if s.eq_ignore_ascii_case("warn") => Some(Level::WARN),
                    s if s.eq_ignore_ascii_case("info") => Some(Level::INFO),
                    s if s.eq_ignore_ascii_case("debug") => Some(Level::DEBUG),
                    s if s.eq_ignore_ascii_case("trace") => Some(Level::TRACE),
                    _ => None,
                })
        }

        fn parse_span_target(from: &str) -> Option<(Option<String>, Option<String>)> {
            let mut parts = from.split('[');
            let target = parts
                .next()
                .and_then(|part| if part.len() == 0 { None } else { Some(part) });
            let span_part = parts.next();
            if parts.next().is_some() {
                return None;
            }
            let in_span = if let Some(part) = span_part {
                let mut parts = part.split(']');
                let (part0, part1) = (parts.next(), parts.next());
                if part1 != Some("") {
                    return None;
                }
                part0
            } else {
                None
            };
            Some((target.map(String::from), in_span.map(String::from)))
        }

        if from.len() == 0 {
            return None;
        }
        let mut parts = from.split('=');
        let parse = (parts.next()?, parts.next().map(|s| s.trim()));
        if parts.next().is_some() {
            return None;
        }
        match parse {
            (part0, None) => Some(if let Some(level) = parse_level(part0) {
                Directive {
                    level,
                    ..Default::default()
                }
            } else {
                let (target, in_span) = parse_span_target(part0)?;
                Directive {
                    target,
                    in_span,
                    ..Default::default()
                }
            }),
            (part0, Some(part1)) => {
                let (target, in_span) = parse_span_target(part0)?;
                let level = parse_level(part1)?;

                Some(Directive {
                    level,
                    target,
                    in_span,
                })
            }
        }
    }
}

impl Default for Directive {
    fn default() -> Self {
        Self {
            level: Level::ERROR,
            target: None,
            in_span: None,
        }
    }
}

impl<F> Filter for F
where
    F: Fn(&Metadata, &span::Context) -> bool,
{
    fn enabled(&self, metadata: &Metadata, ctx: &span::Context) -> bool {
        (self)(metadata, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_directives_valid() {
        let dirs = parse_directives("crate1::mod1=error,crate1::mod2,crate2=debug");
        assert_eq!(dirs.len(), 3);
        assert_eq!(dirs[0].target, Some("crate1::mod1".to_string()));
        assert_eq!(dirs[0].level, Level::ERROR);
        assert_eq!(dirs[0].in_span, None);

        assert_eq!(dirs[1].target, Some("crate1::mod2".to_string()));
        assert_eq!(dirs[1].level, Level::ERROR);
        assert_eq!(dirs[1].in_span, None);

        assert_eq!(dirs[2].target, Some("crate2".to_string()));
        assert_eq!(dirs[2].level, Level::DEBUG);
        assert_eq!(dirs[2].in_span, None);
    }

    #[test]
    fn parse_directives_invalid_crate() {
        // test parse_directives with multiple = in specification
        let dirs = parse_directives("crate1::mod1=warn=info,crate2=debug");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].target, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, Level::DEBUG);
        assert_eq!(dirs[0].in_span, None);
    }

    #[test]
    fn parse_directives_invalid_level() {
        // test parse_directives with 'noNumber' as log level
        let dirs = parse_directives("crate1::mod1=noNumber,crate2=debug");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].target, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, Level::DEBUG);
        assert_eq!(dirs[0].in_span, None);
    }

    #[test]
    fn parse_directives_string_level() {
        // test parse_directives with 'warn' as log level
        let dirs = parse_directives("crate1::mod1=wrong,crate2=warn");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].target, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, Level::WARN);
        assert_eq!(dirs[0].in_span, None);
    }

    #[test]
    fn parse_directives_empty_level() {
        // test parse_directives with '' as log level
        let dirs = parse_directives("crate1::mod1=wrong,crate2=");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].target, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, Level::ERROR);
        assert_eq!(dirs[0].in_span, None);
    }

    #[test]
    fn parse_directives_global() {
        // test parse_directives with no crate
        let dirs = parse_directives("warn,crate2=debug");
        assert_eq!(dirs.len(), 2);
        assert_eq!(dirs[0].target, None);
        assert_eq!(dirs[0].level, Level::WARN);
        assert_eq!(dirs[1].in_span, None);

        assert_eq!(dirs[1].target, Some("crate2".to_string()));
        assert_eq!(dirs[1].level, Level::DEBUG);
        assert_eq!(dirs[1].in_span, None);
    }

    #[test]
    fn parse_directives_valid_with_spans() {
        let dirs = parse_directives("crate1::mod1{foo}=error,crate1::mod2{bar},crate2{baz}=debug");
        assert_eq!(dirs.len(), 3);
        assert_eq!(dirs[0].target, Some("crate1::mod1".to_string()));
        assert_eq!(dirs[0].level, Level::ERROR);
        assert_eq!(dirs[0].in_span, Some("foo".to_string()));

        assert_eq!(dirs[1].target, Some("crate1::mod2".to_string()));
        assert_eq!(dirs[1].level, Level::ERROR);
        assert_eq!(dirs[1].in_span, Some("bar".to_string()));

        assert_eq!(dirs[2].target, Some("crate2".to_string()));
        assert_eq!(dirs[2].level, Level::DEBUG);
        assert_eq!(dirs[2].in_span, Some("baz".to_string()));
    }

}
