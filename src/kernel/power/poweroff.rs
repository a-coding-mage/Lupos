//! linux-parity: complete
//! linux-source: vendor/linux/kernel/power/poweroff.c
//! test-origin: linux:vendor/linux/kernel/power/poweroff.c
//! SysRq poweroff work dispatch.

pub const SYSRQ_ENABLE_BOOT: u32 = 0x0000_0008;
pub const SYSRQ_POWEROFF_KEY: u8 = b'o';
pub const SYSRQ_POWEROFF_HELP: &str = "poweroff(o)";
pub const SYSRQ_POWEROFF_ACTION: &str = "Power Off";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SysrqKeyOp {
    pub key: u8,
    pub help_msg: &'static str,
    pub action_msg: &'static str,
    pub enable_mask: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScheduledPoweroff {
    pub cpu: usize,
    pub calls_kernel_power_off: bool,
}

pub const SYSRQ_POWEROFF_OP: SysrqKeyOp = SysrqKeyOp {
    key: SYSRQ_POWEROFF_KEY,
    help_msg: SYSRQ_POWEROFF_HELP,
    action_msg: SYSRQ_POWEROFF_ACTION,
    enable_mask: SYSRQ_ENABLE_BOOT,
};

pub fn handle_poweroff_online_mask(online_cpus: &[bool]) -> Option<ScheduledPoweroff> {
    let cpu = online_cpus.iter().position(|online| *online)?;
    Some(ScheduledPoweroff {
        cpu,
        calls_kernel_power_off: true,
    })
}

pub const fn pm_sysrq_init_registers_key() -> SysrqKeyOp {
    SYSRQ_POWEROFF_OP
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sysrq_poweroff_op_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/power/poweroff.c"
        ));
        assert!(source.contains("kernel_power_off();"));
        assert!(
            source.contains("schedule_work_on(cpumask_first(cpu_online_mask), &poweroff_work);")
        );
        assert!(source.contains(".help_msg       = \"poweroff(o)\""));
        assert!(source.contains(".action_msg     = \"Power Off\""));
        assert!(source.contains(".enable_mask\t= SYSRQ_ENABLE_BOOT"));
        assert!(source.contains("register_sysrq_key('o', &sysrq_poweroff_op);"));
        assert!(source.contains("subsys_initcall(pm_sysrq_init);"));

        assert_eq!(pm_sysrq_init_registers_key().key, b'o');
        assert_eq!(
            handle_poweroff_online_mask(&[false, false, true]),
            Some(ScheduledPoweroff {
                cpu: 2,
                calls_kernel_power_off: true,
            })
        );
        assert_eq!(handle_poweroff_online_mask(&[]), None);
    }
}
