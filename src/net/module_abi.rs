//! linux-parity: partial
//! linux-source: vendor/linux/net
//! Minimal networking ABI symbols required by Linux-built net driver modules.

use crate::kernel::module::{export_symbol, find_symbol};

const FUNCTION_STUB_EXPORTS: &[&str] = &[
    "__alloc_skb",
    "__kvmalloc_node_noprof",
    "__napi_schedule",
    "___pskb_trim",
    "__netdev_alloc_frag_align",
    "__netif_napi_del_locked",
    "__pskb_pull_tail",
    "__skb_pad",
    "__skb_flow_dissect",
    "__vlan_get_protocol_offset",
    "__xdp_rxq_info_reg",
    "alloc_etherdev_mqs",
    "bpf_warn_invalid_xdp_action",
    "build_skb",
    "dev_addr_mod",
    "dev_kfree_skb_any_reason",
    "do_trace_netlink_extack",
    "dst_release",
    "eth_commit_mac_addr_change",
    "eth_prepare_mac_addr_change",
    "eth_type_trans",
    "eth_validate_addr",
    "ethtool_op_get_link",
    "ethtool_op_get_ts_info",
    "ethtool_sprintf",
    "ethtool_convert_legacy_u32_to_link_mode",
    "ethtool_convert_link_mode_to_legacy_u32",
    "ethtool_virtdev_set_link_ksettings",
    "free_netdev",
    "get_random_bytes",
    "gro_receive_skb",
    "ktime_get",
    "napi_alloc_skb",
    "napi_build_skb",
    "napi_complete_done",
    "napi_consume_skb",
    "napi_disable",
    "napi_enable",
    "napi_get_frags",
    "napi_gro_frags",
    "napi_schedule_prep",
    "net_dim",
    "net_dim_free_irq_moder",
    "net_dim_get_rx_irq_moder",
    "net_dim_init_irq_moder",
    "net_dim_work_cancel",
    "net_failover_create",
    "net_failover_destroy",
    "net_ratelimit",
    "netdev_err",
    "netdev_info",
    "netdev_notify_peers",
    "netdev_printk",
    "netdev_rss_key_fill",
    "netdev_stat_queue_sum",
    "netdev_warn",
    "netif_carrier_off",
    "netif_carrier_on",
    "netif_device_attach",
    "netif_device_detach",
    "netif_is_rxfh_configured",
    "netif_napi_add_weight_locked",
    "netif_napi_set_irq_locked",
    "netif_queue_set_napi",
    "netif_set_real_num_rx_queues",
    "netif_set_real_num_tx_queues",
    "netif_tx_stop_all_queues",
    "netif_tx_wake_queue",
    "passthru_features_check",
    "page_frag_free",
    "register_netdevice",
    "register_netdev",
    "rtnl_lock",
    "rtnl_unlock",
    "sched_clock",
    "sg_init_one",
    "sg_init_table",
    "sized_strscpy",
    "sk_skb_reason_drop",
    "skb_coalesce_rx_frag",
    "skb_partial_csum_set",
    "skb_put",
    "skb_to_sgvec",
    "skb_trim",
    "skb_tstamp_tx",
    "pskb_expand_head",
    "snprintf",
    "sprintf",
    "synchronize_net",
    "unregister_netdev",
    "xdp_convert_zc_to_xdp_frame",
    "xdp_do_flush",
    "xdp_do_redirect",
    "xdp_features_clear_redirect_target",
    "xdp_features_set_redirect_target",
    "xdp_master_redirect",
    "xdp_return_frame",
    "xdp_return_frame_rx_napi",
    "xdp_rxq_info_reg_mem_model",
    "xdp_rxq_info_unreg",
    "xdp_warn",
];

static mut LINUX_BPF_MASTER_REDIRECT_ENABLED_KEY: usize = 0;
static mut LINUX_BPF_STATS_ENABLED_KEY: usize = 0;
static mut LINUX_FLOW_KEYS_BASIC_DISSECTOR: usize = 0;
static mut LINUX_NET_DIM_SETTING: usize = 0;
static mut LINUX_SOFTNET_DATA: usize = 0;

fn export_symbol_once(name: &str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    for name in FUNCTION_STUB_EXPORTS {
        export_symbol_once(name, linux_net_abi_ret0 as usize, false);
    }

    export_symbol_once(
        "bpf_master_redirect_enabled_key",
        core::ptr::addr_of_mut!(LINUX_BPF_MASTER_REDIRECT_ENABLED_KEY) as usize,
        false,
    );
    export_symbol_once(
        "bpf_stats_enabled_key",
        core::ptr::addr_of_mut!(LINUX_BPF_STATS_ENABLED_KEY) as usize,
        false,
    );
    export_symbol_once(
        "flow_keys_basic_dissector",
        core::ptr::addr_of_mut!(LINUX_FLOW_KEYS_BASIC_DISSECTOR) as usize,
        false,
    );
    export_symbol_once(
        "net_dim_setting",
        core::ptr::addr_of_mut!(LINUX_NET_DIM_SETTING) as usize,
        false,
    );
    export_symbol_once(
        "softnet_data",
        core::ptr::addr_of_mut!(LINUX_SOFTNET_DATA) as usize,
        false,
    );
}

extern "C" fn linux_net_abi_ret0() -> usize {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtio_net_link_stubs_include_expected_core_symbols() {
        assert!(FUNCTION_STUB_EXPORTS.contains(&"netif_tx_wake_queue"));
        assert!(FUNCTION_STUB_EXPORTS.contains(&"register_netdevice"));
        assert!(FUNCTION_STUB_EXPORTS.contains(&"xdp_rxq_info_reg_mem_model"));
    }

    #[test]
    fn net_stubs_do_not_shadow_shared_allocators() {
        for symbol in [
            "__kmalloc_cache_noprof",
            "__kmalloc_noprof",
            "devm_kfree",
            "devm_kmalloc",
            "kfree",
            "kmalloc_caches",
            "kmemdup_noprof",
            "kvfree",
        ] {
            assert!(!FUNCTION_STUB_EXPORTS.contains(&symbol));
        }
    }
}
