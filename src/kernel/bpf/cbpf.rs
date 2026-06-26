//! linux-parity: complete
//! linux-source: vendor/linux/kernel/bpf
//! test-origin: linux:vendor/linux/kernel/bpf
//! Classic BPF (cBPF) interpreter — Milestone 27.
//!
//! Linux's seccomp filter accepts cBPF programs (`struct sock_filter[]`) and
//! evaluates them against `struct seccomp_data`.  Although Linux internally
//! transpiles cBPF to eBPF and JITs it, the userspace ABI is pure cBPF — so
//! a small interpreter that walks `sock_filter` instructions in order is
//! sufficient for full seccomp compliance.  The eBPF JIT lands in M63.
//!
//! This module implements:
//!   - `SockFilter` — the 8-byte cBPF instruction record.
//!   - `BPF_*` opcode constants (matching `vendor/linux/include/uapi/linux/bpf_common.h`
//!     and `include/uapi/linux/filter.h`).
//!   - `bpf_run_filter` — the interpreter entry point.
//!
//! Reference: Linux `net/core/filter.c::__bpf_prog_run` (cBPF path),
//! Steven McCanne & Van Jacobson, "The BSD Packet Filter" (USENIX 1993).

// ── Opcode classes (low 3 bits of `code`) ────────────────────────────────────

pub const BPF_LD: u16 = 0x00;
pub const BPF_LDX: u16 = 0x01;
pub const BPF_ST: u16 = 0x02;
pub const BPF_STX: u16 = 0x03;
pub const BPF_ALU: u16 = 0x04;
pub const BPF_JMP: u16 = 0x05;
pub const BPF_RET: u16 = 0x06;
pub const BPF_MISC: u16 = 0x07;

#[inline]
pub const fn bpf_class(code: u16) -> u16 {
    code & 0x07
}

// ── Size modifiers (BPF_LD / BPF_LDX / BPF_ST / BPF_STX) ─────────────────────

pub const BPF_W: u16 = 0x00; // 32-bit word
pub const BPF_H: u16 = 0x08; // 16-bit half-word
pub const BPF_B: u16 = 0x10; // 8-bit byte

#[inline]
pub const fn bpf_size(code: u16) -> u16 {
    code & 0x18
}

// ── Addressing modes ─────────────────────────────────────────────────────────

pub const BPF_IMM: u16 = 0x00;
pub const BPF_ABS: u16 = 0x20;
pub const BPF_IND: u16 = 0x40;
pub const BPF_MEM: u16 = 0x60;
pub const BPF_LEN: u16 = 0x80;
pub const BPF_MSH: u16 = 0xa0;

#[inline]
pub const fn bpf_mode(code: u16) -> u16 {
    code & 0xe0
}

// ── ALU / JMP ops ────────────────────────────────────────────────────────────

pub const BPF_ADD: u16 = 0x00;
pub const BPF_SUB: u16 = 0x10;
pub const BPF_MUL: u16 = 0x20;
pub const BPF_DIV: u16 = 0x30;
pub const BPF_OR: u16 = 0x40;
pub const BPF_AND: u16 = 0x50;
pub const BPF_LSH: u16 = 0x60;
pub const BPF_RSH: u16 = 0x70;
pub const BPF_NEG: u16 = 0x80;
pub const BPF_MOD: u16 = 0x90;
pub const BPF_XOR: u16 = 0xa0;

pub const BPF_JA: u16 = 0x00;
pub const BPF_JEQ: u16 = 0x10;
pub const BPF_JGT: u16 = 0x20;
pub const BPF_JGE: u16 = 0x30;
pub const BPF_JSET: u16 = 0x40;

#[inline]
pub const fn bpf_op(code: u16) -> u16 {
    code & 0xf0
}

// ── Source operand: K (immediate) vs X (register) ────────────────────────────

pub const BPF_K: u16 = 0x00;
pub const BPF_X: u16 = 0x08;

