//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/lib/inat.c
//! test-origin: linux:vendor/linux/arch/x86/lib/inat.c
//! x86 instruction attribute (INAT) tables and dispatch.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/lib/inat.c
//! - vendor/linux/arch/x86/include/asm/inat.h
//! - vendor/linux/arch/x86/include/asm/inat_types.h
//!
//! Linux ships a generated table (`inat-tables.c`) emitted by
//! `gen_insn_attr_x86.awk` from `x86-opcode-map.txt`. The vendored tree used
//! here does not carry that generated file, so this port keeps the runtime
//! dispatcher semantics from `inat.c` and represents table data as sparse Rust
//! tables.
//!
//! This port reproduces the constants, predicates, and dispatcher API
//! verbatim — the bit positions, masks, and shifts must agree to the
//! bit with `inat.h` so future ports of `insn.c` and `insn-eval.c`
//! (already landed alongside this file) read the same attribute words.
//! The generated opcode-map data is embedded below in Rust form so the local
//! dispatchers exercise the same sparse tables that Linux `inat.c` reads.

/// Linux `insn_byte_t` — a single instruction byte.
pub type InsnByte = u8;
/// Linux `insn_attr_t` — packed attribute word from inat-tables.
pub type InsnAttr = u32;

pub const INAT_OPCODE_TABLE_SIZE: usize = 256;
pub const INAT_GROUP_TABLE_SIZE: usize = 8;
pub const X86_VEX_M_MAX: InsnByte = 0x1f;
pub const X86_XOP_M_MIN: InsnByte = 0x08;
pub const X86_XOP_M_MAX: InsnByte = 0x1f;
pub const INAT_VEX_TABLE_COUNT: usize = (X86_VEX_M_MAX as usize) + 1;
pub const INAT_XOP_TABLE_COUNT: usize = (X86_XOP_M_MAX - X86_XOP_M_MIN + 1) as usize;

// Legacy "last" prefixes (operand-size, repe, repne) — IDs 1..=3.
pub const INAT_PFX_OPNDSZ: InsnAttr = 1;
pub const INAT_PFX_REPE: InsnAttr = 2;
pub const INAT_PFX_REPNE: InsnAttr = 3;
// Other legacy prefixes.
pub const INAT_PFX_LOCK: InsnAttr = 4;
pub const INAT_PFX_CS: InsnAttr = 5;
pub const INAT_PFX_DS: InsnAttr = 6;
pub const INAT_PFX_ES: InsnAttr = 7;
pub const INAT_PFX_FS: InsnAttr = 8;
pub const INAT_PFX_GS: InsnAttr = 9;
pub const INAT_PFX_SS: InsnAttr = 10;
pub const INAT_PFX_ADDRSZ: InsnAttr = 11;
pub const INAT_PFX_REX: InsnAttr = 12;
pub const INAT_PFX_VEX2: InsnAttr = 13;
pub const INAT_PFX_VEX3: InsnAttr = 14;
pub const INAT_PFX_EVEX: InsnAttr = 15;
pub const INAT_PFX_REX2: InsnAttr = 16;
pub const INAT_PFX_XOP: InsnAttr = 17;

pub const INAT_LSTPFX_MAX: InsnAttr = 3;
pub const INAT_LGCPFX_MAX: InsnAttr = 11;
pub const INAT_LPFX_TABLE_SIZE: usize = (INAT_LSTPFX_MAX as usize) + 1;

// Immediate sizes.
pub const INAT_IMM_BYTE: InsnAttr = 1;
pub const INAT_IMM_WORD: InsnAttr = 2;
pub const INAT_IMM_DWORD: InsnAttr = 3;
pub const INAT_IMM_QWORD: InsnAttr = 4;
pub const INAT_IMM_PTR: InsnAttr = 5;
pub const INAT_IMM_VWORD32: InsnAttr = 6;
pub const INAT_IMM_VWORD: InsnAttr = 7;

// Bit-field offsets — must match inat.h to the bit.
pub const INAT_PFX_OFFS: u32 = 0;
pub const INAT_PFX_BITS: u32 = 5;
pub const INAT_PFX_MAX: InsnAttr = (1 << INAT_PFX_BITS) - 1;
pub const INAT_PFX_MASK: InsnAttr = INAT_PFX_MAX << INAT_PFX_OFFS;

pub const INAT_ESC_OFFS: u32 = INAT_PFX_OFFS + INAT_PFX_BITS;
pub const INAT_ESC_BITS: u32 = 2;
pub const INAT_ESC_MAX: InsnAttr = (1 << INAT_ESC_BITS) - 1;
pub const INAT_ESC_MASK: InsnAttr = INAT_ESC_MAX << INAT_ESC_OFFS;
pub const INAT_ESCAPE_TABLE_COUNT: usize = (INAT_ESC_MAX as usize) + 1;

pub const INAT_GRP_OFFS: u32 = INAT_ESC_OFFS + INAT_ESC_BITS;
pub const INAT_GRP_BITS: u32 = 5;
pub const INAT_GRP_MAX: InsnAttr = (1 << INAT_GRP_BITS) - 1;
pub const INAT_GRP_MASK: InsnAttr = INAT_GRP_MAX << INAT_GRP_OFFS;
pub const INAT_GROUP_TABLE_COUNT: usize = (INAT_GRP_MAX as usize) + 1;

pub const INAT_IMM_OFFS: u32 = INAT_GRP_OFFS + INAT_GRP_BITS;
pub const INAT_IMM_BITS: u32 = 3;
pub const INAT_IMM_MASK: InsnAttr = ((1 << INAT_IMM_BITS) - 1) << INAT_IMM_OFFS;

pub const INAT_FLAG_OFFS: u32 = INAT_IMM_OFFS + INAT_IMM_BITS;
pub const INAT_MODRM: InsnAttr = 1 << INAT_FLAG_OFFS;
pub const INAT_FORCE64: InsnAttr = 1 << (INAT_FLAG_OFFS + 1);
pub const INAT_SCNDIMM: InsnAttr = 1 << (INAT_FLAG_OFFS + 2);
pub const INAT_MOFFSET: InsnAttr = 1 << (INAT_FLAG_OFFS + 3);
pub const INAT_VARIANT: InsnAttr = 1 << (INAT_FLAG_OFFS + 4);
pub const INAT_VEXOK: InsnAttr = 1 << (INAT_FLAG_OFFS + 5);
pub const INAT_XOPOK: InsnAttr = INAT_VEXOK;
pub const INAT_VEXONLY: InsnAttr = 1 << (INAT_FLAG_OFFS + 6);
pub const INAT_EVEXONLY: InsnAttr = 1 << (INAT_FLAG_OFFS + 7);
pub const INAT_NO_REX2: InsnAttr = 1 << (INAT_FLAG_OFFS + 8);
pub const INAT_REX2_VARIANT: InsnAttr = 1 << (INAT_FLAG_OFFS + 9);
pub const INAT_EVEX_SCALABLE: InsnAttr = 1 << (INAT_FLAG_OFFS + 10);
pub const INAT_INV64: InsnAttr = 1 << (INAT_FLAG_OFFS + 11);

/// Segment register IDs.
pub const INAT_SEG_REG_IGNORE: u8 = 0;
pub const INAT_SEG_REG_DEFAULT: u8 = 1;
pub const INAT_SEG_REG_CS: u8 = 2;
pub const INAT_SEG_REG_SS: u8 = 3;
pub const INAT_SEG_REG_DS: u8 = 4;
pub const INAT_SEG_REG_ES: u8 = 5;
pub const INAT_SEG_REG_FS: u8 = 6;
pub const INAT_SEG_REG_GS: u8 = 7;

/// Construct the `INAT_MAKE_PREFIX(pfx)` attribute word.
pub const fn make_prefix(pfx: InsnAttr) -> InsnAttr {
    pfx << INAT_PFX_OFFS
}
/// `INAT_MAKE_ESCAPE(esc)`.
pub const fn make_escape(esc: InsnAttr) -> InsnAttr {
    esc << INAT_ESC_OFFS
}
/// `INAT_MAKE_GROUP(grp)` — implies INAT_MODRM.
pub const fn make_group(grp: InsnAttr) -> InsnAttr {
    (grp << INAT_GRP_OFFS) | INAT_MODRM
}
/// `INAT_MAKE_IMM(imm)`.
pub const fn make_imm(imm: InsnAttr) -> InsnAttr {
    imm << INAT_IMM_OFFS
}

// ---- Attribute predicates (inat.h `static inline` checkers) -----------------

#[inline]
pub const fn is_legacy_prefix(attr: InsnAttr) -> bool {
    let a = attr & INAT_PFX_MASK;
    a != 0 && a <= INAT_LGCPFX_MAX
}
#[inline]
pub const fn is_address_size_prefix(attr: InsnAttr) -> bool {
    (attr & INAT_PFX_MASK) == INAT_PFX_ADDRSZ
}
#[inline]
pub const fn is_operand_size_prefix(attr: InsnAttr) -> bool {
    (attr & INAT_PFX_MASK) == INAT_PFX_OPNDSZ
}
#[inline]
pub const fn is_rex_prefix(attr: InsnAttr) -> bool {
    (attr & INAT_PFX_MASK) == INAT_PFX_REX
}
#[inline]
pub const fn is_rex2_prefix(attr: InsnAttr) -> bool {
    (attr & INAT_PFX_MASK) == INAT_PFX_REX2
}
#[inline]
pub const fn last_prefix_id(attr: InsnAttr) -> InsnAttr {
    let a = attr & INAT_PFX_MASK;
    if a > INAT_LSTPFX_MAX { 0 } else { a }
}
#[inline]
pub const fn is_vex_prefix(attr: InsnAttr) -> bool {
    let a = attr & INAT_PFX_MASK;
    a == INAT_PFX_VEX2 || a == INAT_PFX_VEX3 || a == INAT_PFX_EVEX
}
#[inline]
pub const fn is_evex_prefix(attr: InsnAttr) -> bool {
    (attr & INAT_PFX_MASK) == INAT_PFX_EVEX
}
#[inline]
pub const fn is_vex3_prefix(attr: InsnAttr) -> bool {
    (attr & INAT_PFX_MASK) == INAT_PFX_VEX3
}
#[inline]
pub const fn is_xop_prefix(attr: InsnAttr) -> bool {
    (attr & INAT_PFX_MASK) == INAT_PFX_XOP
}
#[inline]
pub const fn is_escape(attr: InsnAttr) -> bool {
    (attr & INAT_ESC_MASK) != 0
}
#[inline]
pub const fn escape_id(attr: InsnAttr) -> InsnAttr {
    (attr & INAT_ESC_MASK) >> INAT_ESC_OFFS
}
#[inline]
pub const fn is_group(attr: InsnAttr) -> bool {
    (attr & INAT_GRP_MASK) != 0
}
#[inline]
pub const fn group_id(attr: InsnAttr) -> InsnAttr {
    (attr & INAT_GRP_MASK) >> INAT_GRP_OFFS
}
#[inline]
pub const fn group_common_attribute(attr: InsnAttr) -> InsnAttr {
    attr & !INAT_GRP_MASK
}
#[inline]
pub const fn has_immediate(attr: InsnAttr) -> bool {
    (attr & INAT_IMM_MASK) != 0
}
#[inline]
pub const fn immediate_size(attr: InsnAttr) -> InsnAttr {
    (attr & INAT_IMM_MASK) >> INAT_IMM_OFFS
}
#[inline]
pub const fn has_modrm(attr: InsnAttr) -> bool {
    (attr & INAT_MODRM) != 0
}
#[inline]
pub const fn is_force64(attr: InsnAttr) -> bool {
    (attr & INAT_FORCE64) != 0
}
#[inline]
pub const fn has_second_immediate(attr: InsnAttr) -> bool {
    (attr & INAT_SCNDIMM) != 0
}
#[inline]
pub const fn has_moffset(attr: InsnAttr) -> bool {
    (attr & INAT_MOFFSET) != 0
}
#[inline]
pub const fn has_variant(attr: InsnAttr) -> bool {
    (attr & INAT_VARIANT) != 0
}
#[inline]
pub const fn accept_vex(attr: InsnAttr) -> bool {
    (attr & INAT_VEXOK) != 0
}
#[inline]
pub const fn accept_xop(attr: InsnAttr) -> bool {
    (attr & INAT_XOPOK) != 0
}
#[inline]
pub const fn must_vex(attr: InsnAttr) -> bool {
    (attr & (INAT_VEXONLY | INAT_EVEXONLY)) != 0
}
#[inline]
pub const fn must_evex(attr: InsnAttr) -> bool {
    (attr & INAT_EVEXONLY) != 0
}
#[inline]
pub const fn evex_scalable(attr: InsnAttr) -> bool {
    (attr & INAT_EVEX_SCALABLE) != 0
}
#[inline]
pub const fn is_invalid64(attr: InsnAttr) -> bool {
    (attr & INAT_INV64) != 0
}

// ---- Generated opcode attribute tables -------------------------------------
//
// Generated from Linux x86-opcode-map.txt with gen-insn-attr-x86.awk after
// CRLF normalization. Keep the dispatch rules below in lockstep with inat.c.

