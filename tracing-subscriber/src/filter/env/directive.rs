use super::super::level::{self, LevelFilter};
use super::{field, FieldMap, FilterVec};
use lazy_static::lazy_static;
use regex::Regex;
use std::{
    cmp::Ordering,
    collections::btree_set::{self, BTreeSet},
    error::Error,
    fmt,
    iter::FromIterator,
    str::FromStr,
};
use tracing_core::{span, Metadata};

/// A single filtering directive.
// TODO(eliza): add a builder for programmatically constructing directives?
#[derive(Debug, Eq, PartialEq)]
pub struct Directive {
    target: Option<String>,
    in_span: Option<String>,
    fields: FilterVec<field::Match>,
    level: LevelFilter,
}

/// A directive which will statically enable or disable a given callsite.
///
/// Unlike a dynamic directive, this can be cached by the callsite.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct StaticDirective {
    target: Option<String>,
    field_names: FilterVec<String>,
    level: LevelFilter,
}

pub(crate) trait Match {
    fn cares_about(&self, meta: &Metadata<'_>) -> bool;
    fn level(&self) -> &LevelFilter;
}

/// A set of dynamic filtering directives.
pub(crate) type Dynamics = DirectiveSet<Directive>;

/// A set of static filtering directives.
pub(crate) type Statics = DirectiveSet<StaticDirective>;

#[derive(Debug, PartialEq)]
pub(crate) struct DirectiveSet<T> {
    directives: BTreeSet<T>,
    max_level: LevelFilter,
}

pub(crate) type CallsiteMatcher = MatchSet<field::CallsiteMatch>;
pub(crate) type SpanMatcher = MatchSet<field::SpanMatch>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MatchSet<T> {
    field_matches: FilterVec<T>,
    base_level: LevelFilter,
}

/// Indicates that a string could not be parsed as a filtering directive.
#[derive(Debug)]
pub struct ParseError {
    kind: ParseErrorKind,
}

#[derive(Debug)]
enum ParseErrorKind {
    Field(Box<dyn Error + Send + Sync>),
    Level(level::ParseError),
    Other,
}

impl Directive {
    pub(super) fn has_name(&self) -> bool {
        self.in_span.is_some()
    }

    pub(super) fn has_fields(&self) -> bool {
        !self.fields.is_empty()
    }

    pub(super) fn to_static(&self) -> Option<StaticDirective> {
        if !self.is_static() {
            return None;
        }

        // TODO(eliza): these strings are all immutable; we should consider
        // `Arc`ing them to make this more efficient...
        let field_names = self.fields.iter().map(field::Match::name).collect();

        Some(StaticDirective {
            target: self.target.clone(),
            field_names,
            level: self.level.clone(),
        })
    }

    fn is_static(&self) -> bool {
        !self.has_name() && !self.fields.iter().any(field::Match::has_value)
    }

    pub(super) fn is_dynamic(&self) -> bool {
        self.has_name() || self.has_fields()
    }

    pub(crate) fn field_matcher(&self, meta: &Metadata<'_>) -> Option<field::CallsiteMatch> {
        let fieldset = meta.fields();
        let fields = self
            .fields
            .iter()
            .filter_map(
                |field::Match {
                     ref name,
                     ref value,
                 }| {
                    if let Some(field) = fieldset.field(name) {
                        let value = value.as_ref().cloned()?;
                        Some(Ok((field, value)))
                    } else {
                        Some(Err(()))
                    }
                },
            )
            .collect::<Result<FieldMap<_>, ()>>()
            .ok()?;
        Some(field::CallsiteMatch {
            fields,
            level: self.level.clone(),
        })
    }

    pub(super) fn make_tables(
        directives: impl IntoIterator<Item = Directive>,
    ) -> (Dynamics, Statics) {
        // TODO(eliza): this could be made more efficient...
        let (dyns, stats): (BTreeSet<Directive>, BTreeSet<Directive>) =
            directives.into_iter().partition(Directive::is_dynamic);
        let statics = stats
            .into_iter()
            .filter_map(|d| d.to_static())
            .chain(dyns.iter().filter_map(Directive::to_static))
            .collect();
        (Dynamics::from_iter(dyns), statics)
    }
}

impl Match for Directive {
    fn cares_about(&self, meta: &Metadata<'_>) -> bool {
        // Does this directive have a target filter, and does it match the
        // metadata's target?
        if let Some(ref target) = self.target.as_ref() {
            if !meta.target().starts_with(&target[..]) {
                return false;
            }
        }

        // Do we have a name filter, and does it match the metadata's name?
        // TODO(eliza): put name globbing here?
        if let Some(ref name) = self.in_span {
            if name != meta.name() {
                return false;
            }
        }

        // Does the metadata define all the fields that this directive cares about?
        let fields = meta.fields();
        for field in &self.fields {
            if fields.field(&field.name).is_none() {
                return false;
            }
        }

        true
    }

