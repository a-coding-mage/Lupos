//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/boot/compressed
//! Real-mode decompressor (`boot/compressed/`) helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/compressed/
//!
//! The compressed bzImage stub is the in-between stage between the
//! real-mode setup (`boot_setup`) and the protected-mode kernel image.
//! Lupos' generated bzImage now enters Linux's mirrored `head_64.S`, which
//! calls a temporary stored-block gzip `extract_kernel` shim and returns to
//! the 64-bit `linux64_start` handoff. Its payload uses Linux's
//! `mkpiggy`-style `input_data` convention, where gzip's final little-endian
//! ISIZE word becomes `z_output_len` / `output_len`. The shim calls Linux's
//! preboot gzip `__decompress` implementation and follows Linux's
//! `decompress_kernel -> parse_elf -> handle_relocations` staging. KASLR
//! placement and broader runtime-init parity are still being tightened.

pub mod acpi;
pub mod cmdline;
pub mod cpuflags;
pub mod early_serial_console;
pub mod efi;
pub mod error;
pub mod ident_map_64;
pub mod idt_64;
pub mod kaslr;
pub mod mem;
pub mod misc;
pub mod mkpiggy;
pub mod pgtable_64;
pub mod sev;
pub mod sev_handle_vc;
pub mod string;
pub mod tdx;
pub mod tdx_shared;