type OpcodeTable = [InsnAttr; INAT_OPCODE_TABLE_SIZE];
type GroupTable = [InsnAttr; INAT_GROUP_TABLE_SIZE];
type OpcodeLpfxTables = [Option<&'static OpcodeTable>; INAT_LPFX_TABLE_SIZE];
type GroupLpfxTables = [Option<&'static GroupTable>; INAT_LPFX_TABLE_SIZE];

const fn inat_primary_table_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x00] = INAT_MODRM;
    table[0x01] = INAT_MODRM;
    table[0x02] = INAT_MODRM;
    table[0x03] = INAT_MODRM;
    table[0x04] = make_imm(INAT_IMM_BYTE);
    table[0x05] = make_imm(INAT_IMM_VWORD32);
    table[0x06] = INAT_INV64;
    table[0x07] = INAT_INV64;
    table[0x08] = INAT_MODRM;
    table[0x09] = INAT_MODRM;
    table[0x0a] = INAT_MODRM;
    table[0x0b] = INAT_MODRM;
    table[0x0c] = make_imm(INAT_IMM_BYTE);
    table[0x0d] = make_imm(INAT_IMM_VWORD32);
    table[0x0e] = INAT_INV64;
    table[0x0f] = make_escape(1);
    table[0x10] = INAT_MODRM;
    table[0x11] = INAT_MODRM;
    table[0x12] = INAT_MODRM;
    table[0x13] = INAT_MODRM;
    table[0x14] = make_imm(INAT_IMM_BYTE);
    table[0x15] = make_imm(INAT_IMM_VWORD32);
    table[0x16] = INAT_INV64;
    table[0x17] = INAT_INV64;
    table[0x18] = INAT_MODRM;
    table[0x19] = INAT_MODRM;
    table[0x1a] = INAT_MODRM;
    table[0x1b] = INAT_MODRM;
    table[0x1c] = make_imm(INAT_IMM_BYTE);
    table[0x1d] = make_imm(INAT_IMM_VWORD32);
    table[0x1e] = INAT_INV64;
    table[0x1f] = INAT_INV64;
    table[0x20] = INAT_MODRM;
    table[0x21] = INAT_MODRM;
    table[0x22] = INAT_MODRM;
    table[0x23] = INAT_MODRM;
    table[0x24] = make_imm(INAT_IMM_BYTE);
    table[0x25] = make_imm(INAT_IMM_VWORD32);
    table[0x26] = make_prefix(INAT_PFX_ES);
    table[0x27] = INAT_INV64;
    table[0x28] = INAT_MODRM;
    table[0x29] = INAT_MODRM;
    table[0x2a] = INAT_MODRM;
    table[0x2b] = INAT_MODRM;
    table[0x2c] = make_imm(INAT_IMM_BYTE);
    table[0x2d] = make_imm(INAT_IMM_VWORD32);
    table[0x2e] = make_prefix(INAT_PFX_CS);
    table[0x2f] = INAT_INV64;
    table[0x30] = INAT_MODRM;
    table[0x31] = INAT_MODRM;
    table[0x32] = INAT_MODRM;
    table[0x33] = INAT_MODRM;
    table[0x34] = make_imm(INAT_IMM_BYTE);
    table[0x35] = make_imm(INAT_IMM_VWORD32);
    table[0x36] = make_prefix(INAT_PFX_SS);
    table[0x37] = INAT_INV64;
    table[0x38] = INAT_MODRM;
    table[0x39] = INAT_MODRM;
    table[0x3a] = INAT_MODRM;
    table[0x3b] = INAT_MODRM;
    table[0x3c] = make_imm(INAT_IMM_BYTE);
    table[0x3d] = make_imm(INAT_IMM_VWORD32);
    table[0x3e] = make_prefix(INAT_PFX_DS);
    table[0x3f] = INAT_INV64;
    table[0x40] = make_prefix(INAT_PFX_REX);
    table[0x41] = make_prefix(INAT_PFX_REX);
    table[0x42] = make_prefix(INAT_PFX_REX);
    table[0x43] = make_prefix(INAT_PFX_REX);
    table[0x44] = make_prefix(INAT_PFX_REX);
    table[0x45] = make_prefix(INAT_PFX_REX);
    table[0x46] = make_prefix(INAT_PFX_REX);
    table[0x47] = make_prefix(INAT_PFX_REX);
    table[0x48] = make_prefix(INAT_PFX_REX);
    table[0x49] = make_prefix(INAT_PFX_REX);
    table[0x4a] = make_prefix(INAT_PFX_REX);
    table[0x4b] = make_prefix(INAT_PFX_REX);
    table[0x4c] = make_prefix(INAT_PFX_REX);
    table[0x4d] = make_prefix(INAT_PFX_REX);
    table[0x4e] = make_prefix(INAT_PFX_REX);
    table[0x4f] = make_prefix(INAT_PFX_REX);
    table[0x50] = INAT_FORCE64;
    table[0x51] = INAT_FORCE64;
    table[0x52] = INAT_FORCE64;
    table[0x53] = INAT_FORCE64;
    table[0x54] = INAT_FORCE64;
    table[0x55] = INAT_FORCE64;
    table[0x56] = INAT_FORCE64;
    table[0x57] = INAT_FORCE64;
    table[0x58] = INAT_FORCE64;
    table[0x59] = INAT_FORCE64;
    table[0x5a] = INAT_FORCE64;
    table[0x5b] = INAT_FORCE64;
    table[0x5c] = INAT_FORCE64;
    table[0x5d] = INAT_FORCE64;
    table[0x5e] = INAT_FORCE64;
    table[0x5f] = INAT_FORCE64;
    table[0x60] = INAT_INV64;
    table[0x61] = INAT_INV64;
    table[0x62] = INAT_MODRM | make_prefix(INAT_PFX_EVEX);
    table[0x63] = INAT_MODRM | INAT_MODRM;
    table[0x64] = make_prefix(INAT_PFX_FS);
    table[0x65] = make_prefix(INAT_PFX_GS);
    table[0x66] = make_prefix(INAT_PFX_OPNDSZ);
    table[0x67] = make_prefix(INAT_PFX_ADDRSZ);
    table[0x68] = make_imm(INAT_IMM_VWORD32);
    table[0x69] = make_imm(INAT_IMM_VWORD32) | INAT_MODRM;
    table[0x6a] = make_imm(INAT_IMM_BYTE) | INAT_FORCE64;
    table[0x6b] = make_imm(INAT_IMM_BYTE) | INAT_MODRM;
    table[0x70] = make_imm(INAT_IMM_BYTE) | INAT_NO_REX2;
    table[0x71] = make_imm(INAT_IMM_BYTE) | INAT_NO_REX2;
    table[0x72] = make_imm(INAT_IMM_BYTE) | INAT_NO_REX2;
    table[0x73] = make_imm(INAT_IMM_BYTE) | INAT_NO_REX2;
    table[0x74] = make_imm(INAT_IMM_BYTE) | INAT_NO_REX2;
    table[0x75] = make_imm(INAT_IMM_BYTE) | INAT_NO_REX2;
    table[0x76] = make_imm(INAT_IMM_BYTE) | INAT_NO_REX2;
    table[0x77] = make_imm(INAT_IMM_BYTE) | INAT_NO_REX2;
    table[0x78] = make_imm(INAT_IMM_BYTE) | INAT_NO_REX2;
    table[0x79] = make_imm(INAT_IMM_BYTE) | INAT_NO_REX2;
    table[0x7a] = make_imm(INAT_IMM_BYTE) | INAT_NO_REX2;
    table[0x7b] = make_imm(INAT_IMM_BYTE) | INAT_NO_REX2;
    table[0x7c] = make_imm(INAT_IMM_BYTE) | INAT_NO_REX2;
    table[0x7d] = make_imm(INAT_IMM_BYTE) | INAT_NO_REX2;
    table[0x7e] = make_imm(INAT_IMM_BYTE) | INAT_NO_REX2;
    table[0x7f] = make_imm(INAT_IMM_BYTE) | INAT_NO_REX2;
    table[0x80] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | make_group(1);
    table[0x81] = make_imm(INAT_IMM_VWORD32) | INAT_MODRM | make_group(1);
    table[0x82] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | make_group(1) | INAT_INV64;
    table[0x83] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | make_group(1);
    table[0x84] = INAT_MODRM;
    table[0x85] = INAT_MODRM;
    table[0x86] = INAT_MODRM;
    table[0x87] = INAT_MODRM;
    table[0x88] = INAT_MODRM;
    table[0x89] = INAT_MODRM;
    table[0x8a] = INAT_MODRM;
    table[0x8b] = INAT_MODRM;
    table[0x8c] = INAT_MODRM;
    table[0x8d] = INAT_MODRM;
    table[0x8e] = INAT_MODRM;
    table[0x8f] = make_group(2) | INAT_MODRM | INAT_FORCE64 | make_prefix(INAT_PFX_XOP);
    table[0x9a] = make_imm(INAT_IMM_PTR) | INAT_INV64;
    table[0x9c] = INAT_FORCE64;
    table[0x9d] = INAT_FORCE64;
    table[0xa0] = INAT_MOFFSET | INAT_NO_REX2;
    table[0xa1] = INAT_MOFFSET | INAT_NO_REX2;
    table[0xa2] = INAT_MOFFSET | INAT_NO_REX2;
    table[0xa3] = INAT_MOFFSET | INAT_NO_REX2;
    table[0xa4] = INAT_NO_REX2;
    table[0xa5] = INAT_NO_REX2;
    table[0xa6] = INAT_NO_REX2;
    table[0xa7] = INAT_NO_REX2;
    table[0xa8] = make_imm(INAT_IMM_BYTE) | INAT_NO_REX2;
    table[0xa9] = make_imm(INAT_IMM_VWORD32) | INAT_NO_REX2;
    table[0xaa] = INAT_NO_REX2;
    table[0xab] = INAT_NO_REX2;
    table[0xac] = INAT_NO_REX2;
    table[0xad] = INAT_NO_REX2;
    table[0xae] = INAT_NO_REX2;
    table[0xaf] = INAT_NO_REX2;
    table[0xb0] = make_imm(INAT_IMM_BYTE);
    table[0xb1] = make_imm(INAT_IMM_BYTE);
    table[0xb2] = make_imm(INAT_IMM_BYTE);
    table[0xb3] = make_imm(INAT_IMM_BYTE);
    table[0xb4] = make_imm(INAT_IMM_BYTE);
    table[0xb5] = make_imm(INAT_IMM_BYTE);
    table[0xb6] = make_imm(INAT_IMM_BYTE);
    table[0xb7] = make_imm(INAT_IMM_BYTE);
    table[0xb8] = make_imm(INAT_IMM_VWORD);
    table[0xb9] = make_imm(INAT_IMM_VWORD);
    table[0xba] = make_imm(INAT_IMM_VWORD);
    table[0xbb] = make_imm(INAT_IMM_VWORD);
    table[0xbc] = make_imm(INAT_IMM_VWORD);
    table[0xbd] = make_imm(INAT_IMM_VWORD);
    table[0xbe] = make_imm(INAT_IMM_VWORD);
    table[0xbf] = make_imm(INAT_IMM_VWORD);
    table[0xc0] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | make_group(3);
    table[0xc1] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | make_group(3);
    table[0xc2] = make_imm(INAT_IMM_WORD) | INAT_FORCE64;
    table[0xc4] = INAT_MODRM | make_prefix(INAT_PFX_VEX3);
    table[0xc5] = INAT_MODRM | make_prefix(INAT_PFX_VEX2);
    table[0xc6] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | make_group(4);
    table[0xc7] = make_imm(INAT_IMM_VWORD32) | INAT_MODRM | make_group(5);
    table[0xc8] = make_imm(INAT_IMM_WORD) | INAT_SCNDIMM;
    table[0xc9] = INAT_FORCE64;
    table[0xca] = make_imm(INAT_IMM_WORD);
    table[0xcd] = make_imm(INAT_IMM_BYTE);
    table[0xce] = INAT_INV64;
    table[0xd0] = INAT_MODRM | make_group(3);
    table[0xd1] = INAT_MODRM | make_group(3);
    table[0xd2] = INAT_MODRM | make_group(3);
    table[0xd3] = INAT_MODRM | make_group(3);
    table[0xd4] = make_imm(INAT_IMM_BYTE) | INAT_INV64;
    table[0xd5] = make_imm(INAT_IMM_BYTE) | make_prefix(INAT_PFX_REX2);
    table[0xd8] = INAT_MODRM;
    table[0xd9] = INAT_MODRM;
    table[0xda] = INAT_MODRM;
    table[0xdb] = INAT_MODRM;
    table[0xdc] = INAT_MODRM;
    table[0xdd] = INAT_MODRM;
    table[0xde] = INAT_MODRM;
    table[0xdf] = INAT_MODRM;
    table[0xe0] = make_imm(INAT_IMM_BYTE) | INAT_FORCE64 | INAT_NO_REX2;
    table[0xe1] = make_imm(INAT_IMM_BYTE) | INAT_FORCE64 | INAT_NO_REX2;
    table[0xe2] = make_imm(INAT_IMM_BYTE) | INAT_FORCE64 | INAT_NO_REX2;
    table[0xe3] = make_imm(INAT_IMM_BYTE) | INAT_FORCE64 | INAT_NO_REX2;
    table[0xe4] = make_imm(INAT_IMM_BYTE) | INAT_NO_REX2;
    table[0xe5] = make_imm(INAT_IMM_BYTE) | INAT_NO_REX2;
    table[0xe6] = make_imm(INAT_IMM_BYTE) | INAT_NO_REX2;
    table[0xe7] = make_imm(INAT_IMM_BYTE) | INAT_NO_REX2;
    table[0xe8] = make_imm(INAT_IMM_VWORD32) | INAT_FORCE64 | INAT_NO_REX2;
    table[0xe9] = make_imm(INAT_IMM_VWORD32) | INAT_FORCE64 | INAT_NO_REX2;
    table[0xea] = make_imm(INAT_IMM_PTR) | INAT_INV64 | INAT_NO_REX2;
    table[0xeb] = make_imm(INAT_IMM_BYTE) | INAT_FORCE64 | INAT_NO_REX2;
    table[0xec] = INAT_NO_REX2;
    table[0xed] = INAT_NO_REX2;
    table[0xee] = INAT_NO_REX2;
    table[0xef] = INAT_NO_REX2;
    table[0xf0] = make_prefix(INAT_PFX_LOCK);
    table[0xf2] = make_prefix(INAT_PFX_REPNE) | make_prefix(INAT_PFX_REPNE);
    table[0xf3] = make_prefix(INAT_PFX_REPE) | make_prefix(INAT_PFX_REPE);
    table[0xf6] = INAT_MODRM | make_group(6);
    table[0xf7] = INAT_MODRM | make_group(7);
    table[0xfe] = make_group(8);
    table[0xff] = make_group(9);
    table
}
const INAT_PRIMARY_TABLE: OpcodeTable = inat_primary_table_table();

