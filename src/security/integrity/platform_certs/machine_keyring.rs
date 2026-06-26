//! linux-parity: complete
//! linux-source: vendor/linux/security/integrity/platform_certs/machine_keyring.c
//! test-origin: linux:vendor/linux/security/integrity/platform_certs/machine_keyring.c
//! Machine keyring initialization and MOK trust policy.

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use spin::Mutex;

use crate::security::{certs, keys, platform_certs};

pub const MACHINE_KEYRING_NAME: &str = ".machine";
pub const KEY_POS_ALL: u32 = 0x3f00_0000;
pub const KEY_POS_SETATTR: u32 = 0x2000_0000;
pub const KEY_USR_VIEW: u32 = 0x0001_0000;
pub const MACHINE_KEYRING_PERM: u32 = (KEY_POS_ALL & !KEY_POS_SETATTR) | KEY_USR_VIEW;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachineCertificate {
    pub key_id: i32,
    pub source: String,
    pub description: String,
    pub len: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachineKeyringState {
    pub machine_keyring: i32,
    pub loaded_certificates: Vec<MachineCertificate>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachineTrustConfig {
    pub efi_boot: bool,
    pub mok_list_trusted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MachineKeyringLoad {
    Machine(MachineCertificate),
    Platform(platform_certs::PlatformCertificate),
    Rejected,
}

static MACHINE_KEYRING: Mutex<Option<MachineKeyringState>> = Mutex::new(None);

pub fn machine_keyring_init() -> Result<MachineKeyringState, i32> {
    if let Some(state) = MACHINE_KEYRING.lock().clone() {
        return Ok(state);
    }

    let keyring_id = keys::add_key("keyring", MACHINE_KEYRING_NAME, &[]);
    if keyring_id < 0 {
        return Err(keyring_id);
    }
    keys::set_perm(keyring_id, MACHINE_KEYRING_PERM)?;
    crate::kernel::printk::log_info!("integrity", "Machine keyring initialized");

    let state = MachineKeyringState {
        machine_keyring: keyring_id,
        loaded_certificates: Vec::new(),
    };
    *MACHINE_KEYRING.lock() = Some(state.clone());
    Ok(state)
}

pub fn machine_keyring_id() -> Option<i32> {
    MACHINE_KEYRING
        .lock()
        .as_ref()
        .map(|state| state.machine_keyring)
}

pub fn snapshot() -> Option<MachineKeyringState> {
    MACHINE_KEYRING.lock().clone()
}

pub fn add_to_machine_keyring(
    source: &str,
    data: &[u8],
    efi_boot: bool,
    platform_keyring_enabled: bool,
) -> Result<MachineKeyringLoad, i32> {
    let mut state = machine_keyring_init()?;
    if let Some(certificate) = import_machine_x509(&state, source, data) {
        state.loaded_certificates.push(certificate.clone());
        *MACHINE_KEYRING.lock() = Some(state);
        return Ok(MachineKeyringLoad::Machine(certificate));
    }

    if efi_boot && platform_keyring_enabled {
        if let Some(certificate) = platform_certs::add_to_platform_keyring(source, data)? {
            return Ok(MachineKeyringLoad::Platform(certificate));
        }
    }

    crate::kernel::printk::log_info!(
        "integrity",
        "Error adding keys to machine keyring {}",
        source
    );
    Ok(MachineKeyringLoad::Rejected)
}

pub const fn imputed_trust_enabled(config: MachineTrustConfig) -> bool {
    if config.efi_boot {
        config.mok_list_trusted
    } else {
        true
    }
}

fn import_machine_x509(
    state: &MachineKeyringState,
    source: &str,
    cert_der: &[u8],
) -> Option<MachineCertificate> {
    if certs::x509_der_len_at(cert_der, 0).ok()? != cert_der.len() {
        return None;
    }
    let fallback = format!(
        "{} certificate #{}",
        source,
        state.loaded_certificates.len()
    );
    let description = certs::x509_certificate_description(cert_der).unwrap_or(fallback);
    let certificate = certs::load_x509_certificate(&description, cert_der)?;
    keys::link_key_to_keyring(certificate.key_id, state.machine_keyring).ok()?;
    Some(MachineCertificate {
        key_id: certificate.key_id,
        source: source.to_string(),
        description: certificate.description,
        len: certificate.len,
    })
}

#[cfg(test)]
pub fn reset_for_test() {
    *MACHINE_KEYRING.lock() = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn reset_all() {
        reset_for_test();
        keys::reset_for_test();
        keys::init();
        certs::reset_for_test();
        platform_certs::reset_for_test();
    }

    fn der_with_common_name(name: &str) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&[0x06, 0x03, 0x55, 0x04, 0x03, 0x0c, name.len() as u8]);
        body.extend_from_slice(name.as_bytes());

        let mut der = Vec::new();
        der.extend_from_slice(&[0x30, 0x82]);
        der.extend_from_slice(&(body.len() as u16).to_be_bytes());
        der.extend_from_slice(&body);
        der
    }

    #[test]
    fn machine_keyring_init_and_import_match_linux_source() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        reset_all();

        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/integrity/platform_certs/machine_keyring.c"
        ));
        assert!(source.contains("integrity_init_keyring(INTEGRITY_KEYRING_MACHINE);"));
        assert!(source.contains("pr_notice(\"Machine keyring initialized\\n\");"));
        assert!(source.contains("device_initcall(machine_keyring_init);"));
        assert!(source.contains("add_to_machine_keyring(const char *source"));
        assert!(source.contains("perm = (KEY_POS_ALL & ~KEY_POS_SETATTR) | KEY_USR_VIEW;"));
        assert!(source.contains("integrity_load_cert(INTEGRITY_KEYRING_MACHINE"));
        assert!(source.contains("integrity_load_cert(INTEGRITY_KEYRING_PLATFORM"));
        assert!(source.contains("efi_mokvar_entry_find(\"MokListTrustedRT\")"));
        assert!(source.contains("bool __init imputed_trust_enabled(void)"));

        let state = machine_keyring_init().expect("machine keyring");
        assert!(state.machine_keyring > 0);
        assert_eq!(
            keys::describe(state.machine_keyring).as_deref(),
            Some("keyring;0;0;1f010000;.machine")
        );
        assert_eq!(machine_keyring_id(), Some(state.machine_keyring));

        let cert = der_with_common_name("UEFI Machine CA");
        let loaded = add_to_machine_keyring("MOK:db", &cert, true, true).expect("machine import");
        let MachineKeyringLoad::Machine(machine) = loaded else {
            panic!("expected machine keyring import");
        };
        assert_eq!(machine.source, "MOK:db");
        assert_eq!(machine.description, "UEFI Machine CA");
        assert_eq!(
            snapshot().expect("snapshot").loaded_certificates[0].description,
            "UEFI Machine CA"
        );

        assert!(imputed_trust_enabled(MachineTrustConfig {
            efi_boot: false,
            mok_list_trusted: false
        }));
        assert!(!imputed_trust_enabled(MachineTrustConfig {
            efi_boot: true,
            mok_list_trusted: false
        }));
        assert!(imputed_trust_enabled(MachineTrustConfig {
            efi_boot: true,
            mok_list_trusted: true
        }));
    }
}