    fn level(&self) -> &LevelFilter {
        &self.level
    }
}

impl FromStr for Directive {
    type Err = ParseError;
    fn from_str(from: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref DIRECTIVE_RE: Regex = Regex::new(
                r"(?x)
                ^(?P<global_level>trace|TRACE|debug|DEBUG|info|INFO|warn|WARN|error|ERROR|off|OFF|[0-5])$ |
                ^
                (?: # target name or span name
                    (?P<target>[\w:]+)|(?P<span>\[[^\]]*\])
                ){1,2}
                (?: # level or nothing
                    =(?P<level>trace|TRACE|debug|DEBUG|info|INFO|warn|WARN|error|ERROR|off|OFF|[0-5])?
                )?
                $
                "
            )
            .unwrap();
            static ref SPAN_PART_RE: Regex =
                Regex::new(r#"(?P<name>\w+)?(?:\{(?P<fields>[^\}]*)\})?"#).unwrap();
            static ref FIELD_FILTER_RE: Regex =
                // TODO(eliza): this doesn't _currently_ handle value matchers that include comma
                // characters. We should fix that.
                Regex::new(r#"(?x)
                    (
                        # field name
                        [[:word:]][[[:word:]]\.]*
                        # value part (optional)
                        (?:=[^,]+)?
                    )
                    # trailing comma or EOS
                    (?:,\s?|$)
                "#).unwrap();
        }

        let caps = DIRECTIVE_RE.captures(from).ok_or_else(ParseError::new)?;

        if let Some(level) = caps
            .name("global_level")
            .and_then(|s| s.as_str().parse().ok())
        {
            return Ok(Directive {
                level,
                ..Default::default()
            });
        }

        let target = caps.name("target").and_then(|c| {
            let s = c.as_str();
            if s.parse::<LevelFilter>().is_ok() {
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
                            .map(|c| c.as_str().parse())
                            .collect::<Result<FilterVec<_>, _>>()
                    })
                    .unwrap_or_else(|| Ok(FilterVec::new()));
                Some((span, fields))
            })
            .unwrap_or_else(|| (None, Ok(FilterVec::new())));

        let level = caps
            .name("level")
            .and_then(|l| l.as_str().parse().ok())
            // Setting the target without the level enables every level for that target
            .unwrap_or(LevelFilter::TRACE);

        Ok(Directive {
            level,
            target,
            in_span,
            fields: fields?,
        })
    }
}

impl Default for Directive {
    fn default() -> Self {
        Directive {
            level: LevelFilter::OFF,
            target: None,
            in_span: None,
            fields: FilterVec::new(),
        }
    }
}

impl PartialOrd for Directive {
    fn partial_cmp(&self, other: &Directive) -> Option<Ordering> {
        // We attempt to order directives by how "specific" they are. This
        // ensures that we try the most specific directives first when
        // attempting to match a piece of metadata.

        // First, we compare based on whether a target is specified, and the
        // lengths of those targets if both have targets.
        let ordering = self
            .target
            .as_ref()
            .map(String::len)
            .cmp(&other.target.as_ref().map(String::len))
            // Next compare based on the presence of span names.
            .then_with(|| self.in_span.is_some().cmp(&other.in_span.is_some()))
            // Then we compare how many fields are defined by each
            // directive.
            .then_with(|| self.fields.len().cmp(&other.fields.len()))
            // Finally, we fall back to lexicographical ordering if the directives are
            // equally specific. Although this is no longer semantically important,
            // we need to define a total ordering to determine the directive's place
            // in the BTreeMap.
            .then_with(|| {
                self.target
                    .cmp(&other.target)
                    .then_with(|| self.in_span.cmp(&other.in_span))
                    .then_with(|| self.fields[..].cmp(&other.fields[..]))
            })
            .reverse();

        #[cfg(debug_assertions)]
        {
            if ordering == Ordering::Equal {
                debug_assert_eq!(
                    self.target, other.target,
                    "invariant violated: Ordering::Equal must imply a.target == b.target"
                );
                debug_assert_eq!(
                    self.in_span, other.in_span,
                    "invariant violated: Ordering::Equal must imply a.in_span == b.in_span"
                );
                debug_assert_eq!(
                    self.fields, other.fields,
                    "invariant violated: Ordering::Equal must imply a.fields == b.fields"
                );
            }
        }

        Some(ordering)
    }
}

