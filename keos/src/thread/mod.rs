//! A thread, abstraction of a cpu core.
//!
//! ## The threading model
//!
//! An executing kernel consists of a collection of threads,
//! each with their own stack and local state. Threads can be named, and
//! provide some built-in support for low-level synchronization.
pub mod channel;
pub mod scheduler;

use abyss::{interrupt::InterruptGuard, x86_64::intrinsics::cpuid};
use alloc::{boxed::Box, string::String, sync::Arc};
use core::{
    arch::asm,
    sync::atomic::{AtomicI32, AtomicU64, Ordering},
};

/// Size of each thread's stack.
pub const STACK_SIZE: usize = 0x100000;
/// Thread magic to detect stack overflow.
pub const THREAD_MAGIC: usize = 0xdeadbeefcafebabe;

/// The Thread stack.
///
/// DO NOT MODIFY THIS STRUCT.
#[repr(C, align(0x100000))]
#[doc(hidden)]
pub(crate) struct ThreadStack {
    pub(crate) thread: *mut Thread,
    pub(crate) magic: usize,
    /// Padding to fill up to [`STACK_SIZE`]
    pub(crate) _pad:
        [u8; STACK_SIZE - core::mem::size_of::<*mut Thread>() - core::mem::size_of::<usize>()],
    /// Marker of address of usable stack.
    pub(crate) _usable_marker: [u8; 0],
    /// Pinned.
    _pin: core::marker::PhantomPinned,
}

/// A possible state of the thread.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum ThreadState {
    /// Thread is runnable.
    Runnable,
    /// Thread is running.
    Running,
    /// Thread is exited with exitcode.
    Exited(i32),
    /// Thread is idle.
    Idle,
    /// Thread is parked.
    Parked,
}

#[repr(C)]
/// An thread abstraction.
pub struct Thread {
    /// A stack pointer on context switch.
    ///
    /// ## WARNING
    /// DO NOT CHANGE THE OFFSET THIS FIELDS.
    /// This offset used in context switch with hard-coded value.
    /// You must add your own members **BELOWS** this sp field.
    pub(crate) sp: usize,
    /// Thread Stack
    pub(crate) stack: Box<ThreadStack>,
    /// Thread name
    pub name: String,
    /// State of the thread.
    pub state: ThreadState,
    pub(crate) running_cpu: Arc<AtomicI32>,
    pub(crate) exit_status: Arc<AtomicU64>,
}

impl Thread {
    // DO NOT USE THIS API.
    #[doc(hidden)]
    pub fn new<I>(name: I) -> Box<Self>
    where
        alloc::string::String: core::convert::From<I>,
    {
        let mut stack: Box<ThreadStack> = unsafe { Box::new_uninit().assume_init() };
        stack.magic = THREAD_MAGIC;

        Box::new(Self {
            sp: 0,
            stack,
            name: String::from(name),
            state: ThreadState::Runnable,
            exit_status: Arc::new(AtomicU64::new(0)),
            running_cpu: Arc::new(AtomicI32::new(-1)),
        })
    }

    /// Exit the thread with `exit_code`.
    pub fn exit(&mut self, exit_code: i32) -> ! {
        self.exit_status
            .store(0x8000_0000_0000_0000 | (exit_code as u64), Ordering::SeqCst);
        self.state = ThreadState::Exited(exit_code);
        scheduler::scheduler().reschedule();
        unreachable!()
    }

    /// Park the current threads and run the given closure.
    pub fn park_current_and(f: impl FnOnce(ParkHandle)) {
        let _ = abyss::interrupt::InterruptGuard::new();
        with_current(|th| {
            f(unsafe { scheduler::scheduler().park_thread(th).unwrap() });
        });
        scheduler::scheduler().reschedule();
    }

