use super::{
    super::level::{self, LevelFilter},
    field::{Match, ValueMatch},
    Directive, FilterVec,
};
use std::{borrow::Cow, error::Error, str::FromStr};

fn parse_level_filter(s: &str) -> Result<LevelFilter, <LevelFilter as FromStr>::Err> {
    if s.is_empty() {
        Ok(LevelFilter::TRACE)
    } else {
        s.parse()
    }
}

/// Macro to select execution path with the index of the first occurance of
/// some given reserved syntax character, or EOI if no characters present.
///
/// # Example
///
/// ```rust,ignore
/// let source: &str;
/// # source = "";
/// switch_syntax!(source => |i| {
///     '(' | ')' => println!("paren at index {}", i),
///     '[' | ']' => println!("brack at index {}", i),
///     '{' | '}' => println!("brace at index {}", i),
///     _ => println!("EOI at index {}", i),
/// });
/// ```
macro_rules! switch_syntax {
    ($haystack:expr => |$ix:ident| {
        $($($needle:tt)|+ => $expr:expr),* $(,)?
    }) => {{
        let haystack: &str = &$haystack;
        match find_syntax(haystack) {
            $((ix, $(switch_syntax!(@syntax $needle))|+) => {
                #[allow(unused_variables)]
                let $ix = ix;
                $expr
            })*
        }
    }};

    (@syntax '[') => (Some(Syntax::LBrack));
    (@syntax ']') => (Some(Syntax::RBrack));
    (@syntax '{') => (Some(Syntax::LBrace));
    (@syntax '}') => (Some(Syntax::RBrace));
    (@syntax '=') => (Some(Syntax::Equal));
    (@syntax ',') => (Some(Syntax::Comma));
    (@syntax '"') => (Some(Syntax::Quote));
    (@syntax '/') => (Some(Syntax::Slash));
    (@syntax _) => (None);
    (@syntax $other:literal) => (compile_error!(concat!("unknown syntax character `", $other, "`")));
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum Syntax {
    LBrack = b'[',
    RBrack = b']',
    LBrace = b'{',
    RBrace = b'}',
    Equal = b'=',
    Comma = b',',
    Quote = b'"',
    Slash = b'/',
}

fn find_syntax(haystack: &str) -> (usize, Option<Syntax>) {
    use Syntax::*;
    haystack
        .bytes()
        .enumerate()
        .find_map(|(i, b)| match b {
            b'[' => Some((i, LBrack)),
            b']' => Some((i, RBrack)),
            b'{' => Some((i, LBrace)),
            b'}' => Some((i, RBrace)),
            b'=' => Some((i, Equal)),
            b',' => Some((i, Comma)),
            b'"' => Some((i, Quote)),
            b'/' => Some((i, Slash)),
            _ => None,
        })
        .map_or_else(|| (haystack.len(), None), |(i, c)| (i, Some(c)))
}

#[derive(Debug)]
pub(super) enum ParseErrorKind {
    Field(Box<dyn Error + Send + Sync>),
    Level(level::ParseError),
    UnexpectedSyntax(char),
    Other,
}

/// Parse a single directive off the front of the source string.
///
/// Returns the parsed directive or a parse error, along with the index of
/// the start of the next directive, making an attempt at error recovery
/// in the face of reasonably recoverable parser errors.
///
/// # Syntax
///
/// ```text
/// target[span{field=value}]=level
/// ```
///
/// All of the directive fields are optional, and multiple `field=value` pairs
/// may be provided, so the following are also valid directives:
///
/// ```text
/// target
/// =level
/// [{}]
/// [{field=value,field=value}]
/// ```
#[allow(unused)]
pub(super) fn parse_one_directive(source: &str) -> (Result<Directive, ParseErrorKind>, usize) {
    fn try_recover_with(source: &str, at: usize, syntax: Syntax) -> usize {
        let (i, next_syntax) = find_syntax(&source[at..]);
        if next_syntax == Some(syntax) {
            at + i + 1
        } else {
            source.len()
        }
    }

    fn parse_at_span(
        source: &str,
        target: Option<String>,
    ) -> (Result<Directive, ParseErrorKind>, usize) {
        assert_eq!(source.as_bytes()[0], b'[');

        if source[1..].starts_with('"') {
            return match parse_one_quote_string(&source[1..]) {
                (None, _) => (Err(ParseErrorKind::Other), source.len()),
                (Some(quoted), after_quoted) => match source.as_bytes().get(1 + after_quoted) {
                    None => (Err(ParseErrorKind::Other), source.len()),
                    Some(&syntax @ (b'/' | b'[' | b'}' | b'"' | b',' | b'=')) => (
                        Err(ParseErrorKind::UnexpectedSyntax(syntax as char)),
                        source.len(),
                    ),
                    Some(b']') => {
                        let (parsed, after_parsed) = parse_at_level(
                            &source[1 + after_quoted + 1..],
                            Some(quoted.into()),
                            FilterVec::new(),
                            target,
                        );
                        (parsed, 1 + after_parsed + 1)
                    }
                    Some(b'{') => {
                        let (fields, after_fields) = parse_at_fields(&source[1 + after_quoted..]);
                        match fields {
                            Err(e) => (Err(e), source.len()),
                            Ok(fields) => {
                                let (parsed, after_parsed) = parse_at_level(
                                    &source[1 + after_quoted + after_fields..],
                                    Some(quoted.into()),
                                    FilterVec::new(),
                                    target,
                                );
                                (parsed, 1 + after_quoted + after_fields + after_parsed)
                            }
                        }
                    }
                    _ => (
                        Err(ParseErrorKind::Other),
                        try_recover_with(source, 1 + after_quoted, Syntax::Comma),
                    ),
                },
            };
        }

        switch_syntax!(&source[1..] => |i| {
            '/' => (Err(ParseErrorKind::UnexpectedSyntax('/')), source.len()),
            '[' => (Err(ParseErrorKind::UnexpectedSyntax('[')), source.len()),
            '}' => (Err(ParseErrorKind::UnexpectedSyntax('}')), source.len()),
            '"' => (Err(ParseErrorKind::UnexpectedSyntax('"')), source.len()),
            ',' => (Err(ParseErrorKind::UnexpectedSyntax(',')), source.len()),
            '=' => (Err(ParseErrorKind::UnexpectedSyntax('=')), source.len()),
            ']' => {
                let (parsed, after_parsed) = parse_at_level(
                    &source[1 + i + 1..],
                    Some(source[1..1 + i].trim()).filter(|s| !s.is_empty()).map(Into::into),
                    FilterVec::new(),
                    target,
                );
                (parsed, 1 + i + 1 + after_parsed)
            },
            '{' => {
                let (fields, after_fields) = parse_at_fields(&source[1 + i..]);
                match fields {
                    Err(e) => (Err(e), source.len()),
                    Ok(fields) => {
                        let (parsed, after_parsed) = parse_at_level(
                            &source[1 + i + after_fields..],
                            Some(source[1..1 + i].trim()).filter(|s| !s.is_empty()).map(Into::into),
                            fields,
                            target,
                        );
                        (parsed, 1 + i + after_fields + after_parsed)
                    }
                }
            },
            _ => (Err(ParseErrorKind::Other), source.len()),
        })
    }

    // NB: includes parsing the comma, if present
    fn parse_at_field(source: &str) -> (Result<Match, ParseErrorKind>, usize) {
        let mut cursor = 0;

        let name = if source.starts_with('"') {
            match parse_one_quote_string(source) {
                (None, _) => return (Err(ParseErrorKind::Other), source.len()),
                (Some(quoted), after_quoted) => {
                    cursor = after_quoted;
                    quoted.into()
                }
            }
        } else {
            let (syntax_at, _) = find_syntax(source);
            cursor = syntax_at;
            source[..cursor].trim().into()
        };

        match source.as_bytes().get(cursor) {
            None => (Err(ParseErrorKind::Other), source.len()),
            Some(b',') => (Ok(Match { name, value: None }), cursor + 1),
            Some(b'}') => (Ok(Match { name, value: None }), cursor),
            Some(b'=') => {
                cursor += 1;
                if source[cursor..].starts_with('"') {
                    match parse_one_quote_string(&source[cursor..]) {
                        (None, _) => (Err(ParseErrorKind::Other), source.len()),
                        (Some(quoted), after_quoted) => {
                            // Prefer env filter syntax error
                            cursor += after_quoted;
                            match source.as_bytes().get(cursor) {
                                None => (Err(ParseErrorKind::Other), source.len()),
                                Some(b',') => (
                                    Ok(Match { name, value: None }).and_then(|mut m| {
                                        m.value = Some(
                                            quoted
                                                .parse::<ValueMatch>()
                                                .map_err(|e| ParseErrorKind::Field(e.into()))?,
                                        );
                                        Ok(m)
                                    }),
                                    cursor + 1,
                                ),
                                Some(b'}') => (
                                    Ok(Match { name, value: None }).and_then(|mut m| {
                                        m.value = Some(
                                            quoted
                                                .parse::<ValueMatch>()
                                                .map_err(|e| ParseErrorKind::Field(e.into()))?,
                                        );
                                        Ok(m)
                                    }),
                                    cursor,
                                ),
                                _ => (Err(ParseErrorKind::Other), source.len()),
                            }
                        }
                    }
                } else {
                    switch_syntax!(&source[cursor..] => |i| {
                        '/' => (Err(ParseErrorKind::UnexpectedSyntax('/')), source.len()),
                        '[' => (Err(ParseErrorKind::UnexpectedSyntax('[')), source.len()),
                        '"' => (Err(ParseErrorKind::UnexpectedSyntax('"')), source.len()),
                        '{' => (Err(ParseErrorKind::UnexpectedSyntax('{')), source.len()),
                        '=' => (Err(ParseErrorKind::UnexpectedSyntax('=')), source.len()),
                        ']' => (Err(ParseErrorKind::UnexpectedSyntax(']')), source.len()),
                        '}' => match source[cursor..][..i].trim().parse() {
                            Ok(value) => (Ok(Match { name, value: Some(value) }), cursor + i),
                            Err(e) => (Err(ParseErrorKind::Field(e.into())), source.len()),
                        },
                        ',' => match source[cursor..][..i].trim().parse() {
                            Ok(value) => (Ok(Match { name, value: Some(value) }), cursor + i + 1),
                            Err(e) => (Err(ParseErrorKind::Field(e.into())), source.len()),
                        },
                        _ => (Err(ParseErrorKind::Other), source.len()),
                    })
                }
            }
            _ => (Err(ParseErrorKind::Other), source.len()),
        }
    }

    // NB: includes parsing the rbrack, if present
    fn parse_at_fields(source: &str) -> (Result<FilterVec<Match>, ParseErrorKind>, usize) {
        assert_eq!(source.as_bytes()[0], b'{');

        let mut fields = FilterVec::new();
        let mut cursor = 1;
        while source.as_bytes().get(cursor) != Some(&b'}') {
            let (parsed, after_parsed) = parse_at_field(&source[cursor..]);
            match parsed {
                Err(e) => {
                    return (
                        Err(e),
                        try_recover_with(source, cursor + after_parsed, Syntax::RBrace),
                    )
                }
                Ok(field) => {
                    fields.push(field);
                    cursor += after_parsed;
                }
            }
        }
        if source.as_bytes().get(cursor + 1) == Some(&b']') {
            (Ok(fields), cursor + 1 + 1)
        } else {
            (Err(ParseErrorKind::Other), source.len())
        }
    }

    fn parse_at_level(
        source: &str,
        in_span: Option<String>,
        fields: FilterVec<Match>,
        target: Option<String>,
    ) -> (Result<Directive, ParseErrorKind>, usize) {
        if !source.starts_with('=') {
            // NB: parse_at_comma handles syntax/comma after the level
            return (
                Ok(Directive {
                    in_span,
                    fields,
                    target,
                    level: LevelFilter::TRACE,
                }),
                0,
            );
        }

        if source[1..].starts_with('"') {
            return match parse_one_quote_string(&source[1..]) {
                (None, _) => (Err(ParseErrorKind::Other), source.len()),
                (Some(quoted), after_quoted) => match parse_level_filter(&quoted) {
                    // NB: parse_at_comma handles syntax/comma after the level
                    Ok(level) => (
                        Ok(Directive {
                            in_span,
                            fields,
                            target,
                            level,
                        }),
                        1 + after_quoted,
                    ),
                    Err(err) => (Err(ParseErrorKind::Level(err)), 1 + after_quoted),
                },
            };
        }

        switch_syntax!(&source[1..] => |i| {
            // Prefer syntax error over potential level parse error
            '/' => (Err(ParseErrorKind::UnexpectedSyntax('/')), source.len()),
            '[' => (Err(ParseErrorKind::UnexpectedSyntax('[')), source.len()),
            '"' => (Err(ParseErrorKind::UnexpectedSyntax('"')), source.len()),
            '{' => (Err(ParseErrorKind::UnexpectedSyntax('{')), source.len()),
            '}' => (Err(ParseErrorKind::UnexpectedSyntax('}')), 1 + i),
            '=' => (Err(ParseErrorKind::UnexpectedSyntax('=')), 1 + i),
            ']' => (Err(ParseErrorKind::UnexpectedSyntax(']')), 1 + i),
            ',' | _ => match parse_level_filter(&source[1..1 + i]) {
                Ok(level) => (
                    Ok(Directive {
                        in_span,
                        fields,
                        target,
                        level,
                    }),
                    1 + i,
                ),
                Err(err) => (Err(ParseErrorKind::Level(err)), 1 + i),
            },
        })
    }

    fn parse_at_comma(
        source: &str,
        i: usize,
        directive: Result<Directive, ParseErrorKind>,
    ) -> (Result<Directive, ParseErrorKind>, usize) {
        match source[i..].as_bytes().get(0) {
            None => (directive, i),
            Some(b',') => (directive, i + 1),
            Some(&syntax @ (b'[' | b'{' | b'"' | b'/')) => (
                Err(ParseErrorKind::UnexpectedSyntax(syntax as char)),
                source.len(),
            ),
            Some(&syntax @ (b']' | b'}' | b'=')) => (
                Err(ParseErrorKind::UnexpectedSyntax(syntax as char)),
                try_recover_with(source, i + 1, Syntax::Comma),
            ),
            _ => (
                Err(ParseErrorKind::Other),
                try_recover_with(source, i, Syntax::Comma),
            ),
        }
    }

    if source.starts_with('"') {
        return match parse_one_quote_string(source) {
            (None, after) => (Err(ParseErrorKind::Other), after),
            (Some(quoted), after_quoted) => match source.as_bytes().get(after_quoted) {
                None => (
                    Ok(Directive {
                        in_span: None,
                        fields: FilterVec::new(),
                        target: Some(quoted.to_string()),
                        level: LevelFilter::TRACE,
                    }),
                    after_quoted,
                ),
                Some(b',') => (
                    Ok(Directive {
                        in_span: None,
                        fields: FilterVec::new(),
                        target: Some(quoted.to_string()),
                        level: LevelFilter::TRACE,
                    }),
                    after_quoted + 1,
                ),
                Some(&syntax @ (b'/' | b'{')) => (
                    Err(ParseErrorKind::UnexpectedSyntax(syntax as char)),
                    source.len(),
                ),
                Some(&syntax @ (b']' | b'}')) => (
                    Err(ParseErrorKind::UnexpectedSyntax(syntax as char)),
                    try_recover_with(source, after_quoted + 1, Syntax::Comma),
                ),
                Some(b'[') => {
                    let (parsed, after_parsed) =
                        parse_at_span(&source[after_quoted..], Some(quoted.into()));
                    parse_at_comma(source, after_parsed, parsed)
                }
                Some(b'=') => {
                    let (parsed, after_parsed) = parse_at_level(
                        &source[after_quoted..],
                        None,
                        FilterVec::new(),
                        Some(quoted.into()),
                    );
                    parse_at_comma(source, after_parsed, parsed)
                }
                Some(b'"') => (Err(ParseErrorKind::UnexpectedSyntax('"')), source.len()),
                _ => (
                    Err(ParseErrorKind::Other),
                    try_recover_with(source, after_quoted, Syntax::Comma),
                ),
            },
        };
    }

    switch_syntax!(source => |i| {
        '/' => (Err(ParseErrorKind::UnexpectedSyntax('/')), source.len()),
        '"' => (Err(ParseErrorKind::UnexpectedSyntax('}')), source.len()),
        '{' => (Err(ParseErrorKind::UnexpectedSyntax('{')), source.len()),
        '}' => (Err(ParseErrorKind::UnexpectedSyntax('}')), try_recover_with(source, i + 1, Syntax::Comma)),
        ']' => (Err(ParseErrorKind::UnexpectedSyntax(']')), try_recover_with(source, i + 1, Syntax::Comma)),
        ',' => (parse_one_directive(&source[..i]).0, i + 1),
        '[' => {
            let (parsed, after_parsed) = parse_at_span(
                &source[i..],
                Some(source[..i].trim()).filter(|s| !s.is_empty()).map(Into::into),
            );
            parse_at_comma(source, i + after_parsed, parsed)
        },
        '=' => {
            let (parsed, after_parsed) = parse_at_level(
                &source[i..],
                None,
                FilterVec::new(),
                Some(source[..i].trim()).filter(|s| !s.is_empty()).map(Into::into),
            );
            parse_at_comma(source, i + after_parsed, parsed)
        },
        _ => match parse_level_filter(source.trim()) {
            Ok(level) => (Ok(Directive {
                in_span: None,
                fields: FilterVec::new(),
                target: None,
                level,
            }), source.len()),
            Err(_) => (Ok(Directive {
                in_span: None,
                fields: FilterVec::new(),
                target: Some(source.trim().to_string()),
                level: LevelFilter::TRACE,
            }), source.len()),
        }
    })
}

/// Parse a single quote string off the front of the source string.
///
/// Returns the unquoted string, if terminated, as well as the index after the
/// parsed string.
///
/// Currently has no escape syntax.
fn parse_one_quote_string(source: &str) -> (Option<Cow<'_, str>>, usize) {
    assert!(source.starts_with('"'));
    match source[1..].find('"') {
        None => (None, source.len()),
        Some(i) => (Some(source[1..1 + i].into()), 1 + i + 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_parsing() {
        macro_rules! check {
            ($source:expr => $expected:expr, $expected_after:expr) => {{
                let source: &str = $source;
                let expected: Option<&str> = $expected;
                let expected_after: &str = $expected_after;
                let (parsed, parsed_len) = parse_one_quote_string(source);
                assert_eq!(
                    (parsed, &source[parsed_len..]),
                    (expected.map(Into::into), expected_after),
                    "\nexpected {:?} to parse",
                    source,
                );
            }};
        }

        check!(r#""hello""# => Some("hello"), "");
        check!(r#""hello"world"# => Some("hello"), "world");
        check!(r#""hello"# => None, "");
    }

    #[test]
    fn test_directive_parsing() {
        macro_rules! check_pass {
            ($source:expr => $expected:expr, $expected_after:expr) => {{
                let source: &str = $source;
                let expected: Directive = $expected;
                let expected_after: &str = $expected_after;
                let (parsed, parsed_len) = parse_one_directive(source);
                let parsed = parsed.unwrap_or_else(|e| {
                    panic!("Failed to parse directive {:?} with err {:?}", source, e)
                });
                assert_eq!(
                    (parsed, &source[parsed_len..]),
                    (expected, expected_after),
                    "\nexpected {:?} to parse",
                    source,
                );
            }};
        }

        macro_rules! collect {
            [$($expr:expr),* $(,)?] => {
                IntoIterator::into_iter([$($expr),*]).collect()
            }
        }

        check_pass!("hello" => Directive {
            in_span: None,
            fields: collect![],
            target: Some("hello".into()),
            level: LevelFilter::TRACE
        }, "");

        check_pass!("info" => Directive {
            in_span: None,
            fields: collect![],
            target: None,
            level: LevelFilter::INFO
        }, "");

        check_pass!("INFO" => Directive {
            in_span: None,
            fields: collect![],
            target: None,
            level: LevelFilter::INFO
        }, "");

        check_pass!("hello=debug" => Directive {
            in_span: None,
            fields: collect![],
            target: Some("hello".into()),
            level: LevelFilter::DEBUG
        }, "");

        check_pass!("hello,std::option" => Directive {
            in_span: None,
            fields: collect![],
            target: Some("hello".into()),
            level: LevelFilter::TRACE
        }, "std::option");

        check_pass!("error,hello=warn" => Directive {
            in_span: None,
            fields: collect![],
            target: None,
            level: LevelFilter::ERROR
        }, "hello=warn");

        check_pass!("tokio::net=info" => Directive {
            in_span: None,
            fields: collect![],
            target: Some("tokio::net".into()),
            level: LevelFilter::INFO
        }, "");

        check_pass!("my_crate[span_a]=trace" => Directive {
            in_span: Some("span_a".into()),
            fields: collect![],
            target: Some("my_crate".into()),
            level: LevelFilter::TRACE
        }, "");

        check_pass!("[span_b{name}]" => Directive {
            in_span: Some("span_b".into()),
            fields: collect![Match { name: "name".into(), value: None }],
            target: None,
            level: LevelFilter::TRACE
        }, "");

        check_pass!(r#"[span_b{name=bob}]"# => Directive {
            in_span: Some("span_b".into()),
            fields: collect![Match { name: "name".into(), value: Some("bob".parse().unwrap()) }],
            target: None,
            level: LevelFilter::TRACE
        }, "");

        check_pass!(r#"[span_b{name="bob"}]"# => Directive {
            in_span: Some("span_b".into()),
            fields: collect![Match { name: "name".into(), value: Some("bob".parse().unwrap()) }],
            target: None,
            level: LevelFilter::TRACE
        }, "");
    }
}
