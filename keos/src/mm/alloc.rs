//! Heap allocator for KeOS.

use crate::addressing::{Va, PAGE_MASK};
use crate::{mm::slob_allocator::SlobAllocator, spin_lock::SpinLock};
use core::alloc::{GlobalAlloc, Layout};

use super::ContigPages;

/// Out-of memory handler
#[alloc_error_handler]
fn oom(_l: Layout) -> ! {
    unreachable!()
}

pub struct Allocator(SpinLock<SlobAllocator>);

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() >= 65536 {
            if let Some(pg) = crate::mm::ContigPages::new_with_align(
                (layout.size() + PAGE_MASK) & !PAGE_MASK,
                layout.align(),
            ) {
                let va = pg.va().into_usize();
                core::mem::forget(pg);
                return va as *mut u8;
            } else {
                oom(layout)
            }
        }
        // perform layout adjustments
        let (size, align) = SlobAllocator::align_to_slob_node(layout);
        let mut allocator = self.0.lock();

        loop {
            if let Some((region, alloc_start)) = allocator.find_region(size, align) {
                let alloc_end = alloc_start.checked_add(size).expect("overflow");
                let excess_size = region.end_addr() - alloc_end;
                if excess_size > 0 {
                    allocator.add_free_region(alloc_end, excess_size);
                }
                return alloc_start as *mut u8;
            } else if let Some(pg) = crate::mm::ContigPages::new(size) {
                let va = pg.va().into_usize();
                allocator.add_free_region(va, pg.cnt * 0x1000);
                core::mem::forget(pg);
            } else {
                oom(layout);
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if layout.size() >= 65536 {
            ContigPages::from_va(
                Va::new(ptr as usize).unwrap(),
                (layout.size() + PAGE_MASK) & !PAGE_MASK,
            );
        } else {
            return; // BUG: slob has a bug. Mitigate by not freeing now.
            // perform layout adjustments
            let (size, _) = SlobAllocator::align_to_slob_node(layout);

            self.0.lock().add_free_region(ptr as usize, size)
        }
    }
}

#[global_allocator]
pub(crate) static ALLOCATOR: Allocator = Allocator(SpinLock::new(SlobAllocator::new()));
