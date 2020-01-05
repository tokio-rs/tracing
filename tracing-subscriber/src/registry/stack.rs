use std::cell::RefCell;
use std::collections::HashSet;

pub(crate) use tracing_core::span::Id;

struct ContextId {
    id: Id,
    duplicate: bool,
}

/// `SpanStack` tracks what spans are currently executing on a thread-local basis.
///
/// A "separate current span" for each thread is a semantic choice, as each span
/// can be executing in a different thread.
pub(crate) struct SpanStack {
    stack: Vec<ContextId>,
    ids: HashSet<Id>,
}

impl SpanStack {
    pub(crate) fn new() -> Self {
        SpanStack {
            stack: vec![],
            ids: HashSet::new(),
        }
    }

    pub(crate) fn push(&mut self, id: Id) {
        let duplicate = self.ids.contains(&id);
        if !duplicate {
            self.ids.insert(id.clone());
        }
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
            let ContextId { id, duplicate } = self.stack.remove(idx);
            if !duplicate {
                self.ids.remove(&id);
            }
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

thread_local! {
    static CONTEXT: RefCell<SpanStack> = RefCell::new(SpanStack::new());
}

#[cfg(test)]
mod tests {
    use super::{Id, SpanStack};

    #[test]
    fn pop_last_span() {
        let mut stack = SpanStack::new();
        let id = Id::from_u64(1);
        stack.push(id.clone());

        assert_eq!(Some(id.clone()), stack.pop(&id));
    }

    #[test]
    fn pop_first_span() {
        let mut stack = SpanStack::new();
        stack.push(Id::from_u64(1));
        stack.push(Id::from_u64(2));

        let id = Id::from_u64(1);
        assert_eq!(Some(id.clone()), stack.pop(&id));
    }
}
