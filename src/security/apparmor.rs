//! linux-parity: partial
//! linux-source: vendor/linux/security/apparmor/lsm.c
//! test-origin: linux:vendor/linux/security/apparmor/lsm.c
//! AppArmor LSM initialization spine.
//!
//! This wires the AppArmor LSM identity, init state, and policy-hash boot
//! status into Lupos. The apparmorfs policy store accepts text profiles plus a
//! bounded Linux aa_ext binary stream with `policy_unpack.c`/`match.c`-shaped DFA
//! tables, namespace selection, profile replacement, and cred-style task labels.
//!
//! Refs: `vendor/linux/security/apparmor/{lsm.c,policy_unpack.c,match.c,label.c,policy_ns.c,task.c}`.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::{
    EACCES, EBADMSG, EINVAL, ENOENT, EOVERFLOW, EPROTO, EPROTONOSUPPORT,
};
use crate::include::uapi::fcntl::{O_ACCMODE, O_APPEND, O_RDWR, O_TRUNC, O_WRONLY};
use crate::kernel::capability::{CAP_MAC_ADMIN, capable};
use crate::security::hooks::{LSM_ID_APPARMOR, LsmHooks, NOOP_HOOKS};
use crate::security::lsm_list::register_lsm;

#[path = "apparmor/crypto.rs"]
pub mod crypto;

pub const POLICY_HASH_ALGORITHM: &str = "sha256";
pub const AA_POLICY_ABI_MIN: u32 = 5;
pub const AA_POLICY_ABI_MAX: u32 = 9;
pub const AA_CLASS_FILE: u8 = 2;
pub const AA_MAY_EXEC: u32 = 0x0000_0001;
pub const AA_MAY_WRITE: u32 = 0x0000_0002;
pub const AA_MAY_READ: u32 = 0x0000_0004;
pub const AA_MAY_APPEND: u32 = 0x0000_0008;
pub const AA_MAY_OPEN: u32 = 0x0000_0040;
pub const AA_EXT_U32: u8 = 2;
pub const AA_EXT_NAME: u8 = 4;
pub const AA_EXT_STRING: u8 = 5;
pub const AA_EXT_BLOB: u8 = 6;
pub const AA_EXT_STRUCT: u8 = 7;
pub const AA_EXT_STRUCTEND: u8 = 8;
pub const AA_EXT_ARRAY: u8 = 11;
pub const AA_EXT_ARRAYEND: u8 = 12;

pub const DFA_NOMATCH: u32 = 0;
pub const DFA_START: u32 = 1;
const YYTH_MAGIC: u32 = 0x1B5E_783D;
const YYTH_FLAG_DIFF_ENCODE: u16 = 0x0001;
const YYTH_FLAG_OOB_TRANS: u16 = 0x0002;
const YYTH_FLAGS: u16 = YYTH_FLAG_DIFF_ENCODE | YYTH_FLAG_OOB_TRANS;
const YYTD_ID_ACCEPT: u16 = 0;
const YYTD_ID_BASE: u16 = 1;
const YYTD_ID_CHK: u16 = 2;
const YYTD_ID_DEF: u16 = 3;
const YYTD_ID_EC: u16 = 4;
const YYTD_ID_ACCEPT2: u16 = 6;
const YYTD_ID_NXT: u16 = 7;
const YYTD_DATA8: u16 = 1;
const YYTD_DATA16: u16 = 2;
const YYTD_DATA32: u16 = 4;
const MATCH_FLAG_DIFF_ENCODE: u32 = 0x8000_0000;
const MATCH_FLAG_OOB_TRANSITION: u32 = 0x2000_0000;
const MATCH_FLAGS_MASK: u32 = 0xff00_0000;
const MATCH_FLAGS_VALID: u32 = MATCH_FLAG_DIFF_ENCODE | MATCH_FLAG_OOB_TRANSITION;
const MATCH_FLAGS_INVALID: u32 = MATCH_FLAGS_MASK & !MATCH_FLAGS_VALID;
const AA_CLASS_LAST: usize = 32;
const ROOT_NAMESPACE: &str = "root";

static INITIALIZED: AtomicBool = AtomicBool::new(false);
static POLICY_HASHING_ENABLED: AtomicBool = AtomicBool::new(false);
static POLICY_REVISION: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

