#![feature(negative_impls)]
#![cfg_attr(not(test), no_std)]

#[cfg(feature = "smp")]
pub mod smplock;
#[cfg(feature = "smp")]
pub use smplock::*;

#[cfg(not(feature = "smp"))]
mod unilock;
#[cfg(not(feature = "smp"))]
pub use unilock::*;

#[cfg(all(not(feature = "smp"), test))]
compile_error!("cargo test is only supported with smp flags");
