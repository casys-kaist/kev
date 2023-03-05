//! Filesystem implementation.
//!
//! This filesystem only supported fixed-size file. (No directory!)
pub use simple_fs::*;

/// The filesystem disk.
pub struct FsDisk {
    _p: (),
}

impl Disk for FsDisk {
    fn read(&self, sector: Sector, buf: &mut [u8; 512]) -> Result<(), Error> {
        let dev = abyss::dev::get_bdev(1).ok_or(Error::DiskError)?;
        dev.read_bios(&mut Some((512 * sector.into_usize(), buf.as_mut())).into_iter())
            .map_err(|_| Error::DiskError)
    }
    fn write(&self, sector: Sector, buf: &[u8; 512]) -> Result<(), Error> {
        let dev = abyss::dev::get_bdev(1).ok_or(Error::DiskError)?;
        dev.write_bios(&mut Some((512 * sector.into_usize(), buf.as_ref())).into_iter())
            .map_err(|_| Error::DiskError)
    }
}

static mut FS: Option<FileSystem<FsDisk>> = None;

/// Initialize the fs.
pub unsafe fn init_fs() {
    if let Ok(fs) = FileSystem::load(FsDisk { _p: () }) {
        FS = Some(fs);
    } else {
        warning!("Failed to open fs.");
    }
}

/// Get a filesystem reference of the kernel.
pub fn file_system() -> Option<&'static FileSystem<FsDisk>> {
    unsafe { FS.as_ref() }
}

/// The file.
pub type File = simple_fs::File<'static, FsDisk>;
