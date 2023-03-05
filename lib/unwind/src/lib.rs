// Copyright 2021 Computer Architecture and Systems Lab
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A pure rust implementation of Unwind project.
//!
//! <https://refencodings.linuxfoundation.org/LSB_1.3.0/gLSB/gLSB/ehframehdr.html>

#![no_std]
#![feature(core_intrinsics, lang_items, naked_functions)]

extern crate alloc;

mod ehframe;
mod machine;
mod personality;
mod reader;
mod x86_64;

use alloc::boxed::Box;
use ehframe::EhFrameHeader;
use x86_64::Register;

pub use ehframe::FrameDescriptionEntry;
pub use reader::{DwarfReader, Encoding, Peeker};
pub use x86_64::StackFrame;

pub enum ExceptionHandlingPhase {
    Search,
    Cleanup,
}

#[derive(Clone, Copy)]
enum PersonalityResult {
    Continue,
    Run(usize),
    Error,
    Stop,
}

pub enum UnwindError {
    BadRegister,
    BadOpcode(u8),
    BadOperand(u8),
    InvalidOp(u8),
    InvalidApplication,
    InvalidPc(usize),
    UnknownRegister,
    UnmanagedRegister,
    ParsingFailure,
    UnwindablePc(usize),
    MemoryOutOfBound(usize, core::ops::Range<usize>),
    PersonalityFailure,
}

impl core::fmt::Debug for UnwindError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BadRegister => write!(f, "BadRegister"),
            Self::BadOpcode(v) => write!(f, "BadOpcode(0x{:x})", v),
            Self::BadOperand(v) => write!(f, "BadOperand(0x{:x})", v),
            Self::InvalidOp(v) => write!(f, "InvalidOp(0x{:x})", v),
            Self::InvalidApplication => write!(f, "InvalidApplication"),
            Self::InvalidPc(v) => write!(f, "InvalidPc(0x{:x})", v),
            Self::UnknownRegister => write!(f, "UnknownRegister"),
            Self::UnmanagedRegister => write!(f, "UnmanagedRegister"),
            Self::ParsingFailure => write!(f, "ParsingFailure"),
            Self::UnwindablePc(v) => write!(f, "UnwindablePc(0x{:x})", v),
            Self::MemoryOutOfBound(v, _) => write!(f, "MemoryOutOfBound(0x{:x})", v),
            Self::PersonalityFailure => write!(f, "PersonalityFailure"),
        }
    }
}

#[derive(Clone)]
#[repr(C)]
pub struct UnwindHandler {
    pub resume: unsafe fn(usize) -> !,
    pub finish: unsafe fn(usize),
}

impl UnwindHandler {
    /// Finish the unwinding.
    ///
    /// # Safety
    /// `self` must be the Box object delayed from unwind_raise_exception_with_hook.
    pub unsafe fn finish(_data: *mut u8, exception: *mut u8) {
        let handler = exception as *const Self;
        ((*handler).finish)(exception as usize);
    }
}

#[derive(Clone)]
#[repr(C)]
pub struct UnwindContext<T>
where
    T: Peeker,
{
    handler: UnwindHandler,
    pub frame: StackFrame,
    sp_range: core::ops::Range<usize>,
    cfa: usize,
    reader: DwarfReader<T>,
}

