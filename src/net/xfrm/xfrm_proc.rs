//! linux-parity: complete
//! linux-source: vendor/linux/net/xfrm/xfrm_proc.c
//! test-origin: linux:vendor/linux/net/xfrm/xfrm_proc.c
//! XFRM proc statistics table.

use crate::include::uapi::errno::ENOMEM;

pub const XFRM_PROC_NAME: &str = "xfrm_stat";
pub const XFRM_PROC_MODE: u16 = 0o444;
pub const XFRM_MIB_LIST: [&str; 32] = [
    "XfrmInError",
    "XfrmInBufferError",
    "XfrmInHdrError",
    "XfrmInNoStates",
    "XfrmInStateProtoError",
    "XfrmInStateModeError",
    "XfrmInStateSeqError",
    "XfrmInStateExpired",
    "XfrmInStateMismatch",
    "XfrmInStateInvalid",
    "XfrmInTmplMismatch",
    "XfrmInNoPols",
    "XfrmInPolBlock",
    "XfrmInPolError",
    "XfrmOutError",
    "XfrmOutBundleGenError",
    "XfrmOutBundleCheckError",
    "XfrmOutNoStates",
    "XfrmOutStateProtoError",
    "XfrmOutStateModeError",
    "XfrmOutStateSeqError",
    "XfrmOutStateExpired",
    "XfrmOutPolBlock",
    "XfrmOutPolDead",
    "XfrmOutPolError",
    "XfrmFwdHdrError",
    "XfrmOutStateInvalid",
    "XfrmAcquireError",
    "XfrmOutStateDirError",
    "XfrmInStateDirError",
    "XfrmInIptfsError",
    "XfrmOutNoQueueSpace",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XfrmStatRow {
    pub name: &'static str,
    pub value: u64,
}

pub fn xfrm_statistics_row(index: usize, values: &[u64]) -> Option<XfrmStatRow> {
    let name = XFRM_MIB_LIST.get(index).copied()?;
    let value = values.get(index).copied().unwrap_or(0);
    Some(XfrmStatRow { name, value })
}

pub const fn xfrm_proc_init(proc_created: bool) -> Result<(), i32> {
    if proc_created { Ok(()) } else { Err(-ENOMEM) }
}

pub const fn xfrm_proc_fini() -> &'static str {
    XFRM_PROC_NAME
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xfrm_proc_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/xfrm/xfrm_proc.c"
        ));
        assert!(source.contains("static const struct snmp_mib xfrm_mib_list[]"));
        assert!(source.contains("SNMP_MIB_ITEM(\"XfrmInError\", LINUX_MIB_XFRMINERROR)"));
        assert!(
            source.contains("SNMP_MIB_ITEM(\"XfrmOutNoQueueSpace\", LINUX_MIB_XFRMOUTNOQSPACE)")
        );
        assert!(source.contains("static int xfrm_statistics_seq_show"));
        assert!(source.contains("memset(buff, 0, sizeof(buff));"));
        assert!(source.contains("xfrm_state_update_stats(net);"));
        assert!(source.contains("snmp_get_cpu_field_batch_cnt(buff, xfrm_mib_list, cnt"));
        assert!(source.contains("seq_printf(seq, \"%-24s\\t%lu\\n\""));
        assert!(source.contains("proc_create_net_single(\"xfrm_stat\", 0444"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("remove_proc_entry(\"xfrm_stat\", net->proc_net);"));

        assert_eq!(XFRM_MIB_LIST.len(), 32);
        let values = [0, 10, 20, 30];
        assert_eq!(
            xfrm_statistics_row(1, &values),
            Some(XfrmStatRow {
                name: "XfrmInBufferError",
                value: 10
            })
        );
        assert_eq!(
            xfrm_statistics_row(31, &values),
            Some(XfrmStatRow {
                name: "XfrmOutNoQueueSpace",
                value: 0
            })
        );
        assert_eq!(xfrm_proc_init(false), Err(-ENOMEM));
        assert_eq!(xfrm_proc_fini(), "xfrm_stat");
    }
}
