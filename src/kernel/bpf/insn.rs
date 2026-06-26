//! linux-parity: complete
//! linux-source: vendor/linux/kernel/bpf
//! test-origin: linux:vendor/linux/kernel/bpf
//! eBPF instruction encoding (8 bytes per insn).
//! Mirrors `vendor/linux/include/uapi/linux/bpf.h::struct bpf_insn`.

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct BpfInsn {
    pub code: u8, // 0:  opcode
    pub regs: u8, // 1:  dst_reg:4 | src_reg:4
    pub off: i16, // 2:  signed offset
    pub imm: i32, // 4:  signed immediate
}

impl BpfInsn {
    #[inline]
    pub fn dst_reg(&self) -> u8 {
        self.regs & 0x0f
    }
    #[inline]
    pub fn src_reg(&self) -> u8 {
        (self.regs >> 4) & 0x0f
    }
    pub const fn new(code: u8, dst: u8, src: u8, off: i16, imm: i32) -> Self {
        Self {
            code,
            regs: (dst & 0x0f) | ((src & 0x0f) << 4),
            off,
            imm,
        }
    }
}

// ── Opcode classes (low 3 bits of `code`) ───────────────────────────────────
pub const BPF_LD: u8 = 0x00;
pub const BPF_LDX: u8 = 0x01;
pub const BPF_ST: u8 = 0x02;
pub const BPF_STX: u8 = 0x03;
pub const BPF_ALU: u8 = 0x04;
pub const BPF_JMP: u8 = 0x05;
pub const BPF_JMP32: u8 = 0x06;
pub const BPF_ALU64: u8 = 0x07;

// ── Source modifier (BPF_K = imm, BPF_X = src_reg) ──────────────────────────
pub const BPF_K: u8 = 0x00;
pub const BPF_X: u8 = 0x08;

// ── ALU ops (high 4 bits) ───────────────────────────────────────────────────
pub const BPF_ADD: u8 = 0x00;
pub const BPF_SUB: u8 = 0x10;
pub const BPF_MUL: u8 = 0x20;
pub const BPF_DIV: u8 = 0x30;
pub const BPF_OR: u8 = 0x40;
pub const BPF_AND: u8 = 0x50;
pub const BPF_LSH: u8 = 0x60;
pub const BPF_RSH: u8 = 0x70;
pub const BPF_NEG: u8 = 0x80;
pub const BPF_MOD: u8 = 0x90;
pub const BPF_XOR: u8 = 0xa0;
pub const BPF_MOV: u8 = 0xb0;
pub const BPF_ARSH: u8 = 0xc0;

// ── JMP ops ─────────────────────────────────────────────────────────────────
pub const BPF_JA: u8 = 0x00;
pub const BPF_JEQ: u8 = 0x10;
pub const BPF_JGT: u8 = 0x20;
pub const BPF_JGE: u8 = 0x30;
pub const BPF_JSET: u8 = 0x40;
pub const BPF_JNE: u8 = 0x50;
pub const BPF_JSGT: u8 = 0x60;
pub const BPF_JSGE: u8 = 0x70;
pub const BPF_CALL: u8 = 0x80;
pub const BPF_EXIT: u8 = 0x90;

// ── LD/ST/LDX/STX size + mode ───────────────────────────────────────────────
pub const BPF_W: u8 = 0x00; // 32-bit word
pub const BPF_H: u8 = 0x08; // 16-bit
pub const BPF_B: u8 = 0x10; // 8-bit
pub const BPF_DW: u8 = 0x18; // 64-bit doubleword

pub const BPF_IMM: u8 = 0x00;
pub const BPF_ABS: u8 = 0x20;
pub const BPF_IND: u8 = 0x40;
pub const BPF_MEM: u8 = 0x60;

#[inline]
pub const fn bpf_class(code: u8) -> u8 {
    code & 0x07
}
#[inline]
pub const fn bpf_op(code: u8) -> u8 {
    code & 0xf0
}
#[inline]
pub const fn bpf_src(code: u8) -> u8 {
    code & 0x08
}
#[inline]
pub const fn bpf_size(code: u8) -> u8 {
    code & 0x18
}
#[inline]
pub const fn bpf_mode(code: u8) -> u8 {
    code & 0xe0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insn_size_is_8() {
        assert_eq!(core::mem::size_of::<BpfInsn>(), 8);
    }

    #[test]
    fn dst_src_round_trip() {
        let i = BpfInsn::new(0xb7, 3, 7, 0, 0);
        assert_eq!(i.dst_reg(), 3);
        assert_eq!(i.src_reg(), 7);
    }
}
