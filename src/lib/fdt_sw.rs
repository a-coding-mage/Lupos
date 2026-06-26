//! linux-parity: complete
//! linux-source: vendor/linux/lib/fdt_sw.c
//! test-origin: linux:vendor/linux/lib/fdt_sw.c
//! Linux libfdt sequential-write wrapper source.

pub const LINUX_SOURCE: &str = "vendor/linux/lib/fdt_sw.c";
pub const ENV_HEADER: &str = "linux/libfdt_env.h";
pub const INCLUDED_SOURCE: &str = "../scripts/dtc/libfdt/fdt_sw.c";

pub fn included_source() -> &'static str {
    INCLUDED_SOURCE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapper_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/fdt_sw.c"
        ));
        let mut lines = source.lines();
        assert_eq!(lines.next(), Some("#include <linux/libfdt_env.h>"));
        assert_eq!(
            lines.next(),
            Some("#include \"../scripts/dtc/libfdt/fdt_sw.c\"")
        );
        assert_eq!(lines.next(), None);
        assert_eq!(included_source(), INCLUDED_SOURCE);
        assert_eq!(ENV_HEADER, "linux/libfdt_env.h");
    }
}
