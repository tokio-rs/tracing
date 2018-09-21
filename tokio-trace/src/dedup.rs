use std::iter::Peekable;

pub trait IteratorDedup: Iterator + Sized {
    fn dedup_by<F, K>(self, f: F) -> DedupBy<Self, F, K>
    where
        F: Fn(&Self::Item) -> &K,
        K: PartialEq,
    {
        DedupBy {
            f,
            inner: self.peekable(),
            emit: true,
        }
    }
}

impl<I> IteratorDedup for I where I: Iterator + Sized {}

pub struct DedupBy<I: Iterator, F: Fn(&I::Item) -> &K, K> {
    f: F,
    inner: Peekable<I>,
    emit: bool,
}

impl<I, F, K> Iterator for DedupBy<I, F, K>
where
    I: Iterator,
    F: Fn(&I::Item) -> &K,
    K: PartialEq,
{
    type Item = <I as Iterator>::Item;
    fn next(&mut self) -> Option<<I as Iterator>::Item> {
        let f = &self.f;
        let result = if self.emit {
            self.inner.next()
        } else {
            let first = match self.inner.next() {
                None => return None,
                Some(first) => first,
            };
            self.inner.find(|item| (f)(&first) != (f)(item))
        };
        self.emit = result.as_ref().map(f) != self.inner.peek().map(f);
        result
    }
}
