//! linux-parity: complete
//! linux-source: vendor/linux/net/x25/sysctl_net_x25.c
//! test-origin: linux:vendor/linux/net/x25/sysctl_net_x25.c
//! X.25 sysctl table shape.

use crate::include::uapi::errno::ENOMEM;

pub const HZ: i32 = crate::kernel::time::jiffies::HZ as i32;
pub const MIN_TIMER: i32 = HZ;
pub const MAX_TIMER: i32 = 300 * HZ;
pub const X25_SYSCTL_PATH: &str = "net/x25";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcHandler {
    IntVec,
    IntVecMinMax,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CtlTable {
    pub procname: &'static str,
    pub data: &'static str,
    pub maxlen: usize,
    pub mode: u16,
    pub proc_handler: ProcHandler,
    pub extra1: Option<i32>,
    pub extra2: Option<i32>,
}

pub const X25_TABLE: [CtlTable; 6] = [
    timer_entry(
        "restart_request_timeout",
        "sysctl_x25_restart_request_timeout",
    ),
    timer_entry("call_request_timeout", "sysctl_x25_call_request_timeout"),
    timer_entry("reset_request_timeout", "sysctl_x25_reset_request_timeout"),
    timer_entry("clear_request_timeout", "sysctl_x25_clear_request_timeout"),
    timer_entry(
        "acknowledgement_hold_back_timeout",
        "sysctl_x25_ack_holdback_timeout",
    ),
    CtlTable {
        procname: "x25_forward",
        data: "sysctl_x25_forward",
        maxlen: core::mem::size_of::<i32>(),
        mode: 0o644,
        proc_handler: ProcHandler::IntVec,
        extra1: None,
        extra2: None,
    },
];

const fn timer_entry(procname: &'static str, data: &'static str) -> CtlTable {
    CtlTable {
        procname,
        data,
        maxlen: core::mem::size_of::<i32>(),
        mode: 0o644,
        proc_handler: ProcHandler::IntVecMinMax,
        extra1: Some(MIN_TIMER),
        extra2: Some(MAX_TIMER),
    }
}

pub const fn x25_register_sysctl(header_created: bool) -> Result<(), i32> {
    if header_created { Ok(()) } else { Err(-ENOMEM) }
}

pub const fn x25_unregister_sysctl() -> &'static str {
    X25_SYSCTL_PATH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sysctl_net_x25_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/x25/sysctl_net_x25.c"
        ));
        assert!(source.contains("static int min_timer[] = {   1 * HZ };"));
        assert!(source.contains("static int max_timer[] = { 300 * HZ };"));
        assert!(source.contains("static struct ctl_table_header *x25_table_header;"));
        assert!(source.contains("static struct ctl_table x25_table[]"));
        assert!(source.contains(".procname =\t\"restart_request_timeout\""));
        assert!(source.contains(".data =\t\t&sysctl_x25_restart_request_timeout"));
        assert!(source.contains(".procname =\t\"call_request_timeout\""));
        assert!(source.contains(".procname =\t\"reset_request_timeout\""));
        assert!(source.contains(".procname =\t\"clear_request_timeout\""));
        assert!(source.contains(".procname =\t\"acknowledgement_hold_back_timeout\""));
        assert!(source.contains(".procname =\t\"x25_forward\""));
        assert!(source.contains(".proc_handler =\tproc_dointvec_minmax"));
        assert!(source.contains(".proc_handler = proc_dointvec"));
        assert!(source.contains("register_net_sysctl(&init_net, \"net/x25\", x25_table);"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("unregister_net_sysctl_table(x25_table_header);"));

        assert_eq!(MIN_TIMER, HZ);
        assert_eq!(MAX_TIMER, 300 * HZ);
        assert_eq!(X25_TABLE.len(), 6);
        assert_eq!(X25_TABLE[0].procname, "restart_request_timeout");
        assert_eq!(X25_TABLE[0].extra1, Some(MIN_TIMER));
        assert_eq!(X25_TABLE[4].procname, "acknowledgement_hold_back_timeout");
        assert_eq!(X25_TABLE[5].proc_handler, ProcHandler::IntVec);
        assert_eq!(x25_register_sysctl(false), Err(-ENOMEM));
        assert_eq!(x25_unregister_sysctl(), "net/x25");
    }
}
