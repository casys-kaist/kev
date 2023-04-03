use kev::{
    vcpu::{GenericVCpuState, VmexitResult},
    vmcs::Field,
    Probe, VmError,
};
use project2::vmexit::{
    msr::Msr,
    pio::{Direction, PioHandler},
};

#[derive(Default)]
pub struct EferMsr;

impl Msr for EferMsr {
    fn rdmsr(
        &self,
        _index: u32,
        _p: &dyn Probe,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<u64, VmError> {
        generic_vcpu_state.vmcs.read(Field::GuestIa32Efer)
    }

    fn wrmsr(
        &mut self,
        _index: u32,
        value: u64,
        _p: &dyn Probe,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<(), VmError> {
        generic_vcpu_state.vmcs.write(Field::GuestIa32Efer, value)
    }
}

// Address: 0xCF8.
// output: 0xCFC.
pub struct PciPio;
impl PioHandler for PciPio {
    fn handle(
        &self,
        _port: u16,
        direction: Direction,
        p: &dyn Probe,
        GenericVCpuState { vmcs, gprs, .. }: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        match direction {
            // On every out, make it no ops.
            Direction::Outb(_) | Direction::Outd(_) | Direction::Outw(_) => (),
            // On every in, just returns 0xffff.
            Direction::InbAl => {
                gprs.rax = 0xff;
            }
            Direction::InwAx | Direction::IndEax => {
                gprs.rax = 0xffff;
            }
            Direction::Inbm(gva) => unsafe {
                *p.gva2hva(vmcs, gva).unwrap().as_mut::<u8>().unwrap() = 0xff;
            },
            Direction::Inwm(gva) => unsafe {
                *p.gva2hva(vmcs, gva).unwrap().as_mut::<u16>().unwrap() = 0xffff;
            },
            Direction::Indm(gva) => unsafe {
                *p.gva2hva(vmcs, gva).unwrap().as_mut::<u32>().unwrap() = 0xffff;
            },
        };
        Ok(VmexitResult::Ok)
    }
}

pub struct CmosPio;
impl PioHandler for CmosPio {
    fn handle(
        &self,
        _port: u16,
        _direction: Direction,
        _p: &dyn Probe,
        _generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        // ignore.
        Ok(VmexitResult::Ok)
    }
}
