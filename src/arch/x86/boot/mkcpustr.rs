//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/mkcpustr.c
//! test-origin: linux:vendor/linux/arch/x86/boot/mkcpustr.c
//! Host-side CPU capability string table generator.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/mkcpustr.c
//! - vendor/linux/arch/x86/kernel/cpu/capflags.c

use alloc::string::String;
use core::fmt::Write;

pub fn render_x86_cap_strs(ncapints: usize, flags: &[Option<&str>]) -> String {
    let mut out = String::new();
    out.push_str("#include <asm/cpufeaturemasks.h>\n\n");
    out.push_str("static const char x86_cap_strs[] =\n");

    for i in 0..ncapints {
        for j in 0..32usize {
            let idx = i * 32 + j;
            let flag = flags.get(idx).and_then(|flag| *flag);

            if i == ncapints.saturating_sub(1) && j == 31 {
                let flag = flag.unwrap_or("");
                let _ = writeln!(out, "\t\"\\x{i:02x}\\x{j:02x}\"\"{flag}\"");
            } else if let Some(flag) = flag {
                let _ = writeln!(
                    out,
                    "#if REQUIRED_MASK{i} & (1 << {j})\n\t\"\\x{i:02x}\\x{j:02x}\"\"{flag}\\0\"\n#endif"
                );
            }
        }
    }

    out.push_str("\t;\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_required_mask_guards_for_nonfinal_flags() {
        let mut flags = alloc::vec![None; 64];
        flags[0] = Some("fpu");
        flags[33] = Some("vmx");

        let out = render_x86_cap_strs(2, &flags);

        assert!(out.starts_with("#include <asm/cpufeaturemasks.h>\n\n"));
        assert!(
            out.contains("#if REQUIRED_MASK0 & (1 << 0)\n\t\"\\x00\\x00\"\"fpu\\0\"\n#endif\n")
        );
        assert!(
            out.contains("#if REQUIRED_MASK1 & (1 << 1)\n\t\"\\x01\\x01\"\"vmx\\0\"\n#endif\n")
        );
    }

    #[test]
    fn final_entry_is_unconditional_and_consumes_compiler_nul() {
        let flags = alloc::vec![None; 32];

        let out = render_x86_cap_strs(1, &flags);

        assert!(out.contains("\t\"\\x00\\x1f\"\"\"\n\t;\n"));
        assert!(!out.contains("REQUIRED_MASK0 & (1 << 31)"));
    }
}
