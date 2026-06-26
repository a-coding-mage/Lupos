//! linux-parity: complete
//! linux-source: vendor/linux/security/integrity/integrity_audit.c
//! test-origin: linux:vendor/linux/security/integrity/integrity_audit.c
//! Integrity audit message formatting and filtering.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

static INTEGRITY_AUDIT_INFO: AtomicBool = AtomicBool::new(false);

lazy_static! {
    static ref AUDIT_LOG: Mutex<Vec<IntegrityAuditRecord>> = Mutex::new(Vec::new());
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntegrityAuditRecord {
    pub audit_msgno: i32,
    pub pid: u32,
    pub uid: u32,
    pub auid: u32,
    pub session_id: u32,
    pub op: String,
    pub cause: String,
    pub comm: String,
    pub name: Option<String>,
    pub dev: Option<String>,
    pub ino: Option<u64>,
    pub res: i32,
    pub errno: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntegrityAuditInode<'a> {
    pub dev: &'a str,
    pub ino: u64,
}

pub fn integrity_audit_setup(value: &str) -> i32 {
    if let Some(audit) = parse_u64_auto_radix(value) {
        INTEGRITY_AUDIT_INFO.store(audit != 0, Ordering::Release);
    }
    1
}

fn parse_u64_auto_radix(value: &str) -> Option<u64> {
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16).ok()
    } else if value.starts_with('0') && value.len() > 1 {
        u64::from_str_radix(&value[1..], 8).ok()
    } else {
        value.parse::<u64>().ok()
    }
}

pub fn integrity_audit_msg(
    audit_msgno: i32,
    inode: Option<IntegrityAuditInode<'_>>,
    fname: Option<&str>,
    op: &str,
    cause: &str,
    result: i32,
    audit_info: i32,
) -> Option<IntegrityAuditRecord> {
    integrity_audit_message(audit_msgno, inode, fname, op, cause, result, audit_info, 0)
}

#[allow(clippy::too_many_arguments)]
pub fn integrity_audit_message(
    audit_msgno: i32,
    inode: Option<IntegrityAuditInode<'_>>,
    fname: Option<&str>,
    op: &str,
    cause: &str,
    result: i32,
    audit_info: i32,
    errno: i32,
) -> Option<IntegrityAuditRecord> {
    if !INTEGRITY_AUDIT_INFO.load(Ordering::Acquire) && audit_info == 1 {
        return None;
    }

    let record = IntegrityAuditRecord {
        audit_msgno,
        pid: 0,
        uid: 0,
        auid: 0,
        session_id: 0,
        op: String::from(op),
        cause: String::from(cause),
        comm: String::from("lupos"),
        name: fname.map(String::from),
        dev: inode.map(|inode| String::from(inode.dev)),
        ino: inode.map(|inode| inode.ino),
        res: if result == 0 { 1 } else { 0 },
        errno,
    };
    AUDIT_LOG.lock().push(record.clone());
    Some(record)
}

pub fn audit_log_snapshot() -> Vec<IntegrityAuditRecord> {
    AUDIT_LOG.lock().clone()
}

#[cfg(test)]
pub fn reset_for_test() {
    INTEGRITY_AUDIT_INFO.store(false, Ordering::Release);
    AUDIT_LOG.lock().clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrity_audit_filters_info_and_formats_result_edges() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();

        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/integrity/integrity_audit.c"
        ));
        assert!(source.contains("__setup(\"integrity_audit=\", integrity_audit_setup)"));
        assert!(source.contains("if (!integrity_audit_info && audit_info == 1)"));
        assert!(source.contains("audit_log_format(ab, \" op=%s cause=%s comm=\""));
        assert!(source.contains("audit_log_format(ab, \" res=%d errno=%d\", !result, errno)"));

        assert_eq!(integrity_audit_setup("0"), 1);
        assert!(integrity_audit_msg(1800, None, Some("file"), "appraise", "ok", 0, 1).is_none());
        assert!(audit_log_snapshot().is_empty());

        assert_eq!(integrity_audit_setup("1"), 1);
        let record = integrity_audit_message(
            1800,
            Some(IntegrityAuditInode {
                dev: "sda1",
                ino: 42,
            }),
            Some("/bin/app"),
            "appraise",
            "invalid-signature",
            -1,
            1,
            -13,
        )
        .expect("record");
        assert_eq!(record.name.as_deref(), Some("/bin/app"));
        assert_eq!(record.dev.as_deref(), Some("sda1"));
        assert_eq!(record.ino, Some(42));
        assert_eq!(record.res, 0);
        assert_eq!(record.errno, -13);
        assert_eq!(audit_log_snapshot().len(), 1);
    }
}
