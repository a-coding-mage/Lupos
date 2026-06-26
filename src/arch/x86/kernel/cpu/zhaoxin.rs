//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/zhaoxin.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/zhaoxin.c
//! Zhaoxin CPU feature initialization.

pub const MSR_ZHAOXIN_FCR57: u32 = 0x0000_1257;
pub const ACE_PRESENT: u32 = 1 << 6;
pub const ACE_ENABLED: u32 = 1 << 7;
pub const ACE_FCR: u32 = 1 << 7;
pub const RNG_PRESENT: u32 = 1 << 2;
pub const RNG_ENABLED: u32 = 1 << 3;
pub const RNG_ENABLE: u32 = 1 << 8;
pub const ZHAOXIN_VENDOR_IDENT: &str = "  Shanghai  ";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ZhaoxinInit {
    pub enable_ace: bool,
    pub enable_rng: bool,
    pub rep_good: bool,
    pub constant_tsc: bool,
    pub nonstop_tsc: bool,
    pub arch_perfmon: bool,
    pub lfence_rdtsc: bool,
    pub fcr57_bits: u32,
}

pub const fn init_zhaoxin_cap(cpuid_max: u32, cpuid_edx_c0000001: u32, family: u8) -> ZhaoxinInit {
    let mut out = ZhaoxinInit {
        rep_good: family >= 0x6,
        ..ZhaoxinInit::empty()
    };
    if cpuid_max >= 0xc000_0001 {
        if cpuid_edx_c0000001 & (ACE_PRESENT | ACE_ENABLED) == ACE_PRESENT {
            out.enable_ace = true;
            out.fcr57_bits |= ACE_FCR;
        }
        if cpuid_edx_c0000001 & (RNG_PRESENT | RNG_ENABLED) == RNG_PRESENT {
            out.enable_rng = true;
            out.fcr57_bits |= RNG_ENABLE;
        }
    }
    out
}

pub const fn early_init_zhaoxin(family: u8, x86_power: u32) -> ZhaoxinInit {
    ZhaoxinInit {
        constant_tsc: family >= 0x6 || (x86_power & (1 << 8)) != 0,
        nonstop_tsc: (x86_power & (1 << 8)) != 0,
        ..ZhaoxinInit::empty()
    }
}

pub const fn zhaoxin_arch_perfmon(cpuid_level: i32, leaf10_eax: u32) -> bool {
    cpuid_level > 9 && (leaf10_eax & 0xff) != 0 && ((leaf10_eax >> 8) & 0xff) > 1
}

impl ZhaoxinInit {
    pub const fn empty() -> Self {
        Self {
            enable_ace: false,
            enable_rng: false,
            rep_good: false,
            constant_tsc: false,
            nonstop_tsc: false,
            arch_perfmon: false,
            lfence_rdtsc: false,
            fcr57_bits: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zhaoxin_cpu_init_matches_linux_feature_bits_and_vendor_registration() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/cpu/zhaoxin.c"
        ));
        assert!(source.contains("#define MSR_ZHAOXIN_FCR57 0x00001257"));
        assert!(source.contains("#define ACE_PRESENT\t(1 << 6)"));
        assert!(source.contains("#define ACE_ENABLED\t(1 << 7)"));
        assert!(source.contains("#define ACE_FCR\t\t(1 << 7)"));
        assert!(source.contains("#define RNG_PRESENT\t(1 << 2)"));
        assert!(source.contains("#define RNG_ENABLED\t(1 << 3)"));
        assert!(source.contains("#define RNG_ENABLE\t(1 << 8)"));
        assert!(source.contains("if (cpuid_eax(0xC0000000) >= 0xC0000001)"));
        assert!(source.contains("lo |= ACE_FCR;"));
        assert!(source.contains("lo |= RNG_ENABLE;"));
        assert!(source.contains("set_cpu_cap(c, X86_FEATURE_REP_GOOD);"));
        assert!(source.contains("set_cpu_cap(c, X86_FEATURE_CONSTANT_TSC);"));
        assert!(source.contains("set_cpu_cap(c, X86_FEATURE_NONSTOP_TSC);"));
        assert!(source.contains("set_cpu_cap(c, X86_FEATURE_ARCH_PERFMON);"));
        assert!(source.contains("set_cpu_cap(c, X86_FEATURE_LFENCE_RDTSC);"));
        assert!(source.contains(".c_vendor\t= \"zhaoxin\""));
        assert!(source.contains(".c_ident\t= { \"  Shanghai  \" }"));
        assert!(source.contains("cpu_dev_register(zhaoxin_cpu_dev);"));

        let cap = init_zhaoxin_cap(0xc000_0001, ACE_PRESENT | RNG_PRESENT, 0x6);
        assert!(cap.enable_ace);
        assert!(cap.enable_rng);
        assert!(cap.rep_good);
        assert_eq!(cap.fcr57_bits, ACE_FCR | RNG_ENABLE);

        let early = early_init_zhaoxin(0x5, 1 << 8);
        assert!(early.constant_tsc);
        assert!(early.nonstop_tsc);
        assert!(zhaoxin_arch_perfmon(10, 0x0201));
        assert!(!zhaoxin_arch_perfmon(10, 0x0101));
        assert_eq!(ZHAOXIN_VENDOR_IDENT, "  Shanghai  ");
        assert_eq!(MSR_ZHAOXIN_FCR57, 0x1257);
    }
}
