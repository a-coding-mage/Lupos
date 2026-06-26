//! linux-parity: complete
//! linux-source: vendor/linux/security/keys
//! test-origin: linux:vendor/linux/security/keys
//! Linux keyring (M64 â€” minimal subset).
//!
//! Mirrors `vendor/linux/security/keys/`.  Implements the in-kernel `Key`
//! struct, a global registry, Linux key type registration, and the four keyctl
//! operations `KEYCTL_GET_KEYRING_ID`, `KEYCTL_DESCRIBE`, `KEYCTL_REVOKE`,
//! `KEYCTL_READ`.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicI32, Ordering};

use spin::Mutex;

use crate::include::uapi::errno::{EACCES, EKEYREVOKED, ENODEV, ENOENT, ENOKEY, EOPNOTSUPP};

pub mod compat_dh;
pub mod encrypted_keys;
pub mod keyctl;
pub mod permission;
pub mod syscalls;
pub mod sysctl;

pub use syscalls::{sys_add_key, sys_keyctl, sys_request_key};

/// Special keyring IDs (Linux uapi/linux/keyctl.h).
pub const KEY_SPEC_THREAD_KEYRING: i32 = -1;
pub const KEY_SPEC_PROCESS_KEYRING: i32 = -2;
pub const KEY_SPEC_SESSION_KEYRING: i32 = -3;
pub const KEY_SPEC_USER_KEYRING: i32 = -4;
pub const KEY_SPEC_USER_SESSION_KEYRING: i32 = -5;

/// `keyctl()` subcommands (subset).
pub const KEYCTL_GET_KEYRING_ID: i32 = 0;
pub const KEYCTL_JOIN_SESSION_KEYRING: i32 = 1;
pub const KEYCTL_UPDATE: i32 = 2;
pub const KEYCTL_REVOKE: i32 = 3;
pub const KEYCTL_CHOWN: i32 = 4;
pub const KEYCTL_SETPERM: i32 = 5;
pub const KEYCTL_DESCRIBE: i32 = 6;
pub const KEYCTL_CLEAR: i32 = 7;
pub const KEYCTL_LINK: i32 = 8;
pub const KEYCTL_UNLINK: i32 = 9;
pub const KEYCTL_SEARCH: i32 = 10;
pub const KEYCTL_READ: i32 = 11;
pub const KEYCTL_SET_REQKEY_KEYRING: i32 = 14;
pub const KEYCTL_SESSION_TO_PARENT: i32 = 18;
pub const KEYCTL_INVALIDATE: i32 = 21;
pub const KEYCTL_GET_PERSISTENT: i32 = 22;
pub const KEYCTL_RESTRICT_KEYRING: i32 = 29;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyState {
    Live,
    Revoked,
}

