//! linux-parity: complete
//! linux-source: vendor/linux/lib/audit.c
//! test-origin: linux:vendor/linux/lib/audit.c
//! Native audit syscall classification.

pub use crate::lib::compat_audit::AuditScClass;

pub const AUDIT_ARCH_X86_64: u32 = 0xC000_003E;
pub const AUDIT_ARCH_I386: u32 = 0x4000_0003;

pub const __NR_OPEN: u32 = 2;
pub const __NR_BIND: u32 = 49;
pub const __NR_EXECVE: u32 = 59;
pub const __NR_KILL: u32 = 62;
pub const __NR_TRUNCATE: u32 = 76;
pub const __NR_FTRUNCATE: u32 = 77;
pub const __NR_RENAME: u32 = 82;
pub const __NR_MKDIR: u32 = 83;
pub const __NR_RMDIR: u32 = 84;
pub const __NR_CREAT: u32 = 85;
pub const __NR_LINK: u32 = 86;
pub const __NR_UNLINK: u32 = 87;
pub const __NR_SYMLINK: u32 = 88;
pub const __NR_READLINK: u32 = 89;
pub const __NR_CHMOD: u32 = 90;
pub const __NR_FCHMOD: u32 = 91;
pub const __NR_CHOWN: u32 = 92;
pub const __NR_FCHOWN: u32 = 93;
pub const __NR_LCHOWN: u32 = 94;
pub const __NR_MKNOD: u32 = 133;
pub const __NR_ACCT: u32 = 163;
pub const __NR_SWAPON: u32 = 167;
pub const __NR_QUOTACTL: u32 = 179;
pub const __NR_SETXATTR: u32 = 188;
pub const __NR_LSETXATTR: u32 = 189;
pub const __NR_FSETXATTR: u32 = 190;
pub const __NR_GETXATTR: u32 = 191;
pub const __NR_LGETXATTR: u32 = 192;
pub const __NR_FGETXATTR: u32 = 193;
pub const __NR_LISTXATTR: u32 = 194;
pub const __NR_LLISTXATTR: u32 = 195;
pub const __NR_FLISTXATTR: u32 = 196;
pub const __NR_REMOVEXATTR: u32 = 197;
pub const __NR_LREMOVEXATTR: u32 = 198;
pub const __NR_FREMOVEXATTR: u32 = 199;
pub const __NR_TKILL: u32 = 200;
pub const __NR_TGKILL: u32 = 234;
pub const __NR_OPENAT: u32 = 257;
pub const __NR_MKDIRAT: u32 = 258;
pub const __NR_MKNODAT: u32 = 259;
pub const __NR_FCHOWNAT: u32 = 260;
pub const __NR_UNLINKAT: u32 = 263;
pub const __NR_RENAMEAT: u32 = 264;
pub const __NR_LINKAT: u32 = 265;
pub const __NR_SYMLINKAT: u32 = 266;
pub const __NR_READLINKAT: u32 = 267;
pub const __NR_FCHMODAT: u32 = 268;
pub const __NR_FALLOCATE: u32 = 285;
pub const __NR_RENAMEAT2: u32 = 316;
pub const __NR_EXECVEAT: u32 = 322;
pub const __NR_OPENAT2: u32 = 437;
pub const __NR_FCHMODAT2: u32 = 452;
pub const __NR_SETXATTRAT: u32 = 463;
pub const __NR_GETXATTRAT: u32 = 464;
pub const __NR_LISTXATTRAT: u32 = 465;
pub const __NR_REMOVEXATTRAT: u32 = 466;

pub const AUDIT_CLASS_DIR_WRITE: u32 = 0;
pub const AUDIT_CLASS_DIR_WRITE_32: u32 = 1;
pub const AUDIT_CLASS_CHATTR: u32 = 2;
pub const AUDIT_CLASS_CHATTR_32: u32 = 3;
pub const AUDIT_CLASS_READ: u32 = 4;
pub const AUDIT_CLASS_READ_32: u32 = 5;
pub const AUDIT_CLASS_WRITE: u32 = 6;
pub const AUDIT_CLASS_WRITE_32: u32 = 7;
pub const AUDIT_CLASS_SIGNAL: u32 = 8;
pub const AUDIT_CLASS_SIGNAL_32: u32 = 9;

pub const CLASS_SENTINEL: u32 = u32::MAX;
pub const DIR_CLASS: &[u32] = &[
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
    CLASS_SENTINEL,
];
pub const READ_CLASS: &[u32] = &[
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
    CLASS_SENTINEL,
];
pub const WRITE_CLASS: &[u32] = &[
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
    __NR_FTRUNCATE,
    __NR_BIND,
    __NR_FALLOCATE,
    CLASS_SENTINEL,
];
pub const CHATTR_CLASS: &[u32] = &[
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
    __NR_LINK,
    __NR_LINKAT,
    CLASS_SENTINEL,
];
pub const SIGNAL_CLASS: &[u32] = &[__NR_KILL, __NR_TGKILL, __NR_TKILL, CLASS_SENTINEL];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuditClassRegistration {
    pub class: u32,
    pub syscalls: &'static [u32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuditClassesInitReport {
    pub compat_generic_enabled: bool,
    pub compat_registered: &'static [AuditClassRegistration],
    pub native_registered: &'static [AuditClassRegistration],
    pub return_code: i32,
}

