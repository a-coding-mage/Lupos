//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/mce/apei.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/mce/apei.c
//! ACPI APEI to x86 MCE conversion.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/cpu/mce/apei.c

use alloc::vec::Vec;

use super::core::{
    MCI_STATUS_ADDRV, MCI_STATUS_EN, MCI_STATUS_MISCV, MCI_STATUS_UC, MCI_STATUS_VAL, Mce,
    MceHwErr, MceRecordSource, mce_prep_record,
};
use crate::include::uapi::errno::{EINVAL, ENODEV};

pub const CPER_MEM_VALID_PA: u64 = 1 << 0;
pub const CPER_MEM_VALID_NODE: u64 = 1 << 1;
pub const CPER_MEM_VALID_CARD: u64 = 1 << 2;
pub const CPER_MEM_VALID_MODULE: u64 = 1 << 3;
pub const CPER_MEM_VALID_BANK: u64 = 1 << 4;
pub const CPER_MEM_VALID_DEVICE: u64 = 1 << 5;
pub const CPER_MEM_VALID_ROW: u64 = 1 << 6;
pub const CPER_MEM_VALID_COLUMN: u64 = 1 << 7;
pub const CPER_MEM_VALID_BIT_POSITION: u64 = 1 << 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApeiSeverity {
    Corrected,
    Recoverable,
    Fatal,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CperSecMemErr {
    pub validation_bits: u64,
    pub physical_addr: u64,
    pub error_status: u64,
    pub node: u16,
    pub card: u16,
    pub module: u16,
    pub bank: u16,
    pub device: u16,
    pub row: u32,
    pub column: u32,
    pub bit_pos: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CperIaProcCtx {
    pub lapic_id: u64,
    pub bank: u8,
    pub status: u64,
    pub addr: u64,
    pub misc: u64,
    pub ipid: u64,
    pub synd: u64,
}

pub fn apei_mce_report_mem_error<S: MceRecordSource>(
    severity: ApeiSeverity,
    mem_err: CperSecMemErr,
    source: &S,
) -> MceHwErr {
    let mut err = MceHwErr::default();
    mce_prep_record(source, &mut err);
    err.m.status = MCI_STATUS_VAL | MCI_STATUS_EN;
    if severity != ApeiSeverity::Corrected {
        err.m.status |= MCI_STATUS_UC;
    }
    if (mem_err.validation_bits & CPER_MEM_VALID_PA) != 0 {
        err.m.status |= MCI_STATUS_ADDRV;
        err.m.addr = mem_err.physical_addr;
    }
    if mem_err.error_status != 0 {
        err.m.status |= MCI_STATUS_MISCV;
        err.m.misc = mem_err.error_status;
    }
    err.m.finished = 1;
    err
}

pub fn apei_smca_report_x86_error<S: MceRecordSource>(
    ctx: CperIaProcCtx,
    lapic_id: u64,
    source: &S,
) -> Result<MceHwErr, i32> {
    if ctx.status == 0 {
        return Err(EINVAL);
    }
    let mut err = MceHwErr::default();
    mce_prep_record(source, &mut err);
    err.m.apicid = lapic_id as u32;
    err.m.bank = ctx.bank;
    err.m.status = ctx.status | MCI_STATUS_VAL;
    err.m.addr = ctx.addr;
    err.m.misc = ctx.misc;
    err.m.ipid = ctx.ipid;
    err.m.synd = ctx.synd;
    err.m.finished = 1;
    Ok(err)
}

pub trait ApeiMceStorage {
    fn write_mce(&mut self, m: Mce) -> Result<u64, i32>;
    fn read_mce(&mut self) -> Result<Option<(u64, Mce)>, i32>;
    fn clear_mce(&mut self, record_id: u64) -> Result<(), i32>;
    fn check_mce(&self) -> bool;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnsupportedApei;

impl ApeiMceStorage for UnsupportedApei {
    fn write_mce(&mut self, _m: Mce) -> Result<u64, i32> {
        Err(ENODEV)
    }

    fn read_mce(&mut self) -> Result<Option<(u64, Mce)>, i32> {
        Ok(None)
    }

    fn clear_mce(&mut self, _record_id: u64) -> Result<(), i32> {
        Err(ENODEV)
    }

    fn check_mce(&self) -> bool {
        false
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ApeiRecordStore {
    next_id: u64,
    records: Vec<(u64, Mce)>,
}

impl ApeiMceStorage for ApeiRecordStore {
    fn write_mce(&mut self, m: Mce) -> Result<u64, i32> {
        self.next_id = self.next_id.saturating_add(1);
        let id = self.next_id;
        self.records.push((id, m));
        Ok(id)
    }

    fn read_mce(&mut self) -> Result<Option<(u64, Mce)>, i32> {
        Ok(self.records.first().copied())
    }

    fn clear_mce(&mut self, record_id: u64) -> Result<(), i32> {
        let Some(pos) = self.records.iter().position(|(id, _)| *id == record_id) else {
            return Err(EINVAL);
        };
        self.records.remove(pos);
        Ok(())
    }

    fn check_mce(&self) -> bool {
        !self.records.is_empty()
    }
}

pub fn apei_write_mce<S: ApeiMceStorage>(storage: &mut S, m: Mce) -> Result<u64, i32> {
    storage.write_mce(m)
}

pub fn apei_read_mce<S: ApeiMceStorage>(storage: &mut S) -> Result<Option<(u64, Mce)>, i32> {
    storage.read_mce()
}

pub fn apei_clear_mce<S: ApeiMceStorage>(storage: &mut S, id: u64) -> Result<(), i32> {
    storage.clear_mce(id)
}

pub fn apei_check_mce<S: ApeiMceStorage>(storage: &S) -> bool {
    storage.check_mce()
}

#[cfg(test)]
mod tests {
    use super::super::core::StaticRecordSource;
    use super::*;

    #[test]
    fn cper_memory_error_converts_to_mce_record() {
        let source = StaticRecordSource {
            now: 10,
            ..StaticRecordSource::default()
        };
        let err = apei_mce_report_mem_error(
            ApeiSeverity::Recoverable,
            CperSecMemErr {
                validation_bits: CPER_MEM_VALID_PA,
                physical_addr: 0x1234_5000,
                error_status: 0xab,
                ..CperSecMemErr::default()
            },
            &source,
        );
        assert_ne!(err.m.status & MCI_STATUS_UC, 0);
        assert_eq!(err.m.addr, 0x1234_5000);
        assert_eq!(err.m.misc, 0xab);
    }

    #[test]
    fn smca_context_requires_status_and_copies_registers() {
        let source = StaticRecordSource::default();
        assert_eq!(
            apei_smca_report_x86_error(CperIaProcCtx::default(), 0, &source).map(|_| ()),
            Err(EINVAL)
        );
        let err = apei_smca_report_x86_error(
            CperIaProcCtx {
                bank: 2,
                status: 0x44,
                addr: 0x1000,
                ipid: 0x55,
                ..CperIaProcCtx::default()
            },
            7,
            &source,
        )
        .unwrap();
        assert_eq!(err.m.bank, 2);
        assert_eq!(err.m.apicid, 7);
        assert_eq!(err.m.ipid, 0x55);
    }

    #[test]
    fn apei_storage_reads_and_clears_records() {
        let mut store = ApeiRecordStore::default();
        let id = apei_write_mce(&mut store, Mce::default()).unwrap();
        assert!(apei_check_mce(&store));
        assert_eq!(apei_read_mce(&mut store).unwrap().unwrap().0, id);
        assert_eq!(apei_clear_mce(&mut store, id), Ok(()));
        assert!(!apei_check_mce(&store));
    }

    #[test]
    fn unsupported_apei_fails_closed() {
        let mut storage = UnsupportedApei;
        assert_eq!(apei_write_mce(&mut storage, Mce::default()), Err(ENODEV));
        assert_eq!(apei_clear_mce(&mut storage, 1), Err(ENODEV));
        assert_eq!(apei_read_mce(&mut storage), Ok(None));
    }
}
