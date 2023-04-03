// This is test & bootstrap implementation.
// This file will be overwritten when grading.
#![no_std]
#![no_main]
#![deny(unsafe_code)]

extern crate alloc;
#[macro_use]
extern crate keos;

#[allow(unsafe_code)]
#[no_mangle]
pub unsafe fn main() {
    println!("Hello guest os!");

    // Hypercall exit.
    core::arch::asm!("xor rax, rax", "mov rdi, 0", "vmcall")
}
