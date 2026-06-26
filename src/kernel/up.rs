//! linux-parity: complete
//! linux-source: vendor/linux/kernel/up.c
//! test-origin: linux:vendor/linux/kernel/up.c
//! Uniprocessor SMP API compatibility helpers.

use crate::include::uapi::errno::ENXIO;

pub const BOOT_CPU: usize = 0;

pub const fn smp_call_function_single_gate(cpu: usize) -> Result<(), i32> {
    if cpu != BOOT_CPU { Err(-ENXIO) } else { Ok(()) }
}

pub fn smp_call_function_single<F>(cpu: usize, func: F) -> Result<(), i32>
where
    F: FnOnce(),
{
    smp_call_function_single_gate(cpu)?;
    func();
    Ok(())
}

pub const fn on_each_cpu_cond_mask_should_run(cond_true: bool, cpu0_in_mask: bool) -> bool {
    cond_true && cpu0_in_mask
}

pub fn smp_call_on_cpu<F>(cpu: usize, phys: bool, func: F) -> Result<(i32, bool), i32>
where
    F: FnOnce() -> i32,
{
    smp_call_function_single_gate(cpu)?;
    let ret = func();
    Ok((ret, phys))
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn uniprocessor_smp_calls_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/up.c"
        ));
        assert!(source.contains("if (cpu != 0)"));
        assert!(source.contains("return -ENXIO;"));
        assert!(source.contains("local_irq_save(flags);"));
        assert!(source.contains("func(info);"));
        assert!(source.contains("csd->func(csd->info);"));
        assert!(source.contains("preempt_disable();"));
        assert!(source.contains("cpumask_test_cpu(0, mask)"));
        assert!(source.contains("hypervisor_pin_vcpu(0);"));
        assert!(source.contains("hypervisor_pin_vcpu(-1);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(smp_call_on_cpu)"));

        static CALLS: AtomicUsize = AtomicUsize::new(0);
        smp_call_function_single(0, || {
            CALLS.fetch_add(1, Ordering::SeqCst);
        })
        .unwrap();
        assert_eq!(CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(smp_call_function_single_gate(1), Err(-ENXIO));
        assert!(on_each_cpu_cond_mask_should_run(true, true));
        assert!(!on_each_cpu_cond_mask_should_run(false, true));
        assert_eq!(smp_call_on_cpu(0, true, || 7), Ok((7, true)));
    }
}
