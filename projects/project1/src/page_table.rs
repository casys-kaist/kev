//! 4-level page table of x86_64.
//!
//! Implement x86_64's 4-level page table scheme.
//! More specifically, implement [`PageTable::map`], [`PageTable::unmap`] and [`PageTable::walk`].
//!
//! ## Background
//! ### Memory Protection
//! One of the main role of operating system is resource abstraction.
//! From now, you build an abstraction for CPU resources.
//! The other important resource of computer system is memory.
//!
//! Each process has their own memory. Each process's memory must not visible from others.
//! For example, your web browser should not be able to access the memory of your music player.
//! To do so, the harware introduce the memory protection mechanism to isolate the memory between processes.
//!
//! ### Virtual Memory
//! The concept of virtual memory is to abstract memory addresses from the underlying physical storage device.
//! Instead of accessing the storage device directly, it is authenticated and translated through the memory management unit (MMU).
//! To distinguish the two types of addresses, the pre-translation address is called a virtual address, and the post-translation address is called a physical address.
//! One important difference between these two types of addresses is that the physical addresses are unique and always refer to the same memory location.
//! For virtual addresses, two different virtual addresses can refer to the same physical address. The same virtual address can also refer to different physical addresses.
//!
//! ### Paging
//! Paging is a technique that divides physical and virtual memory space into small chunks of the same size. Each lump is called a page, and it's typically 4096 bytes.
//! The mapping of physical and virtual memory spaces is managed through the page table.
//! Typically, the currently active page table is managed through a register on a special CPU (e.g. cr3 in x86_64).
//!
//! ## Page table
//! For each memory access, the CPU translates the virtual memory address to the physical memory address through the page table.
//! Because it is inefficient to check the page table for each conversion, the CPU stores the previous results in a cache called Translation Lookaside Buffer (TLB).
//! The page table is also used to set attributes such as access permissions (e.g. read/write) for each page.
//! Note that attributes of all levels are **AND**ed.
//!
//! ## Paging in x86_64
//! x86_64 uses a 4096-byte page and a 4-level page table. Each table has 4096 bytes, the same size as the page, and an entry of table is 8 bytes.
//! Therefore, the 4-level page table can cover up to 48bit physical address.
//!
//! The index for each level can be calculated from the virtual memory address:
//! ```
//! 63          48 47            39 38            30 29            21 20         12 11         0
//! +-------------+----------------+----------------+----------------+-------------+------------+
//! | Sign Extend |    Page-Map    | Page-Directory | Page-directory |  Page-Table |    Page    |
//! |             | Level-4 Offset |    Pointer     |     Offset     |   Offset    |   Offset   |
//! +-------------+----------------+----------------+----------------+-------------+------------+
//!               |                |                |                |             |            |
//!               +------- 9 ------+------- 9 ------+------- 9 ------+----- 9 -----+---- 12 ----+
//!                                           Virtual Address
//! ```
//!
//! A page must be page-aligned, that is, start on a virtual address evenly divisible by the page size.
//! Thus, the last 12 bits of a 64-bit virtual address is the page offset (or just offset). The upper bits are used to indicate the index in the page table.
//!
//! Each process has an independent set of user pages, which are those pages below the kernel base.
//! The set of kernel pages, on the other hand, is global, and thus remain in the same position regardless of what thread or process is running.
//! The kernel may access both user and kernel pages, but a user process may access only its own user pages.
//!
//! KeOS provides several useful functions for working with virtual addresses and physical addresses.
//! See [`Pa`] and [`Va`] for details.
//!
//! [`Pa`]: ../../keos/addressing/struct.Pa.html
//! [`Va`]: ../../keos/addressing/struct.Va.html
//!
//! ### Translation lookaside Buffer
//! The TLB entry is not updated when the content of page table is changed.
//! Therefore, the kernel must invalidates the updated entry from the TLB by calling a special CPU instruction.
//! In x86_64, `invlpg` removes the entry according to the specified entry.
//! You can use [`TLBInvalidate`] to invalidate the TLB entry.
//! Note that TLB is fully flushed when `cr3` is reloaded.
//!
//! [`TLBInvalidate`]: TLBInvalidate

use alloc::boxed::Box;
use core::ops::{Deref, DerefMut};
use keos::{
    addressing::{Pa, Va, PAGE_SHIFT},
    mm::Page,
};