pub struct Key {
    pub id: i32,
    pub key_type: String, // "user", "logon", "keyring"
    pub description: String,
    pub payload: Vec<u8>,
    pub links: Vec<i32>,
    pub uid: u32,
    pub gid: u32,
    pub perm: u32, // 0x3f010000 = LinusBilT/A all on possessor
    pub state: KeyState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyType {
    pub name: &'static str,
    pub readable: bool,
}

const BUILTIN_KEY_TYPES: &[KeyType] = &[
    // vendor/linux/security/keys/keyring.c
    KeyType {
        name: "keyring",
        readable: false,
    },
    // vendor/linux/security/keys/user_defined.c
    KeyType {
        name: "user",
        readable: true,
    },
    KeyType {
        name: "logon",
        readable: false,
    },
    KeyType {
        name: "big_key",
        readable: true,
    },
    // vendor/linux/crypto/asymmetric_keys/asymmetric_type.c
    KeyType {
        name: "asymmetric",
        readable: false,
    },
    // vendor/linux/fs/crypto/keyring.c
    KeyType {
        name: ".fscrypt",
        readable: false,
    },
    KeyType {
        name: "fscrypt-provisioning",
        readable: false,
    },
    // vendor/linux/security/keys/{encrypted-keys,trusted-keys}
    KeyType {
        name: "encrypted",
        readable: true,
    },
    KeyType {
        name: "trusted",
        readable: true,
    },
];

static NEXT_ID: AtomicI32 = AtomicI32::new(0x1000_0000);
pub static KEYRING: Mutex<Vec<Key>> = Mutex::new(Vec::new());
static KEY_TYPES: Mutex<Vec<KeyType>> = Mutex::new(Vec::new());

pub fn register_key_type(key_type: KeyType) -> Result<(), i32> {
    let mut types = KEY_TYPES.lock();
    if types.iter().any(|ty| ty.name == key_type.name) {
        return Ok(());
    }
    types.push(key_type);
    crate::kernel::printk::log_info!("", "Key type {} registered", key_type.name);
    Ok(())
}

pub fn registered_key_types() -> Vec<KeyType> {
    KEY_TYPES.lock().clone()
}

fn builtin_key_type(name: &str) -> Option<KeyType> {
    BUILTIN_KEY_TYPES.iter().copied().find(|ty| ty.name == name)
}

fn key_type_metadata(name: &str) -> Option<KeyType> {
    KEY_TYPES
        .lock()
        .iter()
        .copied()
        .find(|ty| ty.name == name)
        .or_else(|| builtin_key_type(name))
}

pub fn key_type_registered(name: &str) -> bool {
    key_type_metadata(name).is_some()
}

pub fn add_key(key_type: &str, description: &str, payload: &[u8]) -> i32 {
    if !key_type_registered(key_type) {
        return -ENODEV;
    }
    let id = NEXT_ID.fetch_add(1, Ordering::AcqRel);
    let mut g = KEYRING.lock();
    g.push(Key {
        id,
        key_type: String::from(key_type),
        description: String::from(description),
        payload: payload.to_vec(),
        links: Vec::new(),
        uid: 0,
        gid: 0,
        perm: 0x3f01_0000,
        state: KeyState::Live,
    });
    id
}

pub fn add_key_to_keyring(
    key_type: &str,
    description: &str,
    payload: &[u8],
    keyring_id: i32,
) -> i32 {
    let id = add_key(key_type, description, payload);
    if id <= 0 || keyring_id == 0 {
        return id;
    }
    match link_key_to_keyring(id, keyring_id) {
        Ok(()) => id,
        Err(err) => {
            remove_key_from_registry(id);
            err
        }
    }
}

pub fn add_key_to_keyring_from_user(
    key_type: &str,
    description: &str,
    payload: &[u8],
    keyring_id: i32,
) -> i32 {
    if key_type == "keyring" && is_restricted_integrity_keyring_description(description) {
        return -EACCES;
    }
    if keyring_id != 0 && is_restricted_integrity_keyring_id(keyring_id) {
        return -EACCES;
    }
    add_key_to_keyring(key_type, description, payload, keyring_id)
}

fn remove_key_from_registry(id: i32) {
    let mut g = KEYRING.lock();
    g.retain(|key| key.id != id);
    for key in g.iter_mut().filter(|key| key.key_type == "keyring") {
        key.links.retain(|linked| *linked != id);
    }
}

pub fn request_key(key_type: &str, description: &str) -> i32 {
    if !key_type_registered(key_type) {
        return -ENODEV;
    }
    let g = KEYRING.lock();
    for k in g.iter() {
        if k.key_type == key_type && k.description == description && k.state == KeyState::Live {
            return k.id;
        }
    }
    -ENOKEY
}

pub fn keyring_id_by_description(description: &str) -> Option<i32> {
    KEYRING.lock().iter().find_map(|key| {
        (key.key_type == "keyring" && key.description == description && key.state == KeyState::Live)
            .then_some(key.id)
    })
}

fn is_restricted_integrity_keyring_description(description: &str) -> bool {
    matches!(description, ".ima" | ".evm")
}

pub fn is_restricted_integrity_keyring_id(keyring_id: i32) -> bool {
    KEYRING.lock().iter().any(|key| {
        key.id == keyring_id
            && key.key_type == "keyring"
            && key.state == KeyState::Live
            && is_restricted_integrity_keyring_description(&key.description)
    })
}

pub fn link_key_to_keyring(key_id: i32, keyring_id: i32) -> Result<(), i32> {
    if keyring_id == 0 {
        return Ok(());
    }

    let mut g = KEYRING.lock();
    let source_ok = g
        .iter()
        .any(|key| key.id == key_id && key.state == KeyState::Live);
    if !source_ok {
        return Err(-ENOENT);
    }

    let Some(keyring) = g
        .iter_mut()
        .find(|key| key.id == keyring_id && key.key_type == "keyring")
    else {
        return Err(-ENOENT);
    };
    if keyring.state == KeyState::Revoked {
        return Err(-EKEYREVOKED);
    }
    if !keyring.links.contains(&key_id) {
        keyring.links.push(key_id);
    }
    Ok(())
}

pub fn link_key_to_keyring_from_user(key_id: i32, keyring_id: i32) -> Result<(), i32> {
    if keyring_id != 0 && is_restricted_integrity_keyring_id(keyring_id) {
        return Err(-EACCES);
    }
    link_key_to_keyring(key_id, keyring_id)
}

pub fn unlink_key_from_keyring(key_id: i32, keyring_id: i32) -> Result<(), i32> {
    if keyring_id == 0 {
        return Ok(());
    }
    let mut g = KEYRING.lock();
    let Some(keyring) = g
        .iter_mut()
        .find(|key| key.id == keyring_id && key.key_type == "keyring")
    else {
        return Err(-ENOENT);
    };
    if keyring.state == KeyState::Revoked {
        return Err(-EKEYREVOKED);
    }
    let before = keyring.links.len();
    keyring.links.retain(|linked| *linked != key_id);
    if keyring.links.len() == before {
        return Err(-ENOENT);
    }
    Ok(())
}

pub fn search_keyring_from_user(keyring_id: i32, key_type: &str, description: &str) -> i32 {
    if keyring_id == 0
        && key_type == "keyring"
        && is_restricted_integrity_keyring_description(description)
    {
        return -EACCES;
    }
    search_keyring(keyring_id, key_type, description)
}

pub fn search_keyring(keyring_id: i32, key_type: &str, description: &str) -> i32 {
    if !key_type_registered(key_type) {
        return -ENODEV;
    }
    if keyring_id == 0 {
        return request_key(key_type, description);
    }

    let g = KEYRING.lock();
    let Some(keyring) = g
        .iter()
        .find(|key| key.id == keyring_id && key.key_type == "keyring")
    else {
        return -ENOENT;
    };
    if keyring.state == KeyState::Revoked {
        return -EKEYREVOKED;
    }

    for linked in keyring.links.iter() {
        if let Some(key) = g.iter().find(|key| key.id == *linked)
            && key.key_type == key_type
            && key.description == description
            && key.state == KeyState::Live
        {
            return key.id;
        }
    }
    -ENOKEY
}

pub fn payloads_in_keyring_matching<F>(
    keyring_id: i32,
    key_type: &str,
    mut matches_key: F,
) -> Vec<Vec<u8>>
where
    F: FnMut(&Key) -> bool,
{
    let g = KEYRING.lock();
    let ids: Vec<i32> = if keyring_id == 0 {
        g.iter().map(|key| key.id).collect()
    } else {
        g.iter()
            .find(|key| {
                key.id == keyring_id && key.key_type == "keyring" && key.state == KeyState::Live
            })
            .map(|keyring| keyring.links.clone())
            .unwrap_or_default()
    };

    ids.iter()
        .filter_map(|id| g.iter().find(|key| key.id == *id))
        .filter(|key| key.key_type == key_type && key.state == KeyState::Live && matches_key(key))
        .map(|key| key.payload.clone())
        .collect()
}

pub fn describe(id: i32) -> Option<String> {
    let g = KEYRING.lock();
    for k in g.iter() {
        if k.id == id {
            return Some(format!(
                "{};{};{};{:08x};{}",
                k.key_type, k.uid, k.gid, k.perm, k.description
            ));
        }
    }
    None
}

pub fn revoke(id: i32) -> Result<(), i32> {
    let mut g = KEYRING.lock();
    for k in g.iter_mut() {
        if k.id == id {
            if k.state == KeyState::Revoked {
                return Err(-EKEYREVOKED);
            }
            k.state = KeyState::Revoked;
            return Ok(());
        }
    }
    Err(-ENOENT)
}

pub fn read(id: i32) -> Result<Vec<u8>, i32> {
    let g = KEYRING.lock();
    for k in g.iter() {
        if k.id == id {
            if k.state == KeyState::Revoked {
                return Err(-EKEYREVOKED);
            }
            if !key_type_metadata(&k.key_type)
                .map(|ty| ty.readable)
                .unwrap_or(false)
            {
                return Err(-EOPNOTSUPP);
            }
            return Ok(k.payload.clone());
        }
    }
    Err(-ENOENT)
}

pub fn update(id: i32, payload: &[u8]) -> Result<(), i32> {
    let mut g = KEYRING.lock();
    for k in g.iter_mut() {
        if k.id == id {
            if k.state == KeyState::Revoked {
                return Err(-EKEYREVOKED);
            }
            k.payload = payload.to_vec();
            return Ok(());
        }
    }
    Err(-ENOENT)
}

pub fn chown(id: i32, uid: u32, gid: u32) -> Result<(), i32> {
    let mut g = KEYRING.lock();
    for k in g.iter_mut() {
        if k.id == id {
            if k.state == KeyState::Revoked {
                return Err(-EKEYREVOKED);
            }
            if uid != u32::MAX {
                k.uid = uid;
            }
            if gid != u32::MAX {
                k.gid = gid;
            }
            return Ok(());
        }
    }
    Err(-ENOENT)
}

pub fn set_perm(id: i32, perm: u32) -> Result<(), i32> {
    let mut g = KEYRING.lock();
    for k in g.iter_mut() {
        if k.id == id {
            if k.state == KeyState::Revoked {
                return Err(-EKEYREVOKED);
            }
            k.perm = perm;
            return Ok(());
        }
    }
    Err(-ENOENT)
}

pub fn search(key_type: &str, description: &str) -> i32 {
    request_key(key_type, description)
}

pub fn clear() {
    let mut g = KEYRING.lock();
    for key in g.iter_mut() {
        if key.key_type == "keyring" {
            key.links.clear();
        }
    }
    g.clear();
}

pub fn key_exists(id: i32) -> bool {
    KEYRING.lock().iter().any(|k| k.id == id)
}

pub fn init() {
    for key_type in BUILTIN_KEY_TYPES {
        let _ = register_key_type(*key_type);
    }
    let _ = sysctl::init_security_keys_sysctls();
}

#[cfg(test)]
pub fn reset_for_test() {
    KEYRING.lock().clear();
    KEY_TYPES.lock().clear();
    NEXT_ID.store(0x1000_0000, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn add_then_request_lookup_round_trip() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        let id = add_key("user", "wolf", b"howl");
        assert!(id > 0);
        assert_eq!(request_key("user", "wolf"), id);
        assert_eq!(request_key("user", "absent"), -126);
        assert_eq!(add_key("unknown", "wolf", b"howl"), -ENODEV);
    }

    #[test]
    fn describe_format_matches_linux() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        let id = add_key("user", "wolf", b"howl");
        let d = describe(id).unwrap();
        // Format: type;uid;gid;perm-hex;description
        assert!(d.starts_with("user;0;0;3f010000;wolf"));
    }

