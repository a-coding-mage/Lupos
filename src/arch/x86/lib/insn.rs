//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/lib/insn.c
//! test-origin: linux:vendor/linux/arch/x86/lib/insn.c
//! x86 instruction decoder.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/lib/insn.c
//! - vendor/linux/arch/x86/include/asm/insn.h
//!
//! Mirrors Linux's `struct insn` / `struct insn_field` layout and the
//! `insn_get_*` walk: prefixes → REX/VEX → opcode → ModRM → SIB →
//! displacement → immediate. The decoder is consumed by:
//!   * kprobes (`crate::kernel::kprobes::arch_prepare_kprobe`),
//!   * uprobes,
//!   * the X86 page-fault helper (`insn-eval.c`),
//!   * the alternative / static-call patchers.
//!
//! Coverage in this port matches the bootstrap subset of `inat.rs` —
//! see that module's commentary for the planned full opcode-map import.

use super::inat::*;

/// `MAX_INSN_SIZE` — Intel's architectural ceiling on instruction
/// length. Mirrors `insn.h` line 108.
pub const MAX_INSN_SIZE: usize = 15;

/// `struct insn_field` from `insn.h`. The bitfield/union shape is
/// replicated literally so byte-wise comparisons against a Linux
/// decoder match exactly.
#[derive(Default, Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct InsnField {
    pub value: u32,
    pub got: u8,
    pub nbytes: u8,
}

impl InsnField {
    /// Linux `insn_field_set(p, v, n)` — store a fully-formed value
    /// alongside the byte count it spans.
    pub fn set(&mut self, value: u32, nbytes: u8) {
        self.value = value;
        self.nbytes = nbytes;
        self.got = 1;
    }
    /// Linux `insn_set_byte(p, n, v)` — append one byte and recompute
    /// the little-endian integer value. Little-endian-only target.
    pub fn set_byte(&mut self, n: u8, byte: u8) {
        let shift = (n as u32) * 8;
        self.value = (self.value & !(0xffu32 << shift)) | ((byte as u32) << shift);
    }
}

/// Mirror of `struct insn` from insn.h. Fields appear in the same
/// order so a future binary-compat check (kprobes shares the layout
/// with userspace tools like `objtool`) can `repr(C)` it directly.
#[derive(Default, Clone, Debug)]
pub struct Insn {
    pub prefixes: InsnField,
    pub rex_prefix: InsnField,
    pub vex_prefix: InsnField,
    pub opcode: InsnField,
    pub modrm: InsnField,
    pub sib: InsnField,
    pub displacement: InsnField,
    pub immediate: InsnField,
    pub immediate2: InsnField,

    pub emulate_prefix_size: u32,
    pub attr: InsnAttr,
    pub opnd_bytes: u8,
    pub addr_bytes: u8,
    pub length: u8,
    pub x86_64: u8,

    /// View of the instruction buffer.
    bytes: [u8; MAX_INSN_SIZE],
    buf_len: u8,
    cursor: u8,
}

impl Insn {
    /// `insn_init(insn, kaddr, buf_len, x86_64)` — initialise a fresh
    /// decoder over a byte slice. Mirrors insn.c:insn_init().
    pub fn init(buf: &[u8], x86_64: bool) -> Self {
        let mut bytes = [0u8; MAX_INSN_SIZE];
        let n = buf.len().min(MAX_INSN_SIZE);
        bytes[..n].copy_from_slice(&buf[..n]);
        Self {
            bytes,
            buf_len: n as u8,
            cursor: 0,
            x86_64: x86_64 as u8,
            opnd_bytes: if x86_64 { 4 } else { 4 },
            addr_bytes: if x86_64 { 8 } else { 4 },
            ..Default::default()
        }
    }

    #[inline]
    fn peek(&self) -> Option<u8> {
        if (self.cursor as usize) < self.buf_len as usize {
            Some(self.bytes[self.cursor as usize])
        } else {
            None
        }
    }

    #[inline]
    fn consume(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.cursor += 1;
        Some(b)
    }

