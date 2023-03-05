//! Project1: KeOS.
//!
//! In this project, you will implement the mini operating system called `KeOS` (KAIST educational operating system).
//! KeOS is a minimalistic OS that includes necessary features to boot and run threads.
//! You will implement small functionalities to learn the APIs and infrastructure of the KeOS.
//!
//! This project is divided into three sections: [`Synchronization`], [`Round-robin Scheduling`] and [`Page Table`].
//! Note that **KeOS will not be booted** without the implementation of the [`Synchronization`] projects.
//!
//! ## Getting started
//!
//! **YOU MUST IMPLEMENT [`Synchronization`] project first.**
//!
//! after implementing the [`Synchronization`] crate, go to the project 1 directory.
//!
//! KeOS will be panic with "not yet implemented" message.
//!
//! ```/bin/bash
//! $ cargo run --target ../.cargo/x86_64-unknown-keos.json
//! ```
//!
//! ## Outline
//! - [`Synchronization`]
//! - [`Round-robin Scheduling`]
//! - [`Page Table`]
//!
//! [`Synchronization`]: ../../../spin_lock/smplock
//! [`Round-robin Scheduling`]: rr
//! [`Page Table`]: page_table

#![no_std]
#![no_main]

extern crate alloc;
#[allow(unused_imports)]
#[macro_use]
extern crate keos;

pub mod page_table;
pub mod rr;
