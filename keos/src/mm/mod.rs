//! Memory management including heap and physical memory.
mod alloc;
mod slob_allocator;

use crate::addressing::{Pa, Va, PAGE_MASK, PAGE_SHIFT};
use crate::sync::SpinLock;
use ::alloc::vec::Vec;
use abyss::boot::Regions;
use core::ops::Range;

/// Initialize the physical memory allocator.
pub unsafe fn init_mm(regions: Regions) {
    extern "C" {
        static __edata_end: u64;
    }

    let edata_end = Va::new(&__edata_end as *const _ as usize).unwrap();

    info!("initialize memory...");
    for region in regions.iter() {
        if region.usable {
            let Range { start, end } = region.addr;
            let (start, end) = (start.into_va().max(edata_end), end.into_va());
            if start < end {
                info!("    Arena: {:?}~{:?}", start, end);
                PALLOC.lock().foster(start, end);
            }
        }
    }
}

// Physical memory allocators.

struct Arena {
    // 0: used, 1: unused
    bitmap: &'static mut [u64],
    start: Va,
    end: Va,
}

impl Arena {
    const EMPTY: Option<Self> = None;
    fn set_used(&mut self, index: usize) {
        let (pos, ofs) = (index / 64, index % 64);
        debug_assert_ne!(self.bitmap[pos] & (1 << ofs), 0);
        self.bitmap[pos] &= !(1 << ofs);
        debug_assert_eq!(self.bitmap[pos] & (1 << ofs), 0);
    }
    fn set_unused(&mut self, index: usize) {
        let (pos, ofs) = (index / 64, index % 64);
        debug_assert_eq!(self.bitmap[pos] & (1 << ofs), 0);
        self.bitmap[pos] |= 1 << ofs;
        debug_assert_ne!(self.bitmap[pos] & (1 << ofs), 0);
    }
    fn alloc(&mut self, cnt: usize, align: usize) -> Option<Va> {
        let mut search = 0;
        while search < self.bitmap.len() * 64 {
            let (mut pos, ofs) = (search / 64, search % 64);
            // search first qword that contains one.
            if ofs % 64 == 0 {
                while self.bitmap[pos] == 0 {
                    pos += 1;
                }
                search = pos * 64;
            }

            let mut cont = 0;
            if align != 0
                && ((unsafe { self.start.into_usize() } >> PAGE_SHIFT) + search) % align != 0
            {
                search += 1;
            } else {
                let start = search;
                loop {
                    // Found!
                    if cont == cnt {
                        for i in start..start + cnt {
                            self.set_used(i);
                        }
                        return Some(self.start + (start << PAGE_SHIFT));
                    }

                    let (pos, ofs) = (search / 64, search % 64);
                    search += 1;
                    if self.bitmap[pos] & (1 << ofs) != 0 {
                        // usable
                        cont += 1;
                    } else {
                        break;
                    }
                }
            }
        }
        None
    }
    fn dealloc(&mut self, va: Va, cnt: usize) {
        let ofs = unsafe { (va.into_usize() - self.start.into_usize()) >> PAGE_SHIFT };
        for i in ofs..ofs + cnt {
            self.set_unused(i);
        }
    }
}

struct PhysicalAllocator {
    inner: [Option<Arena>; 64],
    max_idx: usize,
}

static PALLOC: SpinLock<PhysicalAllocator> = SpinLock::new(PhysicalAllocator {
    inner: [Arena::EMPTY; 64],
    max_idx: 0,
});

impl PhysicalAllocator {
    unsafe fn foster(&mut self, start: Va, end: Va) {
        // Calculate usable page of this region.
        let usable_pages = (end.into_usize() - start.into_usize()) >> PAGE_SHIFT;
        // Each region has alloc bitmap on first N pages.
        let bitmap = core::slice::from_raw_parts_mut(
            start.into_usize() as *mut u64,
            (usable_pages + 63) / 64,
        );
        let len = bitmap.len();
        bitmap.fill(u64::MAX);
        let mut arena = Arena { bitmap, start, end };
        // Pad front.
        for i in 0..((len * 8 + PAGE_MASK) >> PAGE_SHIFT) {
            arena.set_used(i);
        }
        // Pad back.
        for i in usable_pages..((usable_pages + 63) & !63) {
            arena.set_used(i);
        }
        self.inner[self.max_idx] = Some(arena);
        self.max_idx += 1;
    }
}

/// A Page representation.
pub struct Page {
    inner: ContigPages,
}

impl Page {
    /// Allocate a page.
    #[inline]
    pub fn new() -> Option<Self> {
        ContigPages::new(0x1000).map(|inner| Self { inner })
    }

    /// Get virtual address of this page.
    #[inline]
    pub fn va(&self) -> Va {
        self.inner.va
    }

