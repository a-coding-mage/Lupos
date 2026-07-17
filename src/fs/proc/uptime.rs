//! linux-parity: complete
//! linux-source: vendor/linux/fs/proc/uptime.c
//! test-origin: linux:vendor/linux/fs/proc/uptime.c
//! `/proc/uptime`.

use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;

use crate::fs::kernfs::KernfsNode;

pub const NSEC_PER_SEC: u64 = 1_000_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UptimeSnapshot {
    pub uptime_sec: u64,
    pub uptime_nsec: u64,
    pub idle_nsec: u64,
}

pub fn render_uptime(snapshot: UptimeSnapshot) -> String {
    let idle_sec = snapshot.idle_nsec / NSEC_PER_SEC;
    let idle_rem = snapshot.idle_nsec % NSEC_PER_SEC;
    format!(
        "{}.{:02} {}.{:02}\n",
        snapshot.uptime_sec,
        snapshot.uptime_nsec / (NSEC_PER_SEC / 100),
        idle_sec,
        idle_rem / (NSEC_PER_SEC / 100)
    )
}

pub fn uptime_snapshot() -> UptimeSnapshot {
    let uptime_nsec = crate::kernel::time::jiffies::jiffies()
        .saturating_mul(crate::kernel::time::jiffies::NSEC_PER_TICK);
    UptimeSnapshot {
        uptime_sec: uptime_nsec / NSEC_PER_SEC,
        uptime_nsec: uptime_nsec % NSEC_PER_SEC,
        idle_nsec: uptime_nsec,
    }
}

pub fn show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, &render_uptime(uptime_snapshot()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_uptime_rendering_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/uptime.c"
        ));
        assert!(source.contains("static int uptime_proc_show(struct seq_file *m, void *v)"));
        assert!(source.contains("idle_nsec = 0;"));
        assert!(source.contains("for_each_possible_cpu(i)"));
        assert!(source.contains("idle_nsec += kcpustat_field(CPUTIME_IDLE, i);"));
        assert!(source.contains("ktime_get_boottime_ts64(&uptime);"));
        assert!(source.contains("timens_add_boottime(&uptime);"));
        assert!(source.contains("idle.tv_sec = div_u64_rem(idle_nsec, NSEC_PER_SEC, &rem);"));
        assert!(source.contains("\"%lu.%02lu %lu.%02lu\\n\""));
        assert!(source.contains("uptime.tv_nsec / (NSEC_PER_SEC / 100)"));
        assert!(source.contains("idle.tv_nsec / (NSEC_PER_SEC / 100)"));
        assert!(source.contains("proc_create_single(\"uptime\", 0, NULL, uptime_proc_show);"));
        assert!(source.contains("pde_make_permanent(pde);"));

        assert_eq!(
            render_uptime(UptimeSnapshot {
                uptime_sec: 12,
                uptime_nsec: 340_000_000,
                idle_nsec: 56_780_000_000,
            }),
            "12.34 56.78\n"
        );
        assert_eq!(
            render_uptime(UptimeSnapshot {
                uptime_sec: 0,
                uptime_nsec: 9_999_999,
                idle_nsec: 9_999_999,
            }),
            "0.00 0.00\n"
        );
        assert_eq!(
            render_uptime(UptimeSnapshot {
                uptime_sec: 1,
                uptime_nsec: 999_999_999,
                idle_nsec: 1_999_999_999,
            }),
            "1.99 1.99\n"
        );
    }
}
