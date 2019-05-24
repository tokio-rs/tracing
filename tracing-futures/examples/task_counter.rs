//! A demonstration of how a subscriber might use the spans created by the
//! `tokio_trace_futures::spawn!` macro to track the number of
//! currently-executing tasks.
//!
//! Note that this subscriber implementation is not optimized for production
//! use. It is intended only for demonstration purposes.

extern crate futures;
extern crate tokio;
#[macro_use]
extern crate tokio_trace;
#[macro_use]
extern crate tokio_trace_futures;

use futures::future::{self, Future};

use tokio_trace::{
    field::{Field, Visit},
    span,
    subscriber::{self, Subscriber},
    Event, Metadata,
};

use std::{
    collections::HashMap,
    cell::Cell,
    fmt,
    sync::{
        atomic::{self, AtomicUsize, Ordering}, RwLock,
    },
};

struct TaskCounter {
    ids: AtomicUsize,
    tasks: RwLock<HashMap<u64, ActiveTask>>,
    task_list: RwLock<HashMap<&'static str, AtomicUsize>>,
}

struct ActiveTask {
    name: &'static str,
    ref_count: AtomicUsize,
}

thread_local! {
    static CURRENT_TASK: Cell<Option<u64>> = Cell::new(None);
}

impl Subscriber for TaskCounter {
    fn register_callsite(&self, meta: &Metadata) -> subscriber::Interest {
        if meta.is_event() {
            return subscriber::Interest::always();
        }
        let is_task = meta.fields().iter().any(|key| key.name().starts_with("tokio.task"));
        if is_task {
            let mut task_list = self.task_list.write().unwrap();
            task_list.insert(meta.name(), AtomicUsize::new(0));
            subscriber::Interest::always()
        } else {
            subscriber::Interest::never()
        }
    }

    fn new_span(&self, new_span: &span::Attributes) -> span::Id {
        let meta = new_span.metadata();
        let task = ActiveTask {
            name: meta.name(),
            ref_count: AtomicUsize::from(1),
        };
        let id = self.ids.fetch_add(1, Ordering::SeqCst) as u64;

        let task_list = self.task_list.read().unwrap();
        task_list.get(meta.name()).map(|count| count.fetch_add(1, Ordering::Relaxed));

        self.print_ctx(format_args!("task `{}` (id={}) spawned", meta.name(), id));
        let mut tasks = self.tasks.write().unwrap();
        tasks.insert(id, task);
        span::Id::from_u64(id)
    }

    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {
        // unimplemented
    }

    fn record(&self, _: &span::Id, _: &span::Record) {
        // unimplemented
    }

    fn event(&self, event: &Event) {
        struct FmtEvent<'a>(&'a Event<'a>);
        struct Recorder<'a, 'b> {
            f: &'a mut fmt::Formatter<'b>,
            pad: bool,
        }
        impl<'a> fmt::Display for FmtEvent<'a> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let mut rec = Recorder {
                    f,
                    pad: false,
                };
                self.0.record(&mut rec);
                Ok(())
            }
        }

        impl<'a, 'b> Visit for Recorder<'a, 'b> {
            fn record_str(&mut self, field: &Field, value: &str) {
                if field.name() == "message" {
                    self.record_debug(field, &format_args!("{}", value))
                } else {
                    self.record_debug(field, &value)
                }
            }

            fn record_debug(&mut self, field: &Field, value: &fmt::Debug) {
                if self.pad {
                    write!(self.f, "; ").unwrap();
                }
                if field.name() == "message" {
                    write!(self.f, "{:?}", value).unwrap();
                } else {
                    write!(self.f, "{}={:?}", field, value).unwrap();
                }
                self.pad = true;
            }
        }

        self.print_ctx(FmtEvent(event));
    }

    fn enabled(&self, meta: &Metadata) -> bool {
        meta.is_event() || meta.fields().iter().any(|f| f.name().starts_with("tokio.task"))
    }

    fn enter(&self, span: &span::Id) {
        CURRENT_TASK.with(|current| current.set(Some(span.into_u64())));
    }

    fn exit(&self, _span: &span::Id) {
        CURRENT_TASK.with(|current| current.set(None));
    }

    fn clone_span(&self, span: &span::Id) -> span::Id {
        let num = span.into_u64();
        let tasks = self.tasks.read().unwrap();
        if let Some(task) = tasks.get(&num) {
            task.ref_count.fetch_add(1, Ordering::Relaxed);
        }
        span::Id::from_u64(num)
    }

    fn drop_span(&self, span: span::Id) {
        let num = span.into_u64();
        let is_closing = {
            let tasks = match self.tasks.read() {
                Ok(t) => t,
                Err(_) => return, // don't panic in drop!
            };
            if let Some(task) = tasks.get(&num) {
                task.ref_count.fetch_sub(1, Ordering::Relaxed) == 1
            } else {
                false
            }
        };
        if is_closing {
            atomic::fence(Ordering::Release);
            if let Some(task) = self.tasks.write().ok().and_then(|mut tasks| tasks.remove(&num)) {
                if let Ok(task_list) = self.task_list.read() {
                    task_list.get(task.name).map(|count| count.fetch_sub(1, Ordering::Release));
                }
                self.print_ctx(format_args!("task `{}` (id={}) completed", task.name, num));
            }
        }
    }
}

