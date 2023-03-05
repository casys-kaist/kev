//! Interrupt
use crate::x86_64::{
    interrupt::InterruptStackFrame,
    segmentation::{Segment, SegmentSelector},
    PrivilegeLevel, Rflags,
};
use core::arch::asm;

mod entry;

pub use entry::irq_handler;

/// Enumeration for representing interrupt state
#[derive(PartialEq, Eq, Debug)]
pub enum InterruptState {
    /// Interrupt is on.
    On,
    /// Interrupt is off.
    Off,
}

impl InterruptState {
    /// Read the current interrupt state.
    pub fn current() -> Self {
        if Rflags::read().contains(Rflags::IF) {
            Self::On
        } else {
            Self::Off
        }
    }
}

/// An RAII implementation of an interrupt disable. When this structure is
/// dropped (falls out of scope), the interrupt will be recovered into state on creation of this struct.
/// Therefore, you must dropped the this struct in reverse of creation order.
///
///
/// This structure is created by the [`new`].
///
/// [`new`]: InterruptGuard::new
pub struct InterruptGuard {
    state: InterruptState,
}

impl InterruptGuard {
    /// Create a new InterruptGuard.
    pub fn new() -> Self {
        let state = InterruptState::current();
        unsafe {
            asm!("cli");
        }
        Self { state }
    }
}

impl Drop for InterruptGuard {
    fn drop(&mut self) {
        if self.state == InterruptState::On {
            unsafe {
                asm!("sti");
            }
        }
    }
}

/// X86_64 general purpose registers
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GeneralPurposeRegisters {
    // callee-preserved.
    pub r15: usize,
    // callee-preserved.
    pub r14: usize,
    // callee-preserved.
    pub r13: usize,
    // callee-preserved.
    pub r12: usize,
    pub r11: usize,
    pub r10: usize,
    pub r9: usize,
    pub r8: usize,
    pub rsi: usize,
    pub rdi: usize,
    // callee-preserved.
    pub rbp: usize,
    pub rdx: usize,
    pub rcx: usize,
    // callee-preserved.
    pub rbx: usize,
    pub rax: usize,
    pub error_code: u64,
}

impl Default for GeneralPurposeRegisters {
    fn default() -> Self {
        GeneralPurposeRegisters {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            rsi: 0,
            rdi: 0,
            rbp: 0,
            rdx: 0,
            rcx: 0,
            rbx: 0,
            rax: 0,
            error_code: 0,
        }
    }
}

/// x86_64 Trap frame.
#[repr(C)]
#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct TrapFrame {
    gprs: GeneralPurposeRegisters,
    pub(crate) interrupt_stack_frame: InterruptStackFrame,
}

impl TrapFrame {
    /// Create frame for new user thread.
    #[inline]
    pub fn new_user() -> Self {
        Self {
            gprs: GeneralPurposeRegisters::default(),
            interrupt_stack_frame: InterruptStackFrame {
                rip: 0,
                cs: Segment::UserCode.into_selector(),
                __pad0: 0,
                __pad1: 0,
                rflags: Rflags::IF | Rflags::_1,
                rsp: 0,
                ss: Segment::UserData.into_selector(),
                __pad2: 0,
                __pad3: 0,
            },
        }
    }

    /// Launch the frame.
    #[naked]
    pub extern "C" fn launch(&self) -> ! {
        unsafe {
            asm!(
                "mov rax, [rdi + 0x70]",
                "mov rbx, [rdi + 0x68]",
                "mov rcx, [rdi + 0x60]",
                "mov rdx, [rdi + 0x58]",
                "mov rbp, [rdi + 0x50]",
                "mov rsi, [rdi + 0x40]",
                "mov r8, [rdi + 0x38]",
                "mov r9, [rdi + 0x30]",
                "mov r10, [rdi + 0x28]",
                "mov r11, [rdi + 0x20]",
                "mov r12, [rdi + 0x18]",
                "mov r13, [rdi + 0x10]",
                "mov r14, [rdi + 0x8]",
                "mov r15, [rdi]",
                "sti",
                "lea rsp, [rdi + 0x80]",
                "mov rdi, [rdi + 0x48]",
                "swapgs",
                "iretq",
                options(noreturn)
            )
        }
    }

    #[inline]
    pub fn set_cs(&mut self, cs: u16) {
        self.interrupt_stack_frame.cs = SegmentSelector::new(cs >> 3, PrivilegeLevel::Ring3);
    }
    #[inline]
    pub fn set_ss(&mut self, ss: u16) {
        self.interrupt_stack_frame.ss = SegmentSelector::new(ss >> 3, PrivilegeLevel::Ring3);
    }
    #[inline]
    pub fn set_rflags(&mut self, rflags: u64) {
        // IF / IOPL / VIP / VIF / VM
        self.interrupt_stack_frame.rflags = (Rflags::from_bits_truncate(rflags)
            & !(Rflags::IF
                | Rflags::IOPL0
                | Rflags::IOPL1
                | Rflags::VIP
                | Rflags::VIF
                | Rflags::VM))
            | Rflags::IF
            | Rflags::_1
    }
}

impl core::fmt::Debug for TrapFrame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        writeln!(
            f,
            "RAX: {:016x} | RBX: {:016x}  | RCX: {:016x} | RDX: {:016x}\n\
             RSI: {:016x} | RDI: {:016x}  | RBP: {:016x} | RSP: {:016x}\n\
             R8 : {:016x} | R9 : {:016x}  | R10: {:016x} | R11: {:016x}\n\
             R12: {:016x} | R13: {:016x}  | R14: {:016x} | R15: {:016x}\n\
             RIP: {:016x} | RFLAGS: {:016x} [{:?}]\n\
             CS : {:#?}\n\
             SS : {:#?}",
            self.gprs.rax,
            self.gprs.rbx,
            self.gprs.rcx,
            self.gprs.rdx,
            self.gprs.rsi,
            self.gprs.rdi,
            self.gprs.rbp,
            self.interrupt_stack_frame.rsp,
            self.gprs.r8,
            self.gprs.r9,
            self.gprs.r10,
            self.gprs.r11,
            self.gprs.r12,
            self.gprs.r13,
            self.gprs.r14,
            self.gprs.r15,
            self.interrupt_stack_frame.rip,
            self.interrupt_stack_frame.rflags.bits(),
            self.interrupt_stack_frame.rflags,
            self.interrupt_stack_frame.cs,
            self.interrupt_stack_frame.ss,
        )
    }
}
