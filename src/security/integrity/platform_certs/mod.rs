//! linux-parity: partial
//! linux-source: vendor/linux/security/integrity/platform_certs
//! Integrity platform certificate loaders.

pub mod efi_parser;
pub mod keyring_handler;
pub mod load_ipl_s390;
pub mod machine_keyring;
pub mod platform_keyring;
