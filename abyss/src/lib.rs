//! The abyss of kernel that operates hardwares.
//!
//! This crate contains collections of hardware comunnications.
//! You can treat these codes as a some kind of "magic".
//! We **STRONGLY** recommend not to see codes in this crate.

#![no_std]
#![feature(
    alloc_layout_extra,
    abi_x86_interrupt,
    asm_const,
    const_mut_refs,
    core_intrinsics,
    naked_functions
)]

extern crate alloc;

#[macro_use]
pub mod kprint;
pub mod addressing;
pub mod boot;
#[macro_use]
pub mod dev;
pub mod interrupt;
pub mod spin_lock;
pub mod x86_64;

/// Maximum number of CPU this kernel can support.
pub const MAX_CPU: usize = 4;