pub const NATIVE_AUDIT_CLASS_REGISTRATIONS: &[AuditClassRegistration] = &[
    AuditClassRegistration {
        class: AUDIT_CLASS_WRITE,
        syscalls: WRITE_CLASS,
    },
    AuditClassRegistration {
        class: AUDIT_CLASS_READ,
        syscalls: READ_CLASS,
    },
    AuditClassRegistration {
        class: AUDIT_CLASS_DIR_WRITE,
        syscalls: DIR_CLASS,
    },
    AuditClassRegistration {
        class: AUDIT_CLASS_CHATTR,
        syscalls: CHATTR_CLASS,
    },
    AuditClassRegistration {
        class: AUDIT_CLASS_SIGNAL,
        syscalls: SIGNAL_CLASS,
    },
];

pub const COMPAT_AUDIT_CLASS_REGISTRATIONS: &[AuditClassRegistration] = &[
    AuditClassRegistration {
        class: AUDIT_CLASS_WRITE_32,
        syscalls: crate::lib::compat_audit::COMPAT_WRITE_CLASS,
    },
    AuditClassRegistration {
        class: AUDIT_CLASS_READ_32,
        syscalls: crate::lib::compat_audit::COMPAT_READ_CLASS,
    },
    AuditClassRegistration {
        class: AUDIT_CLASS_DIR_WRITE_32,
        syscalls: crate::lib::compat_audit::COMPAT_DIR_CLASS,
    },
    AuditClassRegistration {
        class: AUDIT_CLASS_CHATTR_32,
        syscalls: crate::lib::compat_audit::COMPAT_CHATTR_CLASS,
    },
    AuditClassRegistration {
        class: AUDIT_CLASS_SIGNAL_32,
        syscalls: crate::lib::compat_audit::COMPAT_SIGNAL_CLASS,
    },
];

pub const fn audit_classes_init(compat_generic_enabled: bool) -> AuditClassesInitReport {
    AuditClassesInitReport {
        compat_generic_enabled,
        compat_registered: if compat_generic_enabled {
            COMPAT_AUDIT_CLASS_REGISTRATIONS
        } else {
            &[]
        },
        native_registered: NATIVE_AUDIT_CLASS_REGISTRATIONS,
        return_code: 0,
    }
}

pub const fn audit_is_compat(arch: u32) -> bool {
    arch != AUDIT_ARCH_X86_64
}

pub const fn audit_classify_arch(arch: u32) -> i32 {
    if audit_is_compat(arch) { 1 } else { 0 }
}

