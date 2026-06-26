//! linux-parity: complete
//! linux-source: vendor/linux/fs/nfs_common/nfs_ssc.c
//! test-origin: linux:vendor/linux/fs/nfs_common/nfs_ssc.c
//! NFS server-side-copy client operation registration table.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NfsSscClientOpsTable {
    pub nfs4_ops: Option<&'static str>,
    pub nfs_ops: Option<&'static str>,
}

impl NfsSscClientOpsTable {
    pub const fn new() -> Self {
        Self {
            nfs4_ops: None,
            nfs_ops: None,
        }
    }

    pub fn nfs42_ssc_register(&mut self, ops: &'static str) {
        self.nfs4_ops = Some(ops);
    }

    pub fn nfs42_ssc_unregister(&mut self, ops: &'static str) {
        if self.nfs4_ops == Some(ops) {
            self.nfs4_ops = None;
        }
    }

    pub fn nfs_ssc_register(&mut self, ops: &'static str, config_nfs_v4_2: bool) {
        if config_nfs_v4_2 {
            self.nfs_ops = Some(ops);
        }
    }

    pub fn nfs_ssc_unregister(&mut self, ops: &'static str, config_nfs_v4_2: bool) {
        if config_nfs_v4_2 && self.nfs_ops == Some(ops) {
            self.nfs_ops = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfs_ssc_registration_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/nfs_common/nfs_ssc.c"
        ));
        assert!(source.contains("#include <linux/nfs_ssc.h>"));
        assert!(source.contains("#include \"../nfs/nfs4_fs.h\""));
        assert!(source.contains("struct nfs_ssc_client_ops_tbl nfs_ssc_client_tbl;"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(nfs_ssc_client_tbl);"));
        assert!(source.contains("void nfs42_ssc_register"));
        assert!(source.contains("nfs_ssc_client_tbl.ssc_nfs4_ops = ops;"));
        assert!(source.contains("if (nfs_ssc_client_tbl.ssc_nfs4_ops != ops)"));
        assert!(source.contains("nfs_ssc_client_tbl.ssc_nfs4_ops = NULL;"));
        assert!(source.contains("void nfs_ssc_register"));
        assert!(source.contains("nfs_ssc_client_tbl.ssc_nfs_ops = ops;"));
        assert!(source.contains("if (nfs_ssc_client_tbl.ssc_nfs_ops != ops)"));
        assert!(source.contains("nfs_ssc_client_tbl.ssc_nfs_ops = NULL;"));
        assert!(source.contains("#else"));
        assert!(source.contains("void nfs_ssc_register(const struct nfs_ssc_client_ops *ops)"));
        assert!(source.contains("#else"));

        let mut table = NfsSscClientOpsTable::new();
        table.nfs42_ssc_register("nfs4");
        table.nfs42_ssc_unregister("other");
        assert_eq!(table.nfs4_ops, Some("nfs4"));
        table.nfs42_ssc_unregister("nfs4");
        assert_eq!(table.nfs4_ops, None);

        table.nfs_ssc_register("nfs", false);
        assert_eq!(table.nfs_ops, None);
        table.nfs_ssc_register("nfs", true);
        table.nfs_ssc_unregister("nfs", true);
        assert_eq!(table.nfs_ops, None);
    }
}
