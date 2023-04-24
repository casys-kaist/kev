//! Virtqueue implementation
use super::VirtIoMmioHeader;
use alloc::boxed::Box;
use core::{
    fmt::Debug,
    ptr::{read_volatile, write_volatile},
};
use keos::addressing::{Pa, Va};

/// Command for the virtqueue.
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum VirtQueueEntryCmd {
    /// Read
    Read = 0,
    /// Write
    Write = 1,
}

/// An entry for the virtqueue.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct VirtQueueEntry {
    pub addr: Pa,
    pub size: usize,
    pub sector: usize,
    pub cmd: VirtQueueEntryCmd,
}

/// A container for holding virtqueue.
#[repr(C)]
pub struct VirtQueueContainer {
    size: usize,
    _pad: usize,
    entries: [VirtQueueEntry; 0],
    _pin: core::marker::PhantomPinned,
}

impl core::ops::Index<usize> for VirtQueueContainer {
    type Output = VirtQueueEntry;

    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.size);
        unsafe {
            ((self.entries.as_ptr() as *const _ as usize
                + core::mem::size_of::<VirtQueueEntry>() * index)
                as *const VirtQueueEntry)
                .as_ref()
                .unwrap()
        }
    }
}

impl core::ops::IndexMut<usize> for VirtQueueContainer {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < self.size);
        unsafe {
            ((self.entries.as_mut_ptr() as *mut _ as usize
                + core::mem::size_of::<VirtQueueEntry>() * index)
                as *mut VirtQueueEntry)
                .as_mut()
                .unwrap()
        }
    }
}

/// The virtqueue.
#[repr(C)]
pub struct VirtQueue {
    entries: Box<VirtQueueContainer>,
}

impl VirtQueue {
    /// Create a new virtqueue.
    pub fn new(size: usize) -> Self {
        let mut inner = unsafe {
            Box::from_raw(alloc::alloc::alloc_zeroed(
                alloc::alloc::Layout::from_size_align(
                    core::mem::size_of::<usize>() * 3
                        + core::mem::size_of::<VirtQueueEntry>() * size,
                    16,
                )
                .unwrap(),
            ) as *mut VirtQueueContainer)
        };
        inner.size = size;

        VirtQueue { entries: inner }
    }

    /// Get a virtqueue from Va.
    pub unsafe fn new_from_raw_ptr(queue_va: Va) -> Self {
        let inner = unsafe { Box::from_raw(queue_va.into_usize() as *mut VirtQueueContainer) };

        VirtQueue { entries: inner }
    }

    /// Get a virtual address of the virtqueue.
    pub fn virt_queue_ptr(&self) -> usize {
        self.entries.as_ref() as *const _ as usize
    }

    /// Get a fetcher object of the virtqueue.
    pub fn fetcher<'a>(&'a mut self, mmio: &'a mut VirtIoMmioHeader) -> VirtQueueFetcher {
        let head = unsafe { read_volatile(&mmio.queue_head as *const u32) as usize };
        let tail = unsafe { read_volatile(&mmio.queue_tail as *const u32) as usize };
        VirtQueueFetcher {
            inner: self,
            mmio,
            head,
            tail,
        }
    }
}

/// Fetcher object for the virtqueue.
pub struct VirtQueueFetcher<'a> {
    inner: &'a mut VirtQueue,
    mmio: &'a mut VirtIoMmioHeader,
    head: usize,
    tail: usize,
}

impl<'a> VirtQueueFetcher<'a> {
    fn charge(&self) -> usize {
        (self.head - self.tail) as usize
    }

    fn size(&self) -> usize {
        self.inner.entries.size
    }

    fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    fn is_full(&self) -> bool {
        self.size() == self.charge()
    }

    /// Push a single entry to the virtqueue.
    ///
    /// This does not ring the doorbell.
    pub fn push_front(&mut self, value: VirtQueueEntry) -> Result<(), VirtQueueEntry> {
        if !self.is_full() {
            let size = self.size();
            self.inner.entries[self.head] = value;
            self.head = (self.head + 1) % size;
            Ok(())
        } else {
            Err(value)
        }
    }

    /// Pop a single entry to the virtqueue.
    pub fn pop_back(&mut self) -> Option<VirtQueueEntry> {
        if !self.is_empty() {
            let size = self.size();
            let r = self.inner.entries[self.tail];
            self.tail = (self.tail + 1) % size;
            Some(r)
        } else {
            None
        }
    }

    /// Kick the doorbell to request commands to the VMM.
    pub fn kick(mut self) -> Result<(), ()> {
        // The sequence of the update in this function
        // is really important. Do not change the order.
        unsafe {
            if read_volatile(&mut self.mmio.queue_tail) != self.tail as u32 {
                write_volatile(&mut self.mmio.queue_tail, self.tail as u32);
            } else if read_volatile(&self.mmio.queue_head) != self.head as u32 {
                write_volatile(&mut self.mmio.queue_head, self.head as u32);
                self.tail = read_volatile(&self.mmio.queue_tail) as usize;
            }

            // This check is required to verify the change we made into mmio area.
            if read_volatile(&self.mmio.queue_head) == self.head as u32
                && read_volatile(&self.mmio.queue_tail) == self.tail as u32
            {
                if read_volatile(&self.mmio.status) != super::VirtIoStatus::READY as u32 {
                    return Err(());
                }
                Ok(())
            } else {
                Err(())
            }
        }
    }
}

impl<'a> Debug for VirtQueueFetcher<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VirtQueue")
            .field("size", &self.inner.entries.size)
            .field("head", &self.head)
            .field("tail", &self.tail)
            .finish()
    }
}

impl<'a> Iterator for VirtQueueFetcher<'a> {
    type Item = VirtQueueEntry;

    fn next(&mut self) -> Option<Self::Item> {
        self.pop_back()
    }
}

impl<'a> Drop for VirtQueueFetcher<'a> {
    fn drop(&mut self) {
        let mmio_head = unsafe { read_volatile(&self.mmio.queue_head) as usize };
        let mmio_tail = unsafe { read_volatile(&self.mmio.queue_tail) as usize };

        if mmio_head != self.head || mmio_tail != self.tail {
            println!(
                "mmio queue head: {} mmio queue tail: {}",
                mmio_head, mmio_tail
            );
            println!(
                "fetcher queue head: {} fetcher queue tail: {}",
                self.head, self.tail
            );
            panic!(
                "Possible corruption of virtio index (head and tail).
                    It's recommended to check if 'VirtQueueFetcher::kick' was
                    called after a 'push_queue' or 'pop_queue' operation."
            );
        }
    }
}
