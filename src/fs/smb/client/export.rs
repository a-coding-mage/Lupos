//! linux-parity: complete
//! linux-source: vendor/linux/fs/smb/client/export.c
//! test-origin: linux:vendor/linux/fs/smb/client/export.c
//! CIFS exportfs operation table under CONFIG_CIFS_NFSD_EXPORT.

use crate::include::uapi::errno::EACCES;

pub const CIFS_NFSD_EXPORT_CONFIG: &str = "CONFIG_CIFS_NFSD_EXPORT";
pub const CIFS_EXPORT_OPS_SYMBOL: &str = "cifs_export_ops";
pub const GENERIC_ENCODE_INO32_FH: &str = "generic_encode_ino32_fh";
pub const CIFS_GET_PARENT: &str = "cifs_get_parent";
pub const CIFS_DEBUG_LEVEL: &str = "FYI";
pub const CIFS_GET_PARENT_FORMAT: &str = "get parent for %p\n";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExportOperations {
    pub encode_fh: &'static str,
    pub get_parent: &'static str,
    pub fh_to_dentry: Option<&'static str>,
}

pub const CIFS_EXPORT_OPS: ExportOperations = ExportOperations {
    encode_fh: GENERIC_ENCODE_INO32_FH,
    get_parent: CIFS_GET_PARENT,
    fh_to_dentry: None,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CifsDentry {
    pub address: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CifsGetParentResult {
    pub debug_level: &'static str,
    pub debug_format: &'static str,
    pub dentry_address: usize,
    pub errno: i32,
}

pub const fn cifs_get_parent_result() -> i32 {
    -EACCES
}

pub const fn cifs_get_parent(dentry: CifsDentry) -> CifsGetParentResult {
    CifsGetParentResult {
        debug_level: CIFS_DEBUG_LEVEL,
        debug_format: CIFS_GET_PARENT_FORMAT,
        dentry_address: dentry.address,
        errno: cifs_get_parent_result(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cifs_export_ops_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/smb/client/export.c"
        ));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include <linux/exportfs.h>"));
        assert!(source.contains("#include \"cifsglob.h\""));
        assert!(source.contains("#include \"cifs_debug.h\""));
        assert!(source.contains("#include \"cifsfs.h\""));
        assert!(source.contains("#ifdef CONFIG_CIFS_NFSD_EXPORT"));
        assert!(source.contains("static struct dentry *cifs_get_parent"));
        assert!(source.contains("cifs_dbg(FYI, \"get parent for %p\\n\", dentry);"));
        assert!(source.contains("return ERR_PTR(-EACCES);"));
        assert!(source.contains(CIFS_EXPORT_OPS_SYMBOL));
        assert!(source.contains(".encode_fh = generic_encode_ino32_fh"));
        assert!(source.contains(".get_parent = cifs_get_parent"));
        assert!(
            source.contains("Following export operations are mandatory for NFS export support:")
        );
        assert!(source.contains(".fh_to_dentry ="));

        assert_eq!(CIFS_NFSD_EXPORT_CONFIG, "CONFIG_CIFS_NFSD_EXPORT");
        assert_eq!(
            CIFS_EXPORT_OPS,
            ExportOperations {
                encode_fh: "generic_encode_ino32_fh",
                get_parent: "cifs_get_parent",
                fh_to_dentry: None,
            }
        );
        assert_eq!(cifs_get_parent_result(), -EACCES);
        assert_eq!(
            cifs_get_parent(CifsDentry { address: 0xc1f5 }),
            CifsGetParentResult {
                debug_level: "FYI",
                debug_format: "get parent for %p\n",
                dentry_address: 0xc1f5,
                errno: -EACCES,
            }
        );
    }
}
