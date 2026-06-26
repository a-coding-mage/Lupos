//! linux-parity: complete
//! linux-source: vendor/linux/lib/raid/xor/loongarch/xor_template.c
//! test-origin: linux:vendor/linux/lib/raid/xor/loongarch/xor_template.c
//! LoongArch XOR SIMD template flow.

pub const EXPECTED_DEFINES: &[&str] = &[
    "LINE_WIDTH",
    "XOR_FUNC_NAME(nr)",
    "LD_INOUT_LINE(buf)",
    "LD_AND_XOR_LINE(buf)",
    "ST_LINE(buf)",
];

pub fn xor_template_2(line_width: usize, bytes: usize, v1: &mut [usize], v2: &[usize]) {
    xor_template_words(line_width, bytes, v1, &[v2]);
}

pub fn xor_template_3(
    line_width: usize,
    bytes: usize,
    v1: &mut [usize],
    v2: &[usize],
    v3: &[usize],
) {
    xor_template_words(line_width, bytes, v1, &[v2, v3]);
}

pub fn xor_template_4(
    line_width: usize,
    bytes: usize,
    v1: &mut [usize],
    v2: &[usize],
    v3: &[usize],
    v4: &[usize],
) {
    xor_template_words(line_width, bytes, v1, &[v2, v3, v4]);
}

pub fn xor_template_5(
    line_width: usize,
    bytes: usize,
    v1: &mut [usize],
    v2: &[usize],
    v3: &[usize],
    v4: &[usize],
    v5: &[usize],
) {
    xor_template_words(line_width, bytes, v1, &[v2, v3, v4, v5]);
}

pub const fn words_per_line(line_width: usize) -> usize {
    line_width / core::mem::size_of::<usize>()
}

fn xor_template_words(line_width: usize, bytes: usize, v1: &mut [usize], sources: &[&[usize]]) {
    let words_per_line = words_per_line(line_width);
    let lines = bytes / line_width;
    let words = lines * words_per_line;
    assert!(words_per_line > 0);
    assert!(v1.len() >= words);
    for source in sources {
        assert!(source.len() >= words);
    }

    for line in 0..lines {
        let start = line * words_per_line;
        for offset in 0..words_per_line {
            let index = start + offset;
            let mut value = v1[index];
            for source in sources {
                value ^= source[index];
            }
            v1[index] = value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loongarch_xor_template_matches_linux_macro_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/raid/xor/loongarch/xor_template.c"
        ));
        assert!(source.contains("Template for XOR operations, instantiated in xor_simd.c."));
        for define in EXPECTED_DEFINES {
            assert!(source.contains(define));
        }
        assert!(source.contains("void XOR_FUNC_NAME(2)(unsigned long bytes"));
        assert!(source.contains("unsigned long lines = bytes / LINE_WIDTH;"));
        assert!(source.matches("LD_INOUT_LINE(v1)").count() >= 4);
        assert!(source.contains("LD_AND_XOR_LINE(v5)"));
        assert!(source.contains("ST_LINE(v1)"));
        assert!(source.contains("v1 += LINE_WIDTH / sizeof(unsigned long);"));
        assert!(source.contains("} while (--lines > 0);"));

        let mut v1 = [0x40usize; 16];
        let v2 = [0x01usize; 16];
        let v3 = [0x02usize; 16];
        xor_template_3(64, 128, &mut v1, &v2, &v3);
        assert_eq!(v1, [0x43usize; 16]);
        assert_eq!(words_per_line(64), 64 / core::mem::size_of::<usize>());
    }
}
