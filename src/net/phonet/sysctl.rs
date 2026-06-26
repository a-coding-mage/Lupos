//! linux-parity: complete
//! linux-source: vendor/linux/net/phonet/sysctl.c
//! test-origin: linux:vendor/linux/net/phonet/sysctl.c
//! Phonet local port range sysctl handling.

use crate::include::uapi::errno::{EINVAL, ENOMEM};

pub const DYNAMIC_PORT_MIN: i32 = 0x40;
pub const DYNAMIC_PORT_MAX: i32 = 0x7f;
pub const PHONET_SYSCTL_PATH: &str = "net/phonet";
pub const PHONET_PROCNAME: &str = "local_port_range";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalPortRange {
    pub min: i32,
    pub max: i32,
}

pub const LOCAL_PORT_RANGE_MIN: LocalPortRange = LocalPortRange { min: 0, max: 0 };
pub const LOCAL_PORT_RANGE_MAX: LocalPortRange = LocalPortRange {
    min: 1023,
    max: 1023,
};
pub const LOCAL_PORT_RANGE_DEFAULT: LocalPortRange = LocalPortRange {
    min: DYNAMIC_PORT_MIN,
    max: DYNAMIC_PORT_MAX,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhonetCtlTable {
    pub procname: &'static str,
    pub mode: u16,
    pub path: &'static str,
}

pub const PHONET_TABLE: PhonetCtlTable = PhonetCtlTable {
    procname: PHONET_PROCNAME,
    mode: 0o644,
    path: PHONET_SYSCTL_PATH,
};

pub const fn phonet_get_local_port_range(range: LocalPortRange) -> (i32, i32) {
    (range.min, range.max)
}

pub const fn proc_local_port_range(
    current: LocalPortRange,
    write: bool,
    requested: LocalPortRange,
    proc_ret: i32,
) -> Result<LocalPortRange, i32> {
    if proc_ret != 0 {
        return Err(proc_ret);
    }
    if !write {
        return Ok(current);
    }
    if requested.max < requested.min {
        return Err(-EINVAL);
    }
    Ok(requested)
}

pub const fn phonet_sysctl_init(register_ok: bool) -> Result<&'static PhonetCtlTable, i32> {
    if register_ok {
        Ok(&PHONET_TABLE)
    } else {
        Err(-ENOMEM)
    }
}

pub const fn phonet_sysctl_exit(registered: bool) -> bool {
    registered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phonet_sysctl_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/phonet/sysctl.c"
        ));
        assert!(source.contains("#define DYNAMIC_PORT_MIN\t0x40"));
        assert!(source.contains("#define DYNAMIC_PORT_MAX\t0x7f"));
        assert!(source.contains("static DEFINE_SEQLOCK(local_port_range_lock);"));
        assert!(source.contains("static int local_port_range_min[2] = {0, 0};"));
        assert!(source.contains("static int local_port_range_max[2] = {1023, 1023};"));
        assert!(
            source
                .contains("static int local_port_range[2] = {DYNAMIC_PORT_MIN, DYNAMIC_PORT_MAX};")
        );
        assert!(source.contains("write_seqlock(&local_port_range_lock);"));
        assert!(source.contains("void phonet_get_local_port_range(int *min, int *max)"));
        assert!(source.contains("read_seqbegin(&local_port_range_lock);"));
        assert!(source.contains("proc_dointvec_minmax(&tmp, write, buffer, lenp, ppos);"));
        assert!(source.contains("if (range[1] < range[0])"));
        assert!(source.contains("ret = -EINVAL;"));
        assert!(source.contains(".procname\t= \"local_port_range\""));
        assert!(source.contains("register_net_sysctl(&init_net, \"net/phonet\", phonet_table);"));
        assert!(source.contains("return phonet_table_hrd == NULL ? -ENOMEM : 0;"));
        assert!(source.contains("unregister_net_sysctl_table(phonet_table_hrd);"));
    }

    #[test]
    fn phonet_port_range_rejects_reversed_writes() {
        let current = LOCAL_PORT_RANGE_DEFAULT;
        assert_eq!(phonet_get_local_port_range(current), (0x40, 0x7f));
        assert_eq!(
            proc_local_port_range(current, false, LocalPortRange { min: 9, max: 8 }, 0),
            Ok(current)
        );
        assert_eq!(
            proc_local_port_range(current, true, LocalPortRange { min: 9, max: 8 }, 0),
            Err(-EINVAL)
        );
        assert_eq!(
            proc_local_port_range(current, true, LocalPortRange { min: 10, max: 20 }, 0),
            Ok(LocalPortRange { min: 10, max: 20 })
        );
        assert_eq!(phonet_sysctl_init(false), Err(-ENOMEM));
        assert_eq!(phonet_sysctl_init(true), Ok(&PHONET_TABLE));
        assert!(phonet_sysctl_exit(true));
    }
}
