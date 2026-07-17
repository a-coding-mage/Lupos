//! linux-parity: complete
//! linux-source: vendor/linux/lib/compat_audit.c
//! test-origin: linux:vendor/linux/lib/compat_audit.c
//! Compat audit syscall classification.

#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditScClass {
    Native = 0,
    Compat = 1,
    Open = 2,
    OpenAt = 3,
    SocketCall = 4,
    Execve = 5,
    OpenAt2 = 6,
}

pub const __NR_OPEN: u32 = 5;
pub const __NR_CREAT: u32 = 8;
pub const __NR_LINK: u32 = 9;
pub const __NR_UNLINK: u32 = 10;
pub const __NR_EXECVE: u32 = 11;
pub const __NR_MKNOD: u32 = 14;
pub const __NR_CHMOD: u32 = 15;
pub const __NR_LCHOWN: u32 = 16;
pub const __NR_KILL: u32 = 37;
pub const __NR_RENAME: u32 = 38;
pub const __NR_MKDIR: u32 = 39;
pub const __NR_RMDIR: u32 = 40;
pub const __NR_ACCT: u32 = 51;
pub const __NR_SYMLINK: u32 = 83;
pub const __NR_READLINK: u32 = 85;
pub const __NR_SWAPON: u32 = 87;
pub const __NR_TRUNCATE: u32 = 92;
pub const __NR_FTRUNCATE: u32 = 93;
pub const __NR_FCHMOD: u32 = 94;
pub const __NR_FCHOWN: u32 = 95;
pub const __NR_SOCKETCALL: u32 = 102;
pub const __NR_QUOTACTL: u32 = 131;
pub const __NR_CHOWN: u32 = 182;
pub const __NR_TRUNCATE64: u32 = 193;
pub const __NR_FTRUNCATE64: u32 = 194;
pub const __NR_LCHOWN32: u32 = 198;
pub const __NR_FCHOWN32: u32 = 207;
pub const __NR_CHOWN32: u32 = 212;
pub const __NR_SETXATTR: u32 = 226;
pub const __NR_LSETXATTR: u32 = 227;
pub const __NR_FSETXATTR: u32 = 228;
pub const __NR_GETXATTR: u32 = 229;
pub const __NR_LGETXATTR: u32 = 230;
pub const __NR_FGETXATTR: u32 = 231;
pub const __NR_LISTXATTR: u32 = 232;
pub const __NR_LLISTXATTR: u32 = 233;
pub const __NR_FLISTXATTR: u32 = 234;
pub const __NR_REMOVEXATTR: u32 = 235;
pub const __NR_LREMOVEXATTR: u32 = 236;
pub const __NR_FREMOVEXATTR: u32 = 237;
pub const __NR_TKILL: u32 = 238;
pub const __NR_TGKILL: u32 = 270;
pub const __NR_OPENAT: u32 = 295;
pub const __NR_MKDIRAT: u32 = 296;
pub const __NR_MKNODAT: u32 = 297;
pub const __NR_FCHOWNAT: u32 = 298;
pub const __NR_UNLINKAT: u32 = 301;
pub const __NR_RENAMEAT: u32 = 302;
pub const __NR_LINKAT: u32 = 303;
pub const __NR_SYMLINKAT: u32 = 304;
pub const __NR_READLINKAT: u32 = 305;
pub const __NR_FCHMODAT: u32 = 306;
pub const __NR_FALLOCATE: u32 = 324;
pub const __NR_RENAMEAT2: u32 = 353;
pub const __NR_BIND: u32 = 361;
pub const __NR_OPENAT2: u32 = 437;
pub const __NR_FCHMODAT2: u32 = 452;
pub const __NR_SETXATTRAT: u32 = 463;
pub const __NR_GETXATTRAT: u32 = 464;
pub const __NR_LISTXATTRAT: u32 = 465;
pub const __NR_REMOVEXATTRAT: u32 = 466;

pub const COMPAT_CLASS_SENTINEL: u32 = u32::MAX;

pub const COMPAT_DIR_CLASS: &[u32] = &[
    __NR_RENAME,
    __NR_MKDIR,
    __NR_RMDIR,
    __NR_CREAT,
    __NR_LINK,
    __NR_UNLINK,
    __NR_SYMLINK,
    __NR_MKNOD,
    __NR_MKDIRAT,
    __NR_MKNODAT,
    __NR_UNLINKAT,
    __NR_RENAMEAT,
    __NR_LINKAT,
    __NR_SYMLINKAT,
    __NR_RENAMEAT2,
    COMPAT_CLASS_SENTINEL,
];

pub const COMPAT_READ_CLASS: &[u32] = &[
    __NR_READLINK,
    __NR_QUOTACTL,
    __NR_LISTXATTR,
    __NR_LISTXATTRAT,
    __NR_LLISTXATTR,
    __NR_FLISTXATTR,
    __NR_GETXATTR,
    __NR_GETXATTRAT,
    __NR_LGETXATTR,
    __NR_FGETXATTR,
    __NR_READLINKAT,
    COMPAT_CLASS_SENTINEL,
];

