//! Project 4: Interrupt and I/O virtualization.
//!
//! In this project, you will implement timer interrupt virtualization and I/O virtualization for a simple block device.
//!
//! When KeOS is running as guest OS, all interrupts are trapped into the KeV hypervisor
//! requiring that the interrupts are handled properly.
//! In project3, internal interrupts generated by guest OS are ignored in the hypervisor for simplicity of the project.
//! So there are no supports of multi-threading due to the absence of the timer interrupt.
//! But now, it is the time to handle the parts to support scheduling in the guest.
//!
//! In addition to interrupt virtualization, I/O virtualization is needed to support higher performance device operations within guest operating system.
//! Without I/O virtualization, All privileged instruction in an I/O operation are trap-and-emulated by hypervisor causing several context switching overhead.
//! In the project4, We use para-virtualized I/O model to handle the I/O operations with a simple block device.
//!
//! ## Getting started
//! When you run following command lines in the project4 directory, keos will be panic with "not yet implemented" message.
//! ```/bin/bash
//! $ cargo run --target ../.cargo/x86_64-unknown-keos.json
//! ```
//! ## Outline
//! - [`Interrupt Virtualization`]
//! - [`I/O virtualization`]
//!
//! [`Interrupt Virtualization`]: dev/x2apic
//! [`I/O virtualization`]: dev/simple_virtio
//!
#![no_std]

extern crate alloc;

#[allow(unused_imports)]
#[macro_use]
extern crate keos;

pub mod dev;
pub mod virtio;
pub mod vm;
