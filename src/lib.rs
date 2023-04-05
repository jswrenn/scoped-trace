//! Capture scoped backtraces.
//!
//! Use [`Trace::root`] to define the upper unwinding bound of an execution
//! trace, and [`Trace::leaf`] to define its lower bounds (the points at which
//! backtraces are collected). The resulting traces are trees, since a single
//! invocation of [`Trace::root`] may have multiple sub-invocations of
//! [`Trace::leaf`].
//!
//! [`Trace::root`]: crate::Trace::root
//! [`Trace::leaf`]: crate::Trace::leaf
//!
//! For example, running this program:
//! ```
//! use scoped_trace::Trace;
//!
//! fn main() {
//!     let (_, trace) = Trace::root(|| foo());
//!     println!("{trace}");
//! }
//!
//! fn foo() {
//!     bar();
//!     baz();
//! }
//!
//! fn bar() {
//!     Trace::leaf();
//! }
//!
//! fn baz() {
//!     Trace::leaf();
//! }
//! ```
//! ...will produce an output like:
//! ```text
//! ╼ inlining::main::{{closure}} at example.rs:4:38
//!   ├╼ inlining::foo at example.rs:9:5
//!   │  └╼ inlining::bar at example.rs:14:5
//!   └╼ inlining::foo at example.rs:10:5
//!      └╼ inlining::baz at example.rs:18:5
//! ```

use backtrace::BacktraceFrame;
use std::{cell::Cell, ffi::c_void, fmt, ptr::{self, NonNull}};

mod symbol;
mod tree;

use symbol::Symbol;
use tree::Tree;

type Backtrace = Vec<BacktraceFrame>;
type SymbolTrace = Vec<Symbol>;

/// A [`Frame`] in an intrusive, doubly-linked tree of [`Frame`]s.
pub(crate) struct Frame {
    // The location associated with this frame.
    inner_addr: *const c_void,

    // The kind of this frame — either a root or a node.
    parent: Option<NonNull<Frame>>,
}

/// The ambiant backtracing context.
pub(crate) struct Context {
    /// The address of [`Trace::root`] establishes an upper unwinding bound on
    /// the backtraces in `Trace`.
    active_frame: Cell<Option<NonNull<Frame>>>,
    /// The collection of backtraces collected beneath the invocation of
    /// [`Trace::root`].
    trace: Cell<Option<Trace>>,
}

impl Context {
    pub(crate) unsafe fn with_current<F, R>(f: F) -> R
    where
        F: FnOnce(&Self) -> R,
    {
        std::thread_local! {
            #[allow(clippy::declare_interior_mutable_const)]
            static CONTEXT: Context = const { 
                Context {
                    active_frame: Cell::new(None),
                    trace: Cell::new(None),
                }
            };
        }
        CONTEXT.with(f)
    }

    pub(crate) unsafe fn with_current_frame<F, R>(f: F) -> R
    where
        F: FnOnce(&Cell<Option<NonNull<Frame>>>) -> R,
    {
        Self::with_current(|context| f(&context.active_frame))
    }

    pub(crate) fn with_current_collector<F, R>(f: F) -> R
    where
        F: FnOnce(&Cell<Option<Trace>>) -> R,
    {
        unsafe { Self::with_current(|context| f(&context.trace)) }
    }
}


/// An tree execution trace.
///
/// This trace is constructed by calling [`Trace::root`] with a closure, and
/// includes all execution Trace of that closure that end in an invocation of
/// [`Trace::leaf`].
#[derive(Clone)]
pub struct Trace {
    // The linear backtraces that comprise this trace. These linear traces can
    // be re-knitted into a tree.
    backtraces: Vec<Backtrace>,
}

impl Trace {
    /// Invokes `f`, returning both its result and the collection of backtraces
    /// captured at each sub-invocation of [`Trace::leaf`].
    #[inline(never)]
    pub fn capture<F, R>(f: F) -> (R, Trace)
    where
        F: FnOnce() -> R,
    {
        let collector = Trace {
            backtraces: vec![],
        };

        let previous = Context::with_current_collector(|current| {
            current.replace(Some(collector))
        });

        let result = Trace::root(f);

        let collector =
            Context::with_current_collector(|current| {
                current.replace(previous)
            }).unwrap();

        (result, collector)
    }

    #[inline(never)]
    pub fn root<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        unsafe {
            let mut frame = Frame {
                inner_addr: Self::root::<F, R> as *const c_void,
                parent: None,
            };

            Context::with_current_frame(|current| {
                frame.parent = current.take();
                current.set(Some(NonNull::from(&frame)));
            });

            let _restore = crate::defer(|| {
                Context::with_current_frame(|current| {
                    current.set(frame.parent);
                });
            });

            f()
        }
    }

    /// If this is a sub-invocation of [`Trace::root`], capture a backtrace.
    ///
    /// The captured backtrace will be returned by [`Trace::root`].
    ///
    /// Invoking this function does nothing when it is not a sub-invocation
    /// [`Trace::root`].
    // This function is marked `#[inline(never)]` to ensure that it gets a distinct `Frame` in the
    // backtrace, below which frames should not be included in the backtrace (since they reflect the
    // internal implementation details of this crate).
    #[inline(never)]
    pub fn leaf() {
        unsafe {
        Context::with_current(|context_cell| {
            if let Some(mut collector) = context_cell.trace.take() {
                let mut frames = vec![];
                let mut above_leaf = false;
 
                if let Some(active_frame) = context_cell.active_frame.get() {
                    let active_frame = active_frame.as_ref();

                    backtrace::trace(|frame| {
                        println!("boom!");
                        let below_root = !ptr::eq(frame.symbol_address(), active_frame.inner_addr);

                        // only capture frames above `Trace::leaf()` and below
                        // `Trace::root_inner()`.
                        if dbg!(above_leaf) && dbg!(below_root) {
                            frames.push(frame.to_owned().into());
                        }

                        if ptr::eq(frame.symbol_address(), Self::leaf as *const _) {
                            above_leaf = true;
                        }

                        // only continue unwinding if we're below `Trace::root`
                        dbg!(below_root)
                    });
                }
                collector.backtraces.push(frames);
                context_cell.trace.set(Some(collector));
            }
        });
        }
    }
}

impl fmt::Display for Trace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Tree::from_trace(self.clone()).fmt(f)
    }
}

fn defer<F: FnOnce() -> R, R>(f: F) -> impl Drop {
    use std::mem::ManuallyDrop;

    struct Defer<F: FnOnce() -> R, R>(ManuallyDrop<F>);

    impl<F: FnOnce() -> R, R> Drop for Defer<F, R> {
        #[inline(always)]
        fn drop(&mut self) {
            unsafe {
                ManuallyDrop::take(&mut self.0)();
            }
        }
    }

    Defer(ManuallyDrop::new(f))
}
