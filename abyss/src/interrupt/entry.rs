//! Interrupt handler entries.

use super::TrapFrame;
use crate::x86_64::{interrupt::PFErrorCode, segmentation::SegmentSelector};
use core::arch::global_asm;

global_asm!(include_str!("entry.s"));

// Load interrupt descriptor table.
#[no_mangle]
#[allow(clippy::empty_loop)]
extern "C" fn handle_general_protection_fault(frame: &mut TrapFrame, _c: SegmentSelector) {
    panic!("General Protection Fault! {:#?}", frame);
}

#[no_mangle]
extern "C" fn handle_page_fault(_frame: &mut TrapFrame, _ec: PFErrorCode) {
    todo!("PF");
}

#[no_mangle]
extern "C" fn handle_double_fault(
    frame: &mut TrapFrame,
    _: crate::x86_64::interrupt::MustbeZero,
) -> ! {
    panic!("Double Fault!\n{:#?}", frame);
}

#[no_mangle]
extern "C" fn handle_invalid_opcode(frame: &mut TrapFrame) {
    panic!("Invalid Opcode!\n{:#?}", frame);
}

#[no_mangle]
extern "C" fn handle_simd_floating_point_exception(_frame: &mut TrapFrame) {
    panic!("Floating Point Exception!");
}

#[no_mangle]
extern "C" fn handle_device_not_available(_frame: &mut TrapFrame) {
    panic!("Device Not Available");
}

#[no_mangle]
#[allow(clippy::empty_loop)]
extern "C" fn do_handle_irq(_frame: &mut TrapFrame, vec: usize) {
    irq_handler(vec)
}

#[doc(hidden)]
pub fn irq_handler(vec: usize) {
    extern "Rust" {
        fn do_handle_interrupt(idx: usize);
    }

    crate::dev::x86_64::apic::eoi();

    if vec == 32 {
        unsafe {
            // Reprgram the deadline.
            crate::dev::x86_64::timer::set_tsc_timer();
        }
    }
    unsafe {
        do_handle_interrupt(vec - 32);
    }
}
