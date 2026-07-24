//! linux-parity: complete
//! linux-source: vendor/linux/fs/proc/interrupts.c
//! test-origin: linux:vendor/linux/fs/proc/interrupts.c
//! `/proc/interrupts`.

use alloc::sync::Arc;

use crate::fs::kernfs::KernfsNode;

pub const INT_SEQ_OPERATIONS_SYMBOL: &str = "int_seq_ops";
pub const INT_SEQ_OPERATIONS: &[(&str, &str)] = &[
    ("start", "int_seq_start"),
    ("next", "int_seq_next"),
    ("stop", "int_seq_stop"),
    ("show", "show_interrupts"),
];

pub const fn int_seq_start(pos: u64, nr_irqs: u64) -> Option<u64> {
    if pos <= nr_irqs { Some(pos) } else { None }
}

pub const fn int_seq_next(pos: u64, nr_irqs: u64) -> Option<u64> {
    let next = pos.saturating_add(1);
    if next > nr_irqs { None } else { Some(next) }
}

pub const fn int_seq_stop() {}

pub const fn proc_interrupts_init_creates_seq() -> (&'static str, &'static str) {
    ("interrupts", INT_SEQ_OPERATIONS_SYMBOL)
}

/// Render `/proc/interrupts` from the live descriptor table.
///
/// Mirrors Linux `irq_seq_show()` (`kernel/irq/proc.c`): `num_prec + 8` blanks
/// then one `CPU%-8d` column per online CPU for the header, then per
/// descriptor the IRQ number at `num_prec` width, a colon, the counts, a
/// two-space visual gap, the chip name at `chip_width`, and the action names.
///
/// Two documented divergences from Linux, both because the backing data does
/// not exist yet rather than by choice:
///
/// * Lupos keeps a single `IrqDesc::stat.count` rather than Linux's per-CPU
///   `kstat_irqs`, so exactly one CPU column is emitted.
/// * Linux iterates the allocated descriptors; Lupos has a fixed 256-entry
///   static table, so a descriptor is emitted once it has an action or a
///   non-zero count — the same set that would be allocated under SPARSE_IRQ.
pub fn show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, render().as_str())
}

pub(crate) fn render() -> alloc::string::String {
    use core::fmt::Write as _;
    use core::sync::atomic::Ordering;

    use crate::kernel::irq::irqdesc::{NR_IRQS, desc_for};

    let mut out = alloc::string::String::new();
    // Linux: seq_printf(p, "%*s", constr->num_prec + 8, "") with num_prec = 4.
    let _ = write!(out, "{:>12}", "");
    let _ = writeln!(out, "CPU0");

    for irq in 0..NR_IRQS as u32 {
        let Some(desc) = desc_for(irq) else {
            continue;
        };
        let count = desc.stat.lock().count;
        let action = desc.action.lock();
        if count == 0 && action.is_none() {
            continue;
        }

        let _ = write!(out, "{irq:>4}:{count:>11}  ");
        let chip = if desc.chip.load(Ordering::Relaxed) != 0 {
            "IO-APIC"
        } else {
            "-"
        };
        let _ = write!(out, "{chip:<8}");

        let mut cursor = action.as_deref();
        let mut first = true;
        while let Some(entry) = cursor {
            let _ = write!(out, "{}{}", if first { "  " } else { ", " }, entry.name);
            first = false;
            cursor = entry.next.as_deref();
        }
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `/proc/interrupts` must report the live per-IRQ counters.
    ///
    /// This file previously returned a hardcoded
    /// `"           CPU0\n  0:          0   IO-APIC   timer\n"` for every read,
    /// so `IrqDesc::stat.count` was never observable from userspace and IRQ
    /// delivery could not be diagnosed inside the guest at all. Linux
    /// `irq_seq_show()` emits `desc->kstat_irqs` for each descriptor.
    ///
    /// test-origin: linux:vendor/linux/kernel/irq/proc.c
    #[test]
    fn proc_interrupts_reports_live_irq_counts() {
        // A high vector keeps this clear of the descriptors the other IRQ
        // tests touch; the counter is bumped by the real dispatch entry point.
        const PROBE_IRQ: u32 = 201;

        let before = crate::kernel::irq::irqdesc::desc_for(PROBE_IRQ)
            .expect("descriptor exists")
            .stat
            .lock()
            .count;
        for _ in 0..3 {
            crate::kernel::irq::handle::generic_handle_irq(PROBE_IRQ);
        }

        let rendered = render();
        let expected = alloc::format!("{PROBE_IRQ:>4}:{:>11}", before + 3);
        assert!(
            rendered.contains(&expected),
            "/proc/interrupts did not report the live count for irq {PROBE_IRQ}; \
             expected a line starting {expected:?}, got:\n{rendered}"
        );
        assert!(
            rendered.starts_with("            CPU0\n"),
            "header must match Linux irq_seq_show() spacing, got:\n{rendered}"
        );
    }

    #[test]
    fn proc_interrupts_seq_ops_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/interrupts.c"
        ));
        assert!(source.contains("#include <linux/interrupt.h>"));
        assert!(source.contains("#include <linux/irqnr.h>"));
        assert!(source.contains("static void *int_seq_start"));
        assert!(source.contains("return *pos <= irq_get_nr_irqs() ? pos : NULL;"));
        assert!(source.contains("static void *int_seq_next"));
        assert!(source.contains("(*pos)++;"));
        assert!(source.contains("if (*pos > irq_get_nr_irqs())"));
        assert!(source.contains("static void int_seq_stop"));
        assert!(source.contains("/* Nothing to do */"));
        assert!(source.contains("static const struct seq_operations int_seq_ops"));
        for (slot, target) in INT_SEQ_OPERATIONS {
            assert!(source.contains(slot));
            assert!(source.contains(target));
        }
        assert!(source.contains("proc_create_seq(\"interrupts\", 0, NULL, &int_seq_ops);"));
        assert!(source.contains("fs_initcall(proc_interrupts_init);"));

        assert_eq!(int_seq_start(0, 2), Some(0));
        assert_eq!(int_seq_start(2, 2), Some(2));
        assert_eq!(int_seq_start(3, 2), None);
        assert_eq!(int_seq_next(0, 2), Some(1));
        assert_eq!(int_seq_next(1, 2), Some(2));
        assert_eq!(int_seq_next(2, 2), None);
        assert_eq!(
            proc_interrupts_init_creates_seq(),
            ("interrupts", "int_seq_ops")
        );
        int_seq_stop();
    }
}
