//! linux-parity: complete
//! linux-source: vendor/linux/lib/test_memcat_p.c
//! test-origin: linux:vendor/linux/lib/test_memcat_p.c
//! Source-backed self-test model for memcat_p().

use super::memcat_p::memcat_p_values;

pub const MAGIC: u32 = 0xf00f_f00f;
pub const INPUT_MAX: usize = 128;
pub const EXPECT: usize = INPUT_MAX * 2 - 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TestStruct {
    pub num: i32,
    pub magic: u32,
}

pub fn build_inputs() -> (
    [Option<TestStruct>; INPUT_MAX],
    [Option<TestStruct>; INPUT_MAX],
) {
    let mut in0 = [None; INPUT_MAX];
    let mut in1 = [None; INPUT_MAX];
    let mut r = 1i32;
    for index in 0..(INPUT_MAX - 1) {
        r = ((r as i64 * 725_861) % 6_599) as i32;
        in0[index] = Some(TestStruct {
            num: r,
            magic: MAGIC,
        });
        in1[index] = Some(TestStruct {
            num: -r,
            magic: MAGIC,
        });
    }
    (in0, in1)
}

pub fn test_memcat_p_passes() -> bool {
    let (in0, in1) = build_inputs();
    let out = memcat_p_values(&in0, &in1);
    if out.len() != EXPECT + 1 || out[EXPECT].is_some() {
        return false;
    }

    let mut total = 0i32;
    let mut count = 0usize;
    for item in out.iter().flatten() {
        total += item.num;
        count += 1;
        if item.magic != MAGIC {
            return false;
        }
    }
    if total != 0 || count != EXPECT {
        return false;
    }

    for index in 0..(INPUT_MAX - 1) {
        if out[index] != in0[index] || out[index + INPUT_MAX - 1] != in1[index] {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memcat_p_matches_linux_module_selftest() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/test_memcat_p.c"
        ));
        assert!(source.contains("Test cases for memcat_p() in lib/memcat_p.c"));
        assert!(source.contains("#define MAGIC\t\t0xf00ff00f"));
        assert!(source.contains("#define INPUT_MAX\t128"));
        assert!(source.contains("#define EXPECT\t\t(INPUT_MAX * 2 - 2)"));
        assert!(source.contains("r = (r * 725861) % 6599;"));
        assert!(source.contains("out = memcat_p(in0, in1);"));
        assert!(source.contains("if ((*p)->magic != MAGIC)"));
        assert!(source.contains("if (total)"));
        assert!(source.contains("if (i != EXPECT)"));
        assert!(source.contains("out[i] != in0[i] || out[i + INPUT_MAX - 1] != in1[i]"));
        assert!(
            source.contains("MODULE_DESCRIPTION(\"Test cases for memcat_p() in lib/memcat_p.c\")")
        );

        let (in0, in1) = build_inputs();
        assert_eq!(in0[INPUT_MAX - 1], None);
        assert_eq!(in1[INPUT_MAX - 1], None);
        assert_eq!(in0[0].expect("in0").num, -in1[0].expect("in1").num);
        assert!(test_memcat_p_passes());
    }
}
