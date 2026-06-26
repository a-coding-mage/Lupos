//! linux-parity: complete
//! linux-source: vendor/linux/fs/fuse/trace.c
//! test-origin: linux:vendor/linux/fs/fuse/trace.c
//! FUSE tracepoint compile unit.

use crate::kernel::trace::TraceCompileUnit;

pub const SOURCE: TraceCompileUnit = TraceCompileUnit {
    linux_source: "vendor/linux/fs/fuse/trace.c",
    headers: &[
        "#include \"dev_uring_i.h\"",
        "#include \"fuse_i.h\"",
        "#include \"fuse_dev_i.h\"",
        "#include <linux/pagemap.h>",
        "#include \"fuse_trace.h\"",
    ],
    create_trace_points: true,
    checker_gated: false,
    exported_tracepoints: &[],
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
                "/vendor/linux/fs/fuse/trace.c"
            )),
            SOURCE,
        );
    }
}
