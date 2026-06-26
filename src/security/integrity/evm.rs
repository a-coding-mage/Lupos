//! linux-parity: partial
//! linux-source: vendor/linux/security/integrity/evm/evm_main.c
//! test-origin: linux:vendor/linux/security/integrity/evm/evm_main.c
//! Extended Verification Module boot initialization.
//!
//! Mirrors Linux `evm_init_config()`, `evm_secfs.c` key signalling,
//! `evm_crypto.c` HMAC-SHA1 calculation, `evm_verify_hmac()` signature xattr
//! status handling, `evm_inode_setxattr()`/`evm_inode_removexattr()`/`setattr`
//! hook decisions, POSIX ACL/copy-up decisions, post-update helpers, `.evm`
//! integrity keyring setup, and LSM id registration.

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::types::Inode;
use crate::include::uapi::errno::{ECANCELED, EINVAL, ENODATA, ENOKEY, EOPNOTSUPP, EPERM};
use crate::security::hooks::{LSM_ID_EVM, LsmHooks, NOOP_HOOKS};
use crate::security::lsm_list::register_lsm;

pub const EVM_INIT_HMAC: u32 = 0x0001;
pub const EVM_INIT_X509: u32 = 0x0002;
pub const EVM_ALLOW_METADATA_WRITES: u32 = 0x0004;
pub const EVM_SIGV3_REQUIRED: u32 = 0x0008;
pub const EVM_SETUP_COMPLETE: u32 = 0x8000_0000;
pub const EVM_KEY_MASK: u32 = EVM_INIT_HMAC | EVM_INIT_X509;
pub const EVM_INIT_MASK: u32 =
    EVM_KEY_MASK | EVM_SETUP_COMPLETE | EVM_ALLOW_METADATA_WRITES | EVM_SIGV3_REQUIRED;
pub const EVM_KEY_DESCRIPTION: &str = "evm-key";
pub const EVM_KEYRING_NAME: &str = ".evm";
pub const EVM_MAX_KEY_SIZE: usize = 128;
pub const EVM_XATTR_NAME: &str = "security.evm";
pub const EVM_XATTR_HMAC: u8 = 0x02;
pub const EVM_IMA_XATTR_DIGSIG: u8 = 0x03;
pub const EVM_XATTR_PORTABLE_DIGSIG: u8 = 0x05;
pub const EVM_ATTR_FSUUID: usize = 0x0001;
pub const EVM_HMAC_DIGEST_SIZE: usize = crate::security::integrity::ima::IMA_SHA1_DIGEST_SIZE;
pub const EVM_XATTR_HMAC_SIZE: usize = 1 + EVM_HMAC_DIGEST_SIZE;
pub const POSIX_ACL_ACCESS_XATTR: &str = "system.posix_acl_access";
pub const POSIX_ACL_DEFAULT_XATTR: &str = "system.posix_acl_default";
pub const HASH_ALGO_SHA1: u8 = 2;
pub const HASH_ALGO_SHA256: u8 = crate::security::integrity::ima::HASH_ALGO_SHA256;
pub const HASH_ALGO_LAST: u8 = 23;
pub const SIGNATURE_V2_HDR_SIZE: usize = 9;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvmXattr {
    pub name: &'static str,
    pub enabled: bool,
}

pub const DEFAULT_XATTRS: [EvmXattr; 8] = [
    EvmXattr {
        name: "security.selinux",
        enabled: false,
    },
    EvmXattr {
        name: "security.SMACK64",
        enabled: false,
    },
    EvmXattr {
        name: "security.SMACK64EXEC",
        enabled: false,
    },
    EvmXattr {
        name: "security.SMACK64TRANSMUTE",
        enabled: false,
    },
    EvmXattr {
        name: "security.SMACK64MMAP",
        enabled: false,
    },
    EvmXattr {
        name: "security.apparmor",
        enabled: false,
    },
    EvmXattr {
        name: "security.ima",
        enabled: false,
    },
    EvmXattr {
        name: "security.capability",
        enabled: true,
    },
];

pub const HOOKS: LsmHooks = LsmHooks {
    name: "evm",
    id: LSM_ID_EVM,
    ..NOOP_HOOKS
};

static INITIALIZED: AtomicBool = AtomicBool::new(false);
static XATTR_COUNT: AtomicUsize = AtomicUsize::new(0);
static HMAC_ATTRS: AtomicUsize = AtomicUsize::new(0);
static EVM_INITIALIZED_FLAGS: AtomicU32 = AtomicU32::new(0);
static HMAC_KEY_LEN: AtomicUsize = AtomicUsize::new(0);

