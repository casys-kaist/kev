use simple_fs::*;
use std::fs::OpenOptions;
use std::io::Read;
use std::os::unix::fs::FileExt;
use std::path::Path;

fn for_each_file_in_dir(dir: &std::path::Path, f: &mut impl FnMut(&std::path::Path)) {
    for en in dir.read_dir().unwrap().flatten() {
        if en.path().is_dir() {
            panic!("directory is not supported.")
        } else {
            f(&en.path());
        }
    }
}

pub fn build_fs() {
    // Build disk.
    const M: u64 = 1024 * 1024;
    let disk = "blk.bin";
    let _ = std::fs::remove_file(disk);
    // calculate requirede disk size.
    let mut size = 0;
    let mut files = Vec::new();

    for_each_file_in_dir(Path::new("./rootfs"), &mut |d| {
        let meta = d.metadata().expect("Only a regular file is supported.");
        let entry = d.to_str().unwrap().to_string();
        size += meta.len();
        files.push(entry);
    });
    let disk_size = ((size + M - 1) / M + 1024) * M;
    impl Disk for FileDisk {
        fn read(&self, sector: Sector, buf: &mut [u8; 512]) -> Result<(), Error> {
            self.file
                .read_at(buf.as_mut(), sector.into_offset() as u64)
                .map_err(|_| Error::DiskError)
                .map(|_| ())
        }
        fn write(&self, sector: Sector, buf: &[u8; 512]) -> Result<(), Error> {
            self.file
                .write_at(buf.as_ref(), sector.into_offset() as u64)
                .map_err(|_| Error::DiskError)
                .map(|_| ())
        }
    }

    struct FileDisk {
        file: std::fs::File,
    }

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(disk)
        .expect("Failed to create file.");
    file.set_len(disk_size).unwrap();

    let mut fs = FileSystem::new(FileDisk { file }, disk_size as usize).unwrap();
    for f in files.iter() {
        let dst = Path::new(&f[9..]);
        let mut buf = Vec::new();
        OpenOptions::new()
            .read(true)
            .open(f)
            .expect("Failed to open file")
            .read_to_end(&mut buf)
            .expect("Failed to read file contents");
        fs.create(dst.to_str().unwrap(), &buf)
            .expect("Failed to create file");
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=blk.bin");
    println!("cargo:rerun-if-changed=rootfs");
}
