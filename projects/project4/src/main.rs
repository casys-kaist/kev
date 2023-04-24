// This is test & bootstrap implementation.
// This file will be overwritten when grading.
#![no_std]
#![no_main]

#[allow(unused_imports)]
#[macro_use]
extern crate keos;

extern crate project1;
extern crate project2;

use project1::rr::RoundRobin;

#[allow(unsafe_code)]
#[no_mangle]
pub unsafe fn main() {
    keos::thread::scheduler::set_scheduler(RoundRobin::new());
    unsafe { kev::start_vmx_on_cpu().expect("Failed to initialize VMX.") }
    keos::do_tests(&[&tests::run_keos]);
}

#[allow(unsafe_code)]
#[no_mangle]
pub unsafe fn ap_main() {
    unsafe { kev::start_vmx_on_cpu().expect("Failed to initialize VMX.") }
}

mod tests {
    use kev::vm::VmBuilder;
    use project4::vm::VmState;

    pub fn run_keos() {
        // VM with 256 MiB memory.
        let vm = VmBuilder::new(
            VmState::new(256 * 1024).expect("Failed to crate vmstate"),
            4,
        )
        .expect("Failed to create vmbuilder.")
        .finalize()
        .expect("Failed to create vm.");
        vm.start_bsp().expect("Failed to start bsp.");
        vm.join();
    }
}
