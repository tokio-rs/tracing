use std::cell::RefCell;
use std::collections::HashSet;

pub(crate) use tracing_core::span::Id;

struct ContextId {
    id: Id,
    duplicate: bool,
}

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
        if &self.stack.last()?.id == expected_id {
            let ContextId { id, duplicate } = self.stack.pop()?;
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
