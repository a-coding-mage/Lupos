//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/events/amd/ibs.c
//! test-origin: linux:vendor/linux/arch/x86/events/amd/ibs.c
//! AMD Instruction Based Sampling model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/events/amd/ibs.c

use crate::include::uapi::errno::EOPNOTSUPP;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IbsOp {
    Fetch,
    Op,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IbsControl {
    pub op: IbsOp,
    pub max_count: u16,
    pub randomize: bool,
}

pub const IBS_FETCH_CTL_ENABLE: u64 = 1 << 48;
pub const IBS_OP_CTL_ENABLE: u64 = 1 << 17;

pub const fn ibs_available(cpuid_ibs: bool, mca_enabled: bool) -> bool {
    cpuid_ibs && mca_enabled
}

pub const fn encode_ibs_control(control: IbsControl) -> u64 {
    let enable = match control.op {
        IbsOp::Fetch => IBS_FETCH_CTL_ENABLE,
        IbsOp::Op => IBS_OP_CTL_ENABLE,
    };
    let randomize = if control.randomize { 1u64 << 57 } else { 0 };
    enable | randomize | control.max_count as u64
}

pub const fn ibs_programming_errno() -> i32 {
    EOPNOTSUPP
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ibs_encoding_selects_fetch_or_op_enable_bit() {
        assert_eq!(
            encode_ibs_control(IbsControl {
                op: IbsOp::Fetch,
                max_count: 7,
                randomize: false,
            }),
            IBS_FETCH_CTL_ENABLE | 7
        );
        assert_eq!(
            encode_ibs_control(IbsControl {
                op: IbsOp::Op,
                max_count: 1,
                randomize: true,
            }),
            IBS_OP_CTL_ENABLE | (1u64 << 57) | 1
        );
    }
}
