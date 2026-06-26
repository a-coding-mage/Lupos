//! linux-parity: complete
//! linux-source: vendor/linux/fs/nfs/nfs4trace.c
//! test-origin: linux:vendor/linux/fs/nfs/nfs4trace.c
//! NFSv4 tracepoint compile unit.

use crate::kernel::trace::TraceCompileUnit;

pub const SOURCE: TraceCompileUnit = TraceCompileUnit {
    linux_source: "vendor/linux/fs/nfs/nfs4trace.c",
    headers: &[
        "#include <uapi/linux/pr.h>",
        "#include <linux/blkdev.h>",
        "#include <linux/nfs_fs.h>",
        "#include \"nfs4_fs.h\"",
        "#include \"internal.h\"",
        "#include \"nfs4session.h\"",
        "#include \"callback.h\"",
        "#include \"pnfs.h\"",
        "#include \"nfs4trace.h\"",
    ],
    create_trace_points: true,
    checker_gated: false,
    exported_tracepoints: &[
        "EXPORT_TRACEPOINT_SYMBOL_GPL(nfs4_pnfs_read);",
        "EXPORT_TRACEPOINT_SYMBOL_GPL(nfs4_pnfs_write);",
        "EXPORT_TRACEPOINT_SYMBOL_GPL(nfs4_pnfs_commit_ds);",
        "EXPORT_TRACEPOINT_SYMBOL_GPL(pnfs_ds_connect);",
        "EXPORT_TRACEPOINT_SYMBOL_GPL(ff_layout_read_error);",
        "EXPORT_TRACEPOINT_SYMBOL_GPL(bl_pr_key_reg);",
        "EXPORT_TRACEPOINT_SYMBOL_GPL(fl_getdevinfo);",
    ],
    module_description: None,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfs4trace_compile_unit_matches_linux_source() {
        crate::kernel::trace::assert_trace_compile_unit(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/vendor/linux/fs/nfs/nfs4trace.c"
            )),
            SOURCE,
        );
    }
}
