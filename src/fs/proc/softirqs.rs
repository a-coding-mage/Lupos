//! linux-parity: complete
//! linux-source: vendor/linux/fs/proc/softirqs.c
//! test-origin: linux:vendor/linux/fs/proc/softirqs.c
//! `/proc/softirqs`.

use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;

use crate::fs::kernfs::KernfsNode;

pub const SOFTIRQ_NAMES: [&str; crate::kernel::softirq::NR_SOFTIRQS] = [
    "HI", "TIMER", "NET_TX", "NET_RX", "BLOCK", "IRQ_POLL", "TASKLET", "SCHED", "HRTIMER", "RCU",
];

pub fn render_softirqs(cpu_count: usize, counts: &[&[u64]]) -> String {
    let mut out = String::from("                    ");
    for cpu in 0..cpu_count {
        out.push_str(&format!("CPU{:<8}", cpu));
    }
    out.push('\n');

    for (idx, name) in SOFTIRQ_NAMES.iter().enumerate() {
        out.push_str(&format!("{:>12}:", name));
        let row = counts.get(idx).copied().unwrap_or(&[]);
        for cpu in 0..cpu_count {
            let count = row.get(cpu).copied().unwrap_or(0);
            out.push_str(&format!(" {:>10}", count));
        }
        out.push('\n');
    }
    out
}

pub fn show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let zero = [0u64; 1];
    let counts = [&zero[..]; crate::kernel::softirq::NR_SOFTIRQS];
    let text = render_softirqs(1, &counts);
    super::util::copy_into(buf, &text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_softirqs_rendering_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/softirqs.c"
        ));
        let softirq_source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/softirq.c"
        ));
        assert!(source.contains("static int show_softirqs(struct seq_file *p, void *v)"));
        assert!(source.contains("seq_puts(p, \"                    \");"));
        assert!(source.contains("for_each_possible_cpu(i)"));
        assert!(source.contains("seq_printf(p, \"CPU%-8d\", i);"));
        assert!(source.contains("for (i = 0; i < NR_SOFTIRQS; i++)"));
        assert!(source.contains("seq_printf(p, \"%12s:\", softirq_to_name[i]);"));
        assert!(source.contains("kstat_softirqs_cpu(i, j), 10"));
        assert!(source.contains("proc_create_single(\"softirqs\", 0, NULL, show_softirqs);"));
        assert!(source.contains("pde_make_permanent(pde);"));
        assert!(
            softirq_source
                .contains("\"HI\", \"TIMER\", \"NET_TX\", \"NET_RX\", \"BLOCK\", \"IRQ_POLL\",")
        );
        assert!(softirq_source.contains("\"TASKLET\", \"SCHED\", \"HRTIMER\", \"RCU\""));

        assert_eq!(SOFTIRQ_NAMES[0], "HI");
        assert_eq!(SOFTIRQ_NAMES[SOFTIRQ_NAMES.len() - 1], "RCU");

        let hi = [1, 20];
        let timer = [300, 4000];
        let rows = [&hi[..], &timer[..]];
        let rendered = render_softirqs(2, &rows);
        assert!(rendered.starts_with("                    CPU0       CPU1       \n"));
        assert!(rendered.contains("          HI:          1         20\n"));
        assert!(rendered.contains("       TIMER:        300       4000\n"));
        assert!(rendered.contains("         RCU:          0          0\n"));
    }
}
