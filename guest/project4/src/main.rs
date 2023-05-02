// This is test & bootstrap implementation.
// This file will be overwritten when grading.
#![no_std]
#![no_main]

extern crate alloc;
#[allow(unused_imports)]
#[macro_use]
extern crate keos;
extern crate project1;

mod simple_virtio;

#[allow(unsafe_code)]
#[no_mangle]
pub unsafe fn ap_main() {}

#[allow(unsafe_code)]
#[no_mangle]
pub unsafe fn main() {
    println!("Hello guest os!");
    keos::thread::scheduler::set_scheduler(project1::rr::RoundRobin::new());
    keos::do_tests(&[
        &virtio_check::check_blockio,
        &virtio_check::check_blockio_batching,
        &round_robin::check_balancing1,
        &round_robin::check_balancing2,
        &round_robin::check_affinity,
        &round_robin::check_balancing1,
        &round_robin::check_balancing2,
        &round_robin::check_affinity,
        &page_table::simple,
        &page_table::complicate,
    ]);
}

mod virtio_check {
    use alloc::vec;
    use core::str::from_utf8;
    use keos::fs::{Disk, Sector};
    use crate::simple_virtio::VirtIoDisk;

    const DISK_CONTENT: &str = "Welcome to the KeV project.\n\n\
            Virtualization is an increasingly ubiquitous feature of modern computer systems, and a rapidly evolving part of the system stack. Hardware vendors are adding new features to support more efficient virtualization, OS designs are adapting to perform better in VMs, and VMs are an essential component in cloud computing. Thus, understanding how VMs work is essential to a complete education in computer systems.\n\n\
            In this project, you will skim through the basic components that runs on real virtual machine monitor like KVM. From what you learn, you will build your own type 2 hypervisor and finally extend the hypervisor as an open-ended course project.\n\n\
            In KeV project, we will not bother you from the time-consuming edge case handling and the hidden test cases. The score that you see when run the grading scripts is your final score. We want to keep this project as easy as possible. If you have suggestions on how we can reduce the unnecessary overhead of assignments, cutting them down to the important underlying issues, please let us know.\n";

    // Get virtual disk size
    fn get_disk_size(disk: &VirtIoDisk) -> usize {
        let mut read_buf = [0; 512];
        let mut check1 = [0xff; 512];
        let mut check2 = [0xee; 512];

        let mut idx = 0;
        loop {
            read_buf.fill(0xff);
            assert!(disk.read(Sector(idx), &mut read_buf).is_ok());
            if read_buf.eq(&check1) {
                read_buf.fill(0xee);
                assert!(disk.read(Sector(idx), &mut read_buf).is_ok());
                if read_buf.eq(&check2) {
                    break;
                }
            }
            idx += 1;
        }
        idx -= 1;
        assert!(idx >= 0);
        assert!(disk.read(Sector(idx), &mut check1).is_ok());
        assert!(disk.read(Sector(idx), &mut check2).is_ok());
        let mut off = 0;
        for i in 0..512 {
            if check1[i] != check2[i] {
                off = i;
                break;
            }
        }
        idx * 512 + off
    }

    pub fn check_blockio() {
        let mut read_buf = [0 as u8; 512];
        let mut write_buf = [0 as u8; 512];
        let mut disk = VirtIoDisk::new().unwrap();

        // Test virtio read operation.
        for (idx, off) in (0..DISK_CONTENT.len()).step_by(512).enumerate() {
            let start = off;
            let end = (off + 512).min(DISK_CONTENT.len());
            read_buf.fill(0);

            assert!(disk.read(Sector(idx), &mut read_buf).is_ok());
            assert_eq!(
                &from_utf8(&read_buf).unwrap()[..(end - start)],
                &DISK_CONTENT[start..end]
            );
        }

        // Test virtio write operation.
        write_buf.fill(77);
        read_buf.fill(0);

        assert!(disk.write(Sector(1), &mut write_buf).is_ok());
        assert!(disk.read(Sector(1), &mut read_buf).is_ok());
        assert_eq!(
            from_utf8(&read_buf).unwrap(),
            from_utf8(&write_buf).unwrap()
        );

        // Check that other sectors are not corrupted
        for (idx, off) in (0..DISK_CONTENT.len()).step_by(512).enumerate() {
            if idx == 1 {
                continue;
            }
            let start = off;
            let end = (off + 512).min(DISK_CONTENT.len());
            read_buf.fill(0);

            assert!(disk.read(Sector(idx), &mut read_buf).is_ok());
            assert_eq!(
                &from_utf8(&read_buf).unwrap()[..(end - start)],
                &DISK_CONTENT[start..end]
            );
        }

        // Restore disk contents
        assert!(disk.write(Sector(1),
                           &DISK_CONTENT[512..1024].as_bytes().try_into().unwrap()).is_ok());

        disk.finish();
    }

