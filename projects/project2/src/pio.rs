//! Pio handlers to test pio instructions correctly implemented.
use crate::vmexit::pio::{Direction, PioHandler};
use alloc::{boxed::Box, collections::LinkedList};
use core::fmt::Write;
use keos::spin_lock::SpinLock;
use kev::{
    vcpu::{GenericVCpuState, VmexitResult},
    Probe, VmError,
};

/// emulation of the device that prints the port & direction.
pub struct PioHandlerDummy;
impl PioHandler for PioHandlerDummy {
    fn handle(
        &self,
        port: u16,
        direction: Direction,
        _p: &dyn Probe,
        _generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        let _ = writeln!(
            &mut crate::PrinterProxy,
            "port {} direction {:?}",
            port,
            direction
        );
        Ok(VmexitResult::Ok)
    }
}

/// emulation of the device that print the character of the operand in Out instruction.
pub struct PioHandlerPrint;
impl PioHandler for PioHandlerPrint {
    fn handle(
        &self,
        _port: u16,
        direction: Direction,
        _p: &dyn Probe,
        _generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        let char = match direction {
            Direction::Outb(byte) => byte,
            _ => unreachable!(),
        };
        let b = core::char::from_u32(char as u32).unwrap();
        let _ = write!(&mut crate::PrinterProxy, "{}", b);
        Ok(VmexitResult::Ok)
    }
}

/// emulation of device that tests all In/Out/Ins/Outs instruction family with three queues.
pub struct PioHandlerQueue {
    byte_queue: SpinLock<LinkedList<u8>>,
    word_queue: SpinLock<LinkedList<u16>>,
    dword_queue: SpinLock<LinkedList<u32>>,
}
impl PioHandlerQueue {
    /// Create a new PioHandlerQueue.
    pub fn new() -> Self {
        Self {
            byte_queue: SpinLock::new(LinkedList::new()),
            word_queue: SpinLock::new(LinkedList::new()),
            dword_queue: SpinLock::new(LinkedList::new()),
        }
    }
}
impl PioHandler for PioHandlerQueue {
    fn handle(
        &self,
        _port: u16,
        direction: Direction,
        p: &dyn Probe,
        GenericVCpuState { vmcs, gprs, .. }: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        match direction {
            Direction::InbAl => {
                if let Some(byte) = self.byte_queue.lock().pop_front() {
                    gprs.rax = byte as usize;
                } else {
                    return Err(VmError::ControllerError(Box::new("Empty byte queue")));
                }
            }
            Direction::InwAx => {
                if let Some(word) = self.word_queue.lock().pop_front() {
                    gprs.rax = word as usize;
                } else {
                    return Err(VmError::ControllerError(Box::new("Empty word queue")));
                }
            }
            Direction::IndEax => {
                if let Some(dword) = self.dword_queue.lock().pop_front() {
                    gprs.rax = dword as usize;
                } else {
                    return Err(VmError::ControllerError(Box::new("Empty dword queue")));
                }
            }
            Direction::Inbm(gva) => {
                if let Some(byte) = self.byte_queue.lock().pop_front() {
                    unsafe {
                        *(p.gva2hva(vmcs, gva).unwrap().into_usize() as *mut u8) = byte;
                    }
                } else {
                    return Err(VmError::ControllerError(Box::new("Empty byte queue")));
                }
            }
            Direction::Inwm(gva) => {
                if let Some(word) = self.word_queue.lock().pop_front() {
                    unsafe {
                        *(p.gva2hva(vmcs, gva).unwrap().into_usize() as *mut u16) = word;
                    }
                } else {
                    return Err(VmError::ControllerError(Box::new("Empty word queue")));
                }
            }
            Direction::Indm(gva) => {
                if let Some(dword) = self.dword_queue.lock().pop_front() {
                    unsafe {
                        *(p.gva2hva(vmcs, gva).unwrap().into_usize() as *mut u32) = dword;
                    }
                } else {
                    return Err(VmError::ControllerError(Box::new("Empty dword queue")));
                }
            }
            Direction::Outb(byte) => {
                self.byte_queue.lock().push_back(byte);
            }
            Direction::Outw(word) => {
                self.word_queue.lock().push_back(word);
            }
            Direction::Outd(dword) => {
                self.dword_queue.lock().push_back(dword);
            }
        }
        Ok(VmexitResult::Ok)
    }
}
