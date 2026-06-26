//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/mce/genpool.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/mce/genpool.c
//! MCE event pool.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/cpu/mce/genpool.c

use alloc::vec::Vec;

use super::core::{MceEventSink, MceHwErr};
use crate::include::uapi::errno::{EINVAL, ENOSPC};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MceGenPool {
    capacity: usize,
    records: Vec<MceHwErr>,
}

impl MceGenPool {
    pub fn new(capacity: usize) -> Result<Self, i32> {
        if capacity == 0 {
            return Err(EINVAL);
        }
        Ok(Self {
            capacity,
            records: Vec::with_capacity(capacity),
        })
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn add(&mut self, err: MceHwErr) -> Result<bool, i32> {
        if self
            .records
            .iter()
            .any(|old| is_duplicate_mce_record(old, &err))
        {
            return Ok(false);
        }
        if self.records.len() >= self.capacity {
            return Err(ENOSPC);
        }
        self.records.push(err);
        Ok(true)
    }

    pub fn prepare_records(&mut self) -> Vec<MceHwErr> {
        let mut out = Vec::new();
        ::core::mem::swap(&mut out, &mut self.records);
        out
    }

    pub fn process_into<S: MceEventSink>(&mut self, sink: &mut S) -> Result<usize, i32> {
        let records = self.prepare_records();
        let count = records.len();
        for record in records {
            sink.push_mce(record)?;
        }
        Ok(count)
    }
}

impl MceEventSink for MceGenPool {
    fn push_mce(&mut self, err: MceHwErr) -> Result<(), i32> {
        self.add(err).map(|_| ())
    }
}

pub fn is_duplicate_mce_record(a: &MceHwErr, b: &MceHwErr) -> bool {
    a.m.bank == b.m.bank && a.m.status == b.m.status && a.m.addr == b.m.addr && a.m.misc == b.m.misc
}

pub fn mce_gen_pool_init(capacity: usize) -> Result<MceGenPool, i32> {
    MceGenPool::new(capacity)
}

#[cfg(test)]
mod tests {
    use super::super::core::{Mce, MceHwErr};
    use super::*;

    fn err(bank: u8, status: u64) -> MceHwErr {
        MceHwErr {
            m: Mce {
                bank,
                status,
                addr: 0x1000,
                misc: 0,
                ..Mce::default()
            },
            ..MceHwErr::default()
        }
    }

    #[test]
    fn genpool_rejects_zero_capacity() {
        assert_eq!(MceGenPool::new(0).map(|_| ()), Err(EINVAL));
    }

    #[test]
    fn genpool_suppresses_duplicates_and_bounds_capacity() {
        let mut pool = MceGenPool::new(2).unwrap();
        assert_eq!(pool.add(err(1, 10)), Ok(true));
        assert_eq!(pool.add(err(1, 10)), Ok(false));
        assert_eq!(pool.add(err(2, 10)), Ok(true));
        assert_eq!(pool.add(err(3, 10)), Err(ENOSPC));
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn prepare_records_drains_in_insert_order() {
        let mut pool = MceGenPool::new(4).unwrap();
        pool.add(err(2, 1)).unwrap();
        pool.add(err(1, 2)).unwrap();
        let records = pool.prepare_records();
        assert!(pool.is_empty());
        assert_eq!(records[0].m.bank, 2);
        assert_eq!(records[1].m.bank, 1);
    }
}
