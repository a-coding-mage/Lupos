//! linux-parity: complete
//! linux-source: vendor/linux/lib/test_fpu_impl.c
//! test-origin: linux:vendor/linux/lib/test_fpu_impl.c
//! Floating-point environment probe.

use crate::include::uapi::errno::EINVAL;

pub fn test_fpu() -> i32 {
    let a = 4.0f64;
    let b = 1e-15f64;
    let c = 1e-310f64;

    let d = a + b;
    let e = a + b / 2.0;
    let f = b / c;
    let g = a + c * f;

    if d > a && e > a && g > a { 0 } else { -EINVAL }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fpu_probe_matches_linux_rounding_and_denormal_checks() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/test_fpu_impl.c"
        ));
        assert!(source.contains("#include \"test_fpu.h\""));
        assert!(source.contains("a = 4.0;"));
        assert!(source.contains("b = 1e-15;"));
        assert!(source.contains("c = 1e-310;"));
        assert!(source.contains("d = a + b;"));
        assert!(source.contains("e = a + b / 2;"));
        assert!(source.contains("f = b / c;"));
        assert!(source.contains("g = a + c * f;"));
        assert!(source.contains("return -EINVAL;"));
        assert_eq!(test_fpu(), 0);
    }
}