    /// `insn_get_prefixes()` — walk every legacy prefix and the
    /// optional REX byte. Mirrors insn.c:insn_get_prefixes(). The
    /// bootstrap port handles legacy prefixes + REX; full VEX/EVEX/XOP
    /// branch detection delegates to `super::inat`.
    pub fn get_prefixes(&mut self) {
        let mut nb_prefixes = 0u8;
        let mut last_prefix: u8 = 0;
        loop {
            let Some(b) = self.peek() else {
                break;
            };
            let attr = get_opcode_attribute(b);
            if !is_legacy_prefix(attr) {
                break;
            }
            self.cursor += 1;
            last_prefix = b;
            nb_prefixes += 1;
            if nb_prefixes >= 4 {
                // Linux capping mirrors Intel's 4-prefix limit.
                break;
            }
        }
        self.prefixes.value = last_prefix as u32;
        self.prefixes.nbytes = nb_prefixes;
        self.prefixes.got = 1;

        // REX byte? Only in 64-bit mode.
        if self.x86_64 != 0 {
            if let Some(b) = self.peek() {
                let attr = get_opcode_attribute(b);
                if is_rex_prefix(attr) {
                    self.cursor += 1;
                    self.rex_prefix.set(b as u32, 1);
                    // REX.W flips operand size to 64-bit.
                    if (b & 0x08) != 0 {
                        self.opnd_bytes = 8;
                    }
                }
            }
        }
    }

    /// `insn_get_opcode()` — record the 1- or 2-byte primary opcode.
    /// The 0x0F escape is observed but the secondary opcode table is
    /// not yet populated (returns attr=0).
    pub fn get_opcode(&mut self) {
        let Some(op1) = self.consume() else {
            return;
        };
        let attr1 = get_opcode_attribute(op1);
        self.opcode.set(op1 as u32, 1);
        if is_escape(attr1) {
            if let Some(op2) = self.consume() {
                self.opcode.set_byte(1, op2);
                self.opcode.nbytes = 2;
                self.attr = get_escape_attribute(op2, 0, attr1);
                return;
            }
        }
        self.attr = attr1;
    }

    /// `insn_get_modrm()` — fetch ModRM if the attribute calls for one.
    /// SIB and displacement are queued for `get_sib`/`get_displacement`.
    pub fn get_modrm(&mut self) {
        if !has_modrm(self.attr) {
            return;
        }
        if let Some(modrm) = self.consume() {
            self.modrm.set(modrm as u32, 1);
        }
    }