pub fn audit_classify_syscall(abi: u32, syscall: u32) -> AuditScClass {
    if audit_is_compat(abi) {
        return crate::lib::compat_audit::audit_classify_compat_syscall(abi as i32, syscall);
    }
    match syscall {
        __NR_OPEN => AuditScClass::Open,
        __NR_OPENAT => AuditScClass::OpenAt,
        __NR_EXECVE | __NR_EXECVEAT => AuditScClass::Execve,
        __NR_OPENAT2 => AuditScClass::OpenAt2,
        _ => AuditScClass::Native,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_classifier_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/audit.c"
        ));
        let audit_h = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/audit.h"
        ));
        let dir_write = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/asm-generic/audit_dir_write.h"
        ));
        let read = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/asm-generic/audit_read.h"
        ));
        let write = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/asm-generic/audit_write.h"
        ));
        let chattr = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/asm-generic/audit_change_attr.h"
        ));
        let signal = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/asm-generic/audit_signal.h"
        ));
        assert!(source.contains("static unsigned dir_class[]"));
        assert!(source.contains("#include <asm-generic/audit_dir_write.h>"));
        assert!(source.contains("static unsigned read_class[]"));
        assert!(source.contains("static unsigned write_class[]"));
        assert!(source.contains("static unsigned chattr_class[]"));
        assert!(source.contains("static unsigned signal_class[]"));
        assert!(source.contains("if (audit_is_compat(arch))"));
        assert!(source.contains("return audit_classify_compat_syscall(abi, syscall);"));
        assert!(source.contains("case __NR_openat2:"));
        assert!(source.contains("audit_register_class(AUDIT_CLASS_WRITE, write_class);"));
        assert!(source.contains("audit_register_class(AUDIT_CLASS_READ, read_class);"));
        assert!(source.contains("audit_register_class(AUDIT_CLASS_DIR_WRITE, dir_class);"));
        assert!(source.contains("audit_register_class(AUDIT_CLASS_CHATTR, chattr_class);"));
        assert!(source.contains("audit_register_class(AUDIT_CLASS_SIGNAL, signal_class);"));
        assert!(source.contains("__initcall(audit_classes_init);"));
        assert!(audit_h.contains("#define AUDIT_CLASS_DIR_WRITE 0"));
        assert!(audit_h.contains("#define AUDIT_CLASS_SIGNAL_32 9"));
        assert!(dir_write.contains("__NR_rename,"));
        assert!(dir_write.contains("__NR_renameat2,"));
        assert!(read.contains("__NR_getxattr,"));
        assert!(read.contains("__NR_getxattrat,"));
        assert!(write.contains("#include <asm-generic/audit_dir_write.h>"));
        assert!(write.contains("__NR_fallocate,"));
        assert!(chattr.contains("__NR_fchmodat2,"));
        assert!(signal.contains("__NR_kill,"));

        assert_eq!(audit_classify_arch(AUDIT_ARCH_X86_64), 0);
        assert_eq!(audit_classify_arch(AUDIT_ARCH_I386), 1);
        assert_eq!(
            audit_classify_syscall(AUDIT_ARCH_X86_64, __NR_OPEN),
            AuditScClass::Open
        );
        assert_eq!(
            audit_classify_syscall(AUDIT_ARCH_X86_64, __NR_OPENAT),
            AuditScClass::OpenAt
        );
        assert_eq!(
            audit_classify_syscall(AUDIT_ARCH_X86_64, __NR_EXECVEAT),
            AuditScClass::Execve
        );
        assert_eq!(
            audit_classify_syscall(AUDIT_ARCH_X86_64, __NR_OPENAT2),
            AuditScClass::OpenAt2
        );
        assert_eq!(
            audit_classify_syscall(AUDIT_ARCH_X86_64, 9999),
            AuditScClass::Native
        );
        assert_eq!(
            audit_classify_syscall(AUDIT_ARCH_I386, crate::lib::compat_audit::__NR_OPEN),
            AuditScClass::Open
        );
        assert_eq!(AUDIT_CLASS_DIR_WRITE, 0);
        assert_eq!(AUDIT_CLASS_SIGNAL_32, 9);
        assert_eq!(DIR_CLASS.last(), Some(&CLASS_SENTINEL));
        assert!(DIR_CLASS.contains(&__NR_RENAME));
        assert!(DIR_CLASS.contains(&__NR_RENAMEAT2));
        assert!(READ_CLASS.contains(&__NR_READLINK));
        assert!(READ_CLASS.contains(&__NR_GETXATTRAT));
        assert_eq!(WRITE_CLASS.last(), Some(&CLASS_SENTINEL));
        assert!(WRITE_CLASS.contains(&__NR_RENAME));
        assert!(WRITE_CLASS.contains(&__NR_TRUNCATE));
        assert!(WRITE_CLASS.contains(&__NR_FALLOCATE));
        assert!(CHATTR_CLASS.contains(&__NR_SETXATTRAT));
        assert!(CHATTR_CLASS.contains(&__NR_FCHMODAT2));
        assert_eq!(
            SIGNAL_CLASS,
            &[__NR_KILL, __NR_TGKILL, __NR_TKILL, CLASS_SENTINEL]
        );
        assert_eq!(
            NATIVE_AUDIT_CLASS_REGISTRATIONS,
            &[
                AuditClassRegistration {
                    class: AUDIT_CLASS_WRITE,
                    syscalls: WRITE_CLASS,
                },
                AuditClassRegistration {
                    class: AUDIT_CLASS_READ,
                    syscalls: READ_CLASS,
                },
                AuditClassRegistration {
                    class: AUDIT_CLASS_DIR_WRITE,
                    syscalls: DIR_CLASS,
                },
                AuditClassRegistration {
                    class: AUDIT_CLASS_CHATTR,
                    syscalls: CHATTR_CLASS,
                },
                AuditClassRegistration {
                    class: AUDIT_CLASS_SIGNAL,
                    syscalls: SIGNAL_CLASS,
                },
            ]
        );
        let native_only = audit_classes_init(false);
        assert!(!native_only.compat_generic_enabled);
        assert!(native_only.compat_registered.is_empty());
        assert_eq!(
            native_only.native_registered,
            NATIVE_AUDIT_CLASS_REGISTRATIONS
        );
        assert_eq!(native_only.return_code, 0);
        let with_compat = audit_classes_init(true);
        assert_eq!(
            with_compat.compat_registered,
            COMPAT_AUDIT_CLASS_REGISTRATIONS
        );
        assert_eq!(with_compat.compat_registered[0].class, AUDIT_CLASS_WRITE_32);
        assert_eq!(with_compat.native_registered[4].class, AUDIT_CLASS_SIGNAL);
    }
}
