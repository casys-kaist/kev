//! Multicore Round-robin Scheduling.
//!
//! In this part, you will implement round robin scheduler.
//! Round robin is a scheduler that assigns equal amount of time slice into threads in circular order.
//! When time slice is expired, the scheduler kicks out the thread from the running state
//! and push back to the scheduling queue.
//!
//! ## Background
//! Thread is an abstraction of a cpu core.
//! Through the thread abstraction, the operating system can run multiple tasks
//! on a cpu core.
//!
//! KeOS already implemented some thread functionalities like thread creation,
//! thread switching.
//! You can create a new thread with [`ThreadBuilder`]. You provide a function to be run on
//! the thread as an argument to [`ThreadBuilder::spawn`].
//! When the first time the thread runs, the function executed.
//! When the function is terminated, the thread also be terminated.
//! Each thread, therefore, acts like a mini-program running inside the kernel.
//!
//! At any given time, exactly one thread runs and the rest, if any, become inactive.
//! The scheduler decides which thread to run next by calling [`Scheduler::next_to_run`].
//! If no thread is ready to run at any given time, then the special idle thread runs.
//!
//! The magics of a context switch lies on [`Thread::run`]. It saves the state of the
//! currently running thread and restores the state of the thread we're switching to.
//!
//! ## IMPORTANT NOTES
//! In KeOS, each thread is assigned a [`STACK_SIZE`]-size execution stack. The KeOS try to detect
//! the stack overflow, however it is not perfect. If the stack is overflowed, you can encounter a
//! mysterious kernel panics.
//! To prevent the such situation, DO NOT DECLARE large data structures (e.g. `let v: [u8; 0x200000];`)
//! and allocate them on heap through [`Box`].
//!
//! ## Implementation requirements
//! In this projects, you implements a round robin scheduler with 5ms time slices.
//! Note that [`Scheduler::timer_tick`] is called at every 1ms on each cpu.
//! When a new thread is reached, it must be pushed back to the current scheduling queue.
//! You are required to implement passive job stealing; when there is no task in the current runqueue, then steal from the other.
//!
//! ## Implementation Order
//! You will fill the [`Scheduler`] trait implementor for `RoundRobin` struct.
//! Fill the `todo!()`s of RoundRobin scheduler.
//! Read the documentation of [`Scheduler`] to understand what each member does.
//! The project is straightforward. You will meet several `todo!()`s during implementation,
//! and replace those `todo!()`s to your implementation.
//!
//!
//! [`RoundRobin`]: RoundRobin
//! [`Thread`]: keos::thread::Thread
//! [`Thread::run`]: keos::thread::Thread::run
//! [`Box`]: https://doc.rust-lang.org/alloc/boxed/struct.Box.html
//! [`STACK_SIZE`]: keos::thread::STACK_SIZE
//! [`ThreadBuilder`]: keos::thread::ThreadBuilder
//! [`ThreadBuilder::spawn`]: keos::thread::ThreadBuilder::spawn
//! [`Scheduler`]: keos::thread::scheduler::Scheduler
//! [`Scheduler::next_to_run`]: keos::thread::scheduler::Scheduler::next_to_run

use alloc::{boxed::Box, collections::VecDeque};
use core::sync::atomic::{AtomicIsize, Ordering};
use keos::{
    intrinsics::cpuid,
    sync::SpinLock,
    thread::{scheduler::Scheduler, Thread},
    MAX_CPU,
};

/// A round robin scheduler.
pub struct RoundRobin {
    // Define any member you need.
}
unsafe impl Send for RoundRobin {}
unsafe impl Sync for RoundRobin {}

impl RoundRobin {
    /// Create a new roundrobin scheduler.
    pub fn new() -> Self {
        todo!()
    }
}

impl Scheduler for RoundRobin {
    fn next_to_run(&self) -> Option<Box<Thread>> {
        todo!()
    }
    fn push_to_queue(&self, thread: Box<Thread>) {
        todo!()
    }
    fn timer_tick(&self) {
        todo!()
    }
}