impl Ord for Directive {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other)
            .expect("Directive::partial_cmp should define a total order")
    }
}

impl fmt::Display for Directive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut wrote_any = false;
        if let Some(ref target) = self.target {
            fmt::Display::fmt(target, f)?;
            wrote_any = true;
        }

        if self.in_span.is_some() || !self.fields.is_empty() {
            f.write_str("[")?;

            if let Some(ref span) = self.in_span {
                fmt::Display::fmt(span, f)?;
            }

            let mut fields = self.fields.iter();
            if let Some(field) = fields.next() {
                write!(f, "{{{}", field)?;
                for field in fields {
                    write!(f, ",{}", field)?;
                }
                f.write_str("}")?;
            }

            f.write_str("]")?;
            wrote_any = true;
        }

        if wrote_any {
            f.write_str("=")?;
        }

        fmt::Display::fmt(&self.level, f)
    }
}

impl From<LevelFilter> for Directive {
    fn from(level: LevelFilter) -> Self {
        Self {
            level,
            ..Self::default()
        }
    }
}

// === impl DirectiveSet ===

impl<T> DirectiveSet<T> {
    pub(crate) fn is_empty(&self) -> bool {
        self.directives.is_empty()
    }

    pub(crate) fn iter<'a>(&'a self) -> btree_set::Iter<'a, T> {
        self.directives.iter()
    }
}

impl<T: Ord> Default for DirectiveSet<T> {
    fn default() -> Self {
        Self {
            directives: BTreeSet::new(),
            max_level: LevelFilter::OFF,
        }
    }
}

impl<T: Match + Ord> DirectiveSet<T> {
    fn directives_for<'a>(
        &'a self,
        metadata: &'a Metadata<'a>,
    ) -> impl Iterator<Item = &'a T> + 'a {
        self.directives
            .iter()
            .filter(move |d| d.cares_about(metadata))
    }

    pub(crate) fn add(&mut self, directive: T) {
        let level = directive.level();
        if level > &self.max_level {
            self.max_level = level.clone();
        }
        let _ = self.directives.replace(directive);
    }
}

impl<T: Match + Ord> FromIterator<T> for DirectiveSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut this = Self::default();
        this.extend(iter);
        this
    }
}

impl<T: Match + Ord> Extend<T> for DirectiveSet<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let max_level = &mut self.max_level;
        let ds = iter.into_iter().inspect(|d| {
            let level = d.level();
            if level > &*max_level {
                *max_level = level.clone();
            }
        });
        self.directives.extend(ds);
    }
}

// === impl Dynamics ===

impl Dynamics {
    pub(crate) fn matcher(&self, metadata: &Metadata<'_>) -> Option<CallsiteMatcher> {
        let mut base_level = None;
        let field_matches = self
            .directives_for(metadata)
            .filter_map(|d| {
                if let Some(f) = d.field_matcher(metadata) {
                    return Some(f);
                }
                match base_level {
                    Some(ref b) if &d.level > b => base_level = Some(d.level.clone()),
                    None => base_level = Some(d.level.clone()),
                    _ => {}
                }
                None
            })
            .collect();

        if let Some(base_level) = base_level {
            Some(CallsiteMatcher {
                field_matches,
                base_level,
            })
        } else if !field_matches.is_empty() {
            Some(CallsiteMatcher {
                field_matches,
                base_level: base_level.unwrap_or(LevelFilter::OFF),
            })
        } else {
            None
        }
    }
}

// === impl Statics ===

impl Statics {
    pub(crate) fn enabled(&self, meta: &Metadata<'_>) -> bool {
        let level = meta.level();
        self.directives_for(meta).any(|d| d.level >= *level)
    }
}

