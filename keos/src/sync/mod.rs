//! Useful synchronization primitives.
//! Support similar synchronization primitives like `std::sync`.
//!
//! Belows sections is "copied" from `std::sync`'s documentation.
//!
//! ## The need for synchronization
//!
//! Conceptually, a Rust program is a series of operations which will
//! be executed on a computer. The timeline of events happening in the
//! program is consistent with the order of the operations in the code.
//!
//! Consider the following code, operating on some global static variables:
//!
//! ```rust
//! static mut A: u32 = 0;
//! static mut B: u32 = 0;
//! static mut C: u32 = 0;
//!
//! fn main() {
//!     unsafe {
//!         A = 3;
//!         B = 4;
//!         A = A + B;
//!         C = B;
//!         println!("{A} {B} {C}");
//!         C = A;
//!     }
//! }
//! ```
//!
//! It appears as if some variables stored in memory are changed, an addition
//! is performed, result is stored in `A` and the variable `C` is
//! modified twice.
//!
//! When only a single thread is involved, the results are as expected:
//! the line `7 4 4` gets printed.
//!
//! As for what happens behind the scenes, when optimizations are enabled the
//! final generated machine code might look very different from the code:
//!
//! - The first store to `C` might be moved before the store to `A` or `B`,
//!   _as if_ we had written `C = 4; A = 3; B = 4`.
//!
//! - Assignment of `A + B` to `A` might be removed, since the sum can be stored
//!   in a temporary location until it gets printed, with the global variable
//!   never getting updated.
//!
//! - The final result could be determined just by looking at the code
//!   at compile time, so [constant folding] might turn the whole
//!   block into a simple `println!("7 4 4")`.
//!
//! The compiler is allowed to perform any combination of these
//! optimizations, as long as the final optimized code, when executed,
//! produces the same results as the one without optimizations.
//!
//! Due to the [concurrency] involved in modern computers, assumptions
//! about the program's execution order are often wrong. Access to
//! global variables can lead to nondeterministic results, **even if**
//! compiler optimizations are disabled, and it is **still possible**
//! to introduce synchronization bugs.
//!
//! Note that thanks to Rust's safety guarantees, accessing global (static)
//! variables requires `unsafe` code, assuming we don't use any of the
//! synchronization primitives in this module.
//!
//! [constant folding]: https://en.wikipedia.org/wiki/Constant_folding
//! [concurrency]: https://en.wikipedia.org/wiki/Concurrency_(computer_science)
//!
//! ## Higher-level synchronization objects
//!
//! Most of the low-level synchronization primitives are quite error-prone and
//! inconvenient to use, which is why the standard library also exposes some
//! higher-level synchronization objects.
//!
//! These abstractions can be built out of lower-level primitives.
//! For efficiency, the sync objects in the standard library are usually
//! implemented with help from the operating system's kernel, which is
//! able to reschedule the threads while they are blocked on acquiring
//! a lock.
//!
//! The following is an overview of the available synchronization
//! objects:
//!
//! - [`SpinLock`]: Mutual Exclusion mechanism, which ensures that at
//!   most one thread at a time is able to access some data.
//!
//! [`SpinLock`]: crate::sync::SpinLock

pub use abyss::spin_lock::{SpinLock, SpinLockGuard};