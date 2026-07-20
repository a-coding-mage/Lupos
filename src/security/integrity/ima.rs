//! linux-parity: partial
//! linux-source: vendor/linux/security/integrity/ima/ima_init.c
//! test-origin: linux:vendor/linux/security/integrity/ima/ima_init.c
//! Integrity Measurement Architecture boot initialization.
//!
//! Mirrors the early shape of Linux `ima_init()` plus the append-only runtime
//! measurement queue from `ima_queue.c`: detect the TPM, initialize the
//! keyring/crypto/template spine, add the boot aggregate as the first
//! measurement, register the IMA LSM id, and expose measurements via
//! securityfs. The policy parser follows Linux `ima_policy.c` grammar for the
//! measurement/appraisal hooks and rule conditions Lupos can observe, and
//! rejects the old local `path=` shortcut. Signature appraisal follows Linux's
//! `signature_v2_hdr` path from `digsig.c`/`digsig_asymmetric.c`: `.ima`
//! keyring lookup first, `.platform` fallback for kexec, hash-algorithm
//! selection, and RSA/PKCS#1 verification.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::types::{InodePrivate, InodeRef};
use crate::include::uapi::errno::{E2BIG, EACCES, EBADMSG, EINVAL, ENOKEY, ENOPKG};
use crate::security::hooks::{LSM_ID_IMA, LsmHooks, NOOP_HOOKS};
use crate::security::lsm_list::register_lsm;

pub const BOOT_AGGREGATE_NAME: &str = "boot_aggregate";
pub const IMA_KEYRING_NAME: &str = ".ima";
pub const CONFIG_IMA_MEASURE_PCR_IDX: u32 = 10;
pub const IMA_DEFAULT_TEMPLATE: &str = "ima-ng";
pub const IMA_DEFAULT_HASH_ALGO: &str = "sha1";
pub const IMA_SHA1_DIGEST_SIZE: usize = 20;
pub const IMA_SHA256_DIGEST_SIZE: usize = 32;
pub const BOOT_AGGREGATE_DIGEST_SHA1: [u8; IMA_SHA1_DIGEST_SIZE] = [0u8; IMA_SHA1_DIGEST_SIZE];
pub const IMA_POLICY_MAX_BYTES: usize = 8192;
pub const IMA_XATTR_DIGEST: u8 = 0x01;
pub const EVM_IMA_XATTR_DIGSIG: u8 = 0x03;
pub const IMA_XATTR_DIGEST_NG: u8 = 0x04;
pub const IMA_VERITY_DIGSIG: u8 = 0x06;
pub const HASH_ALGO_SHA1: u8 = 2;
pub const HASH_ALGO_SHA256: u8 = 4;
pub const HASH_ALGO_LAST: u8 = 23;
pub const SIGNATURE_V2_HDR_SIZE: usize = 9;
pub const IMA_DIGEST_XATTR_SIZE: usize = 1 + IMA_SHA1_DIGEST_SIZE;
pub const IMA_DIGEST_NG_XATTR_SIZE: usize = 2 + IMA_SHA1_DIGEST_SIZE;
pub const IMA_MAY_EXEC: u32 = 0x0000_0001;
pub const IMA_MAY_WRITE: u32 = 0x0000_0002;
pub const IMA_MAY_READ: u32 = 0x0000_0004;
pub const IMA_MAY_APPEND: u32 = 0x0000_0008;
const VOLATILE_PATH_PREFIXES: [&str; 5] = ["/dev", "/proc", "/run", "/sys", "/tmp"];

static INITIALIZED: AtomicBool = AtomicBool::new(false);
static TPM_BYPASS: AtomicBool = AtomicBool::new(false);
static BOOT_AGGREGATE_PRESENT: AtomicBool = AtomicBool::new(false);
static MEASUREMENT_COUNT: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static FILE_HOOK_MEASUREMENTS_ENABLED: AtomicBool = AtomicBool::new(false);

lazy_static! {
    static ref MEASUREMENTS: Mutex<Vec<ImaMeasurement>> = Mutex::new(Vec::new());
    static ref POLICY: Mutex<ImaPolicy> = Mutex::new(ImaPolicy::default());
    static ref IMA_KEYRING_ID: Mutex<Option<i32>> = Mutex::new(None);
}