/// Struct for invalidating the tlb entry.
pub struct TLBInvalidate(Va);

impl TLBInvalidate {
    /// Invalidate the underlying va.
    pub fn invalidate(self) {
        let va = core::mem::ManuallyDrop::new(self).0;

        unsafe {
            core::arch::asm!(
                "invlpg [{0}]",
                in(reg) va.into_usize(),
                options(nostack)
            );
        }
    }

    /// Forget this modification.
    ///
    /// # Safety
    /// TLB must be flushed by another way.
    pub unsafe fn forget(self) {
        core::mem::forget(self);
    }
}

impl Drop for TLBInvalidate {
    fn drop(&mut self) {
        panic!("TLB entry for {:?} is not invalidated.", self.0);
    }
}

/// A list specifying categories of page table operation error.
#[derive(Debug, PartialEq, Eq)]
pub enum PageTableMappingError {
    /// Unaligned address
    Unaligned,
    /// Not exist
    NotExist,
    /// Duplicated mapping
    Duplicated,
}

/// Page Map Level 4 entry.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Pml4e(pub usize);

impl Pml4e {
    /// Get a physical address pointed by this entry.
    #[inline]
    pub const fn pa(&self) -> Option<Pa> {
        todo!()
    }

    /// Get a flags this entry.
    #[inline]
    pub const fn flags(&self) -> Pml4eFlags {
        Pml4eFlags::from_bits_truncate(self.0)
    }

    /// Set physical address of this entry.
    ///
    /// # WARNING
    /// Permission of this entry is not changed.
    #[inline]
    pub fn set_pa(&mut self, pa: Pa) -> Result<&mut Self, PageTableMappingError> {
        let pa = unsafe { pa.into_usize() };
        if pa & 0xfff != 0 {
            Err(PageTableMappingError::Unaligned)
        } else {
            self.0 = pa | self.flags().bits() | Pml4eFlags::P.bits();
            Ok(self)
        }
    }

    /// Set a permission of this entry.
    #[inline]
    pub fn set_perm(&mut self, perm: Pml4eFlags) -> &mut Self {
        self.0 = self.pa().map(|n| unsafe { n.into_usize() }).unwrap_or(0) | perm.bits();
        self
    }

    /// Get a mutable reference of page directory pointer table pointed by this entry.
    #[inline]
    pub fn into_pdp_mut(&mut self) -> Result<&mut [Pdpe], PageTableMappingError> {
        let pa = self.pa().ok_or(PageTableMappingError::NotExist)?;
        if !self.flags().contains(Pml4eFlags::P) {
            return Err(PageTableMappingError::NotExist);
        }
        unsafe {
            Ok(core::slice::from_raw_parts_mut(
                pa.into_va().into_usize() as *mut Pdpe,
                512,
            ))
        }
    }

    /// Get a reference of page directory pointer table pointed by this entry.
    #[inline]
    pub fn into_pdp(&self) -> Result<&[Pdpe], PageTableMappingError> {
        let pa = self.pa().ok_or(PageTableMappingError::NotExist)?;
        if !self.flags().contains(Pml4eFlags::P) {
            return Err(PageTableMappingError::NotExist);
        }
        unsafe {
            Ok(core::slice::from_raw_parts(
                pa.into_va().into_usize() as *const Pdpe,
                512,
            ))
        }
    }
}