#[inline]
pub const fn bpf_src(code: u16) -> u16 {
    code & 0x08
}

// ── BPF_RET ──────────────────────────────────────────────────────────────────

pub const BPF_A: u16 = 0x10; // return A register

// ── BPF_MISC ─────────────────────────────────────────────────────────────────

pub const BPF_TAX: u16 = 0x00; // A → X
pub const BPF_TXA: u16 = 0x80; // X → A

// ── SockFilter (a.k.a. struct bpf_filter / struct sock_filter) ───────────────

/// One cBPF instruction.  Layout-compatible with Linux `struct sock_filter`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct SockFilter {
    pub code: u16,
    pub jt: u8,
    pub jf: u8,
    pub k: u32,
}

impl SockFilter {
    pub const fn stmt(code: u16, k: u32) -> Self {
        Self {
            code,
            jt: 0,
            jf: 0,
            k,
        }
    }
    pub const fn jump(code: u16, k: u32, jt: u8, jf: u8) -> Self {
        Self { code, jt, jf, k }
    }
}

/// Maximum legal BPF program length (Linux `BPF_MAXINSNS`).
pub const BPF_MAXINSNS: usize = 4096;

/// Number of M[] scratch slots (Linux uses 16).
pub const BPF_MEMWORDS: usize = 16;

// ── Interpreter ──────────────────────────────────────────────────────────────

/// Return value from a BPF-program run.  Either a 32-bit value (which seccomp
/// reinterprets as a `SECCOMP_RET_*` action) or a hard error (program
/// malformed at runtime).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BpfRunResult {
    Value(u32),
    /// Division-by-zero or out-of-bounds packet load.  Mirrors Linux's
    /// behaviour where the program is treated as returning 0 for
    /// out-of-bounds loads, but seccomp explicitly checks bounds via
    /// `seccomp_data` of fixed size.
    InvalidLoad,
    InvalidJump,
    InvalidOpcode,
    DivByZero,
}

#[derive(Clone, Copy)]
enum LoadEndian {
    Big,
    Native,
}

/// Run a cBPF program over a flat input buffer (e.g. packet bytes).
///
/// `packet` is the byte buffer the program sees through `BPF_LD | BPF_ABS`.
/// Loads follow classic packet-filter byte order.
///
/// On a successful `BPF_RET`, returns `BpfRunResult::Value(k_or_a)`.
pub fn bpf_run_filter(prog: &[SockFilter], packet: &[u8]) -> BpfRunResult {
    bpf_run_filter_inner(prog, packet, LoadEndian::Big)
}

/// Run a cBPF program over a native-endian data record.
///
/// Seccomp filters operate on `struct seccomp_data`, whose fields are compared
/// in host byte order rather than network byte order.
pub fn bpf_run_filter_native(prog: &[SockFilter], packet: &[u8]) -> BpfRunResult {
    bpf_run_filter_inner(prog, packet, LoadEndian::Native)
}

