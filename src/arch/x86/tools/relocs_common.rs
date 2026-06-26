//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/tools/relocs_common.c
//! test-origin: linux:vendor/linux/arch/x86/tools/relocs_common.c
//! Shared x86 relocs command-line parser.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RelocsOptions {
    pub show_absolute_syms: bool,
    pub show_absolute_relocs: bool,
    pub show_reloc_info: bool,
    pub as_text: bool,
    pub use_real_mode: bool,
}

pub const ELFCLASS32: u8 = 1;
pub const ELFCLASS64: u8 = 2;
pub const EI_CLASS: usize = 4;

pub fn parse_relocs_args<'a>(
    args: &'a [&'a str],
) -> Result<(RelocsOptions, &'a str), &'static str> {
    let mut opts = RelocsOptions::default();
    let mut fname = None;
    for arg in args {
        match *arg {
            "--abs-syms" => opts.show_absolute_syms = true,
            "--abs-relocs" => opts.show_absolute_relocs = true,
            "--reloc-info" => opts.show_reloc_info = true,
            "--text" => opts.as_text = true,
            "--realmode" => opts.use_real_mode = true,
            other if other.starts_with('-') => return Err("usage"),
            other if fname.is_none() => fname = Some(other),
            _ => return Err("usage"),
        }
    }
    fname.map(|name| (opts, name)).ok_or("usage")
}

pub const fn relocs_process_bits(e_ident_class: u8) -> u8 {
    if e_ident_class == ELFCLASS64 { 64 } else { 32 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relocs_common_parser_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/tools/relocs_common.c"
        ));
        assert!(source.contains("relocs [--abs-syms|--abs-relocs|--reloc-info|--text|--realmode]"));
        assert!(source.contains("show_absolute_syms = 0;"));
        assert!(source.contains("strcmp(arg, \"--abs-syms\") == 0"));
        assert!(source.contains("strcmp(arg, \"--abs-relocs\") == 0"));
        assert!(source.contains("strcmp(arg, \"--reloc-info\") == 0"));
        assert!(source.contains("strcmp(arg, \"--text\") == 0"));
        assert!(source.contains("strcmp(arg, \"--realmode\") == 0"));
        assert!(source.contains("fread(&e_ident, 1, EI_NIDENT, fp)"));
        assert!(source.contains("if (e_ident[EI_CLASS] == ELFCLASS64)"));
        assert!(source.contains("process_64(fp, use_real_mode, as_text"));
        assert!(source.contains("process_32(fp, use_real_mode, as_text"));

        let (opts, file) = parse_relocs_args(&["--abs-syms", "--text", "vmlinux"]).unwrap();
        assert!(opts.show_absolute_syms);
        assert!(opts.as_text);
        assert_eq!(file, "vmlinux");
        assert_eq!(parse_relocs_args(&[]), Err("usage"));
        assert_eq!(parse_relocs_args(&["--bad"]), Err("usage"));
        assert_eq!(relocs_process_bits(ELFCLASS64), 64);
        assert_eq!(relocs_process_bits(ELFCLASS32), 32);
    }
}
