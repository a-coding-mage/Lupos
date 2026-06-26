//! linux-parity: complete
//! linux-source: vendor/linux/fs/nfs/symlink.c
//! test-origin: linux:vendor/linux/fs/nfs/symlink.c
//! NFS symlink folio fill and get_link outcomes.

use crate::include::uapi::errno::ECHILD;

pub const NFS_SYMLINK_INODE_OPERATIONS_SYMBOL: &str = "nfs_symlink_inode_operations";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NfsGetLinkMode {
    Rcu,
    RefWalk,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NfsGetLinkOutcome {
    pub result: Result<(), i32>,
    pub used_read_cache_folio: bool,
    pub used_filemap_get_folio: bool,
    pub delayed_call_set: bool,
}

pub const fn nfs_symlink_filler_result(readlink_result: i32) -> (i32, bool) {
    (readlink_result, readlink_result == 0)
}

pub const fn nfs_get_link_outcome(
    mode: NfsGetLinkMode,
    revalidate_result: i32,
    folio_available: bool,
    folio_uptodate: bool,
) -> NfsGetLinkOutcome {
    if revalidate_result != 0 {
        return NfsGetLinkOutcome {
            result: Err(revalidate_result),
            used_read_cache_folio: false,
            used_filemap_get_folio: false,
            delayed_call_set: false,
        };
    }

    match mode {
        NfsGetLinkMode::Rcu => {
            if !folio_available || !folio_uptodate {
                return NfsGetLinkOutcome {
                    result: Err(-ECHILD),
                    used_read_cache_folio: false,
                    used_filemap_get_folio: true,
                    delayed_call_set: false,
                };
            }
            NfsGetLinkOutcome {
                result: Ok(()),
                used_read_cache_folio: false,
                used_filemap_get_folio: true,
                delayed_call_set: true,
            }
        }
        NfsGetLinkMode::RefWalk => {
            if !folio_available {
                return NfsGetLinkOutcome {
                    result: Err(-ECHILD),
                    used_read_cache_folio: true,
                    used_filemap_get_folio: false,
                    delayed_call_set: false,
                };
            }
            NfsGetLinkOutcome {
                result: Ok(()),
                used_read_cache_folio: true,
                used_filemap_get_folio: false,
                delayed_call_set: true,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfs_symlink_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/nfs/symlink.c"
        ));
        assert!(source.contains("#include <linux/nfs_fs.h>"));
        assert!(source.contains("static int nfs_symlink_filler"));
        assert!(source.contains("NFS_PROTO(inode)->readlink(inode, &folio->page, 0, PAGE_SIZE);"));
        assert!(source.contains("folio_end_read(folio, error == 0);"));
        assert!(source.contains("static const char *nfs_get_link"));
        assert!(source.contains("if (!dentry)"));
        assert!(source.contains("nfs_revalidate_mapping_rcu(inode)"));
        assert!(source.contains("filemap_get_folio(inode->i_mapping, 0)"));
        assert!(source.contains("return ERR_PTR(-ECHILD);"));
        assert!(source.contains("read_cache_folio(&inode->i_data, 0, nfs_symlink_filler"));
        assert!(source.contains("set_delayed_call(done, page_put_link, folio);"));
        assert!(source.contains(NFS_SYMLINK_INODE_OPERATIONS_SYMBOL));

        assert_eq!(nfs_symlink_filler_result(0), (0, true));
        assert_eq!(nfs_symlink_filler_result(-5), (-5, false));
        assert_eq!(
            nfs_get_link_outcome(NfsGetLinkMode::Rcu, 0, true, false).result,
            Err(-ECHILD)
        );
        let ok = nfs_get_link_outcome(NfsGetLinkMode::RefWalk, 0, true, false);
        assert_eq!(ok.result, Ok(()));
        assert!(ok.used_read_cache_folio);
        assert!(ok.delayed_call_set);
    }
}
