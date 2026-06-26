//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/um/os-Linux/registers.c
//! test-origin: linux:vendor/linux/arch/x86/um/os-Linux/registers.c
//! UML host ptrace register-set probing.

use crate::include::uapi::errno::{ENODEV, ENOMEM};

pub const UML_HOST_FP_PROBE_SIZE: usize = 2 * 1024 * 1024;
pub const NT_X86_XSTATE: u32 = 0x202;
pub const NT_PRFPREG: u32 = 2;
pub const NT_PRXFPREG: u32 = 0x46e62b7f;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostReg {
    Ip,
    Sp,
    Bp,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HostJmpRegs {
    pub ip: usize,
    pub sp: usize,
    pub bp: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FpRegsetProbe {
    pub regset: u32,
    pub host_fp_size: usize,
}

pub const fn ptrace_ret(ptrace_result: i32, errno: i32) -> i32 {
    if ptrace_result < 0 { -errno } else { 0 }
}

pub const fn arch_init_registers_probe(
    mmap_ok: bool,
    xstate_result: Result<usize, i32>,
    fallback_result: Result<usize, i32>,
    is_x86_32: bool,
) -> Result<FpRegsetProbe, i32> {
    if !mmap_ok {
        return Err(ENOMEM);
    }
    match xstate_result {
        Ok(size) => Ok(FpRegsetProbe {
            regset: NT_X86_XSTATE,
            host_fp_size: size,
        }),
        Err(err) if err == -ENODEV => match fallback_result {
            Ok(size) => Ok(FpRegsetProbe {
                regset: if is_x86_32 { NT_PRXFPREG } else { NT_PRFPREG },
                host_fp_size: size,
            }),
            Err(err) => Err(err),
        },
        Err(err) => Err(err),
    }
}

pub const fn get_thread_reg(reg: HostReg, regs: HostJmpRegs) -> usize {
    match reg {
        HostReg::Ip => regs.ip,
        HostReg::Sp => regs.sp,
        HostReg::Bp => regs.bp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uml_register_probe_matches_linux_ptrace_fallbacks() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/um/os-Linux/registers.c"
        ));
        assert!(source.contains("static unsigned long ptrace_regset;"));
        assert!(source.contains("unsigned long host_fp_size;"));
        assert!(source.contains("PTRACE_GETREGSET"));
        assert!(source.contains("PTRACE_SETREGSET"));
        assert!(source.contains("return -errno;"));
        assert!(source.contains("iov.iov_len = 2 * 1024 * 1024"));
        assert!(source.contains("ptrace_regset = NT_X86_XSTATE;"));
        assert!(source.contains("if (ret == -ENODEV)"));
        assert!(source.contains("ptrace_regset = NT_PRXFPREG;"));
        assert!(source.contains("ptrace_regset = NT_PRFPREG;"));
        assert!(source.contains("host_fp_size = iov.iov_len;"));
        assert!(source.contains("case HOST_IP:"));
        assert!(source.contains("case HOST_SP:"));
        assert!(source.contains("case HOST_BP:"));

        assert_eq!(ptrace_ret(-1, ENODEV), -ENODEV);
        assert_eq!(ptrace_ret(0, ENODEV), 0);
        assert_eq!(
            arch_init_registers_probe(true, Ok(4096), Err(-ENODEV), false),
            Ok(FpRegsetProbe {
                regset: NT_X86_XSTATE,
                host_fp_size: 4096
            })
        );
        assert_eq!(
            arch_init_registers_probe(true, Err(-ENODEV), Ok(512), false),
            Ok(FpRegsetProbe {
                regset: NT_PRFPREG,
                host_fp_size: 512
            })
        );
        assert_eq!(
            arch_init_registers_probe(false, Ok(1), Ok(1), false),
            Err(ENOMEM)
        );
        let regs = HostJmpRegs {
            ip: 1,
            sp: 2,
            bp: 3,
        };
        assert_eq!(get_thread_reg(HostReg::Ip, regs), 1);
        assert_eq!(get_thread_reg(HostReg::Sp, regs), 2);
        assert_eq!(get_thread_reg(HostReg::Bp, regs), 3);
    }
}
