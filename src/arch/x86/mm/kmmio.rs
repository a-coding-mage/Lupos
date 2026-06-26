//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/kmmio.c
//! test-origin: linux:vendor/linux/arch/x86/mm/kmmio.c
//! Kernel MMIO probe registration policy.
//!
//! Mirrors the externally visible registration surface from
//! `vendor/linux/arch/x86/mm/kmmio.c`. Runtime KMMIO trapping is not enabled
//! in Lupos, so registration validates the probe and then fails closed.

use crate::include::uapi::errno::{EINVAL, ENODEV};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KmmioProbe {
    pub addr: u64,
    pub len: u64,
}

pub const fn kmmio_enabled() -> bool {
    false
}

pub const fn validate_probe(probe: KmmioProbe) -> Result<(), i32> {
    if probe.len == 0 {
        return Err(EINVAL);
    }
    match probe.addr.checked_add(probe.len - 1) {
        Some(_) => Ok(()),
        None => Err(EINVAL),
    }
}

pub const fn register_kmmio_probe(probe: KmmioProbe) -> Result<(), i32> {
    match validate_probe(probe) {
        Ok(()) => Err(ENODEV),
        Err(err) => Err(err),
    }
}

pub const fn unregister_kmmio_probe(probe: KmmioProbe) -> Result<(), i32> {
    match validate_probe(probe) {
        Ok(()) => Err(ENODEV),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_probe_is_rejected_before_feature_gate() {
        assert_eq!(
            register_kmmio_probe(KmmioProbe {
                addr: 0x1000,
                len: 0
            }),
            Err(EINVAL)
        );
    }

    #[test]
    fn valid_probe_fails_closed_when_kmmio_disabled() {
        assert_eq!(
            register_kmmio_probe(KmmioProbe {
                addr: 0x1000,
                len: 8
            }),
            Err(ENODEV)
        );
    }
}
