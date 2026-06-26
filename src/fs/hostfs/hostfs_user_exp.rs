//! linux-parity: complete
//! linux-source: vendor/linux/fs/hostfs/hostfs_user_exp.c
//! test-origin: linux:vendor/linux/fs/hostfs/hostfs_user_exp.c
//! GPL exports for hostfs user helpers.

pub const GPL_EXPORTS: &[&str] = &[
    "stat_file",
    "access_file",
    "open_file",
    "open_dir",
    "seek_dir",
    "read_dir",
    "read_file",
    "write_file",
    "lseek_file",
    "fsync_file",
    "replace_file",
    "close_file",
    "close_dir",
    "file_create",
    "set_attr",
    "make_symlink",
    "unlink_file",
    "do_mkdir",
    "hostfs_do_rmdir",
    "do_mknod",
    "link_file",
    "hostfs_do_readlink",
    "rename_file",
    "rename2_file",
    "do_statfs",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostfs_user_exports_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/hostfs/hostfs_user_exp.c"
        ));
        assert!(source.contains("#include <linux/module.h>"));
        assert!(source.contains("#include \"hostfs.h\""));
        for symbol in GPL_EXPORTS {
            assert!(source.contains(&alloc::format!("EXPORT_SYMBOL_GPL({symbol});")));
        }
    }
}