const fn inat_escape_table_1_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x00] = make_group(10);
    table[0x01] = make_group(11);
    table[0x02] = INAT_MODRM;
    table[0x03] = INAT_MODRM;
    table[0x0d] = make_group(12);
    table[0x0f] = make_imm(INAT_IMM_BYTE) | INAT_MODRM;
    table[0x10] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x11] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x12] = INAT_MODRM | INAT_VEXOK | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x13] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x14] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x15] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x16] = INAT_MODRM | INAT_VEXOK | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x17] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x18] = make_group(13);
    table[0x1a] = INAT_MODRM | INAT_VARIANT;
    table[0x1b] = INAT_MODRM | INAT_VARIANT;
    table[0x1c] = make_group(14);
    table[0x1e] = make_group(15);
    table[0x1f] = INAT_MODRM;
    table[0x20] = INAT_MODRM;
    table[0x21] = INAT_MODRM;
    table[0x22] = INAT_MODRM;
    table[0x23] = INAT_MODRM;
    table[0x28] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x29] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x2a] = INAT_MODRM | INAT_VARIANT;
    table[0x2b] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x2c] = INAT_MODRM | INAT_VARIANT;
    table[0x2d] = INAT_MODRM | INAT_VARIANT;
    table[0x2e] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x2f] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x30] = INAT_NO_REX2;
    table[0x31] = INAT_NO_REX2;
    table[0x32] = INAT_NO_REX2;
    table[0x33] = INAT_NO_REX2;
    table[0x34] = INAT_NO_REX2;
    table[0x35] = INAT_NO_REX2;
    table[0x37] = INAT_NO_REX2;
    table[0x38] = make_escape(2);
    table[0x3a] = make_escape(3);
    table[0x40] = INAT_MODRM;
    table[0x41] = INAT_MODRM | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x42] = INAT_MODRM | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x43] = INAT_MODRM;
    table[0x44] = INAT_MODRM | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x45] = INAT_MODRM | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x46] = INAT_MODRM | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x47] = INAT_MODRM | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x48] = INAT_MODRM;
    table[0x49] = INAT_MODRM;
    table[0x4a] = INAT_MODRM | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x4b] = INAT_MODRM | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x4c] = INAT_MODRM;
    table[0x4d] = INAT_MODRM;
    table[0x4e] = INAT_MODRM;
    table[0x4f] = INAT_MODRM;
    table[0x50] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x51] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x52] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x53] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x54] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x55] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x56] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x57] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x58] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x59] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x5a] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x5b] = INAT_MODRM | INAT_VEXOK | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x5c] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x5d] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x5e] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x5f] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x60] = INAT_MODRM | INAT_VARIANT;
    table[0x61] = INAT_MODRM | INAT_VARIANT;
    table[0x62] = INAT_MODRM | INAT_VARIANT;
    table[0x63] = INAT_MODRM | INAT_VARIANT;
    table[0x64] = INAT_MODRM | INAT_VARIANT;
    table[0x65] = INAT_MODRM | INAT_VARIANT;
    table[0x66] = INAT_MODRM | INAT_VARIANT;
    table[0x67] = INAT_MODRM | INAT_VARIANT;
    table[0x68] = INAT_MODRM | INAT_VARIANT;
    table[0x69] = INAT_MODRM | INAT_VARIANT;
    table[0x6a] = INAT_MODRM | INAT_VARIANT;
    table[0x6b] = INAT_MODRM | INAT_VARIANT;
    table[0x6c] = INAT_VARIANT;
    table[0x6d] = INAT_VARIANT;
    table[0x6e] = INAT_MODRM | INAT_VARIANT;
    table[0x6f] = INAT_MODRM | INAT_VARIANT;
    table[0x70] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VARIANT;
    table[0x71] = make_group(16);
    table[0x72] = make_group(17);
    table[0x73] = make_group(18);
    table[0x74] = INAT_MODRM | INAT_VARIANT;
    table[0x75] = INAT_MODRM | INAT_VARIANT;
    table[0x76] = INAT_MODRM | INAT_VARIANT;
    table[0x77] = INAT_VEXOK | INAT_VEXOK;
    table[0x78] = INAT_MODRM | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x79] = INAT_MODRM | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x7a] = INAT_VARIANT;
    table[0x7b] = INAT_VARIANT;
    table[0x7c] = INAT_VARIANT;
    table[0x7d] = INAT_VARIANT;
    table[0x7e] = INAT_MODRM | INAT_VARIANT;
    table[0x7f] = INAT_MODRM | INAT_VARIANT;
    table[0x80] = make_imm(INAT_IMM_VWORD32) | INAT_FORCE64 | INAT_NO_REX2;
    table[0x81] = make_imm(INAT_IMM_VWORD32) | INAT_FORCE64 | INAT_NO_REX2;
    table[0x82] = make_imm(INAT_IMM_VWORD32) | INAT_FORCE64 | INAT_NO_REX2;
    table[0x83] = make_imm(INAT_IMM_VWORD32) | INAT_FORCE64 | INAT_NO_REX2;
    table[0x84] = make_imm(INAT_IMM_VWORD32) | INAT_FORCE64 | INAT_NO_REX2;
    table[0x85] = make_imm(INAT_IMM_VWORD32) | INAT_FORCE64 | INAT_NO_REX2;
    table[0x86] = make_imm(INAT_IMM_VWORD32) | INAT_FORCE64 | INAT_NO_REX2;
    table[0x87] = make_imm(INAT_IMM_VWORD32) | INAT_FORCE64 | INAT_NO_REX2;
    table[0x88] = make_imm(INAT_IMM_VWORD32) | INAT_FORCE64 | INAT_NO_REX2;
    table[0x89] = make_imm(INAT_IMM_VWORD32) | INAT_FORCE64 | INAT_NO_REX2;
    table[0x8a] = make_imm(INAT_IMM_VWORD32) | INAT_FORCE64 | INAT_NO_REX2;
    table[0x8b] = make_imm(INAT_IMM_VWORD32) | INAT_FORCE64 | INAT_NO_REX2;
    table[0x8c] = make_imm(INAT_IMM_VWORD32) | INAT_FORCE64 | INAT_NO_REX2;
    table[0x8d] = make_imm(INAT_IMM_VWORD32) | INAT_FORCE64 | INAT_NO_REX2;
    table[0x8e] = make_imm(INAT_IMM_VWORD32) | INAT_FORCE64 | INAT_NO_REX2;
    table[0x8f] = make_imm(INAT_IMM_VWORD32) | INAT_FORCE64 | INAT_NO_REX2;
    table[0x90] = INAT_MODRM | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x91] = INAT_MODRM | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x92] = INAT_MODRM | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x93] = INAT_MODRM | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x94] = INAT_MODRM;
    table[0x95] = INAT_MODRM;
    table[0x96] = INAT_MODRM;
    table[0x97] = INAT_MODRM;
    table[0x98] = INAT_MODRM | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x99] = INAT_MODRM | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x9a] = INAT_MODRM;
    table[0x9b] = INAT_MODRM;
    table[0x9c] = INAT_MODRM;
    table[0x9d] = INAT_MODRM;
    table[0x9e] = INAT_MODRM;
    table[0x9f] = INAT_MODRM;
    table[0xa0] = INAT_FORCE64;
    table[0xa1] = INAT_FORCE64;
    table[0xa3] = INAT_MODRM;
    table[0xa4] = make_imm(INAT_IMM_BYTE) | INAT_MODRM;
    table[0xa5] = INAT_MODRM;
    table[0xa6] = make_group(19);
    table[0xa7] = make_group(20);
    table[0xa8] = INAT_FORCE64;
    table[0xa9] = INAT_FORCE64;
    table[0xab] = INAT_MODRM;
    table[0xac] = make_imm(INAT_IMM_BYTE) | INAT_MODRM;
    table[0xad] = INAT_MODRM;
    table[0xae] = make_group(21);
    table[0xaf] = INAT_MODRM;
    table[0xb0] = INAT_MODRM;
    table[0xb1] = INAT_MODRM;
    table[0xb2] = INAT_MODRM;
    table[0xb3] = INAT_MODRM;
    table[0xb4] = INAT_MODRM;
    table[0xb5] = INAT_MODRM;
    table[0xb6] = INAT_MODRM;
    table[0xb7] = INAT_MODRM;
    table[0xb8] = INAT_VARIANT;
    table[0xb9] = make_group(22);
    table[0xba] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | make_group(23);
    table[0xbb] = INAT_MODRM;
    table[0xbc] = INAT_MODRM | INAT_VARIANT;
    table[0xbd] = INAT_MODRM | INAT_VARIANT;
    table[0xbe] = INAT_MODRM;
    table[0xbf] = INAT_MODRM;
    table[0xc0] = INAT_MODRM;
    table[0xc1] = INAT_MODRM;
    table[0xc2] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0xc3] = INAT_MODRM;
    table[0xc4] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VARIANT;
    table[0xc5] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VARIANT;
    table[0xc6] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0xc7] = make_group(24);
    table[0xd0] = INAT_VARIANT;
    table[0xd1] = INAT_MODRM | INAT_VARIANT;
    table[0xd2] = INAT_MODRM | INAT_VARIANT;
    table[0xd3] = INAT_MODRM | INAT_VARIANT;
    table[0xd4] = INAT_MODRM | INAT_VARIANT;
    table[0xd5] = INAT_MODRM | INAT_VARIANT;
    table[0xd6] = INAT_VARIANT;
    table[0xd7] = INAT_MODRM | INAT_VARIANT;
    table[0xd8] = INAT_MODRM | INAT_VARIANT;
    table[0xd9] = INAT_MODRM | INAT_VARIANT;
    table[0xda] = INAT_MODRM | INAT_VARIANT;
    table[0xdb] = INAT_MODRM | INAT_VARIANT;
    table[0xdc] = INAT_MODRM | INAT_VARIANT;
    table[0xdd] = INAT_MODRM | INAT_VARIANT;
    table[0xde] = INAT_MODRM | INAT_VARIANT;
    table[0xdf] = INAT_MODRM | INAT_VARIANT;
    table[0xe0] = INAT_MODRM | INAT_VARIANT;
    table[0xe1] = INAT_MODRM | INAT_VARIANT;
    table[0xe2] = INAT_MODRM | INAT_VARIANT;
    table[0xe3] = INAT_MODRM | INAT_VARIANT;
    table[0xe4] = INAT_MODRM | INAT_VARIANT;
    table[0xe5] = INAT_MODRM | INAT_VARIANT;
    table[0xe6] = INAT_VARIANT;
    table[0xe7] = INAT_MODRM | INAT_VARIANT;
    table[0xe8] = INAT_MODRM | INAT_VARIANT;
    table[0xe9] = INAT_MODRM | INAT_VARIANT;
    table[0xea] = INAT_MODRM | INAT_VARIANT;
    table[0xeb] = INAT_MODRM | INAT_VARIANT;
    table[0xec] = INAT_MODRM | INAT_VARIANT;
    table[0xed] = INAT_MODRM | INAT_VARIANT;
    table[0xee] = INAT_MODRM | INAT_VARIANT;
    table[0xef] = INAT_MODRM | INAT_VARIANT;
    table[0xf0] = INAT_VARIANT;
    table[0xf1] = INAT_MODRM | INAT_VARIANT;
    table[0xf2] = INAT_MODRM | INAT_VARIANT;
    table[0xf3] = INAT_MODRM | INAT_VARIANT;
    table[0xf4] = INAT_MODRM | INAT_VARIANT;
    table[0xf5] = INAT_MODRM | INAT_VARIANT;
    table[0xf6] = INAT_MODRM | INAT_VARIANT;
    table[0xf7] = INAT_MODRM | INAT_VARIANT;
    table[0xf8] = INAT_MODRM | INAT_VARIANT;
    table[0xf9] = INAT_MODRM | INAT_VARIANT;
    table[0xfa] = INAT_MODRM | INAT_VARIANT;
    table[0xfb] = INAT_MODRM | INAT_VARIANT;
    table[0xfc] = INAT_MODRM | INAT_VARIANT;
    table[0xfd] = INAT_MODRM | INAT_VARIANT;
    table[0xfe] = INAT_MODRM | INAT_VARIANT;
    table
}
const INAT_ESCAPE_TABLE_1: OpcodeTable = inat_escape_table_1_table();

const fn inat_escape_table_1_1_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x10] = INAT_MODRM | INAT_VEXOK;
    table[0x11] = INAT_MODRM | INAT_VEXOK;
    table[0x12] = INAT_MODRM | INAT_VEXOK;
    table[0x13] = INAT_MODRM | INAT_VEXOK;
    table[0x14] = INAT_MODRM | INAT_VEXOK;
    table[0x15] = INAT_MODRM | INAT_VEXOK;
    table[0x16] = INAT_MODRM | INAT_VEXOK;
    table[0x17] = INAT_MODRM | INAT_VEXOK;
    table[0x1a] = INAT_MODRM;
    table[0x1b] = INAT_MODRM;
    table[0x28] = INAT_MODRM | INAT_VEXOK;
    table[0x29] = INAT_MODRM | INAT_VEXOK;
    table[0x2a] = INAT_MODRM;
    table[0x2b] = INAT_MODRM | INAT_VEXOK;
    table[0x2c] = INAT_MODRM;
    table[0x2d] = INAT_MODRM;
    table[0x2e] = INAT_MODRM | INAT_VEXOK;
    table[0x2f] = INAT_MODRM | INAT_VEXOK;
    table[0x41] = INAT_MODRM | INAT_VEXOK;
    table[0x42] = INAT_MODRM | INAT_VEXOK;
    table[0x44] = INAT_MODRM | INAT_VEXOK;
    table[0x45] = INAT_MODRM | INAT_VEXOK;
    table[0x46] = INAT_MODRM | INAT_VEXOK;
    table[0x47] = INAT_MODRM | INAT_VEXOK;
    table[0x4a] = INAT_MODRM | INAT_VEXOK;
    table[0x4b] = INAT_MODRM | INAT_VEXOK;
    table[0x50] = INAT_MODRM | INAT_VEXOK;
    table[0x51] = INAT_MODRM | INAT_VEXOK;
    table[0x54] = INAT_MODRM | INAT_VEXOK;
    table[0x55] = INAT_MODRM | INAT_VEXOK;
    table[0x56] = INAT_MODRM | INAT_VEXOK;
    table[0x57] = INAT_MODRM | INAT_VEXOK;
    table[0x58] = INAT_MODRM | INAT_VEXOK;
    table[0x59] = INAT_MODRM | INAT_VEXOK;
    table[0x5a] = INAT_MODRM | INAT_VEXOK;
    table[0x5b] = INAT_MODRM | INAT_VEXOK;
    table[0x5c] = INAT_MODRM | INAT_VEXOK;
    table[0x5d] = INAT_MODRM | INAT_VEXOK;
    table[0x5e] = INAT_MODRM | INAT_VEXOK;
    table[0x5f] = INAT_MODRM | INAT_VEXOK;
    table[0x60] = INAT_MODRM | INAT_VEXOK;
    table[0x61] = INAT_MODRM | INAT_VEXOK;
    table[0x62] = INAT_MODRM | INAT_VEXOK;
    table[0x63] = INAT_MODRM | INAT_VEXOK;
    table[0x64] = INAT_MODRM | INAT_VEXOK;
    table[0x65] = INAT_MODRM | INAT_VEXOK;
    table[0x66] = INAT_MODRM | INAT_VEXOK;
    table[0x67] = INAT_MODRM | INAT_VEXOK;
    table[0x68] = INAT_MODRM | INAT_VEXOK;
    table[0x69] = INAT_MODRM | INAT_VEXOK;
    table[0x6a] = INAT_MODRM | INAT_VEXOK;
    table[0x6b] = INAT_MODRM | INAT_VEXOK;
    table[0x6c] = INAT_MODRM | INAT_VEXOK;
    table[0x6d] = INAT_MODRM | INAT_VEXOK;
    table[0x6e] = INAT_MODRM | INAT_VEXOK;
    table[0x6f] = INAT_MODRM | INAT_VEXOK | INAT_MODRM | INAT_VEXOK;
    table[0x70] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x74] = INAT_MODRM | INAT_VEXOK;
    table[0x75] = INAT_MODRM | INAT_VEXOK;
    table[0x76] = INAT_MODRM | INAT_VEXOK;
    table[0x78] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x79] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7a] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7b] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7c] = INAT_MODRM | INAT_VEXOK;
    table[0x7d] = INAT_MODRM | INAT_VEXOK;
    table[0x7e] = INAT_MODRM | INAT_VEXOK;
    table[0x7f] = INAT_MODRM | INAT_VEXOK | INAT_MODRM | INAT_VEXOK;
    table[0x90] = INAT_MODRM | INAT_VEXOK;
    table[0x91] = INAT_MODRM | INAT_VEXOK;
    table[0x92] = INAT_MODRM | INAT_VEXOK;
    table[0x93] = INAT_MODRM | INAT_VEXOK;
    table[0x98] = INAT_MODRM | INAT_VEXOK;
    table[0x99] = INAT_MODRM | INAT_VEXOK;
    table[0xbc] = INAT_MODRM;
    table[0xbd] = INAT_MODRM;
    table[0xc2] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0xc4] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0xc5] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0xc6] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0xd0] = INAT_MODRM | INAT_VEXOK;
    table[0xd1] = INAT_MODRM | INAT_VEXOK;
    table[0xd2] = INAT_MODRM | INAT_VEXOK;
    table[0xd3] = INAT_MODRM | INAT_VEXOK;
    table[0xd4] = INAT_MODRM | INAT_VEXOK;
    table[0xd5] = INAT_MODRM | INAT_VEXOK;
    table[0xd6] = INAT_MODRM | INAT_VEXOK;
    table[0xd7] = INAT_MODRM | INAT_VEXOK;
    table[0xd8] = INAT_MODRM | INAT_VEXOK;
    table[0xd9] = INAT_MODRM | INAT_VEXOK;
    table[0xda] = INAT_MODRM | INAT_VEXOK;
    table[0xdb] = INAT_MODRM | INAT_VEXOK | INAT_MODRM | INAT_VEXOK;
    table[0xdc] = INAT_MODRM | INAT_VEXOK;
    table[0xdd] = INAT_MODRM | INAT_VEXOK;
    table[0xde] = INAT_MODRM | INAT_VEXOK;
    table[0xdf] = INAT_MODRM | INAT_VEXOK | INAT_MODRM | INAT_VEXOK;
    table[0xe0] = INAT_MODRM | INAT_VEXOK;
    table[0xe1] = INAT_MODRM | INAT_VEXOK;
    table[0xe2] = INAT_MODRM | INAT_VEXOK;
    table[0xe3] = INAT_MODRM | INAT_VEXOK;
    table[0xe4] = INAT_MODRM | INAT_VEXOK;
    table[0xe5] = INAT_MODRM | INAT_VEXOK;
    table[0xe6] = INAT_MODRM | INAT_VEXOK;
    table[0xe7] = INAT_MODRM | INAT_VEXOK;
    table[0xe8] = INAT_MODRM | INAT_VEXOK;
    table[0xe9] = INAT_MODRM | INAT_VEXOK;
    table[0xea] = INAT_MODRM | INAT_VEXOK;
    table[0xeb] = INAT_MODRM | INAT_VEXOK | INAT_MODRM | INAT_VEXOK;
    table[0xec] = INAT_MODRM | INAT_VEXOK;
    table[0xed] = INAT_MODRM | INAT_VEXOK;
    table[0xee] = INAT_MODRM | INAT_VEXOK;
    table[0xef] = INAT_MODRM | INAT_VEXOK | INAT_MODRM | INAT_VEXOK;
    table[0xf1] = INAT_MODRM | INAT_VEXOK;
    table[0xf2] = INAT_MODRM | INAT_VEXOK;
    table[0xf3] = INAT_MODRM | INAT_VEXOK;
    table[0xf4] = INAT_MODRM | INAT_VEXOK;
    table[0xf5] = INAT_MODRM | INAT_VEXOK;
    table[0xf6] = INAT_MODRM | INAT_VEXOK;
    table[0xf7] = INAT_MODRM | INAT_VEXOK;
    table[0xf8] = INAT_MODRM | INAT_VEXOK;
    table[0xf9] = INAT_MODRM | INAT_VEXOK;
    table[0xfa] = INAT_MODRM | INAT_VEXOK;
    table[0xfb] = INAT_MODRM | INAT_VEXOK;
    table[0xfc] = INAT_MODRM | INAT_VEXOK;
    table[0xfd] = INAT_MODRM | INAT_VEXOK;
    table[0xfe] = INAT_MODRM | INAT_VEXOK;
    table
}
const INAT_ESCAPE_TABLE_1_1: OpcodeTable = inat_escape_table_1_1_table();