pub const COMPAT_WRITE_CLASS: &[u32] = &[
    __NR_RENAME,
    __NR_MKDIR,
    __NR_RMDIR,
    __NR_CREAT,
    __NR_LINK,
    __NR_UNLINK,
    __NR_SYMLINK,
    __NR_MKNOD,
    __NR_MKDIRAT,
    __NR_MKNODAT,
    __NR_UNLINKAT,
    __NR_RENAMEAT,
    __NR_LINKAT,
    __NR_SYMLINKAT,
    __NR_RENAMEAT2,
    __NR_ACCT,
    __NR_SWAPON,
    __NR_QUOTACTL,
    __NR_TRUNCATE,
    __NR_TRUNCATE64,
    __NR_FTRUNCATE,
    __NR_FTRUNCATE64,
    __NR_BIND,
    __NR_FALLOCATE,
    COMPAT_CLASS_SENTINEL,
];

pub const COMPAT_CHATTR_CLASS: &[u32] = &[
    __NR_CHMOD,
    __NR_FCHMOD,
    __NR_CHOWN,
    __NR_LCHOWN,
    __NR_FCHOWN,
    __NR_SETXATTR,
    __NR_SETXATTRAT,
    __NR_LSETXATTR,
    __NR_FSETXATTR,
    __NR_REMOVEXATTR,
    __NR_REMOVEXATTRAT,
    __NR_LREMOVEXATTR,
    __NR_FREMOVEXATTR,
    __NR_FCHOWNAT,
    __NR_FCHMODAT,
    __NR_FCHMODAT2,
    __NR_CHOWN32,
    __NR_FCHOWN32,
    __NR_LCHOWN32,
    __NR_LINK,
    __NR_LINKAT,
    COMPAT_CLASS_SENTINEL,
];

pub const COMPAT_SIGNAL_CLASS: &[u32] =
    &[__NR_KILL, __NR_TGKILL, __NR_TKILL, COMPAT_CLASS_SENTINEL];

pub const fn audit_classify_compat_syscall(_abi: i32, syscall: u32) -> AuditScClass {
    match syscall {
        __NR_OPEN => AuditScClass::Open,
        __NR_OPENAT => AuditScClass::OpenAt,
        __NR_SOCKETCALL => AuditScClass::SocketCall,
        __NR_EXECVE => AuditScClass::Execve,
        __NR_OPENAT2 => AuditScClass::OpenAt2,
        _ => AuditScClass::Compat,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compat_audit_classifier_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/compat_audit.c"
        ));
        assert!(source.contains("unsigned int compat_dir_class[]"));
        assert!(source.contains("#include <asm-generic/audit_dir_write.h>"));
        assert!(source.contains("unsigned int compat_read_class[]"));
        assert!(source.contains("unsigned int compat_write_class[]"));
        assert!(source.contains("unsigned int compat_chattr_class[]"));
        assert!(source.contains("unsigned int compat_signal_class[]"));
        assert!(source.contains("~0U"));
        assert!(source.contains("case __NR_open:"));
        assert!(source.contains("case __NR_openat:"));
        assert!(source.contains("case __NR_socketcall:"));
        assert!(source.contains("case __NR_execve:"));
        assert!(source.contains("case __NR_openat2:"));
        assert!(source.contains("return AUDITSC_COMPAT;"));

        assert_eq!(
            audit_classify_compat_syscall(0, __NR_OPEN),
            AuditScClass::Open
        );
        assert_eq!(
            audit_classify_compat_syscall(0, __NR_OPENAT),
            AuditScClass::OpenAt
        );
        assert_eq!(
            audit_classify_compat_syscall(0, __NR_SOCKETCALL),
            AuditScClass::SocketCall
        );
        assert_eq!(
            audit_classify_compat_syscall(0, __NR_EXECVE),
            AuditScClass::Execve
        );
        assert_eq!(
            audit_classify_compat_syscall(0, __NR_OPENAT2),
            AuditScClass::OpenAt2
        );
        assert_eq!(audit_classify_compat_syscall(0, 9999), AuditScClass::Compat);
        assert_eq!(COMPAT_CLASS_SENTINEL, u32::MAX);
        assert_eq!(COMPAT_DIR_CLASS.last(), Some(&COMPAT_CLASS_SENTINEL));
        assert!(COMPAT_DIR_CLASS.contains(&__NR_RENAMEAT2));
        assert!(COMPAT_READ_CLASS.contains(&__NR_GETXATTRAT));
        assert!(COMPAT_WRITE_CLASS.contains(&__NR_FALLOCATE));
        assert!(COMPAT_CHATTR_CLASS.contains(&__NR_FCHMODAT2));
        assert_eq!(
            COMPAT_SIGNAL_CLASS,
            &[__NR_KILL, __NR_TGKILL, __NR_TKILL, COMPAT_CLASS_SENTINEL]
        );
    }
}
