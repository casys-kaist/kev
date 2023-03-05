//! Memory abstraction.
//!
//! KEOS maps kernel virtual memory directly to physical memory.
//! The first page of kernel virtual memory is mapped to the first frame of physical memory,
//! the second page to the second frame, and so on. Thus, physical address and kernel virtual address can be
//! calculated by simpling adding or substracting constant offset.

const VA_TO_PA_OFF: usize = 0xffff000000000000 | (510 << 39);

/// Page size.
pub const PAGE_SIZE: usize = 0x1000;
/// Shift amount to get page index.
pub const PAGE_SHIFT: usize = 12;
/// Mask for page offset.
pub const PAGE_MASK: usize = 0xfff;

/// Physical address
#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct Pa(usize);

impl Pa {
    /// PA with address 0.
    pub const ZERO: Self = Self(0);

    /// Create a new physical address with a check.
    #[inline]
    pub const fn new(addr: usize) -> Option<Self> {
        if addr < 0xffff_0000_0000_0000 {
            Some(Self(addr))
        } else {
            None
        }
    }

    /// Cast into usize.
    #[inline]
    pub const unsafe fn into_usize(self) -> usize {
        self.0
    }

    /// Cast into virtual address.
    #[inline]
    pub const fn into_va(self) -> Va {
        Va(self.0 + VA_TO_PA_OFF)
    }
}

/// Virtual address
#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct Va(usize);

impl Va {
    /// Create a new virtual address with a check.
    #[inline(always)]
    pub const fn new(addr: usize) -> Option<Self> {
        match addr & 0xffff_8000_0000_0000 {
            m if m == 0xffff_8000_0000_0000 || m == 0 => Some(Self(addr)),
            _ => None,
        }
    }

    /// Cast into usize.
    #[inline]
    pub const unsafe fn into_usize(self) -> usize {
        self.0
    }

    /// Cast into physical address.
    #[inline]
    pub const fn into_pa(self) -> Pa {
        Pa(self.0 - VA_TO_PA_OFF)
    }

    /// Get reference of T underlying the Va.
    ///
    /// # Safety
    ///
    /// When calling this method, you have to ensure that *either* the pointer is null *or*
    /// all of the following is true:
    ///
    /// * The pointer must be properly aligned.
    ///
    /// * It must be "dereferenceable" in the sense defined in [the module documentation].
    ///
    /// * The pointer must point to an initialized instance of `T`.
    ///
    /// This applies even if the result of this method is unused!
    ///
    /// [the module documentation]: core::ptr#safety
    #[inline]
    pub unsafe fn as_ref<'a, T>(&self) -> Option<&'a T> {
        (self.into_usize() as *const T).as_ref()
    }

    /// Get mutable reference of T underlying the Va.
    ///
    /// # Safety
    ///
    /// When calling this method, you have to ensure that *either* the pointer is null *or*
    /// all of the following is true:
    ///
    /// * The pointer must be properly aligned.
    ///
    /// * It must be "dereferenceable" in the sense defined in [the module documentation].
    ///
    /// * The pointer must point to an initialized instance of `T`.
    ///
    /// This applies even if the result of this method is unused!
    ///
    /// [the module documentation]: core::ptr#safety
    #[inline]
    pub unsafe fn as_mut<'a, T>(&self) -> Option<&'a mut T> {
        (self.into_usize() as *mut T).as_mut()
    }
}

macro_rules! impl_arith {
    ($t: ty) => {
        impl core::ops::Add<usize> for $t {
            type Output = Self;

            fn add(self, other: usize) -> Self::Output {
                Self(self.0 + other)
            }
        }
        impl core::ops::AddAssign<usize> for $t {
            fn add_assign(&mut self, other: usize) {
                self.0 = self.0 + other
            }
        }
        impl core::ops::Sub<usize> for $t {
            type Output = Self;

            fn sub(self, other: usize) -> Self::Output {
                Self(self.0 - other)
            }
        }
        impl core::ops::SubAssign<usize> for $t {
            fn sub_assign(&mut self, other: usize) {
                self.0 = self.0 - other
            }
        }
        impl core::ops::BitOr<usize> for $t {
            type Output = Self;

            fn bitor(self, other: usize) -> Self {
                Self(self.0 | other)
            }
        }
        impl core::ops::BitOrAssign<usize> for $t {
            fn bitor_assign(&mut self, other: usize) {
                self.0 = self.0 | other;
            }
        }
        impl core::ops::BitAnd<usize> for $t {
            type Output = Self;

            fn bitand(self, other: usize) -> Self {
                Self(self.0 & other)
            }
        }
        impl core::ops::BitAndAssign<usize> for $t {
            fn bitand_assign(&mut self, other: usize) {
                self.0 = self.0 & other;
            }
        }
    };
}

impl_arith!(Va);
impl_arith!(Pa);

impl core::fmt::Debug for Va {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Va(0x{:x})", self.0)
    }
}
impl core::fmt::Display for Va {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Va(0x{:x})", self.0)
    }
}

impl core::fmt::Debug for Pa {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Pa(0x{:x})", self.0)
    }
}
impl core::fmt::Display for Pa {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Pa(0x{:x})", self.0)
    }
}