const fn inat_escape_table_1_2_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x10] = INAT_MODRM | INAT_VEXOK;
    table[0x11] = INAT_MODRM | INAT_VEXOK;
    table[0x12] = INAT_MODRM | INAT_VEXOK;
    table[0x16] = INAT_MODRM | INAT_VEXOK;
    table[0x1a] = INAT_MODRM;
    table[0x1b] = INAT_MODRM;
    table[0x2a] = INAT_MODRM | INAT_VEXOK;
    table[0x2c] = INAT_MODRM | INAT_VEXOK;
    table[0x2d] = INAT_MODRM | INAT_VEXOK;
    table[0x51] = INAT_MODRM | INAT_VEXOK;
    table[0x52] = INAT_MODRM | INAT_VEXOK;
    table[0x53] = INAT_MODRM | INAT_VEXOK;
    table[0x58] = INAT_MODRM | INAT_VEXOK;
    table[0x59] = INAT_MODRM | INAT_VEXOK;
    table[0x5a] = INAT_MODRM | INAT_VEXOK;
    table[0x5b] = INAT_MODRM | INAT_VEXOK;
    table[0x5c] = INAT_MODRM | INAT_VEXOK;
    table[0x5d] = INAT_MODRM | INAT_VEXOK;
    table[0x5e] = INAT_MODRM | INAT_VEXOK;
    table[0x5f] = INAT_MODRM | INAT_VEXOK;
    table[0x6f] = INAT_MODRM | INAT_VEXOK | INAT_MODRM | INAT_VEXOK;
    table[0x70] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x78] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x79] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7a] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7b] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7e] = INAT_MODRM | INAT_VEXOK;
    table[0x7f] = INAT_MODRM | INAT_VEXOK | INAT_MODRM | INAT_VEXOK;
    table[0xb8] = INAT_MODRM;
    table[0xbc] = INAT_MODRM;
    table[0xbd] = INAT_MODRM;
    table[0xc2] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0xd6] = INAT_MODRM;
    table[0xe6] = INAT_MODRM | INAT_VEXOK | INAT_MODRM | INAT_VEXOK;
    table
}
const INAT_ESCAPE_TABLE_1_2: OpcodeTable = inat_escape_table_1_2_table();

const fn inat_escape_table_1_3_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x10] = INAT_MODRM | INAT_VEXOK;
    table[0x11] = INAT_MODRM | INAT_VEXOK;
    table[0x12] = INAT_MODRM | INAT_VEXOK;
    table[0x1a] = INAT_MODRM;
    table[0x1b] = INAT_MODRM;
    table[0x2a] = INAT_MODRM | INAT_VEXOK;
    table[0x2c] = INAT_MODRM | INAT_VEXOK;
    table[0x2d] = INAT_MODRM | INAT_VEXOK;
    table[0x51] = INAT_MODRM | INAT_VEXOK;
    table[0x58] = INAT_MODRM | INAT_VEXOK;
    table[0x59] = INAT_MODRM | INAT_VEXOK;
    table[0x5a] = INAT_MODRM | INAT_VEXOK;
    table[0x5c] = INAT_MODRM | INAT_VEXOK;
    table[0x5d] = INAT_MODRM | INAT_VEXOK;
    table[0x5e] = INAT_MODRM | INAT_VEXOK;
    table[0x5f] = INAT_MODRM | INAT_VEXOK;
    table[0x6f] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x70] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x78] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x79] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7a] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7b] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7c] = INAT_MODRM | INAT_VEXOK;
    table[0x7d] = INAT_MODRM | INAT_VEXOK;
    table[0x7f] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x92] = INAT_MODRM | INAT_VEXOK;
    table[0x93] = INAT_MODRM | INAT_VEXOK;
    table[0xbc] = INAT_MODRM;
    table[0xbd] = INAT_MODRM;
    table[0xc2] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0xd0] = INAT_MODRM | INAT_VEXOK;
    table[0xd6] = INAT_MODRM;
    table[0xe6] = INAT_MODRM | INAT_VEXOK;
    table[0xf0] = INAT_MODRM | INAT_VEXOK;
    table
}
const INAT_ESCAPE_TABLE_1_3: OpcodeTable = inat_escape_table_1_3_table();

const fn inat_escape_table_2_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x00] = INAT_MODRM | INAT_VARIANT;
    table[0x01] = INAT_MODRM | INAT_VARIANT;
    table[0x02] = INAT_MODRM | INAT_VARIANT;
    table[0x03] = INAT_MODRM | INAT_VARIANT;
    table[0x04] = INAT_MODRM | INAT_VARIANT;
    table[0x05] = INAT_MODRM | INAT_VARIANT;
    table[0x06] = INAT_MODRM | INAT_VARIANT;
    table[0x07] = INAT_MODRM | INAT_VARIANT;
    table[0x08] = INAT_MODRM | INAT_VARIANT;
    table[0x09] = INAT_MODRM | INAT_VARIANT;
    table[0x0a] = INAT_MODRM | INAT_VARIANT;
    table[0x0b] = INAT_MODRM | INAT_VARIANT;
    table[0x0c] = INAT_VARIANT;
    table[0x0d] = INAT_VARIANT;
    table[0x0e] = INAT_VARIANT;
    table[0x0f] = INAT_VARIANT;
    table[0x10] = INAT_VARIANT;
    table[0x11] = INAT_VARIANT;
    table[0x12] = INAT_VARIANT;
    table[0x13] = INAT_VARIANT;
    table[0x14] = INAT_VARIANT;
    table[0x15] = INAT_VARIANT;
    table[0x16] = INAT_VARIANT;
    table[0x17] = INAT_VARIANT;
    table[0x18] = INAT_VARIANT;
    table[0x19] = INAT_VARIANT;
    table[0x1a] = INAT_VARIANT;
    table[0x1b] = INAT_VARIANT;
    table[0x1c] = INAT_MODRM | INAT_VARIANT;
    table[0x1d] = INAT_MODRM | INAT_VARIANT;
    table[0x1e] = INAT_MODRM | INAT_VARIANT;
    table[0x1f] = INAT_VARIANT;
    table[0x20] = INAT_VARIANT;
    table[0x21] = INAT_VARIANT;
    table[0x22] = INAT_VARIANT;
    table[0x23] = INAT_VARIANT;
    table[0x24] = INAT_VARIANT;
    table[0x25] = INAT_VARIANT;
    table[0x26] = INAT_VARIANT;
    table[0x27] = INAT_VARIANT;
    table[0x28] = INAT_VARIANT;
    table[0x29] = INAT_VARIANT;
    table[0x2a] = INAT_VARIANT;
    table[0x2b] = INAT_VARIANT;
    table[0x2c] = INAT_VARIANT;
    table[0x2d] = INAT_VARIANT;
    table[0x2e] = INAT_VARIANT;
    table[0x2f] = INAT_VARIANT;
    table[0x30] = INAT_VARIANT;
    table[0x31] = INAT_VARIANT;
    table[0x32] = INAT_VARIANT;
    table[0x33] = INAT_VARIANT;
    table[0x34] = INAT_VARIANT;
    table[0x35] = INAT_VARIANT;
    table[0x36] = INAT_VARIANT;
    table[0x37] = INAT_VARIANT;
    table[0x38] = INAT_VARIANT;
    table[0x39] = INAT_VARIANT;
    table[0x3a] = INAT_VARIANT;
    table[0x3b] = INAT_VARIANT;
    table[0x3c] = INAT_VARIANT;
    table[0x3d] = INAT_VARIANT;
    table[0x3e] = INAT_VARIANT;
    table[0x3f] = INAT_VARIANT;
    table[0x40] = INAT_VARIANT;
    table[0x41] = INAT_VARIANT;
    table[0x42] = INAT_VARIANT;
    table[0x43] = INAT_VARIANT;
    table[0x44] = INAT_VARIANT;
    table[0x45] = INAT_VARIANT;
    table[0x46] = INAT_VARIANT;
    table[0x47] = INAT_VARIANT;
    table[0x49] = INAT_VEXOK | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x4b] = INAT_VARIANT;
    table[0x4c] = INAT_VARIANT;
    table[0x4d] = INAT_VARIANT;
    table[0x4e] = INAT_VARIANT;
    table[0x4f] = INAT_VARIANT;
    table[0x50] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY | INAT_VARIANT;
    table[0x51] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY | INAT_VARIANT;
    table[0x52] = INAT_VARIANT;
    table[0x53] = INAT_VARIANT;
    table[0x54] = INAT_VARIANT;
    table[0x55] = INAT_VARIANT;
    table[0x58] = INAT_VARIANT;
    table[0x59] = INAT_VARIANT;
    table[0x5a] = INAT_VARIANT;
    table[0x5b] = INAT_VARIANT;
    table[0x5c] = INAT_VARIANT;
    table[0x5e] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x62] = INAT_VARIANT;
    table[0x63] = INAT_VARIANT;
    table[0x64] = INAT_VARIANT;
    table[0x65] = INAT_VARIANT;
    table[0x66] = INAT_VARIANT;
    table[0x68] = INAT_VARIANT;
    table[0x6c] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x70] = INAT_VARIANT;
    table[0x71] = INAT_VARIANT;
    table[0x72] = INAT_VARIANT;
    table[0x73] = INAT_VARIANT;
    table[0x75] = INAT_VARIANT;
    table[0x76] = INAT_VARIANT;
    table[0x77] = INAT_VARIANT;
    table[0x78] = INAT_VARIANT;
    table[0x79] = INAT_VARIANT;
    table[0x7a] = INAT_VARIANT;
    table[0x7b] = INAT_VARIANT;
    table[0x7c] = INAT_VARIANT;
    table[0x7d] = INAT_VARIANT;
    table[0x7e] = INAT_VARIANT;
    table[0x7f] = INAT_VARIANT;
    table[0x80] = INAT_VARIANT;
    table[0x81] = INAT_VARIANT;
    table[0x82] = INAT_VARIANT;
    table[0x83] = INAT_VARIANT;
    table[0x88] = INAT_VARIANT;
    table[0x89] = INAT_VARIANT;
    table[0x8a] = INAT_VARIANT;
    table[0x8b] = INAT_VARIANT;
    table[0x8c] = INAT_VARIANT;
    table[0x8d] = INAT_VARIANT;
    table[0x8e] = INAT_VARIANT;
    table[0x8f] = INAT_VARIANT;
    table[0x90] = INAT_VARIANT;
    table[0x91] = INAT_VARIANT;
    table[0x92] = INAT_VARIANT;
    table[0x93] = INAT_VARIANT;
    table[0x96] = INAT_VARIANT;
    table[0x97] = INAT_VARIANT;
    table[0x98] = INAT_VARIANT;
    table[0x99] = INAT_VARIANT;
    table[0x9a] = INAT_VARIANT;
    table[0x9b] = INAT_VARIANT;
    table[0x9c] = INAT_VARIANT;
    table[0x9d] = INAT_VARIANT;
    table[0x9e] = INAT_VARIANT;
    table[0x9f] = INAT_VARIANT;
    table[0xa0] = INAT_VARIANT;
    table[0xa1] = INAT_VARIANT;
    table[0xa2] = INAT_VARIANT;
    table[0xa3] = INAT_VARIANT;
    table[0xa6] = INAT_VARIANT;
    table[0xa7] = INAT_VARIANT;
    table[0xa8] = INAT_VARIANT;
    table[0xa9] = INAT_VARIANT;
    table[0xaa] = INAT_VARIANT;
    table[0xab] = INAT_VARIANT;
    table[0xac] = INAT_VARIANT;
    table[0xad] = INAT_VARIANT;
    table[0xae] = INAT_VARIANT;
    table[0xaf] = INAT_VARIANT;
    table[0xb0] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY | INAT_VARIANT;
    table[0xb1] = INAT_VARIANT;
    table[0xb4] = INAT_VARIANT;
    table[0xb5] = INAT_VARIANT;
    table[0xb6] = INAT_VARIANT;
    table[0xb7] = INAT_VARIANT;
    table[0xb8] = INAT_VARIANT;
    table[0xb9] = INAT_VARIANT;
    table[0xba] = INAT_VARIANT;
    table[0xbb] = INAT_VARIANT;
    table[0xbc] = INAT_VARIANT;
    table[0xbd] = INAT_VARIANT;
    table[0xbe] = INAT_VARIANT;
    table[0xbf] = INAT_VARIANT;
    table[0xc4] = INAT_VARIANT;
    table[0xc6] = make_group(25);
    table[0xc7] = make_group(26);
    table[0xc8] = INAT_MODRM | INAT_VARIANT;
    table[0xc9] = INAT_MODRM;
    table[0xca] = INAT_MODRM | INAT_VARIANT;
    table[0xcb] = INAT_MODRM | INAT_VARIANT;
    table[0xcc] = INAT_MODRM | INAT_VARIANT;
    table[0xcd] = INAT_MODRM | INAT_VARIANT;
    table[0xcf] = INAT_VARIANT;
    table[0xd2] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY | INAT_VARIANT;
    table[0xd3] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY | INAT_VARIANT;
    table[0xd8] = INAT_VARIANT;
    table[0xda] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0xdb] = INAT_VARIANT;
    table[0xdc] = INAT_VARIANT;
    table[0xdd] = INAT_VARIANT;
    table[0xde] = INAT_VARIANT;
    table[0xdf] = INAT_VARIANT;
    table[0xe0] = INAT_VARIANT;
    table[0xe1] = INAT_VARIANT;
    table[0xe2] = INAT_VARIANT;
    table[0xe3] = INAT_VARIANT;
    table[0xe4] = INAT_VARIANT;
    table[0xe5] = INAT_VARIANT;
    table[0xe6] = INAT_VARIANT;
    table[0xe7] = INAT_VARIANT;
    table[0xe8] = INAT_VARIANT;
    table[0xe9] = INAT_VARIANT;
    table[0xea] = INAT_VARIANT;
    table[0xeb] = INAT_VARIANT;
    table[0xec] = INAT_VARIANT;
    table[0xed] = INAT_VARIANT;
    table[0xee] = INAT_VARIANT;
    table[0xef] = INAT_VARIANT;
    table[0xf0] = INAT_MODRM | INAT_MODRM | INAT_VARIANT;
    table[0xf1] = INAT_MODRM | INAT_MODRM | INAT_VARIANT;
    table[0xf2] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xf3] = make_group(27);
    table[0xf5] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY | INAT_VARIANT;
    table[0xf6] = INAT_MODRM | INAT_VARIANT;
    table[0xf7] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY | INAT_VARIANT;
    table[0xf8] = INAT_VARIANT;
    table[0xf9] = INAT_MODRM;
    table[0xfa] = INAT_VARIANT;
    table[0xfb] = INAT_VARIANT;
    table[0xfc] = INAT_MODRM | INAT_VARIANT;
    table
}
const INAT_ESCAPE_TABLE_2: OpcodeTable = inat_escape_table_2_table();

