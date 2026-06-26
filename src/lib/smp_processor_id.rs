//! linux-parity: complete
//! linux-source: vendor/linux/lib/smp_processor_id.c
//! test-origin: linux:vendor/linux/lib/smp_processor_id.c
//! DEBUG_PREEMPT smp_processor_id checks.

use core::ffi::c_char;

use crate::kernel::module::{export_symbol, find_symbol};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreemptionContext {
    pub raw_cpu: u32,
    pub preempt_count: u32,
    pub irqs_disabled: bool,
    pub is_percpu_thread: bool,
    pub migration_disabled: bool,
    pub system_scheduling_started: bool,
    pub printk_ratelimit: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreemptionCheck {
    pub cpu: u32,
    pub warned: bool,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "debug_smp_processor_id",
        debug_smp_processor_id as usize,
        false,
    );
    export_symbol_once(
        "__this_cpu_preempt_check",
        __this_cpu_preempt_check as usize,
        false,
    );
}

pub const fn check_preemption_disabled(
    context: PreemptionContext,
    _what1: &str,
    _what2: &str,
) -> PreemptionCheck {
    let allowed = context.preempt_count != 0
        || context.irqs_disabled
        || context.is_percpu_thread
        || context.migration_disabled
        || !context.system_scheduling_started;

    PreemptionCheck {
        cpu: context.raw_cpu,
        warned: !allowed && context.printk_ratelimit,
    }
}

pub extern "C" fn debug_smp_processor_id() -> u32 {
    0
}

pub extern "C" fn __this_cpu_preempt_check(_op: *const c_char) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smp_processor_id_preempt_check_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/smp_processor_id.c"
        ));
        assert!(source.contains("raw_smp_processor_id();"));
        assert!(source.contains("if (likely(preempt_count()))"));
        assert!(source.contains("if (irqs_disabled())"));
        assert!(source.contains("if (is_percpu_thread())"));
        assert!(source.contains("if (current->migration_disabled)"));
        assert!(source.contains("if (system_state < SYSTEM_SCHEDULING)"));
        assert!(source.contains("preempt_disable_notrace();"));
        assert!(source.contains("printk_ratelimit()"));
        assert!(source.contains("EXPORT_SYMBOL(debug_smp_processor_id);"));
        assert!(source.contains("EXPORT_SYMBOL(__this_cpu_preempt_check);"));

        let warned = check_preemption_disabled(
            PreemptionContext {
                raw_cpu: 3,
                preempt_count: 0,
                irqs_disabled: false,
                is_percpu_thread: false,
                migration_disabled: false,
                system_scheduling_started: true,
                printk_ratelimit: true,
            },
            "smp_processor_id",
            "",
        );
        assert_eq!(
            warned,
            PreemptionCheck {
                cpu: 3,
                warned: true,
            }
        );

        let allowed = check_preemption_disabled(
            PreemptionContext {
                preempt_count: 1,
                ..PreemptionContext {
                    raw_cpu: 4,
                    preempt_count: 0,
                    irqs_disabled: false,
                    is_percpu_thread: false,
                    migration_disabled: false,
                    system_scheduling_started: true,
                    printk_ratelimit: true,
                }
            },
            "__this_cpu_",
            "read",
        );
        assert_eq!(
            allowed,
            PreemptionCheck {
                cpu: 4,
                warned: false,
            }
        );
    }

    #[test]
    fn smp_processor_id_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("debug_smp_processor_id"),
            Some(debug_smp_processor_id as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("__this_cpu_preempt_check"),
            Some(__this_cpu_preempt_check as usize)
        );
    }
}
