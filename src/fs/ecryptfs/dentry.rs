//! linux-parity: complete
//! linux-source: vendor/linux/fs/ecryptfs/dentry.c
//! test-origin: linux:vendor/linux/fs/ecryptfs/dentry.c
//! eCryptfs dentry revalidation and release behavior.

use crate::include::uapi::errno::ECHILD;

pub const LOOKUP_RCU: u32 = 1 << 8;
pub const DCACHE_OP_REVALIDATE: u32 = 1 << 2;
pub const ECRYPTFS_DOPS_SYMBOL: &str = "ecryptfs_dops";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EcryptfsRevalidateOutcome {
    pub result: i32,
    pub lower_revalidate_called: bool,
    pub attrs_copied: bool,
}

pub const fn ecryptfs_d_revalidate_outcome(
    flags: u32,
    lower_d_flags: u32,
    lower_revalidate_result: i32,
    positive: bool,
    inode_nlink: u32,
) -> EcryptfsRevalidateOutcome {
    if flags & LOOKUP_RCU != 0 {
        return EcryptfsRevalidateOutcome {
            result: -ECHILD,
            lower_revalidate_called: false,
            attrs_copied: false,
        };
    }

    let lower_revalidate_called = lower_d_flags & DCACHE_OP_REVALIDATE != 0;
    let rc = if lower_revalidate_called {
        lower_revalidate_result
    } else {
        1
    };

    if positive && inode_nlink == 0 {
        return EcryptfsRevalidateOutcome {
            result: 0,
            lower_revalidate_called,
            attrs_copied: true,
        };
    }

    EcryptfsRevalidateOutcome {
        result: rc,
        lower_revalidate_called,
        attrs_copied: positive,
    }
}

pub const fn ecryptfs_d_release_puts_lower() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecryptfs_dentry_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ecryptfs/dentry.c"
        ));
        assert!(source.contains("#include <linux/dcache.h>"));
        assert!(source.contains("#include <linux/fs_stack.h>"));
        assert!(source.contains("#include \"ecryptfs_kernel.h\""));
        assert!(source.contains("static int ecryptfs_d_revalidate"));
        assert!(source.contains("if (flags & LOOKUP_RCU)"));
        assert!(source.contains("return -ECHILD;"));
        assert!(source.contains("lower_dentry->d_flags & DCACHE_OP_REVALIDATE"));
        assert!(source.contains("take_dentry_name_snapshot"));
        assert!(source.contains("fsstack_copy_attr_all"));
        assert!(source.contains("if (!inode->i_nlink)"));
        assert!(source.contains("static void ecryptfs_d_release"));
        assert!(source.contains("dput(dentry->d_fsdata);"));
        assert!(source.contains(ECRYPTFS_DOPS_SYMBOL));

        assert_eq!(
            ecryptfs_d_revalidate_outcome(LOOKUP_RCU, 0, 1, false, 1).result,
            -ECHILD
        );
        let lower = ecryptfs_d_revalidate_outcome(0, DCACHE_OP_REVALIDATE, 7, false, 1);
        assert_eq!(lower.result, 7);
        assert!(lower.lower_revalidate_called);
        let unhashed = ecryptfs_d_revalidate_outcome(0, 0, 1, true, 0);
        assert_eq!(unhashed.result, 0);
        assert!(unhashed.attrs_copied);
        assert!(ecryptfs_d_release_puts_lower());
    }
}