bitflags::bitflags! {
    /// Flags for pml4e.
    pub struct Pml4eFlags: usize {
        /// Present; must be 1 to reference a page-directory-pointer table
        const P = 1 << 0;
        /// Read/write; if 0, writes may not be allowed to the 512-GByte region controlled by this entry (see Section 4.6).
        const RW = 1 << 1;
        /// User/supervisor; if 0, user-mode accesses are not allowed to the 512-GByte region controlled by this entry (see Section 4.6)
        const US = 1 << 2;
        /// Page-level write-through; indirectly determines the memory type used to access the page-directory-pointer table referenced by this entry (see Section 4.9.2)
        const PWT = 1 << 3;
        /// Page-level cache disable; indirectly determines the memory type used to access the page-directory-pointer table referenced by this entry (see Section 4.9.2)
        const PCD = 1 << 4;
        /// Accessed; indicates whether this entry has been used for linear-address translation (see Section 4.8)
        const A = 1 << 5;
        #[doc(hidden)] const _IGN_6 = 1 << 6;
        #[doc(hidden)] const _REV_0 = 1 << 7;
        #[doc(hidden)] const _IGN_8 = 1 << 8;
        #[doc(hidden)] const _IGN_9 = 1 << 9;
        #[doc(hidden)] const _IGN_10 = 1 << 10;
        /// For ordinary paging, ignored; for HLAT paging, restart (if 1, linear-address translation is restarted with ordinary paging)
        const R = 1 << 11;
        #[doc(hidden)] const _IGN_52 = 1 << 52;
        #[doc(hidden)] const _IGN_53 = 1 << 53;
        #[doc(hidden)] const _IGN_54 = 1 << 54;
        #[doc(hidden)] const _IGN_55 = 1 << 55;
        #[doc(hidden)] const _IGN_56 = 1 << 56;
        #[doc(hidden)] const _IGN_57 = 1 << 57;
        #[doc(hidden)] const _IGN_58 = 1 << 58;
        #[doc(hidden)] const _IGN_59 = 1 << 59;
        #[doc(hidden)] const _IGN_60 = 1 << 60;
        #[doc(hidden)] const _IGN_61 = 1 << 61;
        #[doc(hidden)] const _IGN_62 = 1 << 62;
        /// If IA32_EFER.NXE = 1, execute-disable (if 1, instruction fetches are not allowed from the 512-GByte region controlled by this entry; see Section 4.6); otherwise, reserved (must be 0)
        const XD = 1 << 63;
    }
}

/// Page directory pointer table entry.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Pdpe(pub usize);

impl Pdpe {
    /// Get a physical address pointed by this entry.
    #[inline]
    pub const fn pa(&self) -> Option<Pa> {
        todo!()
    }

    /// Get a flags this entry.
    #[inline]
    pub const fn flags(&self) -> PdpeFlags {
        PdpeFlags::from_bits_truncate(self.0)
    }

    /// Set physical address of this entry.
    ///
    /// # WARNING
    /// Permission of this entry is not changed.
    #[inline]
    pub fn set_pa(&mut self, pa: Pa) -> Result<&mut Self, PageTableMappingError> {
        let pa = unsafe { pa.into_usize() };
        if pa & 0xfff != 0 {
            Err(PageTableMappingError::Unaligned)
        } else {
            self.0 = pa | self.flags().bits() | PdpeFlags::P.bits();
            Ok(self)
        }
    }

    /// Set a permission of this entry.
    #[inline]
    pub fn set_perm(&mut self, perm: PdpeFlags) -> &mut Self {
        self.0 = self.pa().map(|n| unsafe { n.into_usize() }).unwrap_or(0) | perm.bits();
        self
    }

    /// Get a mutable reference of page directory pointed by this entry.
    #[inline]
    pub fn into_pd_mut(&mut self) -> Result<&mut [Pde], PageTableMappingError> {
        let pa = self.pa().ok_or(PageTableMappingError::NotExist)?;
        if !self.flags().contains(PdpeFlags::P) {
            return Err(PageTableMappingError::NotExist);
        }
        unsafe {
            Ok(core::slice::from_raw_parts_mut(
                pa.into_va().into_usize() as *mut Pde,
                512,
            ))
        }
    }
    /// Get a reference of page directory pointed by this entry.
    #[inline]
    pub fn into_pd(&self) -> Result<&[Pde], PageTableMappingError> {
        let pa = self.pa().ok_or(PageTableMappingError::NotExist)?;
        if !self.flags().contains(PdpeFlags::P) {
            return Err(PageTableMappingError::NotExist);
        }
        unsafe {
            Ok(core::slice::from_raw_parts(
                pa.into_va().into_usize() as *const Pde,
                512,
            ))
        }
    }
}

