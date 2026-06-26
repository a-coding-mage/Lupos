//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/mce/inject.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/mce/inject.c
//! Machine check injection model.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/cpu/mce/inject.c

use super::core::{
    MCG_BANKCNT_MASK, MCG_STATUS_EIPV, MCG_STATUS_MCIP, MCG_STATUS_RIPV, MCI_STATUS_ADDRV,
    MCI_STATUS_DEFERRED, MCI_STATUS_MISCV, MCI_STATUS_PCC, MCI_STATUS_SYNDV, MCI_STATUS_UC,
    MCI_STATUS_VAL, Mce, MceHwErr, MceRecordSource, mce_prep_record_common,
};
use crate::include::uapi::errno::{EINVAL, ENODEV, EOPNOTSUPP};

pub const MCJ_CTX_MASK: u8 = 3;
pub const MCJ_CTX_RANDOM: u8 = 0;
pub const MCJ_CTX_PROCESS: u8 = 1;
pub const MCJ_CTX_IRQ: u8 = 2;
pub const MCJ_NMI_BROADCAST: u8 = 4;
pub const MCJ_EXCEPTION: u8 = 8;
pub const MCJ_IRQ_BROADCAST: u8 = 0x10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InjectionType {
    Software,
    Hardware,
    DeferredInterrupt,
    ThresholdInterrupt,
}

pub fn parse_injection_type(buf: &str, hardware_possible: bool) -> Result<InjectionType, i32> {
    match buf.trim() {
        "sw" => Ok(InjectionType::Software),
        "hw" if hardware_possible => Ok(InjectionType::Hardware),
        "df" if hardware_possible => Ok(InjectionType::DeferredInterrupt),
        "th" if hardware_possible => Ok(InjectionType::ThresholdInterrupt),
        _ => Err(EINVAL),
    }
}

pub fn setup_inj_struct<S: MceRecordSource>(source: &S) -> Mce {
    let mut m = Mce::default();
    mce_prep_record_common(source, &mut m);
    m.microcode = source.microcode();
    m.extcpu = source.cpu();
    m.cpu = source.cpu() as u8;
    m
}

pub fn set_injection_bank(
    m: &mut Mce,
    bank: u8,
    mcg_cap: u64,
    smca: bool,
    ipid_populated: bool,
    injection_type: InjectionType,
) -> Result<(), i32> {
    let n_banks = (mcg_cap & MCG_BANKCNT_MASK) as u8;
    if bank >= n_banks {
        return Err(EINVAL);
    }
    if smca && injection_type != InjectionType::Software && !ipid_populated {
        return Err(ENODEV);
    }
    m.bank = bank;
    Ok(())
}

pub fn prepare_injection(m: &mut Mce, injection_type: InjectionType) {
    m.status |= MCI_STATUS_VAL;
    if m.addr != 0 {
        m.status |= MCI_STATUS_ADDRV;
    }
    if m.misc != 0 {
        m.status |= MCI_STATUS_MISCV;
    }
    if m.synd != 0 {
        m.status |= MCI_STATUS_SYNDV;
    }
    if injection_type == InjectionType::DeferredInterrupt {
        m.status |= MCI_STATUS_DEFERRED;
        m.status &= !MCI_STATUS_UC;
    }
    if injection_type != InjectionType::Software {
        m.mcgstatus = MCG_STATUS_MCIP | MCG_STATUS_EIPV;
        if (m.status & MCI_STATUS_PCC) == 0 {
            m.mcgstatus |= MCG_STATUS_RIPV;
        }
        m.inject_flags = match injection_type {
            InjectionType::Software => 0,
            InjectionType::Hardware => MCJ_EXCEPTION,
            InjectionType::DeferredInterrupt => MCJ_EXCEPTION | MCJ_CTX_IRQ,
            InjectionType::ThresholdInterrupt => MCJ_CTX_IRQ,
        };
    }
}