    /// `insn_get_sib()` — present iff Mod ≠ 3 and R/M = 4 in ModRM.
    pub fn get_sib(&mut self) {
        if self.modrm.got == 0 {
            return;
        }
        let modrm = self.modrm.value as u8;
        let r#mod = (modrm >> 6) & 0x3;
        let rm = modrm & 0x7;
        if r#mod != 3 && rm == 4 {
            if let Some(sib) = self.consume() {
                self.sib.set(sib as u32, 1);
            }
        }
    }

    /// `insn_get_displacement()` — 0/1/4-byte displacement depending
    /// on Mod field. (Mod=10 → 4 bytes, Mod=01 → 1 byte; Mod=00 may
    /// also carry a 4-byte disp32 when R/M=5.)
    pub fn get_displacement(&mut self) {
        if self.modrm.got == 0 {
            return;
        }
        let modrm = self.modrm.value as u8;
        let r#mod = (modrm >> 6) & 0x3;
        let rm = modrm & 0x7;
        let nbytes = match r#mod {
            0 if rm == 5 => 4, // disp32 with no base (RIP-relative in 64-bit).
            0 => {
                // SIB special case: Mod=00, R/M=4, base=5 → disp32.
                if rm == 4 && self.sib.got != 0 && (self.sib.value as u8 & 0x7) == 5 {
                    4
                } else {
                    0
                }
            }
            1 => 1,
            2 => 4,
            _ => 0, // Mod=3: register operand, no displacement.
        };
        if nbytes == 0 {
            return;
        }
        let mut val: u32 = 0;
        let mut got: u8 = 0;
        for i in 0..nbytes {
            if let Some(b) = self.consume() {
                val |= (b as u32) << (i * 8);
                got += 1;
            } else {
                break;
            }
        }
        if got > 0 {
            // Sign-extend if necessary.
            if got == 1 {
                let s = (val & 0xff) as i8 as i32;
                self.displacement.set(s as u32, got);
            } else if got == 4 {
                self.displacement.set(val, got);
            }
        }
    }

    /// `insn_get_immediate()` — read the trailing immediate(s) as
    /// described by `attr`.
    pub fn get_immediate(&mut self) {
        if !has_immediate(self.attr) {
            return;
        }
        let size = match immediate_size(self.attr) {
            INAT_IMM_BYTE => 1,
            INAT_IMM_WORD => 2,
            INAT_IMM_DWORD => 4,
            INAT_IMM_QWORD => 8,
            INAT_IMM_VWORD => self.opnd_bytes as usize,
            INAT_IMM_VWORD32 => 4,
            INAT_IMM_PTR => {
                // Far-pointer immediate: opnd_bytes + 2.
                self.opnd_bytes as usize + 2
            }
            _ => 0,
        };
        if size == 0 {
            return;
        }
        let mut val: u32 = 0;
        let mut got: u8 = 0;
        let take = size.min(4);
        for i in 0..take {
            if let Some(b) = self.consume() {
                val |= (b as u32) << (i * 8);
                got += 1;
            } else {
                break;
            }
        }
        if got > 0 {
            self.immediate.set(val, got);
        }
        // 64-bit immediates land in `immediate2` per insn.h union.
        if size > 4 {
            let mut val2: u32 = 0;
            let mut got2: u8 = 0;
            let extra = size - 4;
            for i in 0..extra {
                if let Some(b) = self.consume() {
                    val2 |= (b as u32) << (i * 8);
                    got2 += 1;
                } else {
                    break;
                }
            }
            if got2 > 0 {
                self.immediate2.set(val2, got2);
            }
        }
    }

    /// `insn_get_length()` — drive every step and record total bytes.
    /// Mirrors insn.c:insn_get_length().
    pub fn get_length(&mut self) -> u8 {
        if self.prefixes.got == 0 {
            self.get_prefixes();
        }
        if self.opcode.got == 0 {
            self.get_opcode();
        }
        if self.modrm.got == 0 {
            self.get_modrm();
        }
        if self.sib.got == 0 {
            self.get_sib();
        }
        if self.displacement.got == 0 {
            self.get_displacement();
        }
        if self.immediate.got == 0 {
            self.get_immediate();
        }
        self.length = self.cursor;
        self.length
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_nop_is_one_byte() {
        let mut i = Insn::init(&[0x90], true);
        assert_eq!(i.get_length(), 1);
        assert_eq!(i.opcode.value, 0x90);
        assert_eq!(i.opcode.nbytes, 1);
    }

    #[test]
    fn decode_call_rel32_consumes_5_bytes() {
        // E8 11 22 33 44 — CALL +0x44332211.
        let mut i = Insn::init(&[0xE8, 0x11, 0x22, 0x33, 0x44], true);
        assert_eq!(i.get_length(), 5);
        assert_eq!(i.immediate.value, 0x4433_2211);
        assert_eq!(i.immediate.nbytes, 4);
    }

    #[test]
    fn decode_jmp_rel8_consumes_2_bytes() {
        let mut i = Insn::init(&[0xEB, 0xFE], true);
        assert_eq!(i.get_length(), 2);
        // Raw immediate stored unsigned (matches insn_field_t semantics
        // — sign extension is callers' job). 0xFE remains 0xFE here;
        // the caller does `value as i8 as i32` to get -2.
        assert_eq!(i.immediate.value, 0xFE);
        assert_eq!(i.immediate.nbytes, 1);
        assert_eq!(i.immediate.value as i8 as i32, -2);
    }

    #[test]
    fn decode_mov_r64_imm64_consumes_10_bytes() {
        // 48 B8 11 22 33 44 55 66 77 88 — MOV rax, 0x8877665544332211.
        let mut i = Insn::init(
            &[0x48, 0xB8, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88],
            true,
        );
        assert_eq!(i.get_length(), 10);
        assert_eq!(i.rex_prefix.value, 0x48); // REX.W set.
        assert_eq!(i.opnd_bytes, 8);
        assert_eq!(i.immediate.value, 0x4433_2211);
        assert_eq!(i.immediate2.value, 0x8877_6655);
    }

    #[test]
    fn decode_mov_with_legacy_prefix_records_prefix() {
        // 66 89 D8 — MOV ax, bx (operand-size prefix).
        let mut i = Insn::init(&[0x66, 0x89, 0xD8], true);
        assert_eq!(i.get_length(), 3);
        assert_eq!(i.prefixes.value, 0x66);
        assert_eq!(i.prefixes.nbytes, 1);
        assert_eq!(i.opcode.value, 0x89);
        assert_eq!(i.modrm.value, 0xD8);
    }

    #[test]
    fn decode_ret_is_one_byte() {
        let mut i = Insn::init(&[0xC3], true);
        assert_eq!(i.get_length(), 1);
        assert_eq!(i.opcode.value, 0xC3);
    }

    #[test]
    fn max_insn_size_matches_intel_architecture_ceiling() {
        assert_eq!(MAX_INSN_SIZE, 15);
    }

    #[test]
    fn insn_field_set_byte_packs_little_endian() {
        let mut f = InsnField::default();
        f.set_byte(0, 0x11);
        f.set_byte(1, 0x22);
        f.set_byte(2, 0x33);
        f.set_byte(3, 0x44);
        assert_eq!(f.value, 0x4433_2211);
    }
}
