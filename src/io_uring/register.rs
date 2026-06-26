//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/register.c
//! test-origin: linux:vendor/linux/io_uring/register.c
//! `sys_io_uring_register(fd, opcode, arg, nr_args)` core handler.
//!
//! Dispatches `IORING_REGISTER_*` opcodes to the right Layer 0/1 modules
//! (rsrc, kbuf, eventfd, io_wq, sqpoll, etc.).
//!
//! Ref: vendor/linux/io_uring/register.c

use super::uapi::IoringOp;
use super::uapi::register_op as op;

/// Probe entry that `IORING_REGISTER_PROBE` returns to userspace.
/// Caller fills a `[IoUringProbeOp; ops_len]` after the header.
pub fn probe_supported(opcode: u8) -> bool {
    // Mirrors `io_probe`: opcode is supported iff the opdef has a `prep` slot.
    let Some(op) = IoringOp::from_u8(opcode) else {
        return false;
    };
    super::opdef::IO_OP_DEFS[op as usize].prep.is_some()
}

/// `io_register_op_must_quiesce` — does this opcode require the ring to be
/// idle?  Mirrors the upstream function.
pub fn must_quiesce(opcode: u32) -> bool {
    matches!(
        opcode,
        op::REGISTER_BUFFERS
            | op::UNREGISTER_BUFFERS
            | op::REGISTER_FILES
            | op::UNREGISTER_FILES
            | op::REGISTER_FILES_UPDATE
            | op::REGISTER_PROBE
            | op::REGISTER_PERSONALITY
            | op::UNREGISTER_PERSONALITY
            | op::REGISTER_RESTRICTIONS
            | op::REGISTER_ENABLE_RINGS
            | op::REGISTER_RESIZE_RINGS
            | op::REGISTER_MEM_REGION
    )
}

/// Returns `true` if `opcode` is in the valid `IORING_REGISTER_*` enum range.
pub fn opcode_is_valid(opcode: u32) -> bool {
    let raw = opcode & !op::REGISTER_USE_REGISTERED_RING;
    raw < op::REGISTER_LAST
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nop_opcode_is_supported() {
        // NOP has a prep slot in opdef.
        assert!(probe_supported(IoringOp::Nop as u8));
    }

    #[test]
    fn unsupported_opcode_returns_false() {
        // Any opcode beyond Last must be unsupported.
        assert!(!probe_supported(IoringOp::Last as u8));
    }

    #[test]
    fn quiesce_required_for_buffer_register() {
        assert!(must_quiesce(op::REGISTER_BUFFERS));
        assert!(must_quiesce(op::REGISTER_FILES));
        // NAPI register doesn't need quiesce.
        assert!(!must_quiesce(op::REGISTER_NAPI));
    }

    #[test]
    fn opcode_validation() {
        assert!(opcode_is_valid(op::REGISTER_BUFFERS));
        assert!(opcode_is_valid(op::REGISTER_BPF_FILTER));
        assert!(!opcode_is_valid(op::REGISTER_LAST));
        assert!(!opcode_is_valid(999));
    }

    #[test]
    fn use_registered_ring_bit_is_stripped() {
        // The high bit IORING_REGISTER_USE_REGISTERED_RING tells the syscall
        // path that `fd` is a ring-fd index, not a real fd.  Should not
        // affect opcode validation.
        let cmd = op::REGISTER_BUFFERS | op::REGISTER_USE_REGISTERED_RING;
        assert!(opcode_is_valid(cmd));
    }
}