    pub(crate) unsafe fn do_run(&mut self) {
        let _p = abyss::interrupt::InterruptGuard::new();
        let next_sp = self.sp;
        let current_sp = with_current(|th| {
            while self.running_cpu.load(Ordering::SeqCst) != -1 {
                core::hint::spin_loop()
            }
            &mut th.sp as *mut usize
        });
        assert_eq!(
            abyss::interrupt::InterruptState::current(),
            abyss::interrupt::InterruptState::Off
        );
        context_switch_trampoline(current_sp, next_sp)
    }

    pub(crate) fn run(self: Box<Self>) {
        unsafe { Box::into_raw(self).as_mut().unwrap().do_run() }
    }

    /// Pin current thread not to be scheduled by block .
    ///
    /// When [`ThreadPinGuard`] is dropped, the current thread is unpinned.
    /// When you hold multiple [`ThreadPinGuard`], you **MUST** drops [`ThreadPinGuard`] as a reverse order of creation.
    pub fn pin() -> ThreadPinGuard {
        ThreadPinGuard::new()
    }
}

/// A RAII implementation of the thread pinning.
pub type ThreadPinGuard = InterruptGuard;

/// A handle to join thread.
pub struct JoinHandle
where
    Self: 'static,
{
    // Define any member you need.
    exit_status: Arc<AtomicU64>,
    running_cpu: Arc<AtomicI32>,
}

impl JoinHandle {
    /// Make a join handle for Thread `th`.
    pub fn new_for(th: &Thread) -> Self {
        // Project1: Fill this function.
        Self {
            exit_status: th.exit_status.clone(),
            running_cpu: th.running_cpu.clone(),
        }
    }

    /// Join this handle and returns exit code.
    pub fn join(self) -> i32 {
        loop {
            let v = self.exit_status.load(Ordering::SeqCst);
            if v >= 0x8000_0000_0000_0000 {
                return v as i32;
            }
        }
    }

    /// Get scheudled cpu id of the underlying thread.
    ///
    /// If the thread is not runnig, returns None.
    pub fn try_get_running_cpu(&self) -> Option<usize> {
        match self.running_cpu.load(Ordering::SeqCst) {
            v if v < 0 => None,
            v => Some(v as usize),
        }
    }
}

unsafe impl Send for JoinHandle {}
unsafe impl Sync for JoinHandle {}

/// A handle that represent the parked thread.
pub struct ParkHandle {
    pub(crate) th: Box<Thread>,
}

impl ParkHandle {
    pub(crate) fn new_for(th: Box<Thread>) -> Self {
        Self { th }
    }

    /// Consume the handle and unpark the underlying thread.
    pub fn unpark(mut self) {
        // Wait until context switch is finished.
        while self.th.running_cpu.load(Ordering::SeqCst) != -1 {
            core::hint::spin_loop()
        }
        self.th.state = ThreadState::Runnable;
        scheduler::scheduler().push_to_queue(self.th);
    }
}

unsafe impl Send for ParkHandle {}
unsafe impl Sync for ParkHandle {}

// Context switch related codes.

/// The context-switch magic.
#[naked]
unsafe extern "C" fn context_switch_trampoline(_current_sp: *mut usize, _next_sp: usize) {
    // XXX: we don't need to rflags because when threads entered this function the
    // rflags state is always same. RDI: Current Stack pointer storage.
    // RSI: Next Stack pointer.
    asm!("push rbp",
         "push rbx",
         "push r12",
         "push r13",
         "push r14",
         "push r15",

         // Switch.
         "mov r8, rsp",
         "mov [rdi], r8",
         "mov rsp, rsi",

         "pop r15",
         "pop r14",
         "pop r13",
         "pop r12",
         "pop rbx",
         "pop rbp",

         // XXX: Tail-call optimization, pass prev thread to rdi
         "jmp {}",
         sym finish_context_switch,
         options(noreturn));
}

