use std::{
    collections::HashSet,
    hash::{BuildHasherDefault, Hasher},
};

pub(crate) use tracing_core::span::Id;

#[derive(Debug)]
struct ContextId {
    id: Id,
    duplicate: bool,
}

/// `SpanStack` tracks what spans are currently executing on a thread-local basis.
///
/// A "separate current span" for each thread is a semantic choice, as each span
/// can be executing in a different thread.
#[derive(Debug, Default)]
pub(crate) struct SpanStack {
    stack: Vec<ContextId>,
    ids: HashSet<Id, BuildHasherDefault<IdHasher>>,
}

#[derive(Default)]
struct IdHasher(u64);

impl SpanStack {
    pub(crate) fn push(&mut self, id: Id) {
        let duplicate = self.stack.iter().any(|i| &i.id == &id);
        self.stack.push(ContextId { id, duplicate })
    }

    pub(crate) fn pop(&mut self, expected_id: &Id) -> Option<Id> {
        if let Some((idx, _)) = self
            .stack
            .iter()
            .enumerate()
            .rev()
            .find(|(_, ctx_id)| ctx_id.id == *expected_id)
        {
            let ContextId { id, duplicate: _ } = self.stack.remove(idx);
            // if !duplicate {
            //     self.ids.remove(&id);
            // }
            Some(id)
        } else {
            None
        }
    }

    #[inline]
    pub(crate) fn current(&self) -> Option<&Id> {
        self.stack
            .iter()
            .rev()
            .find(|context_id| !context_id.duplicate)
            .map(|context_id| &context_id.id)
    }
}

impl Hasher for IdHasher {
    fn write(&mut self, _: &[u8]) {
        unreachable!("span Id calls write_u64");
    }

    #[inline]
    fn write_u64(&mut self, id: u64) {
        self.0 = id;
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::{Id, SpanStack};

    #[test]
    fn pop_last_span() {
        let mut stack = SpanStack::default();
        let id = Id::from_u64(1);
        stack.push(id.clone());

        assert_eq!(Some(id.clone()), stack.pop(&id));
    }

    #[test]
    fn pop_first_span() {
        let mut stack = SpanStack::default();
        stack.push(Id::from_u64(1));
        stack.push(Id::from_u64(2));

        let id = Id::from_u64(1);
        assert_eq!(Some(id.clone()), stack.pop(&id));
    }
}
