//! linux-parity: complete
//! linux-source: vendor/linux/lib/clz_tab.c
//! test-origin: linux:vendor/linux/lib/clz_tab.c
//! Count-leading-zero helper lookup table.

pub const CLZ_TAB: [u8; 256] = build_clz_tab();

const fn build_clz_tab() -> [u8; 256] {
    let mut out = [0u8; 256];
    let mut i = 1usize;
    while i < 256 {
        out[i] = 8 - (i as u8).leading_zeros() as u8;
        i += 1;
    }
    out
}

pub const fn clz_tab_value(byte: u8) -> u8 {
    CLZ_TAB[byte as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clz_table_matches_linux_shape() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/clz_tab.c"
        ));
        assert!(source.contains("const unsigned char __clz_tab[]"));
        assert_eq!(CLZ_TAB.len(), 256);
        assert_eq!(clz_tab_value(0), 0);
        assert_eq!(clz_tab_value(1), 1);
        assert_eq!(clz_tab_value(2), 2);
        assert_eq!(clz_tab_value(3), 2);
        assert_eq!(clz_tab_value(4), 3);
        assert_eq!(clz_tab_value(127), 7);
        assert_eq!(clz_tab_value(128), 8);
        assert_eq!(clz_tab_value(255), 8);
    }
}