    /// Get physical address of this page.
    #[inline]
    pub fn pa(&self) -> Pa {
        self.inner.va.into_pa()
    }

    /// Consumes the page, returning a pa of the page.
    ///
    /// After calling this function, the caller is responsible for the memory previously managed by the Page.
    /// In particular, the caller should properly release the page by calling the `Page::from_pa`.
    #[inline]
    pub fn into_raw(self) -> Pa {
        core::mem::ManuallyDrop::new(self).pa()
    }

    /// Constructs a page from a pa.
    ///
    /// For this to be safe, the pa must have been taken by `Page::into_raw`.
    ///
    /// ## Safety
    /// This function is unsafe because improper use may lead to memory problems. For example, a double-free may occur if the function is called twice on the same raw pointer.
    #[inline]
    pub unsafe fn from_pa(pa: Pa) -> Self {
        let va = pa.into_va();
        let allocator = PALLOC.lock();
        Page {
            inner: ContigPages {
                arena_idx: allocator
                    .inner
                    .iter()
                    .take(allocator.max_idx)
                    .enumerate()
                    .find_map(|(idx, arena)| {
                        let Arena { start, end, .. } = arena.as_ref().unwrap();
                        if (*start..*end).contains(&va) {
                            Some(idx)
                        } else {
                            None
                        }
                    })
                    .expect("Failed to find arena index."),
                va,
                cnt: 1,
            },
        }
    }

    /// Get reference of underlying slice of the Page.
    pub unsafe fn inner(&self) -> &[u8] {
        core::slice::from_raw_parts(self.va().into_usize() as *const u8, 4096)
    }

    /// Get mutable reference of underlying sliceof the Page.
    pub unsafe fn inner_mut(&mut self) -> &mut [u8] {
        core::slice::from_raw_parts_mut(self.va().into_usize() as *mut u8, 4096)
    }
}

/// A contiguous pages representation.
pub struct ContigPages {
    arena_idx: usize,
    va: Va,
    cnt: usize,
}

impl ContigPages {
    /// Allocate a page.
    #[inline]
    pub fn new(size: usize) -> Option<Self> {
        Self::new_with_align(size, 0x1000)
    }

    /// Allocate a page with align
    #[inline]
    pub fn new_with_align(size: usize, align: usize) -> Option<Self> {
        if size != 0 {
            // align up to page size.
            let cnt = (size + PAGE_MASK) >> PAGE_SHIFT;
            let mut allocator = PALLOC.lock();
            let max_idx = allocator.max_idx;
            for (arena_idx, arena) in allocator.inner.iter_mut().take(max_idx).enumerate() {
                if let Some(va) = arena.as_mut().unwrap().alloc(cnt, align >> PAGE_SHIFT) {
                    unsafe {
                        core::slice::from_raw_parts_mut(
                            va.into_usize() as *mut u64,
                            cnt * 0x1000 / core::mem::size_of::<u64>(),
                        )
                        .fill(0);
                    }
                    return Some(Self { arena_idx, va, cnt });
                }
            }
        }
        None
    }

    /// Get virtual address of this page.
    #[inline]
    pub fn va(&self) -> Va {
        self.va
    }

    /// Get physical address of this page.
    #[inline]
    pub fn pa(&self) -> Pa {
        self.va.into_pa()
    }

    /// Split the ContigPages into multiple pages.
    pub fn split(self) -> Vec<Page> {
        let mut out = Vec::new();
        let this = core::mem::ManuallyDrop::new(self);
        for i in 0..this.cnt {
            out.push(Page {
                inner: ContigPages {
                    arena_idx: this.arena_idx,
                    va: this.va + i * 0x1000,
                    cnt: 1,
                },
            })
        }
        out
    }
    /// Constructs a page from a va.
    ///
    /// ## Safety
    /// This function is unsafe because improper use may lead to memory problems. For example, a double-free may occur if the function is called twice on the same raw pointer.
    #[inline]
    pub unsafe fn from_va(va: Va, size: usize) -> Self {
        let allocator = PALLOC.lock();
        ContigPages {
            arena_idx: allocator
                .inner
                .iter()
                .take(allocator.max_idx)
                .enumerate()
                .find_map(|(idx, arena)| {
                    let Arena { start, end, .. } = arena.as_ref().unwrap();
                    if (*start..*end).contains(&va) {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .expect("Failed to find arena index."),
            va,
            cnt: size / 4096,
        }
    }
}

impl Drop for ContigPages {
    fn drop(&mut self) {
        let mut allocator = PALLOC.lock();
        allocator.inner[self.arena_idx]
            .as_mut()
            .unwrap()
            .dealloc(self.va, self.cnt);
    }
}

/// Align upwards. Returns the smallest x with alignment `align`
/// so that x >= addr. The alignment must be a power of 2.
pub fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}
