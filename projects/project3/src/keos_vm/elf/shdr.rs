use super::{Bit, ELF};
use bitflags::bitflags;
use core::convert::TryInto;

/// Section header iterator created by [`ELF::shdrs`] method.
pub struct ShdrIterator<'a, T>
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
pub enum SType {
    /// Unused.
    Null = 0x0,
    /// Program data.
    ProgBits = 0x1,
    /// Symbol table.
    Symtab = 0x2,
    /// String table.
    Strtab = 0x3,
    /// Relocation entries with addends.
    Rela = 0x4,
    /// Symbol hash table.
    Hash = 0x5,
    /// Dynamic linking information.
    Dynamic = 0x6,
    /// Notes.
    Note = 0x7,
    /// Program space with no data (bss).
    Nobits = 0x8,
    /// Relocation entries, no addends.
    Rel = 0x9,
    /// Reserved.
    Shlib = 0xa,
    /// Dynamic linker symbol table.
    Dynsym = 0xb,
    /// Array of constructors.
    InitArray = 0xc,
    /// Array of destructors.
    FiniArray = 0xf,
    /// Array of pre-constructors
    PreinitArray = 0x10,
    /// Section group.
    Group = 0x11,
    /// Extended section indices.
    SymtabShndx = 0x12,
    /// Number of defined types.
    Num = 0x13,
    /// Start OS-specific.
    Loos = 0x60000000,
}

