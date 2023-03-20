//! Model-specific register vmexit controller.
//!
//! x86 processors include a collection of specialized registers that permit the customization of CPU capabilities.
//! Each register possesses a 64-bit space, and accessing these registers necessitates using RDMSR (read) or WRMSR (write) instructions.
//!
//! The RDMSR instruction reads a register that is determined by the index value provided in the ECX register.
//! The retrieved MSR value is stored in the EDX and EAX registers, with the high-order 32 bits of the MSR value placed in the EDX register,
//! and the low-order 32 bits placed in the EAX register. In 64-bit architectures, the high-order 32 bits of both the RDX and RAX registers are set to 0.
//!
//! The WRMSR instruction writes a value to a register specified in the ECX register as an index.
//! The value written to the MSR is obtained from the EDX and EAX registers.
//! Specifically, the value in EDX is written to MSR\[63:32\] (high-order 32bits), and the value in EAX is written to MSR\[31:0\] (low-order 32bits).
//!
//! When managing virtual machines, the default behavior is for the host to intercept RDMSR/MRMSR instructions from the guest.
//! Therefore, the host needs to emulate these MSR access requests.
//!
//! ## Tasks
//! Project2 requires emulating msr requests. The first step involves writing code to register an msr handler to the msr controller for a particular msr index.
//! The second step requires emulating RDMSR/WRMSR using the arguments provided by the guest when msr access requests arrive.
//! Again, you **MUST** forward the vCPU instruction pointer (rip) to prevent it from executing the same instructions indefinitely.
use alloc::{
    boxed::Box,
    collections::{btree_map::Entry, BTreeMap},
};
use kev::{
    vcpu::{GenericVCpuState, VmexitResult},
    vmcs::{BasicExitReason, ExitReason},
    Probe, VmError,
};

/// Trait that represent handlers for MSR registers.
pub trait Msr
where
    Self: Send + Sync,
{
    /// Handler on wrmsr.
    fn rdmsr(
        &self,
        index: u32,
        p: &dyn Probe,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<u64, VmError>;
    /// Handler on wrmsr.
    fn wrmsr(
        &mut self,
        index: u32,
        value: u64,
        p: &dyn Probe,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<(), VmError>;
}

/// Msr vmexit controller.
pub struct Controller {
    msrs: BTreeMap<u32, Box<dyn Msr>>,
}

impl Controller {
    /// Create a new msr controller.
    pub fn new() -> Self {
        Self {
            msrs: BTreeMap::new(),
        }
    }

    /// Insert msr handler to the index.
    ///
    /// Return false if msr handler for index is exists.
    /// Otherwise, return true.
    pub fn insert(&mut self, index: u32, msr: impl Msr + 'static) -> bool {
        todo!()
    }
}

impl kev::vmexits::VmexitController for Controller {
    fn handle<P: kev::Probe>(
        &mut self,
        reason: ExitReason,
        p: &mut P,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, kev::VmError> {
        match reason.get_basic_reason() {
            BasicExitReason::Rdmsr => {
                todo!()
            }
            BasicExitReason::Wrmsr => {
                todo!()
            }
            _ => Err(kev::VmError::HandleVmexitFailed(reason)),
        }
    }
}