pub const HOOKS: LsmHooks = LsmHooks {
    name: "ima",
    id: LSM_ID_IMA,
    ..NOOP_HOOKS
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct ImaMeasurement {
    pcr: u32,
    digest: [u8; IMA_SHA1_DIGEST_SIZE],
    template: &'static str,
    algo: &'static str,
    name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImaHook {
    BprmCheck,
    CredsCheck,
    CriticalData,
    FileCheck,
    FirmwareCheck,
    KexecCmdline,
    KexecInitramfsCheck,
    KexecKernelCheck,
    KeyCheck,
    MmapCheck,
    MmapCheckReqprot,
    ModuleCheck,
    PolicyCheck,
    SetxattrCheck,
}

impl ImaHook {
    fn parse(value: &str) -> Result<Self, i32> {
        match value {
            "BPRM_CHECK" => Ok(Self::BprmCheck),
            "CREDS_CHECK" => Ok(Self::CredsCheck),
            "CRITICAL_DATA" => Ok(Self::CriticalData),
            "FILE_CHECK" => Ok(Self::FileCheck),
            "FIRMWARE_CHECK" => Ok(Self::FirmwareCheck),
            "KEXEC_CMDLINE" => Ok(Self::KexecCmdline),
            "KEXEC_INITRAMFS_CHECK" => Ok(Self::KexecInitramfsCheck),
            "KEXEC_KERNEL_CHECK" => Ok(Self::KexecKernelCheck),
            "KEY_CHECK" => Ok(Self::KeyCheck),
            "MMAP_CHECK" | "FILE_MMAP" => Ok(Self::MmapCheck),
            "MMAP_CHECK_REQPROT" => Ok(Self::MmapCheckReqprot),
            "MODULE_CHECK" => Ok(Self::ModuleCheck),
            "PATH_CHECK" => Ok(Self::FileCheck),
            "POLICY_CHECK" => Ok(Self::PolicyCheck),
            "SETXATTR_CHECK" => Ok(Self::SetxattrCheck),
            _ => Err(-EINVAL),
        }
    }

    fn as_policy_str(self) -> &'static str {
        match self {
            Self::BprmCheck => "BPRM_CHECK",
            Self::CredsCheck => "CREDS_CHECK",
            Self::CriticalData => "CRITICAL_DATA",
            Self::FileCheck => "FILE_CHECK",
            Self::FirmwareCheck => "FIRMWARE_CHECK",
            Self::KexecCmdline => "KEXEC_CMDLINE",
            Self::KexecInitramfsCheck => "KEXEC_INITRAMFS_CHECK",
            Self::KexecKernelCheck => "KEXEC_KERNEL_CHECK",
            Self::KeyCheck => "KEY_CHECK",
            Self::MmapCheck => "MMAP_CHECK",
            Self::MmapCheckReqprot => "MMAP_CHECK_REQPROT",
            Self::ModuleCheck => "MODULE_CHECK",
            Self::PolicyCheck => "POLICY_CHECK",
            Self::SetxattrCheck => "SETXATTR_CHECK",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ImaPolicyAction {
    Measure,
    DontMeasure,
    Appraise,
    DontAppraise,
    Audit,
    DontAudit,
    Hash,
    DontHash,
}

impl ImaPolicyAction {
    fn parse(value: &str) -> Result<Self, i32> {
        match value {
            "measure" => Ok(Self::Measure),
            "dont_measure" => Ok(Self::DontMeasure),
            "appraise" => Ok(Self::Appraise),
            "dont_appraise" => Ok(Self::DontAppraise),
            "audit" => Ok(Self::Audit),
            "dont_audit" => Ok(Self::DontAudit),
            "hash" => Ok(Self::Hash),
            "dont_hash" => Ok(Self::DontHash),
            _ => Err(-EINVAL),
        }
    }

    fn as_policy_str(self) -> &'static str {
        match self {
            Self::Measure => "measure",
            Self::DontMeasure => "dont_measure",
            Self::Appraise => "appraise",
            Self::DontAppraise => "dont_appraise",
            Self::Audit => "audit",
            Self::DontAudit => "dont_audit",
            Self::Hash => "hash",
            Self::DontHash => "dont_hash",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ImaPolicyRule {
    action: ImaPolicyAction,
    func: Option<ImaHook>,
    mask: Option<ImaPolicyMask>,
    fsmagic: Option<u64>,
    fsname: Option<String>,
    fs_subtype: Option<String>,
    fsuuid: Option<String>,
    uid: Option<ImaNumericCondition>,
    euid: Option<ImaNumericCondition>,
    gid: Option<ImaNumericCondition>,
    egid: Option<ImaNumericCondition>,
    fowner: Option<ImaNumericCondition>,
    fgroup: Option<ImaNumericCondition>,
    obj_user: Option<String>,
    obj_role: Option<String>,
    obj_type: Option<String>,
    subj_user: Option<String>,
    subj_role: Option<String>,
    subj_type: Option<String>,
    keyrings: Option<ImaPolicyList>,
    label: Option<ImaPolicyList>,
    digest_type: Option<ImaDigestType>,
    appraise_type: Option<ImaAppraiseType>,
    appraise_algos: Option<ImaHashAlgoSet>,
    permit_directio: bool,
    pcr: Option<u32>,
    template: Option<String>,
}

impl ImaPolicyRule {
    fn matches(&self, hook: ImaHook, mask: u32, context: &ImaPolicyContext<'_>) -> bool {
        if self.func.is_some_and(|func| func != hook) {
            return false;
        }
        if let Some(rule_mask) = self.mask
            && !rule_mask.matches(mask)
        {
            return false;
        }
        if self
            .fsmagic
            .is_some_and(|fsmagic| fsmagic != context.fs_magic)
        {
            return false;
        }
        if self
            .fsname
            .as_ref()
            .is_some_and(|fsname| fsname.as_str() != context.fs_name)
        {
            return false;
        }
        if self.fs_subtype.as_ref().is_some_and(|fs_subtype| {
            context
                .fs_subtype
                .is_none_or(|context_subtype| fs_subtype.as_str() != context_subtype)
        }) {
            return false;
        }
        if self.fsuuid.as_ref().is_some_and(|fsuuid| {
            context
                .fsuuid
                .is_none_or(|context_uuid| !fsuuid.eq_ignore_ascii_case(context_uuid))
        }) {
            return false;
        }
        if !numeric_matches(self.uid, context.uid)
            || !numeric_matches(self.euid, context.euid)
            || !numeric_matches(self.gid, context.gid)
            || !numeric_matches(self.egid, context.egid)
            || !numeric_matches(self.fowner, context.fowner)
            || !numeric_matches(self.fgroup, context.fgroup)
        {
            return false;
        }
        if !string_matches(self.obj_user.as_deref(), context.obj_user)
            || !string_matches(self.obj_role.as_deref(), context.obj_role)
            || !string_matches(self.obj_type.as_deref(), context.obj_type)
            || !string_matches(self.subj_user.as_deref(), context.subj_user)
            || !string_matches(self.subj_role.as_deref(), context.subj_role)
            || !string_matches(self.subj_type.as_deref(), context.subj_type)
        {
            return false;
        }
        match hook {
            ImaHook::KeyCheck => self.keyrings.as_ref().is_none_or(|keyrings| {
                context
                    .func_data
                    .is_some_and(|data| keyrings.contains(data))
            }),
            ImaHook::CriticalData => self
                .label
                .as_ref()
                .is_none_or(|label| context.func_data.is_some_and(|data| label.contains(data))),
            _ => self.keyrings.is_none() && self.label.is_none(),
        }
    }

    fn to_policy_line(&self) -> String {
        let mut out = String::from(self.action.as_policy_str());
        if let Some(func) = self.func {
            out.push_str(" func=");
            out.push_str(func.as_policy_str());
        }
        if let Some(mask) = self.mask {
            out.push_str(" mask=");
            out.push_str(mask.as_policy_str());
        }
        if let Some(fsmagic) = self.fsmagic {
            out.push_str(" fsmagic=0x");
            out.push_str(&format!("{fsmagic:x}"));
        }
        if let Some(fsname) = self.fsname.as_ref() {
            out.push_str(" fsname=");
            out.push_str(fsname);
        }
        if let Some(fs_subtype) = self.fs_subtype.as_ref() {
            out.push_str(" fs_subtype=");
            out.push_str(fs_subtype);
        }
        if let Some(keyrings) = self.keyrings.as_ref() {
            out.push_str(" keyrings=");
            out.push_str(&keyrings.as_policy_str());
        }
        if let Some(label) = self.label.as_ref() {
            out.push_str(" label=");
            out.push_str(&label.as_policy_str());
        }
        if let Some(pcr) = self.pcr {
            out.push_str(" pcr=");
            out.push_str(&format!("{pcr}"));
        }
        if let Some(fsuuid) = self.fsuuid.as_ref() {
            out.push_str(" fsuuid=");
            out.push_str(fsuuid);
        }
        append_numeric(&mut out, "uid", self.uid);
        append_numeric(&mut out, "euid", self.euid);
        append_numeric(&mut out, "gid", self.gid);
        append_numeric(&mut out, "egid", self.egid);
        append_numeric(&mut out, "fowner", self.fowner);
        append_numeric(&mut out, "fgroup", self.fgroup);
        if let Some(appraise_algos) = self.appraise_algos.as_ref() {
            out.push_str(" appraise_algos=");
            out.push_str(&appraise_algos.as_policy_str());
        }
        append_string(&mut out, "obj_user", self.obj_user.as_deref());
        append_string(&mut out, "obj_role", self.obj_role.as_deref());
        append_string(&mut out, "obj_type", self.obj_type.as_deref());
        append_string(&mut out, "subj_user", self.subj_user.as_deref());
        append_string(&mut out, "subj_role", self.subj_role.as_deref());
        append_string(&mut out, "subj_type", self.subj_type.as_deref());
        if let Some(template) = self.template.as_ref() {
            out.push_str(" template=");
            out.push_str(template);
        }
        if let Some(appraise_type) = self.appraise_type {
            out.push_str(" appraise_type=");
            out.push_str(appraise_type.as_policy_str());
        }
        if let Some(digest_type) = self.digest_type {
            out.push_str(" digest_type=");
            out.push_str(digest_type.as_policy_str());
        }
        if self.permit_directio {
            out.push_str(" permit_directio");
        }
        out
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ImaNumericOp {
    Eq,
    Gt,
    Lt,
}

impl ImaNumericOp {
    fn policy_operator(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Gt => ">",
            Self::Lt => "<",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ImaNumericCondition {
    op: ImaNumericOp,
    value: u32,
}

impl ImaNumericCondition {
    fn matches(self, actual: u32) -> bool {
        match self.op {
            ImaNumericOp::Eq => actual == self.value,
            ImaNumericOp::Gt => actual > self.value,
            ImaNumericOp::Lt => actual < self.value,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ImaPolicyList {
    items: Vec<String>,
}

impl ImaPolicyList {
    fn parse(value: &str) -> Result<Self, i32> {
        let mut items = Vec::new();
        for item in value.split('|') {
            if item.is_empty() {
                return Err(-EINVAL);
            }
            items.push(String::from(item));
        }
        if items.is_empty() {
            return Err(-EINVAL);
        }
        Ok(Self { items })
    }

    fn contains(&self, needle: &str) -> bool {
        self.items.iter().any(|item| item == needle)
    }

    fn as_policy_str(&self) -> String {
        self.items.join("|")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ImaDigestType {
    Verity,
}

impl ImaDigestType {
    fn parse(value: &str) -> Result<Self, i32> {
        match value {
            "verity" => Ok(Self::Verity),
            _ => Err(-EINVAL),
        }
    }

    fn as_policy_str(self) -> &'static str {
        match self {
            Self::Verity => "verity",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ImaHashAlgoSet {
    algos: Vec<u8>,
}

impl ImaHashAlgoSet {
    fn parse(value: &str) -> Result<Self, i32> {
        let mut algos = Vec::new();
        for name in value.split(',') {
            let Some(algo) = hash_algo_by_name(name) else {
                return Err(-EINVAL);
            };
            if !algos.contains(&algo) {
                algos.push(algo);
            }
        }
        if algos.is_empty() {
            return Err(-EINVAL);
        }
        Ok(Self { algos })
    }

    fn as_policy_str(&self) -> String {
        let mut out = String::new();
        for (idx, algo) in self.algos.iter().enumerate() {
            if idx != 0 {
                out.push(',');
            }
            out.push_str(hash_algo_name(*algo).unwrap_or("unknown"));
        }
        out
    }
}

#[derive(Clone, Debug)]
struct ImaPolicyContext<'a> {
    fs_magic: u64,
    fs_name: &'a str,
    fs_subtype: Option<&'a str>,
    fsuuid: Option<&'a str>,
    uid: u32,
    euid: u32,
    gid: u32,
    egid: u32,
    fowner: u32,
    fgroup: u32,
    obj_user: Option<&'a str>,
    obj_role: Option<&'a str>,
    obj_type: Option<&'a str>,
    subj_user: Option<&'a str>,
    subj_role: Option<&'a str>,
    subj_type: Option<&'a str>,
    func_data: Option<&'a str>,
}

impl<'a> ImaPolicyContext<'a> {
    fn for_path(path: &'a str) -> Self {
        let (fs_magic, fs_name) = infer_fs_for_path(path);
        Self {
            fs_magic,
            fs_name,
            fs_subtype: None,
            fsuuid: None,
            uid: 0,
            euid: 0,
            gid: 0,
            egid: 0,
            fowner: 0,
            fgroup: 0,
            obj_user: None,
            obj_role: None,
            obj_type: None,
            subj_user: None,
            subj_role: None,
            subj_type: None,
            func_data: None,
        }
    }
}

const PROC_SUPER_MAGIC: u64 = 0x9fa0;
const SYSFS_MAGIC: u64 = 0x6265_6572;
const SECURITYFS_MAGIC: u64 = 0x7363_6673;
const TMPFS_MAGIC: u64 = 0x0102_1994;
const RAMFS_MAGIC: u64 = 0x8584_58f6;

fn infer_fs_for_path(path: &str) -> (u64, &'static str) {
    if path == "/proc" || path.starts_with("/proc/") {
        (PROC_SUPER_MAGIC, "proc")
    } else if path == "/sys/kernel/security" || path.starts_with("/sys/kernel/security/") {
        (SECURITYFS_MAGIC, "securityfs")
    } else if path == "/sys" || path.starts_with("/sys/") {
        (SYSFS_MAGIC, "sysfs")
    } else if path == "/dev"
        || path.starts_with("/dev/")
        || path == "/run"
        || path.starts_with("/run/")
    {
        (TMPFS_MAGIC, "tmpfs")
    } else {
        (RAMFS_MAGIC, "rootfs")
    }
}

fn numeric_matches(condition: Option<ImaNumericCondition>, actual: u32) -> bool {
    condition.is_none_or(|condition| condition.matches(actual))
}

fn string_matches(condition: Option<&str>, actual: Option<&str>) -> bool {
    condition.is_none_or(|condition| actual.is_some_and(|actual| actual == condition))
}

fn append_string(out: &mut String, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        out.push(' ');
        out.push_str(key);
        out.push('=');
        out.push_str(value);
    }
}

fn append_numeric(out: &mut String, key: &str, condition: Option<ImaNumericCondition>) {
    if let Some(condition) = condition {
        out.push(' ');
        out.push_str(key);
        out.push_str(condition.op.policy_operator());
        out.push_str(&format!("{}", condition.value));
    }
}

fn hash_algo_by_name(name: &str) -> Option<u8> {
    match name {
        "md4" => Some(0),
        "md5" => Some(1),
        "sha1" => Some(HASH_ALGO_SHA1),
        "rmd160" => Some(3),
        "sha256" => Some(HASH_ALGO_SHA256),
        "sha384" => Some(5),
        "sha512" => Some(6),
        "sha224" => Some(7),
        "rmd128" => Some(8),
        "rmd256" => Some(9),
        "rmd320" => Some(10),
        "wp256" => Some(11),
        "wp384" => Some(12),
        "wp512" => Some(13),
        "tgr128" => Some(14),
        "tgr160" => Some(15),
        "tgr192" => Some(16),
        "sm3" => Some(17),
        "streebog256" => Some(18),
        "streebog512" => Some(19),
        "sha3-256" => Some(20),
        "sha3-384" => Some(21),
        "sha3-512" => Some(22),
        _ => None,
    }
}

fn hash_algo_name(algo: u8) -> Option<&'static str> {
    match algo {
        0 => Some("md4"),
        1 => Some("md5"),
        HASH_ALGO_SHA1 => Some("sha1"),
        3 => Some("rmd160"),
        HASH_ALGO_SHA256 => Some("sha256"),
        5 => Some("sha384"),
        6 => Some("sha512"),
        7 => Some("sha224"),
        8 => Some("rmd128"),
        9 => Some("rmd256"),
        10 => Some("rmd320"),
        11 => Some("wp256"),
        12 => Some("wp384"),
        13 => Some("wp512"),
        14 => Some("tgr128"),
        15 => Some("tgr160"),
        16 => Some("tgr192"),
        17 => Some("sm3"),
        18 => Some("streebog256"),
        19 => Some("streebog512"),
        20 => Some("sha3-256"),
        21 => Some("sha3-384"),
        22 => Some("sha3-512"),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ImaAppraiseType {
    ImaSig,
    ImaSigOrModSig,
    SigV3,
}

impl ImaAppraiseType {
    fn parse(value: &str) -> Result<Self, i32> {
        match value {
            "imasig" => Ok(Self::ImaSig),
            "imasig|modsig" => Ok(Self::ImaSigOrModSig),
            "sigv3" => Ok(Self::SigV3),
            _ => Err(-EINVAL),
        }
    }

    fn as_policy_str(self) -> &'static str {
        match self {
            Self::ImaSig => "imasig",
            Self::ImaSigOrModSig => "imasig|modsig",
            Self::SigV3 => "sigv3",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ImaPolicyMask {
    value: u32,
    in_mask: bool,
}

impl ImaPolicyMask {
    fn parse(value: &str) -> Result<Self, i32> {
        let (in_mask, value) = value
            .strip_prefix('^')
            .map_or((false, value), |value| (true, value));
        let value = match value {
            "MAY_EXEC" => IMA_MAY_EXEC,
            "MAY_WRITE" => IMA_MAY_WRITE,
            "MAY_READ" => IMA_MAY_READ,
            "MAY_APPEND" => IMA_MAY_APPEND,
            _ => return Err(-EINVAL),
        };
        Ok(Self { value, in_mask })
    }

    fn matches(self, requested_mask: u32) -> bool {
        if self.in_mask {
            self.value & requested_mask != 0
        } else {
            self.value == requested_mask
        }
    }

    fn as_policy_str(self) -> &'static str {
        match (self.in_mask, self.value) {
            (false, IMA_MAY_EXEC) => "MAY_EXEC",
            (false, IMA_MAY_WRITE) => "MAY_WRITE",
            (false, IMA_MAY_READ) => "MAY_READ",
            (false, IMA_MAY_APPEND) => "MAY_APPEND",
            (true, IMA_MAY_EXEC) => "^MAY_EXEC",
            (true, IMA_MAY_WRITE) => "^MAY_WRITE",
            (true, IMA_MAY_READ) => "^MAY_READ",
            (true, IMA_MAY_APPEND) => "^MAY_APPEND",
            _ => "MAY_READ",
        }
    }
}

#[derive(Default)]
struct ImaPolicy {
    rules: Vec<ImaPolicyRule>,
    text: String,
}

impl ImaMeasurement {
    fn boot_aggregate() -> Self {
        Self {
            pcr: CONFIG_IMA_MEASURE_PCR_IDX,
            digest: BOOT_AGGREGATE_DIGEST_SHA1,
            template: IMA_DEFAULT_TEMPLATE,
            algo: IMA_DEFAULT_HASH_ALGO,
            name: String::from(BOOT_AGGREGATE_NAME),
        }
    }

    fn file(name: &str, digest: [u8; IMA_SHA1_DIGEST_SIZE]) -> Self {
        Self {
            pcr: CONFIG_IMA_MEASURE_PCR_IDX,
            digest,
            template: IMA_DEFAULT_TEMPLATE,
            algo: IMA_DEFAULT_HASH_ALGO,
            name: if name.is_empty() {
                String::from("(unknown)")
            } else {
                String::from(name)
            },
        }
    }

    fn is_boot_aggregate(&self) -> bool {
        self.pcr == CONFIG_IMA_MEASURE_PCR_IDX
            && self.digest == BOOT_AGGREGATE_DIGEST_SHA1
            && self.name == BOOT_AGGREGATE_NAME
    }

    fn ascii_row(&self) -> String {
        let digest = digest_hex(&self.digest);
        format!(
            "{:2} {} {} {}:{} {}\n",
            self.pcr, digest, self.template, self.algo, digest, self.name
        )
    }

    fn append_binary(&self, out: &mut Vec<u8>) {
        let template = self.template.as_bytes();
        let event = format!("{}:{} {}", self.algo, digest_hex(&self.digest), self.name);
        let event = event.as_bytes();

        out.extend_from_slice(&self.pcr.to_le_bytes());
        out.extend_from_slice(&self.digest);
        out.extend_from_slice(&(template.len() as u32).to_le_bytes());
        out.extend_from_slice(template);
        out.extend_from_slice(&(event.len() as u32).to_le_bytes());
        out.extend_from_slice(event);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImaState {
    pub initialized: bool,
    pub tpm_bypass: bool,
    pub boot_aggregate_present: bool,
    pub measurement_count: usize,
    pub measure_pcr_idx: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImaAppraisalStatus {
    Pass,
    PassImmutable,
    Fail,
    NoLabel,
    NoXattrs,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImaAppraisalResult {
    pub status: ImaAppraisalStatus,
    pub cause: &'static str,
}

impl ImaAppraisalResult {
    const fn pass(cause: &'static str) -> Self {
        Self {
            status: ImaAppraisalStatus::Pass,
            cause,
        }
    }

    const fn status(status: ImaAppraisalStatus, cause: &'static str) -> Self {
        Self { status, cause }
    }

    pub fn is_pass(self) -> bool {
        matches!(
            self.status,
            ImaAppraisalStatus::Pass | ImaAppraisalStatus::PassImmutable
        )
    }
}

fn tpm_default_chip_present() -> bool {
    false
}

fn sync_measurement_state_locked(measurements: &[ImaMeasurement]) {
    BOOT_AGGREGATE_PRESENT.store(
        measurements.iter().any(ImaMeasurement::is_boot_aggregate),
        Ordering::Release,
    );
    MEASUREMENT_COUNT.store(measurements.len(), Ordering::Release);
}

fn ensure_boot_aggregate_locked(measurements: &mut Vec<ImaMeasurement>) {
    if !measurements.iter().any(ImaMeasurement::is_boot_aggregate) {
        measurements.insert(0, ImaMeasurement::boot_aggregate());
    }
    sync_measurement_state_locked(measurements);
}

fn ensure_ima_keyring() -> Result<i32, i32> {
    crate::security::keys::init();
    if let Some(id) = *IMA_KEYRING_ID.lock() {
        return Ok(id);
    }
    if let Some(id) = crate::security::keys::keyring_id_by_description(IMA_KEYRING_NAME) {
        *IMA_KEYRING_ID.lock() = Some(id);
        return Ok(id);
    }
    let id = crate::security::keys::add_key("keyring", IMA_KEYRING_NAME, &[]);
    if id < 0 {
        return Err(id);
    }
    *IMA_KEYRING_ID.lock() = Some(id);
    Ok(id)
}

pub fn ima_keyring_id() -> Option<i32> {
    *IMA_KEYRING_ID.lock()
}

pub fn init() {
    let _ = register_lsm(HOOKS);

    if INITIALIZED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    let _ = ensure_ima_keyring();

    if !tpm_default_chip_present() {
        TPM_BYPASS.store(true, Ordering::Release);
        crate::kernel::printk::log_info!("ima", "No TPM chip found, activating TPM-bypass!");
    }

    {
        let mut measurements = MEASUREMENTS.lock();
        ensure_boot_aggregate_locked(&mut measurements);
    }
    crate::security::integrity::ima_fs::init_securityfs();
}

pub fn snapshot() -> ImaState {
    ImaState {
        initialized: INITIALIZED.load(Ordering::Acquire),
        tpm_bypass: TPM_BYPASS.load(Ordering::Acquire),
        boot_aggregate_present: BOOT_AGGREGATE_PRESENT.load(Ordering::Acquire),
        measurement_count: MEASUREMENT_COUNT.load(Ordering::Acquire),
        measure_pcr_idx: CONFIG_IMA_MEASURE_PCR_IDX,
    }
}

pub fn runtime_measurements_count() -> usize {
    MEASUREMENT_COUNT.load(Ordering::Acquire)
}

pub fn runtime_violations() -> usize {
    0
}

fn default_should_measure_path(path: &str) -> bool {
    if path.is_empty() || !path.starts_with('/') {
        return false;
    }
    !VOLATILE_PATH_PREFIXES.iter().any(|prefix| {
        path == *prefix
            || path.as_bytes().get(prefix.len()) == Some(&b'/') && path.starts_with(prefix)
    })
}

fn default_should_measure_path_context(context: &ImaPolicyContext<'_>) -> bool {
    !matches!(
        context.fs_magic,
        PROC_SUPER_MAGIC | SYSFS_MAGIC | SECURITYFS_MAGIC | TMPFS_MAGIC
    )
}

fn default_mask_for_hook(hook: ImaHook) -> u32 {
    match hook {
        ImaHook::BprmCheck
        | ImaHook::FirmwareCheck
        | ImaHook::KexecInitramfsCheck
        | ImaHook::KexecKernelCheck
        | ImaHook::MmapCheck
        | ImaHook::MmapCheckReqprot
        | ImaHook::ModuleCheck
        | ImaHook::PolicyCheck => IMA_MAY_EXEC,
        ImaHook::FileCheck => IMA_MAY_READ,
        _ => 0,
    }
}

fn policy_allows_measurement(hook: ImaHook, mask: u32, path: &str) -> bool {
    let context = ImaPolicyContext::for_path(path);
    policy_allows_measurement_context(hook, mask, &context)
}

fn policy_allows_measurement_context(
    hook: ImaHook,
    mask: u32,
    context: &ImaPolicyContext<'_>,
) -> bool {
    let policy = POLICY.lock();
    // vendor/linux/security/integrity/ima/ima_main.c short-circuits file
    // hooks while ima_policy_flag is zero. An empty policy therefore means
    // "no measurement", not an implicit measure-all policy.
    if policy.rules.is_empty() {
        return false;
    }
    for rule in policy.rules.iter() {
        if !matches!(
            rule.action,
            ImaPolicyAction::Measure | ImaPolicyAction::DontMeasure
        ) {
            continue;
        }
        if rule.matches(hook, mask, context) {
            return matches!(rule.action, ImaPolicyAction::Measure);
        }
    }
    false
}

fn policy_requires_appraisal(hook: ImaHook, mask: u32, path: &str) -> bool {
    let context = ImaPolicyContext::for_path(path);
    policy_appraisal_for_context(hook, mask, &context).is_some_and(|appraisal| appraisal.required)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ImaPolicyAppraisal {
    required: bool,
    appraise_type: Option<ImaAppraiseType>,
}

fn policy_appraisal_for(hook: ImaHook, mask: u32, path: &str) -> Option<ImaPolicyAppraisal> {
    let context = ImaPolicyContext::for_path(path);
    policy_appraisal_for_context(hook, mask, &context)
}

fn policy_appraisal_for_context(
    hook: ImaHook,
    mask: u32,
    context: &ImaPolicyContext<'_>,
) -> Option<ImaPolicyAppraisal> {
    let policy = POLICY.lock();
    for rule in policy.rules.iter() {
        if !matches!(
            rule.action,
            ImaPolicyAction::Appraise | ImaPolicyAction::DontAppraise
        ) {
            continue;
        }
        if rule.matches(hook, mask, context) {
            return Some(ImaPolicyAppraisal {
                required: matches!(rule.action, ImaPolicyAction::Appraise),
                appraise_type: if matches!(rule.action, ImaPolicyAction::Appraise) {
                    rule.appraise_type
                } else {
                    None
                },
            });
        }
    }
    None
}

pub fn measure_file(name: &str, bytes: &[u8]) -> Result<bool, i32> {
    if !INITIALIZED.load(Ordering::Acquire) {
        return Ok(false);
    }

    let digest = sha1_digest(bytes);
    let mut measurements = MEASUREMENTS.lock();
    ensure_boot_aggregate_locked(&mut measurements);

    if measurements.iter().any(|measurement| {
        measurement.pcr == CONFIG_IMA_MEASURE_PCR_IDX && measurement.digest == digest
    }) {
        sync_measurement_state_locked(&measurements);
        return Ok(false);
    }

    measurements.push(ImaMeasurement::file(name, digest));
    sync_measurement_state_locked(&measurements);
    Ok(true)
}

pub fn should_measure_path(path: &str) -> bool {
    default_should_measure_path(path)
}

pub fn should_appraise_path_for_hook(hook: ImaHook, path: &str) -> bool {
    should_appraise_path_for_hook_mask(hook, default_mask_for_hook(hook), path)
}

pub fn should_appraise_path_for_hook_mask(hook: ImaHook, mask: u32, path: &str) -> bool {
    policy_requires_appraisal(hook, mask, path)
}

pub fn measure_file_for_hook(hook: ImaHook, name: &str, bytes: &[u8]) -> Result<bool, i32> {
    measure_file_for_hook_mask(hook, default_mask_for_hook(hook), name, bytes)
}

pub fn measure_file_for_hook_mask(
    hook: ImaHook,
    mask: u32,
    name: &str,
    bytes: &[u8],
) -> Result<bool, i32> {
    if !policy_allows_measurement(hook, mask, name) {
        return Ok(false);
    }
    measure_file(name, bytes)
}

pub fn measure_buffer_for_keyring(
    eventname: &str,
    keyring: &str,
    bytes: &[u8],
) -> Result<bool, i32> {
    let context = ImaPolicyContext {
        fs_magic: RAMFS_MAGIC,
        fs_name: "rootfs",
        fs_subtype: None,
        fsuuid: None,
        uid: 0,
        euid: 0,
        gid: 0,
        egid: 0,
        fowner: 0,
        fgroup: 0,
        obj_user: None,
        obj_role: None,
        obj_type: None,
        subj_user: None,
        subj_role: None,
        subj_type: None,
        func_data: Some(keyring),
    };
    if !policy_allows_measurement_context(ImaHook::KeyCheck, 0, &context) {
        return Ok(false);
    }
    measure_file(eventname, bytes)
}

pub fn measure_inode_private_for_hook(
    hook: ImaHook,
    name: &str,
    private: &InodePrivate,
) -> Result<bool, i32> {
    measure_inode_private_for_hook_mask(hook, default_mask_for_hook(hook), name, private)
}

pub fn measure_inode_private_for_hook_mask(
    hook: ImaHook,
    mask: u32,
    name: &str,
    private: &InodePrivate,
) -> Result<bool, i32> {
    if !policy_allows_measurement(hook, mask, name) {
        return Ok(false);
    }

    match private {
        InodePrivate::RamBytes(bytes) => {
            let bytes = bytes.lock();
            measure_file(name, &bytes)
        }
        InodePrivate::StaticBytes(bytes) => measure_file(name, bytes),
        InodePrivate::StaticCowBytes { base, overlay } => {
            let overlay = overlay.lock();
            if let Some(bytes) = overlay.as_ref() {
                measure_file(name, bytes)
            } else {
                measure_file(name, base)
            }
        }
        _ => Ok(false),
    }
}

pub fn measure_inode_private(name: &str, private: &InodePrivate) -> Result<bool, i32> {
    measure_inode_private_for_hook(ImaHook::FileCheck, name, private)
}

fn file_hook_measurements_enabled() -> bool {
    #[cfg(test)]
    {
        FILE_HOOK_MEASUREMENTS_ENABLED.load(Ordering::Acquire)
    }
    #[cfg(not(test))]
    {
        true
    }
}

pub fn measure_opened_inode(name: &str, inode: &InodeRef) -> Result<bool, i32> {
    if !file_hook_measurements_enabled() || !inode.is_reg() {
        return Ok(false);
    }
    measure_inode_private_for_hook(ImaHook::FileCheck, name, &inode.private)
}

pub fn measure_mapped_inode(name: &str, inode: &InodeRef) -> Result<bool, i32> {
    if !file_hook_measurements_enabled() || !inode.is_reg() {
        return Ok(false);
    }
    measure_inode_private_for_hook(ImaHook::MmapCheck, name, &inode.private)
}

pub fn ascii_runtime_measurements_sha1() -> String {
    let measurements = MEASUREMENTS.lock();
    let mut out = String::new();
    for measurement in measurements.iter() {
        out.push_str(&measurement.ascii_row());
    }
    out
}

pub fn binary_runtime_measurements_sha1() -> Vec<u8> {
    let mut out = Vec::new();
    let measurements = MEASUREMENTS.lock();
    for measurement in measurements.iter() {
        measurement.append_binary(&mut out);
    }
    out
}

pub fn policy_text() -> String {
    POLICY.lock().text.clone()
}

pub fn policy_rule_count() -> usize {
    POLICY.lock().rules.len()
}

pub fn load_policy(bytes: &[u8]) -> Result<usize, i32> {
    if bytes.len() > IMA_POLICY_MAX_BYTES {
        return Err(-E2BIG);
    }
    let (rules, text) = parse_policy(bytes)?;
    let mut policy = POLICY.lock();
    policy.rules = rules;
    policy.text = text;
    Ok(bytes.len())
}

pub fn build_digest_xattr(bytes: &[u8]) -> [u8; IMA_DIGEST_NG_XATTR_SIZE] {
    let digest = sha1_digest(bytes);
    let mut out = [0u8; IMA_DIGEST_NG_XATTR_SIZE];
    out[0] = IMA_XATTR_DIGEST_NG;
    out[1] = HASH_ALGO_SHA1;
    out[2..].copy_from_slice(&digest);
    out
}

pub fn build_legacy_digest_xattr(bytes: &[u8]) -> [u8; IMA_DIGEST_XATTR_SIZE] {
    let digest = sha1_digest(bytes);
    let mut out = [0u8; IMA_DIGEST_XATTR_SIZE];
    out[0] = IMA_XATTR_DIGEST;
    out[1..].copy_from_slice(&digest);
    out
}

pub fn build_sha256_digest_xattr(bytes: &[u8]) -> [u8; 2 + IMA_SHA256_DIGEST_SIZE] {
    let digest = sha256_digest(bytes);
    let mut out = [0u8; 2 + IMA_SHA256_DIGEST_SIZE];
    out[0] = IMA_XATTR_DIGEST_NG;
    out[1] = HASH_ALGO_SHA256;
    out[2..].copy_from_slice(&digest);
    out
}

pub(crate) fn digest_for_algo(algo: u8, bytes: &[u8]) -> Result<Vec<u8>, i32> {
    match algo {
        HASH_ALGO_SHA1 => Ok(sha1_digest(bytes).to_vec()),
        HASH_ALGO_SHA256 => Ok(sha256_digest(bytes).to_vec()),
        _ => Err(-ENOPKG),
    }
}

fn verify_digest_xattr(hook: ImaHook, bytes: &[u8], xattr: &[u8]) -> ImaAppraisalResult {
    match xattr.first().copied() {
        Some(IMA_XATTR_DIGEST) => {
            let digest = sha1_digest(bytes);
            if xattr.len() < IMA_DIGEST_XATTR_SIZE {
                return ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-hash");
            }
            if xattr[1..1 + IMA_SHA1_DIGEST_SIZE] == digest {
                ImaAppraisalResult::pass("valid-hash")
            } else {
                ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-hash")
            }
        }
        Some(IMA_XATTR_DIGEST_NG) => {
            let Some(algo) = xattr.get(1).copied() else {
                return ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-hash");
            };
            match digest_for_algo(algo, bytes) {
                Ok(digest) if xattr.len() >= 2 + digest.len() => {
                    if xattr[2..2 + digest.len()] == digest {
                        ImaAppraisalResult::pass("valid-hash")
                    } else {
                        ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-hash")
                    }
                }
                Ok(_) => ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-hash"),
                Err(errno) if errno == -ENOPKG => {
                    ImaAppraisalResult::status(ImaAppraisalStatus::Unknown, "unsupported-hash-algo")
                }
                Err(_) => ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-hash"),
            }
        }
        Some(EVM_IMA_XATTR_DIGSIG) => verify_ima_signature_xattr(hook, bytes, xattr),
        Some(IMA_VERITY_DIGSIG) => verify_ima_verity_signature_xattr(hook, bytes, xattr),
        Some(_) => ImaAppraisalResult::status(ImaAppraisalStatus::Unknown, "unknown-ima-data"),
        None => ImaAppraisalResult::status(ImaAppraisalStatus::NoLabel, "missing-hash"),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SignatureV2Header {
    xattr_type: u8,
    version: u8,
    hash_algo: u8,
    keyid: u32,
    sig_size: usize,
}

fn signature_v2_header(xattr: &[u8]) -> Result<SignatureV2Header, &'static str> {
    if xattr.len() <= SIGNATURE_V2_HDR_SIZE {
        return Err("invalid-signature");
    }
    let header = SignatureV2Header {
        xattr_type: xattr[0],
        version: xattr[1],
        hash_algo: xattr[2],
        keyid: u32::from_be_bytes([xattr[3], xattr[4], xattr[5], xattr[6]]),
        sig_size: u16::from_be_bytes([xattr[7], xattr[8]]) as usize,
    };
    if header.hash_algo >= HASH_ALGO_LAST {
        return Err("unsupported-hash-algo");
    }
    Ok(header)
}

fn signature_v2_payload_len(xattr: &[u8]) -> Result<usize, &'static str> {
    let header = signature_v2_header(xattr)?;
    let payload_len = xattr.len() - SIGNATURE_V2_HDR_SIZE;
    if header.sig_size != payload_len {
        return Err("invalid-signature");
    }
    Ok(payload_len)
}

fn verify_ima_signature_xattr(hook: ImaHook, bytes: &[u8], xattr: &[u8]) -> ImaAppraisalResult {
    if xattr.len() <= 1 {
        return ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-signature");
    }
    let header = match signature_v2_header(xattr) {
        Ok(header) => header,
        Err("unsupported-hash-algo") => {
            return ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-signature");
        }
        Err(cause) => return ImaAppraisalResult::status(ImaAppraisalStatus::Fail, cause),
    };
    if header.version > 3 {
        return ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-signature-version");
    }
    if let Err(cause) = signature_v2_payload_len(xattr) {
        return ImaAppraisalResult::status(ImaAppraisalStatus::Fail, cause);
    }
    match header.version {
        1 => ImaAppraisalResult::status(ImaAppraisalStatus::Unknown, "signature-unsupported"),
        2 | 3 => match verify_signature_v2_with_integrity_keyrings(hook, bytes, xattr, header) {
            Ok(()) => ImaAppraisalResult::pass("valid-signature"),
            Err(errno) if errno == -ENOPKG || errno == -ENOKEY => {
                ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-signature")
            }
            Err(_) => ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-signature"),
        },
        _ => ImaAppraisalResult::status(ImaAppraisalStatus::Unknown, "signature-unsupported"),
    }
}

fn verify_ima_verity_signature_xattr(
    hook: ImaHook,
    bytes: &[u8],
    xattr: &[u8],
) -> ImaAppraisalResult {
    if xattr.len() <= 1 || xattr[1] != 3 {
        return ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-signature-version");
    }
    let header = match signature_v2_header(xattr) {
        Ok(header) => header,
        Err("unsupported-hash-algo") => {
            return ImaAppraisalResult::status(
                ImaAppraisalStatus::Fail,
                "invalid-verity-signature",
            );
        }
        Err(cause) => return ImaAppraisalResult::status(ImaAppraisalStatus::Fail, cause),
    };
    if let Err(cause) = signature_v2_payload_len(xattr) {
        return ImaAppraisalResult::status(ImaAppraisalStatus::Fail, cause);
    }
    match verify_signature_v2_with_integrity_keyrings(hook, bytes, xattr, header) {
        Ok(()) => ImaAppraisalResult::pass("valid-verity-signature"),
        Err(errno) if errno == -ENOPKG || errno == -ENOKEY => {
            ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-verity-signature")
        }
        Err(_) => ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-verity-signature"),
    }
}

fn verify_appraisal_xattr(
    hook: ImaHook,
    bytes: &[u8],
    xattr: &[u8],
    appraise_type: Option<ImaAppraiseType>,
) -> ImaAppraisalResult {
    let Some(appraise_type) = appraise_type else {
        return verify_digest_xattr(hook, bytes, xattr);
    };

    match xattr.first().copied() {
        Some(IMA_XATTR_DIGEST | IMA_XATTR_DIGEST_NG) => {
            ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "IMA-signature-required")
        }
        Some(EVM_IMA_XATTR_DIGSIG) => {
            if appraise_type == ImaAppraiseType::SigV3 && xattr.get(1).copied().unwrap_or(0) != 3 {
                if xattr.get(1).copied().is_some_and(|version| version > 3) {
                    return ImaAppraisalResult::status(
                        ImaAppraisalStatus::Fail,
                        "invalid-signature-version",
                    );
                }
                return ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "IMA-sigv3-required");
            }
            verify_ima_signature_xattr(hook, bytes, xattr)
        }
        Some(IMA_VERITY_DIGSIG) => {
            ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "IMA-signature-required")
        }
        Some(_) => ImaAppraisalResult::status(ImaAppraisalStatus::Unknown, "unknown-ima-data"),
        None => ImaAppraisalResult::status(ImaAppraisalStatus::NoLabel, "missing-hash"),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RsaPublicKey {
    modulus: Vec<u8>,
    exponent: u32,
}

fn verify_signature_v2_data(
    keyring_id: i32,
    bytes: &[u8],
    xattr: &[u8],
    header: SignatureV2Header,
) -> Result<(), i32> {
    let digest = digest_for_algo(header.hash_algo, bytes)?;
    verify_signature_v2_digest_data(keyring_id, xattr, header, &digest)
}

fn verify_signature_v2_digest_data(
    keyring_id: i32,
    xattr: &[u8],
    header: SignatureV2Header,
    digest: &[u8],
) -> Result<(), i32> {
    let signed_digest = if header.version == 3 {
        ima_file_id_hash(header.xattr_type, header.hash_algo, digest)?
    } else {
        digest.to_vec()
    };
    let signature = &xattr[SIGNATURE_V2_HDR_SIZE..];
    let keys = asymmetric_key_payloads_for_keyid(keyring_id, header.keyid);
    if keys.is_empty() {
        return Err(-ENOKEY);
    }

    let mut saw_supported_key = false;
    for payload in keys.iter() {
        let Some(key) = parse_rsa_public_key(payload) else {
            continue;
        };
        saw_supported_key = true;
        if rsa_pkcs1_verify(&key, &signed_digest, header.hash_algo, signature) {
            return Ok(());
        }
    }

    if saw_supported_key {
        Err(-EBADMSG)
    } else {
        Err(-ENOPKG)
    }
}

pub(crate) fn verify_signature_v2_digest_with_keyring(
    keyring_id: i32,
    xattr: &[u8],
    digest: &[u8],
) -> Result<(), i32> {
    let header = signature_v2_header(xattr).map_err(|cause| {
        if cause == "unsupported-hash-algo" {
            -ENOPKG
        } else {
            -EINVAL
        }
    })?;
    if header.version > 3 {
        return Err(-EINVAL);
    }
    if header.version == 1 {
        return Err(-ENOPKG);
    }
    signature_v2_payload_len(xattr).map_err(|_| -EINVAL)?;
    verify_signature_v2_digest_data(keyring_id, xattr, header, digest)
}

fn verify_signature_v2_with_integrity_keyrings(
    hook: ImaHook,
    bytes: &[u8],
    xattr: &[u8],
    header: SignatureV2Header,
) -> Result<(), i32> {
    let ima_keyring = ensure_ima_keyring()?;
    match verify_signature_v2_data(ima_keyring, bytes, xattr, header) {
        Ok(()) => Ok(()),
        Err(err) if hook == ImaHook::KexecKernelCheck => {
            if let Some(platform) = crate::security::platform_certs::platform_keyring_id() {
                verify_signature_v2_data(platform, bytes, xattr, header).or(Err(err))
            } else {
                Err(err)
            }
        }
        Err(err) => Err(err),
    }
}

fn ima_file_id_hash(xattr_type: u8, algo: u8, digest: &[u8]) -> Result<Vec<u8>, i32> {
    if !matches!(
        xattr_type,
        EVM_IMA_XATTR_DIGSIG
            | crate::security::integrity::evm::EVM_XATTR_PORTABLE_DIGSIG
            | IMA_VERITY_DIGSIG
    ) {
        return Err(-EINVAL);
    }

    let mut file_id = Vec::with_capacity(2 + digest.len());
    file_id.push(xattr_type);
    file_id.push(algo);
    file_id.extend_from_slice(digest);
    digest_for_algo(algo, &file_id)
}

fn asymmetric_key_payloads_for_keyid(keyring_id: i32, keyid: u32) -> Vec<Vec<u8>> {
    let wanted = format!("id:{keyid:08x}");
    crate::security::keys::payloads_in_keyring_matching(keyring_id, "asymmetric", |key| {
        key.description == wanted || asymmetric_payload_matches_keyid(&key.payload, keyid)
    })
}

fn asymmetric_payload_matches_keyid(payload: &[u8], keyid: u32) -> bool {
    x509_subject_key_identifier_tail(payload).is_some_and(|candidate| candidate == keyid)
        || rsa_public_key_fingerprint_tail(payload).is_some_and(|candidate| candidate == keyid)
}

fn x509_subject_key_identifier_tail(cert_der: &[u8]) -> Option<u32> {
    const SUBJECT_KEY_IDENTIFIER_OID: &[u8] = &[0x55, 0x1d, 0x0e];
    let mut offset = 0usize;
    while offset + SUBJECT_KEY_IDENTIFIER_OID.len() + 2 <= cert_der.len() {
        if cert_der[offset] != 0x06 {
            offset += 1;
            continue;
        }
        let (oid_len, oid_start) = der_len_at(cert_der, offset + 1)?;
        let oid_end = oid_start.checked_add(oid_len)?;
        if oid_end > cert_der.len() {
            return None;
        }
        if &cert_der[oid_start..oid_end] != SUBJECT_KEY_IDENTIFIER_OID {
            offset = oid_end;
            continue;
        }

        let mut value_offset = oid_end;
        if cert_der.get(value_offset).copied() == Some(0x01) {
            let (_, _, next) = der_tlv(cert_der, value_offset, 0x01)?;
            value_offset = next;
        }
        let (outer_start, outer_len, _) = der_tlv(cert_der, value_offset, 0x04)?;
        let outer_end = outer_start.checked_add(outer_len)?;
        if outer_end > cert_der.len() {
            return None;
        }
        let outer = &cert_der[outer_start..outer_end];
        let keyid = if outer.first().copied() == Some(0x04) {
            let (inner_start, inner_len, _) = der_tlv(outer, 0, 0x04)?;
            outer.get(inner_start..inner_start.checked_add(inner_len)?)?
        } else {
            outer
        };
        if keyid.len() >= 4 {
            let tail = &keyid[keyid.len() - 4..];
            return Some(u32::from_be_bytes([tail[0], tail[1], tail[2], tail[3]]));
        }
        return None;
    }
    None
}

fn rsa_public_key_fingerprint_tail(payload: &[u8]) -> Option<u32> {
    let key = parse_rsa_public_key(payload)?;
    let mut material = key.modulus.clone();
    material.extend_from_slice(&key.exponent.to_be_bytes());
    let digest = sha1_digest(&material);
    Some(u32::from_be_bytes([
        digest[IMA_SHA1_DIGEST_SIZE - 4],
        digest[IMA_SHA1_DIGEST_SIZE - 3],
        digest[IMA_SHA1_DIGEST_SIZE - 2],
        digest[IMA_SHA1_DIGEST_SIZE - 1],
    ]))
}

fn parse_rsa_public_key(payload: &[u8]) -> Option<RsaPublicKey> {
    for offset in 0..payload.len() {
        if payload[offset] != 0x30 {
            continue;
        }
        if let Some(key) = parse_rsa_public_sequence(payload, offset) {
            return Some(key);
        }
        if let Some(key) = parse_rsa_private_sequence(payload, offset) {
            return Some(key);
        }
    }
    None
}

fn parse_rsa_public_sequence(bytes: &[u8], offset: usize) -> Option<RsaPublicKey> {
    let (seq_start, _seq_len, seq_end) = der_tlv(bytes, offset, 0x30)?;
    let (mod_start, mod_len, next) = der_tlv(bytes, seq_start, 0x02)?;
    let (exp_start, exp_len, _) = der_tlv(bytes, next, 0x02)?;
    if mod_start.checked_add(mod_len)? > seq_end || exp_start.checked_add(exp_len)? > seq_end {
        return None;
    }
    rsa_key_from_parts(
        &bytes[mod_start..mod_start + mod_len],
        &bytes[exp_start..exp_start + exp_len],
    )
}

fn parse_rsa_private_sequence(bytes: &[u8], offset: usize) -> Option<RsaPublicKey> {
    let (seq_start, _seq_len, seq_end) = der_tlv(bytes, offset, 0x30)?;
    let (_, _, after_version) = der_tlv(bytes, seq_start, 0x02)?;
    let (mod_start, mod_len, next) = der_tlv(bytes, after_version, 0x02)?;
    let (exp_start, exp_len, _) = der_tlv(bytes, next, 0x02)?;
    if mod_start.checked_add(mod_len)? > seq_end || exp_start.checked_add(exp_len)? > seq_end {
        return None;
    }
    rsa_key_from_parts(
        &bytes[mod_start..mod_start + mod_len],
        &bytes[exp_start..exp_start + exp_len],
    )
}

fn rsa_key_from_parts(modulus: &[u8], exponent: &[u8]) -> Option<RsaPublicKey> {
    let modulus = strip_leading_zeroes(modulus).to_vec();
    if modulus.len() < 64 || exponent.is_empty() || exponent.len() > 4 {
        return None;
    }
    let mut exp = 0u32;
    for byte in exponent {
        exp = exp.checked_shl(8)?.checked_add(*byte as u32)?;
    }
    if !matches!(exp, 3 | 65_537) {
        return None;
    }
    Some(RsaPublicKey {
        modulus,
        exponent: exp,
    })
}

fn der_tlv(bytes: &[u8], offset: usize, tag: u8) -> Option<(usize, usize, usize)> {
    if bytes.get(offset).copied()? != tag {
        return None;
    }
    let (len, start) = der_len_at(bytes, offset + 1)?;
    let end = start.checked_add(len)?;
    if end > bytes.len() {
        return None;
    }
    Some((start, len, end))
}

fn der_len_at(bytes: &[u8], offset: usize) -> Option<(usize, usize)> {
    let first = *bytes.get(offset)?;
    if first < 0x80 {
        return Some((first as usize, offset + 1));
    }
    let width = (first & 0x7f) as usize;
    if width == 0 || width > core::mem::size_of::<usize>() {
        return None;
    }
    let start = offset + 1;
    let end = start.checked_add(width)?;
    if end > bytes.len() {
        return None;
    }
    let mut len = 0usize;
    for byte in &bytes[start..end] {
        len = len.checked_shl(8)?.checked_add(*byte as usize)?;
    }
    Some((len, end))
}

fn strip_leading_zeroes(bytes: &[u8]) -> &[u8] {
    let first_nonzero = bytes
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(bytes.len().saturating_sub(1));
    &bytes[first_nonzero..]
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

fn rsa_pkcs1_verify(key: &RsaPublicKey, digest: &[u8], algo: u8, signature: &[u8]) -> bool {
    if signature.len() != key.modulus.len() {
        return false;
    }
    let Some(encoded) = mod_exp_be(signature, key.exponent, &key.modulus) else {
        return false;
    };
    pkcs1_encoded_digest_matches(&encoded, digest, algo)
}

fn pkcs1_encoded_digest_matches(encoded: &[u8], digest: &[u8], algo: u8) -> bool {
    if encoded.len() < 11 || encoded[0] != 0 || encoded[1] != 1 {
        return false;
    }
    let mut idx = 2usize;
    while idx < encoded.len() && encoded[idx] == 0xff {
        idx += 1;
    }
    if idx < 10 || encoded.get(idx).copied() != Some(0) {
        return false;
    }
    let Some(prefix) = pkcs1_digest_info_prefix(algo) else {
        return false;
    };
    let payload = &encoded[idx + 1..];
    payload.len() == prefix.len() + digest.len()
        && payload.starts_with(prefix)
        && constant_time_eq(&payload[prefix.len()..], digest)
}

fn pkcs1_digest_info_prefix(algo: u8) -> Option<&'static [u8]> {
    match algo {
        HASH_ALGO_SHA1 => Some(&[
            0x30, 0x21, 0x30, 0x09, 0x06, 0x05, 0x2b, 0x0e, 0x03, 0x02, 0x1a, 0x05, 0x00, 0x04,
            0x14,
        ]),
        HASH_ALGO_SHA256 => Some(&[
            0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02,
            0x01, 0x05, 0x00, 0x04, 0x20,
        ]),
        _ => None,
    }
}

fn mod_exp_be(base: &[u8], exponent: u32, modulus: &[u8]) -> Option<Vec<u8>> {
    let modulus = strip_leading_zeroes(modulus);
    if modulus.is_empty() || modulus[0] == 0 || exponent == 0 {
        return None;
    }
    let mut result = vec![0u8; modulus.len()];
    *result.last_mut()? = 1;
    let mut base = mod_reduce_be(strip_leading_zeroes(base), modulus);
    let mut exp = exponent;
    while exp != 0 {
        if exp & 1 == 1 {
            result = mod_mul_be(&result, &base, modulus);
        }
        exp >>= 1;
        if exp != 0 {
            base = mod_mul_be(&base, &base, modulus);
        }
    }
    Some(result)
}

fn mod_mul_be(left: &[u8], right: &[u8], modulus: &[u8]) -> Vec<u8> {
    let product = mul_be(left, right);
    mod_reduce_be(&product, modulus)
}

fn mul_be(left: &[u8], right: &[u8]) -> Vec<u8> {
    let left = strip_leading_zeroes(left);
    let right = strip_leading_zeroes(right);
    let mut digits = vec![0u32; left.len() + right.len()];
    for (li, lb) in left.iter().enumerate() {
        for (ri, rb) in right.iter().enumerate() {
            digits[li + ri + 1] += (*lb as u32) * (*rb as u32);
        }
    }
    for idx in (1..digits.len()).rev() {
        let carry = digits[idx] >> 8;
        digits[idx] &= 0xff;
        digits[idx - 1] += carry;
    }
    let out = digits
        .into_iter()
        .map(|digit| (digit & 0xff) as u8)
        .collect::<Vec<_>>();
    strip_leading_zeroes(&out).to_vec()
}

fn mod_reduce_be(value: &[u8], modulus: &[u8]) -> Vec<u8> {
    let modulus = strip_leading_zeroes(modulus);
    let mut modulus_padded = Vec::with_capacity(modulus.len() + 1);
    modulus_padded.push(0);
    modulus_padded.extend_from_slice(modulus);
    let mut rem = vec![0u8; modulus_padded.len()];

    for byte in value {
        for bit in (0..8).rev() {
            shl1_be(&mut rem);
            let low_bit = (*byte >> bit) & 1;
            if let Some(last) = rem.last_mut() {
                *last |= low_bit;
            }
            if cmp_be_same_len(&rem, &modulus_padded) != core::cmp::Ordering::Less {
                sub_assign_be_same_len(&mut rem, &modulus_padded);
            }
        }
    }
    rem[1..].to_vec()
}

fn shl1_be(bytes: &mut [u8]) {
    let mut carry = 0u8;
    for byte in bytes.iter_mut().rev() {
        let next_carry = (*byte & 0x80) >> 7;
        *byte = (*byte << 1) | carry;
        carry = next_carry;
    }
}

fn cmp_be_same_len(left: &[u8], right: &[u8]) -> core::cmp::Ordering {
    debug_assert_eq!(left.len(), right.len());
    for (lb, rb) in left.iter().zip(right.iter()) {
        match lb.cmp(rb) {
            core::cmp::Ordering::Equal => {}
            other => return other,
        }
    }
    core::cmp::Ordering::Equal
}

fn sub_assign_be_same_len(left: &mut [u8], right: &[u8]) {
    debug_assert_eq!(left.len(), right.len());
    let mut borrow = 0u16;
    for idx in (0..left.len()).rev() {
        let lhs = left[idx] as u16;
        let rhs = right[idx] as u16 + borrow;
        if lhs >= rhs {
            left[idx] = (lhs - rhs) as u8;
            borrow = 0;
        } else {
            left[idx] = (lhs + 256 - rhs) as u8;
            borrow = 1;
        }
    }
}

fn map_evm_appraisal_status(
    status: crate::security::integrity::evm::EvmIntegrityStatus,
) -> Option<ImaAppraisalResult> {
    use crate::security::integrity::evm::EvmIntegrityStatus as EvmStatus;
    match status {
        EvmStatus::Pass => None,
        EvmStatus::PassImmutable => Some(ImaAppraisalResult::status(
            ImaAppraisalStatus::PassImmutable,
            "valid-immutable-HMAC",
        )),
        EvmStatus::Unknown | EvmStatus::NoXattrs => None,
        EvmStatus::NoLabel => Some(ImaAppraisalResult::status(
            ImaAppraisalStatus::NoLabel,
            "missing-HMAC",
        )),
        EvmStatus::Fail | EvmStatus::FailImmutable => Some(ImaAppraisalResult::status(
            ImaAppraisalStatus::Fail,
            "invalid-HMAC",
        )),
    }
}

pub fn appraise_file_for_hook(
    hook: ImaHook,
    path: &str,
    bytes: &[u8],
    ima_xattr: Option<&[u8]>,
    evm_status: Option<crate::security::integrity::evm::EvmIntegrityStatus>,
) -> ImaAppraisalResult {
    appraise_file_for_hook_mask(
        hook,
        default_mask_for_hook(hook),
        path,
        bytes,
        ima_xattr,
        evm_status,
    )
}

pub fn appraise_file_for_hook_mask(
    hook: ImaHook,
    mask: u32,
    path: &str,
    bytes: &[u8],
    ima_xattr: Option<&[u8]>,
    evm_status: Option<crate::security::integrity::evm::EvmIntegrityStatus>,
) -> ImaAppraisalResult {
    let Some(appraisal) = policy_appraisal_for(hook, mask, path) else {
        return ImaAppraisalResult::pass("not-appraised");
    };
    if !appraisal.required {
        return ImaAppraisalResult::pass("not-appraised");
    }

    let Some(ima_xattr) = ima_xattr else {
        return ImaAppraisalResult::status(ImaAppraisalStatus::NoLabel, "missing-hash");
    };

    if let Some(evm_status) = evm_status.and_then(map_evm_appraisal_status) {
        return evm_status;
    }

    verify_appraisal_xattr(hook, bytes, ima_xattr, appraisal.appraise_type)
}

pub fn enforce_appraisal_for_hook(
    hook: ImaHook,
    path: &str,
    bytes: &[u8],
    ima_xattr: Option<&[u8]>,
    evm_status: Option<crate::security::integrity::evm::EvmIntegrityStatus>,
) -> Result<(), i32> {
    let result = appraise_file_for_hook_mask(
        hook,
        default_mask_for_hook(hook),
        path,
        bytes,
        ima_xattr,
        evm_status,
    );
    if result.is_pass() {
        Ok(())
    } else {
        Err(-EACCES)
    }
}

fn parse_policy(bytes: &[u8]) -> Result<(Vec<ImaPolicyRule>, String), i32> {
    let text = core::str::from_utf8(bytes).map_err(|_| -EINVAL)?;
    let mut rules = Vec::new();
    let mut normalized = String::new();
    for raw_line in text.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let rule = parse_policy_rule(line)?;
        normalized.push_str(&rule.to_policy_line());
        normalized.push('\n');
        rules.push(rule);
    }
    Ok((rules, normalized))
}

fn parse_policy_rule(line: &str) -> Result<ImaPolicyRule, i32> {
    let mut tokens = line.split_whitespace();
    let action = ImaPolicyAction::parse(tokens.next().ok_or(-EINVAL)?)?;
    let mut func = None;
    let mut mask = None;
    let mut fsmagic = None;
    let mut fsname = None;
    let mut fs_subtype = None;
    let mut fsuuid = None;
    let mut uid = None;
    let mut euid = None;
    let mut gid = None;
    let mut egid = None;
    let mut fowner = None;
    let mut fgroup = None;
    let mut obj_user = None;
    let mut obj_role = None;
    let mut obj_type = None;
    let mut subj_user = None;
    let mut subj_role = None;
    let mut subj_type = None;
    let mut keyrings = None;
    let mut label = None;
    let mut digest_type = None;
    let mut appraise_type = None;
    let mut appraise_algos = None;
    let mut permit_directio = false;
    let mut pcr = None;
    let mut template = None;

    for token in tokens {
        if let Some(value) = token.strip_prefix("func=") {
            if func.is_some() {
                return Err(-EINVAL);
            }
            func = Some(ImaHook::parse(value)?);
        } else if let Some(value) = token.strip_prefix("mask=") {
            if mask.is_some() {
                return Err(-EINVAL);
            }
            mask = Some(ImaPolicyMask::parse(value)?);
        } else if let Some(value) = token.strip_prefix("fsmagic=") {
            if fsmagic.is_some() {
                return Err(-EINVAL);
            }
            fsmagic = Some(parse_hex_or_decimal_u64(value)?);
        } else if let Some(value) = token.strip_prefix("fsname=") {
            if fsname.is_some() || value.is_empty() {
                return Err(-EINVAL);
            }
            fsname = Some(String::from(value));
        } else if let Some(value) = token.strip_prefix("fs_subtype=") {
            if fs_subtype.is_some() || value.is_empty() {
                return Err(-EINVAL);
            }
            fs_subtype = Some(String::from(value));
        } else if let Some(value) = token.strip_prefix("fsuuid=") {
            if fsuuid.is_some() {
                return Err(-EINVAL);
            }
            fsuuid = Some(parse_fsuuid(value)?);
        } else if let Some(value) = token.strip_prefix("uid=") {
            set_numeric(&mut uid, ImaNumericOp::Eq, value)?;
        } else if let Some(value) = token.strip_prefix("uid>") {
            set_numeric(&mut uid, ImaNumericOp::Gt, value)?;
        } else if let Some(value) = token.strip_prefix("uid<") {
            set_numeric(&mut uid, ImaNumericOp::Lt, value)?;
        } else if let Some(value) = token.strip_prefix("euid=") {
            set_numeric(&mut euid, ImaNumericOp::Eq, value)?;
        } else if let Some(value) = token.strip_prefix("euid>") {
            set_numeric(&mut euid, ImaNumericOp::Gt, value)?;
        } else if let Some(value) = token.strip_prefix("euid<") {
            set_numeric(&mut euid, ImaNumericOp::Lt, value)?;
        } else if let Some(value) = token.strip_prefix("gid=") {
            set_numeric(&mut gid, ImaNumericOp::Eq, value)?;
        } else if let Some(value) = token.strip_prefix("gid>") {
            set_numeric(&mut gid, ImaNumericOp::Gt, value)?;
        } else if let Some(value) = token.strip_prefix("gid<") {
            set_numeric(&mut gid, ImaNumericOp::Lt, value)?;
        } else if let Some(value) = token.strip_prefix("egid=") {
            set_numeric(&mut egid, ImaNumericOp::Eq, value)?;
        } else if let Some(value) = token.strip_prefix("egid>") {
            set_numeric(&mut egid, ImaNumericOp::Gt, value)?;
        } else if let Some(value) = token.strip_prefix("egid<") {
            set_numeric(&mut egid, ImaNumericOp::Lt, value)?;
        } else if let Some(value) = token.strip_prefix("fowner=") {
            set_numeric(&mut fowner, ImaNumericOp::Eq, value)?;
        } else if let Some(value) = token.strip_prefix("fowner>") {
            set_numeric(&mut fowner, ImaNumericOp::Gt, value)?;
        } else if let Some(value) = token.strip_prefix("fowner<") {
            set_numeric(&mut fowner, ImaNumericOp::Lt, value)?;
        } else if let Some(value) = token.strip_prefix("fgroup=") {
            set_numeric(&mut fgroup, ImaNumericOp::Eq, value)?;
        } else if let Some(value) = token.strip_prefix("fgroup>") {
            set_numeric(&mut fgroup, ImaNumericOp::Gt, value)?;
        } else if let Some(value) = token.strip_prefix("fgroup<") {
            set_numeric(&mut fgroup, ImaNumericOp::Lt, value)?;
        } else if let Some(value) = token.strip_prefix("obj_user=") {
            set_string(&mut obj_user, value)?;
        } else if let Some(value) = token.strip_prefix("obj_role=") {
            set_string(&mut obj_role, value)?;
        } else if let Some(value) = token.strip_prefix("obj_type=") {
            set_string(&mut obj_type, value)?;
        } else if let Some(value) = token.strip_prefix("subj_user=") {
            set_string(&mut subj_user, value)?;
        } else if let Some(value) = token.strip_prefix("subj_role=") {
            set_string(&mut subj_role, value)?;
        } else if let Some(value) = token.strip_prefix("subj_type=") {
            set_string(&mut subj_type, value)?;
        } else if let Some(value) = token.strip_prefix("keyrings=") {
            if keyrings.is_some() {
                return Err(-EINVAL);
            }
            keyrings = Some(ImaPolicyList::parse(value)?);
        } else if let Some(value) = token.strip_prefix("label=") {
            if label.is_some() {
                return Err(-EINVAL);
            }
            label = Some(ImaPolicyList::parse(value)?);
        } else if let Some(value) = token.strip_prefix("digest_type=") {
            if digest_type.is_some() {
                return Err(-EINVAL);
            }
            digest_type = Some(ImaDigestType::parse(value)?);
        } else if let Some(value) = token.strip_prefix("appraise_type=") {
            if action != ImaPolicyAction::Appraise || appraise_type.is_some() {
                return Err(-EINVAL);
            }
            appraise_type = Some(ImaAppraiseType::parse(value)?);
        } else if let Some(value) = token.strip_prefix("appraise_flag=") {
            if value.is_empty() {
                return Err(-EINVAL);
            }
        } else if let Some(value) = token.strip_prefix("appraise_algos=") {
            if appraise_algos.is_some() {
                return Err(-EINVAL);
            }
            appraise_algos = Some(ImaHashAlgoSet::parse(value)?);
        } else if token == "permit_directio" {
            if permit_directio {
                return Err(-EINVAL);
            }
            permit_directio = true;
        } else if let Some(value) = token.strip_prefix("pcr=") {
            if pcr.is_some() {
                return Err(-EINVAL);
            }
            let parsed = parse_decimal_u32(value)?;
            if parsed >= 1024 {
                return Err(-EINVAL);
            }
            pcr = Some(parsed);
        } else if let Some(value) = token.strip_prefix("template=") {
            if action != ImaPolicyAction::Measure || template.is_some() || !known_template(value) {
                return Err(-EINVAL);
            }
            template = Some(String::from(value));
        } else {
            return Err(-EINVAL);
        }
    }

    let rule = ImaPolicyRule {
        action,
        func,
        mask,
        fsmagic,
        fsname,
        fs_subtype,
        fsuuid,
        uid,
        euid,
        gid,
        egid,
        fowner,
        fgroup,
        obj_user,
        obj_role,
        obj_type,
        subj_user,
        subj_role,
        subj_type,
        keyrings,
        label,
        digest_type,
        appraise_type,
        appraise_algos,
        permit_directio,
        pcr,
        template,
    };
    validate_policy_rule(&rule)?;
    Ok(rule)
}

fn validate_policy_rule(rule: &ImaPolicyRule) -> Result<(), i32> {
    if rule.pcr.is_some() && rule.action != ImaPolicyAction::Measure {
        return Err(-EINVAL);
    }
    if rule.template.is_some() && rule.action != ImaPolicyAction::Measure {
        return Err(-EINVAL);
    }
    if rule.action != ImaPolicyAction::Appraise
        && (rule.appraise_type.is_some()
            || rule.digest_type.is_some()
            || rule.appraise_algos.is_some())
    {
        return Err(-EINVAL);
    }
    if rule.digest_type == Some(ImaDigestType::Verity)
        && rule.appraise_type != Some(ImaAppraiseType::SigV3)
    {
        return Err(-EINVAL);
    }
    if rule.keyrings.is_some() && rule.func != Some(ImaHook::KeyCheck) {
        return Err(-EINVAL);
    }
    if rule.label.is_some() && rule.func != Some(ImaHook::CriticalData) {
        return Err(-EINVAL);
    }
    if rule.func == Some(ImaHook::SetxattrCheck)
        && (rule.action != ImaPolicyAction::Appraise || rule.appraise_algos.is_none())
    {
        return Err(-EINVAL);
    }
    Ok(())
}

fn set_numeric(
    target: &mut Option<ImaNumericCondition>,
    op: ImaNumericOp,
    value: &str,
) -> Result<(), i32> {
    if target.is_some() {
        return Err(-EINVAL);
    }
    *target = Some(ImaNumericCondition {
        op,
        value: parse_decimal_u32(value)?,
    });
    Ok(())
}

fn set_string(target: &mut Option<String>, value: &str) -> Result<(), i32> {
    if target.is_some() || value.is_empty() {
        return Err(-EINVAL);
    }
    *target = Some(String::from(value));
    Ok(())
}

fn parse_decimal_u32(value: &str) -> Result<u32, i32> {
    value.parse::<u32>().map_err(|_| -EINVAL)
}

fn parse_hex_or_decimal_u64(value: &str) -> Result<u64, i32> {
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16).map_err(|_| -EINVAL)
    } else {
        u64::from_str_radix(value, 16)
            .or_else(|_| value.parse::<u64>())
            .map_err(|_| -EINVAL)
    }
}

fn parse_fsuuid(value: &str) -> Result<String, i32> {
    let mut digits = 0usize;
    for (idx, byte) in value.bytes().enumerate() {
        let is_hyphen = matches!(idx, 8 | 13 | 18 | 23) && byte == b'-';
        if is_hyphen {
            continue;
        }
        if !byte.is_ascii_hexdigit() {
            return Err(-EINVAL);
        }
        digits += 1;
    }
    if value.len() != 36 || digits != 32 {
        return Err(-EINVAL);
    }
    Ok(value.to_ascii_lowercase())
}

fn known_template(value: &str) -> bool {
    matches!(
        value,
        "ima-ng" | "ima-sig" | "ima-ngv2" | "ima-sigv2" | "ima-buf" | "ima-modsig"
    )
}

fn digest_hex(digest: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::new();
    for byte in digest {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

pub fn sha1_digest(bytes: &[u8]) -> [u8; IMA_SHA1_DIGEST_SIZE] {
    let mut h0 = 0x6745_2301u32;
    let mut h1 = 0xefcd_ab89u32;
    let mut h2 = 0x98ba_dcfeu32;
    let mut h3 = 0x1032_5476u32;
    let mut h4 = 0xc3d2_e1f0u32;

    let bit_len = (bytes.len() as u64).wrapping_mul(8);
    let mut msg = Vec::with_capacity(((bytes.len() + 9 + 63) / 64) * 64);
    msg.extend_from_slice(bytes);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 80];
        for (idx, word) in w.iter_mut().take(16).enumerate() {
            let off = idx * 4;
            *word =
                u32::from_be_bytes([chunk[off], chunk[off + 1], chunk[off + 2], chunk[off + 3]]);
        }
        for idx in 16..80 {
            w[idx] = (w[idx - 3] ^ w[idx - 8] ^ w[idx - 14] ^ w[idx - 16]).rotate_left(1);
        }

        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;

        for (idx, word) in w.iter().enumerate() {
            let (f, k) = match idx {
                0..=19 => ((b & c) | ((!b) & d), 0x5a82_7999),
                20..=39 => (b ^ c ^ d, 0x6ed9_eba1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1b_bcdc),
                _ => (b ^ c ^ d, 0xca62_c1d6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    let mut out = [0u8; IMA_SHA1_DIGEST_SIZE];
    out[0..4].copy_from_slice(&h0.to_be_bytes());
    out[4..8].copy_from_slice(&h1.to_be_bytes());
    out[8..12].copy_from_slice(&h2.to_be_bytes());
    out[12..16].copy_from_slice(&h3.to_be_bytes());
    out[16..20].copy_from_slice(&h4.to_be_bytes());
    out
}

pub fn sha256_digest(bytes: &[u8]) -> [u8; IMA_SHA256_DIGEST_SIZE] {
    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];

    let mut state = [
        0x6a09_e667u32,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];

    let bit_len = (bytes.len() as u64).wrapping_mul(8);
    let mut msg = Vec::with_capacity(((bytes.len() + 9 + 63) / 64) * 64);
    msg.extend_from_slice(bytes);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (idx, word) in w.iter_mut().take(16).enumerate() {
            let off = idx * 4;
            *word =
                u32::from_be_bytes([chunk[off], chunk[off + 1], chunk[off + 2], chunk[off + 3]]);
        }
        for idx in 16..64 {
            let s0 =
                w[idx - 15].rotate_right(7) ^ w[idx - 15].rotate_right(18) ^ (w[idx - 15] >> 3);
            let s1 = w[idx - 2].rotate_right(17) ^ w[idx - 2].rotate_right(19) ^ (w[idx - 2] >> 10);
            w[idx] = w[idx - 16]
                .wrapping_add(s0)
                .wrapping_add(w[idx - 7])
                .wrapping_add(s1);
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];

        for idx in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[idx])
                .wrapping_add(w[idx]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    let mut out = [0u8; IMA_SHA256_DIGEST_SIZE];
    for (idx, word) in state.iter().enumerate() {
        out[idx * 4..idx * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

#[cfg(test)]
pub fn reset_for_test() {
    INITIALIZED.store(false, Ordering::Release);
    TPM_BYPASS.store(false, Ordering::Release);
    BOOT_AGGREGATE_PRESENT.store(false, Ordering::Release);
    MEASUREMENT_COUNT.store(0, Ordering::Release);
    FILE_HOOK_MEASUREMENTS_ENABLED.store(false, Ordering::Release);
    MEASUREMENTS.lock().clear();
    *POLICY.lock() = ImaPolicy::default();
    *IMA_KEYRING_ID.lock() = None;
}

#[cfg(test)]
pub fn set_file_hook_measurements_for_test(enabled: bool) {
    FILE_HOOK_MEASUREMENTS_ENABLED.store(enabled, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::ops::{NOOP_FILE_OPS, NOOP_INODE_OPS};
    use crate::fs::types::{Inode, InodeKind};
    use crate::security::hooks::LSM_ID_IMA;
    use crate::security::lsm_list::{TEST_LSM_LOCK, lsm_active_ids, reset_for_test as reset_lsms};

    #[test]
    fn ima_init_no_tpm_enables_bypass_and_boot_aggregate() {
        let _guard = TEST_LSM_LOCK.lock();
        reset_lsms();
        reset_for_test();

        init();

        let state = snapshot();
        assert!(state.initialized);
        assert!(state.tpm_bypass);
        assert!(state.boot_aggregate_present);
        assert_eq!(state.measurement_count, 1);
        assert_eq!(state.measure_pcr_idx, 10);

        let mut ids = [0u64; 2];
        assert_eq!(lsm_active_ids(&mut ids), 1);
        assert_eq!(ids[0], LSM_ID_IMA);
    }

    #[test]
    fn ima_constants_match_linux_defaults() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        assert_eq!(BOOT_AGGREGATE_NAME, "boot_aggregate");
        assert_eq!(CONFIG_IMA_MEASURE_PCR_IDX, 10);
        assert_eq!(IMA_DEFAULT_TEMPLATE, "ima-ng");
        assert_eq!(IMA_DEFAULT_HASH_ALGO, "sha1");
    }

    #[test]
    fn ima_boot_aggregate_measurement_renders_securityfs_rows() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::lsm_list::reset_for_test();
        reset_for_test();
        init();

        assert_eq!(runtime_measurements_count(), 1);
        assert_eq!(runtime_violations(), 0);
        let ascii = ascii_runtime_measurements_sha1();
        assert!(ascii.starts_with("10 "));
        assert!(ascii.contains(" ima-ng sha1:"));
        assert!(ascii.ends_with(" boot_aggregate\n"));
        let binary = binary_runtime_measurements_sha1();
        assert!(binary.len() > IMA_SHA1_DIGEST_SIZE);
    }

    #[test]
    fn ima_sha1_digest_matches_known_vector() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        assert_eq!(
            digest_hex(&sha1_digest(b"abc")),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
    }

    #[test]
    fn ima_measure_file_appends_and_deduplicates_measurements() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::lsm_list::reset_for_test();
        reset_for_test();
        init();

        assert!(measure_file("/bin/bash", b"hello").expect("measure bash"));
        assert!(!measure_file("/usr/bin/same-bash", b"hello").expect("dedupe same digest"));
        assert!(measure_file("/bin/other", b"world").expect("measure other"));
        assert_eq!(runtime_measurements_count(), 3);

        let ascii = ascii_runtime_measurements_sha1();
        assert!(ascii.contains("boot_aggregate"));
        assert!(ascii.contains("/bin/bash"));
        assert!(ascii.contains("/bin/other"));
        assert!(!ascii.contains("/usr/bin/same-bash"));
        assert!(ascii.contains(&digest_hex(&sha1_digest(b"hello"))));

        let binary = binary_runtime_measurements_sha1();
        assert!(
            binary
                .windows(b"/bin/bash".len())
                .any(|w| w == b"/bin/bash")
        );
        assert!(
            binary
                .windows(b"/bin/other".len())
                .any(|w| w == b"/bin/other")
        );
    }

    #[test]
    fn ima_policy_loads_text_rules_and_filters_measurement_hooks() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::lsm_list::reset_for_test();
        reset_for_test();
        init();
        set_file_hook_measurements_for_test(true);

        let policy = b"
# Linux-style IMA policy subset
dont_measure func=FILE_CHECK fsmagic=0x1021994
measure func=BPRM_CHECK mask=MAY_EXEC fsname=rootfs
measure func=FILE_CHECK mask=MAY_READ fsname=rootfs
measure func=MMAP_CHECK fowner=0
";
        assert_eq!(load_policy(policy), Ok(policy.len()));
        assert_eq!(policy_rule_count(), 4);
        assert_eq!(
            policy_text(),
            "dont_measure func=FILE_CHECK fsmagic=0x1021994\n\
measure func=BPRM_CHECK mask=MAY_EXEC fsname=rootfs\n\
measure func=FILE_CHECK mask=MAY_READ fsname=rootfs\n\
measure func=MMAP_CHECK fowner=0\n"
        );

        assert!(
            measure_file_for_hook(ImaHook::BprmCheck, "/bin/bash", b"bash").expect("measure exec")
        );
        let inode = Inode::new(
            6,
            InodeKind::Regular,
            0o444,
            &NOOP_INODE_OPS,
            &NOOP_FILE_OPS,
            InodePrivate::StaticBytes(b"file payload"),
        );
        assert!(!measure_opened_inode("/run/token", &inode).expect("dont_measure wins"));
        assert!(measure_opened_inode("/etc/passwd", &inode).expect("measure file"));
        assert!(!measure_opened_inode("/lib/libc.so", &inode).expect("file hook skips mmap rule"));

        let mmap_inode = Inode::new(
            7,
            InodeKind::Regular,
            0o444,
            &NOOP_INODE_OPS,
            &NOOP_FILE_OPS,
            InodePrivate::StaticBytes(b"mmap payload"),
        );
        assert!(measure_mapped_inode("/lib/libc.so", &mmap_inode).expect("measure mmap"));

        let ascii = ascii_runtime_measurements_sha1();
        assert!(ascii.contains("/bin/bash"));
        assert!(ascii.contains("/etc/passwd"));
        assert!(ascii.contains("/lib/libc.so"));
        assert!(!ascii.contains("/run/token"));
    }

    #[test]
    fn ima_policy_mask_tokens_filter_measurement_and_appraisal_rules() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::lsm_list::reset_for_test();
        reset_for_test();
        init();

        let policy = b"
measure func=FILE_CHECK mask=MAY_WRITE fsname=rootfs
measure func=FILE_CHECK mask=^MAY_APPEND fowner=0
appraise func=FILE_CHECK mask=MAY_READ fsname=rootfs
appraise func=FILE_CHECK mask=^MAY_WRITE uid=0
";
        assert_eq!(load_policy(policy), Ok(policy.len()));
        assert_eq!(
            policy_text(),
            "measure func=FILE_CHECK mask=MAY_WRITE fsname=rootfs\n\
measure func=FILE_CHECK mask=^MAY_APPEND fowner=0\n\
appraise func=FILE_CHECK mask=MAY_READ fsname=rootfs\n\
appraise func=FILE_CHECK mask=^MAY_WRITE uid=0\n"
        );

        assert!(
            !measure_file_for_hook(ImaHook::FileCheck, "/var/log/messages", b"default-read")
                .expect("exact MAY_WRITE rule skips read")
        );
        assert!(
            measure_file_for_hook_mask(
                ImaHook::FileCheck,
                IMA_MAY_WRITE,
                "/var/log/messages",
                b"write",
            )
            .expect("exact MAY_WRITE rule matches write")
        );
        assert!(
            measure_file_for_hook_mask(
                ImaHook::FileCheck,
                IMA_MAY_APPEND | IMA_MAY_WRITE,
                "/var/spool/mail",
                b"append",
            )
            .expect("IMA_INMASK-style append rule matches combined mask")
        );
        assert!(should_appraise_path_for_hook(
            ImaHook::FileCheck,
            "/etc/passwd"
        ));
        assert!(should_appraise_path_for_hook_mask(
            ImaHook::FileCheck,
            IMA_MAY_WRITE,
            "/etc/passwd"
        ));
        assert!(should_appraise_path_for_hook_mask(
            ImaHook::FileCheck,
            IMA_MAY_WRITE | IMA_MAY_APPEND,
            "/root/secret"
        ));
        assert!(!should_appraise_path_for_hook(
            ImaHook::FileCheck,
            "/proc/secret"
        ));
    }

    #[test]
    fn ima_policy_accepts_linux_grammar_conditions_and_rejects_path_shortcut() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        init();

        let policy = b"
measure func=KEY_CHECK keyrings=.ima|.platform pcr=10
measure func=CRITICAL_DATA label=kernel_info|selinux pcr=10
measure func=FILE_CHECK fsmagic=0x858458f6 fsname=rootfs uid=0 euid=0 gid=0 egid=0 template=ima-ngv2
appraise func=FILE_CHECK fsuuid=00112233-4455-6677-8899-aabbccddeeff fowner>100 fgroup<200 obj_user=system_u obj_role=object_r obj_type=bin_t subj_user=system_u subj_role=system_r subj_type=init_t appraise_type=sigv3 digest_type=verity permit_directio
appraise func=SETXATTR_CHECK appraise_algos=sha1,sha256
";
        assert_eq!(load_policy(policy), Ok(policy.len()));
        let text = policy_text();
        assert!(text.contains("measure func=KEY_CHECK keyrings=.ima|.platform pcr=10\n"));
        assert!(text.contains("measure func=CRITICAL_DATA label=kernel_info|selinux pcr=10\n"));
        assert!(text.contains("template=ima-ngv2"));
        assert!(text.contains("appraise_type=sigv3 digest_type=verity permit_directio\n"));
        assert!(text.contains("appraise func=SETXATTR_CHECK appraise_algos=sha1,sha256\n"));

        let key_context = ImaPolicyContext {
            func_data: Some(".ima"),
            ..ImaPolicyContext::for_path("/keys")
        };
        assert!(policy_allows_measurement_context(
            ImaHook::KeyCheck,
            0,
            &key_context
        ));
        let wrong_key_context = ImaPolicyContext {
            func_data: Some(".builtin_trusted_keys"),
            ..ImaPolicyContext::for_path("/keys")
        };
        assert!(!policy_allows_measurement_context(
            ImaHook::KeyCheck,
            0,
            &wrong_key_context
        ));

        let label_context = ImaPolicyContext {
            func_data: Some("kernel_info"),
            ..ImaPolicyContext::for_path("/critical")
        };
        assert!(policy_allows_measurement_context(
            ImaHook::CriticalData,
            0,
            &label_context
        ));

        let appraise_context = ImaPolicyContext {
            fsuuid: Some("00112233-4455-6677-8899-aabbccddeeff"),
            fowner: 101,
            fgroup: 100,
            obj_user: Some("system_u"),
            obj_role: Some("object_r"),
            obj_type: Some("bin_t"),
            subj_user: Some("system_u"),
            subj_role: Some("system_r"),
            subj_type: Some("init_t"),
            ..ImaPolicyContext::for_path("/usr/bin/init")
        };
        assert_eq!(
            policy_appraisal_for_context(ImaHook::FileCheck, IMA_MAY_READ, &appraise_context),
            Some(ImaPolicyAppraisal {
                required: true,
                appraise_type: Some(ImaAppraiseType::SigV3),
            })
        );
        assert_eq!(
            load_policy(b"measure func=FILE_CHECK path=/etc\n"),
            Err(-EINVAL)
        );
    }

    #[test]
    fn ima_policy_loads_appraise_rules_and_filters_hooks() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        init();

        let policy = b"
dont_appraise func=BPRM_CHECK fsmagic=0x1021994
appraise func=BPRM_CHECK fsname=rootfs
appraise func=FILE_CHECK fsname=rootfs
";
        assert_eq!(load_policy(policy), Ok(policy.len()));
        assert_eq!(policy_rule_count(), 3);
        assert_eq!(
            policy_text(),
            "dont_appraise func=BPRM_CHECK fsmagic=0x1021994\n\
appraise func=BPRM_CHECK fsname=rootfs\n\
appraise func=FILE_CHECK fsname=rootfs\n"
        );
        assert!(should_appraise_path_for_hook(
            ImaHook::BprmCheck,
            "/usr/bin/bash"
        ));
        assert!(!should_appraise_path_for_hook(
            ImaHook::BprmCheck,
            "/run/unsigned/tool"
        ));
        assert!(should_appraise_path_for_hook(
            ImaHook::FileCheck,
            "/etc/passwd"
        ));
        assert!(!should_appraise_path_for_hook(
            ImaHook::MmapCheck,
            "/etc/lib.so"
        ));
    }

    #[test]
    fn ima_appraisal_accepts_digest_and_ng_xattrs() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        init();
        let policy = b"appraise func=FILE_CHECK fsname=rootfs\n";
        assert_eq!(load_policy(policy), Ok(policy.len()));

        let payload = b"passwd contents";
        let ng = build_digest_xattr(payload);
        assert_eq!(ng[0], IMA_XATTR_DIGEST_NG);
        assert_eq!(ng[1], HASH_ALGO_SHA1);
        assert_eq!(
            appraise_file_for_hook(ImaHook::FileCheck, "/etc/passwd", payload, Some(&ng), None,),
            ImaAppraisalResult::pass("valid-hash")
        );

        let legacy = build_legacy_digest_xattr(payload);
        assert_eq!(legacy[0], IMA_XATTR_DIGEST);
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::FileCheck,
                "/etc/passwd",
                payload,
                Some(&legacy),
                None,
            ),
            ImaAppraisalResult::pass("valid-hash")
        );

        let mut tampered = ng;
        tampered[7] ^= 0x40;
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::FileCheck,
                "/etc/passwd",
                payload,
                Some(&tampered),
                None,
            ),
            ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-hash")
        );
        assert_eq!(
            enforce_appraisal_for_hook(
                ImaHook::FileCheck,
                "/etc/passwd",
                payload,
                Some(&tampered),
                None,
            ),
            Err(-EACCES)
        );
    }

    #[test]
    fn ima_appraisal_reports_missing_or_unsupported_xattrs() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        init();
        let policy = b"appraise func=BPRM_CHECK fsname=rootfs\n";
        assert_eq!(load_policy(policy), Ok(policy.len()));

        assert_eq!(
            appraise_file_for_hook(ImaHook::BprmCheck, "/bin/bash", b"bash", None, None),
            ImaAppraisalResult::status(ImaAppraisalStatus::NoLabel, "missing-hash")
        );

        let mut unsupported_algo = build_digest_xattr(b"bash");
        unsupported_algo[1] = HASH_ALGO_LAST;
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::BprmCheck,
                "/bin/bash",
                b"bash",
                Some(&unsupported_algo),
                None,
            ),
            ImaAppraisalResult::status(ImaAppraisalStatus::Unknown, "unsupported-hash-algo")
        );

        let sig = [EVM_IMA_XATTR_DIGSIG, 1, 2, 3];
        assert_eq!(
            appraise_file_for_hook(ImaHook::BprmCheck, "/bin/bash", b"bash", Some(&sig), None),
            ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-signature")
        );

        let digest = build_digest_xattr(b"bash");
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::BprmCheck,
                "/proc/bash",
                b"bash",
                Some(&digest),
                None,
            ),
            ImaAppraisalResult::pass("not-appraised")
        );
    }

    #[test]
    fn ima_appraisal_couples_evm_status_before_digest_compare() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        use crate::security::integrity::evm::EvmIntegrityStatus;

        reset_for_test();
        init();
        let policy = b"appraise func=FILE_CHECK fsname=rootfs\n";
        assert_eq!(load_policy(policy), Ok(policy.len()));
        let digest = build_digest_xattr(b"payload");

        assert_eq!(
            appraise_file_for_hook(
                ImaHook::FileCheck,
                "/etc/probe",
                b"payload",
                Some(&digest),
                Some(EvmIntegrityStatus::NoLabel),
            ),
            ImaAppraisalResult::status(ImaAppraisalStatus::NoLabel, "missing-HMAC")
        );
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::FileCheck,
                "/etc/probe",
                b"payload",
                Some(&digest),
                Some(EvmIntegrityStatus::Fail),
            ),
            ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-HMAC")
        );
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::FileCheck,
                "/etc/probe",
                b"payload",
                Some(&digest),
                Some(EvmIntegrityStatus::FailImmutable),
            ),
            ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-HMAC")
        );
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::FileCheck,
                "/etc/probe",
                b"payload",
                Some(&digest),
                Some(EvmIntegrityStatus::NoXattrs),
            ),
            ImaAppraisalResult::pass("valid-hash")
        );
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::FileCheck,
                "/etc/probe",
                b"payload",
                Some(&digest),
                Some(EvmIntegrityStatus::PassImmutable),
            ),
            ImaAppraisalResult::status(ImaAppraisalStatus::PassImmutable, "valid-immutable-HMAC")
        );
    }

    fn signature_xattr(xattr_type: u8, version: u8, hash_algo: u8, payload: &[u8]) -> Vec<u8> {
        let mut xattr = Vec::new();
        xattr.push(xattr_type);
        xattr.push(version);
        xattr.push(hash_algo);
        xattr.extend_from_slice(&0x0a0b_0c0du32.to_be_bytes());
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

    const VENDOR_RSA_SHA256_MESSAGE: &[u8] = &[
        0x49, 0x41, 0xbe, 0x0a, 0x0c, 0xc9, 0xf6, 0x35, 0x51, 0xe4, 0x27, 0x56, 0x13, 0x71, 0x4b,
        0xd0, 0x36, 0x92, 0x84, 0x89, 0x1b, 0xf8, 0x56, 0x4a, 0x72, 0x61, 0x14, 0x69, 0x4f, 0x5e,
        0x98, 0xa5, 0x80, 0x5a, 0x37, 0x51, 0x1f, 0xd8, 0xf5, 0xb5, 0x63, 0xfc, 0xf4, 0xb1, 0xbb,
        0x4d, 0x33, 0xa3, 0x1e, 0xb9, 0x75, 0x8b, 0x9c, 0xda, 0x7e, 0x6d, 0x3a, 0x77, 0x85, 0xf7,
        0xfc, 0x4e, 0xe7, 0x64, 0x43, 0x10, 0x19, 0xa0, 0x59, 0xae, 0xe0, 0xad, 0x4b, 0xd3, 0xc4,
        0x45, 0xf7, 0xb1, 0xc2, 0xc1, 0x65, 0x01, 0x41, 0x39, 0x5b, 0x45, 0x47, 0xed, 0x2b, 0x51,
        0xed, 0xe3, 0xd0, 0x09, 0x10, 0xd2, 0x39, 0x6c, 0x4a, 0x3f, 0xe5, 0xd2, 0x20, 0xe6, 0xb0,
        0x71, 0x7d, 0x5b, 0xed, 0x26, 0x60, 0xf1, 0xb4, 0x73, 0xd1, 0xdb, 0x7d, 0xc4, 0x19, 0x91,
        0xee, 0xf6, 0x32, 0x76, 0xf2, 0x19, 0x7d, 0xb7,
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
    fn ima_asymmetric_rsa_signature_xattr_accepts_vendor_linux_pkcs1_sha256_vector() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        crate::security::keys::reset_for_test();
        crate::security::keys::init();
        init();
        let policy = b"appraise func=FILE_CHECK fsname=rootfs appraise_type=imasig\n";
        assert_eq!(load_policy(policy), Ok(policy.len()));
        assert_eq!(
            sha256_digest(VENDOR_RSA_SHA256_MESSAGE).as_slice(),
            VENDOR_RSA_SHA256_DIGEST
        );
        let key = parse_rsa_public_key(VENDOR_RSA_PUBLIC_KEY).expect("vendor RSA public key");
        assert!(rsa_pkcs1_verify(
            &key,
            VENDOR_RSA_SHA256_DIGEST,
            HASH_ALGO_SHA256,
            VENDOR_RSA_SHA256_SIGNATURE
        ));
        let ima_keyring = ima_keyring_id().expect("ima keyring initialized");
        assert!(
            crate::security::keys::add_key_to_keyring(
                "asymmetric",
                "id:0a0b0c0d",
                VENDOR_RSA_PUBLIC_KEY,
                ima_keyring,
            ) > 0
        );

        let sig = signature_xattr(
            EVM_IMA_XATTR_DIGSIG,
            2,
            HASH_ALGO_SHA256,
            VENDOR_RSA_SHA256_SIGNATURE,
        );
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::FileCheck,
                "/signed/payload",
                VENDOR_RSA_SHA256_MESSAGE,
                Some(&sig),
                None,
            ),
            ImaAppraisalResult::pass("valid-signature")
        );

        let mut tampered = sig.clone();
        let last = tampered.len() - 1;
        tampered[last] ^= 0x01;
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::FileCheck,
                "/signed/payload",
                VENDOR_RSA_SHA256_MESSAGE,
                Some(&tampered),
                None,
            ),
            ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-signature")
        );
    }

    #[test]
    fn ima_signature_searches_ima_keyring_and_platform_only_for_kexec() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        crate::security::keys::reset_for_test();
        crate::security::keys::init();
        crate::security::platform_certs::reset_for_test();
        init();
        let platform = crate::security::platform_certs::init_with_uefi_signature_lists(&[])
            .expect("platform keyring")
            .platform_keyring;
        assert!(
            crate::security::keys::add_key_to_keyring(
                "asymmetric",
                "id:0a0b0c0d",
                VENDOR_RSA_PUBLIC_KEY,
                platform,
            ) > 0
        );
        let policy = b"
