#[path = "../../../projects/project4/src/virtio/mod.rs"]
mod virtio;

use alloc::boxed::Box;
use alloc::sync::Arc;
use core::ptr::{read_volatile, write_volatile};
use keos::{
    addressing::{Pa, Va},
    fs::{Disk, Error},
    sync::SpinLock,
};
use virtio::{
    virt_queue::{VirtQueue, VirtQueueEntry, VirtQueueEntryCmd, VirtQueueFetcher},
    VirtIoMmioHeader, VirtIoStatus,
};

pub struct VirtIoBlockDriver {
    header: *mut VirtIoMmioHeader,
    virt_queue: VirtQueue<Box<[VirtQueueEntry]>>,
}

pub struct VirtIoDisk {
    inner: Arc<SpinLock<VirtIoBlockDriver>>,
}

impl VirtIoBlockDriver {
    pub fn realize(mmio_addr: Pa) -> Option<VirtIoBlockDriver> {
        let header = unsafe { &mut *(mmio_addr.into_va().into_usize() as *mut VirtIoMmioHeader) };
        info!("VirtIo Block Driver Start.");

        let virt_queue = unsafe {
            let virt_queue = if header.status == VirtIoStatus::MAGIC as u32 {
                VirtQueue::new(0x1000)
            } else {
                write_volatile(&mut header.status, VirtIoStatus::RESET as u32);
                return None;
            };
            write_volatile(&mut header.status, VirtIoStatus::DRIVEROK as u32);
            info!("VirtIo Block Driver Init start...");
            if read_volatile(&header.status) != VirtIoStatus::DRIVEROK as u32 {
                write_volatile(&mut header.status, VirtIoStatus::RESET as u32);
                return None;
            }

            let queue_ptr = Va::new(virt_queue.virt_queue_ptr())
                .unwrap()
                .into_pa()
                .into_usize();
            write_volatile(&mut header.queue_addr_hi, (queue_ptr >> 32) as u32);
            write_volatile(&mut header.queue_addr_lo, (queue_ptr & 0xFFFF_FFFF) as u32);
            write_volatile(&mut header.queue_size, 0x1000);

            info!("VirtIo Block Driver Enabled...");
            write_volatile(&mut header.status, VirtIoStatus::READY as u32);

            // Now check driver is ready to use
            if read_volatile(&header.status) != VirtIoStatus::READY as u32 {
                write_volatile(&mut header.status, VirtIoStatus::RESET as u32);
                return None;
            }
            virt_queue
        };

        info!("VirtIo Block Driver Ready.");
        Some(Self {
            header: header as *mut VirtIoMmioHeader,
            virt_queue,
        })
    }

    pub fn finish(&mut self) {
        unsafe {
            let header = &mut *self.header;
            info!("VirtIo Block Driver Finish.");
            write_volatile(&mut header.status as *mut u32, VirtIoStatus::RESET as u32);
        }
    }

    pub fn send_cmd(
        fetcher: &mut VirtQueueFetcher<Box<[VirtQueueEntry]>>,
        buf: &[u8],
        sector: usize,
        cmd: VirtQueueEntryCmd,
    ) -> Result<(), Error> {
        let entry = VirtQueueEntry {
            addr: Va::new(buf.as_ptr() as usize).unwrap().into_pa(),
            size: buf.len(),
            sector,
            cmd,
        };

        fetcher.push_front(entry).map_err(|_| Error::DiskError)
    }

    pub fn kick(fetcher: VirtQueueFetcher<Box<[VirtQueueEntry]>>) -> Result<(), Error> {
        fetcher.kick().map_err(|_| Error::DiskError)
    }
}

impl VirtIoDisk {
    pub fn new() -> Option<Self> {
        VirtIoBlockDriver::realize(Pa::new(0xcafe0000).unwrap()).and_then(|driver| {
            Some(Self {
                inner: Arc::new(SpinLock::new(driver)),
            })
        })
    }

    pub fn finish(&mut self) {
        self.inner.lock().finish();
    }

    pub fn read_many(&self, start_sector: keos::fs::Sector, buf: &mut [u8]) -> Result<(), Error> {
        assert_eq!(buf.len() % 512, 0);
        let mut guard = self.inner.lock();
        let mmio = unsafe { &mut *guard.header };
        let mut fetcher = guard.virt_queue.fetcher(mmio);
        for (idx, off) in (0..buf.len()).step_by(512).enumerate() {
            VirtIoBlockDriver::send_cmd(
                &mut fetcher,
                &buf[off..off + 512],
                start_sector.into_usize() + idx,
                VirtQueueEntryCmd::Read,
            )?;
        }
        VirtIoBlockDriver::kick(fetcher)
    }
    pub fn write_many(&self, start_sector: keos::fs::Sector, buf: &[u8]) -> Result<(), Error> {
        assert_eq!(buf.len() % 512, 0);
        let mut guard = self.inner.lock();
        let mmio = unsafe { &mut *guard.header };
        let mut fetcher = guard.virt_queue.fetcher(mmio);
        for (idx, off) in (0..buf.len()).step_by(512).enumerate() {
            VirtIoBlockDriver::send_cmd(
                &mut fetcher,
                &buf[off..off + 512],
                start_sector.into_usize() + idx,
                VirtQueueEntryCmd::Write,
            )?;
        }
        VirtIoBlockDriver::kick(fetcher)
    }
}

impl Disk for VirtIoDisk {
    fn read(&self, sector: keos::fs::Sector, buf: &mut [u8; 512]) -> Result<(), Error> {
        let mut guard = self.inner.lock();
        let mmio = unsafe { &mut *guard.header };
        let mut fetcher = guard.virt_queue.fetcher(mmio);
        VirtIoBlockDriver::send_cmd(
            &mut fetcher,
            buf,
            sector.into_usize(),
            VirtQueueEntryCmd::Read,
        )?;
        VirtIoBlockDriver::kick(fetcher)
    }

    fn write(&self, sector: keos::fs::Sector, buf: &[u8; 512]) -> Result<(), Error> {
        let mut guard = self.inner.lock();
        let mmio = unsafe { &mut *guard.header };
        let mut fetcher = guard.virt_queue.fetcher(mmio);
        VirtIoBlockDriver::send_cmd(
            &mut fetcher,
            buf,
            sector.into_usize(),
            VirtQueueEntryCmd::Write,
        )?;
        VirtIoBlockDriver::kick(fetcher)
    }
}
