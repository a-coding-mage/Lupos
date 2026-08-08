// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/elf-em.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016112

//! These constants define the various ELF target machines.

pub const EM_NONE: i32 = 0;
pub const EM_M32: i32 = 1;
pub const EM_SPARC: i32 = 2;
pub const EM_386: i32 = 3;
pub const EM_68K: i32 = 4;
pub const EM_88K: i32 = 5;
pub const EM_486: i32 = 6; // Perhaps disused
pub const EM_860: i32 = 7;
pub const EM_MIPS: i32 = 8; // MIPS R3000 (officially, big-endian only)
// Next two are historical and binaries and modules of these types will be
// rejected by Linux.
pub const EM_MIPS_RS3_LE: i32 = 10; // MIPS R3000 little-endian
pub const EM_MIPS_RS4_BE: i32 = 10; // MIPS R4000 big-endian

pub const EM_PARISC: i32 = 15; // HPPA
pub const EM_SPARC32PLUS: i32 = 18; // Sun's "v8plus"
pub const EM_PPC: i32 = 20; // PowerPC
pub const EM_PPC64: i32 = 21; // PowerPC64
pub const EM_SPU: i32 = 23; // Cell BE SPU
pub const EM_ARM: i32 = 40; // ARM 32 bit
pub const EM_SH: i32 = 42; // SuperH
pub const EM_SPARCV9: i32 = 43; // SPARC v9 64-bit
pub const EM_H8_300: i32 = 46; // Renesas H8/300
pub const EM_IA_64: i32 = 50; // HP/Intel IA-64
pub const EM_X86_64: i32 = 62; // AMD x86-64
pub const EM_S390: i32 = 22; // IBM S/390
pub const EM_CRIS: i32 = 76; // Axis Communications 32-bit embedded processor
pub const EM_M32R: i32 = 88; // Renesas M32R
pub const EM_MN10300: i32 = 89; // Panasonic/MEI MN10300, AM33
pub const EM_OPENRISC: i32 = 92; // OpenRISC 32-bit embedded processor
pub const EM_ARCOMPACT: i32 = 93; // ARCompact processor
pub const EM_XTENSA: i32 = 94; // Tensilica Xtensa Architecture
pub const EM_BLACKFIN: i32 = 106; // ADI Blackfin Processor
pub const EM_UNICORE: i32 = 110; // UniCore-32
pub const EM_ALTERA_NIOS2: i32 = 113; // Altera Nios II soft-core processor
pub const EM_TI_C6000: i32 = 140; // TI C6X DSPs
pub const EM_HEXAGON: i32 = 164; // QUALCOMM Hexagon
pub const EM_NDS32: i32 = 167; // Andes Technology compact code size embedded RISC processor family
pub const EM_AARCH64: i32 = 183; // ARM 64 bit
pub const EM_TILEPRO: i32 = 188; // Tilera TILEPro
pub const EM_MICROBLAZE: i32 = 189; // Xilinx MicroBlaze
pub const EM_TILEGX: i32 = 191; // Tilera TILE-Gx
pub const EM_ARCV2: i32 = 195; // ARCv2 Cores
pub const EM_RISCV: i32 = 243; // RISC-V
pub const EM_BPF: i32 = 247; // Linux BPF - in-kernel virtual machine
pub const EM_CSKY: i32 = 252; // C-SKY
pub const EM_LOONGARCH: i32 = 258; // LoongArch
pub const EM_FRV: i32 = 0x5441; // Fujitsu FR-V

// This is an interim value that we will use until the committee comes
// up with a final number.
pub const EM_ALPHA: i32 = 0x9026;

// Bogus old m32r magic number, used by old tools.
pub const EM_CYGNUS_M32R: i32 = 0x9041;
// This is the old interim value for S/390 architecture.
pub const EM_S390_OLD: i32 = 0xA390;
// Also Panasonic/MEI MN10300, AM33.
pub const EM_CYGNUS_MN10300: i32 = 0xbeef;
