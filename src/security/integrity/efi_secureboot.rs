//! linux-parity: complete
//! linux-source: vendor/linux/security/integrity/efi_secureboot.c
//! test-origin: linux:vendor/linux/security/integrity/efi_secureboot.c
//! EFI secure boot status query.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EfiSecurebootMode {
    Unset,
    Unknown,
    Disabled,
    Enabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SecurebootState {
    pub initialized: bool,
    pub sb_mode: EfiSecurebootMode,
}

impl SecurebootState {
    pub const fn new() -> Self {
        Self {
            initialized: false,
            sb_mode: EfiSecurebootMode::Unset,
        }
    }
}

pub fn get_sb_mode(
    get_variable_supported: bool,
    firmware_mode: EfiSecurebootMode,
) -> EfiSecurebootMode {
    if !get_variable_supported {
        crate::kernel::printk::log_info!("integrity", "secureboot mode unknown, no efi");
        return EfiSecurebootMode::Unknown;
    }

    match firmware_mode {
        EfiSecurebootMode::Disabled => {
            crate::kernel::printk::log_info!("integrity", "secureboot mode disabled")
        }
        EfiSecurebootMode::Unknown | EfiSecurebootMode::Unset => {
            crate::kernel::printk::log_info!("integrity", "secureboot mode unknown")
        }
        EfiSecurebootMode::Enabled => {
            crate::kernel::printk::log_info!("integrity", "secureboot mode enabled")
        }
    }

    match firmware_mode {
        EfiSecurebootMode::Unset => EfiSecurebootMode::Unknown,
        mode => mode,
    }
}

pub fn arch_get_secureboot(
    state: &mut SecurebootState,
    efi_boot_enabled: bool,
    arch_efi_boot_mode: EfiSecurebootMode,
    get_variable_supported: bool,
    firmware_mode: EfiSecurebootMode,
) -> bool {
    if !state.initialized && efi_boot_enabled {
        state.sb_mode = arch_efi_boot_mode;
        if state.sb_mode == EfiSecurebootMode::Unset {
            state.sb_mode = get_sb_mode(get_variable_supported, firmware_mode);
        }
        state.initialized = true;
    }

    state.sb_mode == EfiSecurebootMode::Enabled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arch_get_secureboot_initializes_once_and_reports_enabled_only() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/integrity/efi_secureboot.c"
        ));
        assert!(source.contains("static enum efi_secureboot_mode get_sb_mode(void)"));
        assert!(source.contains("efi_rt_services_supported(EFI_RT_SUPPORTED_GET_VARIABLE)"));
        assert!(source.contains("sb_mode = arch_efi_boot_mode;"));
        assert!(source.contains("if (sb_mode == efi_secureboot_mode_unset)"));
        assert!(source.contains("bool arch_get_secureboot(void)"));

        let mut state = SecurebootState::new();
        assert!(!arch_get_secureboot(
            &mut state,
            false,
            EfiSecurebootMode::Unset,
            true,
            EfiSecurebootMode::Enabled
        ));
        assert_eq!(state, SecurebootState::new());

        assert!(arch_get_secureboot(
            &mut state,
            true,
            EfiSecurebootMode::Unset,
            true,
            EfiSecurebootMode::Enabled
        ));
        assert_eq!(state.sb_mode, EfiSecurebootMode::Enabled);

        assert!(arch_get_secureboot(
            &mut state,
            true,
            EfiSecurebootMode::Disabled,
            true,
            EfiSecurebootMode::Disabled
        ));
    }

    #[test]
    fn get_sb_mode_preserves_disabled_and_unknown_edges() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        assert_eq!(
            get_sb_mode(false, EfiSecurebootMode::Enabled),
            EfiSecurebootMode::Unknown
        );
        assert_eq!(
            get_sb_mode(true, EfiSecurebootMode::Disabled),
            EfiSecurebootMode::Disabled
        );
        assert_eq!(
            get_sb_mode(true, EfiSecurebootMode::Unset),
            EfiSecurebootMode::Unknown
        );
    }
}
