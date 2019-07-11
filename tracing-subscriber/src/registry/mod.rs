use std::{
    any::Any,
    ops::Deref,
    ops::DerefMut,
};
use tracing_core::{span, Subscriber, Metadata};

pub trait Registry: Subscriber {
    type Data: SpanData;
    type Ref: Deref<Target = Self::Data>;
    type RefMut: Deref<Target = Self::Data> + DerefMut;

    fn current(&self) -> Option<Self::Ref> {
        let curr = self.current_span();
        let id = curr.id()?;
        self.span(id)
    }

    fn current_mut(&self) -> Option<Self::RefMut> {
        let curr = self.current_span();
        let id = curr.id()?;
        self.span_mut(id)
    }

    fn span(&self, id: &span::Id) -> Option<Self::Ref>;
    fn span_mut(&self, id: &span::Id) -> Option<Self::RefMut>;

    fn parents<'a>(&'a self, span: &span::Id) -> Parents<'a, Self>
    where
        Self: Sized,
    {
        Parents {
            registry: self,
            next: Some(span.clone()),
        }
    }
}

pub trait SpanData {
    fn parent(&self) -> Option<&span::Id>;
    fn metadata(&self) -> &'static Metadata<'static>;
    fn get<T: Any>(&self) -> Option<&T>;
    fn get_mut<T: Any>(&mut self) -> Option<&mut T>;
}

pub struct Parents<'registry, R> {
    registry: &'registry R,
    next: Option<span::Id>,
}

impl<'registry, R> Iterator for Parents<'registry, R>
where
    R: Registry,
{
    type Item = R::Ref;
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.next.as_ref()?;
        let span = self.registry.span(next)?;
        self.next = span.parent().cloned();
        Some(span)
    }
}
