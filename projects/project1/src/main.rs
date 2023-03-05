// This is test & bootstrap implementation.
// This file will be overwritten when grading.
#![no_std]
#![no_main]
#![deny(unsafe_code)]

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
    keos::thread::scheduler::set_scheduler(project1::rr::RoundRobin::new());
    keos::do_tests(&[
        &round_robin::balance,
        &round_robin::balance2,
        &round_robin::affinity,
        &round_robin::balance,
        &round_robin::balance2,
        &round_robin::affinity,
        &round_robin::reschedule,
        &page_table::simple,
        &page_table::complicate,
    ]);
}

mod round_robin {
    use alloc::{collections::VecDeque, format, string::ToString, sync::Arc, vec::Vec};
    use core::sync::atomic::{AtomicUsize, Ordering};
    use keos::{
        intrinsics::cpuid,
        sync::SpinLock,
        thread::{scheduler::Scheduler, Thread, ThreadBuilder},
        MAX_CPU,
    };

    pub fn balance() {
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

    pub fn balance2() {
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

    pub fn affinity() {
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

    pub fn reschedule() {
        const JOB_CNT: usize = 50;
        let cnt = Arc::new(AtomicUsize::new(0));
        // Spawn JOB_CNT threads and spins. Check whether every thread is scheduled.
        let handles = (0..JOB_CNT)
            .map(|i| {
                let c = cnt.clone();
                ThreadBuilder::new("Busy Waiter").spawn(move || {
                    loop {
                        let v = c.load(Ordering::SeqCst);
                        if v == i {
                            c.fetch_add(1, Ordering::SeqCst);
                            break;
                        }
                    }
                    while c.load(Ordering::SeqCst) != JOB_CNT {}
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            assert_eq!(handle.join(), 0);
        }
        assert_eq!(cnt.load(Ordering::SeqCst), JOB_CNT);
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
