use super::{Bit, ELF};
use core::convert::TryInto;

/// Program header iterator created by [`ELF::phdrs`] method.
pub struct PhdrIterator<'a, T>
where
    T: super::Peeker,
{
    pub(super) base: usize,
    pub(super) size: u16,
    pub(super) cursor: u16,
    pub(super) elf: &'a ELF<T>,
}

#[repr(u32)]
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PType {
    Null = 0x0,
    Load = 0x1,
    Dynamic = 0x2,
    Interp = 0x3,
    Note = 0x4,
    Shlib = 0x5,
    Phdr = 0x6,
    Tls = 0x7,
    Loos = 0x60000000,
    GnuEhFrame = 0x6474e550,
    GnuStack = 0x6474e551,
    GnuRelro = 0x6474e552,
    Hios = 0x6FFFFFFF,
    Loproc = 0x70000000,
    Hiproc = 0x7FFFFFFF,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// Program header for 32bit ELF.
pub struct Phdr32 {
    type_: PType,
    p_offset: u32,
    p_vaddr: u32,
    p_paddr: u32,
    p_filesz: u32,
    p_memsz: u32,
    p_flags: u32,
    p_align: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// Program header for 64bit ELF.
pub struct Phdr64 {
    type_: PType,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

/// Program Header.
#[derive(Debug)]
pub enum Phdr {
    Phdr32(Phdr32),
    Phdr64(Phdr64),
}

#[allow(dead_code)]
impl Phdr {
    #[inline]
    pub fn type_(&self) -> PType {
        match self {
            Self::Phdr32(p) => p.type_,
            Self::Phdr64(p) => p.type_,
        }
    }

    #[inline]
    pub fn flags(&self) -> u32 {
        match self {
            Self::Phdr32(p) => p.p_flags,
            Self::Phdr64(p) => p.p_flags,
        }
    }

    #[inline]
    pub fn offset(&self) -> usize {
        match self {
            Self::Phdr32(p) => p.p_offset.try_into().unwrap(),
            Self::Phdr64(p) => p.p_offset.try_into().unwrap(),
        }
    }

    #[inline]
    pub fn vaddr(&self) -> usize {
        match self {
            Self::Phdr32(p) => p.p_vaddr.try_into().unwrap(),
            Self::Phdr64(p) => p.p_vaddr.try_into().unwrap(),
        }
    }

    #[inline]
    pub fn paddr(&self) -> usize {
        match self {
            Self::Phdr32(p) => p.p_paddr.try_into().unwrap(),
            Self::Phdr64(p) => p.p_paddr.try_into().unwrap(),
        }
    }

    #[inline]
    pub fn filesz(&self) -> usize {
        match self {
            Self::Phdr32(p) => p.p_filesz.try_into().unwrap(),
            Self::Phdr64(p) => p.p_filesz.try_into().unwrap(),
        }
    }

    #[inline]
    pub fn memsz(&self) -> usize {
        match self {
            Self::Phdr32(p) => p.p_memsz.try_into().unwrap(),
            Self::Phdr64(p) => p.p_memsz.try_into().unwrap(),
        }
    }

    #[inline]
    pub fn align(&self) -> usize {
        match self {
            Self::Phdr32(p) => p.p_align.try_into().unwrap(),
            Self::Phdr64(p) => p.p_align.try_into().unwrap(),
        }
    }
}

union Reader32 {
    phdr: Phdr32,
    _raw: [u8; 0x20],
}

union Reader64 {
    phdr: Phdr64,
    _raw: [u8; 0x38],
}

impl<'a, T> core::iter::Iterator for PhdrIterator<'a, T>
where
    T: super::Peeker,
{
    type Item = Result<Phdr, ()>;
    fn next(&mut self) -> Option<Self::Item> {
        let bit = self.elf.bit();

        let ELF { peeker, .. } = self.elf;

        if self.size > self.cursor {
            unsafe {
                let phdr = match bit {
                    Bit::Bit32 => {
                        let mut inner = Reader32 { _raw: [0; 0x20] };
                        peeker
                            .peek_bytes(self.base + self.cursor as usize * 0x20, &mut inner._raw)
                            .map(|_| Phdr::Phdr32(inner.phdr))
                            .map_err(|_| ())
                    }
                    Bit::Bit64 => {
                        let mut inner = Reader64 { _raw: [0; 0x38] };
                        peeker
                            .peek_bytes(self.base + self.cursor as usize * 0x38, &mut inner._raw)
                            .map(|_| Phdr::Phdr64(inner.phdr))
                            .map_err(|_| ())
                    }
                };
                self.cursor += 1;
                Some(phdr)
            }
        } else {
            None
        }
    }
}