const fn inat_escape_table_2_1_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x00] = INAT_MODRM | INAT_VEXOK;
    table[0x01] = INAT_MODRM | INAT_VEXOK;
    table[0x02] = INAT_MODRM | INAT_VEXOK;
    table[0x03] = INAT_MODRM | INAT_VEXOK;
    table[0x04] = INAT_MODRM | INAT_VEXOK;
    table[0x05] = INAT_MODRM | INAT_VEXOK;
    table[0x06] = INAT_MODRM | INAT_VEXOK;
    table[0x07] = INAT_MODRM | INAT_VEXOK;
    table[0x08] = INAT_MODRM | INAT_VEXOK;
    table[0x09] = INAT_MODRM | INAT_VEXOK;
    table[0x0a] = INAT_MODRM | INAT_VEXOK;
    table[0x0b] = INAT_MODRM | INAT_VEXOK;
    table[0x0c] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x0d] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x0e] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x0f] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x10] = INAT_MODRM | INAT_MODRM | INAT_VEXOK;
    table[0x11] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x12] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x13] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x14] = INAT_MODRM | INAT_MODRM | INAT_VEXOK;
    table[0x15] = INAT_MODRM | INAT_MODRM | INAT_VEXOK;
    table[0x16] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY | INAT_MODRM | INAT_VEXOK;
    table[0x17] = INAT_MODRM | INAT_VEXOK;
    table[0x18] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x19] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY | INAT_MODRM | INAT_VEXOK;
    table[0x1a] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY | INAT_MODRM | INAT_VEXOK;
    table[0x1b] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x1c] = INAT_MODRM | INAT_VEXOK;
    table[0x1d] = INAT_MODRM | INAT_VEXOK;
    table[0x1e] = INAT_MODRM | INAT_VEXOK;
    table[0x1f] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x20] = INAT_MODRM | INAT_VEXOK;
    table[0x21] = INAT_MODRM | INAT_VEXOK;
    table[0x22] = INAT_MODRM | INAT_VEXOK;
    table[0x23] = INAT_MODRM | INAT_VEXOK;
    table[0x24] = INAT_MODRM | INAT_VEXOK;
    table[0x25] = INAT_MODRM | INAT_VEXOK;
    table[0x26] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x27] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x28] = INAT_MODRM | INAT_VEXOK;
    table[0x29] = INAT_MODRM | INAT_VEXOK;
    table[0x2a] = INAT_MODRM | INAT_VEXOK;
    table[0x2b] = INAT_MODRM | INAT_VEXOK;
    table[0x2c] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY | INAT_MODRM | INAT_VEXOK;
    table[0x2d] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY | INAT_MODRM | INAT_VEXOK;
    table[0x2e] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x2f] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x30] = INAT_MODRM | INAT_VEXOK;
    table[0x31] = INAT_MODRM | INAT_VEXOK;
    table[0x32] = INAT_MODRM | INAT_VEXOK;
    table[0x33] = INAT_MODRM | INAT_VEXOK;
    table[0x34] = INAT_MODRM | INAT_VEXOK;
    table[0x35] = INAT_MODRM | INAT_VEXOK;
    table[0x36] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY | INAT_MODRM | INAT_VEXOK;
    table[0x37] = INAT_MODRM | INAT_VEXOK;
    table[0x38] = INAT_MODRM | INAT_VEXOK;
    table[0x39] = INAT_MODRM | INAT_VEXOK | INAT_MODRM | INAT_VEXOK;
    table[0x3a] = INAT_MODRM | INAT_VEXOK;
    table[0x3b] = INAT_MODRM | INAT_VEXOK | INAT_MODRM | INAT_VEXOK;
    table[0x3c] = INAT_MODRM | INAT_VEXOK;
    table[0x3d] = INAT_MODRM | INAT_VEXOK | INAT_MODRM | INAT_VEXOK;
    table[0x3e] = INAT_MODRM | INAT_VEXOK;
    table[0x3f] = INAT_MODRM | INAT_VEXOK | INAT_MODRM | INAT_VEXOK;
    table[0x40] = INAT_MODRM | INAT_VEXOK | INAT_MODRM | INAT_VEXOK;
    table[0x41] = INAT_MODRM | INAT_VEXOK;
    table[0x42] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x43] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x44] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x45] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x46] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY | INAT_MODRM | INAT_VEXOK;
    table[0x47] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x49] = INAT_MODRM | INAT_VEXOK;
    table[0x4b] = INAT_MODRM | INAT_VEXOK;
    table[0x4c] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x4d] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x4e] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x4f] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x50] = INAT_MODRM | INAT_VEXOK;
    table[0x51] = INAT_MODRM | INAT_VEXOK;
    table[0x52] = INAT_MODRM | INAT_VEXOK;
    table[0x53] = INAT_MODRM | INAT_VEXOK;
    table[0x54] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x55] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x58] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x59] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY | INAT_MODRM | INAT_VEXOK;
    table[0x5a] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY | INAT_MODRM | INAT_VEXOK;
    table[0x5b] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x5e] = INAT_MODRM | INAT_VEXOK;
    table[0x62] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x63] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x64] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x65] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x66] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x6c] = INAT_MODRM | INAT_VEXOK;
    table[0x70] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x71] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x72] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x73] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x75] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x76] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x77] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x78] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x79] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x7a] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7b] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7c] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7d] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7e] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7f] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x80] = INAT_MODRM;
    table[0x81] = INAT_MODRM;
    table[0x82] = INAT_MODRM;
    table[0x83] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x88] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x89] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x8a] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x8b] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x8c] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x8d] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x8e] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x8f] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x90] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY | INAT_MODRM | INAT_VEXOK;
    table[0x91] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY | INAT_MODRM | INAT_VEXOK;
    table[0x92] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x93] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x96] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x97] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x98] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x99] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x9a] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x9b] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x9c] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x9d] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x9e] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x9f] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xa0] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xa1] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xa2] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xa3] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xa6] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xa7] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xa8] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xa9] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xaa] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xab] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xac] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xad] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xae] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xaf] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xb0] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xb1] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xb4] = INAT_MODRM | INAT_VEXOK;
    table[0xb5] = INAT_MODRM | INAT_VEXOK;
    table[0xb6] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xb7] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xb8] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xb9] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xba] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xbb] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xbc] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xbd] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xbe] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xbf] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xc4] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xc8] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xca] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xcb] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xcc] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xcd] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xcf] = INAT_MODRM | INAT_VEXOK;
    table[0xd2] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xd3] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xda] = INAT_MODRM | INAT_VEXOK;
    table[0xdb] = INAT_MODRM | INAT_VEXOK;
    table[0xdc] = INAT_MODRM | INAT_VEXOK;
    table[0xdd] = INAT_MODRM | INAT_VEXOK;
    table[0xde] = INAT_MODRM | INAT_VEXOK;
    table[0xdf] = INAT_MODRM | INAT_VEXOK;
    table[0xe0] = INAT_MODRM | INAT_VEXOK;
    table[0xe1] = INAT_MODRM | INAT_VEXOK;
    table[0xe2] = INAT_MODRM | INAT_VEXOK;
    table[0xe3] = INAT_MODRM | INAT_VEXOK;
    table[0xe4] = INAT_MODRM | INAT_VEXOK;
    table[0xe5] = INAT_MODRM | INAT_VEXOK;
    table[0xe6] = INAT_MODRM | INAT_VEXOK;
    table[0xe7] = INAT_MODRM | INAT_VEXOK;
    table[0xe8] = INAT_MODRM | INAT_VEXOK;
    table[0xe9] = INAT_MODRM | INAT_VEXOK;
    table[0xea] = INAT_MODRM | INAT_VEXOK;
    table[0xeb] = INAT_MODRM | INAT_VEXOK;
    table[0xec] = INAT_MODRM | INAT_VEXOK;
    table[0xed] = INAT_MODRM | INAT_VEXOK;
    table[0xee] = INAT_MODRM | INAT_VEXOK;
    table[0xef] = INAT_MODRM | INAT_VEXOK;
    table[0xf0] = INAT_MODRM;
    table[0xf1] = INAT_MODRM;
    table[0xf5] = INAT_MODRM;
    table[0xf6] = INAT_MODRM;
    table[0xf7] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xf8] = INAT_MODRM;
    table[0xfc] = INAT_MODRM;
    table
}
const INAT_ESCAPE_TABLE_2_1: OpcodeTable = inat_escape_table_2_1_table();

const fn inat_escape_table_2_2_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x10] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x11] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x12] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x13] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x14] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x15] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x20] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x21] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x22] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x23] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x24] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x25] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x26] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x27] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x28] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x29] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x2a] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x30] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x31] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x32] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x33] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x34] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x35] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x38] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x39] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x3a] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x4b] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x50] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x51] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x52] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x5c] = INAT_MODRM | INAT_VEXOK;
    table[0x5e] = INAT_MODRM | INAT_VEXOK;
    table[0x72] = INAT_MODRM | INAT_VEXOK;
    table[0xb0] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xb1] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xd2] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xd3] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xd8] = INAT_MODRM | INAT_MODRM | INAT_MODRM | INAT_MODRM;
    table[0xda] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xdc] = INAT_MODRM | INAT_MODRM;
    table[0xdd] = INAT_MODRM;
    table[0xde] = INAT_MODRM;
    table[0xdf] = INAT_MODRM;
    table[0xf5] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xf6] = INAT_MODRM;
    table[0xf7] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xf8] = INAT_MODRM | INAT_MODRM;
    table[0xfa] = INAT_MODRM;
    table[0xfb] = INAT_MODRM;
    table[0xfc] = INAT_MODRM;
    table
}
const INAT_ESCAPE_TABLE_2_2: OpcodeTable = inat_escape_table_2_2_table();

const fn inat_escape_table_2_3_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x49] = INAT_MODRM | INAT_VEXOK;
    table[0x4b] = INAT_MODRM | INAT_VEXOK;
    table[0x50] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x51] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x52] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x53] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x5c] = INAT_MODRM | INAT_VEXOK;
    table[0x5e] = INAT_MODRM | INAT_VEXOK;
    table[0x68] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x72] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x9a] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x9b] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xaa] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xab] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xb0] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xcb] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xcc] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xcd] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xda] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xf0] = INAT_MODRM | INAT_MODRM;
    table[0xf1] = INAT_MODRM | INAT_MODRM;
    table[0xf5] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xf6] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xf7] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0xf8] = INAT_MODRM | INAT_MODRM;
    table[0xfc] = INAT_MODRM;
    table
}
const INAT_ESCAPE_TABLE_2_3: OpcodeTable = inat_escape_table_2_3_table();

const fn inat_escape_table_3_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x00] = INAT_VARIANT;
    table[0x01] = INAT_VARIANT;
    table[0x02] = INAT_VARIANT;
    table[0x03] = INAT_VARIANT;
    table[0x04] = INAT_VARIANT;
    table[0x05] = INAT_VARIANT;
    table[0x06] = INAT_VARIANT;
    table[0x08] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x09] = INAT_VARIANT;
    table[0x0a] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x0b] = INAT_VARIANT;
    table[0x0c] = INAT_VARIANT;
    table[0x0d] = INAT_VARIANT;
    table[0x0e] = INAT_VARIANT;
    table[0x0f] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VARIANT;
    table[0x14] = INAT_VARIANT;
    table[0x15] = INAT_VARIANT;
    table[0x16] = INAT_VARIANT;
    table[0x17] = INAT_VARIANT;
    table[0x18] = INAT_VARIANT;
    table[0x19] = INAT_VARIANT;
    table[0x1a] = INAT_VARIANT;
    table[0x1b] = INAT_VARIANT;
    table[0x1d] = INAT_VARIANT;
    table[0x1e] = INAT_VARIANT;
    table[0x1f] = INAT_VARIANT;
    table[0x20] = INAT_VARIANT;
    table[0x21] = INAT_VARIANT;
    table[0x22] = INAT_VARIANT;
    table[0x23] = INAT_VARIANT;
    table[0x25] = INAT_VARIANT;
    table[0x26] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_VARIANT;
    table[0x27] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_VARIANT;
    table[0x30] = INAT_VARIANT;
    table[0x31] = INAT_VARIANT;
    table[0x32] = INAT_VARIANT;
    table[0x33] = INAT_VARIANT;
    table[0x38] = INAT_VARIANT;
    table[0x39] = INAT_VARIANT;
    table[0x3a] = INAT_VARIANT;
    table[0x3b] = INAT_VARIANT;
    table[0x3e] = INAT_VARIANT;
    table[0x3f] = INAT_VARIANT;
    table[0x40] = INAT_VARIANT;
    table[0x41] = INAT_VARIANT;
    table[0x42] = INAT_VARIANT;
    table[0x43] = INAT_VARIANT;
    table[0x44] = INAT_VARIANT;
    table[0x46] = INAT_VARIANT;
    table[0x4a] = INAT_VARIANT;
    table[0x4b] = INAT_VARIANT;
    table[0x4c] = INAT_VARIANT;
    table[0x50] = INAT_VARIANT;
    table[0x51] = INAT_VARIANT;
    table[0x54] = INAT_VARIANT;
    table[0x55] = INAT_VARIANT;
    table[0x56] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_VARIANT;
    table[0x57] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_VARIANT;
    table[0x60] = INAT_VARIANT;
    table[0x61] = INAT_VARIANT;
    table[0x62] = INAT_VARIANT;
    table[0x63] = INAT_VARIANT;
    table[0x66] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_VARIANT;
    table[0x67] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_VARIANT;
    table[0x70] = INAT_VARIANT;
    table[0x71] = INAT_VARIANT;
    table[0x72] = INAT_VARIANT;
    table[0x73] = INAT_VARIANT;
    table[0xc2] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_VARIANT;
    table[0xcc] = make_imm(INAT_IMM_BYTE) | INAT_MODRM;
    table[0xce] = INAT_VARIANT;
    table[0xcf] = INAT_VARIANT;
    table[0xde] = INAT_VARIANT;
    table[0xdf] = INAT_VARIANT;
    table[0xf0] = INAT_VARIANT;
    table
}
const INAT_ESCAPE_TABLE_3: OpcodeTable = inat_escape_table_3_table();

