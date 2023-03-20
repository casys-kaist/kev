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
    keos::do_tests(&[
        &tests::hypercall::hypercall_exit,
        &tests::hypercall::hypercall_print,
        &tests::pio::pio_print,
        &tests::pio::pio_dx_port,
        &tests::pio::pio_imm8_port,
        &tests::pio::pio_mem,
        &tests::cpuid::cpuid_leaf_0,
        &tests::cpuid::cpuid_leaf_1,
        &tests::msr::msr,
    ]);
}

#[allow(unsafe_code)]
#[no_mangle]
pub unsafe fn ap_main() {
    unsafe { kev::start_vmx_on_cpu().expect("Failed to initialize VMX.") }
}

mod tests {
    use kev::vm::VmBuilder;
    use project2::no_ept_vm::NoEptVmState;

    fn run_vm<const EXPECTED: i32>(code: &'static [u8]) {
        let vm = VmBuilder::new(NoEptVmState::new(code), 1)
            .expect("Failed to create vmbuilder.")
            .finalize()
            .expect("Failed to create vm.");
        vm.start_bsp().expect("Failed to start bsp.");
        assert_eq!(vm.join(), EXPECTED);
    }

    pub mod hypercall {
        use core::arch::global_asm;

        // Exit kernel with code 0xcafe.
        global_asm!(
            "hcall_exit_start:",
            // hcall_exit(0xcafe);
            "mov edi, 0xcafe",
            "xor eax, eax",
            "vmcall",
            "hcall_exit_end:",
        );
        pub fn hypercall_exit() {
            super::run_vm::<0xcafe>(unsafe {
                extern "C" {
                    static hcall_exit_start: u8;
                    static hcall_exit_end: u8;
                }
                core::slice::from_raw_parts(
                    &hcall_exit_start as *const u8,
                    &hcall_exit_end as *const _ as usize - &hcall_exit_start as *const _ as usize,
                )
            });
        }

        // print 'Hello guest os!' and exit.
        global_asm!(
            "hcall_print_start:",
            // hcall_print(hcall_print_buf, 16);
            "lea rdi, [rip + hcall_print_buf]",
            "mov rsi, 16",
            "mov rax, 1",
            "vmcall",
            // hcall_exit(0);
            "mov rdi, 0",
            "mov rax, 0",
            "vmcall",
            // Hello guest os!\n
            "hcall_print_buf:",
            ".byte 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x67, 0x75, 0x65, 0x73, 0x74, 0x20, 0x6f, 0x73, 0x21, 0xa",
            "hcall_print_end:",
        );
        pub fn hypercall_print() {
            super::run_vm::<0>(unsafe {
                extern "C" {
                    static hcall_print_start: u8;
                    static hcall_print_end: u8;
                }
                core::slice::from_raw_parts(
                    &hcall_print_start as *const u8,
                    &hcall_print_end as *const _ as usize - &hcall_print_start as *const _ as usize,
                )
            });
        }
    }

    pub mod cpuid {
        use core::arch::global_asm;
        use kev::vm::VmBuilder;
        use project2::no_ept_vm::NoEptVmState;

        // Get vendor from this core and exit.
        global_asm!(
            "cpuid_leaf_0_start:",
            "mov rax, 0x0",
            "cpuid",
            "cmp rbx, 0x756e6547",
            "jne cpuid_leaf_0_failed",
            "mov rdi, 0",
            "mov rax, 0",
            "vmcall",
            "cpuid_leaf_0_failed:",
            "mov rdi, 1",
            "mov rax, 0",
            "vmcall",
            "cpuid_leaf_0_end:"
        );
        pub fn cpuid_leaf_0() {
            super::run_vm::<0>(unsafe {
                extern "C" {
                    static cpuid_leaf_0_start: u8;
                    static cpuid_leaf_0_end: u8;
                }
                core::slice::from_raw_parts(
                    &cpuid_leaf_0_start as *const u8,
                    &cpuid_leaf_0_end as *const _ as usize
                        - &cpuid_leaf_0_start as *const _ as usize,
                )
            })
        }