appraise func=FILE_CHECK fsname=rootfs appraise_type=imasig
appraise func=KEXEC_KERNEL_CHECK fsname=rootfs appraise_type=imasig
";
        assert_eq!(load_policy(policy), Ok(policy.len()));
        let sig = signature_xattr(
            EVM_IMA_XATTR_DIGSIG,
            2,
            HASH_ALGO_SHA256,
            VENDOR_RSA_SHA256_SIGNATURE,
        );

        assert_eq!(
            appraise_file_for_hook(
                ImaHook::FileCheck,
                "/signed/payload",
                VENDOR_RSA_SHA256_MESSAGE,
                Some(&sig),
                None,
            ),
            ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-signature")
        );
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::KexecKernelCheck,
                "/boot/vmlinuz",
                VENDOR_RSA_SHA256_MESSAGE,
                Some(&sig),
                None,
            ),
            ImaAppraisalResult::pass("valid-signature")
        );
    }

    #[test]
    fn ima_policy_appraise_type_requires_signatures_and_round_trips() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        init();

        let policy = b"
appraise func=FILE_CHECK fsname=rootfs appraise_type=imasig
appraise func=BPRM_CHECK uid=0 appraise_type=sigv3
appraise func=MMAP_CHECK fowner=0 appraise_type=imasig|modsig
";
        assert_eq!(load_policy(policy), Ok(policy.len()));
        assert_eq!(
            policy_text(),
            "appraise func=FILE_CHECK fsname=rootfs appraise_type=imasig\n\
appraise func=BPRM_CHECK uid=0 appraise_type=sigv3\n\
appraise func=MMAP_CHECK fowner=0 appraise_type=imasig|modsig\n"
        );

        let payload = b"signed payload";
        let digest = build_digest_xattr(payload);
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::FileCheck,
                "/signed/file",
                payload,
                Some(&digest),
                None,
            ),
            ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "IMA-signature-required")
        );
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::MmapCheck,
                "/module/object.ko",
                payload,
                Some(&digest),
                None,
            ),
            ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "IMA-signature-required")
        );

        let sig_v2 = signature_xattr(EVM_IMA_XATTR_DIGSIG, 2, HASH_ALGO_SHA1, b"sig-payload");
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::FileCheck,
                "/signed/file",
                payload,
                Some(&sig_v2),
                None,
            ),
            ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-signature")
        );
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::BprmCheck,
                "/sigv3/program",
                payload,
                Some(&sig_v2),
                None,
            ),
            ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "IMA-sigv3-required")
        );

        let sig_v3 = signature_xattr(EVM_IMA_XATTR_DIGSIG, 3, HASH_ALGO_SHA1, b"sig-payload");
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::BprmCheck,
                "/sigv3/program",
                payload,
                Some(&sig_v3),
                None,
            ),
            ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-signature")
        );
    }

    #[test]
    fn ima_signature_appraisal_follows_linux_xattr_verify_edges() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        init();
        let policy = b"appraise func=BPRM_CHECK fsname=rootfs\n";
        assert_eq!(load_policy(policy), Ok(policy.len()));

        let ima_sig = signature_xattr(EVM_IMA_XATTR_DIGSIG, 2, HASH_ALGO_SHA1, b"sig-payload");
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::BprmCheck,
                "/bin/bash",
                b"bash",
                Some(&ima_sig),
                None
            ),
            ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-signature")
        );

        let ima_sig_v1 = signature_xattr(EVM_IMA_XATTR_DIGSIG, 1, HASH_ALGO_SHA1, b"sig-payload");
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::BprmCheck,
                "/bin/bash",
                b"bash",
                Some(&ima_sig_v1),
                None,
            ),
            ImaAppraisalResult::status(ImaAppraisalStatus::Unknown, "signature-unsupported")
        );

        let ima_sig_bad_version =
            signature_xattr(EVM_IMA_XATTR_DIGSIG, 4, HASH_ALGO_SHA1, b"sig-payload");
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::BprmCheck,
                "/bin/bash",
                b"bash",
                Some(&ima_sig_bad_version),
                None,
            ),
            ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-signature-version")
        );

        let mut ima_sig_bad_size = ima_sig.clone();
        ima_sig_bad_size[7..9].copy_from_slice(&1u16.to_be_bytes());
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::BprmCheck,
                "/bin/bash",
                b"bash",
                Some(&ima_sig_bad_size),
                None,
            ),
            ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-signature")
        );

        let ima_sig_bad_hash =
            signature_xattr(EVM_IMA_XATTR_DIGSIG, 2, HASH_ALGO_LAST, b"sig-payload");
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::BprmCheck,
                "/bin/bash",
                b"bash",
                Some(&ima_sig_bad_hash),
                None,
            ),
            ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-signature")
        );

        let verity_bad_version =
            signature_xattr(IMA_VERITY_DIGSIG, 2, HASH_ALGO_SHA1, b"sig-payload");
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::BprmCheck,
                "/bin/bash",
                b"bash",
                Some(&verity_bad_version),
                None,
            ),
            ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-signature-version")
        );

        let verity_sig = signature_xattr(IMA_VERITY_DIGSIG, 3, HASH_ALGO_SHA1, b"sig-payload");
        assert_eq!(
            appraise_file_for_hook(
                ImaHook::BprmCheck,
                "/bin/bash",
                b"bash",
                Some(&verity_sig),
                None,
            ),
            ImaAppraisalResult::status(ImaAppraisalStatus::Fail, "invalid-verity-signature")
        );
    }

    #[test]
    fn ima_policy_rejects_unknown_or_oversized_rules_without_replacing_policy() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        init();

        let initial = b"measure func=FILE_CHECK fsname=rootfs\n";
        assert_eq!(load_policy(initial), Ok(initial.len()));
        assert_eq!(policy_rule_count(), 1);

        assert_eq!(
            load_policy(b"measure func=NOPE fsname=rootfs\n"),
            Err(-EINVAL)
        );
        assert_eq!(policy_text(), "measure func=FILE_CHECK fsname=rootfs\n");
        assert_eq!(
            load_policy(b"measure func=FILE_CHECK fsname=rootfs appraise_type=imasig\n"),
            Err(-EINVAL)
        );
        assert_eq!(policy_text(), "measure func=FILE_CHECK fsname=rootfs\n");
        assert_eq!(
            load_policy(b"appraise func=FILE_CHECK fsname=rootfs appraise_type=nope\n"),
            Err(-EINVAL)
        );
        assert_eq!(policy_text(), "measure func=FILE_CHECK fsname=rootfs\n");
        assert_eq!(
            load_policy(
                b"appraise func=FILE_CHECK fsname=rootfs appraise_type=imasig appraise_type=sigv3\n"
            ),
            Err(-EINVAL)
        );
        assert_eq!(policy_text(), "measure func=FILE_CHECK fsname=rootfs\n");

        let oversized = alloc::vec![b'x'; IMA_POLICY_MAX_BYTES + 1];
        assert_eq!(load_policy(&oversized), Err(-E2BIG));
        assert_eq!(policy_rule_count(), 1);
    }

    #[test]
    fn ima_inode_measurement_covers_byte_backed_regular_files() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::lsm_list::reset_for_test();
        reset_for_test();
        init();
        let active_policy = b"measure func=FILE_CHECK mask=MAY_READ fsname=rootfs\n";
        assert_eq!(load_policy(active_policy), Ok(active_policy.len()));

        let static_inode = Inode::new(
            1,
            InodeKind::Regular,
            0o444,
            &NOOP_INODE_OPS,
            &NOOP_FILE_OPS,
            InodePrivate::StaticBytes(b"static file"),
        );
        assert!(
            measure_inode_private("/etc/static-probe", &static_inode.private)
                .expect("measure static bytes")
        );

        let ram_inode = Inode::new(
            2,
            InodeKind::Regular,
            0o644,
            &NOOP_INODE_OPS,
            &NOOP_FILE_OPS,
            InodePrivate::RamBytes(Mutex::new(Vec::from(&b"ram file"[..]))),
        );
        assert!(
            measure_inode_private("/var/lib/ram-probe", &ram_inode.private)
                .expect("measure ram bytes")
        );

        let cow_inode = Inode::new(
            3,
            InodeKind::Regular,
            0o644,
            &NOOP_INODE_OPS,
            &NOOP_FILE_OPS,
            InodePrivate::StaticCowBytes {
                base: b"base file",
                overlay: Mutex::new(Some(Vec::from(&b"overlay file"[..]))),
            },
        );
        assert!(
            measure_inode_private("/usr/bin/cow-probe", &cow_inode.private)
                .expect("measure cow overlay")
        );

        let ascii = ascii_runtime_measurements_sha1();
        assert!(ascii.contains("/etc/static-probe"));
        assert!(ascii.contains("/var/lib/ram-probe"));
        assert!(ascii.contains("/usr/bin/cow-probe"));
        assert!(ascii.contains(&digest_hex(&sha1_digest(b"overlay file"))));
        assert!(!ascii.contains(&digest_hex(&sha1_digest(b"base file"))));
    }

    #[test]
    fn ima_opened_inode_measurement_requires_an_active_vendor_policy() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::lsm_list::reset_for_test();
        reset_for_test();
        init();
        set_file_hook_measurements_for_test(true);

        let inode = Inode::new(
            4,
            InodeKind::Regular,
            0o444,
            &NOOP_INODE_OPS,
            &NOOP_FILE_OPS,
            InodePrivate::StaticBytes(b"securityfs self-read"),
        );
        assert!(
            !measure_opened_inode(
                "/sys/kernel/security/integrity/ima/ascii_runtime_measurements_sha1",
                &inode
            )
            .expect("skip securityfs")
        );
        assert!(!measure_opened_inode("/proc/self/status", &inode).expect("skip proc"));
        assert!(
            !measure_opened_inode("/etc/passwd", &inode).expect("empty policy disables file hook")
        );
        let active_policy = b"measure func=FILE_CHECK mask=MAY_READ fsname=rootfs\n";
        assert_eq!(load_policy(active_policy), Ok(active_policy.len()));
        assert!(measure_opened_inode("/etc/passwd", &inode).expect("measure regular path"));

        let dir = Inode::new(
            5,
            InodeKind::Directory,
            0o755,
            &NOOP_INODE_OPS,
            &NOOP_FILE_OPS,
            InodePrivate::None,
        );
        assert!(!measure_opened_inode("/etc", &dir).expect("skip directory"));

        let ascii = ascii_runtime_measurements_sha1();
        assert!(ascii.contains("/etc/passwd"));
        assert!(!ascii.contains("/proc/self/status"));
        assert!(!ascii.contains("ascii_runtime_measurements_sha1"));

        let vendor = include_str!("../../../vendor/linux/security/integrity/ima/ima_main.c");
        assert!(vendor.contains("if (!ima_policy_flag || !S_ISREG(inode->i_mode))"));
    }
}