bitflags::bitflags! {
    /// Flags for pdpe.
    pub struct PdpeFlags: usize {
        /// Present; must be 1 to reference a page directory
        const P = 1 << 0;
        /// Read/write; if 0, writes may not be allowed to the 1-GByte region controlled by this entry (see Section 4.6)
        const RW = 1 << 1;
        /// User/supervisor; if 0, user-mode accesses are not allowed to the 1-GByte region controlled by this entry (see Section 4.6)
        const US = 1 << 2;
        /// Page-level write-through; indirectly determines the memory type used to access the page directory referenced by this entry (see Section 4.9.2)
        const PWT = 1 << 3;
        /// Page-level cache disable; indirectly determines the memory type used to access the page directory referenced by this entry (see Section 4.9.2)
        const PCD = 1 << 4;
        /// Accessed; indicates whether this entry has been used for linear-address translation (see Section 4.8)
        const A = 1 << 5;
        #[doc(hidden)] const _IGN_6 = 1 << 6;
        #[doc(hidden)] const _REV_0 = 1 << 7;
        #[doc(hidden)] const _IGN_8 = 1 << 8;
        #[doc(hidden)] const _IGN_9 = 1 << 9;
        #[doc(hidden)] const _IGN_10 = 1 << 10;
        /// For ordinary paging, ignored; for HLAT paging, restart (if 1, linear-address translation is restarted with ordinary paging)
        const R = 1 << 11;
        #[doc(hidden)] const _IGN_52 = 1 << 52;
        #[doc(hidden)] const _IGN_53 = 1 << 53;
        #[doc(hidden)] const _IGN_54 = 1 << 54;
        #[doc(hidden)] const _IGN_55 = 1 << 55;
        #[doc(hidden)] const _IGN_56 = 1 << 56;
        #[doc(hidden)] const _IGN_57 = 1 << 57;
        #[doc(hidden)] const _IGN_58 = 1 << 58;
        #[doc(hidden)] const _IGN_59 = 1 << 59;
        #[doc(hidden)] const _IGN_60 = 1 << 60;
        #[doc(hidden)] const _IGN_61 = 1 << 61;
        #[doc(hidden)] const _IGN_62 = 1 << 62;
        /// If IA32_EFER.NXE = 1, execute-disable (if 1, instruction fetches are not allowed from the 1-GByte region controlled by this entry; see Section 4.6); otherwise, reserved (must be 0)
        const XD = 1 << 63;
    }
}

/// Page directory entry.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Pde(pub usize);

impl Pde {
    /// Get a physical address pointed by this entry.
    #[inline]
    pub const fn pa(&self) -> Option<Pa> {
        todo!()
    }

    /// Get a flags this entry.
    #[inline]
    pub const fn flags(&self) -> PdeFlags {
        PdeFlags::from_bits_truncate(self.0)
    }

    /// Set physical address of this entry.
    ///
    /// # WARNING
    /// Permission of this entry is not changed.
    #[inline]
    pub fn set_pa(&mut self, pa: Pa) -> Result<&mut Self, PageTableMappingError> {
        let pa = unsafe { pa.into_usize() };
        if pa & 0xfff != 0 {
            Err(PageTableMappingError::Unaligned)
        } else {
            self.0 = pa | self.flags().bits() | PdeFlags::P.bits();
            Ok(self)
        }
    }

    /// Set a permission of this entry.
    #[inline]
    pub fn set_perm(&mut self, perm: PdeFlags) -> &mut Self {
        self.0 = self.pa().map(|n| unsafe { n.into_usize() }).unwrap_or(0) | perm.bits();
        self
    }

    /// Get a mutable reference of page table pointed by this entry.
    #[inline]
    pub fn into_pt_mut(&mut self) -> Result<&mut [Pte], PageTableMappingError> {
        let pa = self.pa().ok_or(PageTableMappingError::NotExist)?;
        if !self.flags().contains(PdeFlags::P) {
            return Err(PageTableMappingError::NotExist);
        }
        unsafe {
            Ok(core::slice::from_raw_parts_mut(
                pa.into_va().into_usize() as *mut Pte,
                512,
            ))
        }
    }

    /// Get a reference of page table pointed by this entry.
    #[inline]
    pub fn into_pt(&self) -> Result<&[Pte], PageTableMappingError> {
        let pa = self.pa().ok_or(PageTableMappingError::NotExist)?;
        if !self.flags().contains(PdeFlags::P) {
            return Err(PageTableMappingError::NotExist);
        }
        unsafe {
            Ok(core::slice::from_raw_parts(
                pa.into_va().into_usize() as *const Pte,
                512,
            ))
        }
    }
}

