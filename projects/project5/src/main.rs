//! Project 5: Final project
//!
//! You have learned the basic components of the virtulaization.
//! In the final project, you will do an open-ended final project based on what you have learned.
//! For the final project, you should add a significant piece of functionality to the kev, as described below.
//! The project must (1) have a significant implementation component, specified by deliverables
//! (2) concern operating systems and (3) be challenging and potentially research-worthy.
//! You can modify any source code of the keos and kev infrastructure.
//! For all projects, you must write a proposal that is accepted by the course staff.
//!
//! ## Candidates for final project
//! Here is a list of possible tasks for a final project. Some are small enough that multiple options should be combined, especially if you have a larger group.
//! You are most welcome to come up with other ideas. The actual project is up to you. Please pick something manageable.
//! It is far better to complete your project to spec than it is to take on something too big and not have anything to show for it except excellent intentions (also true in the real world).
//!
//! ### Hardware
//! - Port the KeV to work on other vendor’s CPUs: AMD SVM (the rough equivalent of Intel’s VMX), ARM Virtualization
//! - Implement support for an IOMMU to allow the guest to directory access hardware
//! - Incorporate x2APIC hardware-based virtualization (Not emulation!)
//! - Implement any driver and work it on the KeV and gKeOS: VirtIO, network, NVMe, ..
//!
//! ### VM Management
//! - Implement the Kernel Same-page Merging (KSM) on KeV
//! - Implement VM snapshot and resume on KeV
//! - Implement VM live migration on KeV
//! - Implement the nested virtualization to run KeV on KeV
//! - Solve the lock-holder preemption problem on gKeOS
//!
//! ### Others
//! - Use binary translate to support shadow page table on an x86 CPU without Extended Page Table
//! - Use binary translate to support trap-and-emulate semantics on an x86 CPU without VMX or SVM support
//! - Implement vmfunc based IPC (skybridge)
//! - Run another operating system on KeV
//! - Any topic related to the virtualization that you want
//!
//! The project you choose must have a significant virtualization component. For example, you shouldn't simply port a user-level application that requires little or no kernel modification.
//! You should email a proposal to the instructor by the notified deadline.
//! The proposal must include: (1) The names of your group members; (2) What you want to do; and (3) What you are expecting to present (a list of deliverables).
//! Please keep it short (no more than several paragraphs).
//!

#![no_std]
#![no_main]

extern crate alloc;
extern crate keos;
extern crate project1;
extern crate project2;
extern crate project3;
extern crate project4;

use project1::rr::RoundRobin;

#[allow(unsafe_code)]
#[no_mangle]
pub unsafe fn main() {
    unsafe { keos::thread::scheduler::set_scheduler(RoundRobin::new()) };
    unsafe { kev::start_vmx_on_cpu().expect("Failed to initialize VMX.") }
    todo!()
}

#[allow(unsafe_code)]
#[no_mangle]
pub unsafe fn ap_main() {
    unsafe { kev::start_vmx_on_cpu().expect("Failed to initialize VMX.") }
}