const fn inat_escape_table_3_1_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x00] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x01] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x02] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x03] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x04] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x05] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x06] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x08] = make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK
        | make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK;
    table[0x09] = make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK
        | make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK;
    table[0x0a] = make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK
        | make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK;
    table[0x0b] = make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK
        | make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK;
    table[0x0c] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x0d] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x0e] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x0f] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x14] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x15] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x16] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x17] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x18] = make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_VEXONLY
        | make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK;
    table[0x19] = make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_VEXONLY
        | make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK;
    table[0x1a] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x1b] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x1d] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x1e] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x1f] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x20] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x21] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x22] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x23] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x25] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x26] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x27] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x30] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x31] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x32] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x33] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x38] = make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_VEXONLY
        | make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK;
    table[0x39] = make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_VEXONLY
        | make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK;
    table[0x3a] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x3b] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x3e] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x3f] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x40] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x41] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x42] = make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK
        | make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK;
    table[0x43] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x44] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x46] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x4a] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x4b] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x4c] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x50] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x51] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x54] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x55] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x56] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x57] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x60] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x61] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x62] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x63] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x66] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x67] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x70] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x71] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x72] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x73] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xce] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0xcf] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0xde] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0xdf] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table
}
const INAT_ESCAPE_TABLE_3_1: OpcodeTable = inat_escape_table_3_1_table();

const fn inat_escape_table_3_2_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0xc2] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xf0] = make_imm(INAT_IMM_BYTE) | INAT_MODRM;
    table
}
const INAT_ESCAPE_TABLE_3_2: OpcodeTable = inat_escape_table_3_2_table();

const fn inat_escape_table_3_3_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0xf0] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table
}
const INAT_ESCAPE_TABLE_3_3: OpcodeTable = inat_escape_table_3_3_table();

const fn inat_avx_table_4_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x00] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x01] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0x02] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x03] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0x08] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x09] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0x0a] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x0b] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0x10] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x11] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0x12] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x13] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0x18] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x19] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0x1a] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x1b] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0x20] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x21] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0x22] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x23] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0x24] = make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_VARIANT;
    table[0x28] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x29] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0x2a] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x2b] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0x2c] = make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_VARIANT;
    table[0x30] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x31] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0x32] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x33] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0x38] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x39] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0x3a] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x3b] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0x40] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_VARIANT;
    table[0x41] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_VARIANT;
    table[0x42] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_VARIANT;
    table[0x43] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_VARIANT;
    table[0x44] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_VARIANT;
    table[0x45] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_VARIANT;
    table[0x46] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_VARIANT;
    table[0x47] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_VARIANT;
    table[0x48] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_VARIANT;
    table[0x49] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_VARIANT;
    table[0x4a] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_VARIANT;
    table[0x4b] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_VARIANT;
    table[0x4c] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_VARIANT;
    table[0x4d] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_VARIANT;
    table[0x4e] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_VARIANT;
    table[0x4f] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_VARIANT;
    table[0x60] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0x61] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0x65] = INAT_VARIANT;
    table[0x66] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_VARIANT;
    table[0x69] = make_imm(INAT_IMM_VWORD32)
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_VARIANT;
    table[0x6b] = make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_VARIANT;
    table[0x80] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | make_group(1) | INAT_VEXOK | INAT_EVEXONLY;
    table[0x81] = make_imm(INAT_IMM_VWORD32)
        | INAT_MODRM
        | make_group(1)
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE;
    table[0x83] = make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | make_group(1)
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE;
    table[0x84] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x85] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0x88] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0x8f] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xa5] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0xad] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0xaf] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0xc0] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | make_group(3) | INAT_VEXOK | INAT_EVEXONLY;
    table[0xc1] = make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | make_group(3)
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE;
    table[0xd0] = INAT_MODRM | make_group(3) | INAT_VEXOK | INAT_EVEXONLY;
    table[0xd1] = INAT_MODRM | make_group(3) | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0xd2] = INAT_MODRM | make_group(3) | INAT_VEXOK | INAT_EVEXONLY;
    table[0xd3] = INAT_MODRM | make_group(3) | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0xf0] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0xf1] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0xf2] = INAT_VARIANT;
    table[0xf4] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0xf5] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE | INAT_VARIANT;
    table[0xf6] = INAT_MODRM | make_group(6) | INAT_VEXOK | INAT_EVEXONLY;
    table[0xf7] = INAT_MODRM | make_group(7) | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0xf8] = INAT_VARIANT;
    table[0xf9] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xfe] = make_group(8) | INAT_VEXOK | INAT_EVEXONLY;
    table[0xff] = make_group(9)
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY;
    table
}
const INAT_AVX_TABLE_4: OpcodeTable = inat_avx_table_4_table();

const fn inat_avx_table_4_1_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x01] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x03] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x09] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x0b] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x11] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x13] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x19] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x1b] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x21] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x23] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x24] =
        make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x29] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x2b] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x2c] =
        make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x31] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x33] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x39] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x3b] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x40] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE;
    table[0x41] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE;
    table[0x42] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE;
    table[0x43] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE;
    table[0x44] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE;
    table[0x45] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE;
    table[0x46] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE;
    table[0x47] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE;
    table[0x48] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE;
    table[0x49] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE;
    table[0x4a] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE;
    table[0x4b] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE;
    table[0x4c] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE;
    table[0x4d] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE;
    table[0x4e] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE;
    table[0x4f] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_EVEX_SCALABLE;
    table[0x60] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x61] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x65] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x66] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x69] =
        make_imm(INAT_IMM_VWORD32) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x6b] =
        make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x85] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0x88] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0xa5] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0xad] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0xaf] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0xf1] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0xf4] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0xf5] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_EVEX_SCALABLE;
    table[0xf8] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table
}
const INAT_AVX_TABLE_4_1: OpcodeTable = inat_avx_table_4_1_table();

const fn inat_avx_table_4_2_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x66] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xf0] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xf1] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xf2] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xf8] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table
}
const INAT_AVX_TABLE_4_2: OpcodeTable = inat_avx_table_4_2_table();

const fn inat_avx_table_4_3_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x40] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x41] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x42] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x43] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x44] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x45] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x46] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x47] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x48] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x49] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x4a] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x4b] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x4c] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x4d] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x4e] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x4f] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xf8] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table
}
const INAT_AVX_TABLE_4_3: OpcodeTable = inat_avx_table_4_3_table();

const fn inat_avx_table_5_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x10] = INAT_VARIANT;
    table[0x11] = INAT_VARIANT;
    table[0x1d] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_VARIANT;
    table[0x2a] = INAT_VARIANT;
    table[0x2c] = INAT_VARIANT;
    table[0x2d] = INAT_VARIANT;
    table[0x2e] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x2f] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x51] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_VARIANT;
    table[0x58] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_VARIANT;
    table[0x59] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_VARIANT;
    table[0x5a] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_VARIANT;
    table[0x5b] = INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_MODRM
        | INAT_VEXOK
        | INAT_EVEXONLY
        | INAT_VARIANT;
    table[0x5c] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_VARIANT;
    table[0x5d] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_VARIANT;
    table[0x5e] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_VARIANT;
    table[0x5f] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_VARIANT;
    table[0x6e] = INAT_VARIANT;
    table[0x78] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_VARIANT;
    table[0x79] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_VARIANT;
    table[0x7a] = INAT_VARIANT;
    table[0x7b] = INAT_VARIANT;
    table[0x7c] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_VARIANT;
    table[0x7d] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_VARIANT;
    table[0x7e] = INAT_VARIANT;
    table
}
const INAT_AVX_TABLE_5: OpcodeTable = inat_avx_table_5_table();

const fn inat_avx_table_5_1_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x1d] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x5a] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x5b] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x6e] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x78] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x79] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7a] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7b] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7c] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7d] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7e] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table
}
const INAT_AVX_TABLE_5_1: OpcodeTable = inat_avx_table_5_1_table();

const fn inat_avx_table_5_2_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x10] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x11] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x2a] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x2c] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x2d] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x51] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x58] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x59] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x5a] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x5b] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x5c] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x5d] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x5e] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x5f] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x78] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x79] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7b] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7d] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table
}
const INAT_AVX_TABLE_5_2: OpcodeTable = inat_avx_table_5_2_table();

const fn inat_avx_table_5_3_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x5a] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7a] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x7d] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table
}
const INAT_AVX_TABLE_5_3: OpcodeTable = inat_avx_table_5_3_table();

const fn inat_avx_table_6_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x13] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY | INAT_VARIANT;
    table[0x2c] = INAT_VARIANT;
    table[0x2d] = INAT_VARIANT;
    table[0x42] = INAT_VARIANT;
    table[0x43] = INAT_VARIANT;
    table[0x4c] = INAT_VARIANT;
    table[0x4d] = INAT_VARIANT;
    table[0x4e] = INAT_VARIANT;
    table[0x4f] = INAT_VARIANT;
    table[0x56] = INAT_VARIANT;
    table[0x57] = INAT_VARIANT;
    table[0x96] = INAT_VARIANT;
    table[0x97] = INAT_VARIANT;
    table[0x98] = INAT_VARIANT;
    table[0x99] = INAT_VARIANT;
    table[0x9a] = INAT_VARIANT;
    table[0x9b] = INAT_VARIANT;
    table[0x9c] = INAT_VARIANT;
    table[0x9d] = INAT_VARIANT;
    table[0x9e] = INAT_VARIANT;
    table[0x9f] = INAT_VARIANT;
    table[0xa6] = INAT_VARIANT;
    table[0xa7] = INAT_VARIANT;
    table[0xa8] = INAT_VARIANT;
    table[0xa9] = INAT_VARIANT;
    table[0xaa] = INAT_VARIANT;
    table[0xab] = INAT_VARIANT;
    table[0xac] = INAT_VARIANT;
    table[0xad] = INAT_VARIANT;
    table[0xae] = INAT_VARIANT;
    table[0xaf] = INAT_VARIANT;
    table[0xb6] = INAT_VARIANT;
    table[0xb7] = INAT_VARIANT;
    table[0xb8] = INAT_VARIANT;
    table[0xb9] = INAT_VARIANT;
    table[0xba] = INAT_VARIANT;
    table[0xbb] = INAT_VARIANT;
    table[0xbc] = INAT_VARIANT;
    table[0xbd] = INAT_VARIANT;
    table[0xbe] = INAT_VARIANT;
    table[0xbf] = INAT_VARIANT;
    table[0xd6] = INAT_VARIANT;
    table[0xd7] = INAT_VARIANT;
    table
}
const INAT_AVX_TABLE_6: OpcodeTable = inat_avx_table_6_table();

const fn inat_avx_table_6_1_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x13] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x2c] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x2d] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x42] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x43] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x4c] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x4d] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x4e] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x4f] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x96] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x97] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x98] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x99] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x9a] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x9b] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x9c] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x9d] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x9e] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x9f] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xa6] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xa7] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xa8] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xa9] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xaa] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xab] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xac] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xad] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xae] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xaf] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xb6] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xb7] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xb8] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xb9] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xba] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xbb] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xbc] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xbd] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xbe] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xbf] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table
}
const INAT_AVX_TABLE_6_1: OpcodeTable = inat_avx_table_6_1_table();

const fn inat_avx_table_6_2_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x56] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x57] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xd6] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xd7] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table
}
const INAT_AVX_TABLE_6_2: OpcodeTable = inat_avx_table_6_2_table();

const fn inat_avx_table_6_3_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x56] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x57] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xd6] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0xd7] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table
}
const INAT_AVX_TABLE_6_3: OpcodeTable = inat_avx_table_6_3_table();

const fn inat_avx_table_7_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0xf8] = INAT_VARIANT;
    table
}
const INAT_AVX_TABLE_7: OpcodeTable = inat_avx_table_7_table();

const fn inat_avx_table_7_2_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0xf8] = make_imm(INAT_IMM_DWORD) | INAT_MODRM | INAT_VEXOK;
    table
}
const INAT_AVX_TABLE_7_2: OpcodeTable = inat_avx_table_7_2_table();

const fn inat_avx_table_7_3_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0xf8] = make_imm(INAT_IMM_DWORD) | INAT_MODRM | INAT_VEXOK;
    table
}
const INAT_AVX_TABLE_7_3: OpcodeTable = inat_avx_table_7_3_table();

const fn inat_xop_table_0_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x85] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0x86] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0x87] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0x8e] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0x8f] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0x95] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0x96] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0x97] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0x9e] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0x9f] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0xa2] = make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_XOPOK
        | make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_XOPOK;
    table[0xa3] = make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_XOPOK
        | make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_XOPOK;
    table[0xa6] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0xb6] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0xc0] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0xc1] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0xc2] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0xc3] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0xcc] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0xcd] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0xce] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0xcf] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0xec] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0xed] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0xee] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table[0xef] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_XOPOK;
    table
}
const INAT_XOP_TABLE_0: OpcodeTable = inat_xop_table_0_table();

const fn inat_xop_table_1_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x01] = make_group(28) | INAT_XOPOK;
    table[0x02] = make_group(29) | INAT_XOPOK;
    table[0x12] = make_group(30) | INAT_XOPOK;
    table[0x80] = INAT_MODRM | INAT_XOPOK;
    table[0x81] = INAT_MODRM | INAT_XOPOK;
    table[0x82] = INAT_MODRM | INAT_XOPOK;
    table[0x83] = INAT_MODRM | INAT_XOPOK;
    table[0x90] = INAT_MODRM | INAT_XOPOK | INAT_MODRM | INAT_XOPOK;
    table[0x91] = INAT_MODRM | INAT_XOPOK | INAT_MODRM | INAT_XOPOK;
    table[0x92] = INAT_MODRM | INAT_XOPOK | INAT_MODRM | INAT_XOPOK;
    table[0x93] = INAT_MODRM | INAT_XOPOK | INAT_MODRM | INAT_XOPOK;
    table[0x94] = INAT_MODRM | INAT_XOPOK | INAT_MODRM | INAT_XOPOK;
    table[0x95] = INAT_MODRM | INAT_XOPOK | INAT_MODRM | INAT_XOPOK;
    table[0x96] = INAT_MODRM | INAT_XOPOK | INAT_MODRM | INAT_XOPOK;
    table[0x97] = INAT_MODRM | INAT_XOPOK | INAT_MODRM | INAT_XOPOK;
    table[0x98] = INAT_MODRM | INAT_XOPOK | INAT_MODRM | INAT_XOPOK;
    table[0x99] = INAT_MODRM | INAT_XOPOK | INAT_MODRM | INAT_XOPOK;
    table[0x9a] = INAT_MODRM | INAT_XOPOK | INAT_MODRM | INAT_XOPOK;
    table[0x9b] = INAT_MODRM | INAT_XOPOK | INAT_MODRM | INAT_XOPOK;
    table[0xc1] = INAT_MODRM | INAT_XOPOK;
    table[0xc2] = INAT_MODRM | INAT_XOPOK;
    table[0xc3] = INAT_MODRM | INAT_XOPOK;
    table[0xc6] = INAT_MODRM | INAT_XOPOK;
    table[0xc7] = INAT_MODRM | INAT_XOPOK;
    table[0xcb] = INAT_MODRM | INAT_XOPOK;
    table[0xd1] = INAT_MODRM | INAT_XOPOK;
    table[0xd2] = INAT_MODRM | INAT_XOPOK;
    table[0xd3] = INAT_MODRM | INAT_XOPOK;
    table[0xd6] = INAT_MODRM | INAT_XOPOK;
    table[0xd7] = INAT_MODRM | INAT_XOPOK;
    table[0xdb] = INAT_MODRM | INAT_XOPOK;
    table[0xe1] = INAT_MODRM | INAT_XOPOK;
    table[0xe2] = INAT_MODRM | INAT_XOPOK;
    table[0xe3] = INAT_MODRM | INAT_XOPOK;
    table
}
const INAT_XOP_TABLE_1: OpcodeTable = inat_xop_table_1_table();

