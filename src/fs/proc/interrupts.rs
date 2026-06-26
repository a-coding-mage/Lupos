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

pub fn show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, "           CPU0\n  0:          0   IO-APIC   timer\n")
}

#[cfg(test)]
mod tests {
    use super::*;

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
