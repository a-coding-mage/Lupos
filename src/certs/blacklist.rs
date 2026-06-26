//! linux-parity: complete
//! linux-source: vendor/linux/certs/blacklist.c
//! test-origin: linux:vendor/linux/certs/blacklist.c
//! System blacklist key description validation, keyring, and hash queries.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use spin::Mutex;

use crate::include::uapi::errno::{
    EEXIST, EINVAL, EKEYREJECTED, ENOKEY, ENOPKG, EOPNOTSUPP, EPERM,
};
use crate::security::keys::{self, KeyType};

pub const MAX_HASH_LEN: usize = 128;
pub const TBS_PREFIX: &str = "tbs";
pub const BIN_PREFIX: &str = "bin";
pub const BLACKLIST_KEY_TYPE: &str = "blacklist";
pub const BLACKLIST_KEYRING_NAME: &str = ".blacklist";
pub const KEY_FLAG_BUILTIN: u32 = 6;
pub const KEY_ALLOC_NOT_IN_QUOTA: u32 = 0x0002;
pub const KEY_ALLOC_BUILT_IN: u32 = 0x0004;
pub const KEY_ALLOC_BYPASS_RESTRICTION: u32 = 0x0008;
pub const BLACKLIST_KEY_PERM: u32 = 0x0909_0000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlacklistHashType {
    X509Tbs,
    Binary,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlacklistState {
    pub keyring_id: i32,
    pub key_count: usize,
    pub revocation_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlacklistKeyInstantiation {
    pub perm: u32,
    pub alloc_flags: u32,
    pub builtin: bool,
}

static BLACKLIST_KEYRING_ID: Mutex<Option<i32>> = Mutex::new(None);
static REVOCATION_LIST: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());

pub fn blacklist_vet_description(desc: &str) -> Result<(), i32> {
    let (prefix, hash) = desc.split_once(':').ok_or(EINVAL)?;
    if prefix != TBS_PREFIX && prefix != BIN_PREFIX {
        return Err(EINVAL);
    }
    if hash.is_empty() || hash.len() > MAX_HASH_LEN || (hash.len() & 1) != 0 {
        return if hash.len() > MAX_HASH_LEN {
            Err(ENOPKG)
        } else {
            Err(EINVAL)
        };
    }
    for byte in hash.bytes() {
        if !byte.is_ascii_hexdigit() || byte.is_ascii_uppercase() {
            return Err(EINVAL);
        }
    }
    Ok(())
}

pub fn init() {
    let _ = init_with_hashes(&[]);
}

pub fn init_with_hashes(hashes: &[&str]) -> Result<BlacklistState, i32> {
    let _ = ensure_blacklist_keyring()?;
    for hash in hashes {
        let _ = mark_raw_hash_blacklisted(hash);
    }
    Ok(snapshot())
}

pub fn snapshot() -> BlacklistState {
    let keyring_id = BLACKLIST_KEYRING_ID.lock().unwrap_or(0);
    BlacklistState {
        keyring_id,
        key_count: blacklist_key_count(keyring_id),
        revocation_count: REVOCATION_LIST.lock().len(),
    }
}

pub fn blacklist_keyring_id() -> Option<i32> {
    *BLACKLIST_KEYRING_ID.lock()
}

pub fn blacklist_key_instantiate(
    builtin: bool,
    auth_update_enabled: bool,
) -> Result<BlacklistKeyInstantiation, i32> {
    if !builtin && !auth_update_enabled {
        return Err(EPERM);
    }
    Ok(BlacklistKeyInstantiation {
        perm: BLACKLIST_KEY_PERM,
        alloc_flags: KEY_ALLOC_NOT_IN_QUOTA | if builtin { KEY_ALLOC_BUILT_IN } else { 0 },
        builtin,
    })
}

pub fn blacklist_describe(desc: &str) -> &str {
    desc
}

pub fn raw_hash_description(hash: &[u8], hash_type: BlacklistHashType) -> String {
    let prefix = match hash_type {
        BlacklistHashType::X509Tbs => TBS_PREFIX,
        BlacklistHashType::Binary => BIN_PREFIX,
    };
    let mut out = String::with_capacity(prefix.len() + 1 + hash.len() * 2);
    out.push_str(prefix);
    out.push(':');
    for byte in hash {
        out.push(nibble_to_hex(byte >> 4));
        out.push(nibble_to_hex(byte & 0x0f));
    }
    out
}

pub fn mark_raw_hash_blacklisted(desc: &str) -> Result<(), i32> {
    blacklist_vet_description(desc)?;
    let keyring_id = ensure_blacklist_keyring()?;
    match keys::search_keyring(keyring_id, BLACKLIST_KEY_TYPE, desc) {
        found if found > 0 => return Err(EEXIST),
        missing if missing == -ENOKEY => {}
        err if err < 0 => return Err(errno_from_ret(err)),
        _ => {}
    }

    let instantiation = blacklist_key_instantiate(true, false)?;
    let key_id = keys::add_key_to_keyring(BLACKLIST_KEY_TYPE, desc, &[], keyring_id);
    if key_id < 0 {
        return Err(errno_from_ret(key_id));
    }
    keys::set_perm(key_id, instantiation.perm).map_err(errno_from_ret)
}

pub fn mark_hash_blacklisted(hash: &[u8], hash_type: BlacklistHashType) -> Result<(), i32> {
    let desc = raw_hash_description(hash, hash_type);
    mark_raw_hash_blacklisted(&desc)
}

pub fn is_hash_blacklisted(hash: &[u8], hash_type: BlacklistHashType) -> Result<(), i32> {
    let desc = raw_hash_description(hash, hash_type);
    let keyring_id = ensure_blacklist_keyring()?;
    match keys::search_keyring(keyring_id, BLACKLIST_KEY_TYPE, &desc) {
        found if found > 0 => Err(EKEYREJECTED),
        missing if missing == -ENOKEY => Ok(()),
        err if err < 0 => Err(errno_from_ret(err)),
        _ => Ok(()),
    }
}

pub fn is_binary_blacklisted(hash: &[u8]) -> Result<(), i32> {
    match is_hash_blacklisted(hash, BlacklistHashType::Binary) {
        Err(EKEYREJECTED) => Err(EPERM),
        result => result,
    }
}

pub const fn blacklist_update_errno() -> i32 {
    EPERM
}

pub const fn restrict_link_for_blacklist(is_blacklist_key_type: bool) -> Result<(), i32> {
    if is_blacklist_key_type {
        Ok(())
    } else {
        Err(EOPNOTSUPP)
    }
}

pub const fn binary_blacklisted_errno(hash_rejected: bool) -> Result<(), i32> {
    if hash_rejected { Err(EPERM) } else { Ok(()) }
}

pub const fn hash_blacklisted_errno(found: bool) -> Result<(), i32> {
    if found { Err(EKEYREJECTED) } else { Ok(()) }
}

pub fn add_key_to_revocation_list(data: &[u8]) -> Result<(), i32> {
    let _ = ensure_blacklist_keyring()?;
    let mut list = REVOCATION_LIST.lock();
    if !list.iter().any(|entry| entry.as_slice() == data) {
        list.push(data.to_vec());
    }
    Ok(())
}

pub fn revocation_list_contains(data: &[u8]) -> bool {
    REVOCATION_LIST
        .lock()
        .iter()
        .any(|entry| entry.as_slice() == data)
}

pub const fn key_on_revocation_list_errno(pkcs7_validate_trust_ret: i32) -> i32 {
    if pkcs7_validate_trust_ret == 0 {
        EKEYREJECTED
    } else {
        ENOKEY
    }
}

fn ensure_blacklist_keyring() -> Result<i32, i32> {
    keys::init();
    keys::register_key_type(KeyType {
        name: BLACKLIST_KEY_TYPE,
        readable: false,
    })
    .map_err(errno_from_ret)?;

    if let Some(id) = *BLACKLIST_KEYRING_ID.lock() {
        return Ok(id);
    }
    if let Some(id) = keys::keyring_id_by_description(BLACKLIST_KEYRING_NAME) {
        *BLACKLIST_KEYRING_ID.lock() = Some(id);
        return Ok(id);
    }

    let id = keys::add_key("keyring", BLACKLIST_KEYRING_NAME, &[]);
    if id < 0 {
        return Err(errno_from_ret(id));
    }
    *BLACKLIST_KEYRING_ID.lock() = Some(id);
    Ok(id)
}

fn blacklist_key_count(keyring_id: i32) -> usize {
    if keyring_id <= 0 {
        return 0;
    }
    keys::payloads_in_keyring_matching(keyring_id, BLACKLIST_KEY_TYPE, |_| true).len()
}

fn errno_from_ret(ret: i32) -> i32 {
    ret.checked_abs().unwrap_or(ret)
}

const fn nibble_to_hex(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        _ => (b'a' + (nibble - 10)) as char,
    }
}