bitflags::bitflags! {
    /// Flags for pde.
    pub struct PdeFlags: usize {
        /// Present; must be 1 to reference a page table
        const P = 1 << 0;
        /// Read/write; if 0, writes may not be allowed to the 2-MByte region controlled by this entry (see Section 4.6)
        const RW = 1 << 1;
        /// User/supervisor; if 0, user-mode accesses are not allowed to the 2-MByte region controlled by this entry (see Section 4.6)
        const US = 1 << 2;
        /// Page-level write-through; indirectly determines the memory type used to access the page table referenced by this entry (see Section 4.9.2)
        const PWT = 1 << 3;
        /// Page-level cache disable; indirectly determines the memory type used to access the page table referenced by this entry (see Section 4.9.2)
        const PCD = 1 << 4;
        /// Accessed; indicates whether this entry has been used for linear-address translation (see Section 4.8)
        const A = 1 << 5;
        /// Page size; indicates whether this entry is 2M page.
        const PS = 1 << 7;
        #[doc(hidden)] const _IGN_6 = 1 << 6;
        #[doc(hidden)] const _REV_0 = 1 << 7;
        #[doc(hidden)] const _IGN_8 = 1 << 8;
        #[doc(hidden)] const _IGN_9 = 1 << 9;
        #[doc(hidden)] const _IGN_10 = 1 << 10;
        /// For ordinary paging, ignored; for HLAT paging, restart (if 1, linear-address translation is restarted with ordinary paging)
        const R = 1 << 11;
        #[doc(hidden)] const _IGN_52 = 1 << 52;
        #[doc(hidden)] const _IGN_53 = 1 << 53;
        #[doc(hidden)] const _IGN_54 = 1 << 54;
        #[doc(hidden)] const _IGN_55 = 1 << 55;
        #[doc(hidden)] const _IGN_56 = 1 << 56;
        #[doc(hidden)] const _IGN_57 = 1 << 57;
        #[doc(hidden)] const _IGN_58 = 1 << 58;
        #[doc(hidden)] const _IGN_59 = 1 << 59;
        #[doc(hidden)] const _IGN_60 = 1 << 60;
        #[doc(hidden)] const _IGN_61 = 1 << 61;
        #[doc(hidden)] const _IGN_62 = 1 << 62;
        /// If IA32_EFER.NXE = 1, execute-disable (if 1, instruction fetches are not allowed from the 2-MByte region controlled by this entry; see Section 4.6); otherwise, reserved (must be 0)
        const XD = 1 << 63;
    }
}

/// Page table entry.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Pte(pub usize);

impl Pte {
    /// Get a physical address pointed by this entry.
    #[inline]
    pub const fn pa(&self) -> Option<Pa> {
        todo!()
    }

    /// Get a flags this entry.
    #[inline]
    pub const fn flags(&self) -> PteFlags {
        PteFlags::from_bits_truncate(self.0)
    }

    /// Set physical address of this entry.
    ///
    /// # WARNING
    /// Permission of this entry is not changed.
    #[inline]
    pub fn set_pa(&mut self, pa: Pa) -> Result<&mut Self, PageTableMappingError> {
        let pa = unsafe { pa.into_usize() };
        if pa & 0xfff != 0 {
            Err(PageTableMappingError::Unaligned)
        } else {
            self.0 = pa | self.flags().bits() | PteFlags::P.bits();
            Ok(self)
        }
    }

    /// Set a permission of this entry.
    #[inline]
    pub fn set_perm(&mut self, perm: PteFlags) -> &mut Self {
        self.0 = self.pa().map(|n| unsafe { n.into_usize() }).unwrap_or(0) | perm.bits();
        self
    }
}