pub trait MceInjectionBackend {
    fn software_log(&mut self, err: MceHwErr) -> Result<(), i32>;
    fn raise_hardware(&mut self, m: Mce, injection_type: InjectionType) -> Result<(), i32>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnsupportedInjection;

impl MceInjectionBackend for UnsupportedInjection {
    fn software_log(&mut self, _err: MceHwErr) -> Result<(), i32> {
        Err(EOPNOTSUPP)
    }

    fn raise_hardware(&mut self, _m: Mce, _injection_type: InjectionType) -> Result<(), i32> {
        Err(EOPNOTSUPP)
    }
}

pub fn trigger_injection<B: MceInjectionBackend>(
    backend: &mut B,
    mut m: Mce,
    injection_type: InjectionType,
) -> Result<(), i32> {
    prepare_injection(&mut m, injection_type);
    if injection_type == InjectionType::Software {
        backend.software_log(MceHwErr {
            m,
            ..MceHwErr::default()
        })
    } else {
        backend.raise_hardware(m, injection_type)
    }
}

#[cfg(test)]
mod tests {
    use super::super::core::StaticRecordSource;
    use super::*;

    #[derive(Default)]
    struct RecordingBackend {
        logged: usize,
        raised: usize,
    }

    impl MceInjectionBackend for RecordingBackend {
        fn software_log(&mut self, _err: MceHwErr) -> Result<(), i32> {
            self.logged += 1;
            Ok(())
        }

        fn raise_hardware(&mut self, _m: Mce, _injection_type: InjectionType) -> Result<(), i32> {
            self.raised += 1;
            Ok(())
        }
    }

    #[test]
    fn flags_parser_rejects_hardware_modes_when_disabled() {
        assert_eq!(
            parse_injection_type("sw\n", false),
            Ok(InjectionType::Software)
        );
        assert_eq!(parse_injection_type("hw", false), Err(EINVAL));
        assert_eq!(
            parse_injection_type("df", true),
            Ok(InjectionType::DeferredInterrupt)
        );
    }

    #[test]
    fn injection_struct_uses_record_source_metadata() {
        let source = StaticRecordSource {
            now: 9,
            cpu: 2,
            microcode: 0x123,
            ..StaticRecordSource::default()
        };
        let m = setup_inj_struct(&source);
        assert_eq!(m.time, 9);
        assert_eq!(m.extcpu, 2);
        assert_eq!(m.microcode, 0x123);
    }

    #[test]
    fn bank_set_checks_capacity_and_smca_population() {
        let mut m = Mce::default();
        assert_eq!(
            set_injection_bank(&mut m, 4, 4, false, true, InjectionType::Software),
            Err(EINVAL)
        );
        assert_eq!(
            set_injection_bank(&mut m, 1, 4, true, false, InjectionType::Hardware),
            Err(ENODEV)
        );
        assert_eq!(
            set_injection_bank(&mut m, 1, 4, true, false, InjectionType::Software),
            Ok(())
        );
    }

    #[test]
    fn deferred_injection_sets_deferred_and_clears_uc() {
        let mut m = Mce {
            status: MCI_STATUS_UC,
            synd: 1,
            ..Mce::default()
        };
        prepare_injection(&mut m, InjectionType::DeferredInterrupt);
        assert_ne!(m.status & MCI_STATUS_DEFERRED, 0);
        assert_eq!(m.status & MCI_STATUS_UC, 0);
        assert_ne!(m.status & MCI_STATUS_SYNDV, 0);
    }

    #[test]
    fn unsupported_hardware_injection_fails_closed() {
        let mut backend = UnsupportedInjection;
        assert_eq!(
            trigger_injection(&mut backend, Mce::default(), InjectionType::Hardware),
            Err(EOPNOTSUPP)
        );
    }

    #[test]
    fn software_injection_can_log_through_backend() {
        let mut backend = RecordingBackend::default();
        trigger_injection(&mut backend, Mce::default(), InjectionType::Software).unwrap();
        assert_eq!(backend.logged, 1);
        assert_eq!(backend.raised, 0);
    }
}
