//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/um/ptrace_user.c
//! test-origin: linux:vendor/linux/arch/x86/um/ptrace_user.c
//! UML host ptrace register access wrappers.

pub const PTRACE_GETREGS: i32 = 12;
pub const PTRACE_SETREGS: i32 = 13;

pub trait PtraceHost {
    fn ptrace(&self, request: i32, pid: i64, regs: &mut [usize]) -> Result<(), i32>;
}

pub fn ptrace_getregs<H: PtraceHost>(host: &H, pid: i64, regs_out: &mut [usize]) -> i32 {
    match host.ptrace(PTRACE_GETREGS, pid, regs_out) {
        Ok(()) => 0,
        Err(errno) => -errno,
    }
}

pub fn ptrace_setregs<H: PtraceHost>(host: &H, pid: i64, regs: &mut [usize]) -> i32 {
    match host.ptrace(PTRACE_SETREGS, pid, regs) {
        Ok(()) => 0,
        Err(errno) => -errno,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::include::uapi::errno::EIO;
    use core::cell::Cell;

    struct StubPtrace {
        fail_errno: Option<i32>,
        last_request: Cell<i32>,
    }

    impl PtraceHost for StubPtrace {
        fn ptrace(&self, request: i32, _pid: i64, regs: &mut [usize]) -> Result<(), i32> {
            self.last_request.set(request);
            if let Some(errno) = self.fail_errno {
                return Err(errno);
            }
            if let Some(first) = regs.first_mut() {
                *first = 0xfeed;
            }
            Ok(())
        }
    }

    #[test]
    fn ptrace_wrappers_return_zero_or_negative_errno() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/um/ptrace_user.c"
        ));
        assert!(source.contains("ptrace(PTRACE_GETREGS, pid, 0, regs_out)"));
        assert!(source.contains("return -errno;"));

        let host = StubPtrace {
            fail_errno: None,
            last_request: Cell::new(0),
        };
        let mut regs = [0usize; 2];
        assert_eq!(ptrace_getregs(&host, 1, &mut regs), 0);
        assert_eq!(host.last_request.get(), PTRACE_GETREGS);
        assert_eq!(regs[0], 0xfeed);

        let failing = StubPtrace {
            fail_errno: Some(EIO),
            last_request: Cell::new(0),
        };
        assert_eq!(ptrace_setregs(&failing, 1, &mut regs), -EIO);
        assert_eq!(failing.last_request.get(), PTRACE_SETREGS);
    }
}
