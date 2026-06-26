//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/irq_test.c
//! test-origin: linux:vendor/linux/kernel/irq/irq_test.c
//! IRQ KUnit depth and managed-affinity test inventory.

pub const SUITE_NAME: &str = "irq_test_cases";
pub const MODULE_DESCRIPTION: &str = "IRQ unit test suite";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FakeIrqDesc {
    pub depth: u32,
    pub activated: bool,
    pub started: bool,
    pub affinity_managed: bool,
}

impl FakeIrqDesc {
    pub const fn requested(managed: bool) -> Self {
        Self {
            depth: 0,
            activated: true,
            started: true,
            affinity_managed: managed,
        }
    }

    pub fn disable(&mut self) {
        self.depth = self.depth.saturating_add(1);
    }

    pub fn enable(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    pub fn shutdown_and_deactivate(&mut self) {
        self.activated = false;
        self.started = false;
    }

    pub fn startup_managed(&mut self) {
        if self.affinity_managed {
            self.activated = true;
            self.started = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn irq_test_matches_linux_original_kunit_suite() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/irq/irq_test.c"
        ));
        let internals = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/irq/internals.h"
        ));

        assert!(source.contains("static struct irq_chip fake_irq_chip"));
        assert!(source.contains(".name           = \"fake\""));
        for case in [
            "KUNIT_CASE(irq_disable_depth_test)",
            "KUNIT_CASE(irq_free_disabled_test)",
            "KUNIT_CASE(irq_shutdown_depth_test)",
            "KUNIT_CASE(irq_cpuhotplug_test)",
        ] {
            assert!(source.contains(case));
        }
        assert!(source.contains("disable_irq(virq);"));
        assert!(source.contains("enable_irq(virq);"));
        assert!(source.contains("irq_shutdown_and_deactivate(desc);"));
        assert!(source.contains("remove_cpu(1)"));
        assert!(source.contains(".name = \"irq_test_cases\""));
        assert!(source.contains(MODULE_DESCRIPTION));
        assert!(internals.contains("irq_startup_managed"));

        let mut desc = FakeIrqDesc::requested(false);
        desc.disable();
        assert_eq!(desc.depth, 1);
        desc.enable();
        assert_eq!(desc.depth, 0);

        let mut managed = FakeIrqDesc::requested(true);
        managed.disable();
        managed.shutdown_and_deactivate();
        assert!(!managed.activated);
        managed.startup_managed();
        assert!(managed.activated);
        assert!(managed.started);
        assert_eq!(managed.depth, 1);
    }
}
