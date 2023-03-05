//! Thread scheduler

use super::{ParkHandle, Thread, ThreadStack, ThreadState, STACK_SIZE, THREAD_MAGIC};
use alloc::boxed::Box;
use core::arch::asm;

/// Common features of thread scheduler.
pub trait Scheduler {
    /// Peek a next thread to run.
    ///
    /// If there is no thread to run, returns None.
    fn next_to_run(&self) -> Option<Box<Thread>>;
    /// Push a thread `th` into scheduling queue.
    fn push_to_queue(&self, th: Box<Thread>);
    /// Called on every timer interrupt (1ms).
    fn timer_tick(&self);
}

static mut SCHEDULER: Option<&'static dyn Scheduler> = None;

/// Set the scheduler of the kernel.
pub unsafe fn set_scheduler(t: impl Scheduler + 'static) {
    SCHEDULER = (Box::into_raw(Box::new(t)) as *const dyn Scheduler).as_ref();
    crate::interrupt::register(32, || scheduler().timer_tick());
}

/// Get the reference of the kernel
pub fn scheduler() -> &'static (dyn Scheduler + 'static) {
    unsafe { *SCHEDULER.as_mut().unwrap() }
}

impl dyn Scheduler {
    /// Reschedule.
    pub fn reschedule(&self) {
        let _p = Thread::pin();
        if let Some(th) = self.next_to_run() {
            drop(_p);
            th.run();
        } else {
            unsafe {
                IDLE[abyss::x86_64::intrinsics::cpuid()]
                    .as_mut()
                    .unwrap()
                    .do_run();
            }
        }
    }

    /// Park a thread 'th' and return ParkHandle.
    pub(crate) unsafe fn park_thread(&self, th: &mut Thread) -> Result<ParkHandle, ()> {
        if matches!(th.state, ThreadState::Parked) {
            return Err(());
        }
        th.state = ThreadState::Parked;
        unsafe {
            Ok(ParkHandle {
                th: Box::from_raw(th),
            })
        }
    }
}

const INIT: Option<Box<Thread>> = None;
static mut IDLE: [Option<Box<Thread>>; abyss::MAX_CPU] = [INIT; abyss::MAX_CPU];

/// Transmute this thread into the idle.
pub unsafe fn start_idle(core_id: usize) -> ! {
    let mut sp: usize;
    asm!("mov {}, rsp", out(reg) sp);

    let mut tcb = Thread::new("idle");
    tcb.state = ThreadState::Idle;

    tcb.stack = Box::from_raw((sp & !(STACK_SIZE - 1)) as *mut ThreadStack);
    tcb.stack.magic = THREAD_MAGIC;
    tcb.stack.thread = tcb.as_mut() as *mut _;
    IDLE[core_id] = Some(tcb);

    let scheduler = scheduler();
    loop {
        if let Some(th) = scheduler.next_to_run() {
            th.run();
        }
    }
}
