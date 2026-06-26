//! linux-parity: complete
//! linux-source: vendor/linux/fs/dlm/util.c
//! test-origin: linux:vendor/linux/fs/dlm/util.c
//! DLM wire-stable errno conversion helpers.

use crate::include::uapi::errno::{EDEADLK, EOPNOTSUPP, EPROTO, ETIMEDOUT};

pub const EBADR: i32 = 53;
pub const EBADSLT: i32 = 57;
pub const EINPROGRESS: i32 = 115;

pub const DLM_ERRNO_EDEADLK: i32 = 35;
pub const DLM_ERRNO_EBADR: i32 = 53;
pub const DLM_ERRNO_EBADSLT: i32 = 57;
pub const DLM_ERRNO_EPROTO: i32 = 71;
pub const DLM_ERRNO_EOPNOTSUPP: i32 = 95;
pub const DLM_ERRNO_ETIMEDOUT: i32 = 110;
pub const DLM_ERRNO_EINPROGRESS: i32 = 115;

pub const fn to_dlm_errno(err: i32) -> i32 {
    match err {
        e if e == -EDEADLK => -DLM_ERRNO_EDEADLK,
        e if e == -EBADR => -DLM_ERRNO_EBADR,
        e if e == -EBADSLT => -DLM_ERRNO_EBADSLT,
        e if e == -EPROTO => -DLM_ERRNO_EPROTO,
        e if e == -EOPNOTSUPP => -DLM_ERRNO_EOPNOTSUPP,
        e if e == -ETIMEDOUT => -DLM_ERRNO_ETIMEDOUT,
        e if e == -EINPROGRESS => -DLM_ERRNO_EINPROGRESS,
        _ => err,
    }
}

pub const fn from_dlm_errno(err: i32) -> i32 {
    match err {
        e if e == -DLM_ERRNO_EDEADLK => -EDEADLK,
        e if e == -DLM_ERRNO_EBADR => -EBADR,
        e if e == -DLM_ERRNO_EBADSLT => -EBADSLT,
        e if e == -DLM_ERRNO_EPROTO => -EPROTO,
        e if e == -DLM_ERRNO_EOPNOTSUPP => -EOPNOTSUPP,
        e if e == -DLM_ERRNO_ETIMEDOUT => -ETIMEDOUT,
        e if e == -DLM_ERRNO_EINPROGRESS => -EINPROGRESS,
        _ => err,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dlm_errno_conversions_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/dlm/util.c"
        ));
        assert!(source.contains("#include \"dlm_internal.h\""));
        assert!(source.contains("#include \"rcom.h\""));
        assert!(source.contains("#include \"util.h\""));
        assert!(source.contains("#define DLM_ERRNO_EDEADLK\t\t35"));
        assert!(source.contains("#define DLM_ERRNO_EBADR\t\t\t53"));
        assert!(source.contains("#define DLM_ERRNO_EBADSLT\t\t57"));
        assert!(source.contains("#define DLM_ERRNO_EPROTO\t\t71"));
        assert!(source.contains("#define DLM_ERRNO_EOPNOTSUPP\t\t95"));
        assert!(source.contains("#define DLM_ERRNO_ETIMEDOUT\t       110"));
        assert!(source.contains("#define DLM_ERRNO_EINPROGRESS\t       115"));
        assert!(source.contains("int to_dlm_errno"));
        assert!(source.contains("case -EDEADLK:"));
        assert!(source.contains("return -DLM_ERRNO_EDEADLK;"));
        assert!(source.contains("int from_dlm_errno"));
        assert!(source.contains("case -DLM_ERRNO_EINPROGRESS:"));
        assert!(source.contains("return -EINPROGRESS;"));

        assert_eq!(to_dlm_errno(-EDEADLK), -DLM_ERRNO_EDEADLK);
        assert_eq!(to_dlm_errno(-EBADR), -DLM_ERRNO_EBADR);
        assert_eq!(to_dlm_errno(-1234), -1234);
        assert_eq!(from_dlm_errno(-DLM_ERRNO_EBADSLT), -EBADSLT);
        assert_eq!(from_dlm_errno(-DLM_ERRNO_EINPROGRESS), -EINPROGRESS);
        assert_eq!(from_dlm_errno(-1234), -1234);
    }
}