fn bpf_run_filter_inner(
    prog: &[SockFilter],
    packet: &[u8],
    load_endian: LoadEndian,
) -> BpfRunResult {
    if prog.is_empty() || prog.len() > BPF_MAXINSNS {
        return BpfRunResult::InvalidOpcode;
    }

    let mut a: u32 = 0;
    let mut x: u32 = 0;
    let mut mem: [u32; BPF_MEMWORDS] = [0; BPF_MEMWORDS];

    let mut pc: usize = 0;
    while pc < prog.len() {
        let ins = prog[pc];
        let code = ins.code;
        let class = bpf_class(code);
        match class {
            // ── BPF_LD (loads into A) ────────────────────────────────────────
            BPF_LD => match bpf_mode(code) {
                BPF_IMM => a = ins.k,
                BPF_LEN => a = packet.len() as u32,
                BPF_ABS => {
                    let off = ins.k as usize;
                    let v = match bpf_size(code) {
                        BPF_W => load_w(packet, off, load_endian),
                        BPF_H => load_h(packet, off, load_endian),
                        BPF_B => load_b(packet, off),
                        _ => return BpfRunResult::InvalidOpcode,
                    };
                    match v {
                        Some(val) => a = val,
                        None => return BpfRunResult::InvalidLoad,
                    }
                }
                BPF_IND => {
                    let off = (ins.k as i64 + x as i64) as usize;
                    let v = match bpf_size(code) {
                        BPF_W => load_w(packet, off, load_endian),
                        BPF_H => load_h(packet, off, load_endian),
                        BPF_B => load_b(packet, off),
                        _ => return BpfRunResult::InvalidOpcode,
                    };
                    match v {
                        Some(val) => a = val,
                        None => return BpfRunResult::InvalidLoad,
                    }
                }
                BPF_MEM => {
                    let m = ins.k as usize;
                    if m >= BPF_MEMWORDS {
                        return BpfRunResult::InvalidLoad;
                    }
                    a = mem[m];
                }
                _ => return BpfRunResult::InvalidOpcode,
            },
            // ── BPF_LDX (loads into X) ───────────────────────────────────────
            BPF_LDX => {
                match bpf_mode(code) {
                    BPF_IMM => x = ins.k,
                    BPF_LEN => x = packet.len() as u32,
                    BPF_MEM => {
                        let m = ins.k as usize;
                        if m >= BPF_MEMWORDS {
                            return BpfRunResult::InvalidLoad;
                        }
                        x = mem[m];
                    }
                    BPF_MSH => {
                        // X = (P[k] & 0xf) << 2 — IPv4 header-len helper.
                        let off = ins.k as usize;
                        match load_b(packet, off) {
                            Some(v) => x = (v & 0xf) << 2,
                            None => return BpfRunResult::InvalidLoad,
                        }
                    }
                    _ => return BpfRunResult::InvalidOpcode,
                }
            }
            // ── BPF_ST / BPF_STX (store A or X to M[k]) ──────────────────────
            BPF_ST => {
                let m = ins.k as usize;
                if m >= BPF_MEMWORDS {
                    return BpfRunResult::InvalidLoad;
                }
                mem[m] = a;
            }
            BPF_STX => {
                let m = ins.k as usize;
                if m >= BPF_MEMWORDS {
                    return BpfRunResult::InvalidLoad;
                }
                mem[m] = x;
            }
            // ── BPF_ALU ──────────────────────────────────────────────────────
            BPF_ALU => {
                let src = if bpf_src(code) == BPF_X { x } else { ins.k };
                a = match bpf_op(code) {
                    BPF_ADD => a.wrapping_add(src),
                    BPF_SUB => a.wrapping_sub(src),
                    BPF_MUL => a.wrapping_mul(src),
                    BPF_DIV => {
                        if src == 0 {
                            return BpfRunResult::DivByZero;
                        }
                        a / src
                    }
                    BPF_OR => a | src,
                    BPF_AND => a & src,
                    BPF_LSH => a.wrapping_shl(src),
                    BPF_RSH => a.wrapping_shr(src),
                    BPF_NEG => 0u32.wrapping_sub(a),
                    BPF_MOD => {
                        if src == 0 {
                            return BpfRunResult::DivByZero;
                        }
                        a % src
                    }
                    BPF_XOR => a ^ src,
                    _ => return BpfRunResult::InvalidOpcode,
                };
            }
            // ── BPF_JMP ──────────────────────────────────────────────────────
            BPF_JMP => {
                let op = bpf_op(code);
                if op == BPF_JA {
                    let target = pc.wrapping_add(1).wrapping_add(ins.k as usize);
                    if target >= prog.len() {
                        return BpfRunResult::InvalidJump;
                    }
                    pc = target;
                    continue;
                }
                let src = if bpf_src(code) == BPF_X { x } else { ins.k };
                let cond = match op {
                    BPF_JEQ => a == src,
                    BPF_JGT => a > src,
                    BPF_JGE => a >= src,
                    BPF_JSET => (a & src) != 0,
                    _ => return BpfRunResult::InvalidOpcode,
                };
                let off = if cond {
                    ins.jt as usize
                } else {
                    ins.jf as usize
                };
                let target = pc.wrapping_add(1).wrapping_add(off);
                if target >= prog.len() {
                    return BpfRunResult::InvalidJump;
                }
                pc = target;
                continue;
            }
            // ── BPF_RET ──────────────────────────────────────────────────────
            // BPF_RET uses bits 0x18 (NOT 0x08 as for ALU/JMP src) to choose
            // between K=0x00, X=0x08, A=0x10.
            BPF_RET => {
                let v = match code & 0x18 {
                    BPF_K => ins.k,
                    BPF_X => x,
                    BPF_A => a,
                    _ => return BpfRunResult::InvalidOpcode,
                };
                return BpfRunResult::Value(v);
            }
            // ── BPF_MISC ─────────────────────────────────────────────────────
            // BPF_MISC uses the high bit (0x80) to distinguish TAX (0) and TXA (0x80).
            BPF_MISC => {
                match code & 0x80 {
                    0 => x = a,    // BPF_TAX
                    0x80 => a = x, // BPF_TXA
                    _ => return BpfRunResult::InvalidOpcode,
                }
            }
            _ => return BpfRunResult::InvalidOpcode,
        }
        pc += 1;
    }

    // Falling off the end without RET is a malformed program.
    BpfRunResult::InvalidOpcode
}

