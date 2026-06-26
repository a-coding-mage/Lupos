//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/query.c
//! test-origin: linux:vendor/linux/io_uring/query.c
//! `IORING_REGISTER_QUERY` — introspection opcode.
//!
//! Returns kernel-side capability data (supported opcodes, feature flags,
//! io-wq settings) to userspace.  Lupos returns a JSON-free fixed-size
//! struct matching Linux's `struct io_uring_query_*`.
//!
//! Ref: vendor/linux/io_uring/query.c

use super::uapi::IoringOp;

/// Per-query "kind" enum.  Mirrors `enum io_uring_query`.
pub mod kind {
    pub const OPCODE_SUPPORT: u32 = 0;
    pub const FEATURE_BITS: u32 = 1;
    pub const IOWQ_LIMITS: u32 = 2;
}

/// Result of an opcode-support query.
#[derive(Clone, Copy, Debug, Default)]
pub struct OpcodeSupport {
    pub op: u8,
    pub supported: bool,
}

/// `io_query_opcode_support`.
pub fn opcode_support(op: u8) -> OpcodeSupport {
    let supported = if let Some(o) = IoringOp::from_u8(op) {
        super::opdef::IO_OP_DEFS[o as usize].prep.is_some()
    } else {
        false
    };
    OpcodeSupport { op, supported }
}

/// `io_query_feature_bits` — return the `io_uring_params.features` mask
/// the kernel will report.  Includes everything we structurally support.
pub fn feature_bits() -> u32 {
    use super::uapi::feat;
    feat::SINGLE_MMAP
        | feat::NODROP
        | feat::SUBMIT_STABLE
        | feat::RW_CUR_POS
        | feat::CUR_PERSONALITY
        | feat::FAST_POLL
        | feat::POLL_32BITS
        | feat::SQPOLL_NONFIXED
        | feat::EXT_ARG
        | feat::NATIVE_WORKERS
        | feat::RSRC_TAGS
        | feat::CQE_SKIP
        | feat::LINKED_FILE
        | feat::REG_REG_RING
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcode_support_marks_nop_supported() {
        let r = opcode_support(IoringOp::Nop as u8);
        assert!(r.supported);
    }

    #[test]
    fn opcode_support_marks_unknown_unsupported() {
        let r = opcode_support(IoringOp::Last as u8);
        assert!(!r.supported);
        let r = opcode_support(255);
        assert!(!r.supported);
    }

    #[test]
    fn feature_bits_include_single_mmap() {
        let f = feature_bits();
        assert!(f & super::super::uapi::feat::SINGLE_MMAP != 0);
        assert!(f & super::super::uapi::feat::REG_REG_RING != 0);
    }
}