lazy_static! {
    static ref HMAC_KEY: Mutex<Option<Vec<u8>>> = Mutex::new(None);
    static ref EVM_KEYRING_ID: Mutex<Option<i32>> = Mutex::new(None);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvmState {
    pub initialized: bool,
    pub xattr_count: usize,
    pub hmac_attrs: usize,
    pub initialized_flags: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvmIntegrityStatus {
    Pass,
    PassImmutable,
    Fail,
    FailImmutable,
    NoLabel,
    NoXattrs,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvmMetadata {
    pub ino: u64,
    pub generation: u32,
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub fsuuid: [u8; 16],
}

impl EvmMetadata {
    pub fn from_inode(inode: &Inode) -> Self {
        Self {
            ino: inode.ino,
            generation: 0,
            uid: inode.uid.load(Ordering::Acquire),
            gid: inode.gid.load(Ordering::Acquire),
            mode: inode.mode.load(Ordering::Acquire),
            fsuuid: inode
                .sb
                .lock()
                .as_ref()
                .map(|sb| sb.uuid())
                .unwrap_or([0; 16]),
        }
    }

    fn append_linux_hmac_misc(self, out: &mut Vec<u8>, xattr_type: u8) {
        self.append_linux_hmac_misc_with_attrs(out, xattr_type, HMAC_ATTRS.load(Ordering::Acquire));
    }

    fn append_linux_hmac_misc_with_attrs(self, out: &mut Vec<u8>, xattr_type: u8, attrs: usize) {
        // Linux `evm_crypto.c::hmac_add_misc()` hashes this zeroed x86_64
        // layout, then appends `inode->i_sb->s_uuid` when
        // `EVM_ATTR_FSUUID` is configured. Portable signatures deliberately
        // omit inode identity and filesystem UUID.
        if xattr_type != EVM_XATTR_PORTABLE_DIGSIG {
            out.extend_from_slice(&self.ino.to_le_bytes());
            out.extend_from_slice(&self.generation.to_le_bytes());
        } else {
            out.extend_from_slice(&0u64.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
        }
        out.extend_from_slice(&self.uid.to_le_bytes());
        out.extend_from_slice(&self.gid.to_le_bytes());
        out.extend_from_slice(&(self.mode as u16).to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        if attrs & EVM_ATTR_FSUUID != 0 && xattr_type != EVM_XATTR_PORTABLE_DIGSIG {
            out.extend_from_slice(&self.fsuuid);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvmProtectedXattr<'a> {
    pub name: &'a str,
    pub value: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvmXattrUpdate {
    Set([u8; EVM_XATTR_HMAC_SIZE]),
    Remove,
}

fn ensure_evm_keyring() -> Result<i32, i32> {
    crate::security::keys::init();
    if let Some(id) = *EVM_KEYRING_ID.lock() {
        return Ok(id);
    }
    if let Some(id) = crate::security::keys::keyring_id_by_description(EVM_KEYRING_NAME) {
        *EVM_KEYRING_ID.lock() = Some(id);
        return Ok(id);
    }
    let id = crate::security::keys::add_key("keyring", EVM_KEYRING_NAME, &[]);
    if id < 0 {
        return Err(id);
    }
    *EVM_KEYRING_ID.lock() = Some(id);
    Ok(id)
}

pub fn evm_keyring_id() -> Option<i32> {
    *EVM_KEYRING_ID.lock()
}

pub fn init() {
    let _ = register_lsm(HOOKS);

    if INITIALIZED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    crate::kernel::printk::log_info!("evm", "Initialising EVM extended attributes:");
    for xattr in DEFAULT_XATTRS {
        if xattr.enabled {
            crate::kernel::printk::log_info!("evm", "{}", xattr.name);
        } else {
            crate::kernel::printk::log_info!("evm", "{} (disabled)", xattr.name);
        }
    }
    XATTR_COUNT.store(DEFAULT_XATTRS.len(), Ordering::Release);
    HMAC_ATTRS.store(EVM_ATTR_FSUUID, Ordering::Release);
    crate::kernel::printk::log_info!("evm", "HMAC attrs: 0x{:x}", EVM_ATTR_FSUUID);
    let _ = ensure_evm_keyring();
    crate::security::integrity::evm_secfs::init_securityfs();
}

pub fn snapshot() -> EvmState {
    EvmState {
        initialized: INITIALIZED.load(Ordering::Acquire),
        xattr_count: XATTR_COUNT.load(Ordering::Acquire),
        hmac_attrs: HMAC_ATTRS.load(Ordering::Acquire),
        initialized_flags: EVM_INITIALIZED_FLAGS.load(Ordering::Acquire),
    }
}

pub fn readable_key_flags() -> u32 {
    EVM_INITIALIZED_FLAGS.load(Ordering::Acquire) & !EVM_SETUP_COMPLETE
}

pub fn hmac_key_loaded() -> bool {
    EVM_INITIALIZED_FLAGS.load(Ordering::Acquire) & EVM_INIT_HMAC != 0
}

pub fn evm_key_loaded() -> bool {
    EVM_INITIALIZED_FLAGS.load(Ordering::Acquire) & EVM_KEY_MASK != 0
}

pub fn hmac_key_len() -> usize {
    HMAC_KEY_LEN.load(Ordering::Acquire)
}

fn hmac_key_bytes() -> Result<Vec<u8>, i32> {
    HMAC_KEY.lock().clone().ok_or(-ENOKEY)
}

fn evm_hmac_disabled() -> bool {
    let flags = EVM_INITIALIZED_FLAGS.load(Ordering::Acquire);
    flags & EVM_INIT_HMAC == 0 && flags & EVM_SETUP_COMPLETE != 0
}

fn set_hmac_key(bytes: &[u8]) -> Result<(), i32> {
    use crate::include::uapi::errno::{EBUSY, EINVAL};

    if bytes.is_empty() || bytes.len() > EVM_MAX_KEY_SIZE {
        return Err(-EINVAL);
    }
    if hmac_key_loaded() {
        return Err(-EBUSY);
    }

    *HMAC_KEY.lock() = Some(bytes.to_vec());
    HMAC_KEY_LEN.store(bytes.len(), Ordering::Release);
    EVM_INITIALIZED_FLAGS.fetch_or(EVM_INIT_HMAC, Ordering::AcqRel);
    crate::kernel::printk::log_info!("evm", "key initialized");
    Ok(())
}

fn configured_xattr_matches(configured_name: &str, requested_name: &str) -> bool {
    if configured_name == requested_name {
        return true;
    }
    configured_name
        .strip_prefix("security.")
        .is_some_and(|suffix| suffix.starts_with(requested_name))
}

pub fn protected_xattr_enabled(name: &str) -> bool {
    DEFAULT_XATTRS
        .iter()
        .find(|xattr| configured_xattr_matches(xattr.name, name))
        .is_some_and(|xattr| xattr.enabled)
}

pub fn protected_xattr_known(name: &str) -> bool {
    DEFAULT_XATTRS
        .iter()
        .any(|xattr| configured_xattr_matches(xattr.name, name))
}

pub fn protected_xattr_count(xattrs: &[EvmProtectedXattr<'_>]) -> usize {
    DEFAULT_XATTRS
        .iter()
        .filter(|configured| xattrs.iter().any(|present| present.name == configured.name))
        .count()
}

pub fn posix_xattr_acl(name: &str) -> bool {
    name == POSIX_ACL_ACCESS_XATTR || name == POSIX_ACL_DEFAULT_XATTR
}

pub fn evm_revalidate_status(xattr_name: Option<&str>) -> bool {
    if !hmac_key_loaded() {
        return false;
    }

    let Some(xattr_name) = xattr_name else {
        return true;
    };

    protected_xattr_enabled(xattr_name)
        || posix_xattr_acl(xattr_name)
        || xattr_name == EVM_XATTR_NAME
}

fn metadata_writes_allowed() -> bool {
    EVM_INITIALIZED_FLAGS.load(Ordering::Acquire) & EVM_ALLOW_METADATA_WRITES != 0
}

fn protect_xattr_common(
    xattr_name: &str,
    current_status: EvmIntegrityStatus,
    cap_sys_admin: bool,
    unsupported_hmac_fs: bool,
    value_changed: bool,
) -> Result<(), i32> {
    if xattr_name == EVM_XATTR_NAME {
        if !cap_sys_admin || unsupported_hmac_fs {
            return Err(-EPERM);
        }
    } else if !protected_xattr_enabled(xattr_name) {
        if !posix_xattr_acl(xattr_name) || unsupported_hmac_fs {
            return Ok(());
        }
        if matches!(
            current_status,
            EvmIntegrityStatus::Pass | EvmIntegrityStatus::NoXattrs
        ) {
            return Ok(());
        }
    } else if unsupported_hmac_fs {
        return Ok(());
    }

    if current_status == EvmIntegrityStatus::NoXattrs && evm_hmac_disabled() {
        return Ok(());
    }

    if evm_hmac_disabled()
        && matches!(
            current_status,
            EvmIntegrityStatus::NoLabel | EvmIntegrityStatus::Unknown
        )
    {
        return Ok(());
    }

    if current_status == EvmIntegrityStatus::FailImmutable {
        return Ok(());
    }

    if current_status == EvmIntegrityStatus::PassImmutable && !value_changed {
        return Ok(());
    }

    if current_status == EvmIntegrityStatus::Pass {
        Ok(())
    } else {
        Err(-EPERM)
    }
}

pub fn protect_setxattr(
    xattr_name: &str,
    xattr_value: &[u8],
    current_status: EvmIntegrityStatus,
    cap_sys_admin: bool,
    unsupported_hmac_fs: bool,
    value_changed: bool,
) -> Result<(), i32> {
    if metadata_writes_allowed() {
        return Ok(());
    }

    if xattr_name == EVM_XATTR_NAME {
        let Some(xattr_type) = xattr_value.first().copied() else {
            return Err(-EINVAL);
        };
        if !matches!(xattr_type, EVM_IMA_XATTR_DIGSIG | EVM_XATTR_PORTABLE_DIGSIG) {
            return Err(-EPERM);
        }
    }

    protect_xattr_common(
        xattr_name,
        current_status,
        cap_sys_admin,
        unsupported_hmac_fs,
        value_changed,
    )
}

pub fn protect_removexattr(
    xattr_name: &str,
    current_status: EvmIntegrityStatus,
    cap_sys_admin: bool,
    unsupported_hmac_fs: bool,
) -> Result<(), i32> {
    if metadata_writes_allowed() {
        return Ok(());
    }

    protect_xattr_common(
        xattr_name,
        current_status,
        cap_sys_admin,
        unsupported_hmac_fs,
        true,
    )
}

pub fn protect_setattr(
    current_status: EvmIntegrityStatus,
    unsupported_hmac_fs: bool,
    metadata_changed: bool,
) -> Result<(), i32> {
    if metadata_writes_allowed() || unsupported_hmac_fs || !metadata_changed {
        return Ok(());
    }

    if matches!(
        current_status,
        EvmIntegrityStatus::Pass | EvmIntegrityStatus::NoXattrs | EvmIntegrityStatus::FailImmutable
    ) {
        return Ok(());
    }

    if evm_hmac_disabled()
        && matches!(
            current_status,
            EvmIntegrityStatus::NoLabel | EvmIntegrityStatus::Unknown
        )
    {
        return Ok(());
    }

    Err(-EPERM)
}

pub fn protect_set_acl(
    current_status: EvmIntegrityStatus,
    unsupported_hmac_fs: bool,
    acl_changes_mode: bool,
) -> Result<(), i32> {
    if metadata_writes_allowed() || unsupported_hmac_fs {
        return Ok(());
    }

    if matches!(
        current_status,
        EvmIntegrityStatus::Pass | EvmIntegrityStatus::NoXattrs
    ) {
        return Ok(());
    }

    if evm_hmac_disabled()
        && matches!(
            current_status,
            EvmIntegrityStatus::NoLabel | EvmIntegrityStatus::Unknown
        )
    {
        return Ok(());
    }

    if current_status == EvmIntegrityStatus::FailImmutable {
        return Ok(());
    }

    if current_status == EvmIntegrityStatus::PassImmutable && !acl_changes_mode {
        return Ok(());
    }

    Err(-EPERM)
}

pub fn protect_remove_acl(
    current_status: EvmIntegrityStatus,
    unsupported_hmac_fs: bool,
) -> Result<(), i32> {
    protect_set_acl(current_status, unsupported_hmac_fs, true)
}

pub fn copy_up_xattr(name: &str, evm_xattr: Option<&[u8]>) -> Result<(), i32> {
    if name != EVM_XATTR_NAME {
        return Err(-EOPNOTSUPP);
    }

    let Some(evm_xattr) = evm_xattr else {
        return Err(-EPERM);
    };
    match evm_xattr.first().copied() {
        Some(EVM_XATTR_PORTABLE_DIGSIG) => Ok(()),
        Some(EVM_XATTR_HMAC | EVM_IMA_XATTR_DIGSIG) | Some(_) => Err(-ECANCELED),
        None => Err(-EPERM),
    }
}

fn hmac_sha1(key: &[u8], message: &[u8]) -> [u8; EVM_HMAC_DIGEST_SIZE] {
    const HMAC_BLOCK_SIZE: usize = 64;
    let mut key_block = [0u8; HMAC_BLOCK_SIZE];
    if key.len() > HMAC_BLOCK_SIZE {
        key_block[..EVM_HMAC_DIGEST_SIZE]
            .copy_from_slice(&crate::security::integrity::ima::sha1_digest(key));
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }

    let mut inner = Vec::with_capacity(HMAC_BLOCK_SIZE + message.len());
    let mut outer = Vec::with_capacity(HMAC_BLOCK_SIZE + EVM_HMAC_DIGEST_SIZE);
    for byte in key_block {
        inner.push(byte ^ 0x36);
        outer.push(byte ^ 0x5c);
    }
    inner.extend_from_slice(message);
    let inner_digest = crate::security::integrity::ima::sha1_digest(&inner);
    outer.extend_from_slice(&inner_digest);
    crate::security::integrity::ima::sha1_digest(&outer)
}

fn append_configured_xattrs_in_linux_order(
    message: &mut Vec<u8>,
    xattrs: &[EvmProtectedXattr<'_>],
    xattr_type: u8,
) -> (bool, bool) {
    let include_disabled = xattr_type == EVM_XATTR_PORTABLE_DIGSIG;
    let mut found = false;
    let mut ima_present = false;
    for configured in DEFAULT_XATTRS {
        if !configured.enabled && !include_disabled {
            continue;
        }
        if let Some(present) = xattrs
            .iter()
            .find(|present| present.name == configured.name)
        {
            message.extend_from_slice(present.value);
            found = true;
            if configured.name == "security.ima" {
                ima_present = true;
            }
        }
    }
    (found, ima_present)
}

pub fn calc_hmac(
    metadata: EvmMetadata,
    xattrs: &[EvmProtectedXattr<'_>],
) -> Result<[u8; EVM_HMAC_DIGEST_SIZE], i32> {
    let key = hmac_key_bytes()?;
    let mut message = Vec::new();
    let (found, _) = append_configured_xattrs_in_linux_order(&mut message, xattrs, EVM_XATTR_HMAC);
    metadata.append_linux_hmac_misc(&mut message, EVM_XATTR_HMAC);
    if !found {
        return Err(-ENODATA);
    }
    Ok(hmac_sha1(&key, &message))
}

pub fn calc_hash(
    metadata: EvmMetadata,
    xattrs: &[EvmProtectedXattr<'_>],
    xattr_type: u8,
    hash_algo: u8,
) -> Result<Vec<u8>, i32> {
    let mut message = Vec::new();
    let (found, ima_present) =
        append_configured_xattrs_in_linux_order(&mut message, xattrs, xattr_type);
    metadata.append_linux_hmac_misc(&mut message, xattr_type);
    if xattr_type == EVM_XATTR_PORTABLE_DIGSIG && !ima_present {
        return Err(-EPERM);
    }
    if !found {
        return Err(-ENODATA);
    }
    crate::security::integrity::ima::digest_for_algo(hash_algo, &message)
}

pub fn build_hmac_xattr(
    metadata: EvmMetadata,
    xattrs: &[EvmProtectedXattr<'_>],
) -> Result<[u8; EVM_XATTR_HMAC_SIZE], i32> {
    let digest = calc_hmac(metadata, xattrs)?;
    let mut out = [0u8; EVM_XATTR_HMAC_SIZE];
    out[0] = EVM_XATTR_HMAC;
    out[1..].copy_from_slice(&digest);
    Ok(out)
}

pub fn update_evmxattr(
    metadata: EvmMetadata,
    protected_xattrs: &[EvmProtectedXattr<'_>],
) -> Result<EvmXattrUpdate, i32> {
    match build_hmac_xattr(metadata, protected_xattrs) {
        Ok(xattr) => Ok(EvmXattrUpdate::Set(xattr)),
        Err(errno) if errno == -ENODATA => Ok(EvmXattrUpdate::Remove),
        Err(errno) => Err(errno),
    }
}

pub fn post_setxattr_update(
    metadata: EvmMetadata,
    xattr_name: &str,
    protected_xattrs: &[EvmProtectedXattr<'_>],
    unsupported_hmac_fs: bool,
) -> Result<Option<EvmXattrUpdate>, i32> {
    if !evm_revalidate_status(Some(xattr_name)) {
        return Ok(None);
    }

    if xattr_name == EVM_XATTR_NAME {
        return Ok(None);
    }

    if !hmac_key_loaded() || unsupported_hmac_fs {
        return Ok(None);
    }

    update_evmxattr(metadata, protected_xattrs).map(Some)
}

pub fn post_removexattr_update(
    metadata: EvmMetadata,
    xattr_name: &str,
    protected_xattrs: &[EvmProtectedXattr<'_>],
) -> Result<Option<EvmXattrUpdate>, i32> {
    if !evm_revalidate_status(Some(xattr_name)) {
        return Ok(None);
    }

    if xattr_name == EVM_XATTR_NAME {
        return Ok(None);
    }

    if !hmac_key_loaded() {
        return Ok(None);
    }

    update_evmxattr(metadata, protected_xattrs).map(Some)
}

pub fn post_set_acl_update(
    metadata: EvmMetadata,
    acl_name: &str,
    protected_xattrs: &[EvmProtectedXattr<'_>],
    unsupported_hmac_fs: bool,
) -> Result<Option<EvmXattrUpdate>, i32> {
    post_setxattr_update(metadata, acl_name, protected_xattrs, unsupported_hmac_fs)
}

pub fn post_remove_acl_update(
    metadata: EvmMetadata,
    acl_name: &str,
    protected_xattrs: &[EvmProtectedXattr<'_>],
) -> Result<Option<EvmXattrUpdate>, i32> {
    post_removexattr_update(metadata, acl_name, protected_xattrs)
}

pub fn post_setattr_update(
    metadata: EvmMetadata,
    protected_xattrs: &[EvmProtectedXattr<'_>],
    unsupported_hmac_fs: bool,
    metadata_changed: bool,
) -> Result<Option<EvmXattrUpdate>, i32> {
    if !evm_revalidate_status(None) {
        return Ok(None);
    }

    if !hmac_key_loaded() || unsupported_hmac_fs || !metadata_changed {
        return Ok(None);
    }

    update_evmxattr(metadata, protected_xattrs).map(Some)
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right.iter())
        .fold(0u8, |acc, (left, right)| acc | (left ^ right))
        == 0
}

fn evm_sigv3_required() -> bool {
    EVM_INITIALIZED_FLAGS.load(Ordering::Acquire) & EVM_SIGV3_REQUIRED != 0
}

fn signature_failure_status(immutable: bool) -> EvmIntegrityStatus {
    if immutable {
        EvmIntegrityStatus::FailImmutable
    } else {
        EvmIntegrityStatus::Fail
    }
}

fn verify_signature_digest(evm_xattr: &[u8], digest: &[u8], immutable: bool) -> EvmIntegrityStatus {
    let Ok(keyring) = ensure_evm_keyring() else {
        return signature_failure_status(immutable);
    };
    match crate::security::integrity::ima::verify_signature_v2_digest_with_keyring(
        keyring, evm_xattr, digest,
    ) {
        Ok(()) if immutable => EvmIntegrityStatus::PassImmutable,
        Ok(()) => EvmIntegrityStatus::Pass,
        Err(_) => signature_failure_status(immutable),
    }
}

fn verify_signature_xattr(
    metadata: EvmMetadata,
    xattrs: &[EvmProtectedXattr<'_>],
    evm_xattr: &[u8],
    immutable: bool,
) -> EvmIntegrityStatus {
    if evm_xattr.len() <= SIGNATURE_V2_HDR_SIZE {
        return EvmIntegrityStatus::Fail;
    }

    let version = evm_xattr[1];
    if evm_sigv3_required() && version != 3 {
        return EvmIntegrityStatus::Fail;
    }

    let hash_algo = evm_xattr[2];
    let sig_size = u16::from_be_bytes([evm_xattr[7], evm_xattr[8]]) as usize;
    let payload_len = evm_xattr.len() - SIGNATURE_V2_HDR_SIZE;
    let structurally_valid =
        matches!(version, 1..=3) && hash_algo < HASH_ALGO_LAST && sig_size == payload_len;

    if !structurally_valid {
        return signature_failure_status(immutable);
    }

    let xattr_type = evm_xattr[0];
    match calc_hash(metadata, xattrs, xattr_type, hash_algo) {
        Ok(digest) => verify_signature_digest(evm_xattr, &digest, immutable),
        Err(errno) if errno == -ENODATA => EvmIntegrityStatus::NoXattrs,
        Err(_) => signature_failure_status(immutable),
    }
}

pub fn verify_hmac_xattr(
    metadata: EvmMetadata,
    evm_xattr: Option<&[u8]>,
    protected_xattrs: &[EvmProtectedXattr<'_>],
) -> EvmIntegrityStatus {
    let Some(evm_xattr) = evm_xattr else {
        return if protected_xattr_count(protected_xattrs) > 0 {
            EvmIntegrityStatus::NoLabel
        } else {
            EvmIntegrityStatus::NoXattrs
        };
    };

    match evm_xattr.first().copied() {
        Some(EVM_XATTR_HMAC) => {
            if evm_xattr.len() != EVM_XATTR_HMAC_SIZE {
                return EvmIntegrityStatus::Fail;
            }
            match calc_hmac(metadata, protected_xattrs) {
                Ok(digest) if constant_time_eq(&evm_xattr[1..], &digest) => {
                    EvmIntegrityStatus::Pass
                }
                Err(errno) if errno == -ENODATA => EvmIntegrityStatus::NoXattrs,
                _ => EvmIntegrityStatus::Fail,
            }
        }
        Some(EVM_XATTR_PORTABLE_DIGSIG) => {
            verify_signature_xattr(metadata, protected_xattrs, evm_xattr, true)
        }
        Some(EVM_IMA_XATTR_DIGSIG) => {
            verify_signature_xattr(metadata, protected_xattrs, evm_xattr, false)
        }
        _ => EvmIntegrityStatus::Fail,
    }
}

fn init_hmac_key_from_keyring() -> Result<(), i32> {
    use crate::include::uapi::errno::ENOENT;

    let key = crate::security::keys::request_key("encrypted", EVM_KEY_DESCRIPTION);
    if key < 0 {
        return Err(-ENOENT);
    }
    let payload = crate::security::keys::read(key)?;
    let result = set_hmac_key(&payload);
    let zeros = alloc::vec![0u8; payload.len()];
    let _ = crate::security::keys::update(key, &zeros);
    result
}

pub fn write_key_flags(value: u32) -> Result<(), i32> {
    use crate::include::uapi::errno::{EINVAL, EPERM};

    if value == 0 || (value & !EVM_INIT_MASK) != 0 {
        return Err(-EINVAL);
    }

    let current = EVM_INITIALIZED_FLAGS.load(Ordering::Acquire);
    if current & EVM_SETUP_COMPLETE != 0 {
        return Err(-EPERM);
    }
    if value & EVM_ALLOW_METADATA_WRITES != 0 && current & EVM_INIT_HMAC != 0 {
        return Err(-EPERM);
    }

    let mut value = value;
    if value & EVM_INIT_HMAC != 0 {
        init_hmac_key_from_keyring()?;
        value |= EVM_SETUP_COMPLETE;
    }
    if value & EVM_INIT_X509 != 0 {
        ensure_evm_keyring()?;
    }

    let mut next = EVM_INITIALIZED_FLAGS.load(Ordering::Acquire) | value;
    if next & EVM_INIT_HMAC != 0 {
        next &= !EVM_ALLOW_METADATA_WRITES;
    }
    EVM_INITIALIZED_FLAGS.store(next, Ordering::Release);
    Ok(())
}

pub fn enabled_xattrs_text() -> alloc::string::String {
    let mut out = alloc::string::String::new();
    for xattr in DEFAULT_XATTRS {
        if xattr.enabled {
            out.push_str(xattr.name);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
pub fn reset_for_test() {
    INITIALIZED.store(false, Ordering::Release);
    XATTR_COUNT.store(0, Ordering::Release);
    HMAC_ATTRS.store(0, Ordering::Release);
    EVM_INITIALIZED_FLAGS.store(0, Ordering::Release);
    HMAC_KEY_LEN.store(0, Ordering::Release);
    *HMAC_KEY.lock() = None;
    *EVM_KEYRING_ID.lock() = None;
}

#[cfg(test)]
fn set_hmac_attrs_for_test(attrs: usize) {
    HMAC_ATTRS.store(attrs, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::hooks::LSM_ID_EVM;
    use crate::security::lsm_list::{TEST_LSM_LOCK, lsm_active_ids, reset_for_test as reset_lsms};

    #[test]
    fn evm_default_xattr_config_matches_linux_order() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        assert_eq!(DEFAULT_XATTRS[0].name, "security.selinux");
        assert_eq!(DEFAULT_XATTRS[1].name, "security.SMACK64");
        assert_eq!(DEFAULT_XATTRS[5].name, "security.apparmor");
        assert_eq!(DEFAULT_XATTRS[6].name, "security.ima");
        assert_eq!(DEFAULT_XATTRS[7].name, "security.capability");
        assert!(!DEFAULT_XATTRS[0].enabled);
        assert!(DEFAULT_XATTRS[7].enabled);
    }

    #[test]
    fn evm_init_registers_lsm_and_configures_xattrs() {
        let _guard = TEST_LSM_LOCK.lock();
        reset_lsms();
        reset_for_test();

        init();

        let state = snapshot();
        assert!(state.initialized);
        assert_eq!(state.xattr_count, DEFAULT_XATTRS.len());
        assert_eq!(state.hmac_attrs, EVM_ATTR_FSUUID);
        assert_eq!(state.initialized_flags, 0);

        let mut ids = [0u64; 2];
        assert_eq!(lsm_active_ids(&mut ids), 1);
        assert_eq!(ids[0], LSM_ID_EVM);
    }

    #[test]
    fn evm_secfs_key_flags_match_linux_masks_without_hmac_key() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::keys::reset_for_test();
        crate::security::keys::init();
        reset_for_test();

        assert_eq!(readable_key_flags(), 0);
        assert_eq!(
            write_key_flags(0),
            Err(-crate::include::uapi::errno::EINVAL)
        );
        assert_eq!(
            write_key_flags(EVM_INIT_HMAC),
            Err(-crate::include::uapi::errno::ENOENT)
        );
        write_key_flags(EVM_INIT_X509 | EVM_SETUP_COMPLETE).expect("x509 setup");
        assert_eq!(readable_key_flags(), EVM_INIT_X509);
        assert_eq!(
            write_key_flags(EVM_ALLOW_METADATA_WRITES),
            Err(-crate::include::uapi::errno::EPERM)
        );
    }

    #[test]
    fn evm_hmac_write_loads_encrypted_evm_key_and_locks_setup() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::keys::reset_for_test();
        crate::security::keys::init();
        reset_for_test();

        let key_payload = [0x42u8; 32];
        let key = crate::security::keys::add_key("encrypted", EVM_KEY_DESCRIPTION, &key_payload);
        assert!(key > 0);

        write_key_flags(EVM_INIT_HMAC | EVM_ALLOW_METADATA_WRITES).expect("load hmac key");

        let state = snapshot();
        assert!(hmac_key_loaded());
        assert_eq!(hmac_key_len(), key_payload.len());
        assert_eq!(readable_key_flags(), EVM_INIT_HMAC);
        assert_eq!(state.initialized_flags, EVM_INIT_HMAC | EVM_SETUP_COMPLETE);
        assert_eq!(
            crate::security::keys::read(key).expect("burned payload"),
            alloc::vec![0u8; key_payload.len()]
        );
        assert_eq!(
            write_key_flags(EVM_INIT_X509),
            Err(-crate::include::uapi::errno::EPERM)
        );
    }

    #[test]
    fn evm_rejects_oversized_hmac_key_without_setting_flags() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::keys::reset_for_test();
        crate::security::keys::init();
        reset_for_test();

        let oversized = alloc::vec![0x7bu8; EVM_MAX_KEY_SIZE + 1];
        assert!(crate::security::keys::add_key("encrypted", EVM_KEY_DESCRIPTION, &oversized) > 0);
        assert_eq!(
            write_key_flags(EVM_INIT_HMAC),
            Err(-crate::include::uapi::errno::EINVAL)
        );
        assert!(!hmac_key_loaded());
        assert_eq!(hmac_key_len(), 0);
    }

    #[test]
    fn evm_hmac_sha1_matches_rfc2202_vector() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let digest = hmac_sha1(b"Jefe", b"what do ya want for nothing?");
        assert_eq!(
            digest,
            [
                0xef, 0xfc, 0xdf, 0x6a, 0xe5, 0xeb, 0x2f, 0xa2, 0xd2, 0x74, 0x16, 0xd5, 0xf1, 0x84,
                0xdf, 0x9c, 0x25, 0x9a, 0x7c, 0x79,
            ]
        );
    }

    #[test]
    fn evm_hmac_uses_enabled_xattrs_and_linux_inode_misc() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        set_hmac_key(&[0x11; 32]).expect("hmac key");

        let metadata = EvmMetadata {
            ino: 0xfeed_beef,
            generation: 7,
            uid: 1000,
            gid: 1001,
            mode: 0o100755,
            fsuuid: [0x44; 16],
        };
        let disabled_apparmor = EvmProtectedXattr {
            name: "security.apparmor",
            value: b"profile-a",
        };
        let capability = EvmProtectedXattr {
            name: "security.capability",
            value: b"cap-v3",
        };

        let digest = calc_hmac(metadata, &[disabled_apparmor, capability]).expect("digest");
        let same_without_disabled = calc_hmac(metadata, &[capability]).expect("same digest");
        assert_eq!(digest, same_without_disabled);

        let changed_capability = EvmProtectedXattr {
            name: "security.capability",
            value: b"cap-v4",
        };
        assert_ne!(
            digest,
            calc_hmac(metadata, &[changed_capability]).expect("changed xattr")
        );

        let mut changed_metadata = metadata;
        changed_metadata.mode = 0o100644;
        assert_ne!(
            digest,
            calc_hmac(changed_metadata, &[capability]).expect("changed metadata")
        );
    }

    #[test]
    fn evm_metadata_from_inode_carries_superblock_uuid() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        let inode = Inode::new(
            99,
            crate::fs::types::InodeKind::Regular,
            0o600,
            &crate::fs::ops::NOOP_INODE_OPS,
            &crate::fs::ops::NOOP_FILE_OPS,
            crate::fs::types::InodePrivate::None,
        );
        let sb = crate::fs::types::SuperBlock::alloc(
            "evmfs",
            0x4556_4d46,
            &crate::fs::ops::NOOP_SUPER_OPS,
        );
        let fsuuid = [0xa5; 16];
        sb.set_uuid(fsuuid);
        *inode.sb.lock() = Some(sb);

        let metadata = EvmMetadata::from_inode(&inode);
        assert_eq!(metadata.ino, 99);
        assert_eq!(metadata.fsuuid, fsuuid);
    }

    #[test]
    fn evm_hmac_xattr_layout_and_verifier_match_linux_type2() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        set_hmac_key(&[0x22; 32]).expect("hmac key");

        let metadata = EvmMetadata {
            ino: 42,
            generation: 0,
            uid: 0,
            gid: 0,
            mode: 0o100644,
            fsuuid: [0x55; 16],
        };
        let capability = EvmProtectedXattr {
            name: "security.capability",
            value: &[1, 2, 3, 4],
        };
        let evm = build_hmac_xattr(metadata, &[capability]).expect("evm hmac");

        assert_eq!(evm[0], EVM_XATTR_HMAC);
        assert_eq!(evm.len(), EVM_XATTR_HMAC_SIZE);
        assert_eq!(
            verify_hmac_xattr(metadata, Some(&evm), &[capability]),
            EvmIntegrityStatus::Pass
        );

        let mut tampered = evm;
        tampered[7] ^= 0x80;
        assert_eq!(
            verify_hmac_xattr(metadata, Some(&tampered), &[capability]),
            EvmIntegrityStatus::Fail
        );
        assert_eq!(
            verify_hmac_xattr(metadata, None, &[capability]),
            EvmIntegrityStatus::NoLabel
        );
        assert_eq!(
            verify_hmac_xattr(metadata, None, &[]),
            EvmIntegrityStatus::NoXattrs
        );
    }

    #[test]
    fn evm_hmac_requires_loaded_key_and_enabled_protected_xattr() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        let metadata = EvmMetadata {
            ino: 1,
            generation: 0,
            uid: 0,
            gid: 0,
            mode: 0o100644,
            fsuuid: [0; 16],
        };
        let capability = EvmProtectedXattr {
            name: "security.capability",
            value: b"cap",
        };
        assert_eq!(
            calc_hmac(metadata, &[capability]),
            Err(-crate::include::uapi::errno::ENOKEY)
        );

        set_hmac_key(&[0x33; 32]).expect("hmac key");
        let disabled_only = EvmProtectedXattr {
            name: "security.ima",
            value: b"digest",
        };
        assert_eq!(
            calc_hmac(metadata, &[disabled_only]),
            Err(-crate::include::uapi::errno::ENODATA)
        );
        assert!(protected_xattr_known("security.ima"));
        assert!(protected_xattr_known("ima"));
        assert!(!protected_xattr_enabled("security.ima"));
        assert!(protected_xattr_enabled("capability"));
        assert!(protected_xattr_enabled("security.capability"));
    }

    #[test]
    fn evm_revalidate_status_matches_linux_key_and_xattr_rules() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();

        assert!(!evm_revalidate_status(None));
        assert!(!evm_revalidate_status(Some("security.capability")));
        assert!(!evm_revalidate_status(Some(EVM_XATTR_NAME)));

        set_hmac_key(&[0x35; 32]).expect("hmac key");

        assert!(evm_revalidate_status(None));
        assert!(evm_revalidate_status(Some("security.capability")));
        assert!(evm_revalidate_status(Some("capability")));
        assert!(evm_revalidate_status(Some(POSIX_ACL_ACCESS_XATTR)));
        assert!(evm_revalidate_status(Some(POSIX_ACL_DEFAULT_XATTR)));
        assert!(evm_revalidate_status(Some(EVM_XATTR_NAME)));
        assert!(!evm_revalidate_status(Some("user.comment")));
    }

    #[test]
    fn evm_setxattr_protection_follows_linux_status_edges() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();

        assert_eq!(
            protect_setxattr(
                EVM_XATTR_NAME,
                &[],
                EvmIntegrityStatus::Pass,
                true,
                false,
                true
            ),
            Err(-crate::include::uapi::errno::EINVAL)
        );
        assert_eq!(
            protect_setxattr(
                EVM_XATTR_NAME,
                &[EVM_XATTR_HMAC],
                EvmIntegrityStatus::Pass,
                true,
                false,
                true,
            ),
            Err(-crate::include::uapi::errno::EPERM)
        );
        assert_eq!(
            protect_setxattr(
                EVM_XATTR_NAME,
                &[EVM_IMA_XATTR_DIGSIG, 2],
                EvmIntegrityStatus::Pass,
                false,
                false,
                true,
            ),
            Err(-crate::include::uapi::errno::EPERM)
        );
        assert_eq!(
            protect_setxattr(
                EVM_XATTR_NAME,
                &[EVM_IMA_XATTR_DIGSIG, 2],
                EvmIntegrityStatus::Pass,
                true,
                true,
                true,
            ),
            Err(-crate::include::uapi::errno::EPERM)
        );
        assert_eq!(
            protect_setxattr(
                EVM_XATTR_NAME,
                &[EVM_IMA_XATTR_DIGSIG, 2],
                EvmIntegrityStatus::Pass,
                true,
                false,
                true,
            ),
            Ok(())
        );

        assert_eq!(
            protect_setxattr(
                "user.comment",
                b"x",
                EvmIntegrityStatus::Fail,
                false,
                false,
                true,
            ),
            Ok(())
        );
        assert_eq!(
            protect_setxattr(
                "security.capability",
                b"cap",
                EvmIntegrityStatus::Pass,
                false,
                false,
                true,
            ),
            Ok(())
        );
        assert_eq!(
            protect_setxattr(
                "security.capability",
                b"cap",
                EvmIntegrityStatus::NoXattrs,
                false,
                false,
                true,
            ),
            Err(-crate::include::uapi::errno::EPERM)
        );
        assert_eq!(
            protect_setxattr(
                POSIX_ACL_ACCESS_XATTR,
                b"acl",
                EvmIntegrityStatus::NoXattrs,
                false,
                false,
                true,
            ),
            Ok(())
        );

        EVM_INITIALIZED_FLAGS.store(EVM_SETUP_COMPLETE, core::sync::atomic::Ordering::Release);
        assert_eq!(
            protect_setxattr(
                "security.capability",
                b"cap",
                EvmIntegrityStatus::NoLabel,
                false,
                false,
                true,
            ),
            Ok(())
        );
        assert_eq!(
            protect_setxattr(
                "security.capability",
                b"cap",
                EvmIntegrityStatus::Unknown,
                false,
                false,
                true,
            ),
            Ok(())
        );
        assert_eq!(
            protect_setxattr(
                "security.capability",
                b"cap",
                EvmIntegrityStatus::NoXattrs,
                false,
                false,
                true,
            ),
            Ok(())
        );

        reset_for_test();
        assert_eq!(
            protect_setxattr(
                "security.capability",
                b"cap",
                EvmIntegrityStatus::FailImmutable,
                false,
                false,
                true,
            ),
            Ok(())
        );
        assert_eq!(
            protect_setxattr(
                "security.capability",
                b"cap",
                EvmIntegrityStatus::PassImmutable,
                false,
                false,
                false,
            ),
            Ok(())
        );
        assert_eq!(
            protect_setxattr(
                "security.capability",
                b"cap",
                EvmIntegrityStatus::PassImmutable,
                false,
                false,
                true,
            ),
            Err(-crate::include::uapi::errno::EPERM)
        );
        assert_eq!(
            protect_removexattr(
                "security.capability",
                EvmIntegrityStatus::Pass,
                false,
                false,
            ),
            Ok(())
        );

        EVM_INITIALIZED_FLAGS.store(
            EVM_ALLOW_METADATA_WRITES,
            core::sync::atomic::Ordering::Release,
        );
        assert_eq!(
            protect_setxattr(
                EVM_XATTR_NAME,
                &[EVM_XATTR_HMAC],
                EvmIntegrityStatus::Fail,
                false,
                true,
                true,
            ),
            Ok(())
        );
    }

    #[test]
    fn evm_hmac_attrs_fsuuid_matches_linux_hmac_add_misc() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let mut metadata = EvmMetadata {
            ino: 9,
            generation: 3,
            uid: 0,
            gid: 0,
            mode: 0o100644,
            fsuuid: [0x11; 16],
        };

        let mut without_fsuuid = Vec::new();
        metadata.append_linux_hmac_misc_with_attrs(&mut without_fsuuid, EVM_XATTR_HMAC, 0);
        metadata.fsuuid = [0x22; 16];
        let mut still_without_fsuuid = Vec::new();
        metadata.append_linux_hmac_misc_with_attrs(&mut still_without_fsuuid, EVM_XATTR_HMAC, 0);
        assert_eq!(without_fsuuid, still_without_fsuuid);

        let mut with_fsuuid = Vec::new();
        metadata.append_linux_hmac_misc_with_attrs(
            &mut with_fsuuid,
            EVM_XATTR_HMAC,
            EVM_ATTR_FSUUID,
        );
        metadata.fsuuid = [0x33; 16];
        let mut changed_fsuuid = Vec::new();
        metadata.append_linux_hmac_misc_with_attrs(
            &mut changed_fsuuid,
            EVM_XATTR_HMAC,
            EVM_ATTR_FSUUID,
        );
        assert_ne!(with_fsuuid, changed_fsuuid);

        let mut portable = Vec::new();
        metadata.append_linux_hmac_misc_with_attrs(
            &mut portable,
            EVM_XATTR_PORTABLE_DIGSIG,
            EVM_ATTR_FSUUID,
        );
        assert_eq!(portable.len(), without_fsuuid.len());
    }

    fn signature_xattr(xattr_type: u8, version: u8, hash_algo: u8, payload: &[u8]) -> Vec<u8> {
        let mut xattr = Vec::new();
        xattr.push(xattr_type);
        xattr.push(version);
        xattr.push(hash_algo);
        xattr.extend_from_slice(&0x0102_0304u32.to_be_bytes());
        xattr.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        xattr.extend_from_slice(payload);
        xattr
    }

    const VENDOR_RSA_PUBLIC_KEY: &[u8] = &[
        0x30, 0x82, 0x01, 0x0a, 0x02, 0x82, 0x01, 0x01, 0x00, 0xd7, 0x1e, 0x77, 0x82, 0x8c, 0x92,
        0x31, 0xe7, 0x69, 0x02, 0xa2, 0xd5, 0x5c, 0x78, 0xde, 0xa2, 0x0c, 0x8f, 0xfe, 0x28, 0x59,
        0x31, 0xdf, 0x40, 0x9c, 0x60, 0x61, 0x06, 0xb9, 0x2f, 0x62, 0x40, 0x80, 0x76, 0xcb, 0x67,
        0x4a, 0xb5, 0x59, 0x56, 0x69, 0x17, 0x07, 0xfa, 0xf9, 0x4c, 0xbd, 0x6c, 0x37, 0x7a, 0x46,
        0x7d, 0x70, 0xa7, 0x67, 0x22, 0xb3, 0x4d, 0x7a, 0x94, 0xc3, 0xba, 0x4b, 0x7c, 0x4b, 0xa9,
        0x32, 0x7c, 0xb7, 0x38, 0x95, 0x45, 0x64, 0xa4, 0x05, 0xa8, 0x9f, 0x12, 0x7c, 0x4e, 0xc6,
        0xc8, 0x2d, 0x40, 0x06, 0x30, 0xf4, 0x60, 0xa6, 0x91, 0xbb, 0x9b, 0xca, 0x04, 0x79, 0x11,
        0x13, 0x75, 0xf0, 0xae, 0xd3, 0x51, 0x89, 0xc5, 0x74, 0xb9, 0xaa, 0x3f, 0xb6, 0x83, 0xe4,
        0x78, 0x6b, 0xcd, 0xf9, 0x5c, 0x4c, 0x85, 0xea, 0x52, 0x3b, 0x51, 0x93, 0xfc, 0x14, 0x6b,
        0x33, 0x5d, 0x30, 0x70, 0xfa, 0x50, 0x1b, 0x1b, 0x38, 0x81, 0x13, 0x8d, 0xf7, 0xa5, 0x0c,
        0xc0, 0x8e, 0xf9, 0x63, 0x52, 0x18, 0x4e, 0xa9, 0xf9, 0xf8, 0x5c, 0x5d, 0xcd, 0x7a, 0x0d,
        0xd4, 0x8e, 0x7b, 0xee, 0x91, 0x7b, 0xad, 0x7d, 0xb4, 0x92, 0xd5, 0xab, 0x16, 0x3b, 0x0a,
        0x8a, 0xce, 0x8e, 0xde, 0x47, 0x1a, 0x17, 0x01, 0x86, 0x7b, 0xab, 0x99, 0xf1, 0x4b, 0x0c,
        0x3a, 0x0d, 0x82, 0x47, 0xc1, 0x91, 0x8c, 0xbb, 0x2e, 0x22, 0x9e, 0x49, 0x63, 0x6e, 0x02,
        0xc1, 0xc9, 0x3a, 0x9b, 0xa5, 0x22, 0x1b, 0x07, 0x95, 0xd6, 0x10, 0x02, 0x50, 0xfd, 0xfd,
        0xd1, 0x9b, 0xbe, 0xab, 0xc2, 0xc0, 0x74, 0xd7, 0xec, 0x00, 0xfb, 0x11, 0x71, 0xcb, 0x7a,
        0xdc, 0x81, 0x79, 0x9f, 0x86, 0x68, 0x46, 0x63, 0x82, 0x4d, 0xb7, 0xf1, 0xe6, 0x16, 0x6f,
        0x42, 0x63, 0xf4, 0x94, 0xa0, 0xca, 0x33, 0xcc, 0x75, 0x13, 0x02, 0x03, 0x01, 0x00, 0x01,
    ];

    const VENDOR_RSA_SHA256_DIGEST: &[u8] = &[
        0x3e, 0xc8, 0xa1, 0x26, 0x20, 0x54, 0x44, 0x52, 0x48, 0x0d, 0xe5, 0x66, 0xf3, 0xb3, 0xf5,
        0x04, 0xbe, 0x10, 0xa8, 0x48, 0x94, 0x22, 0x2d, 0xdd, 0xba, 0x7a, 0xb4, 0x76, 0x8d, 0x79,
        0x98, 0x89,
    ];

    const VENDOR_RSA_SHA256_SIGNATURE: &[u8] = &[
        0xc7, 0xa3, 0x98, 0xeb, 0x43, 0xd1, 0x08, 0xc2, 0x3d, 0x78, 0x45, 0x04, 0x70, 0xc9, 0x01,
        0xee, 0xf8, 0x85, 0x37, 0x7c, 0x0b, 0xf9, 0x19, 0x70, 0x5c, 0x45, 0x7b, 0x2f, 0x3a, 0x0b,
        0xb7, 0x8b, 0xc4, 0x0d, 0x7b, 0x3a, 0x64, 0x0b, 0x0f, 0xdb, 0x78, 0xa9, 0x0b, 0xfd, 0x8d,
        0x82, 0xa4, 0x86, 0x39, 0xbf, 0x21, 0xb8, 0x84, 0xc4, 0xce, 0x9f, 0xc2, 0xe8, 0xb6, 0x61,
        0x46, 0x17, 0xb9, 0x4e, 0x0b, 0x57, 0x05, 0xb4, 0x4f, 0xf9, 0x9c, 0x93, 0x2d, 0x9b, 0xd5,
        0x48, 0x1d, 0x80, 0x12, 0xef, 0x3a, 0x77, 0x7f, 0xbc, 0xb5, 0x8e, 0x2b, 0x6b, 0x7c, 0xfc,
        0x9f, 0x8c, 0x9d, 0xa2, 0xc4, 0x85, 0xb0, 0x87, 0xe9, 0x17, 0x9b, 0xb6, 0x23, 0x62, 0xd2,
        0xa9, 0x9f, 0x57, 0xe8, 0xf7, 0x04, 0x45, 0x24, 0x3a, 0x45, 0xeb, 0xeb, 0x6a, 0x08, 0x8e,
        0xaf, 0xc8, 0xa0, 0x84, 0xbc, 0x5d, 0x13, 0x38, 0xf5, 0x17, 0x8c, 0xa3, 0x96, 0x9b, 0xa9,
        0x38, 0x8d, 0xf0, 0x35, 0xad, 0x32, 0x8a, 0x72, 0x5b, 0xdf, 0x21, 0xab, 0x4b, 0x0e, 0xa8,
        0x29, 0xbb, 0x61, 0x54, 0xbf, 0x05, 0xdb, 0x84, 0x84, 0xde, 0xdd, 0x16, 0x36, 0x31, 0xda,
        0xf3, 0x42, 0x6d, 0x7a, 0x90, 0x22, 0x9b, 0x11, 0x29, 0xa6, 0xf8, 0x30, 0x61, 0xda, 0xd3,
        0x8b, 0x54, 0x1e, 0x42, 0xd1, 0x47, 0x1d, 0x6f, 0xd1, 0xcd, 0x42, 0x0b, 0xd1, 0xe4, 0x15,
        0x85, 0x7e, 0x08, 0xd6, 0x59, 0x64, 0x4c, 0x01, 0x34, 0x91, 0x92, 0x26, 0xe8, 0xb0, 0x25,
        0x8c, 0xf8, 0xf4, 0xfa, 0x8b, 0xc9, 0x31, 0x33, 0x76, 0x72, 0xfb, 0x64, 0x92, 0x9f, 0xda,
        0x62, 0x8d, 0xe1, 0x2a, 0x71, 0x91, 0x43, 0x40, 0x61, 0x3c, 0x5a, 0xbe, 0x86, 0xfc, 0x5b,
        0xe6, 0xf9, 0xa9, 0x16, 0x31, 0x1f, 0xaf, 0x25, 0x6d, 0xc2, 0x4a, 0x23, 0x6e, 0x63, 0x02,
        0xa2,
    ];

    #[test]
    fn evm_post_metadata_update_recalculates_type2_hmac_when_revalidation_needed() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        set_hmac_key(&[0x66; 32]).expect("hmac key");
        set_hmac_attrs_for_test(EVM_ATTR_FSUUID);

        let metadata = EvmMetadata {
            ino: 108,
            generation: 2,
            uid: 0,
            gid: 0,
            mode: 0o100644,
            fsuuid: [0x77; 16],
        };
        let cap_v1 = EvmProtectedXattr {
            name: "security.capability",
            value: b"cap-v1",
        };
        let cap_v2 = EvmProtectedXattr {
            name: "security.capability",
            value: b"cap-v2",
        };
        let old_hmac = build_hmac_xattr(metadata, &[cap_v1]).expect("old hmac");

        let update = post_setxattr_update(metadata, "security.capability", &[cap_v2], false)
            .expect("setxattr update");
        let Some(EvmXattrUpdate::Set(new_hmac)) = update else {
            panic!("expected setxattr hmac update");
        };
        assert_ne!(old_hmac, new_hmac);
        assert_eq!(
            verify_hmac_xattr(metadata, Some(&new_hmac), &[cap_v2]),
            EvmIntegrityStatus::Pass
        );

        assert_eq!(
            post_setxattr_update(metadata, "user.comment", &[cap_v2], false)
                .expect("unrelated xattr"),
            None
        );
        assert_eq!(
            post_setxattr_update(metadata, EVM_XATTR_NAME, &[cap_v2], false).expect("evm xattr"),
            None
        );
        assert_eq!(
            post_setxattr_update(metadata, "security.capability", &[cap_v2], true)
                .expect("unsupported fs"),
            None
        );
        assert_eq!(
            post_removexattr_update(metadata, "security.capability", &[]).expect("remove xattr"),
            Some(EvmXattrUpdate::Remove)
        );

        let mut changed_metadata = metadata;
        changed_metadata.uid = 1000;
        let update =
            post_setattr_update(changed_metadata, &[cap_v2], false, true).expect("setattr update");
        let Some(EvmXattrUpdate::Set(changed_hmac)) = update else {
            panic!("expected setattr hmac update");
        };
        assert_ne!(new_hmac, changed_hmac);
        assert_eq!(
            post_setattr_update(changed_metadata, &[cap_v2], false, false)
                .expect("unchanged metadata"),
            None
        );
    }

    #[test]
    fn evm_signature_xattrs_follow_linux_status_edges_without_asymmetric_keys() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        let metadata = EvmMetadata {
            ino: 77,
            generation: 0,
            uid: 0,
            gid: 0,
            mode: 0o100644,
            fsuuid: [0; 16],
        };
        let capability = EvmProtectedXattr {
            name: "security.capability",
            value: b"cap",
        };

        assert_eq!(
            verify_hmac_xattr(
                metadata,
                Some(&[EVM_XATTR_PORTABLE_DIGSIG, 2]),
                &[capability],
            ),
            EvmIntegrityStatus::Fail
        );

        let portable = signature_xattr(
            EVM_XATTR_PORTABLE_DIGSIG,
            2,
            HASH_ALGO_SHA1,
            b"signature-payload",
        );
        assert_eq!(
            verify_hmac_xattr(metadata, Some(&portable), &[capability]),
            EvmIntegrityStatus::FailImmutable
        );

        let ima_digsig = signature_xattr(
            EVM_IMA_XATTR_DIGSIG,
            2,
            HASH_ALGO_SHA1,
            b"signature-payload",
        );
        assert_eq!(
            verify_hmac_xattr(metadata, Some(&ima_digsig), &[capability]),
            EvmIntegrityStatus::Fail
        );

        let mut bad_sig_size = portable.clone();
        bad_sig_size[7..9].copy_from_slice(&1u16.to_be_bytes());
        assert_eq!(
            verify_hmac_xattr(metadata, Some(&bad_sig_size), &[capability]),
            EvmIntegrityStatus::FailImmutable
        );

        let unsupported_hash = signature_xattr(
            EVM_XATTR_PORTABLE_DIGSIG,
            2,
            HASH_ALGO_LAST,
            b"signature-payload",
        );
        assert_eq!(
            verify_hmac_xattr(metadata, Some(&unsupported_hash), &[capability]),
            EvmIntegrityStatus::FailImmutable
        );

        write_key_flags(EVM_SIGV3_REQUIRED | EVM_SETUP_COMPLETE).expect("sigv3 required");
        assert_eq!(
            verify_hmac_xattr(metadata, Some(&portable), &[capability]),
            EvmIntegrityStatus::Fail
        );
    }

    #[test]
    fn evm_x509_mode_initializes_evm_keyring_and_accepts_vendor_linux_rsa_signature() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::keys::reset_for_test();
        crate::security::keys::init();
        reset_for_test();

        write_key_flags(EVM_INIT_X509 | EVM_SETUP_COMPLETE).expect("x509 setup");
        let keyring = evm_keyring_id().expect("evm keyring");
        assert!(
            crate::security::keys::add_key_to_keyring(
                "asymmetric",
                "id:01020304",
                VENDOR_RSA_PUBLIC_KEY,
                keyring,
            ) > 0
        );

        let evm_sig = signature_xattr(
            EVM_IMA_XATTR_DIGSIG,
            2,
            HASH_ALGO_SHA256,
            VENDOR_RSA_SHA256_SIGNATURE,
        );
        assert_eq!(
            verify_signature_digest(&evm_sig, VENDOR_RSA_SHA256_DIGEST, false),
            EvmIntegrityStatus::Pass
        );

        let portable_sig = signature_xattr(
            EVM_XATTR_PORTABLE_DIGSIG,
            2,
            HASH_ALGO_SHA256,
            VENDOR_RSA_SHA256_SIGNATURE,
        );
        assert_eq!(
            verify_signature_digest(&portable_sig, VENDOR_RSA_SHA256_DIGEST, true),
            EvmIntegrityStatus::PassImmutable
        );

        let mut tampered = evm_sig;
        let last = tampered.len() - 1;
        tampered[last] ^= 0x01;
        assert_eq!(
            verify_signature_digest(&tampered, VENDOR_RSA_SHA256_DIGEST, false),
            EvmIntegrityStatus::Fail
        );
    }

    #[test]
    fn evm_portable_signature_hash_includes_ima_and_omits_inode_identity() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        set_hmac_attrs_for_test(EVM_ATTR_FSUUID);
        let metadata_a = EvmMetadata {
            ino: 11,
            generation: 2,
            uid: 1000,
            gid: 1001,
            mode: 0o100644,
            fsuuid: [0x11; 16],
        };
        let metadata_b = EvmMetadata {
            ino: 99,
            generation: 77,
            uid: 1000,
            gid: 1001,
            mode: 0o100644,
            fsuuid: [0x22; 16],
        };
        let ima = EvmProtectedXattr {
            name: "security.ima",
            value: b"ima-digest",
        };
        let capability = EvmProtectedXattr {
            name: "security.capability",
            value: b"cap",
        };

        let portable_a = calc_hash(
            metadata_a,
            &[ima, capability],
            EVM_XATTR_PORTABLE_DIGSIG,
            HASH_ALGO_SHA256,
        )
        .expect("portable digest");
        let portable_b = calc_hash(
            metadata_b,
            &[ima, capability],
            EVM_XATTR_PORTABLE_DIGSIG,
            HASH_ALGO_SHA256,
        )
        .expect("portable digest");
        assert_eq!(portable_a, portable_b);
        assert_eq!(
            calc_hash(
                metadata_a,
                &[capability],
                EVM_XATTR_PORTABLE_DIGSIG,
                HASH_ALGO_SHA256,
            ),
            Err(-EPERM)
        );

        let local_a = calc_hash(
            metadata_a,
            &[ima, capability],
            EVM_IMA_XATTR_DIGSIG,
            HASH_ALGO_SHA256,
        )
        .expect("local digest");
        let local_b = calc_hash(
            metadata_b,
            &[ima, capability],
            EVM_IMA_XATTR_DIGSIG,
            HASH_ALGO_SHA256,
        )
        .expect("local digest");
        assert_ne!(local_a, local_b);
    }

    #[test]
    fn evm_acl_and_copy_up_hooks_follow_linux_evm_main_edges() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        assert_eq!(
            protect_set_acl(EvmIntegrityStatus::Pass, false, true),
            Ok(())
        );
        assert_eq!(
            protect_set_acl(EvmIntegrityStatus::NoXattrs, false, true),
            Ok(())
        );
        assert_eq!(
            protect_set_acl(EvmIntegrityStatus::FailImmutable, false, true),
            Ok(())
        );
        assert_eq!(
            protect_set_acl(EvmIntegrityStatus::PassImmutable, false, false),
            Ok(())
        );
        assert_eq!(
            protect_set_acl(EvmIntegrityStatus::PassImmutable, false, true),
            Err(-EPERM)
        );
        assert_eq!(
            protect_remove_acl(EvmIntegrityStatus::Fail, false),
            Err(-EPERM)
        );
        assert_eq!(
            protect_set_acl(EvmIntegrityStatus::Unknown, true, true),
            Ok(())
        );

        let portable = [EVM_XATTR_PORTABLE_DIGSIG, 2, HASH_ALGO_SHA256];
        let hmac = [EVM_XATTR_HMAC, 0xaa];
        assert_eq!(copy_up_xattr(EVM_XATTR_NAME, Some(&portable)), Ok(()));
        assert_eq!(copy_up_xattr(EVM_XATTR_NAME, Some(&hmac)), Err(-ECANCELED));
        assert_eq!(copy_up_xattr(EVM_XATTR_NAME, None), Err(-EPERM));
        assert_eq!(
            copy_up_xattr("security.capability", Some(&hmac)),
            Err(-EOPNOTSUPP)
        );
    }
}
