//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/compressed/cmdline.c
//! test-origin: linux:vendor/linux/arch/x86/boot/compressed/cmdline.c
//! Compressed-kernel cmdline shim.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/compressed/cmdline.c
//!
//! Linux's compressed stub re-uses `../cmdline.c` (already covered by
//! the parent `boot_setup` batch) and adds two thin wrappers that
//! resolve the cmdline pointer from `boot_params_ptr`. The 32-bit
//! `boot_params.hdr.cmd_line_ptr` is combined with the 32-bit
//! `ext_cmd_line_ptr` extension to form a 64-bit physical address.

/// Pack the legacy and extended cmdline pointer fields into a single
/// 64-bit physical address. Mirrors `get_cmd_line_ptr()` in
/// boot/compressed/cmdline.c lines 18-24.
#[inline]
pub fn get_cmd_line_ptr(cmd_line_ptr: u32, ext_cmd_line_ptr: u32) -> u64 {
    cmd_line_ptr as u64 | ((ext_cmd_line_ptr as u64) << 32)
}

/// Stub for the real-mode `set_fs(seg)` helper used by the decompressor
/// when reading the cmdline. Linux stores `seg << 4` so subsequent
/// `rdfs8(addr)` calls compute `fs + addr` (segmented-style access).
/// Lupos targets long mode, so this is documentation-only.
#[inline]
pub const fn segment_base(seg: u16) -> u32 {
    (seg as u32) << 4
}

/// Compressed-kernel `cmdline_find_option(option)` — Linux's C signature
/// writes the value into a caller-provided buffer and returns its
/// length; the Rust port from `boot.rs` already does the same lookup
/// and returns a borrowed slice, which we copy on the caller's behalf
/// to match the Linux byte-count return.
pub fn cmdline_find_option(cmdline_bytes: &[u8], option: &str, buffer: &mut [u8]) -> usize {
    match crate::arch::x86::boot::cmdline_find_option(cmdline_bytes, option) {
        Some(value) => {
            let n = value.len().min(buffer.len());
            buffer[..n].copy_from_slice(&value[..n]);
            n
        }
        None => 0,
    }
}

/// Compressed-kernel `cmdline_find_option_bool(option)` —
/// delegates to the parent helper.
pub fn cmdline_find_option_bool(cmdline_bytes: &[u8], option: &str) -> bool {
    crate::arch::x86::boot::cmdline_has_option(cmdline_bytes, option)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_cmd_line_ptr_packs_extended_pointer_high() {
        assert_eq!(
            get_cmd_line_ptr(0xdeadbeef, 0x12345678),
            0x12345678_deadbeef
        );
        assert_eq!(get_cmd_line_ptr(0, 0), 0);
    }

    #[test]
    fn segment_base_left_shifts_by_4() {
        // Real-mode segments are scaled by 16 to form their linear base.
        assert_eq!(segment_base(0x1000), 0x10000);
        assert_eq!(segment_base(0), 0);
    }

    #[test]
    fn cmdline_find_option_bool_delegates_to_parent_helper() {
        assert!(cmdline_find_option_bool(
            b"quiet nokaslr console=ttyS0",
            "nokaslr"
        ));
        assert!(!cmdline_find_option_bool(b"quiet nokaslr", "absent"));
    }

    #[test]
    fn cmdline_find_option_copies_value_into_caller_buffer() {
        let mut buf = [0u8; 16];
        let n = cmdline_find_option(b"console=ttyS0 quiet", "console", &mut buf);
        assert_eq!(n, 5);
        assert_eq!(&buf[..n], b"ttyS0");
    }
}
