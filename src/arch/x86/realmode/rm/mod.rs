//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/realmode/rm
//! Real-mode build wrappers for setup-code sources reused by Linux.
//!
//! Linux builds several real-mode objects by compiling one-line C files in
//! `arch/x86/realmode/rm` that include the matching `arch/x86/boot/*.c`
//! source. Rust mirrors that shape with exact-source wrapper modules that
//! re-export the already ported boot implementations.

pub mod regs;
pub mod video_bios;
pub mod video_mode;
pub mod video_vesa;
pub mod video_vga;
pub mod wakemain;
