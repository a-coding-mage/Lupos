//! linux-parity: complete
//! linux-source: vendor/linux/fs/nfs/nfs3super.c
//! test-origin: linux:vendor/linux/fs/nfs/nfs3super.c
//! NFSv3 subversion registration metadata.

pub const MODULE_DESCRIPTION: &str = "NFSv3 client support";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_OWNER: &str = "THIS_MODULE";
pub const NFS_FS_TYPE: &str = "nfs_fs_type";
pub const INIT_FUNCTION: &str = "init_nfs_v3";
pub const EXIT_FUNCTION: &str = "exit_nfs_v3";
pub const MODULE_INIT_HOOK: &str = "module_init(init_nfs_v3)";
pub const MODULE_EXIT_HOOK: &str = "module_exit(exit_nfs_v3)";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NfsSubversionContract {
    pub symbol: &'static str,
    pub owner: &'static str,
    pub nfs_fs: &'static str,
    pub rpc_version: &'static str,
    pub rpc_ops: &'static str,
    pub super_ops: &'static str,
}

pub const NFS_V3: NfsSubversionContract = NfsSubversionContract {
    symbol: "nfs_v3",
    owner: MODULE_OWNER,
    nfs_fs: NFS_FS_TYPE,
    rpc_version: "nfs_version3",
    rpc_ops: "nfs_v3_clientops",
    super_ops: "nfs_sops",
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NfsVersionRegistry {
    pub registered_symbol: Option<&'static str>,
}

pub fn register_nfs_version(
    registry: &mut NfsVersionRegistry,
    version: &'static NfsSubversionContract,
) {
    registry.registered_symbol = Some(version.symbol);
}

pub fn unregister_nfs_version(
    registry: &mut NfsVersionRegistry,
    version: &'static NfsSubversionContract,
) {
    if registry.registered_symbol == Some(version.symbol) {
        registry.registered_symbol = None;
    }
}

pub fn init_nfs_v3(registry: &mut NfsVersionRegistry) -> i32 {
    register_nfs_version(registry, &NFS_V3);
    0
}

pub fn exit_nfs_v3(registry: &mut NfsVersionRegistry) {
    unregister_nfs_version(registry, &NFS_V3);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfs3_super_registration_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/nfs/nfs3super.c"
        ));
        assert!(source.contains("#include <linux/module.h>"));
        assert!(source.contains("#include <linux/nfs_fs.h>"));
        assert!(source.contains("#include \"internal.h\""));
        assert!(source.contains("#include \"nfs3_fs.h\""));
        assert!(source.contains("#include \"nfs.h\""));
        assert!(source.contains("struct nfs_subversion nfs_v3 = {"));
        assert!(source.contains(".owner = THIS_MODULE"));
        assert!(source.contains(".nfs_fs   = &nfs_fs_type"));
        assert!(source.contains(".rpc_vers = &nfs_version3"));
        assert!(source.contains(".rpc_ops  = &nfs_v3_clientops"));
        assert!(source.contains(".sops     = &nfs_sops"));
        assert!(source.contains("static int __init init_nfs_v3(void)"));
        assert!(source.contains("register_nfs_version(&nfs_v3);"));
        assert!(source.contains("return 0;"));
        assert!(source.contains("static void __exit exit_nfs_v3(void)"));
        assert!(source.contains("unregister_nfs_version(&nfs_v3);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"NFSv3 client support\")"));
        assert!(source.contains("MODULE_LICENSE(\"GPL\")"));
        assert!(source.contains("module_init(init_nfs_v3);"));
        assert!(source.contains("module_exit(exit_nfs_v3);"));
        assert_eq!(
            NFS_V3,
            NfsSubversionContract {
                symbol: "nfs_v3",
                owner: "THIS_MODULE",
                nfs_fs: "nfs_fs_type",
                rpc_version: "nfs_version3",
                rpc_ops: "nfs_v3_clientops",
                super_ops: "nfs_sops",
            }
        );
        assert_eq!(NFS_V3.rpc_version, "nfs_version3");
        assert_eq!(MODULE_DESCRIPTION, "NFSv3 client support");
        let mut registry = NfsVersionRegistry::default();
        assert_eq!(init_nfs_v3(&mut registry), 0);
        assert_eq!(registry.registered_symbol, Some("nfs_v3"));
        exit_nfs_v3(&mut registry);
        assert_eq!(registry.registered_symbol, None);
    }
}
