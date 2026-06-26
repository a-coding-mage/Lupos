//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/compressed/mkpiggy.c
//! test-origin: linux:vendor/linux/arch/x86/boot/compressed/mkpiggy.c
//! Compressed-kernel assembly wrapper generator.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/compressed/mkpiggy.c
//! - vendor/linux/tools/le_byteshift.h

use alloc::string::{String, ToString};
use core::fmt::Write;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MkPiggyError {
    MissingInput,
    InputTooShort,
}

pub fn render_piggy_asm(path: &str, compressed_image: &[u8]) -> Result<String, MkPiggyError> {
    if path.is_empty() {
        return Err(MkPiggyError::MissingInput);
    }
    if compressed_image.len() < 4 {
        return Err(MkPiggyError::InputTooShort);
    }

    let ilen = compressed_image.len() as u64;
    let tail = &compressed_image[compressed_image.len() - 4..];
    let olen = u32::from_le_bytes(tail.try_into().unwrap()) as u64;
    Ok(render_piggy_asm_lengths(path, ilen, olen))
}

pub fn render_piggy_asm_lengths(path: &str, input_len: u64, output_len: u64) -> String {
    let mut out = String::new();
    let _ = writeln!(out, ".section \".rodata..compressed\",\"a\",@progbits");
    let _ = writeln!(out, ".globl z_input_len");
    let _ = writeln!(out, "z_input_len = {input_len}");
    let _ = writeln!(out, ".globl z_output_len");
    let _ = writeln!(out, "z_output_len = {output_len}");
    out.push('\n');
    let _ = writeln!(out, ".globl input_data, input_data_end");
    let _ = writeln!(out, "input_data:");
    let _ = writeln!(out, ".incbin \"{}\"", path);
    let _ = writeln!(out, "input_data_end:");
    out.push('\n');
    let _ = writeln!(out, ".section \".rodata\",\"a\",@progbits");
    let _ = writeln!(out, ".globl input_len");
    let _ = writeln!(out, "input_len:\n\t.long {input_len}");
    let _ = writeln!(out, ".globl output_len");
    let _ = writeln!(out, "output_len:\n\t.long {output_len}");
    out
}

pub fn usage(program: &str) -> String {
    alloc::format!("Usage: {} compressed_file\n", program)
}

pub fn perror_label(path: &str) -> String {
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_output_length_from_last_four_little_endian_bytes() {
        let image = [0xaa, 0xbb, 0xcc, 0x44, 0x33, 0x22, 0x11];

        let out = render_piggy_asm("arch/x86/boot/compressed/vmlinux.bin.zst", &image).unwrap();

        assert!(out.contains("z_input_len = 7\n"));
        assert!(out.contains("z_output_len = 287454020\n"));
        assert!(out.contains(".incbin \"arch/x86/boot/compressed/vmlinux.bin.zst\"\n"));
        assert!(out.contains("input_len:\n\t.long 7\n"));
        assert!(out.contains("output_len:\n\t.long 287454020\n"));
    }

    #[test]
    fn rejects_missing_argument_and_too_short_input() {
        assert_eq!(
            render_piggy_asm("", &[0, 0, 0, 0]),
            Err(MkPiggyError::MissingInput)
        );
        assert_eq!(
            render_piggy_asm("vmlinux.bin", &[1, 2, 3]),
            Err(MkPiggyError::InputTooShort)
        );
        assert_eq!(usage("mkpiggy"), "Usage: mkpiggy compressed_file\n");
    }
}