impl PartialOrd for StaticDirective {
    fn partial_cmp(&self, other: &StaticDirective) -> Option<Ordering> {
        // We attempt to order directives by how "specific" they are. This
        // ensures that we try the most specific directives first when
        // attempting to match a piece of metadata.

        // First, we compare based on whether a target is specified, and the
        // lengths of those targets if both have targets.
        let ordering = self
            .target
            .as_ref()
            .map(String::len)
            .cmp(&other.target.as_ref().map(String::len))
            // Then we compare how many field names are matched by each directive.
            .then_with(|| self.field_names.len().cmp(&other.field_names.len()))
            // Finally, we fall back to lexicographical ordering if the directives are
            // equally specific. Although this is no longer semantically important,
            // we need to define a total ordering to determine the directive's place
            // in the BTreeMap.
            .then_with(|| {
                self.target
                    .cmp(&other.target)
                    .then_with(|| self.field_names[..].cmp(&other.field_names[..]))
            })
            .reverse();

        #[cfg(debug_assertions)]
        {
            if ordering == Ordering::Equal {
                debug_assert_eq!(
                    self.target, other.target,
                    "invariant violated: Ordering::Equal must imply a.target == b.target"
                );
                debug_assert_eq!(
                    self.field_names, other.field_names,
                    "invariant violated: Ordering::Equal must imply a.field_names == b.field_names"
                );
            }
        }

        Some(ordering)
    }
}

impl Ord for StaticDirective {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other)
            .expect("StaticDirective::partial_cmp should define a total order")
    }
}

// ===== impl StaticDirective =====

impl Match for StaticDirective {
    fn cares_about(&self, meta: &Metadata<'_>) -> bool {
        // Does this directive have a target filter, and does it match the
        // metadata's target?
        if let Some(ref target) = self.target.as_ref() {
            if !meta.target().starts_with(&target[..]) {
                return false;
            }
        }

        if meta.is_event() && !self.field_names.is_empty() {
            let fields = meta.fields();
            for name in &self.field_names {
                if !fields.field(name).is_some() {
                    return false;
                }
            }
        }

        true
    }

    fn level(&self) -> &LevelFilter {
        &self.level
    }
}

impl Default for StaticDirective {
    fn default() -> Self {
        StaticDirective {
            target: None,
            field_names: FilterVec::new(),
            level: LevelFilter::ERROR,
        }
    }
}

impl fmt::Display for StaticDirective {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut wrote_any = false;
        if let Some(ref target) = self.target {
            fmt::Display::fmt(target, f)?;
            wrote_any = true;
        }

        if !self.field_names.is_empty() {
            f.write_str("[")?;

            let mut fields = self.field_names.iter();
            if let Some(field) = fields.next() {
                write!(f, "{{{}", field)?;
                for field in fields {
                    write!(f, ",{}", field)?;
                }
                f.write_str("}")?;
            }

            f.write_str("]")?;
            wrote_any = true;
        }

        if wrote_any {
            f.write_str("=")?;
        }

        fmt::Display::fmt(&self.level, f)
    }
}

// ===== impl ParseError =====

impl ParseError {
    fn new() -> Self {
        ParseError {
            kind: ParseErrorKind::Other,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ParseErrorKind::Other => f.pad("invalid filter directive"),
            ParseErrorKind::Level(ref l) => l.fmt(f),
            ParseErrorKind::Field(ref e) => write!(f, "invalid field filter: {}", e),
        }
    }
}

impl Error for ParseError {
    fn description(&self) -> &str {
        "invalid filter directive"
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self.kind {
            ParseErrorKind::Other => None,
            ParseErrorKind::Level(ref l) => Some(l),
            ParseErrorKind::Field(ref n) => Some(n.as_ref()),
        }
    }
}

impl From<Box<dyn Error + Send + Sync>> for ParseError {
    fn from(e: Box<dyn Error + Send + Sync>) -> Self {
        Self {
            kind: ParseErrorKind::Field(e),
        }
    }
}

impl From<level::ParseError> for ParseError {
    fn from(l: level::ParseError) -> Self {
        Self {
            kind: ParseErrorKind::Level(l),
        }
    }
}

// ===== impl DynamicMatch =====

impl CallsiteMatcher {
    /// Create a new `SpanMatch` for a given instance of the matched callsite.
    pub(crate) fn to_span_match(&self, attrs: &span::Attributes<'_>) -> SpanMatcher {
        let field_matches = self
            .field_matches
            .iter()
            .map(|m| {
                let m = m.to_span_match();
                attrs.record(&mut m.visitor());
                m
            })
            .collect();
        SpanMatcher {
            field_matches,
            base_level: self.base_level.clone(),
        }
    }
}

