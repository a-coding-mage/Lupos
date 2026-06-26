//! linux-parity: complete
//! linux-source: vendor/linux/kernel/bpf/btf_relocate.c
//! test-origin: linux:vendor/linux/kernel/bpf/btf_relocate.c
//! BPF BTF relocation libbpf wrapper source.

pub const LINUX_SOURCE: &str = "vendor/linux/kernel/bpf/btf_relocate.c";
pub const LICENSE: &str = "(LGPL-2.1 OR BSD-2-Clause)";
pub const INCLUDED_SOURCE: &str = "../../tools/lib/bpf/btf_relocate.c";

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
            "/vendor/linux/kernel/bpf/btf_relocate.c"
        ));
        let mut lines = source.lines();
        assert_eq!(
            lines.next(),
            Some("// SPDX-License-Identifier: (LGPL-2.1 OR BSD-2-Clause)")
        );
        assert_eq!(
            lines.next(),
            Some("#include \"../../tools/lib/bpf/btf_relocate.c\"")
        );
        assert_eq!(lines.next(), None);
        assert_eq!(included_source(), INCLUDED_SOURCE);
        assert_eq!(LICENSE, "(LGPL-2.1 OR BSD-2-Clause)");
    }
}