    #[test]
    fn revoke_then_read_returns_ekeyrevoked() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        let id = add_key("user", "k", b"v");
        revoke(id).unwrap();
        assert_eq!(read(id), Err(-EKEYREVOKED));
        // Second revoke also returns EKEYREVOKED.
        assert_eq!(revoke(id), Err(-EKEYREVOKED));
    }

    #[test]
    fn update_search_and_clear_round_trip() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        let id = add_key("user", "k", b"old");
        update(id, b"new").unwrap();
        assert_eq!(read(id).unwrap(), b"new");
        assert_eq!(search("user", "k"), id);
        clear();
        assert_eq!(request_key("user", "k"), -ENOKEY);
    }

    #[test]
    fn keyring_links_scope_search_like_linux_keyrings() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        init();
        let ring = add_key("keyring", ".ima", &[]);
        let other_ring = add_key("keyring", ".platform", &[]);
        let ima_key = add_key_to_keyring("asymmetric", "id:01020304", b"ima", ring);
        let platform_key = add_key_to_keyring("asymmetric", "id:05060708", b"platform", other_ring);

        assert!(ima_key > 0);
        assert!(platform_key > 0);
        assert_eq!(keyring_id_by_description(".ima"), Some(ring));
        assert_eq!(search_keyring(ring, "asymmetric", "id:01020304"), ima_key);
        assert_eq!(search_keyring(ring, "asymmetric", "id:05060708"), -ENOKEY);
        assert_eq!(
            payloads_in_keyring_matching(ring, "asymmetric", |key| key
                .description
                .starts_with("id:")),
            vec![b"ima".to_vec()]
        );
        unlink_key_from_keyring(ima_key, ring).expect("unlink key");
        assert_eq!(search_keyring(ring, "asymmetric", "id:01020304"), -ENOKEY);
    }

    #[test]
    fn add_key_to_missing_keyring_does_not_leave_global_key() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        init();

        assert_eq!(
            add_key_to_keyring("user", "unlinked", b"payload", 0x1234),
            -ENOENT
        );
        assert_eq!(request_key("user", "unlinked"), -ENOKEY);
    }

    #[test]
    fn userspace_cannot_create_or_mutate_integrity_keyrings() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        init();

        assert_eq!(
            add_key_to_keyring_from_user("keyring", ".ima", &[], 0),
            -EACCES
        );

        let ima_ring = add_key("keyring", ".ima", &[]);
        let evm_ring = add_key("keyring", ".evm", &[]);
        let attacker_key = add_key("asymmetric", "id:01020304", b"attacker");

        assert_eq!(search_keyring_from_user(0, "keyring", ".ima"), -EACCES);
        assert_eq!(search_keyring_from_user(0, "keyring", ".evm"), -EACCES);
        assert_eq!(
            add_key_to_keyring_from_user("asymmetric", "id:01020304", b"attacker", ima_ring),
            -EACCES
        );
        assert_eq!(
            link_key_to_keyring_from_user(attacker_key, evm_ring),
            Err(-EACCES)
        );

        assert!(add_key_to_keyring("asymmetric", "id:01020304", b"trusted", ima_ring) > 0);
        assert_eq!(
            payloads_in_keyring_matching(ima_ring, "asymmetric", |_| true),
            vec![b"trusted".to_vec()]
        );
    }

    #[test]
    fn init_registers_linux_boot_key_types() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        init();
        let types = registered_key_types();
        for name in [
            "keyring",
            "user",
            "logon",
            "big_key",
            "asymmetric",
            ".fscrypt",
            "fscrypt-provisioning",
            "encrypted",
            "trusted",
        ] {
            assert!(
                types.iter().any(|ty| ty.name == name),
                "missing key type {name}"
            );
        }
    }

    #[test]
    fn logon_and_fscrypt_key_types_are_not_userspace_readable() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        let logon = add_key("logon", "svc:secret", b"hidden");
        assert!(logon > 0);
        assert_eq!(read(logon), Err(-EOPNOTSUPP));

        let fscrypt = add_key("fscrypt-provisioning", "raw:1", b"hidden");
        assert!(fscrypt > 0);
        assert_eq!(read(fscrypt), Err(-EOPNOTSUPP));
    }
}
