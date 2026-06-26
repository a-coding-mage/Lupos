//! linux-parity: partial
//! linux-source: vendor/linux/arch
/// Architecture-specific code.
///
/// Currently only `x86` (32-bit protected mode) is implemented.
/// When 64-bit long mode or AArch64 support is added, new sub-modules
/// will live here behind `#[cfg(target_arch = "...")]` gates.
pub mod x86;