    pub fn check_blockio_batching() {
        let mut disk = VirtIoDisk::new().unwrap();
        let disk_len = get_disk_size(&disk);
        let mut read_buf = vec![0; (disk_len + 511) / 512 * 512];
        let mut read_buf1 = vec![0; (disk_len + 511) / 512 * 512];
        let write_buf = vec![77; (disk_len + 511) / 512 * 512];

        // Test virtio read batch.
        assert!(disk.read_many(Sector(0), &mut read_buf).is_ok());
        assert_eq!(&from_utf8(&read_buf).unwrap()[..DISK_CONTENT.len()], DISK_CONTENT);

        // Test virtio write batch
        assert!(disk.write_many(Sector(0), &write_buf).is_ok());
        assert!(disk.read_many(Sector(0), &mut read_buf1).is_ok());
        assert_eq!(
            from_utf8(&read_buf1[..disk_len]).unwrap(),
            from_utf8(&write_buf[..disk_len]).unwrap()
        );

        // Restore contents
        assert!(disk.write_many(Sector(0), &mut read_buf).is_ok());
        disk.finish();
    }
}

mod round_robin {
    use alloc::{collections::VecDeque, format, string::ToString, sync::Arc};
    use keos::{
        intrinsics::cpuid,
        sync::SpinLock,
        thread::{scheduler::Scheduler, Thread, ThreadBuilder},
        MAX_CPU,
    };

    pub fn check_balancing1() {
        let counts = Arc::new(SpinLock::new([0; MAX_CPU]));
        let total = 500;
        let mut handles = VecDeque::new();
        for i in 0..total {
            let counts = counts.clone();
            let handle = ThreadBuilder::new(format!("t{}", i)).spawn(move || {
                counts.lock()[cpuid()] += 1;
            });
            handles.push_back(handle);
        }
        while let Some(handle) = handles.pop_front() {
            assert_eq!(handle.join(), 0);
        }

        // Panic if one core consumes all tasks.
        let mut check_total = 0;
        for count in *counts.lock() {
            assert_ne!(count, total);
            check_total += count;
        }
        // Check every task is successfully finished.
        assert_eq!(check_total, total);
    }

    pub fn check_balancing2() {
        let cnt = Arc::new(SpinLock::new(0));
        let scheduler = Arc::new(project1::rr::RoundRobin::new());
        let mut handles = VecDeque::new();
        for i in 0..MAX_CPU {
            let cnt = cnt.clone();
            let scheduler = scheduler.clone();
            let handle = ThreadBuilder::new(format!("t{}", i)).spawn(move || {
                // Pin all cores not to be scheduled.
                let _p = Thread::pin();
                let cid = cpuid();
                {
                    *cnt.lock() += 1;
                }
                while *cnt.lock() < MAX_CPU {}

                // Generate N-1 Tasks and pull them on a single core.
                for i in 0..MAX_CPU {
                    if i != cid {
                        let thread = Thread::new(cid.to_string());
                        scheduler.push_to_queue(thread);
                        *cnt.lock() += 1;
                        while *cnt.lock() < (2 + i) * MAX_CPU {}
                    } else {
                        while *cnt.lock() != (2 + i) * MAX_CPU - 1 {}
                        for _ in 0..MAX_CPU - 1 {
                            assert!(scheduler.next_to_run().is_some());
                        }
                        assert!(scheduler.next_to_run().is_none());
                        *cnt.lock() += 1;
                    }
                }
            });
            handles.push_back(handle);
        }
        while let Some(handle) = handles.pop_front() {
            handle.join();
        }
    }

    pub fn check_affinity() {
        let cnt = Arc::new(SpinLock::new(0));
        let scheduler = Arc::new(project1::rr::RoundRobin::new());
        let mut handles = VecDeque::new();
        for i in 0..MAX_CPU {
            // Diable all cores' interrupt.
            let cnt = cnt.clone();
            let scheduler = scheduler.clone();
            let handle = ThreadBuilder::new(format!("t{}", i)).spawn(move || {
                let _p = Thread::pin();
                let cid = cpuid();
                {
                    *cnt.lock() += 1;
                }
                while *cnt.lock() < MAX_CPU {}

                // Now, all cores pushed a dummy thread into their run queue one by one.
                loop {
                    let mut c = cnt.lock();
                    if *c >= 5 * MAX_CPU {
                        break;
                    } else if *c % MAX_CPU == cid {
                        scheduler.push_to_queue(Thread::new(cid.to_string()));
                        *c += 1;
                    }
                }

                // Check all cores' runqueue state.
                loop {
                    let mut c = cnt.lock();
                    // Because each core pushes the thread with same frequency, threads MUST not be moved between queues.
                    if *c == 9 * MAX_CPU {
                        break;
                    } else if MAX_CPU - 1 - *c % MAX_CPU == cid {
                        assert_eq!(
                            scheduler
                                .next_to_run()
                                .and_then(|th| th.name.parse::<usize>().ok())
                                .unwrap(),
                            cid,
                        );
                        *c += 1;
                    }
                }
            });
            handles.push_back(handle);
        }
        while let Some(handle) = handles.pop_front() {
            assert_eq!(handle.join(), 0);
        }
    }
}