impl<T> UnwindContext<T>
where
    T: Peeker,
{
    #[inline]
    pub fn new_boxed(
        frame: StackFrame,
        sp_range: core::ops::Range<usize>,
        reader: DwarfReader<T>,
    ) -> Box<Self> {
        let cfa = frame.sp();
        Box::new(UnwindContext {
            handler: UnwindHandler {
                resume: Self::resume_unwind,
                finish: Self::finish_unwind,
            },
            frame,
            sp_range,
            cfa,
            reader,
        })
    }

    #[inline]
    pub fn read_mem(&self, addr: usize) -> Result<usize, UnwindError> {
        if self.sp_range.contains(&addr) {
            Ok(unsafe { (addr as *const usize).as_ref() }.cloned().unwrap())
        } else {
            Err(UnwindError::MemoryOutOfBound(addr, self.sp_range.clone()))
        }
    }

    #[inline]
    fn do_unwind_frame<UnwindFn>(
        mut self: Box<Self>,
        mut unwind_fn: UnwindFn,
    ) -> Result<(), UnwindError>
    where
        UnwindFn: FnMut(Box<Self>, &FrameDescriptionEntry) -> (Box<Self>, bool),
    {
        let hdr = EhFrameHeader::parse(self.reader.clone());
        let (mut previous_pc, mut previous_cfa) = (self.frame.pc(), self.cfa);
        while self.frame.pc() != 0 {
            let fde = hdr
                .find(self.frame.pc())
                .unwrap()
                .insn
                .parse(self.reader.clone())
                .ok_or(UnwindError::ParsingFailure)?;
            if fde.pc.contains(&self.frame.pc()) {
                let (s, is_stop) = unwind_fn(self, &fde);
                if is_stop {
                    return Ok(());
                }
                self = s;
                fde.run(self.frame.pc())?.apply(&mut self)?;
                if self.frame.pc() == previous_pc && self.cfa == previous_cfa {
                    return Err(UnwindError::UnwindablePc(self.frame.pc()));
                }

                previous_pc = self.frame.pc();
                previous_cfa = self.cfa;
            } else {
                return Err(UnwindError::InvalidPc(self.frame.pc()));
            }
        }
        Ok(())
    }

    #[inline]
    pub fn unwind_frame<UnwindFn>(
        self: Box<Self>,
        mut unwind_fn: UnwindFn,
    ) -> Result<(), UnwindError>
    where
        UnwindFn: FnMut(&Box<Self>, &FrameDescriptionEntry),
    {
        self.do_unwind_frame(|this, fde| {
            unwind_fn(&this, fde);
            (this, false)
        })
    }

    /// Raise exception through unwind with hook function.
    ///
    /// # Safety
    /// LSDA in Unwind table must point the valid handler address.
    #[inline]
    pub unsafe fn unwind_raise_exception_with_hook<S, UnwindFn, FinishFn>(
        self: Box<Self>,
        state: S,
        mut hook: UnwindFn,
        mut finish: FinishFn,
    ) -> Result<(), UnwindError>
    where
        UnwindFn: FnMut(&mut S, &Box<Self>, &FrameDescriptionEntry),
        FinishFn: FnMut(S),
    {
        let mut action_after_search = PersonalityResult::Continue;

        let mut state = state;
        // 1. Search phase.
        let maybe_error = self.clone().do_unwind_frame(|this, fde| {
            hook(&mut state, &this, fde);
            // Call personality if available.
            if let Some(personality) = fde.cie.personality {
                let routine = personality
                    as *const fn(
                        ExceptionHandlingPhase,
                        &FrameDescriptionEntry,
                        &StackFrame,
                    ) -> PersonalityResult;
                match (*routine)(ExceptionHandlingPhase::Search, fde, &this.frame) {
                    PersonalityResult::Error => action_after_search = PersonalityResult::Error,
                    PersonalityResult::Stop => return (this, true),
                    _ => (),
                }
            }
            (this, false)
        });

        finish(state);
        maybe_error?;

        // 2. Cleanup Phase
        // If it is okay to unwind, run cleanup.
        if !matches!(action_after_search, PersonalityResult::Error) {
            self.do_unwind_frame(|this, fde| Self::cleanup_phase(this, fde))
        } else {
            Err(UnwindError::PersonalityFailure)
        }
    }

    unsafe fn cleanup_phase(this: Box<Self>, fde: &FrameDescriptionEntry) -> (Box<Self>, bool) {
        // Call personality if available.
        if let Some(personality) = fde.cie.personality {
            let routine = personality
                as *const fn(
                    ExceptionHandlingPhase,
                    &FrameDescriptionEntry,
                    &StackFrame,
                ) -> PersonalityResult;
            match (*routine)(ExceptionHandlingPhase::Cleanup, fde, &this.frame) {
                PersonalityResult::Run(ldap) => {
                    let mut frame = this.frame.clone();
                    let this_ptr = Box::into_raw(this);
                    *frame.get_mut(Register::DATA_REG1).unwrap() = this_ptr as usize;
                    *frame.get_mut(Register::DATA_REG2).unwrap() = 0;
                    *frame.get_mut(Register::IP).unwrap() = ldap;
                    x86_64::apply_state(&frame);
                }
                PersonalityResult::Error => {
                    core::intrinsics::abort();
                }
                _ => (this, false),
            }
        } else {
            (this, false)
        }
    }

    unsafe fn resume_unwind(s: usize) -> ! {
        let mut this = Box::from_raw(s as *mut Self);

        let hdr = EhFrameHeader::parse(this.reader.clone());
        let fde = hdr
            .find(this.frame.pc())
            .unwrap()
            .insn
            .parse(this.reader.clone())
            .unwrap();
        if fde.pc.contains(&this.frame.pc()) {
            fde.run(this.frame.pc())
                .and_then(|n| n.apply(&mut this))
                .unwrap_or_else(|_| core::intrinsics::abort());
        } else {
            core::intrinsics::abort()
        }
        let _ = this.do_unwind_frame(|this, fde| Self::cleanup_phase(this, fde));
        core::intrinsics::abort()
    }

    unsafe fn finish_unwind(s: usize) {
        let mut _this = Box::from_raw(s as *mut Self);
    }
}