unsafe extern "C" fn finish_context_switch(prev: &'static mut Thread) {
    assert_eq!(
        abyss::interrupt::InterruptState::current(),
        abyss::interrupt::InterruptState::Off
    );
    match prev.state {
        ThreadState::Exited(_e) => {
            let _ = Box::from_raw(prev);
        }
        ThreadState::Idle => (),
        ThreadState::Running => {
            prev.state = ThreadState::Runnable;
            let th = Box::from_raw(prev);
            scheduler::scheduler().push_to_queue(th);
        }
        ThreadState::Parked => (),
        ThreadState::Runnable => unreachable!("{:?} {:?}", prev as *const _, prev.name),
    }
    with_current(|th| {
        if th.state != ThreadState::Idle {
            th.state = ThreadState::Running
        }
        abyss::x86_64::segmentation::SegmentTable::update_tss(
            th.stack.as_mut() as *mut _ as usize + STACK_SIZE,
        );
        th.running_cpu.store(cpuid() as i32, Ordering::SeqCst);
    });
    prev.running_cpu.store(-1, Ordering::SeqCst);
}

/// Run a function `f` with current thread as an argument.
pub fn with_current<R>(f: impl FnOnce(&mut Thread) -> R) -> R {
    unsafe {
        let mut sp: usize;
        asm!("mov {}, rsp", out(reg) sp);
        let current_stack = ((sp & !(STACK_SIZE - 1)) as *mut ThreadStack)
            .as_mut()
            .unwrap();

        if current_stack.magic != THREAD_MAGIC {
            panic!(
                "Stack overflow detected! You might allocate big local variable. Stack: {:?}",
                current_stack as *const _
            )
        } else {
            f(current_stack.thread.as_mut().unwrap())
        }
    }
}

/// A struct to build a new thread.
pub struct ThreadBuilder {
    th: Box<Thread>,
}

/// A struct to mimic a stack state on context switch.
#[repr(C)]
struct ContextSwitchFrame<F: FnOnce() + Send> {
    _r15: usize,
    _r14: usize,
    _r13: usize,
    _r12: usize,
    _bx: usize,
    _bp: usize,
    ret_addr: usize,
    thread_fn: *mut F,
    end_of_stack: usize,
}

impl ThreadBuilder {
    /// Create a new thread builder for thread `name`.
    pub fn new<I>(name: I) -> Self
    where
        alloc::string::String: core::convert::From<I>,
    {
        Self {
            th: Thread::new(name),
        }
    }

    fn to_thread<F: FnOnce() + Send + 'static>(self, thread_fn: F) -> Box<Thread> {
        /// The very beginning of the thread
        #[naked]
        unsafe extern "C" fn start<F: FnOnce() + Send>() -> ! {
            asm!(
                "pop rdi",
                "sti",
                "jmp {}",
                sym thread_start::<F>,
                options(noreturn),
            );
        }

        fn thread_start<F: FnOnce() + Send>(thread_fn: *mut F) {
            let o = unsafe { *Box::from_raw(thread_fn) };
            o();
            with_current(|current| current.exit(0));
            scheduler::scheduler().reschedule();
            unreachable!()
        }

        let Self { mut th } = self;
        let stack = th.stack.as_mut();
        let frame = unsafe {
            ((&mut stack._usable_marker as *mut _ as usize
                - core::mem::size_of::<ContextSwitchFrame<F>>())
                as *mut ContextSwitchFrame<F>)
                .as_mut()
                .unwrap()
        };
        frame.end_of_stack = 0;
        frame.thread_fn = Box::into_raw(Box::new(thread_fn));
        frame.ret_addr = start::<F> as usize;
        th.sp = frame as *mut _ as usize;
        th.stack.thread = th.as_mut() as *mut _;
        th
    }

    /// Spawn the thread as a parked state.
    pub fn spawn_as_parked<F: FnOnce() + Send + 'static>(self, thread_fn: F) -> ParkHandle {
        let th = self.to_thread(thread_fn);
        ParkHandle::new_for(th)
    }

    /// Spawn the thread.
    pub fn spawn<F: FnOnce() + Send + 'static>(self, thread_fn: F) -> JoinHandle {
        let th = self.to_thread(thread_fn);
        let handle = JoinHandle::new_for(&th);
        scheduler::scheduler().push_to_queue(th);
        handle
    }
}
