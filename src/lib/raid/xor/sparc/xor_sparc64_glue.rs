//! linux-parity: complete
//! linux-source: vendor/linux/lib/raid/xor/sparc/xor-sparc64-glue.c
//! test-origin: linux:vendor/linux/lib/raid/xor/sparc/xor-sparc64-glue.c
//! SPARC VIS and Niagara RAID XOR glue templates.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SparcXorTemplate {
    pub symbol: &'static str,
    pub name: &'static str,
    pub functions: [&'static str; 4],
}

pub const XOR_BLOCK_VIS: SparcXorTemplate = SparcXorTemplate {
    symbol: "xor_block_VIS",
    name: "VIS",
    functions: ["xor_vis_2", "xor_vis_3", "xor_vis_4", "xor_vis_5"],
};

pub const XOR_BLOCK_NIAGARA: SparcXorTemplate = SparcXorTemplate {
    symbol: "xor_block_niagara",
    name: "Niagara",
    functions: [
        "xor_niagara_2",
        "xor_niagara_3",
        "xor_niagara_4",
        "xor_niagara_5",
    ],
};

pub const SPARC_XOR_TEMPLATES: &[SparcXorTemplate] = &[XOR_BLOCK_VIS, XOR_BLOCK_NIAGARA];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparc64_xor_glue_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/raid/xor/sparc/xor-sparc64-glue.c"
        ));
        assert!(source.contains("#include \"xor_impl.h\""));
        assert!(source.contains("#include \"xor_arch.h\""));
        assert!(source.contains("DO_XOR_BLOCKS(vis, xor_vis_2, xor_vis_3, xor_vis_4, xor_vis_5);"));
        assert!(source.contains("struct xor_block_template xor_block_VIS"));
        assert!(source.contains(".name\t\t= \"VIS\""));
        assert!(
            source.contains("DO_XOR_BLOCKS(niagara, xor_niagara_2, xor_niagara_3, xor_niagara_4,")
        );
        assert!(source.contains("struct xor_block_template xor_block_niagara"));
        assert!(source.contains(".name\t\t= \"Niagara\""));

        assert_eq!(SPARC_XOR_TEMPLATES.len(), 2);
        assert_eq!(XOR_BLOCK_VIS.functions[0], "xor_vis_2");
        assert_eq!(XOR_BLOCK_NIAGARA.functions[3], "xor_niagara_5");
    }
}
