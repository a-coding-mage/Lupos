//! linux-parity: complete
//! linux-source: vendor/linux/lib/raid/xor/xor-8regs.c
//! test-origin: linux:vendor/linux/lib/raid/xor/xor-8regs.c
//! Eight-register generic RAID XOR block helper.

pub const XOR_8REGS_NAME: &str = "8regs";
pub const WORDS_PER_LINE: usize = 8;

pub fn xor_8regs_2(bytes: usize, p1: &mut [usize], p2: &[usize]) {
    xor_words(bytes, p1, &[p2]);
}

pub fn xor_8regs_3(bytes: usize, p1: &mut [usize], p2: &[usize], p3: &[usize]) {
    xor_words(bytes, p1, &[p2, p3]);
}

pub fn xor_8regs_4(bytes: usize, p1: &mut [usize], p2: &[usize], p3: &[usize], p4: &[usize]) {
    xor_words(bytes, p1, &[p2, p3, p4]);
}

pub fn xor_8regs_5(
    bytes: usize,
    p1: &mut [usize],
    p2: &[usize],
    p3: &[usize],
    p4: &[usize],
    p5: &[usize],
) {
    xor_words(bytes, p1, &[p2, p3, p4, p5]);
}

pub const fn xor_8regs_words(bytes: usize) -> usize {
    bytes / core::mem::size_of::<usize>() / WORDS_PER_LINE * WORDS_PER_LINE
}

fn xor_words(bytes: usize, p1: &mut [usize], sources: &[&[usize]]) {
    let words = xor_8regs_words(bytes);
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
    fn xor_8regs_matches_linux_unrolled_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/raid/xor/xor-8regs.c"
        ));
        assert!(source.contains("#include \"xor_impl.h\""));
        assert!(source.contains("long lines = bytes / (sizeof (long)) / 8;"));
        assert!(source.contains("p1[0] ^= p2[0];"));
        assert!(source.contains("p1[7] ^= p2[7];"));
        assert!(source.contains("p1[0] ^= p2[0] ^ p3[0] ^ p4[0] ^ p5[0];"));
        assert!(source.contains("DO_XOR_BLOCKS(8regs"));
        assert!(source.contains(".name\t\t= \"8regs\""));

        let mut p1 = [0x10usize; 16];
        let p2 = [0x01usize; 16];
        let p3 = [0x02usize; 16];
        let p4 = [0x04usize; 16];
        let p5 = [0x08usize; 16];
        xor_8regs_5(core::mem::size_of_val(&p1), &mut p1, &p2, &p3, &p4, &p5);
        assert_eq!(p1, [0x1fusize; 16]);

        let mut partial = [0x80usize; 9];
        xor_8regs_2(core::mem::size_of::<usize>() * 8, &mut partial, &p2);
        assert_eq!(&partial[..8], &[0x81usize; 8]);
        assert_eq!(partial[8], 0x80);
        assert_eq!(XOR_8REGS_NAME, "8regs");
    }
}