impl TaskCounter {
    pub fn new() -> Self {
        Self {
            ids: AtomicUsize::from(1),
            tasks: RwLock::new(HashMap::new()),
            task_list: RwLock::new(HashMap::new()),
        }
    }

    fn print_ctx(&self, msg: impl fmt::Display) {

        static LONGEST: AtomicUsize = AtomicUsize::new(20);
        let curr  = self.tasks.read().ok().and_then(|tasks| {
            let id = CURRENT_TASK.try_with(|id| id.get()).ok()?;
            id
                .and_then(|id| Some((tasks.get(&id)?, id)))
                .map(|(task, id)| format!("{} (id={})", task.name, id))
                .or_else(|| Some(String::new()))
        }).unwrap_or_else(|| String::from("panicking"));

        let lock = self.task_list.read().ok();
        let mut task_list = lock.iter().flat_map(|ts| ts.iter());
        let mut counts = String::new();
        if let Some((name, count)) = task_list.next() {
            use fmt::Write;

            write!(&mut counts, "{{ {}: {}", name, count.load(Ordering::Acquire)).unwrap();
            for (name, count) in task_list {
                write!(&mut counts, ", {}: {}", name, count.load(Ordering::Acquire)).unwrap();
            }
            write!(&mut counts, "}} ").unwrap();
        }

        let len = curr.len();
        let longest = LONGEST.load(Ordering::Relaxed);
        let padding = if len > longest {
            LONGEST.store(len, Ordering::Relaxed);
            len
        } else {
            longest
        };

        println!(
            "{}{:<padding$} | {msg}",
            counts,
            curr,
            padding = padding,
            msg = msg,
        );
    }
}

fn main() {
    fn parent_task(how_many: usize) -> impl Future<Item = (), Error = ()> {
        future::lazy(move || {
            // Since this span does not directly correspond to a spawned task,
            // it will be ignored.
            debug_span!("spawning subtasks", tasks = how_many).enter(|| {
                for i in 1..=how_many {
                    debug!(message = "spawning subtask", number = i);
                    spawn!(subtask(i), number = i);
                }
            });

            Ok(())
        })
        .map(|_| {
            info!("all subtasks spawned");
        })
    }

    fn subtask(number: usize) -> impl Future<Item = (), Error = ()> {
        future::lazy(move || {
            debug!("polling subtask...");
            Ok(number)
        })
        .map(|i| trace!(i = i))
    }

    tokio_trace::subscriber::with_default(TaskCounter::new(), || {
        tokio::run(future::lazy(|| {
            spawn!(parent_task(10))
        }))
    });

    // counters.print_counters();
}
