//! linux-parity: complete
//! linux-source: vendor/linux/lib/raid/xor
//! RAID XOR implementation source contracts.

pub mod arm;
pub mod arm64;
pub mod loongarch;
pub mod powerpc;
pub mod riscv;
pub mod sparc;
pub mod xor_32regs;
pub mod xor_32regs_prefetch;
pub mod xor_8regs;
pub mod xor_8regs_prefetch;
