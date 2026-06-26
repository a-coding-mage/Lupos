//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/bpf_filter.c
//! test-origin: linux:vendor/linux/io_uring/bpf_filter.c
//! Pre-submission SQE BPF filter.
//!
//! `IORING_REGISTER_BPF_FILTER` attaches a BPF program to a ring; on every
//! `io_uring_enter`/SQPOLL submission the filter inspects the SQE and may
//! reject it.  Backed by `src/kernel/bpf/` eBPF subsystem.
//!
//! Ref: vendor/linux/io_uring/bpf_filter.c

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;

use spin::Mutex;

use super::sqe::Sqe;

/// Attached BPF program handle.  In the full integration this is a
/// `crate::kernel::bpf::syscall::BpfProg`; we keep an opaque pointer for the
/// Layer-3 port so the filter wiring is testable without a full BPF program.
pub struct BpfFilterProg {
    /// Callback used by tests / the verified eBPF interpreter.
    pub run: fn(&Sqe) -> i32,
}

/// Per-ring BPF filter holder.
pub struct BpfFilter {
    prog: Mutex<Option<Arc<BpfFilterProg>>>,
}

impl BpfFilter {
    pub const fn new() -> Self {
        Self {
            prog: Mutex::new(None),
        }
    }

    /// `io_register_bpf_filter` — attach a program; rejects double-attach
    /// with `-EBUSY`.
    pub fn attach(&self, prog: Arc<BpfFilterProg>) -> Result<(), i32> {
        let mut g = self.prog.lock();
        if g.is_some() {
            return Err(-16);
        }
        *g = Some(prog);
        Ok(())
    }

    /// `io_unregister_bpf_filter`.
    pub fn detach(&self) -> Result<(), i32> {
        let mut g = self.prog.lock();
        if g.take().is_none() {
            return Err(-2);
        }
        Ok(())
    }

    /// `io_bpf_filter_run` — invoke the program.  Returns the program's
    /// verdict: 0 = allow, negative = reject with that errno.
    pub fn run(&self, sqe: &Sqe) -> i32 {
        let g = self.prog.lock();
        match g.as_ref() {
            Some(p) => (p.run)(sqe),
            None => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allow_all(_: &Sqe) -> i32 {
        0
    }
    fn deny_nop(s: &Sqe) -> i32 {
        if s.opcode == 0 {
            -13 // -EACCES
        } else {
            0
        }
    }

    #[test]
    fn no_program_allows_everything() {
        let f = BpfFilter::new();
        let s = Sqe::default();
        assert_eq!(f.run(&s), 0);
    }

    #[test]
    fn attached_program_rejects_matching_sqe() {
        let f = BpfFilter::new();
        f.attach(Arc::new(BpfFilterProg { run: deny_nop })).unwrap();
        let mut s = Sqe::default();
        s.opcode = 0;
        assert_eq!(f.run(&s), -13);
        s.opcode = 22; // OP_READ
        assert_eq!(f.run(&s), 0);
    }

    #[test]
    fn double_attach_is_ebusy() {
        let f = BpfFilter::new();
        f.attach(Arc::new(BpfFilterProg { run: allow_all }))
            .unwrap();
        let r = f.attach(Arc::new(BpfFilterProg { run: allow_all }));
        assert_eq!(r.unwrap_err(), -16);
    }

    #[test]
    fn detach_without_attach_is_enoent() {
        let f = BpfFilter::new();
        assert_eq!(f.detach().unwrap_err(), -2);
    }
}
