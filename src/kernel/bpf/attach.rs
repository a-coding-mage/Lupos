//! linux-parity: partial
//! linux-source: vendor/linux/kernel/bpf
//! Attach an eBPF program to a tracepoint (M62 → M63 bridge).
//!
//! Linux exposes this via `BPF_PROG_TYPE_TRACEPOINT` + `BPF_LINK_CREATE`
//! or `perf_event_open` with `attr.type = PERF_TYPE_TRACEPOINT`.  M63
//! ships a kernel-side direct-attach API only; userspace plumbing follows.

use core::sync::atomic::{AtomicUsize, Ordering};

use super::interp;
use super::syscall::find_prog;

/// Single global attach slot for M63 (one prog at a time).
static ATTACHED_PROG_FD: AtomicUsize = AtomicUsize::new(0);

pub fn attach_to_tracepoint(prog_fd: i32) -> Result<(), i32> {
    if find_prog(prog_fd).is_none() {
        return Err(-9); // EBADF
    }
    ATTACHED_PROG_FD.store(prog_fd as usize, Ordering::Release);
    Ok(())
}

pub fn detach() {
    ATTACHED_PROG_FD.store(0, Ordering::Release);
}

/// Run the attached program (called from a tracepoint probe).
/// Returns the program's r0 retval, or 0 if no attachment.
pub fn run_attached(ctx_r1: u64) -> u64 {
    let fd = ATTACHED_PROG_FD.load(Ordering::Acquire);
    if fd == 0 {
        return 0;
    }
    if let Some(prog) = find_prog(fd as i32) {
        return interp::run(&prog.insns, ctx_r1);
    }
    0
}
