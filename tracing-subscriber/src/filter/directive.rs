use crate::filter::level::LevelFilter;
use std::{cmp::Ordering, fmt, iter::FromIterator};
use tracing_core::Metadata;
/// A directive which will statically enable or disable a given callsite.
///
/// Unlike a dynamic directive, this can be cached by the callsite.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct StaticDirective {
    target: Option<String>,
    field_names: FilterVec<String>,
    level: LevelFilter,
}

#[cfg(feature = "smallvec")]
pub(in crate::filter) type FilterVec<T> = smallvec::SmallVec<[T; 8]>;
#[cfg(not(feature = "smallvec"))]
pub(in crate::filter) type FilterVec<T> = Vec<T>;

#[derive(Debug, PartialEq)]
pub(in crate::filter) struct DirectiveSet<T> {
    directives: Vec<T>,
    pub(in crate::filter) max_level: LevelFilter,
}

pub(in crate::filter) trait Match {
    fn cares_about(&self, meta: &Metadata<'_>) -> bool;
    fn level(&self) -> &LevelFilter;
}

// === impl DirectiveSet ===

impl<T> DirectiveSet<T> {
    pub(crate) fn is_empty(&self) -> bool {
        self.directives.is_empty()
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, T> {
        self.directives.iter()
    }
}

impl<T: Ord> Default for DirectiveSet<T> {
    fn default() -> Self {
        Self {
            directives: Vec::new(),
            max_level: LevelFilter::OFF,
        }
    }
}

impl<T: Match + Ord> DirectiveSet<T> {
    pub(crate) fn directives(&self) -> impl Iterator<Item = &T> {
        self.directives.iter()
    }

    pub(crate) fn directives_for<'a>(
        &'a self,
        metadata: &'a Metadata<'a>,
    ) -> impl Iterator<Item = &'a T> + 'a {
        self.directives().filter(move |d| d.cares_about(metadata))
    }

    pub(crate) fn add(&mut self, directive: T) {
        // does this directive enable a more verbose level than the current
        // max? if so, update the max level.
        let level = *directive.level();
        if level > self.max_level {
            self.max_level = level;
        }
        // insert the directive into the vec of directives, ordered by
        // specificity (length of target + number of field filters). this
        // ensures that, when finding a directive to match a span or event, we
        // search the directive set in most specific first order.
        match self.directives.binary_search(&directive) {
            Ok(i) => self.directives[i] = directive,
            Err(i) => self.directives.insert(i, directive),
        }
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
        for directive in iter.into_iter() {
            self.add(directive);
        }
    }
}

// === impl Statics ===

impl DirectiveSet<StaticDirective> {
    pub(crate) fn enabled(&self, meta: &Metadata<'_>) -> bool {
        let level = meta.level();
        match self.directives_for(meta).next() {
            Some(d) => d.level >= *level,
            None => false,
        }
    }
}

// === impl StaticDirective ===

impl StaticDirective {
    pub(in crate::filter) fn new(
        target: Option<String>,
        field_names: FilterVec<String>,
        level: LevelFilter,
    ) -> Self {
        Self {
            target,
            field_names,
            level,
        }
    }
}

impl Ord for StaticDirective {
    fn cmp(&self, other: &StaticDirective) -> Ordering {
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

        ordering
    }
}

impl PartialOrd for StaticDirective {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Match for StaticDirective {
    fn cares_about(&self, meta: &Metadata<'_>) -> bool {
        // Does this directive have a target filter, and does it match the
        // metadata's target?
        if let Some(ref target) = self.target {
            if !meta.target().starts_with(&target[..]) {
                return false;
            }
        }

        if meta.is_event() && !self.field_names.is_empty() {
            let fields = meta.fields();
            for name in &self.field_names {
                if fields.field(name).is_none() {
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
