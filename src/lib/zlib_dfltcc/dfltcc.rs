//! linux-parity: complete
//! linux-source: vendor/linux/lib/zlib_dfltcc/dfltcc.c
//! test-origin: linux:vendor/linux/lib/zlib_dfltcc/dfltcc.c
//! DFLTCC operation-ending supplemental code handling.

pub const DFLTCC_QAF: u8 = 0;
pub const DFLTCC_RIBM: u32 = 0;
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DfltccState {
    pub enabled: bool,
    pub af_initialized: bool,
    pub nt: u8,
    pub ribm: u32,
}

pub fn oesc_msg(oesc: i32, static_build: bool) -> Option<&'static str> {
    if oesc == 0 || static_build {
        None
    } else {
        Some("Operation-Ending-Supplemental Code is")
    }
}

pub const fn dfltcc_reset_state(enabled: bool) -> DfltccState {
    DfltccState {
        enabled,
        af_initialized: enabled,
        nt: 1,
        ribm: DFLTCC_RIBM,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dfltcc_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/zlib_dfltcc/dfltcc.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/zlib_dfltcc/dfltcc.h"
        ));
        assert!(source.contains("#include \"dfltcc_util.h\""));
        assert!(source.contains("#include \"dfltcc.h\""));
        assert!(source.contains("char *oesc_msg"));
        assert!(source.contains("if (oesc == 0x00)"));
        assert!(source.contains("Operation-Ending-Supplemental Code is 0x%.2X"));
        assert!(source.contains("void dfltcc_reset_state"));
        assert!(source.contains("dfltcc(DFLTCC_QAF"));
        assert!(source.contains("dfltcc_state->param.nt = 1;"));
        assert!(source.contains("dfltcc_state->param.ribm = DFLTCC_RIBM;"));
        assert!(header.contains("#define DFLTCC_RIBM 0"));
        assert!(source.contains("MODULE_LICENSE(\"GPL\")"));
        assert_eq!(oesc_msg(0, false), None);
        assert!(oesc_msg(0x22, false).is_some());
        assert_eq!(oesc_msg(0x22, true), None);
        assert_eq!(dfltcc_reset_state(true).nt, 1);
        assert_eq!(MODULE_LICENSE, "GPL");
    }
}
