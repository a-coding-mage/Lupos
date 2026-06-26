//! linux-parity: complete
//! linux-source: vendor/linux/net/core/hotdata.c
//! test-origin: linux:vendor/linux/net/core/hotdata.c
//! Networking hot-data defaults.

pub const GRO_NORMAL_BATCH: u32 = 8;
pub const NETDEV_BUDGET: u32 = 300;
pub const USEC_PER_SEC: u32 = 1_000_000;
pub const HZ: u32 = 250;
pub const NETDEV_BUDGET_USECS: u32 = 2 * USEC_PER_SEC / HZ;
pub const TSTAMP_PREQUEUE: u32 = 1;
pub const MAX_BACKLOG: u32 = 1000;
pub const QDISC_MAX_BURST: u32 = 1000;
pub const DEV_TX_WEIGHT: u32 = 64;
pub const DEV_RX_WEIGHT: u32 = 64;
pub const MAX_SKB_FRAGS: u32 = 17;
pub const PAGE_SHIFT: u32 = 12;
pub const SK_MEMORY_PCPU_RESERVE: u32 = 1 << (20 - PAGE_SHIFT);
pub const SYSCTL_MAX_SKB_FRAGS: u32 = MAX_SKB_FRAGS;
pub const SYSCTL_SKB_DEFER_MAX: u32 = 128;
pub const SYSCTL_MEM_PCPU_RSV: u32 = SK_MEMORY_PCPU_RESERVE;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NetHotdataDefaults {
    pub gro_normal_batch: u32,
    pub netdev_budget: u32,
    pub netdev_budget_usecs: u32,
    pub tstamp_prequeue: u32,
    pub max_backlog: u32,
    pub qdisc_max_burst: u32,
    pub dev_tx_weight: u32,
    pub dev_rx_weight: u32,
    pub sysctl_max_skb_frags: u32,
    pub sysctl_skb_defer_max: u32,
    pub sysctl_mem_pcpu_rsv: u32,
}

pub const NET_HOTDATA_DEFAULTS: NetHotdataDefaults = NetHotdataDefaults {
    gro_normal_batch: GRO_NORMAL_BATCH,
    netdev_budget: NETDEV_BUDGET,
    netdev_budget_usecs: NETDEV_BUDGET_USECS,
    tstamp_prequeue: TSTAMP_PREQUEUE,
    max_backlog: MAX_BACKLOG,
    qdisc_max_burst: QDISC_MAX_BURST,
    dev_tx_weight: DEV_TX_WEIGHT,
    dev_rx_weight: DEV_RX_WEIGHT,
    sysctl_max_skb_frags: SYSCTL_MAX_SKB_FRAGS,
    sysctl_skb_defer_max: SYSCTL_SKB_DEFER_MAX,
    sysctl_mem_pcpu_rsv: SYSCTL_MEM_PCPU_RSV,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NetAlignedData;

pub const NET_ALIGNED_DATA: NetAlignedData = NetAlignedData;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn net_hotdata_defaults_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/core/hotdata.c"
        ));
        assert!(source.contains("struct net_hotdata net_hotdata __cacheline_aligned"));
        assert!(source.contains(".offload_base = LIST_HEAD_INIT(net_hotdata.offload_base)"));
        assert!(source.contains(".gro_normal_batch = 8"));
        assert!(source.contains(".netdev_budget = 300"));
        assert!(source.contains(".netdev_budget_usecs = 2 * USEC_PER_SEC / HZ"));
        assert!(source.contains(".tstamp_prequeue = 1"));
        assert!(source.contains(".max_backlog = 1000"));
        assert!(source.contains(".qdisc_max_burst = 1000"));
        assert!(source.contains(".dev_tx_weight = 64"));
        assert!(source.contains(".dev_rx_weight = 64"));
        assert!(source.contains(".sysctl_max_skb_frags = MAX_SKB_FRAGS"));
        assert!(source.contains(".sysctl_skb_defer_max = 128"));
        assert!(source.contains(".sysctl_mem_pcpu_rsv = SK_MEMORY_PCPU_RESERVE"));
        assert!(source.contains("EXPORT_SYMBOL(net_hotdata);"));
        assert!(source.contains("struct net_aligned_data net_aligned_data;"));
        assert_eq!(NET_HOTDATA_DEFAULTS.gro_normal_batch, 8);
        assert_eq!(NET_HOTDATA_DEFAULTS.netdev_budget, 300);
        assert_eq!(NET_HOTDATA_DEFAULTS.netdev_budget_usecs, 8000);
        assert_eq!(NET_HOTDATA_DEFAULTS.max_backlog, 1000);
        assert_eq!(NET_HOTDATA_DEFAULTS.sysctl_max_skb_frags, 17);
        assert_eq!(NET_HOTDATA_DEFAULTS.sysctl_mem_pcpu_rsv, 256);
        assert_eq!(NET_ALIGNED_DATA, NetAlignedData);
    }
}
