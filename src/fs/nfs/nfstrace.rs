//! linux-parity: complete
//! linux-source: vendor/linux/fs/nfs/nfstrace.c
//! test-origin: linux:vendor/linux/fs/nfs/nfstrace.c
//! NFS client tracepoint compile unit.

use crate::kernel::trace::TraceCompileUnit;

pub const SOURCE: TraceCompileUnit = TraceCompileUnit {
    linux_source: "vendor/linux/fs/nfs/nfstrace.c",
    headers: &[
        "#include <linux/nfs_fs.h>",
        "#include <linux/namei.h>",
        "#include \"internal.h\"",
        "#include \"nfstrace.h\"",
    ],
    create_trace_points: true,
    checker_gated: false,
    exported_tracepoints: &[
        "EXPORT_TRACEPOINT_SYMBOL_GPL(nfs_fsync_enter);",
        "EXPORT_TRACEPOINT_SYMBOL_GPL(nfs_fsync_exit);",
        "EXPORT_TRACEPOINT_SYMBOL_GPL(nfs_xdr_status);",
        "EXPORT_TRACEPOINT_SYMBOL_GPL(nfs_xdr_bad_filehandle);",
    ],
    module_description: None,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_compile_unit_matches_linux_source() {
        crate::kernel::trace::assert_trace_compile_unit(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/vendor/linux/fs/nfs/nfstrace.c"
            )),
            SOURCE,
        );
    }
}