        // Check the current virtual core id repeatedly and exit.
        global_asm!(
            "cpuid_leaf_1_start:",
            "mov r8, 0x100",
            "l:",
            "mov rax, 0x1",
            "cpuid",
            "shr ebx, 24",
            "and ebx, 0xFF",
            "cmp ebx, 0xba",
            "jne cpuid_leaf_1_failed",
            "dec r8",
            "jnz l",
            "mov rdi, 0",
            "mov rax, 0",
            "vmcall",
            "cpuid_leaf_1_failed:",
            "mov rdi, 1",
            "mov rax, 0",
            "vmcall",
            "cpuid_leaf_1_end:"
        );
        pub fn cpuid_leaf_1() {
            let vm = VmBuilder::new(
                NoEptVmState::new(unsafe {
                    extern "C" {
                        static cpuid_leaf_1_start: u8;
                        static cpuid_leaf_1_end: u8;
                    }
                    core::slice::from_raw_parts(
                        &cpuid_leaf_1_start as *const u8,
                        &cpuid_leaf_1_end as *const _ as usize
                            - &cpuid_leaf_1_start as *const _ as usize,
                    )
                }),
                1,
            )
            .expect("Failed to create vmbuilder.")
            .finalize()
            .expect("Failed to create vm.");
            vm.vcpu(0).unwrap().lock().vcpu_id = 0xba;
            vm.start_bsp().expect("Failed to start bsp.");
            assert_eq!(vm.join(), 0);
        }
    }

    pub mod pio {
        use core::arch::global_asm;

        // print 'Hello pio\n' and exit.
        global_asm!(
            "pio_print_start:",
            "in al, 3",
            // Hello\n
            "mov dx, 0x3f8",
            "mov al, 0x48",
            "out dx, al",
            "mov al, 0x65",
            "out dx, al",
            "mov al, 0x6c",
            "out dx, al",
            "mov al, 0x6c",
            "out dx, al",
            "mov al, 0x6f",
            "out dx, al",
            "mov al, 0x20",
            "out dx, al",
            "mov al, 0x70",
            "out dx, al",
            "mov al, 0x69",
            "out dx, al",
            "mov al, 0x6f",
            "out dx, al",
            "mov al, 0x20",
            "out dx, al",
            // hcall_exit(0);
            "mov rdi, 0",
            "mov rax, 0",
            "vmcall",
            "pio_print_end:",
        );
        pub fn pio_print() {
            super::run_vm::<0>(unsafe {
                extern "C" {
                    static pio_print_start: u8;
                    static pio_print_end: u8;
                }
                core::slice::from_raw_parts(
                    &pio_print_start as *const u8,
                    &pio_print_end as *const _ as usize - &pio_print_start as *const _ as usize,
                )
            });
        }

        // Test for out/in (e)a(x|l), dx instructions
        // Check PioQueueHandler in pio.rs that represents queuing operations.
        global_asm!(
            "pio_dx_port_start:",
            // out dx, (e)a(x|l)
            "mov dx, 0xbb",
            "mov al, 0x11",
            "out dx, al", // Out_DX_AL
            "mov ax, 0x2222",
            "out dx, ax", // Out_DX_AX
            "mov eax, 0x33333333",
            "out dx, eax", // Out_DX_EAX
            // in (e)a(x|l), dx
            "xor al, al",
            "in al, dx", // In_AL_DX
            "cmp al, 0x11",
            "jne pio_dx_port_failed",
            "xor ax, ax",
            "in ax, dx", // In_AX_DX
            "cmp ax, 0x2222",
            "jne pio_dx_port_failed",
            "xor eax, eax",
            "in eax, dx", // In_EAX_DX
            "cmp eax, 0x33333333",
            "jne pio_dx_port_failed",
            // hcall_exit(0);
            "mov rdi, 0",
            "mov rax, 0",
            "vmcall",
            "pio_dx_port_failed:",
            // hcall_exit(1); if failed
            "mov rdi, 1",
            "mov rax, 0",
            "vmcall",
            "pio_dx_port_end:",
        );
        pub fn pio_dx_port() {
            super::run_vm::<0>(unsafe {
                extern "C" {
                    static pio_dx_port_start: u8;
                    static pio_dx_port_end: u8;
                }
                core::slice::from_raw_parts(
                    &pio_dx_port_start as *const u8,
                    &pio_dx_port_end as *const _ as usize - &pio_dx_port_start as *const _ as usize,
                )
            });
        }

