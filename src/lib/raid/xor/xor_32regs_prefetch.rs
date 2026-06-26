//! linux-parity: complete
//! linux-source: vendor/linux/lib/raid/xor/xor-32regs-prefetch.c
//! test-origin: linux:vendor/linux/lib/raid/xor/xor-32regs-prefetch.c
//! Thirty-two-register generic RAID XOR block helper with Linux prefetch points modeled as no-ops.

pub const XOR_32REGS_PREFETCH_NAME: &str = "32regs_prefetch";
pub const WORDS_PER_LINE: usize = 8;

pub fn xor_32regs_p_2(bytes: usize, p1: &mut [usize], p2: &[usize]) {
    xor_words(bytes, p1, &[p2]);
}

pub fn xor_32regs_p_3(bytes: usize, p1: &mut [usize], p2: &[usize], p3: &[usize]) {
    xor_words(bytes, p1, &[p2, p3]);
}

pub fn xor_32regs_p_4(bytes: usize, p1: &mut [usize], p2: &[usize], p3: &[usize], p4: &[usize]) {
    xor_words(bytes, p1, &[p2, p3, p4]);
}

pub fn xor_32regs_p_5(
    bytes: usize,
    p1: &mut [usize],
    p2: &[usize],
    p3: &[usize],
    p4: &[usize],
    p5: &[usize],
) {
    xor_words(bytes, p1, &[p2, p3, p4, p5]);
}

pub fn xor_gen_32regs_p(bytes: usize, dest: &mut [usize], srcs: &[&[usize]]) {
    let mut src_off = 0;
    let mut src_cnt = srcs.len();

    while src_cnt > 0 {
        let this_cnt = if src_cnt < 4 { src_cnt } else { 4 };
        match this_cnt {
            1 => xor_32regs_p_2(bytes, dest, srcs[src_off]),
            2 => xor_32regs_p_3(bytes, dest, srcs[src_off], srcs[src_off + 1]),
            3 => xor_32regs_p_4(
                bytes,
                dest,
                srcs[src_off],
                srcs[src_off + 1],
                srcs[src_off + 2],
            ),
            _ => xor_32regs_p_5(
                bytes,
                dest,
                srcs[src_off],
                srcs[src_off + 1],
                srcs[src_off + 2],
                srcs[src_off + 3],
            ),
        }
        src_cnt -= this_cnt;
        src_off += this_cnt;
    }
}

pub const fn xor_32regs_p_words(bytes: usize) -> usize {
    bytes / core::mem::size_of::<usize>() / WORDS_PER_LINE * WORDS_PER_LINE
}

fn xor_words(bytes: usize, p1: &mut [usize], sources: &[&[usize]]) {
    let words = xor_32regs_p_words(bytes);
    assert!(p1.len() >= words);
    for source in sources {
        assert!(source.len() >= words);
    }

    for line in 0..(words / WORDS_PER_LINE) {
        let start = line * WORDS_PER_LINE;
        for offset in 0..WORDS_PER_LINE {
            let index = start + offset;
            let mut value = p1[index];
            for source in sources {
                value ^= source[index];
            }
            p1[index] = value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xor_32regs_prefetch_matches_linux_register_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/raid/xor/xor-32regs-prefetch.c"
        ));
        assert!(source.contains("#include <linux/prefetch.h>"));
        assert!(source.contains("long lines = bytes / (sizeof (long)) / 8 - 1;"));
        assert!(source.contains("register long d0, d1, d2, d3, d4, d5, d6, d7;"));
        assert!(source.contains("prefetchw(p1+8);"));
        assert!(source.contains("once_more:"));
        assert!(source.contains("goto once_more;"));
        assert!(source.contains("DO_XOR_BLOCKS(32regs_p"));
        assert!(source.contains(".name\t\t= \"32regs_prefetch\""));

        let mut p1 = [0x10usize; 16];
        let p2 = [0x01usize; 16];
        let p3 = [0x02usize; 16];
        let p4 = [0x04usize; 16];
        let p5 = [0x08usize; 16];
        xor_32regs_p_5(core::mem::size_of_val(&p1), &mut p1, &p2, &p3, &p4, &p5);
        assert_eq!(p1, [0x1fusize; 16]);

        let mut generated = [0x20usize; 16];
        xor_gen_32regs_p(
            core::mem::size_of_val(&generated),
            &mut generated,
            &[&p2, &p3, &p4, &p5, &p2],
        );
        assert_eq!(generated, [0x2eusize; 16]);
        assert_eq!(XOR_32REGS_PREFETCH_NAME, "32regs_prefetch");
    }
}