#[cfg(test)]
pub fn reset_for_test() {
    *BLACKLIST_KEYRING_ID.lock() = None;
    REVOCATION_LIST.lock().clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn reset_all() {
        crate::security::keys::reset_for_test();
        crate::security::keys::init();
        reset_for_test();
    }

    #[test]
    fn blacklist_description_rules_match_linux_source() {
        let _lsm_guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/certs/blacklist.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/keys/system_keyring.h"
        ));
        assert!(source.contains("#define MAX_HASH_LEN\t128"));
        assert!(source.contains("static const char tbs_prefix[] = \"tbs\";"));
        assert!(source.contains("static const char bin_prefix[] = \"bin\";"));
        assert!(source.contains("blacklist_vet_description"));
        assert!(source.contains("!isxdigit(*desc) || isupper(*desc)"));
        assert!(source.contains("return -ENOPKG;"));
        assert!(source.contains("if (i == 0 || i & 1)"));
        assert!(source.contains("key->perm = BLACKLIST_KEY_PERM;"));
        assert!(source.contains("KEY_ALLOC_NOT_IN_QUOTA |"));
        assert!(source.contains("KEY_ALLOC_BUILT_IN"));
        assert!(source.contains("Duplicate blacklisted hash"));
        assert!(source.contains("keyring_search(make_key_ref(blacklist_keyring, true)"));
        assert!(source.contains("return -ENOKEY;"));
        assert!(source.contains("return -EPERM;"));
        assert!(source.contains("return -EOPNOTSUPP;"));
        assert!(source.contains("return -EKEYREJECTED;"));
        assert!(source.contains("return generic_key_instantiate(key, prep);"));
        assert!(source.contains("device_initcall(blacklist_init);"));
        assert!(header.contains("BLACKLIST_HASH_X509_TBS"));
        assert!(header.contains("BLACKLIST_HASH_BINARY"));

        assert_eq!(blacklist_vet_description("tbs:00ff"), Ok(()));
        assert_eq!(blacklist_vet_description("bin:abcdef"), Ok(()));
        assert_eq!(blacklist_vet_description("bad:00"), Err(EINVAL));
        assert_eq!(blacklist_vet_description("tbs:"), Err(EINVAL));
        assert_eq!(blacklist_vet_description("tbs:0"), Err(EINVAL));
        assert_eq!(blacklist_vet_description("tbs:AA"), Err(EINVAL));
        assert_eq!(
            blacklist_vet_description(&format!("bin:{}", "a".repeat(MAX_HASH_LEN + 1))),
            Err(ENOPKG)
        );
        assert_eq!(
            raw_hash_description(&[0x23, 0xaa, 0x04], BlacklistHashType::X509Tbs),
            "tbs:23aa04"
        );
        assert_eq!(
            raw_hash_description(&[0xff], BlacklistHashType::Binary),
            "bin:ff"
        );
        assert_eq!(blacklist_update_errno(), EPERM);
        assert_eq!(restrict_link_for_blacklist(false), Err(EOPNOTSUPP));
        assert_eq!(hash_blacklisted_errno(true), Err(EKEYREJECTED));
        assert_eq!(binary_blacklisted_errno(true), Err(EPERM));
    }

    #[test]
    fn blacklist_key_type_and_instantiation_match_linux_permissions() {
        let _lsm_guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        assert_eq!(BLACKLIST_KEY_PERM, 0x0909_0000);
        assert_eq!(KEY_FLAG_BUILTIN, 6);
        assert_eq!(KEY_ALLOC_NOT_IN_QUOTA, 0x0002);
        assert_eq!(KEY_ALLOC_BUILT_IN, 0x0004);
        assert_eq!(KEY_ALLOC_BYPASS_RESTRICTION, 0x0008);

        assert_eq!(
            blacklist_key_instantiate(true, false),
            Ok(BlacklistKeyInstantiation {
                perm: BLACKLIST_KEY_PERM,
                alloc_flags: KEY_ALLOC_NOT_IN_QUOTA | KEY_ALLOC_BUILT_IN,
                builtin: true,
            })
        );
        assert_eq!(blacklist_key_instantiate(false, false), Err(EPERM));
        assert_eq!(
            blacklist_key_instantiate(false, true),
            Ok(BlacklistKeyInstantiation {
                perm: BLACKLIST_KEY_PERM,
                alloc_flags: KEY_ALLOC_NOT_IN_QUOTA,
                builtin: false,
            })
        );
        assert_eq!(blacklist_update_errno(), EPERM);
        assert_eq!(blacklist_describe("bin:00"), "bin:00");
    }

    #[test]
    fn mark_and_query_blacklisted_hashes_use_linux_keyring_results() {
        let _lsm_guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        reset_all();

        let initial = init_with_hashes(&["tbs:00ff", "bin:a0"]).expect("blacklist init");
        assert!(initial.keyring_id > 0);
        assert_eq!(initial.key_count, 2);
        assert_eq!(
            crate::security::keys::describe(initial.keyring_id).as_deref(),
            Some("keyring;0;0;3f010000;.blacklist")
        );
        assert!(crate::security::keys::key_type_registered(
            BLACKLIST_KEY_TYPE
        ));

        assert_eq!(mark_raw_hash_blacklisted("tbs:00ff"), Err(EEXIST));
        assert_eq!(mark_raw_hash_blacklisted("tbs:0"), Err(EINVAL));
        assert_eq!(
            mark_hash_blacklisted(&[0x12, 0x34], BlacklistHashType::X509Tbs),
            Ok(())
        );
        assert_eq!(
            is_hash_blacklisted(&[0x12, 0x34], BlacklistHashType::X509Tbs),
            Err(EKEYREJECTED)
        );
        assert_eq!(
            is_hash_blacklisted(&[0x99], BlacklistHashType::X509Tbs),
            Ok(())
        );
        assert_eq!(is_binary_blacklisted(&[0xa0]), Err(EPERM));
        assert_eq!(is_binary_blacklisted(&[0xa1]), Ok(()));

        let blacklist_key = crate::security::keys::search_keyring(
            initial.keyring_id,
            BLACKLIST_KEY_TYPE,
            "tbs:1234",
        );
        assert!(blacklist_key > 0);
        assert!(
            crate::security::keys::describe(blacklist_key)
                .expect("blacklist key")
                .contains(";09090000;tbs:1234")
        );
        assert_eq!(snapshot().key_count, 3);
    }

    #[test]
    fn revocation_list_helpers_follow_blacklist_source_edges() {
        let _lsm_guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        reset_all();

        assert_eq!(add_key_to_revocation_list(b"cert-a"), Ok(()));
        assert_eq!(add_key_to_revocation_list(b"cert-a"), Ok(()));
        assert!(revocation_list_contains(b"cert-a"));
        assert!(!revocation_list_contains(b"cert-b"));
        assert_eq!(snapshot().revocation_count, 1);
        assert_eq!(key_on_revocation_list_errno(0), EKEYREJECTED);
        assert_eq!(key_on_revocation_list_errno(-ENOKEY), ENOKEY);
    }
}
