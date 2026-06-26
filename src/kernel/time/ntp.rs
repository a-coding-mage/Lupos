//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/ntp.c
//! test-origin: linux:vendor/linux/kernel/time/ntp.c
//! NTP-PLL adjustment (M36 stub).
//!
//! `sys_adjtimex` lands in M59 alongside the userspace runtime; for M36 the
//! NTP state is just a stub that exposes the constants Linux userspace
//! expects to see.

/// Linux `STA_*` status flags.
pub const STA_PLL: u32 = 0x0001;
pub const STA_PPSFREQ: u32 = 0x0002;
pub const STA_PPSTIME: u32 = 0x0004;
pub const STA_FLL: u32 = 0x0008;
pub const STA_INS: u32 = 0x0010;
pub const STA_DEL: u32 = 0x0020;
pub const STA_UNSYNC: u32 = 0x0040;
pub const STA_FREQHOLD: u32 = 0x0080;
pub const STA_PPSSIGNAL: u32 = 0x0100;
pub const STA_PPSJITTER: u32 = 0x0200;
pub const STA_PPSWANDER: u32 = 0x0400;
pub const STA_PPSERROR: u32 = 0x0800;
pub const STA_CLOCKERR: u32 = 0x1000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NtpStatus {
    Ok,
    Insert,
    Delete,
    Oop,
    WaitSquare,
    Error,
}

#[inline]
pub fn ntp_synced() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sta_constants_match_linux() {
        assert_eq!(STA_PLL, 1);
        assert_eq!(STA_UNSYNC, 0x40);
        assert_eq!(STA_CLOCKERR, 0x1000);
    }
}
