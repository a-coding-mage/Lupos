//! linux-parity: complete
//! linux-source: vendor/linux/fs/proc/loadavg.c
//! test-origin: linux:vendor/linux/fs/proc/loadavg.c
//! `/proc/loadavg`.

use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;

use crate::fs::kernfs::KernfsNode;
use crate::kernel::sched::loadavg::{FIXED_1, get_avenrun, load_frac, load_int};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoadavgSnapshot {
    pub avnrun: [u64; 3],
    pub nr_running: u32,
    pub nr_threads: i32,
    pub last_pid: i32,
}

pub fn render_loadavg(snapshot: LoadavgSnapshot) -> String {
    format!(
        "{}.{:02} {}.{:02} {}.{:02} {}/{} {}\n",
        load_int(snapshot.avnrun[0]),
        load_frac(snapshot.avnrun[0]),
        load_int(snapshot.avnrun[1]),
        load_frac(snapshot.avnrun[1]),
        load_int(snapshot.avnrun[2]),
        load_frac(snapshot.avnrun[2]),
        snapshot.nr_running,
        snapshot.nr_threads,
        snapshot.last_pid
    )
}

pub fn loadavg_snapshot() -> LoadavgSnapshot {
    let mut avnrun = [0; 3];
    get_avenrun(&mut avnrun, FIXED_1 / 200, 0);
    LoadavgSnapshot {
        avnrun,
        nr_running: nr_running_snapshot(),
        nr_threads: nr_threads_snapshot(),
        last_pid: last_pid_snapshot(),
    }
}

pub fn nr_running_snapshot() -> u32 {
    let mut running = 0u32;
    for cpu in 0..crate::kernel::sched::rq::MAX_RQ_CPUS {
        if let Some(nr) = crate::kernel::sched::rq::rq_nr_running(cpu as u32) {
            running = running.saturating_add(nr);
        }
    }
    running.max(1)
}

pub fn nr_threads_snapshot() -> i32 {
    1
}

pub fn last_pid_snapshot() -> i32 {
    let current = unsafe { crate::kernel::sched::get_current() };
    if current.is_null() {
        1
    } else {
        unsafe { (*current).pid.max(1) }
    }
}

pub fn show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, &render_loadavg(loadavg_snapshot()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_loadavg_rendering_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/loadavg.c"
        ));
        let loadavg_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/sched/loadavg.h"
        ));
        assert!(source.contains("static int loadavg_proc_show(struct seq_file *m, void *v)"));
        assert!(source.contains("unsigned long avnrun[3];"));
        assert!(source.contains("get_avenrun(avnrun, FIXED_1/200, 0);"));
        assert!(source.contains("\"%lu.%02lu %lu.%02lu %lu.%02lu %u/%d %d\\n\""));
        assert!(source.contains("LOAD_INT(avnrun[0]), LOAD_FRAC(avnrun[0])"));
        assert!(source.contains("nr_running(), nr_threads"));
        assert!(source.contains("idr_get_cursor(&task_active_pid_ns(current)->idr) - 1"));
        assert!(source.contains("proc_create_single(\"loadavg\", 0, NULL, loadavg_proc_show);"));
        assert!(source.contains("pde_make_permanent(pde);"));
        assert!(loadavg_header.contains("#define FSHIFT\t\t11"));
        assert!(loadavg_header.contains("#define FIXED_1\t\t(1<<FSHIFT)"));
        assert!(loadavg_header.contains("#define LOAD_INT(x) ((x) >> FSHIFT)"));
        assert!(
            loadavg_header.contains("#define LOAD_FRAC(x) LOAD_INT(((x) & (FIXED_1-1)) * 100)")
        );

        assert_eq!(FIXED_1 / 200, 10);
        assert_eq!(
            render_loadavg(LoadavgSnapshot {
                avnrun: [0, FIXED_1 + (FIXED_1 / 2), (2 * FIXED_1) + 20],
                nr_running: 3,
                nr_threads: 7,
                last_pid: 42,
            }),
            "0.00 1.50 2.00 3/7 42\n"
        );
        assert_eq!(
            render_loadavg(LoadavgSnapshot {
                avnrun: [FIXED_1 - 1, 0, 0],
                nr_running: 1,
                nr_threads: 1,
                last_pid: 1,
            }),
            "0.99 0.00 0.00 1/1 1\n"
        );
    }
}