mod page_table {
    use keos::{
        addressing::{Va, PAGE_SHIFT},
        mm::Page,
    };
    use project1::page_table::{PageTable, PageTableMappingError, Permission, PteFlags};

    fn check_insert_one(pgtbl: &mut PageTable, va: usize, permission: Permission) {
        let va = Va::new(va).unwrap();
        let pg = Page::new().unwrap();
        let pa = pg.pa();
        assert!(pgtbl.map(va, pg, permission).is_ok());
        let pte = pgtbl.walk(va);
        assert!(pte.is_ok());
        let pte = pte.unwrap();
        assert_eq!(pte.pa().unwrap(), pa);
        let mut expected = PteFlags::empty();
        if !permission.is_empty() {
            expected |= PteFlags::P;
        }
        if permission.contains(Permission::WRITE) {
            expected |= PteFlags::RW;
        }
        if permission.contains(Permission::USER) {
            expected |= PteFlags::US;
        }
        if !permission.contains(Permission::EXECUTABLE) {
            expected |= PteFlags::XD;
        }
        assert_eq!(pte.flags(), expected);
    }

    fn check_remove_one(pgtbl: &mut PageTable, va: usize) {
        let va = Va::new(va).unwrap();
        assert!(pgtbl.unmap(va).is_ok());
        assert!(matches!(
            pgtbl.walk(va),
            Err(PageTableMappingError::NotExist)
        ));
    }

    pub fn simple() {
        let mut pgtbl = PageTable::new();
        assert!(pgtbl
            .map(
                Va::new(0x1234000).unwrap(),
                Page::new().unwrap(),
                Permission::READ,
            )
            .is_ok());
        assert_eq!(
            pgtbl.map(
                Va::new(0x1234000).unwrap(),
                Page::new().unwrap(),
                Permission::READ,
            ),
            Err(PageTableMappingError::Duplicated)
        );
        assert_eq!(
            pgtbl.map(
                Va::new(0x1234123).unwrap(),
                Page::new().unwrap(),
                Permission::READ,
            ),
            Err(PageTableMappingError::Unaligned)
        );
        assert_eq!(
            pgtbl.unmap(Va::new(0x1235000).unwrap()).map(|_| ()),
            Err(PageTableMappingError::NotExist)
        );
        assert!(pgtbl.unmap(Va::new(0x1234000).unwrap()).is_ok());
    }

    pub fn complicate() {
        let mut pgtbl = PageTable::new();

        let addr = 0x1234000;
        // Check combination of permissions
        for i in 1..0x10 {
            if !Permission::from_bits_truncate(i).intersects(Permission::READ | Permission::WRITE) {
                continue;
            }
            check_insert_one(&mut pgtbl, addr, Permission::from_bits_truncate(i));
            check_remove_one(&mut pgtbl, addr);
        }

        let permission = Permission::READ | Permission::EXECUTABLE;
        let mut addrs: [usize; 5] = [0xffff_ffff_ffff_f000; 5];
        for (i, p) in addrs.iter_mut().enumerate() {
            if i == 0 {
                continue;
            }
            *p = *p ^ (1 << (PAGE_SHIFT + 9 * (i - 1)));
            // 0xffff_ffff_ffff_f000
            // 0xffff_ffff_ffff_e000
            // 0xffff_ffff_ffdf_f000
            // 0xffff_ffff_bfff_f000
            // 0xffff_ff7f_ffff_f000
        }

        for (i, addr) in addrs.iter().enumerate() {
            check_insert_one(&mut pgtbl, *addr, permission);
            if i != 0 {
                // Check the previous map not to be forgotten if additional mapping created
                assert!(pgtbl.walk(Va::new(addrs[i - 1]).unwrap()).is_ok());
            }
        }
        for (i, addr) in addrs.iter().enumerate() {
            if i == 0 {
                continue;
            };
            check_remove_one(&mut pgtbl, *addr);
            // Check the first map not to be forgotten if other mapping removed
            assert!(pgtbl.walk(Va::new(addrs[0]).unwrap()).is_ok());
        }
        check_remove_one(&mut pgtbl, addrs[0]);
    }
}
