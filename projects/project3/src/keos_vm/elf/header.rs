#[derive(PartialEq, Eq, Debug, Copy, Clone)]
#[repr(u16)]
pub enum EType {
    None = 0,
    Rel = 1,
    Exec = 2,
    Dyn = 3,
    Core = 4,
    Loos = 0xfe00,
    Hios = 0xfeff,
    Loproc = 0xff00,
    Hiproc = 0xffff,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
#[repr(u16)]
pub enum EMachine {
    None = 0x0,
    ATnTWe = 0x1,
    Sparc = 0x2,
    X86 = 0x3,
    M68k = 0x4,
    M88k = 0x5,
    IntelMcu = 0x6,
    Intel8086 = 0x7,
    Mips = 0x8,
    Ibm370 = 0x9,
    Intel80960 = 0x13,
    PowerPC32 = 0x14,
    PowerPC64 = 0x15,
    S390 = 0x16,
    ARM = 0x28,
    SuperH = 0x2A,
    IA64 = 0x32,
    Amd64 = 0x3E,
    Tms320c6000 = 0x8C,
    Aarch64 = 0xB7,
    RiscV = 0xF3,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
#[repr(u8)]
pub enum Bit {
    Bit32 = 1,
    Bit64 = 2,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
#[repr(u8)]
pub enum Endian {
    Little = 1,
    Big = 2,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ELF64Union {
    /// Memory address of the entry point from where the process starts
    /// executing.
    pub e_entry: u64,
    /// Start of the program header table.
    pub e_phoff: u64,
    /// Start of the section header table.
    pub e_shoff: u64,
    /// Target specific flag.
    pub e_flags: u32,
    /// Size of this header.
    pub e_ehsize: u16,
    /// Size of program header table entry.
    pub e_phentsize: u16,
    /// Number of entries in program header table.
    pub e_phnum: u16,
    /// Size of section header table entry.
    pub e_shentsize: u16,
    /// Number of entries in section header table.
    pub e_shnum: u16,
    /// Index of the section header table entry that contains the section names.
    pub e_shstrndx: u16,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ELF32Union {
    /// Memory address of the entry point from where the process starts
    /// executing.
    pub e_entry: u32,
    /// Start of the program header table.
    pub e_phoff: u32,
    /// Start of the section header table.
    pub e_shoff: u32,
    /// Target specific flag.
    pub e_flags: u32,
    /// Size of this header.
    pub e_ehsize: u16,
    /// Size of program header table entry.
    pub e_phentsize: u16,
    /// Number of entries in program header table.
    pub e_phnum: u16,
    /// Size of section header table entry.
    pub e_shentsize: u16,
    /// Number of entries in section header table.
    pub e_shnum: u16,
    /// Index of the section header table entry that contains the section names.
    pub e_shstrndx: u16,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub(crate) union ELFUnion {
    pub(crate) _u64: ELF64Union,
    pub(crate) _u32: ELF32Union,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub(crate) struct UHeader {
    /// 0x7F followed by ELF(45 4c 46) in ASCII; these four bytes constitute
    /// the magic number.
    pub magic: [u8; 4],
    /// Signify 32- or 64-bit format.
    pub class: Bit,
    /// Signify little or big endianness.
    pub data: Endian,
    /// Set to 1 for the original and current version of ELF.
    pub _version: u8,
    /// Identifies the target operating system ABI.
    pub _abi: u8,
    /// Further specifies the ABI version.
    pub _abi_version: u8,
    /// Unused.
    pub _pad: [u8; 7],
    /// Identifies object file type.
    pub e_type: EType,
    /// Specifies target instruction set architecture.
    pub e_machine: EMachine,
    /// Set to 1 for the original version of ELF.
    pub e_version: u32,
    pub _u: ELFUnion,
}
