mod header;
mod phdr;
mod shdr;

use core::convert::TryInto;
use header::UHeader;

pub use header::{Bit, EMachine, EType, Endian};
pub use phdr::{PType, Phdr, PhdrIterator};
pub use shdr::{SFlags, SType, Shdr, ShdrIterator};

/// Byte peekable object.
pub trait Peeker {
    type Error;
    fn peek_bytes(&self, pos: usize, slice: &mut [u8]) -> Result<(), Self::Error>;
}

union HeaderUnion {
    _raw: [u8; 128],
    header: UHeader,
}

/// Generic ELF representation.
pub struct ELF<T>
where
    T: Peeker,
{
    _u: HeaderUnion,
    peeker: T,
}

const SYSTEM_IS_LE: bool = u32::from_le(1) == 1;

impl<T> ELF<T>
where
    T: Peeker,
{
    /// Create object from bytes.
    pub fn from_peeker(peeker: T) -> Result<ELF<T>, Option<T::Error>> {
        let mut this = ELF {
            _u: HeaderUnion { _raw: [0; 128] },
            peeker,
        };
        unsafe {
            let Self { _u, peeker } = &mut this;
            peeker.peek_bytes(0, &mut _u._raw).map_err(Some)?;

            let uheader: &UHeader = &this._u.header;
            if &uheader.magic != b"\x7FELF" {
                return Err(None);
            }
        }
        // Support only native endian
        if SYSTEM_IS_LE && this.endian() == Endian::Little
            || !SYSTEM_IS_LE && this.endian() == Endian::Big
        {
            Ok(this)
        } else {
            Err(None)
        }
    }

    #[inline]
    pub fn peeker(&self) -> &T {
        &self.peeker
    }

    #[inline]
    pub fn bit(&self) -> Bit {
        unsafe { self._u.header.class }
    }

    #[inline]
    pub fn endian(&self) -> Endian {
        unsafe { self._u.header.data }
    }

    #[inline]
    pub fn type_(&self) -> EType {
        unsafe { self._u.header.e_type }
    }

    #[inline]
    pub fn machine(&self) -> EMachine {
        unsafe { self._u.header.e_machine }
    }

    /// Get entry point of this binary.
    pub fn entry(&self) -> usize {
        unsafe {
            match self.bit() {
                Bit::Bit32 => self._u.header._u._u32.e_entry as usize,
                Bit::Bit64 => self._u.header._u._u64.e_entry.try_into().unwrap(),
            }
        }
    }

    pub fn phentsize(&self) -> usize {
        unsafe {
            match self.bit() {
                Bit::Bit32 => self._u.header._u._u32.e_phentsize as usize,
                Bit::Bit64 => self._u.header._u._u64.e_phentsize.try_into().unwrap(),
            }
        }
    }

    pub fn phnum(&self) -> usize {
        unsafe {
            match self.bit() {
                Bit::Bit32 => self._u.header._u._u32.e_phnum as usize,
                Bit::Bit64 => self._u.header._u._u64.e_phnum.try_into().unwrap(),
            }
        }
    }

    pub fn phoff(&self) -> usize {
        unsafe {
            match self.bit() {
                Bit::Bit32 => self._u.header._u._u32.e_phoff as usize,
                Bit::Bit64 => self._u.header._u._u64.e_phoff.try_into().unwrap(),
            }
        }
    }

    pub fn shstrndx(&self) -> usize {
        unsafe {
            match self.bit() {
                Bit::Bit32 => self._u.header._u._u32.e_shstrndx as usize,
                Bit::Bit64 => self._u.header._u._u64.e_shstrndx as usize,
            }
        }
    }

    /// Get iterator that iterates over program headers in this binary.
    pub fn phdrs(&self) -> PhdrIterator<T> {
        let (base, size) = unsafe {
            match self.bit() {
                Bit::Bit32 => (
                    self._u.header._u._u32.e_phoff.try_into().unwrap(),
                    self._u.header._u._u32.e_phnum,
                ),
                Bit::Bit64 => (
                    self._u.header._u._u64.e_phoff.try_into().unwrap(),
                    self._u.header._u._u64.e_phnum,
                ),
            }
        };
        PhdrIterator {
            base,
            size,
            cursor: 0,
            elf: self,
        }
    }

    /// Get iterator that iterates over section headers in this binary.
    pub fn shdrs(&self) -> ShdrIterator<T> {
        let (base, size) = unsafe {
            match self.bit() {
                Bit::Bit32 => (
                    self._u.header._u._u32.e_shoff.try_into().unwrap(),
                    self._u.header._u._u32.e_shnum,
                ),
                Bit::Bit64 => (
                    self._u.header._u._u64.e_shoff.try_into().unwrap(),
                    self._u.header._u._u64.e_shnum,
                ),
            }
        };
        ShdrIterator {
            base,
            size,
            cursor: 0,
            elf: self,
        }
    }
}
