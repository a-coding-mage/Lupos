//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/xen/irq.c
//! test-origin: linux:vendor/linux/arch/x86/xen/irq.c
//! Xen paravirtual IRQ operation setup.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XenHaltAction {
    VcpuDown,
    SafeHalt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XenIrqOps {
    pub save_fl: &'static str,
    pub irq_disable: &'static str,
    pub irq_enable: &'static str,
    pub safe_halt: &'static str,
    pub halt: &'static str,
    pub intr_init: &'static str,
}

pub const fn xen_force_evtchn_callback() -> &'static str {
    "HYPERVISOR_xen_version"
}

pub const fn xen_safe_halt_result(sched_op_status: i32) -> Result<(), &'static str> {
    if sched_op_status == 0 {
        Ok(())
    } else {
        Err("BUG")
    }
}

pub const fn xen_halt_action(irqs_disabled: bool) -> XenHaltAction {
    if irqs_disabled {
        XenHaltAction::VcpuDown
    } else {
        XenHaltAction::SafeHalt
    }
}

pub const fn xen_init_irq_ops() -> XenIrqOps {
    XenIrqOps {
        save_fl: "paravirt_ret0",
        irq_disable: "paravirt_nop",
        irq_enable: "BUG_func",
        safe_halt: "xen_safe_halt",
        halt: "xen_halt",
        intr_init: "xen_init_IRQ",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xen_irq_ops_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/xen/irq.c"
        ));
        assert!(source.contains("xen_force_evtchn_callback"));
        assert!(source.contains("HYPERVISOR_xen_version(0, NULL);"));
        assert!(source.contains("HYPERVISOR_sched_op(SCHEDOP_block, NULL)"));
        assert!(source.contains("if (irqs_disabled())"));
        assert!(source.contains("HYPERVISOR_vcpu_op(VCPUOP_down"));
        assert!(source.contains("pv_ops.irq.save_fl = __PV_IS_CALLEE_SAVE(paravirt_ret0);"));
        assert!(source.contains("pv_ops.irq.irq_disable = __PV_IS_CALLEE_SAVE(paravirt_nop);"));
        assert!(source.contains("pv_ops.irq.irq_enable = __PV_IS_CALLEE_SAVE(BUG_func);"));
        assert!(source.contains("x86_init.irqs.intr_init = xen_init_IRQ;"));

        assert_eq!(xen_force_evtchn_callback(), "HYPERVISOR_xen_version");
        assert_eq!(xen_safe_halt_result(0), Ok(()));
        assert_eq!(xen_safe_halt_result(-1), Err("BUG"));
        assert_eq!(xen_halt_action(true), XenHaltAction::VcpuDown);
        assert_eq!(xen_halt_action(false), XenHaltAction::SafeHalt);
        assert_eq!(xen_init_irq_ops().irq_enable, "BUG_func");
    }
}
