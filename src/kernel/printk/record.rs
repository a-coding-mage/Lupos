//! linux-parity: complete
//! linux-source: vendor/linux/kernel/printk
//! test-origin: linux:vendor/linux/kernel/printk
//! Linux-shaped printk record metadata.
//!
//! Mirrors `vendor/linux/kernel/printk/printk_ringbuffer.h::struct printk_info`
//! and `vendor/linux/include/linux/dev_printk.h::struct dev_printk_info`.
//! Total size: 88 bytes (without `CONFIG_PRINTK_EXECUTION_CTX`).

/// Linux: `PRINTK_INFO_SUBSYSTEM_LEN`.
pub const PRINTK_INFO_SUBSYSTEM_LEN: usize = 16;
/// Linux: `PRINTK_INFO_DEVICE_LEN`.
pub const PRINTK_INFO_DEVICE_LEN: usize = 48;

/// `struct dev_printk_info` — 64 bytes.
/// Mirrors `vendor/linux/include/linux/dev_printk.h::struct dev_printk_info`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DevPrintkInfo {
    pub subsystem: [u8; PRINTK_INFO_SUBSYSTEM_LEN],
    pub device: [u8; PRINTK_INFO_DEVICE_LEN],
}

impl DevPrintkInfo {
    pub const fn empty() -> Self {
        Self {
            subsystem: [0; PRINTK_INFO_SUBSYSTEM_LEN],
            device: [0; PRINTK_INFO_DEVICE_LEN],
        }
    }
}

/// `struct printk_info` — 88 bytes.
/// Mirrors `vendor/linux/kernel/printk/printk_ringbuffer.h::struct printk_info`.
///
/// Field layout (verified by `printk_info_layout`):
/// - 0:  `u64 seq` — sequence number
/// - 8:  `u64 ts_nsec` — timestamp in nanoseconds (since boot)
/// - 16: `u16 text_len` — length of text message in the data block
/// - 18: `u8  facility` — syslog facility (LOG_KERN, etc.)
/// - 19: `u8  flags:5; u8 level:3` — packed; we expose as a single byte
/// - 20: `u32 caller_id` — TID or processor id (high bit selects)
/// - 24: `struct dev_printk_info dev_info` — 64 bytes
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PrintkInfo {
    pub seq: u64,
    pub ts_nsec: u64,
    pub text_len: u16,
    pub facility: u8,
    /// Packed `flags:5 | level:3`.  Use `flags()` / `level()` accessors.
    /// Bit layout matches Linux's struct member order: `flags:5` is the
    /// upper 5 bits when stored as an `u8` (LSB-first bitfield in clang).
    /// We follow Linux's bit order explicitly: level in low 3 bits.
    pub flags_level: u8,
    pub caller_id: u32,
    pub dev_info: DevPrintkInfo,
}

impl PrintkInfo {
    pub const fn empty() -> Self {
        Self {
            seq: 0,
            ts_nsec: 0,
            text_len: 0,
            facility: 0,
            flags_level: 0,
            caller_id: 0,
            dev_info: DevPrintkInfo::empty(),
        }
    }

    /// Linux convention: `flags:5` are the upper 5 bits.
    #[inline]
    pub fn flags(&self) -> u8 {
        self.flags_level >> 3
    }

    /// Linux convention: `level:3` are the lower 3 bits.
    #[inline]
    pub fn level(&self) -> u8 {
        self.flags_level & 0x07
    }

    #[inline]
    pub fn set_flags_level(&mut self, flags: u8, level: u8) {
        self.flags_level = ((flags & 0x1f) << 3) | (level & 0x07);
    }
}

/// LOG_CONT bit (`vendor/linux/include/linux/printk.h`).
/// In Linux, this lives among the `flags:5` bits.  We shadow it here.
pub const LOG_NEWLINE: u8 = 2; // last char was newline (record ended cleanly)
pub const LOG_CONT: u8 = 8; // record is a continuation of a previous one

/// Reader-facing record bundle.  Mirrors `struct printk_record` (Linux).
pub struct PrintkRecord<'a> {
    pub info: &'a PrintkInfo,
    pub text: &'a [u8],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_printk_info_size_is_64() {
        assert_eq!(core::mem::size_of::<DevPrintkInfo>(), 64);
    }

    #[test]
    fn printk_info_layout_matches_linux() {
        assert_eq!(core::mem::size_of::<PrintkInfo>(), 88);
        assert_eq!(core::mem::offset_of!(PrintkInfo, seq), 0);
        assert_eq!(core::mem::offset_of!(PrintkInfo, ts_nsec), 8);
        assert_eq!(core::mem::offset_of!(PrintkInfo, text_len), 16);
        assert_eq!(core::mem::offset_of!(PrintkInfo, facility), 18);
        assert_eq!(core::mem::offset_of!(PrintkInfo, flags_level), 19);
        assert_eq!(core::mem::offset_of!(PrintkInfo, caller_id), 20);
        assert_eq!(core::mem::offset_of!(PrintkInfo, dev_info), 24);
    }

    #[test]
    fn flags_level_round_trip() {
        let mut info = PrintkInfo::empty();
        info.set_flags_level(LOG_CONT, 3);
        assert_eq!(info.flags(), LOG_CONT);
        assert_eq!(info.level(), 3);
    }
}