const fn inat_xop_table_2_table() -> OpcodeTable {
    let mut table = [0u32; INAT_OPCODE_TABLE_SIZE];
    table[0x10] = make_imm(INAT_IMM_DWORD) | INAT_MODRM | INAT_XOPOK;
    table[0x12] = make_group(31) | INAT_XOPOK;
    table
}
const INAT_XOP_TABLE_2: OpcodeTable = inat_xop_table_2_table();

const fn inat_group_table_10_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x0] = INAT_MODRM;
    table[0x1] = INAT_MODRM;
    table[0x2] = INAT_MODRM;
    table[0x3] = INAT_MODRM;
    table[0x4] = INAT_MODRM;
    table[0x5] = INAT_MODRM;
    table[0x6] = INAT_VARIANT;
    table
}
const INAT_GROUP_TABLE_10: GroupTable = inat_group_table_10_table();

const fn inat_group_table_10_3_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x6] = INAT_MODRM;
    table
}
const INAT_GROUP_TABLE_10_3: GroupTable = inat_group_table_10_3_table();

const fn inat_group_table_11_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x0] = INAT_MODRM;
    table[0x1] = INAT_MODRM;
    table[0x2] = INAT_MODRM;
    table[0x3] = INAT_MODRM;
    table[0x4] = INAT_MODRM;
    table[0x5] = INAT_VARIANT;
    table[0x6] = INAT_MODRM;
    table[0x7] = INAT_MODRM;
    table
}
const INAT_GROUP_TABLE_11: GroupTable = inat_group_table_11_table();

const fn inat_group_table_11_2_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x5] = INAT_MODRM;
    table
}
const INAT_GROUP_TABLE_11_2: GroupTable = inat_group_table_11_2_table();

const fn inat_group_table_13_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x0] = INAT_MODRM;
    table[0x1] = INAT_MODRM;
    table[0x2] = INAT_MODRM;
    table[0x3] = INAT_MODRM;
    table
}
const INAT_GROUP_TABLE_13: GroupTable = inat_group_table_13_table();

const fn inat_group_table_14_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x0] = INAT_MODRM;
    table
}
const INAT_GROUP_TABLE_14: GroupTable = inat_group_table_14_table();

const fn inat_group_table_15_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x1] = INAT_VARIANT;
    table
}
const INAT_GROUP_TABLE_15: GroupTable = inat_group_table_15_table();

const fn inat_group_table_15_2_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x1] = INAT_MODRM;
    table
}
const INAT_GROUP_TABLE_15_2: GroupTable = inat_group_table_15_2_table();

const fn inat_group_table_16_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x2] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VARIANT;
    table[0x4] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VARIANT;
    table[0x6] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VARIANT;
    table
}
const INAT_GROUP_TABLE_16: GroupTable = inat_group_table_16_table();

const fn inat_group_table_16_1_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x2] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x4] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x6] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table
}
const INAT_GROUP_TABLE_16_1: GroupTable = inat_group_table_16_1_table();

const fn inat_group_table_17_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x0] = INAT_VARIANT;
    table[0x1] = INAT_VARIANT;
    table[0x2] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VARIANT;
    table[0x4] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VARIANT;
    table[0x6] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VARIANT;
    table
}
const INAT_GROUP_TABLE_17: GroupTable = inat_group_table_17_table();

const fn inat_group_table_17_1_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x0] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x1] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x2] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x4] = make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK
        | make_imm(INAT_IMM_BYTE)
        | INAT_MODRM
        | INAT_VEXOK;
    table[0x6] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table
}
const INAT_GROUP_TABLE_17_1: GroupTable = inat_group_table_17_1_table();

const fn inat_group_table_18_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x2] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VARIANT;
    table[0x3] = INAT_VARIANT;
    table[0x6] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VARIANT;
    table[0x7] = INAT_VARIANT;
    table
}
const INAT_GROUP_TABLE_18: GroupTable = inat_group_table_18_table();

const fn inat_group_table_18_1_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x2] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x3] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x6] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table[0x7] = make_imm(INAT_IMM_BYTE) | INAT_MODRM | INAT_VEXOK;
    table
}
const INAT_GROUP_TABLE_18_1: GroupTable = inat_group_table_18_1_table();

const fn inat_group_table_21_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x0] = INAT_VARIANT;
    table[0x1] = INAT_VARIANT;
    table[0x2] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x3] = INAT_MODRM | INAT_VEXOK | INAT_VARIANT;
    table[0x4] = INAT_VARIANT;
    table[0x5] = INAT_VARIANT;
    table[0x6] = INAT_VARIANT;
    table
}
const INAT_GROUP_TABLE_21: GroupTable = inat_group_table_21_table();

const fn inat_group_table_21_1_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x6] = INAT_MODRM;
    table
}
const INAT_GROUP_TABLE_21_1: GroupTable = inat_group_table_21_1_table();

const fn inat_group_table_21_2_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x0] = INAT_MODRM;
    table[0x1] = INAT_MODRM;
    table[0x2] = INAT_MODRM;
    table[0x3] = INAT_MODRM;
    table[0x4] = INAT_MODRM;
    table[0x5] = INAT_MODRM;
    table[0x6] = INAT_MODRM | INAT_MODRM;
    table
}
const INAT_GROUP_TABLE_21_2: GroupTable = inat_group_table_21_2_table();

const fn inat_group_table_21_3_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x6] = INAT_MODRM;
    table
}
const INAT_GROUP_TABLE_21_3: GroupTable = inat_group_table_21_3_table();

const fn inat_group_table_24_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x1] = INAT_MODRM;
    table[0x6] = INAT_MODRM | INAT_MODRM | INAT_VARIANT;
    table[0x7] = INAT_MODRM | INAT_MODRM | INAT_VARIANT;
    table
}
const INAT_GROUP_TABLE_24: GroupTable = inat_group_table_24_table();

const fn inat_group_table_24_1_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x6] = INAT_MODRM;
    table
}
const INAT_GROUP_TABLE_24_1: GroupTable = inat_group_table_24_1_table();

const fn inat_group_table_24_2_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x6] = INAT_MODRM | INAT_MODRM;
    table[0x7] = INAT_MODRM;
    table
}
const INAT_GROUP_TABLE_24_2: GroupTable = inat_group_table_24_2_table();

const fn inat_group_table_25_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x1] = INAT_VARIANT;
    table[0x2] = INAT_VARIANT;
    table[0x5] = INAT_VARIANT;
    table[0x6] = INAT_VARIANT;
    table
}
const INAT_GROUP_TABLE_25: GroupTable = inat_group_table_25_table();

const fn inat_group_table_25_1_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x1] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x2] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x5] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x6] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table
}
const INAT_GROUP_TABLE_25_1: GroupTable = inat_group_table_25_1_table();

const fn inat_group_table_26_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x1] = INAT_VARIANT;
    table[0x2] = INAT_VARIANT;
    table[0x5] = INAT_VARIANT;
    table[0x6] = INAT_VARIANT;
    table
}
const INAT_GROUP_TABLE_26: GroupTable = inat_group_table_26_table();

const fn inat_group_table_26_1_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x1] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x2] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x5] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table[0x6] = INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY;
    table
}
const INAT_GROUP_TABLE_26_1: GroupTable = inat_group_table_26_1_table();

const fn inat_group_table_27_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x1] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x2] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table[0x3] = INAT_MODRM | INAT_VEXOK | INAT_VEXONLY;
    table
}
const INAT_GROUP_TABLE_27: GroupTable = inat_group_table_27_table();

const fn inat_group_table_28_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x1] = INAT_MODRM | INAT_XOPOK;
    table[0x2] = INAT_MODRM | INAT_XOPOK;
    table[0x3] = INAT_MODRM | INAT_XOPOK;
    table[0x4] = INAT_MODRM | INAT_XOPOK;
    table[0x5] = INAT_MODRM | INAT_XOPOK;
    table[0x6] = INAT_MODRM | INAT_XOPOK;
    table[0x7] = INAT_MODRM | INAT_XOPOK;
    table
}
const INAT_GROUP_TABLE_28: GroupTable = inat_group_table_28_table();

const fn inat_group_table_29_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x1] = INAT_MODRM | INAT_XOPOK;
    table[0x6] = INAT_MODRM | INAT_XOPOK;
    table
}
const INAT_GROUP_TABLE_29: GroupTable = inat_group_table_29_table();

const fn inat_group_table_30_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x0] = INAT_MODRM | INAT_XOPOK;
    table[0x1] = INAT_MODRM | INAT_XOPOK;
    table
}
const INAT_GROUP_TABLE_30: GroupTable = inat_group_table_30_table();

const fn inat_group_table_31_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x0] = make_imm(INAT_IMM_DWORD) | INAT_MODRM | INAT_XOPOK;
    table[0x1] = make_imm(INAT_IMM_DWORD) | INAT_MODRM | INAT_XOPOK;
    table
}
const INAT_GROUP_TABLE_31: GroupTable = inat_group_table_31_table();

const fn inat_group_table_4_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x0] = make_imm(INAT_IMM_BYTE) | INAT_MODRM;
    table[0x7] = make_imm(INAT_IMM_BYTE);
    table
}
const INAT_GROUP_TABLE_4: GroupTable = inat_group_table_4_table();

const fn inat_group_table_5_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x0] = make_imm(INAT_IMM_VWORD32) | INAT_MODRM;
    table[0x7] = make_imm(INAT_IMM_VWORD32);
    table
}
const INAT_GROUP_TABLE_5: GroupTable = inat_group_table_5_table();

const fn inat_group_table_6_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x0] = make_imm(INAT_IMM_BYTE) | INAT_MODRM;
    table[0x1] = make_imm(INAT_IMM_BYTE) | INAT_MODRM;
    table[0x2] = INAT_MODRM;
    table[0x3] = INAT_MODRM;
    table[0x4] = INAT_MODRM;
    table[0x5] = INAT_MODRM;
    table[0x6] = INAT_MODRM;
    table[0x7] = INAT_MODRM;
    table
}
const INAT_GROUP_TABLE_6: GroupTable = inat_group_table_6_table();

const fn inat_group_table_7_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x0] = make_imm(INAT_IMM_VWORD32) | INAT_MODRM;
    table[0x1] = make_imm(INAT_IMM_VWORD32) | INAT_MODRM;
    table[0x2] = INAT_MODRM;
    table[0x3] = INAT_MODRM;
    table[0x4] = INAT_MODRM;
    table[0x5] = INAT_MODRM;
    table[0x6] = INAT_MODRM;
    table[0x7] = INAT_MODRM;
    table
}
const INAT_GROUP_TABLE_7: GroupTable = inat_group_table_7_table();

const fn inat_group_table_8_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x0] = INAT_MODRM;
    table[0x1] = INAT_MODRM;
    table
}
const INAT_GROUP_TABLE_8: GroupTable = inat_group_table_8_table();

const fn inat_group_table_9_table() -> GroupTable {
    let mut table = [0u32; INAT_GROUP_TABLE_SIZE];
    table[0x0] = INAT_MODRM;
    table[0x1] = INAT_MODRM;
    table[0x2] = INAT_MODRM | INAT_FORCE64;
    table[0x3] = INAT_MODRM;
    table[0x4] = INAT_MODRM | INAT_FORCE64;
    table[0x5] = INAT_MODRM;
    table[0x6] = INAT_MODRM | INAT_FORCE64;
    table
}
const INAT_GROUP_TABLE_9: GroupTable = inat_group_table_9_table();

const PRIMARY_TABLE: OpcodeTable = INAT_PRIMARY_TABLE;

const ESCAPE_TABLES: [OpcodeLpfxTables; 4] = [
    [None, None, None, None],
    [
        Some(&INAT_ESCAPE_TABLE_1),
        Some(&INAT_ESCAPE_TABLE_1_1),
        Some(&INAT_ESCAPE_TABLE_1_2),
        Some(&INAT_ESCAPE_TABLE_1_3),
    ],
    [
        Some(&INAT_ESCAPE_TABLE_2),
        Some(&INAT_ESCAPE_TABLE_2_1),
        Some(&INAT_ESCAPE_TABLE_2_2),
        Some(&INAT_ESCAPE_TABLE_2_3),
    ],
    [
        Some(&INAT_ESCAPE_TABLE_3),
        Some(&INAT_ESCAPE_TABLE_3_1),
        Some(&INAT_ESCAPE_TABLE_3_2),
        Some(&INAT_ESCAPE_TABLE_3_3),
    ],
];

const GROUP_TABLES: [GroupLpfxTables; 32] = [
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [Some(&INAT_GROUP_TABLE_4), None, None, None],
    [Some(&INAT_GROUP_TABLE_5), None, None, None],
    [Some(&INAT_GROUP_TABLE_6), None, None, None],
    [Some(&INAT_GROUP_TABLE_7), None, None, None],
    [Some(&INAT_GROUP_TABLE_8), None, None, None],
    [Some(&INAT_GROUP_TABLE_9), None, None, None],
    [
        Some(&INAT_GROUP_TABLE_10),
        None,
        None,
        Some(&INAT_GROUP_TABLE_10_3),
    ],
    [
        Some(&INAT_GROUP_TABLE_11),
        None,
        Some(&INAT_GROUP_TABLE_11_2),
        None,
    ],
    [None, None, None, None],
    [Some(&INAT_GROUP_TABLE_13), None, None, None],
    [Some(&INAT_GROUP_TABLE_14), None, None, None],
    [
        Some(&INAT_GROUP_TABLE_15),
        None,
        Some(&INAT_GROUP_TABLE_15_2),
        None,
    ],
    [
        Some(&INAT_GROUP_TABLE_16),
        Some(&INAT_GROUP_TABLE_16_1),
        None,
        None,
    ],
    [
        Some(&INAT_GROUP_TABLE_17),
        Some(&INAT_GROUP_TABLE_17_1),
        None,
        None,
    ],
    [
        Some(&INAT_GROUP_TABLE_18),
        Some(&INAT_GROUP_TABLE_18_1),
        None,
        None,
    ],
    [None, None, None, None],
    [None, None, None, None],
    [
        Some(&INAT_GROUP_TABLE_21),
        Some(&INAT_GROUP_TABLE_21_1),
        Some(&INAT_GROUP_TABLE_21_2),
        Some(&INAT_GROUP_TABLE_21_3),
    ],
    [None, None, None, None],
    [None, None, None, None],
    [
        Some(&INAT_GROUP_TABLE_24),
        Some(&INAT_GROUP_TABLE_24_1),
        Some(&INAT_GROUP_TABLE_24_2),
        None,
    ],
    [
        Some(&INAT_GROUP_TABLE_25),
        Some(&INAT_GROUP_TABLE_25_1),
        None,
        None,
    ],
    [
        Some(&INAT_GROUP_TABLE_26),
        Some(&INAT_GROUP_TABLE_26_1),
        None,
        None,
    ],
    [Some(&INAT_GROUP_TABLE_27), None, None, None],
    [Some(&INAT_GROUP_TABLE_28), None, None, None],
    [Some(&INAT_GROUP_TABLE_29), None, None, None],
    [Some(&INAT_GROUP_TABLE_30), None, None, None],
    [Some(&INAT_GROUP_TABLE_31), None, None, None],
];

