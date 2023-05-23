// This is test & bootstrap implementation.
// This file will be overwritten when grading.
#![no_std]
#![no_main]

extern crate alloc;
#[allow(unused_imports)]
#[macro_use]
extern crate keos;
extern crate project1;

#[allow(unsafe_code)]
#[no_mangle]
pub unsafe fn ap_main() {}

#[allow(unsafe_code)]
#[no_mangle]
pub unsafe fn main() {
    println!("Hello guest os!");
    loop {}
}
