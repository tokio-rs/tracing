use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub mod event;
mod expectation;
pub mod field;
mod metadata;
pub mod span;
pub mod subscriber;

#[cfg(feature = "tracing-subscriber")]
pub mod layer;

#[derive(Debug, Eq, PartialEq)]
pub enum Parent {
    ContextualRoot,
    Contextual(String),
    ExplicitRoot,
    Explicit(String),
}

pub struct PollN<T, E> {
    and_return: Option<Result<T, E>>,
    finish_at: usize,
    polls: usize,
}

impl Parent {
    pub fn check_parent_name(
        &self,
        parent_name: Option<&str>,
        provided_parent: Option<tracing_core::span::Id>,
        ctx: impl std::fmt::Display,
        collector_name: &str,
    ) {
        match self {
            Parent::ExplicitRoot => {
                assert!(
                    provided_parent.is_none(),
                    "[{}] expected {} to be an explicit root, but its parent was actually {:?} (name: {:?})",
                    collector_name,
                    ctx,
                    provided_parent,
                    parent_name,
                );
            }
            Parent::Explicit(expected_parent) => {
                assert_eq!(
                    Some(expected_parent.as_ref()),
                    parent_name,
                    "[{}] expected {} to have explicit parent {}, but its parent was actually {:?} (name: {:?})",
                    collector_name,
                    ctx,
                    expected_parent,
                    provided_parent,
                    parent_name,
                );
            }
            Parent::ContextualRoot => {
                assert!(
                    provided_parent.is_none(),
                    "[{}] expected {} to have a contextual parent, but its parent was actually {:?} (name: {:?})",
                    collector_name,
                    ctx,
                    provided_parent,
                    parent_name,
                );
                assert!(
                    parent_name.is_none(),
                    "[{}] expected {} to be contextual a root, but we were inside span {:?}",
                    collector_name,
                    ctx,
                    parent_name,
                );
            }
            Parent::Contextual(expected_parent) => {
                assert!(provided_parent.is_none(),
                    "[{}] expected {} to have a contextual parent\nbut its parent was actually {:?} (name: {:?})",
                    collector_name,
                    ctx,
                    provided_parent,
                    parent_name,
                );
                assert_eq!(
                    Some(expected_parent.as_ref()),
                    parent_name,
                    "[{}] expected {} to have contextual parent {:?}, but got {:?}",
                    collector_name,
                    ctx,
                    expected_parent,
                    parent_name,
                );
            }
        }
    }
}

impl<T, E> std::future::Future for PollN<T, E>
where
    T: Unpin,
    E: Unpin,
{
    type Output = Result<T, E>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.get_mut();

        this.polls += 1;
        if this.polls == this.finish_at {
            let value = this.and_return.take().expect("polled after ready");

            Poll::Ready(value)
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

impl PollN<(), ()> {
    pub fn new_ok(finish_at: usize) -> Self {
        Self {
            and_return: Some(Ok(())),
            finish_at,
            polls: 0,
        }
    }

    pub fn new_err(finish_at: usize) -> Self {
        Self {
            and_return: Some(Err(())),
            finish_at,
            polls: 0,
        }
    }
}

#[cfg(feature = "tokio-test")]
pub fn block_on_future<F>(future: F) -> F::Output
where
    F: std::future::Future,
{
    use tokio_test::task;

    let mut task = task::spawn(future);
    loop {
        if let Poll::Ready(v) = task.poll() {
            break v;
        }
    }
}
