//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/mce/winchip.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/mce/winchip.c
//! Winchip MCE model.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/cpu/mce/winchip.c

pub const MSR_IDT_FCR1: u32 = 0x107;
pub const WINCHIP_EIERRINT_BIT: u32 = 1 << 2;
pub const WINCHIP_MCE_DISABLE_BIT: u32 = 1 << 4;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WinchipMachineCheck {
    pub taint_machine_check: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WinchipInitPlan {
    pub fcr1_value: u32,
    pub set_cr4_mce: bool,
}

pub const fn winchip_machine_check() -> WinchipMachineCheck {
    WinchipMachineCheck {
        taint_machine_check: true,
    }
}

pub const fn winchip_mcheck_init(fcr1_low: u32) -> WinchipInitPlan {
    WinchipInitPlan {
        fcr1_value: (fcr1_low | WINCHIP_EIERRINT_BIT) & !WINCHIP_MCE_DISABLE_BIT,
        set_cr4_mce: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn winchip_init_enables_interrupt_and_clears_disable_bit() {
        let plan = winchip_mcheck_init(WINCHIP_MCE_DISABLE_BIT);
        assert_ne!(plan.fcr1_value & WINCHIP_EIERRINT_BIT, 0);
        assert_eq!(plan.fcr1_value & WINCHIP_MCE_DISABLE_BIT, 0);
        assert!(plan.set_cr4_mce);
    }

    #[test]
    fn winchip_machine_check_taints_like_linux_handler() {
        assert!(winchip_machine_check().taint_machine_check);
    }
}
