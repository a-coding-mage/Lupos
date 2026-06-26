//! linux-parity: complete
//! linux-source: vendor/linux/fs/autofs/symlink.c
//! test-origin: linux:vendor/linux/fs/autofs/symlink.c
//! autofs symlink get_link behavior.

use crate::include::uapi::errno::ECHILD;

pub const INODE_OPERATIONS_SYMBOL: &str = "autofs_symlink_inode_operations";
pub const GET_LINK_SYMBOL: &str = "autofs_get_link";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AutofsSbInfo {
    pub oz_mode: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AutofsInfo {
    pub last_used: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AutofsDentry<'a> {
    pub sbi: AutofsSbInfo,
    pub ino: Option<AutofsInfo>,
    pub inode_private: &'a str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AutofsGetLinkResult<'a> {
    pub link: &'a str,
    pub ino: Option<AutofsInfo>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AutofsInodeOperations {
    pub get_link: &'static str,
}

pub const AUTOFS_SYMLINK_INODE_OPERATIONS: AutofsInodeOperations = AutofsInodeOperations {
    get_link: GET_LINK_SYMBOL,
};

pub const fn autofs_get_link_null_dentry_error() -> i32 {
    -ECHILD
}

pub const fn autofs_get_link_updates_last_used(has_info: bool, oz_mode: bool) -> bool {
    has_info && !oz_mode
}

pub const fn autofs_get_link<'a>(
    dentry: Option<AutofsDentry<'a>>,
    jiffies: u64,
) -> Result<AutofsGetLinkResult<'a>, i32> {
    let Some(dentry) = dentry else {
        return Err(-ECHILD);
    };

    let ino = match dentry.ino {
        Some(mut ino) if !dentry.sbi.oz_mode => {
            ino.last_used = jiffies;
            Some(ino)
        }
        other => other,
    };

    Ok(AutofsGetLinkResult {
        link: dentry.inode_private,
        ino,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autofs_symlink_get_link_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/autofs/symlink.c"
        ));
        assert!(source.contains("#include \"autofs_i.h\""));
        assert!(source.contains("static const char *autofs_get_link(struct dentry *dentry"));
        assert!(source.contains("struct autofs_sb_info *sbi;"));
        assert!(source.contains("struct autofs_info *ino;"));
        assert!(source.contains("return ERR_PTR(-ECHILD);"));
        assert!(source.contains("sbi = autofs_sbi(dentry->d_sb);"));
        assert!(source.contains("ino = autofs_dentry_ino(dentry);"));
        assert!(source.contains("ino && !autofs_oz_mode(sbi)"));
        assert!(source.contains("ino->last_used = jiffies;"));
        assert!(source.contains("return d_inode(dentry)->i_private;"));
        assert!(source.contains(INODE_OPERATIONS_SYMBOL));
        assert!(source.contains(".get_link\t= autofs_get_link"));
        assert_eq!(AUTOFS_SYMLINK_INODE_OPERATIONS.get_link, GET_LINK_SYMBOL);
        assert_eq!(autofs_get_link_null_dentry_error(), -ECHILD);
        assert!(autofs_get_link_updates_last_used(true, false));
        assert!(!autofs_get_link_updates_last_used(true, true));
        assert!(!autofs_get_link_updates_last_used(false, false));

        assert_eq!(autofs_get_link(None, 99), Err(-ECHILD));
        let dentry = AutofsDentry {
            sbi: AutofsSbInfo { oz_mode: false },
            ino: Some(AutofsInfo { last_used: 1 }),
            inode_private: "/target",
        };
        let result = autofs_get_link(Some(dentry), 99).unwrap();
        assert_eq!(result.link, "/target");
        assert_eq!(result.ino.unwrap().last_used, 99);

        let oz_dentry = AutofsDentry {
            sbi: AutofsSbInfo { oz_mode: true },
            ino: Some(AutofsInfo { last_used: 1 }),
            inode_private: "/target",
        };
        let result = autofs_get_link(Some(oz_dentry), 99).unwrap();
        assert_eq!(result.ino.unwrap().last_used, 1);
    }
}