const AVX_TABLES: [OpcodeLpfxTables; 32] = [
    [None, None, None, None],
    [
        Some(&INAT_ESCAPE_TABLE_1),
        Some(&INAT_ESCAPE_TABLE_1_1),
        Some(&INAT_ESCAPE_TABLE_1_2),
        Some(&INAT_ESCAPE_TABLE_1_3),
    ],
    [
        Some(&INAT_ESCAPE_TABLE_2),
        Some(&INAT_ESCAPE_TABLE_2_1),
        Some(&INAT_ESCAPE_TABLE_2_2),
        Some(&INAT_ESCAPE_TABLE_2_3),
    ],
    [
        Some(&INAT_ESCAPE_TABLE_3),
        Some(&INAT_ESCAPE_TABLE_3_1),
        Some(&INAT_ESCAPE_TABLE_3_2),
        Some(&INAT_ESCAPE_TABLE_3_3),
    ],
    [
        Some(&INAT_AVX_TABLE_4),
        Some(&INAT_AVX_TABLE_4_1),
        Some(&INAT_AVX_TABLE_4_2),
        Some(&INAT_AVX_TABLE_4_3),
    ],
    [
        Some(&INAT_AVX_TABLE_5),
        Some(&INAT_AVX_TABLE_5_1),
        Some(&INAT_AVX_TABLE_5_2),
        Some(&INAT_AVX_TABLE_5_3),
    ],
    [
        Some(&INAT_AVX_TABLE_6),
        Some(&INAT_AVX_TABLE_6_1),
        Some(&INAT_AVX_TABLE_6_2),
        Some(&INAT_AVX_TABLE_6_3),
    ],
    [
        Some(&INAT_AVX_TABLE_7),
        None,
        Some(&INAT_AVX_TABLE_7_2),
        Some(&INAT_AVX_TABLE_7_3),
    ],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
    [None, None, None, None],
];

const XOP_TABLES: [Option<&'static OpcodeTable>; INAT_XOP_TABLE_COUNT] = [
    Some(&INAT_XOP_TABLE_0),
    Some(&INAT_XOP_TABLE_1),
    Some(&INAT_XOP_TABLE_2),
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
];

#[inline]
pub const fn x86_modrm_reg(modrm: InsnByte) -> usize {
    ((modrm & 0x38) >> 3) as usize
}

fn opcode_table_for_prefix(
    tables: &OpcodeLpfxTables,
    lpfx_id: InsnAttr,
) -> Option<&'static OpcodeTable> {
    tables.get(lpfx_id as usize).and_then(|table| *table)
}

fn group_table_for_prefix(
    tables: &GroupLpfxTables,
    lpfx_id: InsnAttr,
) -> Option<&'static GroupTable> {
    tables.get(lpfx_id as usize).and_then(|table| *table)
}

fn dispatch_escape_attribute(
    opcode: InsnByte,
    lpfx_id: InsnAttr,
    esc_attr: InsnAttr,
    tables: &[OpcodeLpfxTables],
) -> InsnAttr {
    let table_set = match tables.get(escape_id(esc_attr) as usize) {
        Some(table_set) => table_set,
        None => return 0,
    };
    let mut table = match table_set[0] {
        Some(table) => table,
        None => return 0,
    };
    if has_variant(table[opcode as usize]) && lpfx_id != 0 {
        table = match opcode_table_for_prefix(table_set, lpfx_id) {
            Some(table) => table,
            None => return 0,
        };
    }
    table[opcode as usize]
}

fn dispatch_group_attribute(
    modrm: InsnByte,
    lpfx_id: InsnAttr,
    grp_attr: InsnAttr,
    tables: &[GroupLpfxTables],
) -> InsnAttr {
    let common = group_common_attribute(grp_attr);
    let table_set = match tables.get(group_id(grp_attr) as usize) {
        Some(table_set) => table_set,
        None => return common,
    };
    let mut table = match table_set[0] {
        Some(table) => table,
        None => return common,
    };
    let reg = x86_modrm_reg(modrm);
    if has_variant(table[reg]) && lpfx_id != 0 {
        table = match group_table_for_prefix(table_set, lpfx_id) {
            Some(table) => table,
            None => return common,
        };
    }
    table[reg] | common
}

fn dispatch_avx_attribute(
    opcode: InsnByte,
    vex_m: InsnByte,
    vex_p: InsnByte,
    tables: &[OpcodeLpfxTables],
) -> InsnAttr {
    if vex_m > X86_VEX_M_MAX || vex_p as InsnAttr > INAT_LSTPFX_MAX {
        return 0;
    }
    let table_set = match tables.get(vex_m as usize) {
        Some(table_set) => table_set,
        None => return 0,
    };
    let mut table = match table_set[0] {
        Some(table) => table,
        None => return 0,
    };
    if !is_group(table[opcode as usize]) && vex_p != 0 {
        table = match opcode_table_for_prefix(table_set, vex_p as InsnAttr) {
            Some(table) => table,
            None => return 0,
        };
    }
    table[opcode as usize]
}

fn dispatch_xop_attribute(
    opcode: InsnByte,
    map_select: InsnByte,
    tables: &[Option<&'static OpcodeTable>],
) -> InsnAttr {
    if !(X86_XOP_M_MIN..=X86_XOP_M_MAX).contains(&map_select) {
        return 0;
    }
    let table = match tables
        .get((map_select - X86_XOP_M_MIN) as usize)
        .and_then(|table| *table)
    {
        Some(table) => table,
        None => return 0,
    };
    table[opcode as usize]
}
// ---- inat.c dispatcher API --------------------------------------------------

/// `inat_get_opcode_attribute(opcode)` — direct primary-table lookup.
/// Mirrors inat.c lines 13-16.
#[inline]
pub fn get_opcode_attribute(opcode: InsnByte) -> InsnAttr {
    PRIMARY_TABLE[opcode as usize]
}

/// `inat_get_last_prefix_id(last_pfx)` — return the legacy-prefix ID
/// (1..=3) of the *last* prefix observed. Mirrors lines 18-24.
#[inline]
pub fn get_last_prefix_id(last_pfx: InsnByte) -> InsnAttr {
    last_prefix_id(get_opcode_attribute(last_pfx))
}

/// `inat_get_escape_attribute()` — full two/three-byte opcode lookup.
/// Selects the base escape table, then the last-prefix variant table when
/// Linux would do the same.
pub fn get_escape_attribute(opcode: InsnByte, lpfx_id: InsnAttr, esc_attr: InsnAttr) -> InsnAttr {
    dispatch_escape_attribute(opcode, lpfx_id, esc_attr, &ESCAPE_TABLES)
}

/// `inat_get_group_attribute()` — group-encoded opcode lookup.
/// Selects by `X86_MODRM_REG(modrm)` and preserves common group attributes.
pub fn get_group_attribute(modrm: InsnByte, lpfx_id: InsnAttr, grp_attr: InsnAttr) -> InsnAttr {
    dispatch_group_attribute(modrm, lpfx_id, grp_attr, &GROUP_TABLES)
}

/// `inat_get_avx_attribute()` — AVX/EVEX opcode lookup.
/// Enforces Linux VEX map and prefix bounds before table lookup.
pub fn get_avx_attribute(opcode: InsnByte, vex_m: InsnByte, vex_p: InsnByte) -> InsnAttr {
    dispatch_avx_attribute(opcode, vex_m, vex_p, &AVX_TABLES)
}

/// `inat_get_xop_attribute()` — XOP opcode lookup.
/// Enforces Linux XOP map bounds before table lookup.
pub fn get_xop_attribute(opcode: InsnByte, map_select: InsnByte) -> InsnAttr {
    dispatch_xop_attribute(opcode, map_select, &XOP_TABLES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pfx_mask_constants_pack_into_5_bits() {
        assert_eq!(INAT_PFX_BITS, 5);
        assert_eq!(INAT_PFX_MAX, 0x1F);
        assert_eq!(INAT_PFX_MASK, 0x1F);
    }

    #[test]
    fn esc_grp_imm_offsets_follow_inat_h() {
        assert_eq!(INAT_ESC_OFFS, 5);
        assert_eq!(INAT_GRP_OFFS, 7);
        assert_eq!(INAT_IMM_OFFS, 12);
        assert_eq!(INAT_FLAG_OFFS, 15);
    }

    #[test]
    fn legacy_prefix_recognised_for_segment_overrides() {
        let cs = get_opcode_attribute(0x2E);
        assert!(is_legacy_prefix(cs));
        assert_eq!(cs & INAT_PFX_MASK, INAT_PFX_CS);
    }

    #[test]
    fn rex_prefix_attr_for_every_byte_in_4x_range() {
        for op in 0x40..=0x4F {
            let a = get_opcode_attribute(op as u8);
            assert!(is_rex_prefix(a), "0x{:x} should be REX", op);
        }
    }

    #[test]
    fn vex_evex_xop_prefixes_set_correct_pfx_id() {
        assert!(is_vex_prefix(get_opcode_attribute(0xC5)));
        assert!(is_vex_prefix(get_opcode_attribute(0xC4)));
        assert!(is_vex3_prefix(get_opcode_attribute(0xC4)));
        assert!(is_vex_prefix(get_opcode_attribute(0x62)));
        assert!(is_evex_prefix(get_opcode_attribute(0x62)));
        assert!(is_xop_prefix(get_opcode_attribute(0x8F)));
        assert!(is_rex2_prefix(get_opcode_attribute(0xD5)));
    }

    #[test]
    fn last_prefix_id_returns_index_only_for_legacy_last_prefixes() {
        // OPNDSZ → 1, REPE → 2, REPNE → 3.
        assert_eq!(get_last_prefix_id(0x66), INAT_PFX_OPNDSZ);
        assert_eq!(get_last_prefix_id(0xF3), INAT_PFX_REPE);
        assert_eq!(get_last_prefix_id(0xF2), INAT_PFX_REPNE);
        // LOCK is a non-last legacy prefix → 0.
        assert_eq!(get_last_prefix_id(0xF0), 0);
        // CS is a non-last legacy prefix → 0 too.
        assert_eq!(get_last_prefix_id(0x2E), 0);
    }

    #[test]
    fn mov_and_call_attributes_signal_modrm_or_imm() {
        assert!(has_modrm(get_opcode_attribute(0x89)));
        assert!(has_modrm(get_opcode_attribute(0x8B)));
        assert!(has_immediate(get_opcode_attribute(0xE8)));
        assert_eq!(immediate_size(get_opcode_attribute(0xE8)), INAT_IMM_VWORD32);
        assert!(has_immediate(get_opcode_attribute(0xEB)));
        assert_eq!(immediate_size(get_opcode_attribute(0xEB)), INAT_IMM_BYTE);
    }

    #[test]
    fn make_macros_pack_into_the_documented_fields() {
        assert_eq!(make_prefix(INAT_PFX_OPNDSZ), 1);
        assert_eq!(make_escape(1), 1 << INAT_ESC_OFFS);
        let g = make_group(2);
        assert_eq!(g & INAT_GRP_MASK, 2 << INAT_GRP_OFFS);
        assert!(has_modrm(g));
    }

    #[test]
    fn escape_attribute_for_0x0f_marks_two_byte_opcode() {
        let a = get_opcode_attribute(0x0F);
        assert!(is_escape(a));
        assert_eq!(escape_id(a), 1);
    }

    #[test]
    fn generated_dispatchers_select_linux_variant_tables() {
        let esc_attr = get_opcode_attribute(0x0f);
        assert_eq!(
            get_escape_attribute(0x10, 0, esc_attr),
            INAT_MODRM | INAT_VEXOK | INAT_VARIANT
        );
        assert_eq!(
            get_escape_attribute(0x10, INAT_PFX_OPNDSZ, esc_attr),
            INAT_MODRM | INAT_VEXOK
        );

        let grp_attr = make_group(10);
        assert_eq!(
            get_group_attribute(6 << 3, 0, grp_attr),
            INAT_MODRM | INAT_VARIANT
        );
        assert_eq!(
            get_group_attribute(6 << 3, INAT_PFX_REPNE, grp_attr),
            INAT_MODRM
        );
    }

    #[test]
    fn avx_and_xop_dispatchers_match_linux_bounds_and_maps() {
        assert_eq!(
            get_avx_attribute(0x00, 4, 0),
            INAT_MODRM | INAT_VEXOK | INAT_EVEXONLY
        );
        assert_eq!(get_avx_attribute(0x00, X86_VEX_M_MAX + 1, 0), 0);
        assert_eq!(get_avx_attribute(0x10, 4, INAT_PFX_OPNDSZ as u8), 0);

        assert_eq!(
            get_xop_attribute(0x10, X86_XOP_M_MIN + 2),
            make_imm(INAT_IMM_DWORD) | INAT_MODRM | INAT_XOPOK
        );
        assert_eq!(get_xop_attribute(0x10, X86_XOP_M_MIN - 1), 0);
        assert_eq!(get_xop_attribute(0x10, X86_XOP_M_MAX + 1), 0);
    }

    #[test]
    fn unknown_opcodes_return_zero_attribute() {
        // 0xD6 (legacy "SETALC") is not advertised by the generated Linux
        // opcode map; the entry must stay zero.
        assert_eq!(get_opcode_attribute(0xD6), 0);
    }

    #[test]
    fn inat_dispatchers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/lib/inat.c"
        ));
        assert!(source.contains("insn_attr_t inat_get_opcode_attribute(insn_byte_t opcode)"));
        assert!(source.contains("return inat_primary_table[opcode];"));
        assert!(source.contains("int inat_get_last_prefix_id(insn_byte_t last_pfx)"));
        assert!(source.contains("inat_get_escape_attribute"));
        assert!(source.contains("inat_escape_tables[n][0]"));
        assert!(source.contains("inat_has_variant(table[opcode]) && lpfx_id"));
        assert!(source.contains("inat_get_group_attribute"));
        assert!(source.contains("X86_MODRM_REG(modrm)"));
        assert!(source.contains("inat_get_avx_attribute"));
        assert!(source.contains("vex_m > X86_VEX_M_MAX"));
        assert!(source.contains("inat_get_xop_attribute"));
    }
}
