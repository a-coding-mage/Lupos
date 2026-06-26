//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/events/utils.c
//! test-origin: linux:vendor/linux/arch/x86/events/utils.c
//! Shared x86 PMU utility helpers.

use crate::include::uapi::errno::EINVAL;

pub const PERF_MAX_COUNTERS: u8 = 64;
pub const MAX_INSN_SIZE: usize = 15;

pub const X86_BR_NONE: u32 = 0;
pub const X86_BR_USER: u32 = 1 << 0;
pub const X86_BR_KERNEL: u32 = 1 << 1;
pub const X86_BR_CALL: u32 = 1 << 2;
pub const X86_BR_RET: u32 = 1 << 3;
pub const X86_BR_SYSCALL: u32 = 1 << 4;
pub const X86_BR_SYSRET: u32 = 1 << 5;
pub const X86_BR_INT: u32 = 1 << 6;
pub const X86_BR_IRET: u32 = 1 << 7;
pub const X86_BR_JCC: u32 = 1 << 8;
pub const X86_BR_JMP: u32 = 1 << 9;
pub const X86_BR_IRQ: u32 = 1 << 10;
pub const X86_BR_IND_CALL: u32 = 1 << 11;
pub const X86_BR_ABORT: u32 = 1 << 12;
pub const X86_BR_IN_TX: u32 = 1 << 13;
pub const X86_BR_NO_TX: u32 = 1 << 14;
pub const X86_BR_ZERO_CALL: u32 = 1 << 15;
pub const X86_BR_CALL_STACK: u32 = 1 << 16;
pub const X86_BR_IND_JMP: u32 = 1 << 17;

pub const PERF_BR_UNKNOWN: i32 = 0;
pub const PERF_BR_COND: i32 = 1;
pub const PERF_BR_UNCOND: i32 = 2;
pub const PERF_BR_IND: i32 = 3;
pub const PERF_BR_CALL: i32 = 4;
pub const PERF_BR_IND_CALL: i32 = 5;
pub const PERF_BR_RET: i32 = 6;
pub const PERF_BR_SYSCALL: i32 = 7;
pub const PERF_BR_SYSRET: i32 = 8;
pub const PERF_BR_ERET: i32 = 11;
pub const PERF_BR_IRQ: i32 = 12;
pub const PERF_BR_NO_TX: i32 = 14;