lazy_static! {
    static ref PROFILES: Mutex<Vec<AppArmorProfile>> = Mutex::new(Vec::new());
    static ref NAMESPACES: Mutex<Vec<AppArmorNamespace>> = Mutex::new(Vec::new());
    static ref CURRENT_LABEL: Mutex<Option<AppArmorLabel>> = Mutex::new(None);
    static ref TASK_LABELS: Mutex<Vec<TaskLabel>> = Mutex::new(Vec::new());
    static ref PENDING_EXEC_LABELS: Mutex<Vec<TaskLabel>> = Mutex::new(Vec::new());
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileMode {
    Enforce,
    Complain,
}

impl ProfileMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Enforce => "enforce",
            Self::Complain => "complain",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppArmorProfile {
    pub namespace: String,
    pub name: String,
    pub revision: u64,
    pub mode: ProfileMode,
    pub attach: Option<String>,
    pub rules: Vec<AppArmorRule>,
    pub file: Option<AppArmorPolicyDb>,
    pub data: Vec<AppArmorDataBlock>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppArmorRule {
    pub path: String,
    pub allow: u32,
    pub deny: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppArmorDataBlock {
    pub key: String,
    pub value: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppArmorNamespace {
    pub name: String,
    pub revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppArmorLabel {
    pub generation: u64,
    pub components: Vec<AppArmorLabelComponent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppArmorLabelComponent {
    pub namespace: String,
    pub profile: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppArmorPolicyDb {
    pub dfa: AppArmorDfa,
    pub perms: Vec<AppArmorPerms>,
    pub start: [u32; AA_CLASS_LAST + 1],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppArmorPerms {
    pub allow: u32,
    pub deny: u32,
    pub subtree: u32,
    pub cond: u32,
    pub kill: u32,
    pub complain: u32,
    pub prompt: u32,
    pub audit: u32,
    pub quiet: u32,
    pub hide: u32,
    pub xindex: u32,
    pub tag: u32,
    pub label: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppArmorDfa {
    flags: u16,
    max_oob: u32,
    accept: Vec<u32>,
    accept2: Vec<u32>,
    default: Vec<u32>,
    base: Vec<u32>,
    next: Vec<u32>,
    check: Vec<u32>,
    equiv: Option<Vec<u8>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppArmorQueryPerms {
    pub allow: u32,
    pub deny: u32,
    pub audit: u32,
    pub quiet: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TaskLabel {
    task_id: u32,
    label: AppArmorLabel,
}

fn apparmor_task_alloc(task_id: u32, _clone_flags: u64) -> i32 {
    if let Some(label) = current_label() {
        set_task_label(task_id, Some(label));
    }
    0
}

fn apparmor_task_free(task_id: u32) {
    set_task_profile(task_id, None);
    set_pending_exec_profile(task_id, None);
}

fn apparmor_cred_alloc_blank() -> i32 {
    0
}

fn apparmor_cred_prepare() -> i32 {
    0
}

fn apparmor_cred_transfer() {}

fn apparmor_bprm_creds_for_exec(filename: &[u8]) -> i32 {
    let err = mediate_current_path(filename, AA_MAY_EXEC);
    if err != 0 {
        return err;
    }
    if let Some(task_id) = current_task_id() {
        set_pending_exec_profile(task_id, attach_profile_for_path(filename));
    }
    0
}

fn apparmor_bprm_committing_creds(_filename: &[u8]) {}

fn apparmor_bprm_committed_creds(_filename: &[u8]) {
    if let Some(task_id) = current_task_id()
        && let Some(label) = take_pending_exec_profile(task_id)
    {
        set_task_label(task_id, Some(label));
    }
}

fn apparmor_capable(_cap: u32) -> i32 {
    0
}

fn apparmor_path_open(path: &[u8], flags: i32) -> i32 {
    mediate_current_path(path, request_mask_from_open_flags(flags))
}

fn apparmor_file_permission(_fd: i32, _mask: u32) -> i32 {
    0
}

fn apparmor_socket_create(_family: i32, _kind: i32, _proto: i32) -> i32 {
    0
}

pub const HOOKS: LsmHooks = LsmHooks {
    name: "apparmor",
    id: LSM_ID_APPARMOR,
    task_alloc: Some(apparmor_task_alloc),
    task_free: Some(apparmor_task_free),
    bprm_creds_for_exec: Some(apparmor_bprm_creds_for_exec),
    bprm_committing_creds: Some(apparmor_bprm_committing_creds),
    bprm_committed_creds: Some(apparmor_bprm_committed_creds),
    cred_alloc_blank: Some(apparmor_cred_alloc_blank),
    cred_prepare: Some(apparmor_cred_prepare),
    cred_transfer: Some(apparmor_cred_transfer),
    capable: Some(apparmor_capable),
    path_open: Some(apparmor_path_open),
    file_permission: Some(apparmor_file_permission),
    socket_create: Some(apparmor_socket_create),
    ..NOOP_HOOKS
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppArmorState {
    pub initialized: bool,
    pub policy_hashing_enabled: bool,
    pub policy_hash_algorithm: &'static str,
    pub policy_revision: u64,
    pub profile_count: usize,
    pub namespace_count: usize,
}

pub fn init() {
    let _ = register_lsm(HOOKS);

    if INITIALIZED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    crate::kernel::printk::log_info!("AppArmor", "AppArmor initialized");
    POLICY_HASHING_ENABLED.store(true, Ordering::Release);
    crate::kernel::printk::log_info!(
        "AppArmor",
        "AppArmor {} policy hashing enabled",
        POLICY_HASH_ALGORITHM
    );
    crate::security::apparmorfs::init_securityfs();
}

pub fn snapshot() -> AppArmorState {
    AppArmorState {
        initialized: INITIALIZED.load(Ordering::Acquire),
        policy_hashing_enabled: POLICY_HASHING_ENABLED.load(Ordering::Acquire),
        policy_hash_algorithm: POLICY_HASH_ALGORITHM,
        policy_revision: policy_revision(),
        profile_count: PROFILES.lock().len(),
        namespace_count: NAMESPACES.lock().len(),
    }
}

pub fn policy_revision() -> u64 {
    POLICY_REVISION.load(Ordering::Acquire)
}

pub fn profiles_text() -> String {
    let profiles = PROFILES.lock();
    let mut out = String::new();
    for profile in profiles.iter() {
        out.push_str(&profile.fqname());
        out.push_str(" (");
        out.push_str(profile.mode.as_str());
        out.push_str(")\n");
    }
    out
}

fn ensure_policy_admin() -> Result<(), i32> {
    if capable(CAP_MAC_ADMIN) {
        Ok(())
    } else {
        Err(-crate::include::uapi::errno::EPERM)
    }
}

pub fn load_policy_blob(bytes: &[u8]) -> Result<usize, i32> {
    ensure_policy_admin()?;
    let profiles = parse_policy_blob(bytes)?;
    commit_profiles(profiles)?;
    Ok(bytes.len())
}

pub fn replace_policy_blob(bytes: &[u8]) -> Result<usize, i32> {
    ensure_policy_admin()?;
    let profiles = parse_policy_blob(bytes)?;
    commit_profiles(profiles)?;
    Ok(bytes.len())
}

pub fn remove_policy_blob(bytes: &[u8]) -> Result<usize, i32> {
    ensure_policy_admin()?;
    let text = core::str::from_utf8(bytes).map_err(|_| -EINVAL)?;
    let (namespace, name) = remove_profile_target(text).ok_or(-EINVAL)?;
    let mut profiles = PROFILES.lock();
    let before = profiles.len();
    if let Some(name) = name {
        profiles.retain(|profile| !(profile.namespace == namespace && profile.name == name));
    } else {
        profiles.retain(|profile| profile.namespace != namespace);
    }
    if profiles.len() == before {
        return Err(-ENOENT);
    }
    let revision = POLICY_REVISION.fetch_add(1, Ordering::AcqRel) + 1;
    bump_namespace_revision(&namespace, revision);
    refresh_all_labels_after_policy_change();
    Ok(bytes.len())
}

fn commit_profiles(mut profiles_to_commit: Vec<AppArmorProfile>) -> Result<(), i32> {
    if profiles_to_commit.is_empty() {
        return Err(-EINVAL);
    }
    let namespace = profiles_to_commit[0].namespace.clone();
    if profiles_to_commit
        .iter()
        .any(|profile| profile.namespace != namespace)
    {
        return Err(-EACCES);
    }
    let revision = POLICY_REVISION.fetch_add(1, Ordering::AcqRel) + 1;
    let mut profiles = PROFILES.lock();
    for profile in profiles_to_commit.iter_mut() {
        profile.revision = revision;
        if let Some(existing) = profiles.iter_mut().find(|existing| {
            existing.namespace == profile.namespace && existing.name == profile.name
        }) {
            *existing = profile.clone();
        } else {
            profiles.push(profile.clone());
        }
    }
    drop(profiles);
    bump_namespace_revision(&namespace, revision);
    refresh_all_labels_after_policy_change();
    Ok(())
}

fn parse_policy_blob(bytes: &[u8]) -> Result<Vec<AppArmorProfile>, i32> {
    if matches!(
        bytes.first().copied(),
        Some(AA_EXT_NAME | AA_EXT_U32 | AA_EXT_STRUCT)
    ) {
        parse_binary_profiles(bytes)
    } else {
        let mut profiles = Vec::new();
        profiles.push(parse_text_profile(bytes)?);
        Ok(profiles)
    }
}

fn parse_text_profile(bytes: &[u8]) -> Result<AppArmorProfile, i32> {
    let text = core::str::from_utf8(bytes).map_err(|_| -EINVAL)?;
    let raw_name = profile_name(text).ok_or(-EINVAL)?;
    let (namespace, name) = split_profile_fqname(&raw_name);
    let mode = if text.contains("complain") {
        ProfileMode::Complain
    } else {
        ProfileMode::Enforce
    };
    let attach = profile_attach(text);
    let rules = parse_text_rules(text)?;
    Ok(AppArmorProfile {
        namespace,
        name,
        revision: 0,
        mode,
        attach,
        rules,
        file: None,
        data: Vec::new(),
    })
}

fn profile_name(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut offset = find_word(text, "profile")?;
    offset += "profile".len();
    while bytes
        .get(offset)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        offset += 1;
    }
    read_profile_token(text, offset)
}

fn remove_profile_target(text: &str) -> Option<(String, Option<String>)> {
    let trimmed = text.trim();
    let raw = if let Some(name) = trimmed.strip_prefix("profile ") {
        read_profile_token(name, 0)
    } else {
        read_profile_token(trimmed, 0)
    }?;
    let (namespace, name) = split_profile_fqname(&raw);
    if raw.starts_with(':') && name.is_empty() {
        Some((namespace, None))
    } else {
        Some((namespace, Some(name)))
    }
}

fn split_profile_fqname(raw: &str) -> (String, String) {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix(':') {
        if let Some(split) = rest.find(':') {
            let namespace = rest[..split].trim();
            let mut profile = rest[split + 1..].trim_start();
            if let Some(after) = profile.strip_prefix("//") {
                profile = after;
            }
            return (namespace_or_root(namespace), String::from(profile));
        }
        return (namespace_or_root(rest.trim()), String::new());
    }
    if let Some(split) = trimmed.find("://") {
        let namespace = trimmed[..split].trim();
        let profile = trimmed[split + 3..].trim_start();
        return (namespace_or_root(namespace), String::from(profile));
    }
    (String::from(ROOT_NAMESPACE), String::from(trimmed))
}

fn namespace_or_root(namespace: &str) -> String {
    if namespace.is_empty() {
        String::from(ROOT_NAMESPACE)
    } else {
        String::from(namespace)
    }
}

impl AppArmorProfile {
    fn fqname(&self) -> String {
        if self.namespace == ROOT_NAMESPACE {
            self.name.clone()
        } else {
            alloc::format!(":{}:{}", self.namespace, self.name)
        }
    }
}

fn profile_attach(text: &str) -> Option<String> {
    let flags = text.find("flags=").unwrap_or(text.len());
    let brace = text.find('{').unwrap_or(text.len());
    let end = flags.min(brace);
    let header = &text[..end];
    let profile = profile_name(header)?;
    if profile.starts_with('/') {
        Some(profile)
    } else {
        None
    }
}

fn find_word(text: &str, word: &str) -> Option<usize> {
    let mut start = 0;
    while let Some(relative) = text[start..].find(word) {
        let idx = start + relative;
        let before_ok = idx == 0
            || text.as_bytes()[idx - 1].is_ascii_whitespace()
            || text.as_bytes()[idx - 1] == b';';
        let after = idx + word.len();
        let after_ok = after == text.len()
            || text.as_bytes()[after].is_ascii_whitespace()
            || text.as_bytes()[after] == b'"';
        if before_ok && after_ok {
            return Some(idx);
        }
        start = after;
    }
    None
}

fn read_profile_token(text: &str, mut offset: usize) -> Option<String> {
    let bytes = text.as_bytes();
    while bytes
        .get(offset)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        offset += 1;
    }
    if bytes.get(offset) == Some(&b'"') {
        let start = offset + 1;
        let end = text[start..].find('"')? + start;
        return Some(String::from(&text[start..end]));
    }
    let start = offset;
    while let Some(byte) = bytes.get(offset) {
        if byte.is_ascii_whitespace() || matches!(*byte, b'{' | b',' | b'\0') {
            break;
        }
        offset += 1;
    }
    if offset == start {
        None
    } else {
        Some(String::from(&text[start..offset]))
    }
}

fn parse_text_rules(text: &str) -> Result<Vec<AppArmorRule>, i32> {
    let Some(open) = text.find('{') else {
        return Ok(Vec::new());
    };
    let close = text.rfind('}').unwrap_or(text.len());
    if close <= open {
        return Ok(Vec::new());
    }

    let mut rules = Vec::new();
    for raw_line in text[open + 1..close].lines() {
        let mut line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line == "file," {
            continue;
        }
        let deny = if let Some(rest) = line.strip_prefix("deny ") {
            line = rest.trim_start();
            true
        } else if let Some(rest) = line.strip_prefix("allow ") {
            line = rest.trim_start();
            false
        } else {
            false
        };

        let Some((path, rest)) = split_rule_path(line) else {
            continue;
        };
        let perms = parse_perm_token(rest.trim_start_matches(|ch: char| ch.is_ascii_whitespace()))?;
        if perms == 0 {
            continue;
        }
        rules.push(AppArmorRule {
            path,
            allow: if deny { 0 } else { perms },
            deny: if deny { perms } else { 0 },
        });
    }
    Ok(rules)
}

fn parse_binary_profiles(bytes: &[u8]) -> Result<Vec<AppArmorProfile>, i32> {
    let mut ext = AaExt::new(bytes);
    let mut profiles = Vec::new();
    let mut namespace: Option<String> = None;
    while ext.pos < bytes.len() {
        let required_header = ext.pos == 0;
        verify_binary_header(&mut ext, required_header, &mut namespace)?;
        if ext.pos >= bytes.len() {
            break;
        }
        profiles.push(parse_binary_profile(&mut ext, namespace.as_deref())?);
    }
    if profiles.is_empty() {
        return Err(-EINVAL);
    }
    Ok(profiles)
}

fn verify_binary_header(
    ext: &mut AaExt<'_>,
    required: bool,
    namespace: &mut Option<String>,
) -> Result<(), i32> {
    let pos = ext.pos;
    let version = match ext.unpack_u32(Some("version")) {
        Ok(version) => version,
        Err(err) if !required => {
            ext.pos = pos;
            return if err == -EBADMSG { Ok(()) } else { Err(err) };
        }
        Err(_) => return Err(-EPROTONOSUPPORT),
    };
    if !(AA_POLICY_ABI_MIN..=AA_POLICY_ABI_MAX).contains(&(version & 0x3ff)) {
        return Err(-EPROTONOSUPPORT);
    }

    let pos = ext.pos;
    match ext.unpack_string(Some("namespace")) {
        Ok(ns) if ns.is_empty() => return Err(-EINVAL),
        Ok(ns) => match namespace {
            Some(existing) if existing != &ns => return Err(-EACCES),
            Some(_) => {}
            None => *namespace = Some(ns),
        },
        Err(_) => ext.pos = pos,
    }
    Ok(())
}

fn parse_binary_profile(
    ext: &mut AaExt<'_>,
    header_namespace: Option<&str>,
) -> Result<AppArmorProfile, i32> {
    ext.unpack_struct_start(Some("profile"))?;
    let raw_name = ext.unpack_string(Some("name"))?;
    let (mut namespace, name) = split_profile_fqname(&raw_name);
    if let Some(header_namespace) = header_namespace {
        if namespace != ROOT_NAMESPACE && namespace != header_namespace {
            return Err(-EACCES);
        }
        namespace = String::from(header_namespace);
    }
    let mode = match ext.unpack_u32(Some("mode")).unwrap_or(0) {
        0 => ProfileMode::Enforce,
        1 => ProfileMode::Complain,
        _ => return Err(-EINVAL),
    };
    let attach = ext.unpack_string(Some("attach")).ok();
    let rules = if let Ok(rule_count) = ext.unpack_array(Some("rules")) {
        let mut rules = Vec::new();
        for _ in 0..rule_count {
            ext.unpack_struct_start(None)?;
            let path = ext.unpack_string(Some("path"))?;
            let allow = ext.unpack_u32(Some("allow")).unwrap_or(0);
            let deny = ext.unpack_u32(Some("deny")).unwrap_or(0);
            if allow & deny != 0 {
                return Err(-EINVAL);
            }
            ext.unpack_struct_end()?;
            rules.push(AppArmorRule { path, allow, deny });
        }
        let _ = ext.unpack_array_end();
        rules
    } else {
        Vec::new()
    };
    let file = parse_binary_profile_file(ext)?;
    let data = parse_binary_profile_data(ext)?;
    ext.unpack_struct_end()?;
    Ok(AppArmorProfile {
        namespace,
        name,
        revision: 0,
        mode,
        attach,
        rules,
        file,
        data,
    })
}

fn parse_binary_profile_file(ext: &mut AaExt<'_>) -> Result<Option<AppArmorPolicyDb>, i32> {
    let pos = ext.pos;
    let wrapped = ext.unpack_struct_start(Some("file")).is_ok();
    if !wrapped {
        ext.pos = pos;
    }

    let pdb = match parse_policydb(ext) {
        Ok(Some(pdb)) => Some(pdb),
        Ok(None) if wrapped => return Err(-EBADMSG),
        Ok(None) => None,
        Err(err) => return Err(err),
    };

    if wrapped {
        ext.unpack_struct_end()?;
    }
    Ok(pdb)
}

fn parse_policydb(ext: &mut AaExt<'_>) -> Result<Option<AppArmorPolicyDb>, i32> {
    let start_pos = ext.pos;
    let perms = parse_perms_table(ext)?;
    let (dfa_blob, _) = match ext.unpack_blob_with_offset(Some("aadfa")) {
        Ok(blob) if !blob.0.is_empty() => blob,
        _ => {
            ext.pos = start_pos;
            return Ok(None);
        }
    };
    let dfa = AppArmorDfa::unpack(&dfa_blob)?;
    let mut start = [DFA_NOMATCH; AA_CLASS_LAST + 1];
    let state_count = dfa.state_count() as u32;
    let default_start = ext.unpack_u32(Some("start")).unwrap_or(DFA_START);
    let file_start = ext.unpack_u32(Some("dfa_start")).unwrap_or(DFA_START);
    if default_start >= state_count || file_start >= state_count {
        return Err(-EPROTO);
    }
    start[0] = default_start;
    start[AA_CLASS_FILE as usize] = file_start;
    for class in AA_CLASS_FILE as usize + 1..=AA_CLASS_LAST {
        start[class] = dfa.next_state(default_start, class as u8);
    }
    verify_dfa_accept_indexes(&dfa, &perms)?;
    Ok(Some(AppArmorPolicyDb { dfa, perms, start }))
}

fn parse_perms_table(ext: &mut AaExt<'_>) -> Result<Vec<AppArmorPerms>, i32> {
    let pos = ext.pos;
    if ext.unpack_struct_start(Some("perms")).is_err() {
        ext.pos = pos;
        return Ok(default_perms_table());
    }
    let version = ext.unpack_u32(Some("version"))?;
    if version != 1 {
        return Err(-EPROTONOSUPPORT);
    }
    let count = ext.unpack_array(None)?;
    let mut perms = Vec::new();
    for _ in 0..count {
        let reserved = ext.unpack_u32(None)?;
        let _ = reserved;
        let perm = AppArmorPerms {
            allow: ext.unpack_u32(None)?,
            deny: ext.unpack_u32(None)?,
            subtree: ext.unpack_u32(None)?,
            cond: ext.unpack_u32(None)?,
            kill: ext.unpack_u32(None)?,
            complain: ext.unpack_u32(None)?,
            prompt: ext.unpack_u32(None)?,
            audit: ext.unpack_u32(None)?,
            quiet: ext.unpack_u32(None)?,
            hide: ext.unpack_u32(None)?,
            xindex: ext.unpack_u32(None)?,
            tag: ext.unpack_u32(None)?,
            label: ext.unpack_u32(None)?,
        };
        if !verify_perm(&perm) {
            return Err(-EPROTO);
        }
        perms.push(perm);
    }
    let _ = ext.unpack_array_end();
    ext.unpack_struct_end()?;
    if perms.is_empty() {
        return Err(-EPROTO);
    }
    Ok(perms)
}

fn default_perms_table() -> Vec<AppArmorPerms> {
    let mut perms = Vec::new();
    perms.push(AppArmorPerms::default());
    perms
}

fn verify_perm(perm: &AppArmorPerms) -> bool {
    if perm.allow & perm.deny != 0 {
        return false;
    }
    if perm.subtree & !perm.allow != 0 {
        return false;
    }
    if perm.cond & (perm.allow | perm.deny) != 0 {
        return false;
    }
    if perm.kill & perm.allow != 0 {
        return false;
    }
    if perm.complain & (perm.allow | perm.deny) != 0 {
        return false;
    }
    if perm.prompt & (perm.allow | perm.deny) != 0 {
        return false;
    }
    if perm.complain & perm.prompt != 0 {
        return false;
    }
    if perm.hide & perm.allow != 0 {
        return false;
    }
    true
}

fn verify_dfa_accept_indexes(dfa: &AppArmorDfa, perms: &[AppArmorPerms]) -> Result<(), i32> {
    for index in &dfa.accept {
        if (*index as usize) >= perms.len() {
            return Err(-EPROTO);
        }
    }
    Ok(())
}

impl Default for AppArmorPerms {
    fn default() -> Self {
        Self {
            allow: 0,
            deny: 0,
            subtree: 0,
            cond: 0,
            kill: 0,
            complain: 0,
            prompt: 0,
            audit: 0,
            quiet: 0,
            hide: 0,
            xindex: 0,
            tag: 0,
            label: 0,
        }
    }
}

impl AppArmorPolicyDb {
    fn perms_for_path(&self, path: &str) -> Option<&AppArmorPerms> {
        let state = self
            .dfa
            .match_bytes(self.start[AA_CLASS_FILE as usize], path.as_bytes());
        let index = self.dfa.accept.get(state as usize).copied()? as usize;
        self.perms.get(index)
    }
}

impl AppArmorDfa {
    fn unpack(bytes: &[u8]) -> Result<Self, i32> {
        let bytes = dfa_aligned_payload(bytes).ok_or(-EPROTO)?;
        if bytes.len() < 14 || read_be_u32(bytes, 0)? != YYTH_MAGIC {
            return Err(-EPROTO);
        }
        let hsize = read_be_u32(bytes, 4)? as usize;
        if hsize < 14 || bytes.len() < hsize {
            return Err(-EPROTO);
        }
        let flags = read_be_u16(bytes, 12)?;
        if flags & !YYTH_FLAGS != 0 {
            return Err(-EPROTO);
        }

        let mut accept: Option<Vec<u32>> = None;
        let mut accept2: Option<Vec<u32>> = None;
        let mut default: Option<Vec<u32>> = None;
        let mut base: Option<Vec<u32>> = None;
        let mut next: Option<Vec<u32>> = None;
        let mut check: Option<Vec<u32>> = None;
        let mut equiv: Option<Vec<u8>> = None;
        let mut offset = hsize;
        while offset < bytes.len() {
            let (table, consumed) = unpack_dfa_table(&bytes[offset..])?;
            match table.id {
                YYTD_ID_ACCEPT => set_table(&mut accept, table.values32)?,
                YYTD_ID_ACCEPT2 => set_table(&mut accept2, table.values32)?,
                YYTD_ID_BASE => {
                    if table.flags != YYTD_DATA32 {
                        return Err(-EPROTO);
                    }
                    set_table(&mut base, table.values32)?;
                }
                YYTD_ID_DEF => set_table(&mut default, table.values32)?,
                YYTD_ID_NXT => set_table(&mut next, table.values32)?,
                YYTD_ID_CHK => set_table(&mut check, table.values32)?,
                YYTD_ID_EC => set_table(&mut equiv, table.values8)?,
                _ => return Err(-EPROTO),
            }
            offset = offset.checked_add(consumed).ok_or(-EOVERFLOW)?;
        }

        let accept = accept.ok_or(-EPROTO)?;
        let default = default.ok_or(-EPROTO)?;
        let base = base.ok_or(-EPROTO)?;
        let next = next.ok_or(-EPROTO)?;
        let check = check.ok_or(-EPROTO)?;
        let accept2 = accept2.unwrap_or_else(|| alloc::vec![0; accept.len()]);
        let dfa = Self {
            flags,
            max_oob: 1,
            accept,
            accept2,
            default,
            base,
            next,
            check,
            equiv,
        };
        dfa.verify()?;
        Ok(dfa)
    }

    fn state_count(&self) -> usize {
        self.base.len()
    }

    fn match_bytes(&self, mut state: u32, bytes: &[u8]) -> u32 {
        if state == DFA_NOMATCH {
            return DFA_NOMATCH;
        }
        for byte in bytes {
            state = self.next_state(state, *byte);
            if state == DFA_NOMATCH {
                break;
            }
        }
        state
    }

    fn next_state(&self, mut state: u32, byte: u8) -> u32 {
        if state == DFA_NOMATCH {
            return DFA_NOMATCH;
        }
        let c = self
            .equiv
            .as_ref()
            .and_then(|equiv| equiv.get(byte as usize).copied())
            .unwrap_or(byte) as usize;
        loop {
            let index = state as usize;
            let Some((&base, &default)) = self.base.get(index).zip(self.default.get(index)) else {
                return DFA_NOMATCH;
            };
            let Some(pos) = base_idx(base).checked_add(c) else {
                return DFA_NOMATCH;
            };
            if self.check.get(pos).copied() != Some(state) {
                state = default;
                if base & MATCH_FLAG_DIFF_ENCODE != 0 {
                    continue;
                }
                break;
            }
            state = self.next.get(pos).copied().unwrap_or(DFA_NOMATCH);
            break;
        }
        state
    }

    fn verify(&self) -> Result<(), i32> {
        let state_count = self.base.len();
        let trans_count = self.next.len();
        if state_count < 2
            || self.default.len() != state_count
            || self.accept.len() != state_count
            || self.accept2.len() != state_count
            || self.check.len() != trans_count
        {
            return Err(-EPROTO);
        }
        if let Some(equiv) = &self.equiv
            && equiv.len() != 256
        {
            return Err(-EPROTO);
        }
        for state in 0..state_count {
            if self.default[state] as usize >= state_count {
                return Err(-EPROTO);
            }
            if self.base[state] & MATCH_FLAGS_INVALID != 0 {
                return Err(-EPROTO);
            }
            if self.base[state] & MATCH_FLAG_DIFF_ENCODE != 0
                && self.flags & YYTH_FLAG_DIFF_ENCODE == 0
            {
                return Err(-EPROTO);
            }
            if self.base[state] & MATCH_FLAG_OOB_TRANSITION != 0 {
                if base_idx(self.base[state]) < self.max_oob as usize
                    || self.flags & YYTH_FLAG_OOB_TRANS == 0
                {
                    return Err(-EPROTO);
                }
            }
            if base_idx(self.base[state])
                .checked_add(255)
                .ok_or(-EOVERFLOW)?
                >= trans_count
            {
                return Err(-EPROTO);
            }
            verify_diff_chain(state, &self.base, &self.default)?;
        }
        for state in &self.next {
            if *state as usize >= state_count {
                return Err(-EPROTO);
            }
        }
        for state in &self.check {
            if *state as usize >= state_count {
                return Err(-EPROTO);
            }
        }
        Ok(())
    }
}

struct DfaTable {
    id: u16,
    flags: u16,
    values32: Vec<u32>,
    values8: Vec<u8>,
}

fn unpack_dfa_table(bytes: &[u8]) -> Result<(DfaTable, usize), i32> {
    const TABLE_HEADER_SIZE: usize = 12;
    if bytes.len() < TABLE_HEADER_SIZE {
        return Err(-EPROTO);
    }
    let raw_id = read_be_u16(bytes, 0)?;
    let id = raw_id.checked_sub(1).ok_or(-EPROTO)?;
    let flags = read_be_u16(bytes, 2)?;
    let len = read_be_u32(bytes, 8)? as usize;
    if len == 0 || !matches!(flags, YYTD_DATA8 | YYTD_DATA16 | YYTD_DATA32) {
        return Err(-EPROTO);
    }
    let elem_size = flags as usize;
    let data_len = len.checked_mul(elem_size).ok_or(-EOVERFLOW)?;
    let data_end = TABLE_HEADER_SIZE.checked_add(data_len).ok_or(-EOVERFLOW)?;
    if bytes.len() < data_end {
        return Err(-EPROTO);
    }
    let consumed = align8(data_end);
    if bytes.len() < consumed {
        return Err(-EPROTO);
    }
    let data = &bytes[TABLE_HEADER_SIZE..data_end];
    let mut values32 = Vec::new();
    let mut values8 = Vec::new();
    match flags {
        YYTD_DATA8 => values8.extend_from_slice(data),
        YYTD_DATA16 => {
            for chunk in data.chunks_exact(2) {
                values32.push(u16::from_be_bytes([chunk[0], chunk[1]]) as u32);
            }
        }
        YYTD_DATA32 => {
            for chunk in data.chunks_exact(4) {
                values32.push(u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
            }
        }
        _ => return Err(-EPROTO),
    }
    Ok((
        DfaTable {
            id,
            flags,
            values32,
            values8,
        },
        consumed,
    ))
}

fn set_table<T>(slot: &mut Option<Vec<T>>, value: Vec<T>) -> Result<(), i32> {
    if slot.is_some() {
        return Err(-EPROTO);
    }
    *slot = Some(value);
    Ok(())
}

fn dfa_aligned_payload(bytes: &[u8]) -> Option<&[u8]> {
    let magic = YYTH_MAGIC.to_be_bytes();
    for pad in 0..=7 {
        if bytes.get(pad..pad + 4) == Some(magic.as_slice()) {
            return bytes.get(pad..);
        }
    }
    None
}

fn verify_diff_chain(state: usize, base: &[u32], default: &[u32]) -> Result<(), i32> {
    if base[state] & MATCH_FLAG_DIFF_ENCODE == 0 {
        return Ok(());
    }
    let mut seen = alloc::vec![false; base.len()];
    let mut current = state;
    while base[current] & MATCH_FLAG_DIFF_ENCODE != 0 {
        if seen[current] {
            return Err(-EPROTO);
        }
        seen[current] = true;
        let next = default[current] as usize;
        if next == current || next >= base.len() {
            return Err(-EPROTO);
        }
        current = next;
    }
    Ok(())
}

fn base_idx(value: u32) -> usize {
    (value & 0x00ff_ffff) as usize
}

fn align8(value: usize) -> usize {
    (value + 7) & !7
}

fn read_be_u16(bytes: &[u8], offset: usize) -> Result<u16, i32> {
    let end = offset.checked_add(2).ok_or(-EOVERFLOW)?;
    let bytes = bytes.get(offset..end).ok_or(-EPROTO)?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn read_be_u32(bytes: &[u8], offset: usize) -> Result<u32, i32> {
    let end = offset.checked_add(4).ok_or(-EOVERFLOW)?;
    let bytes = bytes.get(offset..end).ok_or(-EPROTO)?;
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn parse_binary_profile_data(ext: &mut AaExt<'_>) -> Result<Vec<AppArmorDataBlock>, i32> {
    let pos = ext.pos;
    if ext.unpack_struct_start(Some("data")).is_err() {
        ext.pos = pos;
        return Ok(Vec::new());
    }

    let mut blocks: Vec<AppArmorDataBlock> = Vec::new();
    loop {
        let entry_pos = ext.pos;
        let Ok(key) = ext.unpack_string(None) else {
            ext.pos = entry_pos;
            break;
        };
        if blocks.iter().any(|block| block.key == key) {
            return Err(-EBADMSG);
        }
        let value = ext.unpack_blob(None)?;
        blocks.push(AppArmorDataBlock { key, value });
    }
    ext.unpack_struct_end()?;
    Ok(blocks)
}

struct AaExt<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> AaExt<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn unpack_u32(&mut self, name: Option<&str>) -> Result<u32, i32> {
        let pos = self.pos;
        if self.unpack_name_x(AA_EXT_U32, name).is_err() {
            self.pos = pos;
            return Err(-EBADMSG);
        }
        let value = self.read_u32().inspect_err(|_| self.pos = pos)?;
        Ok(value)
    }

    fn unpack_string(&mut self, name: Option<&str>) -> Result<String, i32> {
        let pos = self.pos;
        if self.unpack_name_x(AA_EXT_STRING, name).is_err() {
            self.pos = pos;
            return Err(-EBADMSG);
        }
        let chunk = self.read_u16_chunk().inspect_err(|_| self.pos = pos)?;
        if chunk.last() != Some(&0) {
            self.pos = pos;
            return Err(-EBADMSG);
        }
        core::str::from_utf8(&chunk[..chunk.len() - 1])
            .map(String::from)
            .map_err(|_| {
                self.pos = pos;
                -EINVAL
            })
    }

    fn unpack_blob(&mut self, name: Option<&str>) -> Result<Vec<u8>, i32> {
        self.unpack_blob_with_offset(name).map(|(bytes, _)| bytes)
    }

    fn unpack_blob_with_offset(&mut self, name: Option<&str>) -> Result<(Vec<u8>, usize), i32> {
        let pos = self.pos;
        if self.unpack_name_x(AA_EXT_BLOB, name).is_err() {
            self.pos = pos;
            return Err(-EBADMSG);
        }
        let len = self.read_u32().inspect_err(|_| self.pos = pos)? as usize;
        let end = self.pos.checked_add(len).ok_or(-EOVERFLOW)?;
        let bytes = self.bytes.get(self.pos..end).ok_or_else(|| {
            self.pos = pos;
            -EBADMSG
        })?;
        let data_offset = self.pos;
        self.pos = end;
        Ok((bytes.to_vec(), data_offset))
    }

    fn unpack_array(&mut self, name: Option<&str>) -> Result<u16, i32> {
        let pos = self.pos;
        if self.unpack_name_x(AA_EXT_ARRAY, name).is_err() {
            self.pos = pos;
            return Err(-EBADMSG);
        }
        self.read_u16().inspect_err(|_| self.pos = pos)
    }

    fn unpack_struct_start(&mut self, name: Option<&str>) -> Result<(), i32> {
        self.unpack_name_x(AA_EXT_STRUCT, name)
    }

    fn unpack_struct_end(&mut self) -> Result<(), i32> {
        self.unpack_x(AA_EXT_STRUCTEND)
    }

    fn unpack_array_end(&mut self) -> Result<(), i32> {
        self.unpack_x(AA_EXT_ARRAYEND)
    }

    fn unpack_name_x(&mut self, code: u8, name: Option<&str>) -> Result<(), i32> {
        let pos = self.pos;
        if self.unpack_x(AA_EXT_NAME).is_ok() {
            let tag = self.read_u16_chunk().inspect_err(|_| self.pos = pos)?;
            if tag.last() != Some(&0) {
                self.pos = pos;
                return Err(-EBADMSG);
            }
            if let Some(expected) = name {
                if &tag[..tag.len() - 1] != expected.as_bytes() {
                    self.pos = pos;
                    return Err(-EBADMSG);
                }
            }
        } else if name.is_some() {
            self.pos = pos;
            return Err(-EBADMSG);
        }

        if self.unpack_x(code).is_err() {
            self.pos = pos;
            return Err(-EBADMSG);
        }
        Ok(())
    }

    fn unpack_x(&mut self, code: u8) -> Result<(), i32> {
        if self.bytes.get(self.pos) != Some(&code) {
            return Err(-EBADMSG);
        }
        self.pos += 1;
        Ok(())
    }

    fn read_u16(&mut self) -> Result<u16, i32> {
        let end = self.pos.checked_add(2).ok_or(-EOVERFLOW)?;
        let bytes = self.bytes.get(self.pos..end).ok_or(-EBADMSG)?;
        self.pos = end;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, i32> {
        let end = self.pos.checked_add(4).ok_or(-EOVERFLOW)?;
        let bytes = self.bytes.get(self.pos..end).ok_or(-EBADMSG)?;
        self.pos = end;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_u16_chunk(&mut self) -> Result<&'a [u8], i32> {
        let len = self.read_u16()? as usize;
        let end = self.pos.checked_add(len).ok_or(-EOVERFLOW)?;
        let bytes = self.bytes.get(self.pos..end).ok_or(-EBADMSG)?;
        self.pos = end;
        Ok(bytes)
    }
}

fn split_rule_path(line: &str) -> Option<(String, &str)> {
    if line.starts_with('"') {
        let end = line[1..].find('"')? + 1;
        let path = String::from(&line[1..end]);
        return Some((path, &line[end + 1..]));
    }

    if !line.starts_with('/') {
        return None;
    }
    let end = line
        .find(|ch: char| ch.is_ascii_whitespace() || ch == ',')
        .unwrap_or(line.len());
    Some((String::from(&line[..end]), &line[end..]))
}

fn parse_perm_token(token: &str) -> Result<u32, i32> {
    let mut mask = 0;
    let token = token.trim_start_matches(',').trim();
    for ch in token.chars() {
        match ch {
            'r' => mask |= AA_MAY_READ,
            'w' => mask |= AA_MAY_WRITE,
            'a' => mask |= AA_MAY_APPEND,
            'x' | 'm' => mask |= AA_MAY_EXEC,
            'k' | 'l' => {}
            ',' | '}' | '#' | '\0' => break,
            ch if ch.is_ascii_whitespace() => break,
            _ => return Err(-EINVAL),
        }
    }
    Ok(mask)
}

pub fn check_path_open_for_profile(profile_name: &str, path: &[u8], flags: i32) -> i32 {
    mediate_label_path(profile_name, path, request_mask_from_open_flags(flags))
}

pub fn check_exec_for_profile(profile_name: &str, path: &[u8]) -> i32 {
    mediate_label_path(profile_name, path, AA_MAY_EXEC)
}

pub fn profile_exists(profile_name: &str) -> bool {
    let (namespace, name) = split_profile_fqname(profile_name);
    PROFILES
        .lock()
        .iter()
        .any(|profile| profile.namespace == namespace && profile.name == name)
}

pub fn query_label_permissions(
    label_name: &str,
    match_bytes: &[u8],
) -> Result<AppArmorQueryPerms, i32> {
    let Some((&class, path)) = match_bytes.split_first() else {
        return Err(-EINVAL);
    };
    if class != AA_CLASS_FILE {
        return Err(-EINVAL);
    }
    let path = core::str::from_utf8(path).map_err(|_| -EINVAL)?;
    let label = AppArmorLabel::parse(label_name)?;
    let profiles = PROFILES.lock();
    let (allow, deny, audit, quiet, found) = label_masks_for_path(&profiles, &label, path);
    if !found {
        return Err(-ENOENT);
    }
    Ok(AppArmorQueryPerms {
        allow,
        deny,
        audit,
        quiet,
    })
}

pub fn query_label_data(label_name: &str, key: &str) -> Result<Vec<Vec<u8>>, i32> {
    let label = AppArmorLabel::parse(label_name)?;
    let profiles = PROFILES.lock();
    let mut out = Vec::new();
    for component in &label.components {
        let Some(profile) = find_profile(&profiles, component) else {
            return Err(-ENOENT);
        };
        out.extend(
            profile
                .data
                .iter()
                .filter(|block| block.key == key)
                .map(|block| block.value.clone()),
        );
    }
    Ok(out)
}

fn mediate_current_path(path: &[u8], request: u32) -> i32 {
    match current_label() {
        Some(label) => mediate_label(&label, path, request),
        None => 0,
    }
}

fn current_task_id() -> Option<u32> {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return None;
    }
    let pid = unsafe { (*task).pid };
    u32::try_from(pid).ok().filter(|pid| *pid != 0)
}

fn current_label() -> Option<AppArmorLabel> {
    if let Some(task_id) = current_task_id()
        && let Some(label) = task_label(task_id)
    {
        return Some(label);
    }
    CURRENT_LABEL
        .lock()
        .clone()
        .and_then(|label| refresh_label(&label))
}

fn task_label(task_id: u32) -> Option<AppArmorLabel> {
    TASK_LABELS
        .lock()
        .iter()
        .find(|label| label.task_id == task_id)
        .and_then(|label| refresh_label(&label.label))
}

fn set_task_profile(task_id: u32, profile: Option<String>) {
    set_label_entry(&mut TASK_LABELS.lock(), task_id, profile.as_deref());
}

fn set_pending_exec_profile(task_id: u32, profile: Option<String>) {
    set_label_entry(&mut PENDING_EXEC_LABELS.lock(), task_id, profile.as_deref());
}

fn take_pending_exec_profile(task_id: u32) -> Option<AppArmorLabel> {
    let mut labels = PENDING_EXEC_LABELS.lock();
    let index = labels.iter().position(|label| label.task_id == task_id)?;
    refresh_label(&labels.remove(index).label)
}

fn set_label_entry(labels: &mut Vec<TaskLabel>, task_id: u32, profile: Option<&str>) {
    labels.retain(|label| label.task_id != task_id);
    if let Some(profile) = profile {
        if let Ok(label) = AppArmorLabel::parse(profile)
            && let Some(label) = refresh_label(&label)
        {
            labels.push(TaskLabel { task_id, label });
        }
    }
}

fn attach_profile_for_path(path: &[u8]) -> Option<String> {
    let Ok(path) = core::str::from_utf8(path) else {
        return None;
    };
    PROFILES
        .lock()
        .iter()
        .find(|profile| {
            profile
                .attach
                .as_deref()
                .is_some_and(|attach| apparmor_path_matches(attach, path))
        })
        .map(|profile| profile.fqname())
}

fn mediate_label_path(label_name: &str, path: &[u8], request: u32) -> i32 {
    let Ok(label) = AppArmorLabel::parse(label_name) else {
        return -EINVAL;
    };
    let Some(label) = refresh_label(&label) else {
        return -ENOENT;
    };
    mediate_label(&label, path, request)
}

fn mediate_label(label: &AppArmorLabel, path: &[u8], request: u32) -> i32 {
    if request == 0 {
        return 0;
    }
    let Ok(path) = core::str::from_utf8(path) else {
        return -EINVAL;
    };
    let profiles = PROFILES.lock();
    let (allow, deny, _audit, _quiet, found) = label_masks_for_path(&profiles, label, path);
    if !found {
        return -ENOENT;
    }
    if request & !(allow & !deny) == 0 {
        return 0;
    }
    if label
        .components
        .iter()
        .filter_map(|component| find_profile(&profiles, component))
        .all(|profile| profile.mode == ProfileMode::Complain)
    {
        0
    } else {
        -EACCES
    }
}

fn label_masks_for_path(
    profiles: &[AppArmorProfile],
    label: &AppArmorLabel,
    path: &str,
) -> (u32, u32, u32, u32, bool) {
    let mut allow = u32::MAX;
    let mut deny = 0;
    let mut audit = 0;
    let mut quiet = 0;
    let mut found = false;
    for component in &label.components {
        let Some(profile) = find_profile(profiles, component) else {
            return (0, 0, 0, 0, false);
        };
        found = true;
        let (profile_allow, profile_deny, profile_audit, profile_quiet) =
            profile_masks_for_path(profile, path);
        allow &= profile_allow;
        deny |= profile_deny;
        audit |= profile_audit & profile_allow;
        quiet |= profile_quiet & !profile_allow;
    }
    if !found {
        return (0, 0, 0, 0, false);
    }
    (allow, deny, audit, quiet, true)
}

fn profile_masks_for_path(profile: &AppArmorProfile, path: &str) -> (u32, u32, u32, u32) {
    let mut allow = 0;
    let mut deny = 0;
    let mut audit = 0;
    let mut quiet = 0;
    if let Some(policy) = &profile.file
        && let Some(perms) = policy.perms_for_path(path)
    {
        allow |= perms.allow;
        deny |= perms.deny;
        audit |= perms.audit;
        quiet |= perms.quiet;
    }
    for rule in &profile.rules {
        if apparmor_path_matches(&rule.path, path) {
            allow |= rule.allow;
            deny |= rule.deny;
        }
    }
    if allow & (AA_MAY_READ | AA_MAY_WRITE | AA_MAY_APPEND | AA_MAY_EXEC) != 0 {
        allow |= AA_MAY_OPEN;
    }
    (allow, deny, audit, quiet)
}

fn find_profile<'a>(
    profiles: &'a [AppArmorProfile],
    component: &AppArmorLabelComponent,
) -> Option<&'a AppArmorProfile> {
    profiles.iter().find(|profile| {
        profile.namespace == component.namespace && profile.name == component.profile
    })
}

impl AppArmorLabel {
    fn parse(text: &str) -> Result<Self, i32> {
        let mut components = Vec::new();
        let mut input = text.trim();
        if let Some(rest) = input.strip_prefix('&') {
            input = rest;
        }
        for raw in input.split("//&") {
            let raw = raw.trim();
            if raw.is_empty() {
                return Err(-EINVAL);
            }
            let (namespace, profile) = split_profile_fqname(raw);
            if profile.is_empty() {
                return Err(-EINVAL);
            }
            components.push(AppArmorLabelComponent { namespace, profile });
        }
        if components.is_empty() {
            return Err(-EINVAL);
        }
        Ok(Self {
            generation: policy_revision(),
            components,
        })
    }

    fn display_name(&self) -> String {
        let mut out = String::new();
        for (index, component) in self.components.iter().enumerate() {
            if index != 0 {
                out.push_str("//&");
            }
            if component.namespace == ROOT_NAMESPACE {
                out.push_str(&component.profile);
            } else {
                out.push_str(&alloc::format!(
                    ":{}:{}",
                    component.namespace,
                    component.profile
                ));
            }
        }
        out
    }
}

fn refresh_label(label: &AppArmorLabel) -> Option<AppArmorLabel> {
    let profiles = PROFILES.lock();
    if label
        .components
        .iter()
        .all(|component| find_profile(&profiles, component).is_some())
    {
        let mut newest = label.clone();
        newest.generation = policy_revision();
        Some(newest)
    } else {
        None
    }
}

fn refresh_all_labels_after_policy_change() {
    refresh_label_vec(&mut TASK_LABELS.lock());
    refresh_label_vec(&mut PENDING_EXEC_LABELS.lock());
    let current = CURRENT_LABEL.lock().clone();
    let refreshed = current.and_then(|label| refresh_label(&label));
    *CURRENT_LABEL.lock() = refreshed;
}

fn refresh_label_vec(labels: &mut Vec<TaskLabel>) {
    let mut refreshed = Vec::new();
    for label in labels.iter() {
        if let Some(newest) = refresh_label(&label.label) {
            refreshed.push(TaskLabel {
                task_id: label.task_id,
                label: newest,
            });
        }
    }
    *labels = refreshed;
}

fn set_task_label(task_id: u32, label: Option<AppArmorLabel>) {
    let mut labels = TASK_LABELS.lock();
    labels.retain(|entry| entry.task_id != task_id);
    if let Some(label) = label.and_then(|label| refresh_label(&label)) {
        labels.push(TaskLabel { task_id, label });
    }
}

fn bump_namespace_revision(namespace: &str, revision: u64) {
    let mut namespaces = NAMESPACES.lock();
    if let Some(existing) = namespaces.iter_mut().find(|entry| entry.name == namespace) {
        existing.revision = revision;
    } else {
        namespaces.push(AppArmorNamespace {
            name: String::from(namespace),
            revision,
        });
    }
}

fn apparmor_path_matches(rule: &str, path: &str) -> bool {
    if rule == path || rule == "/**" {
        return true;
    }
    if let Some(prefix) = rule.strip_suffix("/**") {
        return path == prefix
            || path
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with('/'));
    }
    if let Some(prefix) = rule.strip_suffix('*') {
        return path.starts_with(prefix);
    }
    false
}

fn request_mask_from_open_flags(flags: i32) -> u32 {
    let flags = flags as u32;
    let mut mask = match flags & O_ACCMODE {
        O_WRONLY => AA_MAY_WRITE,
        O_RDWR => AA_MAY_READ | AA_MAY_WRITE,
        _ => AA_MAY_READ,
    };
    if flags & O_TRUNC != 0 {
        mask |= AA_MAY_WRITE;
    }
    if flags & O_APPEND != 0 {
        mask |= AA_MAY_APPEND;
    }
    mask | AA_MAY_OPEN
}

#[cfg(feature = "test-lsm-suite")]
pub fn run_lsm_suite_acceptance() -> Result<(), i32> {
    let one = b"profile apparmor.one { /shared r, /only-one r, }\n";
    let two = b"profile apparmor.two { /shared r, }\n";
    load_policy_blob(one)?;
    load_policy_blob(two)?;
    if check_path_open_for_profile("apparmor.one//&apparmor.two", b"/shared", 0) != 0 {
        return Err(-EACCES);
    }
    if check_path_open_for_profile("apparmor.one//&apparmor.two", b"/only-one", 0) != -EACCES {
        return Err(-EINVAL);
    }

    let first = lsm_suite_binary_dfa_policy("apparmor.dfa", "suite", "/old", AA_MAY_READ, 1);
    load_policy_blob(&first)?;
    set_current_profile_for_acceptance(Some(":suite:apparmor.dfa"))?;
    if check_path_open_for_profile(":suite:apparmor.dfa", b"/old", 0) != 0 {
        return Err(-EACCES);
    }

    let second = lsm_suite_binary_dfa_policy(
        "apparmor.dfa",
        "suite",
        "/new",
        AA_MAY_READ | AA_MAY_WRITE,
        1,
    );
    replace_policy_blob(&second)?;
    if check_path_open_for_profile(":suite:apparmor.dfa", b"/old", 0) != -EACCES {
        return Err(-EINVAL);
    }
    if check_path_open_for_profile(
        ":suite:apparmor.dfa",
        b"/new",
        crate::include::uapi::fcntl::O_WRONLY as i32,
    ) != 0
    {
        return Err(-EACCES);
    }
    set_current_profile_for_acceptance(None)?;
    Ok(())
}

#[cfg(feature = "test-lsm-suite")]
fn set_current_profile_for_acceptance(profile: Option<&str>) -> Result<(), i32> {
    *CURRENT_LABEL.lock() = match profile {
        Some(profile) => Some(refresh_label(&AppArmorLabel::parse(profile)?).ok_or(-ENOENT)?),
        None => None,
    };
    Ok(())
}

#[cfg(feature = "test-lsm-suite")]
fn lsm_suite_binary_dfa_policy(
    name: &str,
    namespace: &str,
    path: &str,
    allow: u32,
    accept_index: u32,
) -> Vec<u8> {
    let mut out = Vec::new();
    aa_named_u32(&mut out, "version", AA_POLICY_ABI_MAX);
    aa_named_string(&mut out, "namespace", namespace);
    aa_named_struct_start(&mut out, "profile");
    aa_named_string(&mut out, "name", name);
    aa_named_u32(&mut out, "mode", 0);
    aa_named_struct_start(&mut out, "file");
    aa_named_struct_start(&mut out, "perms");
    aa_named_u32(&mut out, "version", 1);
    out.push(AA_EXT_ARRAY);
    out.extend_from_slice(&2u16.to_le_bytes());
    aa_raw_perm(&mut out, AppArmorPerms::default());
    aa_raw_perm(
        &mut out,
        AppArmorPerms {
            allow,
            audit: allow,
            ..AppArmorPerms::default()
        },
    );
    out.push(AA_EXT_ARRAYEND);
    out.push(AA_EXT_STRUCTEND);
    aa_named_blob(
        &mut out,
        "aadfa",
        &lsm_suite_exact_path_dfa(path, accept_index),
    );
    aa_named_u32(&mut out, "start", DFA_START);
    aa_named_u32(&mut out, "dfa_start", DFA_START);
    out.push(AA_EXT_STRUCTEND);
    out.push(AA_EXT_STRUCTEND);
    out
}

#[cfg(feature = "test-lsm-suite")]
fn aa_named_u32(out: &mut Vec<u8>, name: &str, value: u32) {
    aa_named(out, name);
    out.push(AA_EXT_U32);
    out.extend_from_slice(&value.to_le_bytes());
}

#[cfg(feature = "test-lsm-suite")]
fn aa_named_string(out: &mut Vec<u8>, name: &str, value: &str) {
    aa_named(out, name);
    out.push(AA_EXT_STRING);
    let len = value.len() + 1;
    out.extend_from_slice(&(len as u16).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    out.push(0);
}

#[cfg(feature = "test-lsm-suite")]
fn aa_named_struct_start(out: &mut Vec<u8>, name: &str) {
    aa_named(out, name);
    out.push(AA_EXT_STRUCT);
}

#[cfg(feature = "test-lsm-suite")]
fn aa_named_blob(out: &mut Vec<u8>, name: &str, value: &[u8]) {
    aa_named(out, name);
    out.push(AA_EXT_BLOB);
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value);
}

#[cfg(feature = "test-lsm-suite")]
fn aa_named(out: &mut Vec<u8>, name: &str) {
    out.push(AA_EXT_NAME);
    let len = name.len() + 1;
    out.extend_from_slice(&(len as u16).to_le_bytes());
    out.extend_from_slice(name.as_bytes());
    out.push(0);
}

#[cfg(feature = "test-lsm-suite")]
fn aa_raw_perm(out: &mut Vec<u8>, perm: AppArmorPerms) {
    for value in [
        0,
        perm.allow,
        perm.deny,
        perm.subtree,
        perm.cond,
        perm.kill,
        perm.complain,
        perm.prompt,
        perm.audit,
        perm.quiet,
        perm.hide,
        perm.xindex,
        perm.tag,
        perm.label,
    ] {
        out.push(AA_EXT_U32);
        out.extend_from_slice(&value.to_le_bytes());
    }
}

#[cfg(feature = "test-lsm-suite")]
fn lsm_suite_exact_path_dfa(path: &str, accept_index: u32) -> Vec<u8> {
    let state_count = path.len() + 2;
    let trans_count = state_count * 256;
    let mut accept = alloc::vec![0u32; state_count];
    let mut default = alloc::vec![DFA_NOMATCH; state_count];
    let mut base = alloc::vec![0u32; state_count];
    let mut next = alloc::vec![DFA_NOMATCH; trans_count];
    let mut check = alloc::vec![DFA_NOMATCH; trans_count];
    for state in 0..state_count {
        base[state] = (state * 256) as u32;
        default[state] = DFA_NOMATCH;
    }
    let mut state = DFA_START as usize;
    for byte in path.as_bytes() {
        let next_state = state + 1;
        let pos = base[state] as usize + *byte as usize;
        check[pos] = state as u32;
        next[pos] = next_state as u32;
        state = next_state;
    }
    accept[state] = accept_index;

    let mut out = Vec::new();
    out.extend_from_slice(&YYTH_MAGIC.to_be_bytes());
    out.extend_from_slice(&16u32.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    aa_dfa_table32(&mut out, YYTD_ID_ACCEPT, &accept);
    aa_dfa_table32(&mut out, YYTD_ID_BASE, &base);
    aa_dfa_table32(&mut out, YYTD_ID_CHK, &check);
    aa_dfa_table32(&mut out, YYTD_ID_DEF, &default);
    aa_dfa_table32(&mut out, YYTD_ID_NXT, &next);
    out
}

#[cfg(feature = "test-lsm-suite")]
fn aa_dfa_table32(out: &mut Vec<u8>, id: u16, values: &[u32]) {
    out.extend_from_slice(&(id + 1).to_be_bytes());
    out.extend_from_slice(&YYTD_DATA32.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&(values.len() as u32).to_be_bytes());
    for value in values {
        out.extend_from_slice(&value.to_be_bytes());
    }
    while out.len() % 8 != 0 {
        out.push(0);
    }
}

#[cfg(test)]
pub fn reset_for_test() {
    INITIALIZED.store(false, Ordering::Release);
    POLICY_HASHING_ENABLED.store(false, Ordering::Release);
    POLICY_REVISION.store(0, Ordering::Release);
    PROFILES.lock().clear();
    NAMESPACES.lock().clear();
    *CURRENT_LABEL.lock() = None;
    TASK_LABELS.lock().clear();
    PENDING_EXEC_LABELS.lock().clear();
}

#[cfg(test)]
pub fn set_current_profile_for_test(profile: Option<&str>) {
    *CURRENT_LABEL.lock() = profile.and_then(|profile| {
        AppArmorLabel::parse(profile)
            .ok()
            .and_then(|label| refresh_label(&label))
    });
}

#[cfg(test)]
pub fn set_task_profile_for_test(task_id: u32, profile: Option<&str>) {
    set_task_profile(task_id, profile.map(String::from));
}

#[cfg(test)]
pub fn task_profile_for_test(task_id: u32) -> Option<String> {
    task_label(task_id).map(|label| label.display_name())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::cred::INIT_CRED;
    use crate::kernel::sched;
    use crate::kernel::task::TaskStruct;
    use crate::security::hooks::LSM_ID_APPARMOR;
    use crate::security::lsm_list::{TEST_LSM_LOCK, lsm_active_ids, reset_for_test as reset_lsms};
    use alloc::boxed::Box;

    struct CurrentTaskGuard {
        previous: *mut TaskStruct,
    }

    impl CurrentTaskGuard {
        fn set(task: *mut TaskStruct) -> Self {
            let previous = unsafe { sched::get_current() };
            unsafe { sched::set_current(task) };
            Self { previous }
        }
    }

    impl Drop for CurrentTaskGuard {
        fn drop(&mut self) {
            unsafe { sched::set_current(self.previous) };
        }
    }

    fn task(pid: i32) -> Box<TaskStruct> {
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        task.pid = pid;
        task.tgid = pid;
        task.cred = &raw const INIT_CRED;
        task.m27.real_cred = &raw const INIT_CRED;
        task
    }

    #[test]
    fn apparmor_init_registers_lsm_and_hash_status() {
        let _guard = TEST_LSM_LOCK.lock();
        reset_lsms();
        reset_for_test();

        init();

        let state = snapshot();
        assert!(state.initialized);
        assert!(state.policy_hashing_enabled);
        assert_eq!(state.policy_hash_algorithm, "sha256");
        assert_eq!(state.policy_revision, 0);
        assert_eq!(state.profile_count, 0);

        let mut ids = [0u64; 2];
        assert_eq!(lsm_active_ids(&mut ids), 1);
        assert_eq!(ids[0], LSM_ID_APPARMOR);
    }

    #[test]
    fn apparmor_hooks_are_present_but_unconfined_until_policy_lands() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        assert_eq!(HOOKS.name, "apparmor");
        assert_eq!(HOOKS.id, LSM_ID_APPARMOR);
        assert_eq!((HOOKS.path_open.unwrap())(b"/etc/passwd", 0), 0);
        assert_eq!((HOOKS.socket_create.unwrap())(2, 1, 0), 0);
    }

    #[test]
    fn apparmor_policy_store_parses_text_profiles_and_revision() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();

        let policy = br#"abi <abi/4.0>,
profile "usr.bin.demo" flags=(attach_disconnected) {
  file,
}
"#;
        assert_eq!(load_policy_blob(policy), Ok(policy.len()));
        assert_eq!(policy_revision(), 1);
        assert_eq!(profiles_text(), "usr.bin.demo (enforce)\n");

        let complain = b"profile usr.bin.demo flags=(complain) { file, }\n";
        assert_eq!(replace_policy_blob(complain), Ok(complain.len()));
        assert_eq!(policy_revision(), 2);
        assert_eq!(profiles_text(), "usr.bin.demo (complain)\n");

        assert_eq!(remove_policy_blob(b"usr.bin.demo\n"), Ok(13));
        assert_eq!(policy_revision(), 3);
        assert_eq!(profiles_text(), "");
    }

    #[test]
    fn apparmor_text_policy_rules_mediate_open_and_exec_requests() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        let policy = br#"profile "/usr/bin/demo" flags=(attach_disconnected) {
  /etc/** r,
  deny /etc/shadow r,
  /tmp/** rw,
  /usr/bin/demo x,
}
"#;
        assert_eq!(load_policy_blob(policy), Ok(policy.len()));

        assert_eq!(
            check_path_open_for_profile("/usr/bin/demo", b"/etc/hostname", 0),
            0
        );
        assert_eq!(
            check_path_open_for_profile(
                "/usr/bin/demo",
                b"/etc/hostname",
                crate::include::uapi::fcntl::O_WRONLY as i32
            ),
            -EACCES
        );
        assert_eq!(
            check_path_open_for_profile("/usr/bin/demo", b"/etc/shadow", 0),
            -EACCES
        );
        assert_eq!(
            check_path_open_for_profile(
                "/usr/bin/demo",
                b"/tmp/out",
                crate::include::uapi::fcntl::O_RDWR as i32
            ),
            0
        );
        assert_eq!(check_exec_for_profile("/usr/bin/demo", b"/usr/bin/demo"), 0);
        assert_eq!(
            check_exec_for_profile("/usr/bin/demo", b"/usr/bin/other"),
            -EACCES
        );
    }

    #[test]
    fn apparmor_complain_mode_reports_allow_for_denied_requests() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        let policy = br#"profile demo flags=(complain) {
  /safe/** r,
}
"#;
        assert_eq!(load_policy_blob(policy), Ok(policy.len()));
        assert_eq!(
            check_path_open_for_profile(
                "demo",
                b"/blocked",
                crate::include::uapi::fcntl::O_WRONLY as i32
            ),
            0
        );
    }

    #[test]
    fn apparmor_binary_policy_unpack_loads_rules_and_revision() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        let policy = binary_profile(
            "bin.demo",
            ProfileMode::Enforce,
            Some("/usr/bin/bin-demo"),
            &[
                ("/var/lib/demo/**", AA_MAY_READ | AA_MAY_WRITE, 0),
                ("/var/lib/demo/private", 0, AA_MAY_READ | AA_MAY_WRITE),
            ],
        );

        assert_eq!(load_policy_blob(&policy), Ok(policy.len()));
        assert_eq!(profiles_text(), "bin.demo (enforce)\n");
        assert_eq!(policy_revision(), 1);
        assert_eq!(
            check_path_open_for_profile(
                "bin.demo",
                b"/var/lib/demo/state",
                crate::include::uapi::fcntl::O_RDWR as i32
            ),
            0
        );
        assert_eq!(
            check_path_open_for_profile("bin.demo", b"/var/lib/demo/private", 0),
            -EACCES
        );
    }

    #[test]
    fn apparmor_binary_policy_unpack_loads_linux_data_blocks() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        let policy = binary_profile_with_data(
            "data.demo",
            ProfileMode::Enforce,
            None,
            &[],
            &[("hostname", b"lupos"), ("release", b"trixie")],
        );

        assert_eq!(load_policy_blob(&policy), Ok(policy.len()));
        assert_eq!(
            query_label_data("data.demo", "hostname"),
            Ok(alloc::vec![b"lupos".to_vec()])
        );
        assert_eq!(
            query_label_data("data.demo", "release"),
            Ok(alloc::vec![b"trixie".to_vec()])
        );
        assert_eq!(query_label_data("data.demo", "missing"), Ok(Vec::new()));
        assert_eq!(query_label_data("missing.demo", "hostname"), Err(-ENOENT));

        let duplicate = binary_profile_with_data(
            "dup.demo",
            ProfileMode::Enforce,
            None,
            &[],
            &[("same", b"one"), ("same", b"two")],
        );
        assert_eq!(load_policy_blob(&duplicate), Err(-EBADMSG));
    }

    #[test]
    fn apparmor_binary_policy_dfa_unpack_mediates_file_permissions() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        assert!(AppArmorDfa::unpack(&exact_path_dfa("/srv/allowed", 1)).is_ok());
        let policy =
            binary_profile_with_file_dfa("dfa.demo", None, "/srv/allowed", AA_MAY_READ, 0, 1);
        let mut ext = AaExt::new(&policy);
        let mut namespace = None;
        verify_binary_header(&mut ext, true, &mut namespace).expect("header");
        ext.unpack_struct_start(Some("profile"))
            .expect("profile struct");
        assert_eq!(ext.unpack_string(Some("name")).unwrap(), "dfa.demo");
        assert_eq!(ext.unpack_u32(Some("mode")).unwrap(), 0);
        let pos = ext.pos;
        assert!(ext.unpack_string(Some("attach")).is_err());
        ext.pos = pos;
        assert!(ext.unpack_array(Some("rules")).is_err());
        ext.unpack_struct_start(Some("file")).expect("file struct");
        assert_eq!(parse_perms_table(&mut ext).expect("perms").len(), 2);
        let (blob, _) = ext.unpack_blob_with_offset(Some("aadfa")).expect("aadfa");
        let dfa = AppArmorDfa::unpack(&blob).expect("dfa");
        assert_eq!(dfa.accept.iter().copied().max(), Some(1));

        assert_eq!(load_policy_blob(&policy), Ok(policy.len()));
        assert_eq!(profiles_text(), "dfa.demo (enforce)\n");
        assert_eq!(
            check_path_open_for_profile("dfa.demo", b"/srv/allowed", 0),
            0
        );
        assert_eq!(
            check_path_open_for_profile(
                "dfa.demo",
                b"/srv/allowed",
                crate::include::uapi::fcntl::O_WRONLY as i32
            ),
            -EACCES
        );
        assert_eq!(
            check_path_open_for_profile("dfa.demo", b"/srv/other", 0),
            -EACCES
        );
        let mut query = Vec::new();
        query.push(AA_CLASS_FILE);
        query.extend_from_slice(b"/srv/allowed");
        assert_eq!(
            query_label_permissions("dfa.demo", &query),
            Ok(AppArmorQueryPerms {
                allow: AA_MAY_READ | AA_MAY_OPEN,
                deny: 0,
                audit: AA_MAY_READ,
                quiet: 0,
            })
        );

        let invalid =
            binary_profile_with_file_dfa("bad-dfa.demo", None, "/srv/allowed", AA_MAY_READ, 0, 2);
        assert_eq!(load_policy_blob(&invalid), Err(-EPROTO));
    }

    #[test]
    fn apparmor_namespace_label_refreshes_after_profile_replace() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        let first =
            binary_profile_with_file_dfa("dfa.demo", Some("tenant"), "/old", AA_MAY_READ, 0, 1);
        assert_eq!(load_policy_blob(&first), Ok(first.len()));
        assert_eq!(profiles_text(), ":tenant:dfa.demo (enforce)\n");

        set_task_profile_for_test(400, Some(":tenant:dfa.demo"));
        assert_eq!(
            task_profile_for_test(400).as_deref(),
            Some(":tenant:dfa.demo")
        );

        let second = binary_profile_with_file_dfa(
            "dfa.demo",
            Some("tenant"),
            "/new",
            AA_MAY_READ | AA_MAY_WRITE,
            0,
            1,
        );
        assert_eq!(replace_policy_blob(&second), Ok(second.len()));

        let mut current = task(400);
        let _guard = CurrentTaskGuard::set(&mut *current as *mut TaskStruct);
        assert_eq!((HOOKS.path_open.unwrap())(b"/old", 0), -EACCES);
        assert_eq!((HOOKS.path_open.unwrap())(b"/new", 0), 0);
        assert_eq!(
            (HOOKS.path_open.unwrap())(b"/new", crate::include::uapi::fcntl::O_WRONLY as i32),
            0
        );
    }

    #[test]
    fn apparmor_compound_labels_intersect_file_permissions() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        let one = b"profile one { /shared r, /only-one r, }\n";
        let two = b"profile two { /shared r, }\n";
        assert_eq!(load_policy_blob(one), Ok(one.len()));
        assert_eq!(load_policy_blob(two), Ok(two.len()));

        assert_eq!(check_path_open_for_profile("one//&two", b"/shared", 0), 0);
        assert_eq!(
            check_path_open_for_profile("one//&two", b"/only-one", 0),
            -EACCES
        );
        let mut query = Vec::new();
        query.push(AA_CLASS_FILE);
        query.extend_from_slice(b"/shared");
        assert_eq!(
            query_label_permissions("one//&two", &query),
            Ok(AppArmorQueryPerms {
                allow: AA_MAY_READ | AA_MAY_OPEN,
                deny: 0,
                audit: 0,
                quiet: 0,
            })
        );
    }

    #[test]
    fn apparmor_lsm_hook_uses_current_profile_when_task_label_exists() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        let policy = b"profile hook.demo { /allowed/** r, }\n";
        assert_eq!(load_policy_blob(policy), Ok(policy.len()));
        set_current_profile_for_test(Some("hook.demo"));

        assert_eq!((HOOKS.path_open.unwrap())(b"/allowed/file", 0), 0);
        assert_eq!(
            (HOOKS.path_open.unwrap())(
                b"/allowed/file",
                crate::include::uapi::fcntl::O_WRONLY as i32
            ),
            -EACCES
        );
        assert_eq!((HOOKS.path_open.unwrap())(b"/blocked", 0), -EACCES);

        set_current_profile_for_test(None);
        assert_eq!((HOOKS.path_open.unwrap())(b"/blocked", 0), 0);
    }

    #[test]
    fn apparmor_path_hook_uses_current_task_label() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        let policy = b"profile task.demo { /allowed/** r, }\n";
        assert_eq!(load_policy_blob(policy), Ok(policy.len()));

        let mut confined = task(100);
        let mut unconfined = task(101);
        set_task_profile_for_test(100, Some("task.demo"));

        {
            let _guard = CurrentTaskGuard::set(&mut *confined as *mut TaskStruct);
            assert_eq!((HOOKS.path_open.unwrap())(b"/allowed/file", 0), 0);
            assert_eq!((HOOKS.path_open.unwrap())(b"/blocked", 0), -EACCES);
        }
        {
            let _guard = CurrentTaskGuard::set(&mut *unconfined as *mut TaskStruct);
            assert_eq!((HOOKS.path_open.unwrap())(b"/blocked", 0), 0);
        }
    }

    #[test]
    fn apparmor_task_alloc_clones_and_task_free_clears_label() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        let policy = b"profile parent.demo { /allowed/** r, }\n";
        assert_eq!(load_policy_blob(policy), Ok(policy.len()));

        let mut parent = task(200);
        set_task_profile_for_test(200, Some("parent.demo"));
        let _guard = CurrentTaskGuard::set(&mut *parent as *mut TaskStruct);

        assert_eq!(apparmor_task_alloc(201, 0), 0);
        assert_eq!(task_profile_for_test(201).as_deref(), Some("parent.demo"));

        apparmor_task_free(201);
        assert_eq!(task_profile_for_test(201), None);
    }

    #[test]
    fn apparmor_exec_attach_profile_commits_after_successful_exec() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        let policy = br#"profile "/usr/bin/demo" flags=(attach_disconnected) {
  /usr/bin/demo x,
  /etc/** r,
}
"#;
        assert_eq!(load_policy_blob(policy), Ok(policy.len()));

        let mut current = task(300);
        let _guard = CurrentTaskGuard::set(&mut *current as *mut TaskStruct);

        assert_eq!(task_profile_for_test(300), None);
        assert_eq!(apparmor_bprm_creds_for_exec(b"/usr/bin/demo"), 0);
        assert_eq!(task_profile_for_test(300), None);
        assert_eq!(apparmor_path_open(b"/blocked", 0), 0);

        apparmor_bprm_committed_creds(b"/usr/bin/demo");

        assert_eq!(task_profile_for_test(300).as_deref(), Some("/usr/bin/demo"));
        assert_eq!(apparmor_path_open(b"/etc/hostname", 0), 0);
        assert_eq!(apparmor_path_open(b"/blocked", 0), -EACCES);
    }

    fn binary_profile(
        name: &str,
        mode: ProfileMode,
        attach: Option<&str>,
        rules: &[(&str, u32, u32)],
    ) -> Vec<u8> {
        binary_profile_with_data(name, mode, attach, rules, &[])
    }

    fn binary_profile_with_data(
        name: &str,
        mode: ProfileMode,
        attach: Option<&str>,
        rules: &[(&str, u32, u32)],
        data: &[(&str, &[u8])],
    ) -> Vec<u8> {
        let mut out = Vec::new();
        named_u32(&mut out, "version", AA_POLICY_ABI_MAX);
        named_struct_start(&mut out, "profile");
        named_string(&mut out, "name", name);
        named_u32(
            &mut out,
            "mode",
            match mode {
                ProfileMode::Enforce => 0,
                ProfileMode::Complain => 1,
            },
        );
        if let Some(attach) = attach {
            named_string(&mut out, "attach", attach);
        }
        named_array(&mut out, "rules", rules.len() as u16);
        for (path, allow, deny) in rules {
            out.push(AA_EXT_STRUCT);
            named_string(&mut out, "path", path);
            named_u32(&mut out, "allow", *allow);
            named_u32(&mut out, "deny", *deny);
            out.push(AA_EXT_STRUCTEND);
        }
        if !data.is_empty() {
            named_struct_start(&mut out, "data");
            for (key, value) in data {
                raw_string(&mut out, key);
                raw_blob(&mut out, value);
            }
            out.push(AA_EXT_STRUCTEND);
        }
        out.push(AA_EXT_STRUCTEND);
        out
    }

    fn binary_profile_with_file_dfa(
        name: &str,
        namespace: Option<&str>,
        path: &str,
        allow: u32,
        deny: u32,
        accept_index: u32,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        named_u32(&mut out, "version", AA_POLICY_ABI_MAX);
        if let Some(namespace) = namespace {
            named_string(&mut out, "namespace", namespace);
        }
        named_struct_start(&mut out, "profile");
        named_string(&mut out, "name", name);
        named_u32(&mut out, "mode", 0);
        named_struct_start(&mut out, "file");
        named_struct_start(&mut out, "perms");
        named_u32(&mut out, "version", 1);
        out.push(AA_EXT_ARRAY);
        out.extend_from_slice(&2u16.to_le_bytes());
        raw_perm(&mut out, AppArmorPerms::default());
        raw_perm(
            &mut out,
            AppArmorPerms {
                allow,
                deny,
                audit: allow,
                ..AppArmorPerms::default()
            },
        );
        out.push(AA_EXT_ARRAYEND);
        out.push(AA_EXT_STRUCTEND);
        named_blob(&mut out, "aadfa", &exact_path_dfa(path, accept_index));
        named_u32(&mut out, "start", DFA_START);
        named_u32(&mut out, "dfa_start", DFA_START);
        out.push(AA_EXT_STRUCTEND);
        out.push(AA_EXT_STRUCTEND);
        out
    }

    fn raw_perm(out: &mut Vec<u8>, perm: AppArmorPerms) {
        for value in [
            0,
            perm.allow,
            perm.deny,
            perm.subtree,
            perm.cond,
            perm.kill,
            perm.complain,
            perm.prompt,
            perm.audit,
            perm.quiet,
            perm.hide,
            perm.xindex,
            perm.tag,
            perm.label,
        ] {
            out.push(AA_EXT_U32);
            out.extend_from_slice(&value.to_le_bytes());
        }
    }

    fn exact_path_dfa(path: &str, accept_index: u32) -> Vec<u8> {
        let state_count = path.len() + 2;
        let trans_count = state_count * 256;
        let mut accept = alloc::vec![0u32; state_count];
        let mut default = alloc::vec![DFA_NOMATCH; state_count];
        let mut base = alloc::vec![0u32; state_count];
        let mut next = alloc::vec![DFA_NOMATCH; trans_count];
        let mut check = alloc::vec![DFA_NOMATCH; trans_count];
        for state in 0..state_count {
            base[state] = (state * 256) as u32;
            default[state] = DFA_NOMATCH;
        }
        let mut state = DFA_START as usize;
        for byte in path.as_bytes() {
            let next_state = state + 1;
            let pos = base[state] as usize + *byte as usize;
            check[pos] = state as u32;
            next[pos] = next_state as u32;
            state = next_state;
        }
        accept[state] = accept_index;

        let mut out = Vec::new();
        out.extend_from_slice(&YYTH_MAGIC.to_be_bytes());
        out.extend_from_slice(&16u32.to_be_bytes());
        out.extend_from_slice(&0u32.to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes());
        dfa_table32(&mut out, YYTD_ID_ACCEPT, &accept);
        dfa_table32(&mut out, YYTD_ID_BASE, &base);
        dfa_table32(&mut out, YYTD_ID_CHK, &check);
        dfa_table32(&mut out, YYTD_ID_DEF, &default);
        dfa_table32(&mut out, YYTD_ID_NXT, &next);
        out
    }

    fn dfa_table32(out: &mut Vec<u8>, id: u16, values: &[u32]) {
        out.extend_from_slice(&(id + 1).to_be_bytes());
        out.extend_from_slice(&YYTD_DATA32.to_be_bytes());
        out.extend_from_slice(&0u32.to_be_bytes());
        out.extend_from_slice(&(values.len() as u32).to_be_bytes());
        for value in values {
            out.extend_from_slice(&value.to_be_bytes());
        }
        while out.len() % 8 != 0 {
            out.push(0);
        }
    }

    fn named_u32(out: &mut Vec<u8>, name: &str, value: u32) {
        named(out, name);
        out.push(AA_EXT_U32);
        out.extend_from_slice(&value.to_le_bytes());
    }

    fn named_string(out: &mut Vec<u8>, name: &str, value: &str) {
        named(out, name);
        raw_string(out, value);
    }

    fn raw_string(out: &mut Vec<u8>, value: &str) {
        out.push(AA_EXT_STRING);
        let len = value.len() + 1;
        out.extend_from_slice(&(len as u16).to_le_bytes());
        out.extend_from_slice(value.as_bytes());
        out.push(0);
    }

    fn raw_blob(out: &mut Vec<u8>, value: &[u8]) {
        out.push(AA_EXT_BLOB);
        out.extend_from_slice(&(value.len() as u32).to_le_bytes());
        out.extend_from_slice(value);
    }

    fn named_blob(out: &mut Vec<u8>, name: &str, value: &[u8]) {
        named(out, name);
        raw_blob(out, value);
    }

    fn named_struct_start(out: &mut Vec<u8>, name: &str) {
        named(out, name);
        out.push(AA_EXT_STRUCT);
    }

    fn named_array(out: &mut Vec<u8>, name: &str, count: u16) {
        named(out, name);
        out.push(AA_EXT_ARRAY);
        out.extend_from_slice(&count.to_le_bytes());
    }

    fn named(out: &mut Vec<u8>, name: &str) {
        out.push(AA_EXT_NAME);
        let len = name.len() + 1;
        out.extend_from_slice(&(len as u16).to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.push(0);
    }
}
#[path = "apparmor/ipc.rs"]
pub mod ipc;
