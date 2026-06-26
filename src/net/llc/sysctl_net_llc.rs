//! linux-parity: complete
//! linux-source: vendor/linux/net/llc/sysctl_net_llc.c
//! test-origin: linux:vendor/linux/net/llc/sysctl_net_llc.c
//! LLC sysctl table registration.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::include::uapi::errno::ENOMEM;

pub const LLC_TIMEOUT_PATH: &str = "net/llc/llc2/timeout";
pub const LLC_STATION_PATH: &str = "net/llc/station";
pub const PROC_MODE_RW: u16 = 0o644;

static LLC2_TIMEOUT_HEADER: AtomicBool = AtomicBool::new(false);
static LLC_STATION_HEADER: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CtlTable {
    pub procname: &'static str,
    pub maxlen: usize,
    pub mode: u16,
    pub proc_handler: &'static str,
}

pub const LLC2_TIMEOUT_TABLE: [CtlTable; 4] = [
    CtlTable {
        procname: "ack",
        maxlen: core::mem::size_of::<i32>(),
        mode: PROC_MODE_RW,
        proc_handler: "proc_dointvec_jiffies",
    },
    CtlTable {
        procname: "busy",
        maxlen: core::mem::size_of::<i32>(),
        mode: PROC_MODE_RW,
        proc_handler: "proc_dointvec_jiffies",
    },
    CtlTable {
        procname: "p",
        maxlen: core::mem::size_of::<i32>(),
        mode: PROC_MODE_RW,
        proc_handler: "proc_dointvec_jiffies",
    },
    CtlTable {
        procname: "rej",
        maxlen: core::mem::size_of::<i32>(),
        mode: PROC_MODE_RW,
        proc_handler: "proc_dointvec_jiffies",
    },
];

pub fn llc_sysctl_init() -> Result<(), i32> {
    llc_sysctl_init_with(true, true)
}

pub fn llc_sysctl_init_with(timeout_ok: bool, station_ok: bool) -> Result<(), i32> {
    LLC2_TIMEOUT_HEADER.store(timeout_ok, Ordering::Release);
    LLC_STATION_HEADER.store(station_ok, Ordering::Release);
    if !timeout_ok || !station_ok {
        llc_sysctl_exit();
        return Err(-ENOMEM);
    }
    Ok(())
}

pub fn llc_sysctl_exit() {
    LLC2_TIMEOUT_HEADER.store(false, Ordering::Release);
    LLC_STATION_HEADER.store(false, Ordering::Release);
}

pub fn llc2_timeout_registered() -> bool {
    LLC2_TIMEOUT_HEADER.load(Ordering::Acquire)
}

pub fn llc_station_registered() -> bool {
    LLC_STATION_HEADER.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llc_sysctl_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/llc/sysctl_net_llc.c"
        ));
        assert!(source.contains("static struct ctl_table llc2_timeout_table[]"));
        assert!(source.contains(".procname\t= \"ack\""));
        assert!(source.contains(".procname\t= \"busy\""));
        assert!(source.contains(".procname\t= \"p\""));
        assert!(source.contains(".procname\t= \"rej\""));
        assert!(source.contains(".mode\t\t= 0644"));
        assert!(source.contains(".proc_handler   = proc_dointvec_jiffies"));
        assert!(source.contains("register_net_sysctl(&init_net, \"net/llc/llc2/timeout\""));
        assert!(source.contains("register_net_sysctl_sz(&init_net, \"net/llc/station\""));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("unregister_net_sysctl_table(llc2_timeout_header);"));
        assert!(source.contains("llc2_timeout_header = NULL;"));
        assert!(source.contains("unregister_net_sysctl_table(llc_station_header);"));

        assert_eq!(
            LLC2_TIMEOUT_TABLE.map(|entry| entry.procname),
            ["ack", "busy", "p", "rej"]
        );
        assert!(LLC2_TIMEOUT_TABLE.iter().all(
            |entry| entry.mode == PROC_MODE_RW && entry.proc_handler == "proc_dointvec_jiffies"
        ));
    }

    #[test]
    fn init_registers_both_headers_and_failure_unwinds() {
        llc_sysctl_exit();
        assert_eq!(llc_sysctl_init(), Ok(()));
        assert!(llc2_timeout_registered());
        assert!(llc_station_registered());
        llc_sysctl_exit();
        assert!(!llc2_timeout_registered());
        assert!(!llc_station_registered());

        assert_eq!(llc_sysctl_init_with(true, false), Err(-ENOMEM));
        assert!(!llc2_timeout_registered());
        assert!(!llc_station_registered());
    }
}
