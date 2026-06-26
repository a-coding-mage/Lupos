//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/boot
//! Real-mode `setup.bin` helpers ported from `vendor/linux/arch/x86/boot/`.
//!
//! lupos is now packaged as a Linux boot-protocol bzImage for GRUB. These
//! helpers are ported per CLAUDE.md Rule 1 (Linux ABI fidelity) and the
//! user's "real ports for everything" decision so the Linux source has a
//! behaviourally faithful Rust twin — useful for:
//!   * cross-checking ABI parity for `boot_params` and `biosregs`,
//!   * tools that compare lupos behaviour against the Linux setup stub,
//!   * keeping the setup/decompressor ABI exact while the live entry path
//!     routes through the generated compressed-stage extractor before
//!     handing off to `linux64_start` (with `linux32_start` retained as a
//!     compatibility entry).
//!
//! Submodules mirror the Linux file layout one-to-one.

pub mod a20;
pub mod apm;
pub mod biosregs;
pub mod bitops;
pub mod boot;
pub mod compressed;
pub mod cpu;
pub mod cpucheck;
pub mod cpuflags;
pub mod ctype;
pub mod early_serial_console;
pub mod edd;
pub mod io;
pub mod main;
pub mod memory;
pub mod mkcpustr;
pub mod pm;
pub mod printf;
pub mod regs;
pub mod startup;
pub mod string;
pub mod tty;
pub mod version;
pub mod vesa;
pub mod video;
pub mod video_bios;
pub mod video_mode;
pub mod video_vesa;
pub mod video_vga;

pub mod cmdline;
pub use cmdline::*;
pub mod legacy;