// ── Load helpers ─────────────────────────────────────────────────────────────

#[inline]
fn load_w(buf: &[u8], off: usize, endian: LoadEndian) -> Option<u32> {
    if off.checked_add(4)? > buf.len() {
        return None;
    }
    let bytes = [buf[off], buf[off + 1], buf[off + 2], buf[off + 3]];
    Some(match endian {
        LoadEndian::Big => u32::from_be_bytes(bytes),
        LoadEndian::Native => u32::from_ne_bytes(bytes),
    })
}

#[inline]
fn load_h(buf: &[u8], off: usize, endian: LoadEndian) -> Option<u32> {
    if off.checked_add(2)? > buf.len() {
        return None;
    }
    let bytes = [buf[off], buf[off + 1]];
    Some(match endian {
        LoadEndian::Big => u16::from_be_bytes(bytes) as u32,
        LoadEndian::Native => u16::from_ne_bytes(bytes) as u32,
    })
}

#[inline]
fn load_b(buf: &[u8], off: usize) -> Option<u32> {
    if off >= buf.len() {
        return None;
    }
    Some(buf[off] as u32)
}

// ── Convenience: little-endian field load (seccomp_data is host-endian) ──────

/// Load a 32-bit value from `buf[off..off+4]` interpreting bytes as
/// host-endian (little-endian on x86_64).  Used by callers that want to
/// pre-stage a buffer with native field layout instead of network byte order.
#[inline]
pub fn load_le_u32(buf: &[u8], off: usize) -> Option<u32> {
    if off.checked_add(4)? > buf.len() {
        return None;
    }
    Some(u32::from_le_bytes([
        buf[off],
        buf[off + 1],
        buf[off + 2],
        buf[off + 3],
    ]))
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcode_decoders() {
        assert_eq!(bpf_class(BPF_LD | BPF_ABS | BPF_W), BPF_LD);
        assert_eq!(bpf_size(BPF_LD | BPF_ABS | BPF_W), BPF_W);
        assert_eq!(bpf_mode(BPF_LD | BPF_ABS | BPF_W), BPF_ABS);
        assert_eq!(bpf_op(BPF_ALU | BPF_K | BPF_ADD), BPF_ADD);
        assert_eq!(bpf_src(BPF_ALU | BPF_X | BPF_ADD), BPF_X);
    }

    #[test]
    fn ret_immediate() {
        let prog = [SockFilter::stmt(BPF_RET | BPF_K, 0xdeadbeef)];
        assert_eq!(bpf_run_filter(&prog, &[]), BpfRunResult::Value(0xdeadbeef));
    }

    #[test]
    fn alu_add_immediate() {
        let prog = [
            SockFilter::stmt(BPF_LD | BPF_IMM, 7),
            SockFilter::stmt(BPF_ALU | BPF_K | BPF_ADD, 35),
            SockFilter::stmt(BPF_RET | BPF_A, 0),
        ];
        assert_eq!(bpf_run_filter(&prog, &[]), BpfRunResult::Value(42));
    }

    #[test]
    fn alu_div_by_zero() {
        let prog = [
            SockFilter::stmt(BPF_LD | BPF_IMM, 100),
            SockFilter::stmt(BPF_ALU | BPF_K | BPF_DIV, 0),
            SockFilter::stmt(BPF_RET | BPF_A, 0),
        ];
        assert_eq!(bpf_run_filter(&prog, &[]), BpfRunResult::DivByZero);
    }

    #[test]
    fn jeq_taken() {
        // if A == 5 jump to RET 1 else RET 2
        let prog = [
            SockFilter::stmt(BPF_LD | BPF_IMM, 5),
            SockFilter::jump(BPF_JMP | BPF_K | BPF_JEQ, 5, 0, 1),
            SockFilter::stmt(BPF_RET | BPF_K, 1),
            SockFilter::stmt(BPF_RET | BPF_K, 2),
        ];
        assert_eq!(bpf_run_filter(&prog, &[]), BpfRunResult::Value(1));
    }

    #[test]
    fn jeq_not_taken() {
        let prog = [
            SockFilter::stmt(BPF_LD | BPF_IMM, 4),
            SockFilter::jump(BPF_JMP | BPF_K | BPF_JEQ, 5, 0, 1),
            SockFilter::stmt(BPF_RET | BPF_K, 1),
            SockFilter::stmt(BPF_RET | BPF_K, 2),
        ];
        assert_eq!(bpf_run_filter(&prog, &[]), BpfRunResult::Value(2));
    }

    #[test]
    fn ld_abs_word_big_endian() {
        // Buffer is interpreted big-endian by BPF_LD | BPF_ABS.
        let buf = [0xde, 0xad, 0xbe, 0xef, 0x00, 0x00, 0x00, 0x00];
        let prog = [
            SockFilter::stmt(BPF_LD | BPF_ABS | BPF_W, 0),
            SockFilter::stmt(BPF_RET | BPF_A, 0),
        ];
        assert_eq!(bpf_run_filter(&prog, &buf), BpfRunResult::Value(0xdeadbeef));
    }

    #[test]
    fn ld_abs_word_native_endian() {
        let mut buf = [0u8; 8];
        buf[0..4].copy_from_slice(&0xc000_003e_u32.to_ne_bytes());
        let prog = [
            SockFilter::stmt(BPF_LD | BPF_ABS | BPF_W, 0),
            SockFilter::stmt(BPF_RET | BPF_A, 0),
        ];
        assert_eq!(
            bpf_run_filter_native(&prog, &buf),
            BpfRunResult::Value(0xc000_003e)
        );
    }

    #[test]
    fn ld_abs_out_of_bounds() {
        let buf = [0u8; 4];
        let prog = [
            SockFilter::stmt(BPF_LD | BPF_ABS | BPF_W, 100),
            SockFilter::stmt(BPF_RET | BPF_A, 0),
        ];
        assert_eq!(bpf_run_filter(&prog, &buf), BpfRunResult::InvalidLoad);
    }

    #[test]
    fn maxinsns_bound_enforced() {
        let big: alloc::vec::Vec<SockFilter> = (0..(BPF_MAXINSNS + 1))
            .map(|_| SockFilter::stmt(BPF_RET | BPF_K, 0))
            .collect();
        assert_eq!(bpf_run_filter(&big, &[]), BpfRunResult::InvalidOpcode);
    }
}

extern crate alloc;
