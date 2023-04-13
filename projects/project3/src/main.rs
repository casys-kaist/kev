// This is test & bootstrap implementation.
// This file will be overwritten when grading.
#![no_std]
#![no_main]

#[allow(unused_imports)]
#[macro_use]
extern crate keos;

extern crate project1;
extern crate project2;
extern crate alloc;

use project1::rr::RoundRobin;

#[allow(unsafe_code)]
#[no_mangle]
pub unsafe fn main() {
    keos::thread::scheduler::set_scheduler(RoundRobin::new());
    unsafe { kev::start_vmx_on_cpu().expect("Failed to initialize VMX.") }
    keos::do_tests(&[
        &tests::part1::ept::simple,
        &tests::part1::ept::complicate,
        &tests::part1::ept::check_huge_translation,
        &tests::part1::mmio::mmio_print,
        &tests::part2::run_keos,
    ]);
}

#[allow(unsafe_code)]
#[no_mangle]
pub unsafe fn ap_main() {
    unsafe { kev::start_vmx_on_cpu().expect("Failed to initialize VMX.") }
}

mod tests {
    pub mod part1 {
        pub mod ept {
            use alloc::vec::Vec;
            use keos::{
                addressing::PAGE_SHIFT,
                mm::Page,
            };
            use keos::addressing::Pa;
            use keos::thread::Thread;
            use kev::{Probe, vm::Gpa};
            use kev::vm::Gva;
            use kev::vmcs::{Field, Vmcs};
            use project1::page_table::{Pde, PdeFlags, Pdpe, PdpeFlags, Pml4e, Pml4eFlags};
            use project3::ept::{EptMappingError, EptPteFlags, ExtendedPageTable, Permission};

            fn check_insert_one(pgtbl: &mut ExtendedPageTable, gpa: usize, permission: Permission) {
                let gpa = Gpa::new(gpa).unwrap();
                let pg = Page::new().unwrap();
                let pa = pg.pa();
                assert!(pgtbl.map(gpa, pg, permission).is_ok());
                let pte = pgtbl.walk(gpa);
                assert!(pte.is_ok());
                let pte = pte.unwrap();
                assert_eq!(pte.pa().unwrap(), pa);
                assert_eq!(
                    pte.flags().intersection(EptPteFlags::FULL),
                    EptPteFlags::from_bits_truncate(permission.bits())
                );
            }

            fn check_remove_one(pgtbl: &mut ExtendedPageTable, gpa: usize) {
                let gpa = Gpa::new(gpa).unwrap();
                assert!(pgtbl.unmap(gpa).is_ok());
                assert!(matches!(pgtbl.walk(gpa), Err(EptMappingError::NotExist)));
            }

            pub fn simple() {
                let mut pgtbl = ExtendedPageTable::new();
                assert!(pgtbl
                    .map(
                        Gpa::new(0x1234000).unwrap(),
                        Page::new().unwrap(),
                        Permission::READ,
                    )
                    .is_ok());
                assert_eq!(
                    pgtbl.map(
                        Gpa::new(0x1234000).unwrap(),
                        Page::new().unwrap(),
                        Permission::READ,
                    ),
                    Err(EptMappingError::Duplicated)
                );
                assert_eq!(
                    pgtbl.map(
                        Gpa::new(0x1234123).unwrap(),
                        Page::new().unwrap(),
                        Permission::READ,
                    ),
                    Err(EptMappingError::Unaligned)
                );
                assert_eq!(
                    pgtbl.unmap(Gpa::new(0x1235000).unwrap()).map(|_| ()),
                    Err(EptMappingError::NotExist)
                );
                assert!(pgtbl.unmap(Gpa::new(0x1234000).unwrap()).is_ok());
            }

            pub fn complicate() {
                let mut pgtbl = ExtendedPageTable::new();

                let addr = 0x1234000;
                // Check combination of permissions
                for i in 1..8 {
                    check_insert_one(&mut pgtbl, addr, Permission::from_bits_truncate(i));
                    check_remove_one(&mut pgtbl, addr);
                }

                let permission = Permission::READ | Permission::EXECUTABLE;
                let mut addrs: [usize; 5] = [0xeeee_ffff_ffff_f000; 5];
                for (i, p) in addrs.iter_mut().enumerate() {
                    if i == 0 {
                        continue;
                    }
                    *p = *p ^ (1 << (PAGE_SHIFT + 9 * (i - 1)));
                    // 0xeeee_ffff_ffff_f000
                    // 0xeeee_ffff_ffff_e000
                    // 0xeeee_ffff_ffdf_f000
                    // 0xeeee_ffff_bfff_f000
                    // 0xeeee_ff7f_ffff_f000
                }

                for (i, addr) in addrs.iter().enumerate() {
                    check_insert_one(&mut pgtbl, *addr, permission);
                    if i != 0 {
                        // Check the previous map not to be forgotten if additional mapping created
                        assert!(pgtbl.walk(Gpa::new(addrs[i - 1]).unwrap()).is_ok());
                    }
                }
                for (i, addr) in addrs.iter().enumerate() {
                    if i == 0 {
                        continue;
                    };
                    check_remove_one(&mut pgtbl, *addr);
                    // Check the first map not to be forgotten if other mapping removed
                    assert!(pgtbl.walk(Gpa::new(addrs[0]).unwrap()).is_ok());
                }
                check_remove_one(&mut pgtbl, addrs[0]);
            }