        // Test for out/in (e)a(x|l), imm8 instructions
        // Check PioQueueHandler in pio.rs that represents queuing operations.
        global_asm!(
            "pio_imm8_port_start:",
            // out imm8, (e)a(x|l)
            "mov al, 0x44",
            "out 0xbb, al", // Out_imm8_AL
            "mov ax, 0x5555",
            "out 0xbb, ax", // Out_imm8_AX
            "mov eax, 0x66666666",
            "out 0xbb, eax", // Out_imm8_EAX
            // in (e)a(x|l), imm8
            "xor al, al",
            "in al, 0xbb", // In_AL_Imm8
            "cmp al, 0x44",
            "jne pio_imm8_port_failed",
            "xor ax, ax",
            "in ax, 0xbb", // In_AX_Imm8
            "cmp ax, 0x5555",
            "jne pio_imm8_port_failed",
            "xor eax, eax",
            "in eax, 0xbb", // In_EAX_Imm8
            "cmp eax, 0x66666666",
            "jne pio_imm8_port_failed",
            // hcall_exit(0);
            "mov rdi, 0",
            "mov rax, 0",
            "vmcall",
            "pio_imm8_port_failed:",
            // hcall_exit(1); if failed
            "mov rdi, 1",
            "mov rax, 0",
            "vmcall",
            "pio_imm8_port_end:",
        );
        pub fn pio_imm8_port() {
            super::run_vm::<0>(unsafe {
                extern "C" {
                    static pio_imm8_port_start: u8;
                    static pio_imm8_port_end: u8;
                }
                core::slice::from_raw_parts(
                    &pio_imm8_port_start as *const u8,
                    &pio_imm8_port_end as *const _ as usize
                        - &pio_imm8_port_start as *const _ as usize,
                )
            });
        }

        // Test for outs(b|w|d), ins(b|w|d) instructions
        // Check PioQueueHandler in pio.rs that represents queuing operations.
        global_asm!(
            "pio_mem_start:",
            "mov dx, 0xbb",
            // outs(b|w|d)
            "lea rsi, [rip + pio_mem_byte]",
            "outsb", // Outsb_DX_m8
            "lea rsi, [rip + pio_mem_word]",
            "outsw", // Outsw_DX_m16
            "lea rsi, [rip + pio_mem_dword]",
            "outsd", // Outsd_DX_m32
            // ins(b|w|d)
            // gva of a writable region page
            "mov rdi, 0x2000",
            "cld",
            "mov byte ptr [rdi], 0",
            "insb", // Insb_m8_DX
            "cmp byte ptr [rdi - 1], 0x77",
            "jne pio_mem_failed",
            "mov word ptr [rdi], 0",
            "insw", // Insw_m16_DX
            "cmp word ptr [rdi - 2], 0x8888",
            "jne pio_mem_failed",
            "mov dword ptr [rdi], 0",
            "insd", // Insd_m32_DX
            "cmp dword ptr [rdi - 4], 0x99999999",
            "jne pio_mem_failed",
            // hcall_exit(0);
            "mov rdi, 0",
            "mov rax, 0",
            "vmcall",
            "pio_mem_failed:",
            // hcall_exit(1); if failed
            "mov rdi, 1",
            "mov rax, 0",
            "vmcall",
            "pio_mem_byte:",
            ".byte 0x77",
            "pio_mem_word:",
            ".2byte 0x8888",
            "pio_mem_dword:",
            ".4byte 0x99999999",
            "pio_mem_end:",
        );
        pub fn pio_mem() {
            super::run_vm::<0>(unsafe {
                extern "C" {
                    static pio_mem_start: u8;
                    static pio_mem_end: u8;
                }
                core::slice::from_raw_parts(
                    &pio_mem_start as *const u8,
                    &pio_mem_end as *const _ as usize - &pio_mem_start as *const _ as usize,
                )
            });
        }
    }
    pub mod msr {
        use core::arch::global_asm;

        // Test for msr
        global_asm!(
            "msr_start:",
            "mov rcx, 0xabc",
            "mov rdx, 0xFFFFFFFF11112222",
            "mov rax, 0xFFFFFFFF33334444",
            "wrmsr",
            "mov rdx, 0",
            "mov rax, 0",
            "rdmsr",
            "cmp rdx, 0x11112222",
            "jne msr_failed",
            "cmp rax, 0x33334444",
            "jne msr_failed",
            // hcall_exit(0);
            "mov rdi, 0",
            "mov rax, 0",
            "vmcall",
            "msr_failed:",
            "mov rdi, 1",
            "mov rax, 0",
            "vmcall",
            "msr_end:",
        );
        pub fn msr() {
            super::run_vm::<0>(unsafe {
                extern "C" {
                    static msr_start: u8;
                    static msr_end: u8;
                }
                core::slice::from_raw_parts(
                    &msr_start as *const u8,
                    &msr_end as *const _ as usize - &msr_start as *const _ as usize,
                )
            });
        }
    }
}