impl SpanMatcher {
    /// Returns the level currently enabled for this callsite.
    pub(crate) fn level(&self) -> LevelFilter {
        self.field_matches
            .iter()
            .filter_map(field::SpanMatch::filter)
            .max()
            .unwrap_or_else(|| self.base_level.clone())
    }

    pub(crate) fn record_update(&self, record: &span::Record<'_>) {
        for m in &self.field_matches {
            record.record(&mut m.visitor())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn parse_directives(dirs: impl AsRef<str>) -> Vec<Directive> {
        dirs.as_ref()
            .split(',')
            .filter_map(|s| s.parse().ok())
            .collect()
    }

    fn expect_parse(dirs: impl AsRef<str>) -> Vec<Directive> {
        dirs.as_ref()
            .split(',')
            .map(|s| {
                s.parse()
                    .unwrap_or_else(|err| panic!("directive '{:?}' should parse: {}", s, err))
            })
            .collect()
    }

    #[test]
    fn directive_ordering_by_target_len() {
        // TODO(eliza): it would be nice to have a property-based test for this
        // instead.
        let mut dirs = expect_parse(
            "foo::bar=debug,foo::bar::baz=trace,foo=info,a_really_long_name_with_no_colons=warn",
        );
        dirs.sort_unstable();

        let expected = vec![
            "a_really_long_name_with_no_colons",
            "foo::bar::baz",
            "foo::bar",
            "foo",
        ];
        let sorted = dirs
            .iter()
            .map(|d| d.target.as_ref().unwrap())
            .collect::<Vec<_>>();

        assert_eq!(expected, sorted);
    }
    #[test]
    fn directive_ordering_by_span() {
        // TODO(eliza): it would be nice to have a property-based test for this
        // instead.
        let mut dirs = expect_parse("bar[span]=trace,foo=debug,baz::quux=info,a[span]=warn");
        dirs.sort_unstable();

        let expected = vec!["baz::quux", "bar", "foo", "a"];
        let sorted = dirs
            .iter()
            .map(|d| d.target.as_ref().unwrap())
            .collect::<Vec<_>>();

        assert_eq!(expected, sorted);
    }

    #[test]
    fn directive_ordering_uses_lexicographic_when_equal() {
        // TODO(eliza): it would be nice to have a property-based test for this
        // instead.
        let mut dirs = expect_parse("span[b]=debug,b=debug,a=trace,c=info,span[a]=info");
        dirs.sort_unstable();

        let expected = vec![
            ("span", Some("b")),
            ("span", Some("a")),
            ("c", None),
            ("b", None),
            ("a", None),
        ];
        let sorted = dirs
            .iter()
            .map(|d| {
                (
                    d.target.as_ref().unwrap().as_ref(),
                    d.in_span.as_ref().map(String::as_ref),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(expected, sorted);
    }

    // TODO: this test requires the parser to support directives with multiple
    // fields, which it currently can't handle. We should enable this test when
    // that's implemented.
    #[test]
    #[ignore]
    fn directive_ordering_by_field_num() {
        // TODO(eliza): it would be nice to have a property-based test for this
        // instead.
        let mut dirs = expect_parse(
            "b[{foo,bar}]=info,c[{baz,quuux,quuux}]=debug,a[{foo}]=warn,bar[{field}]=trace,foo=debug,baz::quux=info"
        );
        dirs.sort_unstable();

        let expected = vec!["baz::quux", "bar", "foo", "c", "b", "a"];
        let sorted = dirs
            .iter()
            .map(|d| d.target.as_ref().unwrap())
            .collect::<Vec<_>>();

        assert_eq!(expected, sorted);
    }

    #[test]
    fn parse_directives_ralith() {
        let dirs = parse_directives("common=trace,server=trace");
        assert_eq!(dirs.len(), 2, "\nparsed: {:#?}", dirs);
        assert_eq!(dirs[0].target, Some("common".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::TRACE);
        assert_eq!(dirs[0].in_span, None);

        assert_eq!(dirs[1].target, Some("server".to_string()));
        assert_eq!(dirs[1].level, LevelFilter::TRACE);
        assert_eq!(dirs[1].in_span, None);
    }

    #[test]
    fn parse_directives_valid() {
        let dirs = parse_directives("crate1::mod1=error,crate1::mod2,crate2=debug,crate3=off");
        assert_eq!(dirs.len(), 4, "\nparsed: {:#?}", dirs);
        assert_eq!(dirs[0].target, Some("crate1::mod1".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::ERROR);
        assert_eq!(dirs[0].in_span, None);

        assert_eq!(dirs[1].target, Some("crate1::mod2".to_string()));
        assert_eq!(dirs[1].level, LevelFilter::TRACE);
        assert_eq!(dirs[1].in_span, None);

        assert_eq!(dirs[2].target, Some("crate2".to_string()));
        assert_eq!(dirs[2].level, LevelFilter::DEBUG);
        assert_eq!(dirs[2].in_span, None);

        assert_eq!(dirs[3].target, Some("crate3".to_string()));
        assert_eq!(dirs[3].level, LevelFilter::OFF);
        assert_eq!(dirs[3].in_span, None);
    }

    #[test]

    fn parse_level_directives() {
        let dirs = parse_directives(
            "crate1::mod1=error,crate1::mod2=warn,crate1::mod2::mod3=info,\
             crate2=debug,crate3=trace,crate3::mod2::mod1=off",
        );
        assert_eq!(dirs.len(), 6, "\nparsed: {:#?}", dirs);
        assert_eq!(dirs[0].target, Some("crate1::mod1".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::ERROR);
        assert_eq!(dirs[0].in_span, None);

        assert_eq!(dirs[1].target, Some("crate1::mod2".to_string()));
        assert_eq!(dirs[1].level, LevelFilter::WARN);
        assert_eq!(dirs[1].in_span, None);

        assert_eq!(dirs[2].target, Some("crate1::mod2::mod3".to_string()));
        assert_eq!(dirs[2].level, LevelFilter::INFO);
        assert_eq!(dirs[2].in_span, None);

        assert_eq!(dirs[3].target, Some("crate2".to_string()));
        assert_eq!(dirs[3].level, LevelFilter::DEBUG);
        assert_eq!(dirs[3].in_span, None);

        assert_eq!(dirs[4].target, Some("crate3".to_string()));
        assert_eq!(dirs[4].level, LevelFilter::TRACE);
        assert_eq!(dirs[4].in_span, None);

        assert_eq!(dirs[5].target, Some("crate3::mod2::mod1".to_string()));
        assert_eq!(dirs[5].level, LevelFilter::OFF);
        assert_eq!(dirs[5].in_span, None);
    }

    #[test]
    fn parse_uppercase_level_directives() {
        let dirs = parse_directives(
            "crate1::mod1=ERROR,crate1::mod2=WARN,crate1::mod2::mod3=INFO,\
             crate2=DEBUG,crate3=TRACE,crate3::mod2::mod1=OFF",
        );
        assert_eq!(dirs.len(), 6, "\nparsed: {:#?}", dirs);
        assert_eq!(dirs[0].target, Some("crate1::mod1".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::ERROR);
        assert_eq!(dirs[0].in_span, None);

        assert_eq!(dirs[1].target, Some("crate1::mod2".to_string()));
        assert_eq!(dirs[1].level, LevelFilter::WARN);
        assert_eq!(dirs[1].in_span, None);

        assert_eq!(dirs[2].target, Some("crate1::mod2::mod3".to_string()));
        assert_eq!(dirs[2].level, LevelFilter::INFO);
        assert_eq!(dirs[2].in_span, None);

        assert_eq!(dirs[3].target, Some("crate2".to_string()));
        assert_eq!(dirs[3].level, LevelFilter::DEBUG);
        assert_eq!(dirs[3].in_span, None);

        assert_eq!(dirs[4].target, Some("crate3".to_string()));
        assert_eq!(dirs[4].level, LevelFilter::TRACE);
        assert_eq!(dirs[4].in_span, None);

        assert_eq!(dirs[5].target, Some("crate3::mod2::mod1".to_string()));
        assert_eq!(dirs[5].level, LevelFilter::OFF);
        assert_eq!(dirs[5].in_span, None);
    }

    #[test]
    fn parse_numeric_level_directives() {
        let dirs = parse_directives(
            "crate1::mod1=1,crate1::mod2=2,crate1::mod2::mod3=3,crate2=4,\
             crate3=5,crate3::mod2::mod1=0",
        );
        assert_eq!(dirs.len(), 6, "\nparsed: {:#?}", dirs);
        assert_eq!(dirs[0].target, Some("crate1::mod1".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::ERROR);
        assert_eq!(dirs[0].in_span, None);

        assert_eq!(dirs[1].target, Some("crate1::mod2".to_string()));
        assert_eq!(dirs[1].level, LevelFilter::WARN);
        assert_eq!(dirs[1].in_span, None);

        assert_eq!(dirs[2].target, Some("crate1::mod2::mod3".to_string()));
        assert_eq!(dirs[2].level, LevelFilter::INFO);
        assert_eq!(dirs[2].in_span, None);

        assert_eq!(dirs[3].target, Some("crate2".to_string()));
        assert_eq!(dirs[3].level, LevelFilter::DEBUG);
        assert_eq!(dirs[3].in_span, None);

        assert_eq!(dirs[4].target, Some("crate3".to_string()));
        assert_eq!(dirs[4].level, LevelFilter::TRACE);
        assert_eq!(dirs[4].in_span, None);

        assert_eq!(dirs[5].target, Some("crate3::mod2::mod1".to_string()));
        assert_eq!(dirs[5].level, LevelFilter::OFF);
        assert_eq!(dirs[5].in_span, None);
    }

    #[test]
    fn parse_directives_invalid_crate() {
        // test parse_directives with multiple = in specification
        let dirs = parse_directives("crate1::mod1=warn=info,crate2=debug");
        assert_eq!(dirs.len(), 1, "\nparsed: {:#?}", dirs);
        assert_eq!(dirs[0].target, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::DEBUG);
        assert_eq!(dirs[0].in_span, None);
    }

    #[test]
    fn parse_directives_invalid_level() {
        // test parse_directives with 'noNumber' as log level
        let dirs = parse_directives("crate1::mod1=noNumber,crate2=debug");
        assert_eq!(dirs.len(), 1, "\nparsed: {:#?}", dirs);
        assert_eq!(dirs[0].target, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::DEBUG);
        assert_eq!(dirs[0].in_span, None);
    }

    #[test]
    fn parse_directives_string_level() {
        // test parse_directives with 'warn' as log level
        let dirs = parse_directives("crate1::mod1=wrong,crate2=warn");
        assert_eq!(dirs.len(), 1, "\nparsed: {:#?}", dirs);
        assert_eq!(dirs[0].target, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::WARN);
        assert_eq!(dirs[0].in_span, None);
    }

    #[test]
    fn parse_directives_empty_level() {
        // test parse_directives with '' as log level
        let dirs = parse_directives("crate1::mod1=wrong,crate2=");
        assert_eq!(dirs.len(), 1, "\nparsed: {:#?}", dirs);
        assert_eq!(dirs[0].target, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::TRACE);
        assert_eq!(dirs[0].in_span, None);
    }

    #[test]
    fn parse_directives_global() {
        // test parse_directives with no crate
        let dirs = parse_directives("warn,crate2=debug");
        assert_eq!(dirs.len(), 2, "\nparsed: {:#?}", dirs);
        assert_eq!(dirs[0].target, None);
        assert_eq!(dirs[0].level, LevelFilter::WARN);
        assert_eq!(dirs[1].in_span, None);

        assert_eq!(dirs[1].target, Some("crate2".to_string()));
        assert_eq!(dirs[1].level, LevelFilter::DEBUG);
        assert_eq!(dirs[1].in_span, None);
    }

    #[test]
    fn parse_directives_valid_with_spans() {
        let dirs = parse_directives("crate1::mod1[foo]=error,crate1::mod2[bar],crate2[baz]=debug");
        assert_eq!(dirs.len(), 3, "\nparsed: {:#?}", dirs);
        assert_eq!(dirs[0].target, Some("crate1::mod1".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::ERROR);
        assert_eq!(dirs[0].in_span, Some("foo".to_string()));

        assert_eq!(dirs[1].target, Some("crate1::mod2".to_string()));
        assert_eq!(dirs[1].level, LevelFilter::TRACE);
        assert_eq!(dirs[1].in_span, Some("bar".to_string()));

        assert_eq!(dirs[2].target, Some("crate2".to_string()));
        assert_eq!(dirs[2].level, LevelFilter::DEBUG);
        assert_eq!(dirs[2].in_span, Some("baz".to_string()));
    }
}