            pub fn check_huge_translation() {
                let _p = Thread::pin();
                let mut ept = ExtendedPageTable::new();
                let vmcs = Vmcs::activate(&mut Vmcs::new()).unwrap();

                vmcs.write(Field::GuestCr3, 0x1000).unwrap();
                let pml4_page = Page::new().unwrap();
                let pml4 = unsafe { pml4_page.va().as_mut::<[Pml4e; 512]>().unwrap() };
                pml4[0].set_pa(Pa::new(0x2000).unwrap()).unwrap().set_perm(Pml4eFlags::P | Pml4eFlags::RW);
                assert!(ept.map(Gpa::new(0x1000).unwrap(), pml4_page, Permission::all()).is_ok());

                let pdp_page = Page::new().unwrap();
                let pdp = unsafe { pdp_page.va().as_mut::<[Pdpe; 512]>().unwrap() };
                pdp[0].set_pa(Pa::new(0x3000).unwrap()).unwrap().set_perm(PdpeFlags::P | PdpeFlags::RW);
                assert!(ept.map(Gpa::new(0x2000).unwrap(), pdp_page, Permission::all()).is_ok());

                let pd_page = Page::new().unwrap();
                let pd = unsafe { pd_page.va().as_mut::<[Pde; 512]>().unwrap() };
                pd[1].set_pa(Pa::new(0x200000).unwrap()).unwrap().set_perm(PdeFlags::P | PdeFlags::RW | PdeFlags::PS);
                assert!(ept.map(Gpa::new(0x3000).unwrap(), pd_page, Permission::all()).is_ok());

                let mut pgs = (0..512).map(|_| Page::new().unwrap()).collect::<Vec<Page>>();
                let mut pas = pgs.iter().map(|pg| pg.pa()).collect::<Vec<Pa>>();
                for i in (0x200_000..0x400_000).step_by(0x1000) {
                    assert!(ept.map(Gpa::new(i).unwrap(), pgs.pop().unwrap(), Permission::all()).is_ok());
                }

                for i in (0x200_000..0x400_000).step_by(0x1000) {
                    let o = ept.gva2hpa(&vmcs, Gva::new(i).unwrap());
                    assert!(o.is_some());
                    assert_eq!(o.unwrap(), pas.pop().unwrap());
                }
            }
        }


        use kev::vm::VmBuilder;
        use project3::simple_ept_vm::SimpleEptVmState;

        fn run_code_on_vm<const EXPECTED: i32>(code: &'static [u8]) {
            let vm = VmBuilder::new(SimpleEptVmState::new(code), 1)
                .expect("Failed to create vmbuilder.")
                .finalize()
                .expect("Failed to create vm.");
            vm.start_bsp().expect("Failed to start bsp.");
            assert_eq!(vm.join(), EXPECTED);
        }

        pub mod mmio {
            use core::arch::global_asm;
            use project2::PRINTER_PROXY;

            // print 'Hello mmio!\n' and exit.
            global_asm!(
                "mmio_print_start:",
                "mov rax, 0xcafe0000",
                "lea rbx, [rip+mmio_print_buf]",
                "mov QWORD PTR [rax], rbx", // buffer address
                "mov rax, 0xcafe0008",
                "mov QWORD PTR [rax], 0xc", // buffer size
                "mov rax, 0xcafe0010",
                "mov QWORD PTR [rax], 0x1", // Ring doorbell
                "mov rdi, 0",
                "mov rax, 0",
                "vmcall",
                "mmio_print_buf:",
                ".byte 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x6d, 0x6d, 0x69, 0x6f, 0x21, 0xa",
                "mmio_print_end:",
            );
            pub fn mmio_print() {
                unsafe { PRINTER_PROXY.clear(); }
                super::run_code_on_vm::<0>(unsafe {
                    extern "C" {
                        static mmio_print_start: u8;
                        static mmio_print_end: u8;
                    }
                    core::slice::from_raw_parts(
                        &mmio_print_start as *const u8,
                        &mmio_print_end as *const _ as usize
                            - &mmio_print_start as *const _ as usize,
                    )
                });
                assert_eq!(unsafe { &PRINTER_PROXY }, "Hello mmio!\n");
            }
        }
    }
    pub mod part2 {
        use kev::vm::VmBuilder;
        use project3::keos_vm::VmState;

        pub fn run_keos() {
            // VM with 256 MiB memory.
            let vm = VmBuilder::new(
                VmState::new(256 * 1024).expect("Failed to crate vmstate"),
                1,
            )
            .expect("Failed to create vmbuilder.")
            .finalize()
            .expect("Failed to create vm.");
            vm.start_bsp().expect("Failed to start bsp.");
            vm.join();
        }
    }
}
