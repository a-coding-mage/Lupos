//! linux-parity: complete
//! linux-source: vendor/linux/fs/ntfs/upcase.c
//! test-origin: linux:vendor/linux/fs/ntfs/upcase.c
//! NTFS default upcase table generation rules.

pub const DEFAULT_UPCASE_LEN: usize = 0x10000;

const UC_RUN_TABLE: &[(u16, u16, i16)] = &[
    (0x0061, 0x007b, -32),
    (0x0451, 0x045d, -80),
    (0x1f70, 0x1f72, 74),
    (0x00e0, 0x00f7, -32),
    (0x045e, 0x0460, -80),
    (0x1f72, 0x1f76, 86),
    (0x00f8, 0x00ff, -32),
    (0x0561, 0x0587, -48),
    (0x1f76, 0x1f78, 100),
    (0x0256, 0x0258, -205),
    (0x1f00, 0x1f08, 8),
    (0x1f78, 0x1f7a, 128),
    (0x028a, 0x028c, -217),
    (0x1f10, 0x1f16, 8),
    (0x1f7a, 0x1f7c, 112),
    (0x03ac, 0x03ad, -38),
    (0x1f20, 0x1f28, 8),
    (0x1f7c, 0x1f7e, 126),
    (0x03ad, 0x03b0, -37),
    (0x1f30, 0x1f38, 8),
    (0x1fb0, 0x1fb2, 8),
    (0x03b1, 0x03c2, -32),
    (0x1f40, 0x1f46, 8),
    (0x1fd0, 0x1fd2, 8),
    (0x03c2, 0x03c3, -31),
    (0x1f51, 0x1f52, 8),
    (0x1fe0, 0x1fe2, 8),
    (0x03c3, 0x03cc, -32),
    (0x1f53, 0x1f54, 8),
    (0x1fe5, 0x1fe6, 7),
    (0x03cc, 0x03cd, -64),
    (0x1f55, 0x1f56, 8),
    (0x2170, 0x2180, -16),
    (0x03cd, 0x03cf, -63),
    (0x1f57, 0x1f58, 8),
    (0x24d0, 0x24ea, -26),
    (0x0430, 0x0450, -32),
    (0x1f60, 0x1f68, 8),
    (0xff41, 0xff5b, -32),
];

const UC_DUP_TABLE: &[(u16, u16)] = &[
    (0x0100, 0x012f),
    (0x01a0, 0x01a6),
    (0x03e2, 0x03ef),
    (0x04cb, 0x04cc),
    (0x0132, 0x0137),
    (0x01b3, 0x01b7),
    (0x0460, 0x0481),
    (0x04d0, 0x04eb),
    (0x0139, 0x0149),
    (0x01cd, 0x01dd),
    (0x0490, 0x04bf),
    (0x04ee, 0x04f5),
    (0x014a, 0x0178),
    (0x01de, 0x01ef),
    (0x04bf, 0x04bf),
    (0x04f8, 0x04f9),
    (0x0179, 0x017e),
    (0x01f4, 0x01f5),
    (0x04c1, 0x04c4),
    (0x1e00, 0x1e95),
    (0x018b, 0x018b),
    (0x01fa, 0x0218),
    (0x04c7, 0x04c8),
    (0x1ea0, 0x1ef9),
];

const UC_WORD_TABLE: &[(u16, u16)] = &[
    (0x00ff, 0x0178),
    (0x01ad, 0x01ac),
    (0x01f3, 0x01f1),
    (0x0269, 0x0196),
    (0x0183, 0x0182),
    (0x01b0, 0x01af),
    (0x0253, 0x0181),
    (0x026f, 0x019c),
    (0x0185, 0x0184),
    (0x01b9, 0x01b8),
    (0x0254, 0x0186),
    (0x0272, 0x019d),
    (0x0188, 0x0187),
    (0x01bd, 0x01bc),
    (0x0259, 0x018f),
    (0x0275, 0x019f),
    (0x018c, 0x018b),
    (0x01c6, 0x01c4),
    (0x025b, 0x0190),
    (0x0283, 0x01a9),
    (0x0192, 0x0191),
    (0x01c9, 0x01c7),
    (0x0260, 0x0193),
    (0x0288, 0x01ae),
    (0x0199, 0x0198),
    (0x01cc, 0x01ca),
    (0x0263, 0x0194),
    (0x0292, 0x01b7),
    (0x01a8, 0x01a7),
    (0x01dd, 0x018e),
    (0x0268, 0x0197),
];

pub fn ntfs_default_upcase(codepoint: u16) -> u16 {
    for &(offset, value) in UC_WORD_TABLE {
        if codepoint == offset {
            return value;
        }
    }

    for &(start, end) in UC_DUP_TABLE {
        if codepoint >= start && codepoint < end && (codepoint - start) % 2 == 1 {
            return codepoint - 1;
        }
    }

    for &(start, end, add) in UC_RUN_TABLE {
        if codepoint >= start && codepoint < end {
            return ((codepoint as i32) + add as i32) as u16;
        }
    }

    codepoint
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ntfs_upcase_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ntfs/upcase.c"
        ));
        assert!(source.contains("#include \"ntfs.h\""));
        assert!(source.contains("__le16 *generate_default_upcase(void)"));
        assert!(source.contains("static const int uc_run_table[][3]"));
        assert!(source.contains("static const int uc_dup_table[][2]"));
        assert!(source.contains("static const int uc_word_table[][2]"));
        assert!(source.contains("kvcalloc(default_upcase_len, sizeof(__le16), GFP_NOFS)"));
        assert!(source.contains("uc[i] = cpu_to_le16(i);"));
        assert!(source.contains("le16_add_cpu(&uc[i], uc_run_table[r][2]);"));
        assert!(source.contains("le16_add_cpu(&uc[i + 1], -1);"));
        assert!(source.contains("uc[uc_word_table[r][0]] = cpu_to_le16(uc_word_table[r][1]);"));

        assert_eq!(DEFAULT_UPCASE_LEN, 0x10000);
        assert_eq!(ntfs_default_upcase(b'a' as u16), b'A' as u16);
        assert_eq!(ntfs_default_upcase(0x0101), 0x0100);
        assert_eq!(ntfs_default_upcase(0x00ff), 0x0178);
        assert_eq!(ntfs_default_upcase(0x2172), 0x2162);
        assert_eq!(ntfs_default_upcase(0x1234), 0x1234);
    }
}
