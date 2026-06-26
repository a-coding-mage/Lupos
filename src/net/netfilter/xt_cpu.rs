//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_cpu.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_cpu.c
//! Xtables running-CPU match.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_AUTHOR: &str = "Eric Dumazet <eric.dumazet@gmail.com>";
pub const MODULE_DESCRIPTION: &str = "Xtables: CPU match";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_cpu", "ip6t_cpu"];
pub const NFPROTO_UNSPEC: u8 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtCpuInfo {
    pub cpu: u32,
    pub invert: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
    pub matchsize: usize,
}

pub const CPU_MT_REG: XtMatch = XtMatch {
    name: "cpu",
    revision: 0,
    family: NFPROTO_UNSPEC,
    matchsize: core::mem::size_of::<XtCpuInfo>(),
};

pub fn cpu_mt_check(info: XtCpuInfo) -> Result<(), i32> {
    if (info.invert & !1) != 0 {
        Err(EINVAL)
    } else {
        Ok(())
    }
}

pub fn cpu_mt(info: XtCpuInfo, current_cpu: u32) -> bool {
    (info.cpu == current_cpu) ^ (info.invert != 0)
}

pub const fn cpu_mt_init() -> &'static XtMatch {
    &CPU_MT_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_cpu_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_cpu.c"
        ));
        assert!(source.contains("MODULE_AUTHOR(\"Eric Dumazet"));
        assert!(source.contains("MODULE_ALIAS(\"ipt_cpu\")"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_cpu\")"));
        assert!(source.contains("if (info->invert & ~1)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("raw_smp_processor_id()"));
        assert!(source.contains(".name       = \"cpu\""));
        assert!(source.contains(".family     = NFPROTO_UNSPEC"));
        assert!(source.contains(".matchsize  = sizeof(struct xt_cpu_info)"));
        assert!(source.contains("xt_register_match(&cpu_mt_reg);"));

        let info = XtCpuInfo { cpu: 3, invert: 0 };
        assert_eq!(cpu_mt_check(info), Ok(()));
        assert!(cpu_mt(info, 3));
        assert!(!cpu_mt(info, 2));
        assert!(cpu_mt(XtCpuInfo { invert: 1, ..info }, 2));
        assert_eq!(cpu_mt_check(XtCpuInfo { invert: 2, ..info }), Err(EINVAL));
        assert_eq!(CPU_MT_REG.name, "cpu");
        assert_eq!(MODULE_ALIASES, ["ipt_cpu", "ip6t_cpu"]);
    }
}
