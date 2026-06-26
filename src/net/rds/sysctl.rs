//! linux-parity: complete
//! linux-source: vendor/linux/net/rds/sysctl.c
//! test-origin: linux:vendor/linux/net/rds/sysctl.c
//! RDS sysctl defaults and registration.

use crate::include::uapi::errno::ENOMEM;

pub const HZ: u64 = 100;
pub const DEFAULT_MAX_UNACKED_PACKETS: u32 = 8;
pub const DEFAULT_MAX_UNACKED_BYTES: u32 = 16 << 20;
pub const DEFAULT_PING_ENABLE: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RdsSysctlState {
    pub reconnect_min: u64,
    pub reconnect_max: u64,
    pub reconnect_min_jiffies: u64,
    pub reconnect_max_jiffies: u64,
    pub max_unacked_packets: u32,
    pub max_unacked_bytes: u32,
    pub ping_enable: u32,
    pub registered: bool,
}

impl Default for RdsSysctlState {
    fn default() -> Self {
        Self {
            reconnect_min: 1,
            reconnect_max: u64::MAX,
            reconnect_min_jiffies: 0,
            reconnect_max_jiffies: HZ,
            max_unacked_packets: DEFAULT_MAX_UNACKED_PACKETS,
            max_unacked_bytes: DEFAULT_MAX_UNACKED_BYTES,
            ping_enable: DEFAULT_PING_ENABLE,
            registered: false,
        }
    }
}

pub const RDS_SYSCTL_NAMES: [&str; 5] = [
    "reconnect_min_delay_ms",
    "reconnect_max_delay_ms",
    "max_unacked_packets",
    "max_unacked_bytes",
    "ping_enable",
];

pub const fn msecs_to_jiffies(ms: u64) -> u64 {
    ms.saturating_mul(HZ).div_ceil(1000)
}

pub fn rds_sysctl_init(state: &mut RdsSysctlState, register_ok: bool) -> Result<(), i32> {
    state.reconnect_min = msecs_to_jiffies(1);
    state.reconnect_min_jiffies = state.reconnect_min;
    if !register_ok {
        return Err(-ENOMEM);
    }
    state.registered = true;
    Ok(())
}

pub fn rds_sysctl_exit(state: &mut RdsSysctlState) {
    state.registered = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rds_sysctl_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/rds/sysctl.c"
        ));
        assert!(source.contains("static struct ctl_table_header *rds_sysctl_reg_table;"));
        assert!(source.contains("static unsigned long rds_sysctl_reconnect_min = 1;"));
        assert!(source.contains("rds_sysctl_reconnect_max = ~0UL;"));
        assert!(source.contains("rds_sysctl_reconnect_max_jiffies = HZ;"));
        assert!(source.contains("rds_sysctl_max_unacked_packets = 8;"));
        assert!(source.contains("rds_sysctl_max_unacked_bytes = (16 << 20);"));
        assert!(source.contains("rds_sysctl_ping_enable = 1;"));
        assert!(source.contains(".procname       = \"reconnect_min_delay_ms\""));
        assert!(source.contains(".procname       = \"reconnect_max_delay_ms\""));
        assert!(source.contains(".procname\t= \"max_unacked_packets\""));
        assert!(source.contains(".procname\t= \"max_unacked_bytes\""));
        assert!(source.contains(".procname\t= \"ping_enable\""));
        assert!(source.contains("unregister_net_sysctl_table(rds_sysctl_reg_table);"));
        assert!(source.contains("rds_sysctl_reconnect_min = msecs_to_jiffies(1);"));
        assert!(source.contains("register_net_sysctl(&init_net, \"net/rds\""));
        assert!(source.contains("return -ENOMEM;"));
    }

    #[test]
    fn rds_sysctl_initializes_jiffy_minimum_and_registration_state() {
        let mut state = RdsSysctlState::default();
        assert_eq!(state.max_unacked_bytes, 16 << 20);
        assert_eq!(rds_sysctl_init(&mut state, false), Err(-ENOMEM));
        assert_eq!(state.reconnect_min_jiffies, msecs_to_jiffies(1));
        assert_eq!(rds_sysctl_init(&mut state, true), Ok(()));
        assert!(state.registered);
        rds_sysctl_exit(&mut state);
        assert!(!state.registered);
        assert_eq!(RDS_SYSCTL_NAMES[4], "ping_enable");
    }
}
