//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/svm/sev.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/svm/sev.c
//! AMD SEV enablement policy for SVM guests.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/svm/sev.c

use crate::include::uapi::errno::{EINVAL, ENODEV};

pub const SEV_STATUS_ENABLED: u64 = 1 << 0;
pub const SEV_STATUS_ES_ENABLED: u64 = 1 << 1;
pub const SEV_STATUS_SNP_ENABLED: u64 = 1 << 2;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SevHostState {
    pub sev_supported: bool,
    pub sev_es_supported: bool,
    pub sev_snp_supported: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SevGuestMode {
    Plain,
    Sev,
    SevEs,
    SevSnp,
}

pub const fn sev_status(mode: SevGuestMode) -> u64 {
    match mode {
        SevGuestMode::Plain => 0,
        SevGuestMode::Sev => SEV_STATUS_ENABLED,
        SevGuestMode::SevEs => SEV_STATUS_ENABLED | SEV_STATUS_ES_ENABLED,
        SevGuestMode::SevSnp => SEV_STATUS_ENABLED | SEV_STATUS_ES_ENABLED | SEV_STATUS_SNP_ENABLED,
    }
}

pub const fn validate_sev_launch(host: SevHostState, mode: SevGuestMode) -> Result<u64, i32> {
    match mode {
        SevGuestMode::Plain => Ok(0),
        SevGuestMode::Sev if host.sev_supported => Ok(sev_status(mode)),
        SevGuestMode::SevEs if host.sev_supported && host.sev_es_supported => Ok(sev_status(mode)),
        SevGuestMode::SevSnp
            if host.sev_supported && host.sev_es_supported && host.sev_snp_supported =>
        {
            Ok(sev_status(mode))
        }
        SevGuestMode::Sev | SevGuestMode::SevEs | SevGuestMode::SevSnp => Err(ENODEV),
    }
}

pub const fn sev_measurement_len_is_valid(len: usize) -> Result<(), i32> {
    if len == 0 || len > 4096 {
        Err(EINVAL)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sev_modes_are_feature_gated() {
        let host = SevHostState {
            sev_supported: true,
            sev_es_supported: false,
            sev_snp_supported: false,
        };
        assert_eq!(
            validate_sev_launch(host, SevGuestMode::Sev),
            Ok(SEV_STATUS_ENABLED)
        );
        assert_eq!(validate_sev_launch(host, SevGuestMode::SevEs), Err(ENODEV));
    }

    #[test]
    fn measurement_buffer_rejects_empty_or_oversized_input() {
        assert_eq!(sev_measurement_len_is_valid(0), Err(EINVAL));
        assert!(sev_measurement_len_is_valid(64).is_ok());
        assert_eq!(sev_measurement_len_is_valid(4097), Err(EINVAL));
    }
}