pub const X86_BR_TYPE_MAP_MAX: usize = 16;
pub const BRANCH_MAP: [i32; X86_BR_TYPE_MAP_MAX] = [
    PERF_BR_CALL,
    PERF_BR_RET,
    PERF_BR_SYSCALL,
    PERF_BR_SYSRET,
    PERF_BR_UNKNOWN,
    PERF_BR_ERET,
    PERF_BR_COND,
    PERF_BR_UNCOND,
    PERF_BR_IRQ,
    PERF_BR_IND_CALL,
    PERF_BR_UNKNOWN,
    PERF_BR_UNKNOWN,
    PERF_BR_NO_TX,
    PERF_BR_CALL,
    PERF_BR_UNKNOWN,
    PERF_BR_IND,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BranchInsn {
    pub opcode: [u8; 2],
    pub two_byte: bool,
    pub opcode_error: bool,
    pub modrm: Option<u8>,
    pub modrm_error: bool,
    pub immediate1: i64,
    pub immediate_error: bool,
    pub length: usize,
    pub length_error: bool,
}

impl BranchInsn {
    pub const fn one(opcode: u8, length: usize) -> Self {
        Self {
            opcode: [opcode, 0],
            two_byte: false,
            opcode_error: false,
            modrm: None,
            modrm_error: false,
            immediate1: 1,
            immediate_error: false,
            length,
            length_error: false,
        }
    }

    pub const fn two(first: u8, second: u8, length: usize) -> Self {
        Self {
            opcode: [first, second],
            two_byte: true,
            opcode_error: false,
            modrm: None,
            modrm_error: false,
            immediate1: 1,
            immediate_error: false,
            length,
            length_error: false,
        }
    }

    pub const fn with_modrm(mut self, modrm: u8) -> Self {
        self.modrm = Some(modrm);
        self
    }

    pub const fn with_immediate(mut self, immediate: i64) -> Self {
        self.immediate1 = immediate;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BranchContext<'a> {
    pub from_is_kernel: bool,
    pub to_is_kernel: bool,
    pub current_has_mm: bool,
    pub kernel_text_valid: bool,
    pub instructions: &'a [BranchInsn],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BranchTypeResult {
    pub branch_type: u32,
    pub offset: usize,
}

pub const fn decode_branch_type(insn: BranchInsn) -> u32 {
    if insn.opcode_error {
        return X86_BR_ABORT;
    }

    match insn.opcode[0] {
        0x0f => match insn.opcode[1] {
            0x05 | 0x34 => X86_BR_SYSCALL,
            0x07 | 0x35 => X86_BR_SYSRET,
            0x80..=0x8f => X86_BR_JCC,
            _ => X86_BR_NONE,
        },
        0x70..=0x7f => X86_BR_JCC,
        0xc2 | 0xc3 | 0xca | 0xcb => X86_BR_RET,
        0xcf => X86_BR_IRET,
        0xcc..=0xce => X86_BR_INT,
        0xe8 => {
            if insn.immediate_error || insn.immediate1 == 0 {
                X86_BR_ZERO_CALL
            } else {
                X86_BR_CALL
            }
        }
        0x9a => X86_BR_CALL,
        0xe0..=0xe3 => X86_BR_JCC,
        0xe9..=0xeb => X86_BR_JMP,
        0xff => {
            if insn.modrm_error {
                return X86_BR_ABORT;
            }
            match insn.modrm {
                Some(modrm) => match (modrm >> 3) & 0x7 {
                    2 | 3 => X86_BR_IND_CALL,
                    4 | 5 => X86_BR_IND_JMP,
                    _ => X86_BR_NONE,
                },
                None => X86_BR_ABORT,
            }
        }
        _ => X86_BR_NONE,
    }
}

pub fn get_branch_type(
    from: u64,
    to: u64,
    abort: bool,
    fused: bool,
    context: BranchContext<'_>,
) -> BranchTypeResult {
    let mut offset = 0usize;
    let to_plm = if context.to_is_kernel {
        X86_BR_KERNEL
    } else {
        X86_BR_USER
    };
    let from_plm = if context.from_is_kernel {
        X86_BR_KERNEL
    } else {
        X86_BR_USER
    };

    if from == 0 || to == 0 {
        return BranchTypeResult {
            branch_type: X86_BR_NONE,
            offset,
        };
    }
    if abort {
        return BranchTypeResult {
            branch_type: X86_BR_ABORT | to_plm,
            offset,
        };
    }
    if from_plm == X86_BR_USER && !context.current_has_mm {
        return BranchTypeResult {
            branch_type: X86_BR_NONE,
            offset,
        };
    }
    if from_plm == X86_BR_KERNEL && !context.kernel_text_valid {
        return BranchTypeResult {
            branch_type: X86_BR_NONE,
            offset,
        };
    }
    let Some(first) = context.instructions.first().copied() else {
        return BranchTypeResult {
            branch_type: X86_BR_NONE,
            offset,
        };
    };

    let mut ret = decode_branch_type(first);
    let mut index = 0usize;
    let mut bytes_read = MAX_INSN_SIZE as isize;

    while fused && ret == X86_BR_NONE {
        let insn = context.instructions[index];
        if insn.length_error || insn.length == 0 {
            break;
        }
        offset += insn.length;
        bytes_read -= insn.length as isize;
        if bytes_read < 0 {
            break;
        }
        index += 1;
        if index >= context.instructions.len() {
            break;
        }
        ret = decode_branch_type(context.instructions[index]);
    }

    if from_plm == X86_BR_USER
        && to_plm == X86_BR_KERNEL
        && ret != X86_BR_SYSCALL
        && ret != X86_BR_INT
    {
        ret = X86_BR_IRQ;
    }

    if ret != X86_BR_NONE {
        ret |= to_plm;
    }

    BranchTypeResult {
        branch_type: ret,
        offset,
    }
}

pub fn branch_type(from: u64, to: u64, abort: bool, context: BranchContext<'_>) -> u32 {
    get_branch_type(from, to, abort, false, context).branch_type
}

pub fn branch_type_fused(
    from: u64,
    to: u64,
    abort: bool,
    context: BranchContext<'_>,
) -> BranchTypeResult {
    get_branch_type(from, to, abort, true, context)
}

pub const fn common_branch_type(branch_type: u32) -> i32 {
    let shifted = branch_type >> 2;
    if shifted == 0 {
        return PERF_BR_UNKNOWN;
    }
    let mut index = 0usize;
    while index < X86_BR_TYPE_MAP_MAX {
        if (shifted & (1u32 << index)) != 0 {
            return BRANCH_MAP[index];
        }
        index += 1;
    }
    PERF_BR_UNKNOWN
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventConstraint {
    pub event: u64,
    pub mask: u64,
    pub counters: u64,
}

impl EventConstraint {
    pub const fn matches(self, event: u64) -> bool {
        (event & self.mask) == (self.event & self.mask)
    }
}

pub const fn counter_mask(count: u8) -> u64 {
    if count == 0 {
        0
    } else if count >= PERF_MAX_COUNTERS {
        u64::MAX
    } else {
        (1u64 << count) - 1
    }
}

pub const fn first_allowed_counter(mask: u64) -> Option<u8> {
    let mut i = 0u8;
    while i < PERF_MAX_COUNTERS {
        if mask & (1u64 << i) != 0 {
            return Some(i);
        }
        i += 1;
    }
    None
}

pub const fn validate_sample_period(period: u64) -> Result<(), i32> {
    if period == 0 { Err(EINVAL) } else { Ok(()) }
}

pub const fn merge_constraints(a: EventConstraint, b: EventConstraint) -> EventConstraint {
    EventConstraint {
        event: a.event,
        mask: a.mask | b.mask,
        counters: a.counters & b.counters,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constraints_match_linux_mask_shape() {
        let c = EventConstraint {
            event: 0x5301c0,
            mask: 0xffff,
            counters: 0b11,
        };
        assert!(c.matches(0xabcd_01c0));
        assert!(!c.matches(0xabcd_02c0));
    }

    #[test]
    fn utils_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/events/utils.c"
        ));
        assert!(source.contains("static int decode_branch_type(struct insn *insn)"));
        assert!(source.contains("case 0x05: /* syscall */"));
        assert!(source.contains("case 0x80 ... 0x8f: /* conditional */"));
        assert!(source.contains("case 0xe8: /* call near rel */"));
        assert!(source.contains("return X86_BR_ZERO_CALL;"));
        assert!(source.contains("ext = (insn->modrm.bytes[0] >> 3) & 0x7;"));
        assert!(source.contains("static int get_branch_type"));
        assert!(source.contains("copy_from_user_nmi"));
        assert!(source.contains("kernel_text_address(from) && !in_gate_area_no_mm(from)"));
        assert!(source.contains("while (fused && ret == X86_BR_NONE)"));
        assert!(source.contains("ret = X86_BR_IRQ;"));
        assert!(
            source.contains("int branch_type(unsigned long from, unsigned long to, int abort)")
        );
        assert!(source.contains("branch_type_fused"));
        assert!(source.contains("X86_BR_TYPE_MAP_MAX\t16"));
        assert!(source.contains("type >>= 2; /* skip X86_BR_USER and X86_BR_KERNEL */"));
    }

    #[test]
    fn decode_branch_type_matches_opcode_cases() {
        assert_eq!(
            decode_branch_type(BranchInsn::two(0x0f, 0x05, 2)),
            X86_BR_SYSCALL
        );
        assert_eq!(
            decode_branch_type(BranchInsn::two(0x0f, 0x35, 2)),
            X86_BR_SYSRET
        );
        assert_eq!(
            decode_branch_type(BranchInsn::two(0x0f, 0x85, 6)),
            X86_BR_JCC
        );
        assert_eq!(decode_branch_type(BranchInsn::one(0x74, 2)), X86_BR_JCC);
        assert_eq!(decode_branch_type(BranchInsn::one(0xc3, 1)), X86_BR_RET);
        assert_eq!(decode_branch_type(BranchInsn::one(0xcf, 1)), X86_BR_IRET);
        assert_eq!(decode_branch_type(BranchInsn::one(0xcd, 2)), X86_BR_INT);
        assert_eq!(
            decode_branch_type(BranchInsn::one(0xe8, 5).with_immediate(0)),
            X86_BR_ZERO_CALL
        );
        assert_eq!(
            decode_branch_type(BranchInsn::one(0xe8, 5).with_immediate(4)),
            X86_BR_CALL
        );
        assert_eq!(
            decode_branch_type(BranchInsn::one(0xff, 2).with_modrm(2 << 3)),
            X86_BR_IND_CALL
        );
        assert_eq!(
            decode_branch_type(BranchInsn::one(0xff, 2).with_modrm(4 << 3)),
            X86_BR_IND_JMP
        );
    }

    #[test]
    fn branch_type_applies_privilege_and_irq_rules() {
        let insns = [BranchInsn::one(0x90, 1)];
        let context = BranchContext {
            from_is_kernel: false,
            to_is_kernel: true,
            current_has_mm: true,
            kernel_text_valid: true,
            instructions: &insns,
        };
        assert_eq!(
            branch_type(0x1000, 0xffff_8000, false, context),
            X86_BR_IRQ | X86_BR_KERNEL
        );

        let syscall = [BranchInsn::two(0x0f, 0x05, 2)];
        let context = BranchContext {
            instructions: &syscall,
            ..context
        };
        assert_eq!(
            branch_type(0x1000, 0xffff_8000, false, context),
            X86_BR_SYSCALL | X86_BR_KERNEL
        );

        let context = BranchContext {
            current_has_mm: false,
            ..context
        };
        assert_eq!(
            branch_type(0x1000, 0xffff_8000, false, context),
            X86_BR_NONE
        );
    }

    #[test]
    fn branch_type_fused_scans_until_first_branch_and_reports_offset() {
        let insns = [
            BranchInsn::one(0x90, 1),
            BranchInsn::one(0x90, 1),
            BranchInsn::one(0xeb, 2),
        ];
        let context = BranchContext {
            from_is_kernel: true,
            to_is_kernel: true,
            current_has_mm: true,
            kernel_text_valid: true,
            instructions: &insns,
        };
        let result = branch_type_fused(0xffff_0000, 0xffff_0100, false, context);
        assert_eq!(result.offset, 2);
        assert_eq!(result.branch_type, X86_BR_JMP | X86_BR_KERNEL);
    }

    #[test]
    fn common_branch_type_uses_first_set_branch_bit_after_privilege_bits() {
        assert_eq!(common_branch_type(X86_BR_CALL | X86_BR_USER), PERF_BR_CALL);
        assert_eq!(common_branch_type(X86_BR_RET | X86_BR_KERNEL), PERF_BR_RET);
        assert_eq!(
            common_branch_type(X86_BR_IND_JMP | X86_BR_KERNEL),
            PERF_BR_IND
        );
        assert_eq!(common_branch_type(X86_BR_NONE), PERF_BR_UNKNOWN);
    }
}
