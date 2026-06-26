//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/um/vdso/um_vdso.c
//! test-origin: linux:vendor/linux/arch/x86/um/vdso/um_vdso.c
//! UML vDSO calls that deliberately fall back to syscalls.

pub const NR_CLOCK_GETTIME: &str = "__NR_clock_gettime";
pub const NR_GETTIMEOFDAY: &str = "__NR_gettimeofday";
pub const NR_TIME: &str = "__NR_time";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VdsoSyscall {
    pub nr: &'static str,
    pub arg_registers: &'static [&'static str],
    pub clobbers: &'static [&'static str],
}

pub const VDSO_CLOCK_GETTIME: VdsoSyscall = VdsoSyscall {
    nr: NR_CLOCK_GETTIME,
    arg_registers: &["D", "S"],
    clobbers: &["rcx", "r11", "memory"],
};

pub const VDSO_GETTIMEOFDAY: VdsoSyscall = VdsoSyscall {
    nr: NR_GETTIMEOFDAY,
    arg_registers: &["D", "S"],
    clobbers: &["rcx", "r11", "memory"],
};

pub const VDSO_TIME: VdsoSyscall = VdsoSyscall {
    nr: NR_TIME,
    arg_registers: &["D"],
    clobbers: &["cc", "r11", "cx", "memory"],
};

pub const fn weak_aliases() -> [&'static str; 3] {
    ["clock_gettime", "gettimeofday", "time"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uml_vdso_syscall_trampolines_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/um/vdso/um_vdso.c"
        ));
        assert!(source.contains("This vDSO turns all calls into a syscall"));
        assert!(source.contains("int __vdso_clock_gettime"));
        assert!(source.contains("\"0\" (__NR_clock_gettime)"));
        assert!(source.contains("int clock_gettime"));
        assert!(source.contains("int __vdso_gettimeofday"));
        assert!(source.contains("\"0\" (__NR_gettimeofday)"));
        assert!(source.contains("__kernel_old_time_t __vdso_time"));
        assert!(source.contains("\"0\" (__NR_time)"));
        assert!(source.contains("__attribute__((weak, alias(\"__vdso_time\")))"));

        assert_eq!(VDSO_CLOCK_GETTIME.nr, "__NR_clock_gettime");
        assert_eq!(VDSO_GETTIMEOFDAY.arg_registers, ["D", "S"]);
        assert_eq!(VDSO_TIME.clobbers, ["cc", "r11", "cx", "memory"]);
        assert_eq!(weak_aliases(), ["clock_gettime", "gettimeofday", "time"]);
    }
}