bitflags! {
    #[allow(dead_code)]
    pub struct SFlags: u32 {
    /// Writable.
    const WRITE = 0x1;
    /// Occupies memory during execution.
    const ALLOC = 0x2;
    /// Executable.
    const EXECINSTR = 0x4;
    /// Might be merged.
    const MERGE = 0x10;
    /// Contains null-terminated strings.
    const STRINGS = 0x20;
    /// 'sh_info' contains SHT index
    const INFOLINK = 0x40;
    /// Preserve order after combining.
    const LINKORDER = 0x80;
    /// Non-standard OS specific handling required.
    const OSNONCONFORMING = 0x100;
    /// Section is member of a group.
    const GROUP = 0x200;
    /// Section hold thread-local data.
    const TLS = 0x400;
    /// OS-specific.
    const MASKOS = 0x0ff00000;
    /// Processor-specific.
    const MASKPROC = 0xf0000000;
    /// Special ordering requirement (Solaris).
    const ORDERED = 0x4000000;
    /// Section is excluded unless referenced or allocated (Solaris).
    const EXCLUDE = 0x8000000;
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// Section header for 32bit ELF.
pub struct Shdr32 {
    /// An offset to a string in the .shstrtab section that represents the name
    /// of this section.
    name: u32,
    /// Identifies the type of this header.
    sh_type: SType,
    /// Identifies the attributes of the section.
    sh_flags: SFlags,
    /// Virtual address of the section in memory, for sections that are loaded.
    sh_addr: u32,
    /// Offset of the section in the file image.
    sh_offset: u32,
    /// Size in bytes of the section in the file image. May be 0.
    sh_size: u32,
    /// Contains the section index of an associated section. This field is used
    /// for several purposes, depending on the type of section.
    sh_link: u32,
    /// Contains extra information about the section. This field is used for
    /// several purposes, depending on the type of section.
    sh_info: u32,
    /// Contains the required alignment of the section. This field must be a
    /// power of two.
    sh_addralign: u32,
    /// Contains the size, in bytes, of each entry, for sections that contain
    /// fixed-size entries. Otherwise, this field contains zero.
    sh_ent_size: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// Section header for 64bit ELF.
pub struct Shdr64 {
    /// An offset to a string in the .shstrtab section that represents the name
    /// of this section.
    name: u32,
    /// Identifies the type of this header.
    sh_type: SType,
    /// Identifies the attributes of the section.
    sh_flags: SFlags,
    /// Virtual address of the section in memory, for sections that are loaded.
    sh_addr: u64,
    /// Offset of the section in the file image.
    sh_offset: u64,
    /// Size in bytes of the section in the file image. May be 0.
    sh_size: u64,
    /// Contains the section index of an associated section. This field is used
    /// for several purposes, depending on the type of section.
    sh_link: u32,
    /// Contains extra information about the section. This field is used for
    /// several purposes, depending on the type of section.
    sh_info: u32,
    /// Contains the required alignment of the section. This field must be a
    /// power of two.
    sh_addralign: u64,
    /// Contains the size, in bytes, of each entry, for sections that contain
    /// fixed-size entries. Otherwise, this field contains zero.
    sh_ent_size: u64,
}

/// Section Header.
pub enum Shdr {
    Shdr32(Shdr32),
    Shdr64(Shdr64),
}

#[allow(dead_code)]
impl Shdr {
    #[inline]
    fn name(&self) -> u32 {
        match self {
            Shdr::Shdr32(s) => s.name,
            Shdr::Shdr64(s) => s.name,
        }
    }

    #[inline]
    fn type_(&self) -> SType {
        match self {
            Shdr::Shdr32(s) => s.sh_type,
            Shdr::Shdr64(s) => s.sh_type,
        }
    }

    #[inline]
    fn flags(&self) -> SFlags {
        match self {
            Shdr::Shdr32(s) => s.sh_flags,
            Shdr::Shdr64(s) => s.sh_flags,
        }
    }

    #[inline]
    fn addr(&self) -> usize {
        match self {
            Shdr::Shdr32(s) => s.sh_addr as usize,
            Shdr::Shdr64(s) => s.sh_addr.try_into().unwrap(),
        }
    }

    #[inline]
    fn offset(&self) -> usize {
        match self {
            Shdr::Shdr32(s) => s.sh_offset as usize,
            Shdr::Shdr64(s) => s.sh_offset.try_into().unwrap(),
        }
    }

    #[inline]
    fn size(&self) -> usize {
        match self {
            Shdr::Shdr32(s) => s.sh_size as usize,
            Shdr::Shdr64(s) => s.sh_size.try_into().unwrap(),
        }
    }

    #[inline]
    fn link(&self) -> u32 {
        match self {
            Shdr::Shdr32(s) => s.sh_link,
            Shdr::Shdr64(s) => s.sh_link,
        }
    }

    #[inline]
    fn info(&self) -> u32 {
        match self {
            Shdr::Shdr32(s) => s.sh_info,
            Shdr::Shdr64(s) => s.sh_info,
        }
    }

    #[inline]
    fn addralign(&self) -> usize {
        match self {
            Shdr::Shdr32(s) => s.sh_addralign as usize,
            Shdr::Shdr64(s) => s.sh_addralign.try_into().unwrap(),
        }
    }

    #[inline]
    fn ent_size(&self) -> usize {
        match self {
            Shdr::Shdr32(s) => s.sh_ent_size as usize,
            Shdr::Shdr64(s) => s.sh_ent_size.try_into().unwrap(),
        }
    }
}

union Reader32 {
    shdr: Shdr32,
    _raw: [u8; 0x20],
}

union Reader64 {
    shdr: Shdr64,
    _raw: [u8; 0x40],
}

impl<'a, T> core::iter::Iterator for ShdrIterator<'a, T>
where
    T: super::Peeker,
{
    type Item = Result<Shdr, ()>;
    fn next(&mut self) -> Option<Self::Item> {
        let bit = self.elf.bit();

        let ELF { peeker, .. } = self.elf;

        if self.size > self.cursor {
            unsafe {
                let shdr = match bit {
                    Bit::Bit32 => {
                        let mut inner = Reader32 { _raw: [0; 0x20] };
                        peeker
                            .peek_bytes(self.base + self.cursor as usize * 0x20, &mut inner._raw)
                            .map(|_| Shdr::Shdr32(inner.shdr))
                            .map_err(|_| ())
                    }
                    Bit::Bit64 => {
                        let mut inner = Reader64 { _raw: [0; 0x40] };
                        peeker
                            .peek_bytes(self.base + self.cursor as usize * 0x40, &mut inner._raw)
                            .map(|_| Shdr::Shdr64(inner.shdr))
                            .map_err(|_| ())
                    }
                };
                self.cursor += 1;
                Some(shdr)
            }
        } else {
            None
        }
    }
}
