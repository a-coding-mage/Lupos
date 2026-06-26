//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/mce/dev-mcelog.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/mce/dev-mcelog.c
//! Legacy `/dev/mcelog` buffer model.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/cpu/mce/dev-mcelog.c

use alloc::vec::Vec;

use super::core::{MCE_HANDLED_CEC, MCE_HANDLED_MCELOG, MCE_LOG_MIN_LEN, MCE_LOG_SIGNATURE, Mce};
use crate::arch::x86::kernel::cpu::CpuVendor;
use crate::include::uapi::errno::{EBUSY, EINVAL, ENOTTY, EPERM};

pub const MCE_OVERFLOW: u32 = 1 << 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum McelogIoctl {
    GetRecordLen,
    GetLogLen,
    GetClearFlags,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MceLogBuffer {
    pub signature: [u8; 12],
    capacity: usize,
    entries: Vec<Mce>,
    flags: u32,
}

impl MceLogBuffer {
    pub fn new(requested_len: usize) -> Self {
        let capacity = requested_len.max(MCE_LOG_MIN_LEN);
        Self {
            signature: *MCE_LOG_SIGNATURE,
            capacity,
            entries: Vec::with_capacity(capacity),
            flags: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.capacity
    }

    pub fn next(&self) -> usize {
        self.entries.len()
    }

    pub fn flags(&self) -> u32 {
        self.flags
    }

    pub fn log(&mut self, mce: &mut Mce) -> Result<bool, i32> {
        if (mce.kflags & MCE_HANDLED_CEC) != 0 {
            return Ok(false);
        }
        if self.entries.len() >= self.capacity {
            self.flags |= MCE_OVERFLOW;
            return Ok(false);
        }
        let mut stored = *mce;
        stored.finished = 1;
        stored.kflags = 0;
        self.entries.push(stored);
        if mce.cpuvendor != CpuVendor::Amd {
            mce.kflags |= MCE_HANDLED_MCELOG;
        }
        Ok(true)
    }

    pub fn read_all(&mut self, usize: usize, off: u64) -> Result<Vec<Mce>, i32> {
        if off != 0 || usize < self.capacity * ::core::mem::size_of::<Mce>() {
            return Err(EINVAL);
        }
        let mut out = Vec::new();
        ::core::mem::swap(&mut out, &mut self.entries);
        Ok(out)
    }

    pub fn ioctl(&mut self, cmd: McelogIoctl, capable_sys_admin: bool) -> Result<usize, i32> {
        if !capable_sys_admin {
            return Err(EPERM);
        }
        match cmd {
            McelogIoctl::GetRecordLen => Ok(::core::mem::size_of::<Mce>()),
            McelogIoctl::GetLogLen => Ok(self.capacity),
            McelogIoctl::GetClearFlags => {
                let flags = self.flags as usize;
                self.flags = 0;
                Ok(flags)
            }
            McelogIoctl::Unknown => Err(ENOTTY),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MceChrdevOpenState {
    open_count: usize,
    exclusive: bool,
}

impl MceChrdevOpenState {
    pub fn open(&mut self, exclusive: bool) -> Result<(), i32> {
        if self.exclusive || (self.open_count != 0 && exclusive) {
            return Err(EBUSY);
        }
        self.open_count += 1;
        if exclusive {
            self.exclusive = true;
        }
        Ok(())
    }

    pub fn release(&mut self) {
        self.open_count = self.open_count.saturating_sub(1);
        if self.open_count == 0 {
            self.exclusive = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcelog_records_until_full_then_sets_overflow() {
        let mut log = MceLogBuffer::new(1);
        log.capacity = 1;
        let mut first = Mce {
            cpuvendor: CpuVendor::Intel,
            ..Mce::default()
        };
        let mut second = first;
        assert_eq!(log.log(&mut first), Ok(true));
        assert_ne!(first.kflags & MCE_HANDLED_MCELOG, 0);
        assert_eq!(log.log(&mut second), Ok(false));
        assert_ne!(log.flags(), 0);
    }

    #[test]
    fn mcelog_read_requires_full_buffer_shape_and_clears_entries() {
        let mut log = MceLogBuffer::new(1);
        log.capacity = 1;
        let mut m = Mce::default();
        log.log(&mut m).unwrap();
        assert_eq!(log.read_all(0, 0).map(|_| ()), Err(EINVAL));
        let records = log.read_all(::core::mem::size_of::<Mce>(), 0).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(log.next(), 0);
    }

    #[test]
    fn ioctl_reports_lengths_and_clears_flags() {
        let mut log = MceLogBuffer::new(2);
        log.flags = MCE_OVERFLOW;
        assert_eq!(log.ioctl(McelogIoctl::GetLogLen, true), Ok(MCE_LOG_MIN_LEN));
        assert_eq!(
            log.ioctl(McelogIoctl::GetClearFlags, true),
            Ok(MCE_OVERFLOW as usize)
        );
        assert_eq!(log.flags(), 0);
        assert_eq!(log.ioctl(McelogIoctl::Unknown, true), Err(ENOTTY));
        assert_eq!(log.ioctl(McelogIoctl::GetLogLen, false), Err(EPERM));
    }

    #[test]
    fn exclusive_open_matches_chrdev_rules() {
        let mut state = MceChrdevOpenState::default();
        assert_eq!(state.open(true), Ok(()));
        assert_eq!(state.open(false), Err(EBUSY));
        state.release();
        assert_eq!(state.open(false), Ok(()));
        assert_eq!(state.open(true), Err(EBUSY));
    }
}
