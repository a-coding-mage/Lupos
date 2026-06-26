//! linux-parity: complete
//! linux-source: vendor/linux/fs/smb/client/trace.c
//! test-origin: linux:vendor/linux/fs/smb/client/trace.c
//! SMB client tracepoint compile unit.

use crate::kernel::trace::TraceCompileUnit;

pub const SOURCE: TraceCompileUnit = TraceCompileUnit {
    linux_source: "vendor/linux/fs/smb/client/trace.c",
    headers: &[
        "#include \"cifsglob.h\"",
        "#include \"cifs_spnego.h\"",
        "#include \"trace.h\"",
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
                "/vendor/linux/fs/smb/client/trace.c"
            )),
            SOURCE,
        );
    }
}