bitflags::bitflags! {
    /// Flags for pte.
    pub struct PteFlags: usize {
        /// Present; must be 1 to map a 4-KByte page
        const P = 1 << 0;
        /// Read/write; if 0, writes may not be allowed to the 4-KByte page referenced by this entry (see Section 4.6)
        const RW = 1 << 1;
        /// User/supervisor; if 0, user-mode accesses are not allowed to the 4-KByte page referenced by this entry (see Section 4.6)
        const US = 1 << 2;
        /// Page-level write-through; indirectly determines the memory type used to access the 4-KByte page referenced by this entry (see Section 4.9.2)
        const PWT = 1 << 3;
        /// Page-level cache disable; indirectly determines the memory type used to access the 4-KByte page referenced by this entry (see Section 4.9.2)
        const PCD = 1 << 4;
        /// Accessed; indicates whether software has accessed the 4-KByte page referenced by this entry (see Section 4.8)
        const A = 1 << 5;
        /// Dirty; indicates whether software has written to the 4-KByte page referenced by this entry (see Section 4.8)
        const D = 1 << 6;
        /// Indirectly determines the memory type used to access the 4-KByte page referenced by this entry (see Section 4.9.2)
        const PAT = 1 << 7;
        /// Global; if CR4.PGE = 1, determines whether the translation is global (see Section 4.10); ignored otherwise
        const G = 1 << 8;
        #[doc(hidden)] const _IGN_9 = 1 << 9;
        #[doc(hidden)] const _IGN_10 = 1 << 10;
        /// For ordinary paging, ignored; for HLAT paging, restart (if 1, linear-address translation is restarted with ordinary paging)
        const R = 1 << 11;
        #[doc(hidden)] const _IGN_52 = 1 << 52;
        #[doc(hidden)] const _IGN_53 = 1 << 53;
        #[doc(hidden)] const _IGN_54 = 1 << 54;
        #[doc(hidden)] const _IGN_55 = 1 << 55;
        #[doc(hidden)] const _IGN_56 = 1 << 56;
        #[doc(hidden)] const _IGN_57 = 1 << 57;
        #[doc(hidden)] const _IGN_58 = 1 << 58;
        /// Protection key bit 0; if CR4.PKE = 1 or CR4.PKS = 1, this may control the page’s access rights (see Section 4.6.2); otherwise, it is ignored and not used to control access rights.
        const PK_0 = 1 << 59;
        /// Protection key bit 1; if CR4.PKE = 1 or CR4.PKS = 1, this may control the page’s access rights (see Section 4.6.2); otherwise, it is ignored and not used to control access rights.
        const PK_1 = 1 << 60;
        /// Protection key bit 2; if CR4.PKE = 1 or CR4.PKS = 1, this may control the page’s access rights (see Section 4.6.2); otherwise, it is ignored and not used to control access rights.
        const PK_2 = 1 << 61;
        /// Protection key bit 3; if CR4.PKE = 1 or CR4.PKS = 1, this may control the page’s access rights (see Section 4.6.2); otherwise, it is ignored and not used to control access rights.
        const PK_3 = 1 << 62;
        /// If IA32_EFER.NXE = 1, execute-disable (if 1, instruction fetches are not allowed from the 4-KByte page controlled by this entry; see Section 4.6); otherwise, reserved (must be 0)
        const XD = 1 << 63;
    }
}

bitflags::bitflags! {
    /// Possible memory permissions.
    pub struct Permission: usize {
        /// Page is readable.
        const READ = 1 << 0;
        /// Page is writable.
        const WRITE = 1 << 1;
        /// Page is executable.
        const EXECUTABLE = 1 << 2;
        /// Page can be referred by user application.
        const USER = 1 << 3;
    }
}

// Make page table align to 4096.
#[repr(align(4096))]
struct Inner([Pml4e; 512]);

impl Deref for Inner {
    type Target = [Pml4e; 512];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Inner {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// 4-level page table of x86_64.
pub struct PageTable(Box<Inner>);

impl PageTable {
    /// Create a empty page table.
    pub fn new() -> Self {
        Self(Box::new(Inner([Pml4e(0); 512])))
    }

    /// Get physical address of this page table.
    pub fn pa(&self) -> Pa {
        Va::new(self.0.as_ref().as_ptr() as usize)
            .unwrap()
            .into_pa()
    }

    /// Map `pg` into `va` with permission `perm`.
    pub fn map(&mut self, va: Va, pg: Page, perm: Permission) -> Result<(), PageTableMappingError> {
        unsafe { self.do_map(va, pg.into_raw(), perm) }
    }

    /// Map `pa` into `va` with permission `perm`.
    ///
    /// # Safety
    /// `pa` must be valid.
    pub unsafe fn do_map(
        &mut self,
        va: Va,
        pa: Pa,
        perm: Permission,
    ) -> Result<(), PageTableMappingError> {
        // Hint: Use `Page::new()` to allocate tables.
        todo!()
    }

    /// Unmap the `va` and returns `Page` that was mapped to `va`.
    pub fn unmap(&mut self, va: Va) -> Result<Page, PageTableMappingError> {
        // Hint: Use `Page::from_pa()`.
        todo!()
    }

    /// Walk the page table and return corresponding pte of the `va` if exist.
    pub fn walk(&self, va: Va) -> Result<&Pte, PageTableMappingError> {
        todo!()
    }
}
