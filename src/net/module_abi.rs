//! linux-parity: partial
//! linux-source: vendor/linux/net
//! Minimal networking ABI symbols required by Linux-built net driver modules.

extern crate alloc;

use crate::include::uapi::errno::ENODEV;
use crate::kernel::module::{export_symbol, find_symbol};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

const FUNCTION_STUB_EXPORTS: &[&str] = &[
    "___pskb_trim",
    "__netdev_alloc_frag_align",
    "__pskb_pull_tail",
    "__skb_pad",
    "__skb_flow_dissect",
    "__vlan_get_protocol_offset",
    "__xdp_rxq_info_reg",
    "bpf_warn_invalid_xdp_action",
    "do_trace_netlink_extack",
    "dst_release",
    "eth_commit_mac_addr_change",
    "eth_prepare_mac_addr_change",
    "eth_validate_addr",
    "ethtool_op_get_link",
    "ethtool_op_get_ts_info",
    "ethtool_sprintf",
    "ethtool_convert_legacy_u32_to_link_mode",
    "ethtool_convert_link_mode_to_legacy_u32",
    "ethtool_virtdev_set_link_ksettings",
    "ktime_get",
    "napi_get_frags",
    "napi_gro_frags",
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
    "netdev_notice",
    "netdev_notify_peers",
    "netdev_printk",
    "netdev_rss_key_fill",
    "netdev_stat_queue_sum",
    "netdev_warn",
    "netif_is_rxfh_configured",
    "passthru_features_check",
    "page_frag_free",
    "sched_clock",
    "sized_strscpy",
    "sk_skb_reason_drop",
    "skb_partial_csum_set",
    "skb_trim",
    "skb_tstamp_tx",
    "pskb_expand_head",
    "snprintf",
    "sprintf",
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

fn export_symbol_once(name: &str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    #[cfg(not(test))]
    if !NET_RX_SOFTIRQ_REGISTERED.swap(true, core::sync::atomic::Ordering::AcqRel) {
        crate::kernel::softirq::open_softirq(
            crate::kernel::softirq::SoftIrqVec::NetRx,
            net_rx_action,
        );
    }
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
        crate::arch::x86::kernel::setup_percpu::softnet_data_symbol(),
        false,
    );
    export_symbol_once("synchronize_net", synchronize_net as usize, false);
    export_symbol_once("nf_conntrack_destroy", nf_conntrack_destroy as usize, false);
    export_symbol_once("netif_tx_lock", netif_tx_lock as usize, false);
    export_symbol_once("netif_tx_unlock", netif_tx_unlock as usize, false);
    export_symbol_once("netif_schedule_queue", netif_schedule_queue as usize, false);
    export_symbol_once("netif_tx_wake_queue", netif_tx_wake_queue as usize, false);
    export_symbol_once(
        "netif_tx_stop_all_queues",
        netif_tx_stop_all_queues as usize,
        false,
    );
    export_symbol_once(
        "__netif_set_xps_queue",
        __netif_set_xps_queue as usize,
        true,
    );
    export_symbol_once(
        "netif_get_num_default_rss_queues",
        netif_get_num_default_rss_queues as usize,
        false,
    );
    export_symbol_once("alloc_etherdev_mqs", alloc_etherdev_mqs as usize, false);
    export_symbol_once("free_netdev", free_netdev as usize, false);
    export_symbol_once("register_netdevice", register_netdevice as usize, false);
    export_symbol_once("register_netdev", register_netdev as usize, false);
    export_symbol_once("unregister_netdev", unregister_netdev as usize, false);
    export_symbol_once(
        "register_netdevice_notifier",
        register_netdevice_notifier as usize,
        false,
    );
    export_symbol_once(
        "unregister_netdevice_notifier",
        unregister_netdevice_notifier as usize,
        false,
    );
    export_symbol_once("rtnl_lock", rtnl_lock as usize, false);
    export_symbol_once("rtnl_unlock", rtnl_unlock as usize, false);
    export_symbol_once("rtnl_is_locked", rtnl_is_locked as usize, false);
    export_symbol_once("netif_carrier_on", netif_carrier_on as usize, false);
    export_symbol_once("netif_carrier_off", netif_carrier_off as usize, false);
    export_symbol_once("linkwatch_fire_event", linkwatch_fire_event as usize, false);
    export_symbol_once("netif_device_attach", netif_device_attach as usize, false);
    export_symbol_once("netif_device_detach", netif_device_detach as usize, false);
    export_symbol_once("dev_trans_start", dev_trans_start as usize, false);
    export_symbol_once(
        "netif_set_real_num_tx_queues",
        netif_set_real_num_tx_queues as usize,
        false,
    );
    export_symbol_once(
        "netif_set_real_num_rx_queues",
        netif_set_real_num_rx_queues as usize,
        false,
    );
    export_symbol_once(
        "netif_set_tso_max_size",
        netif_set_tso_max_size as usize,
        false,
    );
    export_symbol_once(
        "netif_set_tso_max_segs",
        netif_set_tso_max_segs as usize,
        false,
    );
    export_symbol_once("dev_addr_mod", dev_addr_mod as usize, false);
    export_symbol_once("eth_mac_addr", eth_mac_addr as usize, false);
    export_symbol_once("get_random_bytes", get_random_bytes as usize, false);
    export_symbol_once("get_random_u8", get_random_u8 as usize, false);
    export_symbol_once("get_random_u16", get_random_u16 as usize, false);
    export_symbol_once("get_random_u32", get_random_u32 as usize, false);
    export_symbol_once("get_random_u64", get_random_u64 as usize, false);
    export_symbol_once(
        "__get_random_u32_below",
        __get_random_u32_below as usize,
        false,
    );
    export_symbol_once("__napi_schedule", __napi_schedule as usize, false);
    export_symbol_once(
        "__napi_schedule_irqoff",
        __napi_schedule_irqoff as usize,
        false,
    );
    export_symbol_once("napi_schedule_prep", napi_schedule_prep as usize, false);
    export_symbol_once("napi_complete_done", napi_complete_done as usize, false);
    export_symbol_once("napi_enable", napi_enable as usize, false);
    export_symbol_once("napi_enable_locked", napi_enable_locked as usize, false);
    export_symbol_once("napi_disable", napi_disable as usize, false);
    export_symbol_once(
        "netif_napi_add_weight_locked",
        netif_napi_add_weight_locked as usize,
        false,
    );
    export_symbol_once(
        "netif_napi_set_irq_locked",
        netif_napi_set_irq_locked as usize,
        false,
    );
    export_symbol_once("netif_queue_set_napi", netif_queue_set_napi as usize, false);
    export_symbol_once(
        "__netif_napi_del_locked",
        __netif_napi_del_locked as usize,
        false,
    );
    export_symbol_once("__netdev_alloc_skb", __netdev_alloc_skb as usize, false);
    export_symbol_once("__alloc_skb", __alloc_skb as usize, false);
    export_symbol_once(
        "devm_alloc_etherdev_mqs",
        devm_alloc_etherdev_mqs as usize,
        false,
    );
    export_symbol_once(
        "__napi_alloc_frag_align",
        __napi_alloc_frag_align as usize,
        false,
    );
    export_symbol_once("build_skb", build_skb as usize, false);
    export_symbol_once("slab_build_skb", slab_build_skb as usize, false);
    export_symbol_once("napi_build_skb", napi_build_skb as usize, false);
    export_symbol_once("napi_alloc_skb", napi_alloc_skb as usize, false);
    export_symbol_once("skb_put", linux_skb_put as usize, false);
    export_symbol_once("skb_push", linux_skb_push as usize, false);
    export_symbol_once("skb_dequeue", linux_skb_dequeue as usize, false);
    export_symbol_once("eth_type_trans", eth_type_trans as usize, false);
    export_symbol_once(
        "device_get_mac_address",
        device_get_mac_address as usize,
        false,
    );
    export_symbol_once(
        "eth_platform_get_mac_address",
        eth_platform_get_mac_address as usize,
        false,
    );
    export_symbol_once("gro_receive_skb", gro_receive_skb as usize, false);
    export_symbol_once("netif_receive_skb", netif_receive_skb as usize, false);
    export_symbol_once(
        "dev_kfree_skb_any_reason",
        dev_kfree_skb_any_reason as usize,
        false,
    );
    export_symbol_once(
        "dev_kfree_skb_irq_reason",
        dev_kfree_skb_irq_reason as usize,
        false,
    );
    export_symbol_once("napi_consume_skb", napi_consume_skb as usize, false);
    export_symbol_once("consume_skb", consume_skb as usize, false);
    export_symbol_once(
        "skb_copy_and_csum_dev",
        skb_copy_and_csum_dev as usize,
        false,
    );
    export_symbol_once("skb_copy_bits", skb_copy_bits as usize, false);
    export_symbol_once("skb_copy", skb_copy as usize, false);
    export_symbol_once("skb_copy_expand", skb_copy_expand as usize, false);
    export_symbol_once("skb_checksum_help", skb_checksum_help as usize, false);
    export_symbol_once("__skb_gso_segment", __skb_gso_segment as usize, false);
    export_symbol_once("skb_coalesce_rx_frag", skb_coalesce_rx_frag as usize, false);
    export_symbol_once("skb_to_sgvec", skb_to_sgvec as usize, false);
    export_symbol_once("dev_close", dev_close as usize, false);
    export_symbol_once(
        "netdev_update_features",
        netdev_update_features as usize,
        false,
    );
    export_symbol_once(
        "netdev_sw_irq_coalesce_default_on",
        netdev_sw_irq_coalesce_default_on as usize,
        true,
    );
    export_symbol_once(
        "netdev_stats_to_stats64",
        netdev_stats_to_stats64 as usize,
        false,
    );
    export_symbol_once(
        "dev_fetch_sw_netstats",
        dev_fetch_sw_netstats as usize,
        true,
    );
    export_symbol_once("dev_get_stats", dev_get_stats as usize, false);
    export_symbol_once("ethtool_puts", ethtool_puts as usize, false);
    export_symbol_once(
        "link_mode_params",
        LINUX_LINK_MODE_PARAMS.as_ptr() as usize,
        true,
    );
    export_symbol_once(
        "ethtool_set_ethtool_phy_ops",
        ethtool_set_ethtool_phy_ops as usize,
        true,
    );
    export_symbol_once(
        "ethtool_str_to_medium",
        ethtool_str_to_medium as usize,
        true,
    );
    export_symbol_once(
        "ethnl_cable_test_alloc",
        ethnl_cable_test_alloc as usize,
        true,
    );
    export_symbol_once(
        "ethnl_cable_test_free",
        ethnl_cable_test_free as usize,
        true,
    );
    export_symbol_once(
        "ethnl_cable_test_finished",
        ethnl_cable_test_finished as usize,
        true,
    );
    export_symbol_once(
        "ethnl_cable_test_result_with_src",
        ethnl_cable_test_result_with_src as usize,
        true,
    );
    export_symbol_once(
        "ethnl_cable_test_fault_length_with_src",
        ethnl_cable_test_fault_length_with_src as usize,
        true,
    );
    export_symbol_once(
        "phylib_stubs",
        core::ptr::addr_of_mut!(LINUX_PHYLIB_STUBS) as usize,
        true,
    );
    export_symbol_once("ptp_clock_register", ptp_clock_register as usize, false);
    export_symbol_once("ptp_clock_unregister", ptp_clock_unregister as usize, false);
    export_symbol_once("ptp_clock_index", ptp_clock_index as usize, false);
    export_symbol_once("ptp_schedule_worker", ptp_schedule_worker as usize, false);
    export_symbol_once("netpoll_setup", netpoll_setup as usize, false);
    export_symbol_once("netpoll_cleanup", netpoll_cleanup as usize, false);
    export_symbol_once("do_netpoll_cleanup", do_netpoll_cleanup as usize, false);
    export_symbol_once("netpoll_poll_dev", netpoll_poll_dev as usize, false);
    export_symbol_once("netpoll_send_skb", netpoll_send_skb as usize, false);
    export_symbol_once(
        "netpoll_zap_completion_queue",
        netpoll_zap_completion_queue as usize,
        true,
    );
}

extern "C" fn linux_net_abi_ret0() -> usize {
    0
}

#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxLinkModeInfo {
    speed: i32,
    lanes: u8,
    min_pairs: u8,
    pairs: u8,
    duplex: u8,
    mediums: u16,
}

const LINUX_ETHTOOL_LINK_MODE_MASK_NBITS: usize = 125;
const LINUX_SPEED_UNKNOWN: i32 = -1;
const LINUX_ETHTOOL_LINK_MEDIUM_NONE: i32 = 10;
const LINUX_EOPNOTSUPP: i32 = 95;
const NET_XMIT_DROP: i32 = 0x01;

const LINUX_LINK_MODE_UNKNOWN: LinuxLinkModeInfo = LinuxLinkModeInfo {
    speed: LINUX_SPEED_UNKNOWN,
    lanes: 0,
    min_pairs: 0,
    pairs: 0,
    duplex: 0,
    mediums: 0,
};

static LINUX_LINK_MODE_PARAMS: [LinuxLinkModeInfo; LINUX_ETHTOOL_LINK_MODE_MASK_NBITS] =
    [LINUX_LINK_MODE_UNKNOWN; LINUX_ETHTOOL_LINK_MODE_MASK_NBITS];
static mut LINUX_ETHTOOL_PHY_OPS: usize = 0;
static mut LINUX_PHYLIB_STUBS: usize = 0;

const NAPIF_STATE_SCHED: u64 = 1 << 0;
const NAPIF_STATE_MISSED: u64 = 1 << 1;
const NAPIF_STATE_DISABLE: u64 = 1 << 2;
const NAPIF_STATE_NPSVC: u64 = 1 << 3;
const NAPIF_STATE_LISTED: u64 = 1 << 4;
const NAPIF_STATE_NO_BUSY_POLL: u64 = 1 << 5;
const NAPIF_STATE_PREFER_BUSY_POLL: u64 = 1 << 7;
const NAPIF_STATE_SCHED_THREADED: u64 = 1 << 9;

const NAPI_POLL_LIST_OFFSET: usize = 8;
const NAPI_WEIGHT_OFFSET: usize = 24;
const NAPI_POLL_OFFSET: usize = 32;
const NAPI_POLL_OWNER_OFFSET: usize = 40;
const NAPI_LIST_OWNER_OFFSET: usize = 44;
const NAPI_DEV_OFFSET: usize = 48;
const NAPI_GRO_RX_LIST_OFFSET: usize = 264;
const NAPI_GRO_FLUSH_TIMEOUT_OFFSET: usize = 376;
const NAPI_IRQ_SUSPEND_TIMEOUT_OFFSET: usize = 384;
const NAPI_DEFER_HARD_IRQS_OFFSET: usize = 392;
const NAPI_ID_OFFSET: usize = 396;
const NAPI_DEV_LIST_OFFSET: usize = 400;
const NAPI_HASH_OFFSET: usize = 416;
const NAPI_IRQ_OFFSET: usize = 432;
const NAPI_RMAP_INDEX_OFFSET: usize = 496;
const NAPI_CONFIG_OFFSET: usize = 504;

static SCHEDULED_NAPI: spin::Mutex<Vec<usize>> = spin::Mutex::new(Vec::new());
static NETDEVICE_NOTIFIERS: spin::Mutex<Vec<usize>> = spin::Mutex::new(Vec::new());
static NEXT_NAPI_ID: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(1);
#[cfg(not(test))]
static NET_RX_SOFTIRQ_REGISTERED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

unsafe fn napi_state(napi: *mut u8) -> Option<&'static core::sync::atomic::AtomicU64> {
    if napi.is_null() {
        None
    } else {
        Some(unsafe { &*napi.cast::<core::sync::atomic::AtomicU64>() })
    }
}

unsafe fn init_raw_list_head(base: *mut u8, offset: usize) {
    let list = unsafe { base.add(offset).cast::<usize>() };
    unsafe {
        list.write(list as usize);
        list.add(1).write(list as usize);
    }
}

fn queue_napi(napi: *mut u8) {
    if napi.is_null() {
        return;
    }
    let address = napi as usize;
    let queued = {
        let mut pending = SCHEDULED_NAPI.lock();
        if pending.contains(&address) {
            false
        } else {
            pending.push(address);
            true
        }
    };
    #[cfg(not(test))]
    if queued {
        crate::kernel::softirq::raise_softirq(crate::kernel::softirq::SoftIrqVec::NetRx);
    }
}

#[cfg(not(test))]
fn net_rx_action() {
    let _ = poll_scheduled_napi();
}

/// Lupos's NET_RX softirq executor. Driver interrupt callbacks retain Linux's
/// NAPI state machine; this poller supplies the deferred execution point used
/// by the existing vendor-driver event pump.
fn poll_scheduled_napi() -> usize {
    let batch = {
        let mut pending = SCHEDULED_NAPI.lock();
        core::mem::take(&mut *pending)
    };
    let mut handled = 0usize;
    for address in batch.into_iter().take(256) {
        let napi = address as *mut u8;
        let Some(state) = (unsafe { napi_state(napi) }) else {
            continue;
        };
        let current = state.load(core::sync::atomic::Ordering::Acquire);
        if current & NAPIF_STATE_SCHED == 0 || current & NAPIF_STATE_DISABLE != 0 {
            continue;
        }
        let poll = unsafe { read_field::<usize>(napi, NAPI_POLL_OFFSET) };
        if poll == 0 {
            continue;
        }
        let weight = unsafe { read_field::<i32>(napi, NAPI_WEIGHT_OFFSET) }.max(1);
        let callback: unsafe extern "C" fn(*mut u8, i32) -> i32 =
            unsafe { core::mem::transmute(poll) };
        let work = unsafe { callback(napi, weight) };
        handled = handled
            .saturating_add(work.max(0) as usize)
            .saturating_add(1);
        if state.load(core::sync::atomic::Ordering::Acquire) & NAPIF_STATE_SCHED != 0 {
            queue_napi(napi);
        }
    }
    handled
}

/// `napi_schedule_prep()` — `vendor/linux/net/core/dev.c:6733`.
#[unsafe(no_mangle)]
unsafe extern "C" fn napi_schedule_prep(napi: *mut u8) -> bool {
    let Some(state) = (unsafe { napi_state(napi) }) else {
        return false;
    };
    let mut value = state.load(core::sync::atomic::Ordering::Acquire);
    loop {
        if value & NAPIF_STATE_DISABLE != 0 {
            return false;
        }
        let mut new = value | NAPIF_STATE_SCHED;
        if value & NAPIF_STATE_SCHED != 0 {
            new |= NAPIF_STATE_MISSED;
        }
        match state.compare_exchange_weak(
            value,
            new,
            core::sync::atomic::Ordering::AcqRel,
            core::sync::atomic::Ordering::Acquire,
        ) {
            Ok(_) => return value & NAPIF_STATE_SCHED == 0,
            Err(observed) => value = observed,
        }
    }
}

/// `__napi_schedule()` — defers the poll callback until the driver event pump
/// leaves the virtqueue callback, matching Linux's hardirq-to-NET_RX ordering.
#[unsafe(no_mangle)]
unsafe extern "C" fn __napi_schedule(napi: *mut u8) {
    if let Some(state) = unsafe { napi_state(napi) }
        && state.load(core::sync::atomic::Ordering::Acquire) & NAPIF_STATE_SCHED != 0
    {
        queue_napi(napi);
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn __napi_schedule_irqoff(napi: *mut u8) {
    unsafe { __napi_schedule(napi) };
}

/// `napi_complete_done()` — configured non-threaded, no-GRO-timeout branch.
#[unsafe(no_mangle)]
unsafe extern "C" fn napi_complete_done(napi: *mut u8, _work_done: i32) -> bool {
    let Some(state) = (unsafe { napi_state(napi) }) else {
        return false;
    };
    let mut value = state.load(core::sync::atomic::Ordering::Acquire);
    loop {
        if value & NAPIF_STATE_NPSVC != 0 {
            return false;
        }
        let mut new = value
            & !(NAPIF_STATE_MISSED
                | NAPIF_STATE_SCHED
                | NAPIF_STATE_SCHED_THREADED
                | NAPIF_STATE_PREFER_BUSY_POLL);
        if value & NAPIF_STATE_MISSED != 0 {
            new |= NAPIF_STATE_SCHED;
        }
        match state.compare_exchange_weak(
            value,
            new,
            core::sync::atomic::Ordering::AcqRel,
            core::sync::atomic::Ordering::Acquire,
        ) {
            Ok(_) => {
                if value & NAPIF_STATE_MISSED != 0 {
                    queue_napi(napi);
                    return false;
                }
                return true;
            }
            Err(observed) => value = observed,
        }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn napi_enable(napi: *mut u8) {
    let Some(state) = (unsafe { napi_state(napi) }) else {
        return;
    };
    let config = unsafe { read_field::<*mut u8>(napi, NAPI_CONFIG_OFFSET) };
    if !config.is_null() {
        unsafe {
            write_field(
                napi,
                NAPI_GRO_FLUSH_TIMEOUT_OFFSET,
                read_field::<u64>(config, 0),
            );
            write_field(
                napi,
                NAPI_IRQ_SUSPEND_TIMEOUT_OFFSET,
                read_field::<u64>(config, 8),
            );
            write_field(
                napi,
                NAPI_DEFER_HARD_IRQS_OFFSET,
                read_field::<u32>(config, 16),
            );
        }
    }
    state.fetch_and(
        !(NAPIF_STATE_SCHED | NAPIF_STATE_NPSVC),
        core::sync::atomic::Ordering::AcqRel,
    );
}

#[unsafe(no_mangle)]
unsafe extern "C" fn napi_enable_locked(napi: *mut u8) {
    unsafe { napi_enable(napi) };
}

#[unsafe(no_mangle)]
unsafe extern "C" fn napi_disable(napi: *mut u8) {
    let Some(state) = (unsafe { napi_state(napi) }) else {
        return;
    };
    state.fetch_or(NAPIF_STATE_DISABLE, core::sync::atomic::Ordering::AcqRel);
    SCHEDULED_NAPI
        .lock()
        .retain(|entry| *entry != napi as usize);
    state
        .fetch_update(
            core::sync::atomic::Ordering::AcqRel,
            core::sync::atomic::Ordering::Acquire,
            |value| {
                Some(
                    (value | NAPIF_STATE_SCHED | NAPIF_STATE_NPSVC)
                        & !(NAPIF_STATE_DISABLE
                            | NAPIF_STATE_MISSED
                            | NAPIF_STATE_SCHED_THREADED
                            | NAPIF_STATE_PREFER_BUSY_POLL),
                )
            },
        )
        .ok();
}

/// `netif_napi_add_weight_locked()` — initializes the exact configured NAPI
/// fields consumed inline by `virtio_net.ko`.
#[unsafe(no_mangle)]
unsafe extern "C" fn netif_napi_add_weight_locked(
    dev: *mut u8,
    napi: *mut u8,
    poll: usize,
    weight: i32,
) {
    if dev.is_null() || napi.is_null() || poll == 0 {
        return;
    }
    let Some(state) = (unsafe { napi_state(napi) }) else {
        return;
    };
    let preserved = state.load(core::sync::atomic::Ordering::Acquire) & NAPIF_STATE_NO_BUSY_POLL;
    unsafe {
        init_raw_list_head(napi, NAPI_POLL_LIST_OFFSET);
        init_raw_list_head(napi, NAPI_GRO_RX_LIST_OFFSET);
        init_raw_list_head(napi, NAPI_DEV_LIST_OFFSET);
        write_field(napi, NAPI_WEIGHT_OFFSET, weight);
        write_field(napi, NAPI_POLL_OFFSET, poll);
        write_field(napi, NAPI_POLL_OWNER_OFFSET, -1i32);
        write_field(napi, NAPI_LIST_OWNER_OFFSET, -1i32);
        write_field(napi, NAPI_DEV_OFFSET, dev);
        write_field(napi, NAPI_HASH_OFFSET, 0usize);
        write_field(napi, NAPI_HASH_OFFSET + 8, 0usize);
        write_field(napi, NAPI_IRQ_OFFSET, -1i32);
        write_field(napi, NAPI_RMAP_INDEX_OFFSET, -1i32);
    }
    let config = unsafe { read_field::<*mut u8>(napi, NAPI_CONFIG_OFFSET) };
    let mut id = if config.is_null() {
        0
    } else {
        unsafe { read_field::<u32>(config, 36) }
    };
    if id == 0 {
        id = NEXT_NAPI_ID.fetch_add(1, core::sync::atomic::Ordering::AcqRel);
        if !config.is_null() {
            unsafe { write_field(config, 36, id) };
        }
    }
    unsafe { write_field(napi, NAPI_ID_OFFSET, id) };
    state.store(
        preserved | NAPIF_STATE_LISTED | NAPIF_STATE_SCHED | NAPIF_STATE_NPSVC,
        core::sync::atomic::Ordering::Release,
    );
}

#[unsafe(no_mangle)]
unsafe extern "C" fn netif_napi_set_irq_locked(napi: *mut u8, irq: i32) {
    if !napi.is_null() {
        unsafe { write_field(napi, NAPI_IRQ_OFFSET, irq) };
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn netif_queue_set_napi(
    dev: *mut u8,
    queue_index: u32,
    queue_type: u32,
    napi: *mut u8,
) {
    if dev.is_null() {
        return;
    }
    match queue_type {
        0 => {
            let queues = unsafe { read_field::<*mut u8>(dev, NET_DEVICE_RX_PTR_OFFSET) };
            let count = unsafe { read_field::<u32>(dev, NET_DEVICE_NUM_RX_QUEUES_OFFSET) };
            if !queues.is_null() && queue_index < count {
                unsafe {
                    write_field(
                        queues.add(queue_index as usize * NETDEV_RX_QUEUE_SIZE),
                        160,
                        napi,
                    )
                };
            }
        }
        1 => {
            let queues = unsafe { read_field::<*mut u8>(dev, NET_DEVICE_TX_PTR_OFFSET) };
            let count = unsafe { read_field::<u32>(dev, NET_DEVICE_NUM_TX_QUEUES_OFFSET) };
            if !queues.is_null() && queue_index < count {
                unsafe {
                    write_field(
                        queues.add(queue_index as usize * NETDEV_QUEUE_SIZE),
                        280,
                        napi,
                    )
                };
            }
        }
        _ => {}
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn __netif_napi_del_locked(napi: *mut u8) {
    let Some(state) = (unsafe { napi_state(napi) }) else {
        return;
    };
    if state.fetch_and(!NAPIF_STATE_LISTED, core::sync::atomic::Ordering::AcqRel)
        & NAPIF_STATE_LISTED
        == 0
    {
        return;
    }
    SCHEDULED_NAPI
        .lock()
        .retain(|entry| *entry != napi as usize);
    unsafe {
        write_field(napi, NAPI_DEV_OFFSET, core::ptr::null_mut::<u8>());
        write_field(napi, NAPI_POLL_OFFSET, 0usize);
    }
}

const SKB_SIZE: usize = 232;
const SKB_SHARED_INFO_SIZE: usize = 320;
const SKB_DEV_OFFSET: usize = 16;
const SKB_LEN_OFFSET: usize = 112;
const SKB_DATA_LEN_OFFSET: usize = 116;
const SKB_MAC_LEN_OFFSET: usize = 120;
const SKB_FLAGS_OFFSET: usize = 126;
const SKB_PKT_TYPE_OFFSET: usize = 128;
const SKB_ALLOC_CPU_OFFSET: usize = 136;
const SKB_NAPI_ID_OFFSET: usize = 160;
const SKB_PROTOCOL_OFFSET: usize = 180;
const SKB_TRANSPORT_HEADER_OFFSET: usize = 182;
const SKB_NETWORK_HEADER_OFFSET: usize = 184;
const SKB_MAC_HEADER_OFFSET: usize = 186;
const SKB_TAIL_OFFSET: usize = 188;
const SKB_END_OFFSET: usize = 192;
const SKB_HEAD_OFFSET: usize = 200;
const SKB_DATA_OFFSET: usize = 208;
const SKB_TRUESIZE_OFFSET: usize = 216;
const SKB_USERS_OFFSET: usize = 220;
const SKB_IP_SUMMED_MASK: u8 = 0b0110_0000;

fn linux_net_err_ptr(errno: i32) -> *mut u8 {
    let errno = if errno < 0 { errno } else { -errno };
    errno as isize as usize as *mut u8
}

unsafe fn initialize_raw_skb(skb: *mut u8, head: *mut u8, allocation_size: usize) -> bool {
    if skb.is_null()
        || head.is_null()
        || allocation_size < SKB_SHARED_INFO_SIZE
        || allocation_size - SKB_SHARED_INFO_SIZE > u32::MAX as usize
    {
        return false;
    }
    let end = allocation_size - SKB_SHARED_INFO_SIZE;
    unsafe {
        core::ptr::write_bytes(skb, 0, SKB_SIZE);
        write_field(skb, SKB_HEAD_OFFSET, head);
        write_field(skb, SKB_DATA_OFFSET, head);
        write_field(skb, SKB_TAIL_OFFSET, 0u32);
        write_field(skb, SKB_END_OFFSET, end as u32);
        write_field(
            skb,
            SKB_TRUESIZE_OFFSET,
            SKB_SIZE.saturating_add(allocation_size) as u32,
        );
        write_field(skb, SKB_USERS_OFFSET, 1i32);
        write_field(skb, SKB_MAC_HEADER_OFFSET, u16::MAX);
        write_field(skb, SKB_NETWORK_HEADER_OFFSET, u16::MAX);
        write_field(skb, SKB_TRANSPORT_HEADER_OFFSET, u16::MAX);
        write_field(
            skb,
            SKB_ALLOC_CPU_OFFSET,
            crate::kernel::sched::current_cpu() as u16,
        );
        let shinfo = head.add(end);
        core::ptr::write_bytes(shinfo, 0, SKB_SHARED_INFO_SIZE);
        write_field(shinfo, 32, 1i32);
    }
    true
}

/// `__alloc_skb()` — the non-fclone allocation branch from
/// `vendor/linux/net/core/skbuff.c:650`.
#[unsafe(no_mangle)]
unsafe extern "C" fn __alloc_skb(
    size: u32,
    gfp: crate::mm::page_flags::GfpFlags,
    _flags: i32,
    _node: i32,
) -> *mut u8 {
    let data_size = (size as usize).next_multiple_of(64);
    let Some(allocation_size) = data_size.checked_add(SKB_SHARED_INFO_SIZE) else {
        return core::ptr::null_mut();
    };
    let skb = unsafe { crate::mm::slab::kzalloc_noprof(SKB_SIZE, gfp) };
    if skb.is_null() {
        return core::ptr::null_mut();
    }
    let head = unsafe { crate::mm::slab::kzalloc_noprof(allocation_size, gfp) };
    if head.is_null() || !unsafe { initialize_raw_skb(skb, head, allocation_size) } {
        if !head.is_null() {
            unsafe { crate::mm::slab::kfree(head) };
        }
        unsafe { crate::mm::slab::kfree(skb) };
        return core::ptr::null_mut();
    }
    skb
}

/// `__napi_alloc_frag_align()` — aligned NAPI fragment allocation used by
/// drivers before attaching the returned buffer to an skb.
#[unsafe(no_mangle)]
unsafe extern "C" fn __napi_alloc_frag_align(fragsz: u32, align_mask: u32) -> *mut u8 {
    let size = (fragsz as usize).next_multiple_of(64);
    let align = (!align_mask).wrapping_add(1).max(1) as usize;
    let extra = align.saturating_sub(1);
    let Some(allocation_size) = size.checked_add(extra) else {
        return core::ptr::null_mut();
    };
    let raw = unsafe {
        crate::mm::slab::kzalloc_noprof(allocation_size, crate::mm::page_flags::GFP_ATOMIC)
    };
    if raw.is_null() || !align.is_power_of_two() {
        return raw;
    }
    let aligned = (raw as usize + extra) & !extra;
    aligned as *mut u8
}

/// `build_skb()` — attaches an skb header to a page/page-fragment buffer.
#[unsafe(no_mangle)]
unsafe extern "C" fn build_skb(data: *mut u8, frag_size: u32) -> *mut u8 {
    if data.is_null() || frag_size as usize <= SKB_SHARED_INFO_SIZE {
        return core::ptr::null_mut();
    }
    let skb =
        unsafe { crate::mm::slab::kzalloc_noprof(SKB_SIZE, crate::mm::page_flags::GFP_ATOMIC) };
    if skb.is_null() || !unsafe { initialize_raw_skb(skb, data, frag_size as usize) } {
        if !skb.is_null() {
            unsafe { crate::mm::slab::kfree(skb) };
        }
        return core::ptr::null_mut();
    }
    unsafe {
        let flags = read_field::<u8>(skb, SKB_FLAGS_OFFSET) | (1 << 5);
        write_field(skb, SKB_FLAGS_OFFSET, flags);
    }
    skb
}

/// `slab_build_skb()` — build an skb around a kmalloc/slab data buffer.
#[unsafe(no_mangle)]
unsafe extern "C" fn slab_build_skb(data: *mut u8) -> *mut u8 {
    if data.is_null() {
        return core::ptr::null_mut();
    }
    let size = crate::mm::slab::ksize(data);
    if size <= SKB_SHARED_INFO_SIZE || size > u32::MAX as usize {
        return core::ptr::null_mut();
    }
    let skb =
        unsafe { crate::mm::slab::kzalloc_noprof(SKB_SIZE, crate::mm::page_flags::GFP_ATOMIC) };
    if skb.is_null() || !unsafe { initialize_raw_skb(skb, data, size) } {
        if !skb.is_null() {
            unsafe { crate::mm::slab::kfree(skb) };
        }
        return core::ptr::null_mut();
    }
    skb
}

#[unsafe(no_mangle)]
unsafe extern "C" fn napi_build_skb(data: *mut u8, frag_size: u32) -> *mut u8 {
    unsafe { build_skb(data, frag_size) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn napi_alloc_skb(napi: *mut u8, length: u32) -> *mut u8 {
    let Some(size) = length.checked_add(34) else {
        return core::ptr::null_mut();
    };
    let skb = unsafe { __alloc_skb(size, crate::mm::page_flags::GFP_ATOMIC, 0, -1) };
    if skb.is_null() {
        return skb;
    }
    let data = unsafe { read_field::<*mut u8>(skb, SKB_DATA_OFFSET) };
    unsafe {
        write_field(skb, SKB_DATA_OFFSET, data.add(34));
        write_field(skb, SKB_TAIL_OFFSET, 34u32);
        if !napi.is_null() {
            write_field(
                skb,
                SKB_DEV_OFFSET,
                read_field::<*mut u8>(napi, NAPI_DEV_OFFSET),
            );
            write_field(
                skb,
                SKB_NAPI_ID_OFFSET,
                read_field::<u32>(napi, NAPI_ID_OFFSET),
            );
        }
    }
    skb
}

#[unsafe(no_mangle)]
unsafe extern "C" fn __netdev_alloc_skb(
    dev: *mut u8,
    length: u32,
    gfp: crate::mm::page_flags::GfpFlags,
) -> *mut u8 {
    let Some(size) = length.checked_add(34) else {
        return core::ptr::null_mut();
    };
    let skb = unsafe { __alloc_skb(size, gfp, 0, -1) };
    if skb.is_null() {
        return skb;
    }
    let data = unsafe { read_field::<*mut u8>(skb, SKB_DATA_OFFSET) };
    unsafe {
        write_field(skb, SKB_DATA_OFFSET, data.add(34));
        write_field(skb, SKB_TAIL_OFFSET, 34u32);
        write_field(skb, SKB_DEV_OFFSET, dev);
    }
    skb
}

/// Checked `skb_put()` used by module code when compiler inlining does not
/// select `__skb_put()`.
#[unsafe(no_mangle)]
unsafe extern "C" fn linux_skb_put(skb: *mut u8, length: u32) -> *mut u8 {
    if skb.is_null() {
        return core::ptr::null_mut();
    }
    let tail = unsafe { read_field::<u32>(skb, SKB_TAIL_OFFSET) };
    let end = unsafe { read_field::<u32>(skb, SKB_END_OFFSET) };
    let Some(next_tail) = tail.checked_add(length) else {
        return core::ptr::null_mut();
    };
    if next_tail > end {
        return core::ptr::null_mut();
    }
    let head = unsafe { read_field::<*mut u8>(skb, SKB_HEAD_OFFSET) };
    let len = unsafe { read_field::<u32>(skb, SKB_LEN_OFFSET) };
    unsafe {
        write_field(skb, SKB_TAIL_OFFSET, next_tail);
        write_field(skb, SKB_LEN_OFFSET, len.saturating_add(length));
        head.add(tail as usize)
    }
}

/// Checked `skb_push()` used by netpoll frame assembly when it is not inlined.
#[unsafe(no_mangle)]
unsafe extern "C" fn linux_skb_push(skb: *mut u8, length: u32) -> *mut u8 {
    if skb.is_null() {
        return core::ptr::null_mut();
    }
    let data = unsafe { read_field::<*mut u8>(skb, SKB_DATA_OFFSET) };
    let head = unsafe { read_field::<*mut u8>(skb, SKB_HEAD_OFFSET) };
    if data.is_null() || head.is_null() || (data as usize) < (head as usize) {
        return core::ptr::null_mut();
    }
    let headroom = (data as usize) - (head as usize);
    let length = length as usize;
    if length > headroom {
        return core::ptr::null_mut();
    }
    let len = unsafe { read_field::<u32>(skb, SKB_LEN_OFFSET) };
    let Some(next_len) = len.checked_add(length as u32) else {
        return core::ptr::null_mut();
    };
    let new_data = unsafe { data.sub(length) };
    unsafe {
        write_field(skb, SKB_DATA_OFFSET, new_data);
        write_field(skb, SKB_LEN_OFFSET, next_len);
    }
    new_data
}

/// `skb_dequeue()` — `vendor/linux/net/core/skbuff.c:3965`.
#[unsafe(no_mangle)]
unsafe extern "C" fn linux_skb_dequeue(_list: *mut u8) -> *mut u8 {
    core::ptr::null_mut()
}

/// `eth_type_trans()` — Ethernet header classification and pull for the
/// ordinary Ethernet-II receive path used by virtio-net.
#[unsafe(no_mangle)]
unsafe extern "C" fn eth_type_trans(skb: *mut u8, dev: *mut u8) -> u16 {
    if skb.is_null() {
        return 0;
    }
    let data = unsafe { read_field::<*mut u8>(skb, SKB_DATA_OFFSET) };
    let head = unsafe { read_field::<*mut u8>(skb, SKB_HEAD_OFFSET) };
    let len = unsafe { read_field::<u32>(skb, SKB_LEN_OFFSET) };
    if data.is_null() || head.is_null() || len < 14 {
        return 0;
    }
    let mac = unsafe { data.offset_from(head) };
    if mac < 0 || mac > u16::MAX as isize {
        return 0;
    }
    let protocol = unsafe { data.add(12).cast::<u16>().read_unaligned() };
    let destination = unsafe { core::slice::from_raw_parts(data, 6) };
    let packet_type = if destination == [0xff; 6] {
        1u8
    } else if destination[0] & 1 != 0 {
        2u8
    } else {
        0u8
    };
    unsafe {
        write_field(skb, SKB_DEV_OFFSET, dev);
        write_field(skb, SKB_MAC_HEADER_OFFSET, mac as u16);
        write_field(skb, SKB_MAC_LEN_OFFSET, 14u16);
        write_field(skb, SKB_PKT_TYPE_OFFSET, packet_type);
        write_field(skb, SKB_PROTOCOL_OFFSET, protocol);
        write_field(skb, SKB_DATA_OFFSET, data.add(14));
        write_field(skb, SKB_LEN_OFFSET, len - 14);
    }
    protocol
}

/// `device_get_mac_address()` — Lupos has no firmware node or NVMEM MAC
/// provider, so match Linux's no-address result for those backends.
#[unsafe(no_mangle)]
unsafe extern "C" fn device_get_mac_address(_dev: *mut u8, _addr: *mut u8) -> i32 {
    -crate::include::uapi::errno::ENOENT
}

/// `eth_platform_get_mac_address()` — `vendor/linux/net/ethernet/eth.c:484`.
///
/// Lupos currently has neither Open Firmware MAC properties nor an
/// arch-specific platform MAC provider, matching Linux's `-ENODEV` fallback.
#[unsafe(no_mangle)]
unsafe extern "C" fn eth_platform_get_mac_address(_dev: *mut u8, _addr: *mut u8) -> i32 {
    -crate::include::uapi::errno::ENODEV
}

unsafe fn raw_skb_frame(skb: *mut u8) -> Option<Vec<u8>> {
    let head = unsafe { read_field::<*mut u8>(skb, SKB_HEAD_OFFSET) };
    let data = unsafe { read_field::<*mut u8>(skb, SKB_DATA_OFFSET) };
    let len = unsafe { read_field::<u32>(skb, SKB_LEN_OFFSET) } as usize;
    let data_len = unsafe { read_field::<u32>(skb, SKB_DATA_LEN_OFFSET) } as usize;
    let mac_offset = unsafe { read_field::<u16>(skb, SKB_MAC_HEADER_OFFSET) } as usize;
    if head.is_null() || data.is_null() || data_len > len {
        return None;
    }
    let mut frame = Vec::new();
    frame.try_reserve_exact(14usize.checked_add(len)?).ok()?;
    frame.extend_from_slice(unsafe { core::slice::from_raw_parts(head.add(mac_offset), 14) });
    let linear = len - data_len;
    frame.extend_from_slice(unsafe { core::slice::from_raw_parts(data, linear) });
    if data_len != 0 {
        let end = unsafe { read_field::<u32>(skb, SKB_END_OFFSET) } as usize;
        let shinfo = unsafe { head.add(end) };
        let nr_frags = unsafe { read_field::<u8>(shinfo, 2) } as usize;
        for index in 0..nr_frags.min(17) {
            let frag = unsafe { shinfo.add(48 + index * 16) };
            let netmem = unsafe { read_field::<usize>(frag, 0) };
            let frag_len = unsafe { read_field::<u32>(frag, 8) } as usize;
            let offset = unsafe { read_field::<u32>(frag, 12) } as usize;
            let page = (netmem & !3) as *mut crate::mm::page::Page;
            if page.is_null() || !crate::mm::buddy::page_in_mem_map(page) {
                return None;
            }
            let address = unsafe {
                crate::arch::x86::mm::paging::pfn_to_virt(crate::mm::buddy::page_to_pfn(page))
                    .add(offset)
            };
            frame.extend_from_slice(unsafe { core::slice::from_raw_parts(address, frag_len) });
        }
    }
    frame.truncate(14 + len);
    Some(frame)
}

unsafe fn free_raw_skb(skb: *mut u8) {
    if skb.is_null() {
        return;
    }
    let users = unsafe {
        &*skb
            .add(SKB_USERS_OFFSET)
            .cast::<core::sync::atomic::AtomicI32>()
    };
    if users.fetch_sub(1, core::sync::atomic::Ordering::AcqRel) > 1 {
        return;
    }
    let head = unsafe { read_field::<*mut u8>(skb, SKB_HEAD_OFFSET) };
    let end = unsafe { read_field::<u32>(skb, SKB_END_OFFSET) } as usize;
    if !head.is_null() {
        let shinfo = unsafe { head.add(end) };
        let nr_frags = unsafe { read_field::<u8>(shinfo, 2) } as usize;
        for index in 0..nr_frags.min(17) {
            let frag = unsafe { shinfo.add(48 + index * 16) };
            let netmem = unsafe { read_field::<usize>(frag, 0) };
            let page = (netmem & !3) as *mut crate::mm::page::Page;
            if !page.is_null() && crate::mm::buddy::page_in_mem_map(page) {
                if !unsafe { crate::net::core::page_pool::recycle_skb_page(page) } {
                    crate::mm::page_alloc::__free_pages(page, 0);
                }
            }
        }
        let flags = unsafe { read_field::<u8>(skb, SKB_FLAGS_OFFSET) };
        if flags & (1 << 5) != 0 {
            if let Some(phys) = crate::arch::x86::mm::paging::virt_to_phys(head as u64) {
                let page = crate::mm::buddy::pfn_to_page((phys as usize) >> 12);
                if crate::mm::buddy::page_in_mem_map(page) {
                    if !unsafe { crate::net::core::page_pool::recycle_skb_page(page) } {
                        crate::mm::page_alloc::__free_pages(page, 0);
                    }
                }
            }
        } else {
            unsafe { crate::mm::slab::kfree(head) };
        }
    }
    unsafe { crate::mm::slab::kfree(skb) };
}

/// GRO entry point. With no GRO handlers registered, Linux takes
/// `GRO_NORMAL`; Lupos delivers that frame to its IPv4 socket core directly.
#[unsafe(no_mangle)]
unsafe extern "C" fn gro_receive_skb(_gro: *mut u8, skb: *mut u8) -> i32 {
    if skb.is_null() {
        return 4;
    }
    let dev = unsafe { read_field::<*mut u8>(skb, SKB_DEV_OFFSET) };
    if let Some(frame) = unsafe { raw_skb_frame(skb) } {
        #[cfg(not(test))]
        if crate::kernel::debug_trace::ping_enabled() && frame.len() >= 14 {
            crate::linux_driver_abi::tty::serial_println!(
                "net-rx: len={} ethertype={:04x}",
                frame.len(),
                u16::from_be_bytes([frame[12], frame[13]])
            );
        }
        crate::net::socket::receive_linux_ethernet_frame(dev, &frame);
    }
    unsafe { free_raw_skb(skb) };
    3
}

#[unsafe(no_mangle)]
unsafe extern "C" fn netif_receive_skb(skb: *mut u8) -> i32 {
    if skb.is_null() {
        return 1;
    }
    let dev = unsafe { read_field::<*mut u8>(skb, SKB_DEV_OFFSET) };
    if let Some(frame) = unsafe { raw_skb_frame(skb) } {
        crate::net::socket::receive_linux_ethernet_frame(dev, &frame);
    }
    unsafe { free_raw_skb(skb) };
    0
}

#[unsafe(no_mangle)]
unsafe extern "C" fn dev_kfree_skb_any_reason(skb: *mut u8, _reason: u32) {
    unsafe { free_raw_skb(skb) };
}

#[unsafe(no_mangle)]
unsafe extern "C" fn dev_kfree_skb_irq_reason(skb: *mut u8, _reason: u32) {
    unsafe { free_raw_skb(skb) };
}

#[unsafe(no_mangle)]
unsafe extern "C" fn napi_consume_skb(skb: *mut u8, _budget: i32) {
    unsafe { free_raw_skb(skb) };
}

/// `consume_skb()` drops the skb users reference and releases the complete
/// buffer when it reaches zero. `free_raw_skb()` owns the same Linux-layout
/// reference and fragment teardown used by the receive/NAPI entry points.
#[unsafe(no_mangle)]
unsafe extern "C" fn consume_skb(skb: *mut u8) {
    unsafe { free_raw_skb(skb) };
}

#[unsafe(no_mangle)]
unsafe extern "C" fn skb_copy_and_csum_dev(skb: *mut u8, to: *mut u8) {
    if skb.is_null() || to.is_null() {
        return;
    }
    if let Some(frame) = unsafe { raw_skb_frame(skb) } {
        unsafe { core::ptr::copy_nonoverlapping(frame.as_ptr(), to, frame.len()) };
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn skb_copy_bits(skb: *mut u8, offset: i32, to: *mut u8, len: i32) -> i32 {
    if skb.is_null() || to.is_null() || offset < 0 || len < 0 {
        return -crate::include::uapi::errno::EFAULT;
    }
    if len == 0 {
        return 0;
    }

    let total_len = unsafe { read_field::<u32>(skb, SKB_LEN_OFFSET) } as usize;
    let data_len = unsafe { read_field::<u32>(skb, SKB_DATA_LEN_OFFSET) } as usize;
    let mut skip = offset as usize;
    let mut remaining = len as usize;
    if data_len > total_len || skip > total_len || remaining > total_len - skip {
        return -crate::include::uapi::errno::EFAULT;
    }

    let mut out = to;
    let linear_len = total_len - data_len;
    if skip < linear_len && remaining != 0 {
        let data = unsafe { read_field::<*mut u8>(skb, SKB_DATA_OFFSET) };
        if data.is_null() {
            return -crate::include::uapi::errno::EFAULT;
        }
        let count = remaining.min(linear_len - skip);
        unsafe {
            core::ptr::copy_nonoverlapping(data.add(skip), out, count);
            out = out.add(count);
        }
        remaining -= count;
        skip = 0;
    } else {
        skip = skip.saturating_sub(linear_len);
    }

    if remaining != 0 {
        let head = unsafe { read_field::<*mut u8>(skb, SKB_HEAD_OFFSET) };
        if head.is_null() {
            return -crate::include::uapi::errno::EFAULT;
        }
        let end = unsafe { read_field::<u32>(skb, SKB_END_OFFSET) } as usize;
        let shinfo = unsafe { head.add(end) };
        let nr_frags = unsafe { read_field::<u8>(shinfo, 2) } as usize;
        for index in 0..nr_frags.min(17) {
            let frag = unsafe { shinfo.add(48 + index * 16) };
            let frag_len = unsafe { read_field::<u32>(frag, 8) } as usize;
            if skip >= frag_len {
                skip -= frag_len;
                continue;
            }

            let count = remaining.min(frag_len - skip);
            let page = (unsafe { read_field::<usize>(frag, 0) } & !3) as *mut crate::mm::page::Page;
            let Some(page_offset) =
                (unsafe { read_field::<u32>(frag, 12) } as usize).checked_add(skip)
            else {
                return -crate::include::uapi::errno::EFAULT;
            };
            if page.is_null() || !crate::mm::buddy::page_in_mem_map(page) {
                return -crate::include::uapi::errno::EFAULT;
            }
            let address = unsafe {
                crate::arch::x86::mm::paging::pfn_to_virt(crate::mm::buddy::page_to_pfn(page))
                    .add(page_offset)
            };
            unsafe {
                core::ptr::copy_nonoverlapping(address, out, count);
                out = out.add(count);
            }
            remaining -= count;
            skip = 0;
            if remaining == 0 {
                break;
            }
        }
    }

    if remaining == 0 {
        0
    } else {
        -crate::include::uapi::errno::EFAULT
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn skb_copy_expand(
    skb: *mut u8,
    newheadroom: i32,
    newtailroom: i32,
    gfp: crate::mm::page_flags::GfpFlags,
) -> *mut u8 {
    if skb.is_null() || newheadroom < 0 || newtailroom < 0 {
        return core::ptr::null_mut();
    }
    let len = unsafe { read_field::<u32>(skb, SKB_LEN_OFFSET) } as usize;
    let Some(size) = (newheadroom as usize)
        .checked_add(len)
        .and_then(|size| size.checked_add(newtailroom as usize))
    else {
        return core::ptr::null_mut();
    };
    if size > u32::MAX as usize {
        return core::ptr::null_mut();
    }
    if len > i32::MAX as usize {
        return core::ptr::null_mut();
    }

    let copy = unsafe { __alloc_skb(size as u32, gfp, 0, -1) };
    if copy.is_null() {
        return core::ptr::null_mut();
    }
    let head = unsafe { read_field::<*mut u8>(copy, SKB_HEAD_OFFSET) };
    if head.is_null() {
        unsafe { free_raw_skb(copy) };
        return core::ptr::null_mut();
    }
    let data = unsafe { head.add(newheadroom as usize) };
    unsafe {
        write_field(copy, SKB_DATA_OFFSET, data);
        write_field(copy, SKB_TAIL_OFFSET, (newheadroom as usize + len) as u32);
        write_field(copy, SKB_LEN_OFFSET, len as u32);
        write_field(copy, SKB_DATA_LEN_OFFSET, 0u32);
        write_field(
            copy,
            SKB_DEV_OFFSET,
            read_field::<*mut u8>(skb, SKB_DEV_OFFSET),
        );
        write_field(
            copy,
            SKB_MAC_LEN_OFFSET,
            read_field::<u16>(skb, SKB_MAC_LEN_OFFSET),
        );
        write_field(
            copy,
            SKB_FLAGS_OFFSET,
            read_field::<u8>(skb, SKB_FLAGS_OFFSET),
        );
        write_field(
            copy,
            SKB_PKT_TYPE_OFFSET,
            read_field::<u8>(skb, SKB_PKT_TYPE_OFFSET),
        );
        write_field(
            copy,
            SKB_PROTOCOL_OFFSET,
            read_field::<u16>(skb, SKB_PROTOCOL_OFFSET),
        );
        write_field(
            copy,
            SKB_TRANSPORT_HEADER_OFFSET,
            read_field::<u16>(skb, SKB_TRANSPORT_HEADER_OFFSET),
        );
        write_field(
            copy,
            SKB_NETWORK_HEADER_OFFSET,
            read_field::<u16>(skb, SKB_NETWORK_HEADER_OFFSET),
        );
        write_field(
            copy,
            SKB_MAC_HEADER_OFFSET,
            read_field::<u16>(skb, SKB_MAC_HEADER_OFFSET),
        );
    }
    if unsafe { skb_copy_bits(skb, 0, data, len as i32) } != 0 {
        unsafe { free_raw_skb(copy) };
        return core::ptr::null_mut();
    }
    copy
}

#[unsafe(no_mangle)]
unsafe extern "C" fn skb_copy(skb: *mut u8, gfp: crate::mm::page_flags::GfpFlags) -> *mut u8 {
    unsafe { skb_copy_expand(skb, 0, 0, gfp) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn skb_checksum_help(skb: *mut u8) -> i32 {
    if skb.is_null() {
        return -crate::include::uapi::errno::EINVAL;
    }
    let flags = unsafe { read_field::<u8>(skb, SKB_PKT_TYPE_OFFSET) };
    unsafe { write_field(skb, SKB_PKT_TYPE_OFFSET, flags & !SKB_IP_SUMMED_MASK) };
    0
}

/// `__skb_gso_segment()` — `vendor/linux/net/core/gso.c:88`.
#[unsafe(no_mangle)]
unsafe extern "C" fn __skb_gso_segment(_skb: *mut u8, _features: u64, _tx_path: bool) -> *mut u8 {
    linux_net_err_ptr(crate::include::uapi::errno::EOPNOTSUPP)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn skb_coalesce_rx_frag(skb: *mut u8, index: i32, size: i32, truesize: u32) {
    if skb.is_null() || !(0..17).contains(&index) || size < 0 {
        return;
    }
    let head = unsafe { read_field::<*mut u8>(skb, SKB_HEAD_OFFSET) };
    let end = unsafe { read_field::<u32>(skb, SKB_END_OFFSET) } as usize;
    let frag = unsafe { head.add(end + 48 + index as usize * 16) };
    let old_len = unsafe { read_field::<u32>(frag, 8) };
    let old_true = unsafe { read_field::<u32>(skb, SKB_TRUESIZE_OFFSET) };
    unsafe {
        write_field(frag, 8, old_len.saturating_add(size as u32));
        write_field(skb, SKB_TRUESIZE_OFFSET, old_true.saturating_add(truesize));
    }
}

unsafe fn set_sg_buffer(
    sg: *mut crate::lib::scatterlist::LinuxScatterList,
    address: *const u8,
    length: u32,
) -> bool {
    if sg.is_null() || address.is_null() || length == 0 {
        return false;
    }
    let Some(phys) = crate::arch::x86::mm::paging::virt_to_phys(address as u64) else {
        return false;
    };
    let page = crate::mm::buddy::pfn_to_page((phys as usize) >> 12);
    let flags = unsafe { (*sg).page_link & crate::lib::scatterlist::SG_PAGE_LINK_MASK };
    unsafe {
        (*sg).page_link = (page as usize) | flags;
        (*sg).offset = (phys as usize & 0xfff) as u32;
        (*sg).length = length;
        (*sg).dma_address = 0;
        (*sg).dma_length = 0;
        (*sg).dma_flags = 0;
    }
    true
}

/// `skb_to_sgvec()` — maps the skb's linear area followed by its page frags
/// into the caller's pre-initialized scatterlist.
#[unsafe(no_mangle)]
unsafe extern "C" fn skb_to_sgvec(
    skb: *mut u8,
    sg: *mut crate::lib::scatterlist::LinuxScatterList,
    offset: i32,
    length: i32,
) -> i32 {
    if skb.is_null() || sg.is_null() || offset < 0 || length < 0 {
        return -crate::include::uapi::errno::EINVAL;
    }
    if length == 0 {
        return 0;
    }
    let total_len = unsafe { read_field::<u32>(skb, SKB_LEN_OFFSET) } as usize;
    let data_len = unsafe { read_field::<u32>(skb, SKB_DATA_LEN_OFFSET) } as usize;
    let mut skip = offset as usize;
    let mut remaining = length as usize;
    if skip > total_len || remaining > total_len - skip || data_len > total_len {
        return -crate::include::uapi::errno::EINVAL;
    }
    let mut entries = 0usize;
    let linear_len = total_len - data_len;
    if skip < linear_len && remaining != 0 {
        let count = remaining.min(linear_len - skip);
        let data = unsafe { read_field::<*mut u8>(skb, SKB_DATA_OFFSET) };
        if !unsafe { set_sg_buffer(sg, data.add(skip), count as u32) } {
            return -crate::include::uapi::errno::EINVAL;
        }
        entries += 1;
        remaining -= count;
        skip = 0;
    } else {
        skip = skip.saturating_sub(linear_len);
    }
    if remaining != 0 {
        let head = unsafe { read_field::<*mut u8>(skb, SKB_HEAD_OFFSET) };
        let end = unsafe { read_field::<u32>(skb, SKB_END_OFFSET) } as usize;
        let shinfo = unsafe { head.add(end) };
        let nr_frags = unsafe { read_field::<u8>(shinfo, 2) } as usize;
        for index in 0..nr_frags.min(17) {
            let frag = unsafe { shinfo.add(48 + index * 16) };
            let frag_len = unsafe { read_field::<u32>(frag, 8) } as usize;
            if skip >= frag_len {
                skip -= frag_len;
                continue;
            }
            let count = remaining.min(frag_len - skip);
            let page = (unsafe { read_field::<usize>(frag, 0) } & !3) as *mut crate::mm::page::Page;
            let page_offset = unsafe { read_field::<u32>(frag, 12) } as usize + skip;
            if page.is_null() || !crate::mm::buddy::page_in_mem_map(page) {
                return -crate::include::uapi::errno::EINVAL;
            }
            let address = unsafe {
                crate::arch::x86::mm::paging::pfn_to_virt(crate::mm::buddy::page_to_pfn(page))
                    .add(page_offset)
            };
            if !unsafe { set_sg_buffer(sg.add(entries), address, count as u32) } {
                return -crate::include::uapi::errno::EINVAL;
            }
            entries += 1;
            remaining -= count;
            skip = 0;
            if remaining == 0 {
                break;
            }
        }
    }
    if remaining == 0 {
        entries as i32
    } else {
        -crate::include::uapi::errno::EINVAL
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn dev_close(_dev: *mut u8) -> i32 {
    0
}

#[unsafe(no_mangle)]
unsafe extern "C" fn netdev_update_features(_dev: *mut u8) {}

#[unsafe(no_mangle)]
unsafe extern "C" fn netdev_sw_irq_coalesce_default_on(_dev: *mut u8) {}

#[unsafe(no_mangle)]
unsafe extern "C" fn dev_trans_start(_dev: *mut u8) -> usize {
    0
}

#[unsafe(no_mangle)]
unsafe extern "C" fn netdev_stats_to_stats64(stats64: *mut u8, _netdev_stats: *const u8) {
    if !stats64.is_null() {
        unsafe { core::ptr::write_bytes(stats64, 0, 200) };
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn dev_fetch_sw_netstats(_stats64: *mut u8, _netstats: *const u8) {}

#[unsafe(no_mangle)]
unsafe extern "C" fn dev_get_stats(_dev: *mut u8, storage: *mut u8) -> *mut u8 {
    if !storage.is_null() {
        unsafe { core::ptr::write_bytes(storage, 0, 200) };
    }
    storage
}

#[unsafe(no_mangle)]
unsafe extern "C" fn ethtool_puts(data: *mut *mut u8, string: *const u8) {
    if data.is_null() || string.is_null() {
        return;
    }
    let mut out = unsafe { *data };
    if out.is_null() {
        return;
    }
    let mut src = string;
    loop {
        let byte = unsafe { *src };
        unsafe { *out = byte };
        out = unsafe { out.add(1) };
        if byte == 0 {
            break;
        }
        src = unsafe { src.add(1) };
    }
    unsafe { *data = out };
}

#[unsafe(no_mangle)]
unsafe extern "C" fn ethtool_set_ethtool_phy_ops(ops: *const u8) {
    unsafe { LINUX_ETHTOOL_PHY_OPS = ops as usize };
}

#[unsafe(no_mangle)]
unsafe extern "C" fn ethtool_str_to_medium(str: *const u8) -> i32 {
    if str.is_null() {
        return LINUX_ETHTOOL_LINK_MEDIUM_NONE;
    }
    for (index, name) in [
        b"BaseT".as_slice(),
        b"BaseK".as_slice(),
        b"BaseS".as_slice(),
        b"BaseC".as_slice(),
        b"BaseL".as_slice(),
        b"BaseD".as_slice(),
        b"BaseE".as_slice(),
        b"BaseF".as_slice(),
        b"BaseV".as_slice(),
        b"BaseMLD".as_slice(),
        b"None".as_slice(),
    ]
    .iter()
    .enumerate()
    {
        if unsafe { c_str_eq_bytes(str, name) } {
            return index as i32;
        }
    }
    LINUX_ETHTOOL_LINK_MEDIUM_NONE
}

unsafe fn c_str_eq_bytes(mut c_str: *const u8, bytes: &[u8]) -> bool {
    for expected in bytes {
        if unsafe { *c_str } != *expected {
            return false;
        }
        c_str = unsafe { c_str.add(1) };
    }
    unsafe { *c_str == 0 }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn ethnl_cable_test_alloc(_phydev: *mut u8, _cmd: u8) -> i32 {
    -LINUX_EOPNOTSUPP
}

#[unsafe(no_mangle)]
unsafe extern "C" fn ethnl_cable_test_free(_phydev: *mut u8) {}

#[unsafe(no_mangle)]
unsafe extern "C" fn ethnl_cable_test_finished(_phydev: *mut u8) {}

#[unsafe(no_mangle)]
unsafe extern "C" fn ethnl_cable_test_result_with_src(
    _phydev: *mut u8,
    _pair: u8,
    _result: u8,
    _src: u32,
) -> i32 {
    0
}

#[unsafe(no_mangle)]
unsafe extern "C" fn ethnl_cable_test_fault_length_with_src(
    _phydev: *mut u8,
    _pair: u8,
    _cm: u32,
    _src: u32,
) -> i32 {
    0
}

#[unsafe(no_mangle)]
unsafe extern "C" fn ptp_clock_register(_info: *mut u8, _parent: *mut u8) -> *mut u8 {
    core::ptr::null_mut()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn ptp_clock_unregister(_clock: *mut u8) {}

#[unsafe(no_mangle)]
unsafe extern "C" fn ptp_clock_index(_clock: *mut u8) -> i32 {
    -1
}

#[unsafe(no_mangle)]
unsafe extern "C" fn ptp_schedule_worker(_clock: *mut u8, _delay: usize) -> i32 {
    0
}

/// Submit one complete Ethernet frame through a registered vendor driver's
/// `ndo_start_xmit` entry point.
pub fn transmit_linux_ethernet_frame(linux_dev: *mut u8, frame: &[u8]) -> Result<(), i32> {
    if linux_dev.is_null() || frame.len() < 14 || frame.len() > u32::MAX as usize {
        return Err(crate::include::uapi::errno::EINVAL);
    }
    let skb = unsafe {
        __alloc_skb(
            (frame.len() + 64) as u32,
            crate::mm::page_flags::GFP_ATOMIC,
            0,
            -1,
        )
    };
    if skb.is_null() {
        return Err(crate::include::uapi::errno::ENOMEM);
    }
    let head = unsafe { read_field::<*mut u8>(skb, SKB_HEAD_OFFSET) };
    unsafe {
        write_field(skb, SKB_DATA_OFFSET, head.add(64));
        write_field(skb, SKB_TAIL_OFFSET, 64u32);
        write_field(skb, SKB_DEV_OFFSET, linux_dev);
        write_field(skb, SKB_MAC_HEADER_OFFSET, 64u16);
        write_field(skb, SKB_NETWORK_HEADER_OFFSET, 78u16);
        write_field(
            skb,
            SKB_PROTOCOL_OFFSET,
            u16::from_ne_bytes([frame[12], frame[13]]),
        );
    }
    let payload = unsafe { linux_skb_put(skb, frame.len() as u32) };
    if payload.is_null() {
        unsafe { free_raw_skb(skb) };
        return Err(crate::include::uapi::errno::ENOMEM);
    }
    unsafe { core::ptr::copy_nonoverlapping(frame.as_ptr(), payload, frame.len()) };
    let ops = unsafe { read_field::<*const u8>(linux_dev, 8) };
    if ops.is_null() {
        unsafe { free_raw_skb(skb) };
        return Err(crate::include::uapi::errno::ENODEV);
    }
    let function = unsafe { ops.add(32).cast::<usize>().read_unaligned() };
    if function == 0 {
        unsafe { free_raw_skb(skb) };
        return Err(crate::include::uapi::errno::EOPNOTSUPP);
    }
    let start_xmit: unsafe extern "C" fn(*mut u8, *mut u8) -> u32 =
        unsafe { core::mem::transmute(function) };
    let result = unsafe { start_xmit(skb, linux_dev) };
    #[cfg(not(test))]
    if crate::kernel::debug_trace::ping_enabled() {
        crate::linux_driver_abi::tty::serial_println!(
            "net-tx: len={} ethertype={:04x} xmit={}",
            frame.len(),
            u16::from_be_bytes([frame[12], frame[13]]),
            result
        );
    }
    if result != 0 {
        unsafe { free_raw_skb(skb) };
        return Err(crate::include::uapi::errno::EAGAIN);
    }
    if let Some(dev) = crate::net::device::lookup_linux_netdevice(linux_dev) {
        dev.tx_packets
            .fetch_add(1, core::sync::atomic::Ordering::AcqRel);
    }
    Ok(())
}

/// `synchronize_net()` — `vendor/linux/net/core/dev.c`; its ordering contract
/// is an RCU grace period for packet-path readers.
#[unsafe(no_mangle)]
extern "C" fn synchronize_net() {
    crate::kernel::rcu::synchronize_rcu();
}

/// Core netfilter owns the optional conntrack hook. Lupos does not install a
/// conntrack hook yet, so this preserves Linux's no-hook warning branch. Raw
/// skbs created by the core keep `_nfct == 0` and do not reach this function.
#[unsafe(no_mangle)]
unsafe extern "C" fn nf_conntrack_destroy(_nfct: *mut core::ffi::c_void) {
    crate::kernel::rcu::rcu_read_lock();
    crate::kernel::rcu::rcu_read_unlock();
    crate::kernel::printk::log_warn!(
        "netfilter",
        "nf_conntrack_destroy called without a registered conntrack hook"
    );
}

const NET_DEVICE_TX_PTR_OFFSET: usize = 24;
const NET_DEVICE_REAL_NUM_TX_QUEUES_OFFSET: usize = 40;
const NET_DEVICE_GSO_MAX_SIZE_OFFSET: usize = 44;
const NET_DEVICE_GSO_IPV4_MAX_SIZE_OFFSET: usize = 48;
const NET_DEVICE_GSO_MAX_SEGS_OFFSET: usize = 52;
const NET_DEVICE_NUM_TC_OFFSET: usize = 54;
const NET_DEVICE_MTU_OFFSET: usize = 56;
const NET_DEVICE_XPS_MAPS_OFFSET: usize = 128;
const NET_DEVICE_STATE_OFFSET: usize = 168;
const NET_DEVICE_FLAGS_OFFSET: usize = 176;
const NET_DEVICE_IFINDEX_OFFSET: usize = 224;
const NET_DEVICE_REAL_NUM_RX_QUEUES_OFFSET: usize = 228;
const NET_DEVICE_RX_PTR_OFFSET: usize = 232;
const NET_DEVICE_NAME_OFFSET: usize = 288;
const NET_DEVICE_NAME_LEN: usize = 16;
const NET_DEVICE_ADDR_LEN_OFFSET: usize = 808;
const NET_DEVICE_PRIV_LEN_OFFSET: usize = 824;
const NET_DEVICE_DEV_ADDR_OFFSET: usize = 1088;
const NET_DEVICE_NUM_RX_QUEUES_OFFSET: usize = 1096;
const NET_DEVICE_NUM_TX_QUEUES_OFFSET: usize = 1176;
const NET_DEVICE_TX_GLOBAL_LOCK_OFFSET: usize = 1196;
const NET_DEVICE_REG_STATE_OFFSET: usize = 1432;
const NET_DEVICE_TSO_MAX_SIZE_OFFSET: usize = 2296;
const NET_DEVICE_TSO_MAX_SEGS_OFFSET: usize = 2300;
const NET_DEVICE_DEV_ADDR_SHADOW_OFFSET: usize = 2472;
const NET_DEVICE_NAPI_CONFIG_OFFSET: usize = 2544;
const NET_DEVICE_NUM_NAPI_CONFIGS_OFFSET: usize = 2552;
const NET_DEVICE_SIZE: usize = 2624;
const NETDEV_QUEUE_SIZE: usize = 320;
const NETDEV_RX_QUEUE_SIZE: usize = 256;
const NAPI_CONFIG_SIZE: usize = 40;
const GSO_MAX_SIZE: u32 = 65_536;
const IFF_LIVE_ADDR_CHANGE: u64 = 1 << 15;
const LINK_STATE_START: u64 = 1 << 0;
const NETDEV_QUEUE_XMIT_LOCK_OFFSET: usize = 256;
const NETDEV_QUEUE_XMIT_OWNER_OFFSET: usize = 260;
const NETDEV_QUEUE_STATE_OFFSET: usize = 272;
const NETDEV_QUEUE_NUMA_NODE_OFFSET: usize = 288;
const QUEUE_STATE_DRV_XOFF: u64 = 1;
const QUEUE_STATE_FROZEN: u64 = 1 << 2;

unsafe fn read_field<T: Copy>(base: *const u8, offset: usize) -> T {
    unsafe { base.add(offset).cast::<T>().read_unaligned() }
}

unsafe fn write_field<T>(base: *mut u8, offset: usize, value: T) {
    unsafe { base.add(offset).cast::<T>().write_unaligned(value) };
}

unsafe fn init_list_head(base: *mut u8, offset: usize) {
    let head = unsafe { base.add(offset) };
    unsafe {
        write_field(head, 0, head as usize);
        write_field(head, core::mem::size_of::<usize>(), head as usize);
    }
}

unsafe fn raw_netdev_name(dev: *const u8) -> String {
    let bytes = unsafe {
        core::slice::from_raw_parts(dev.add(NET_DEVICE_NAME_OFFSET), NET_DEVICE_NAME_LEN)
    };
    let len = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..len]).into_owned()
}

unsafe fn set_raw_netdev_name(dev: *mut u8, name: &str) {
    let destination = unsafe { dev.add(NET_DEVICE_NAME_OFFSET) };
    unsafe { core::ptr::write_bytes(destination, 0, NET_DEVICE_NAME_LEN) };
    let bytes = name.as_bytes();
    let len = bytes.len().min(NET_DEVICE_NAME_LEN - 1);
    unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), destination, len) };
}

unsafe fn initialize_netdev_lists(dev: *mut u8) {
    for offset in [
        208usize, 344, 360, 376, 392, 408, 424, 440, 832, 864, 896, 952, 984, 1392, 1416, 2368,
    ] {
        unsafe { init_list_head(dev, offset) };
    }
}

unsafe fn free_allocated_netdev_parts(dev: *mut u8) {
    if dev.is_null() {
        return;
    }
    for offset in [
        NET_DEVICE_NAPI_CONFIG_OFFSET,
        NET_DEVICE_RX_PTR_OFFSET,
        NET_DEVICE_TX_PTR_OFFSET,
    ] {
        let ptr = unsafe { read_field::<*mut u8>(dev, offset) };
        if !ptr.is_null() {
            unsafe { crate::mm::slab::kfree(ptr) };
            unsafe { write_field(dev, offset, core::ptr::null_mut::<u8>()) };
        }
    }
}

/// `alloc_etherdev_mqs()` — `vendor/linux/net/ethernet/eth.c:376`, backed by
/// the configured `alloc_netdev_mqs()` layout from `net/core/dev.c:12025`.
#[unsafe(no_mangle)]
unsafe extern "C" fn alloc_etherdev_mqs(sizeof_priv: i32, txqs: u32, rxqs: u32) -> *mut u8 {
    if sizeof_priv < 0 || txqs == 0 || rxqs == 0 || txqs > u16::MAX as u32 {
        return core::ptr::null_mut();
    }
    let Some(total) = NET_DEVICE_SIZE.checked_add(sizeof_priv as usize) else {
        return core::ptr::null_mut();
    };
    let dev = unsafe { crate::mm::slab::kzalloc_noprof(total, crate::mm::page_flags::GFP_KERNEL) };
    if dev.is_null() || (dev as usize) & 63 != 0 {
        if !dev.is_null() {
            unsafe { crate::mm::slab::kfree(dev) };
        }
        return core::ptr::null_mut();
    }

    let tx_size = match (txqs as usize).checked_mul(NETDEV_QUEUE_SIZE) {
        Some(size) => size,
        None => {
            unsafe { crate::mm::slab::kfree(dev) };
            return core::ptr::null_mut();
        }
    };
    let rx_size = match (rxqs as usize).checked_mul(NETDEV_RX_QUEUE_SIZE) {
        Some(size) => size,
        None => {
            unsafe { crate::mm::slab::kfree(dev) };
            return core::ptr::null_mut();
        }
    };
    let maxqs = txqs.max(rxqs) as usize;
    let napi_size = match maxqs.checked_mul(NAPI_CONFIG_SIZE) {
        Some(size) => size,
        None => {
            unsafe { crate::mm::slab::kfree(dev) };
            return core::ptr::null_mut();
        }
    };
    let tx = unsafe { crate::mm::slab::kzalloc_noprof(tx_size, crate::mm::page_flags::GFP_KERNEL) };
    let rx = unsafe { crate::mm::slab::kzalloc_noprof(rx_size, crate::mm::page_flags::GFP_KERNEL) };
    let napi_config =
        unsafe { crate::mm::slab::kzalloc_noprof(napi_size, crate::mm::page_flags::GFP_KERNEL) };
    if tx.is_null() || rx.is_null() || napi_config.is_null() {
        for ptr in [tx, rx, napi_config] {
            if !ptr.is_null() {
                unsafe { crate::mm::slab::kfree(ptr) };
            }
        }
        unsafe { crate::mm::slab::kfree(dev) };
        return core::ptr::null_mut();
    }

    unsafe {
        write_field(dev, NET_DEVICE_TX_PTR_OFFSET, tx);
        write_field(dev, NET_DEVICE_REAL_NUM_TX_QUEUES_OFFSET, txqs);
        write_field(dev, NET_DEVICE_REAL_NUM_RX_QUEUES_OFFSET, rxqs);
        write_field(dev, NET_DEVICE_RX_PTR_OFFSET, rx);
        write_field(dev, NET_DEVICE_NUM_RX_QUEUES_OFFSET, rxqs);
        write_field(dev, NET_DEVICE_NUM_TX_QUEUES_OFFSET, txqs);
        write_field(dev, NET_DEVICE_PRIV_LEN_OFFSET, sizeof_priv as u32);
        write_field(dev, NET_DEVICE_NAPI_CONFIG_OFFSET, napi_config);
        write_field(dev, NET_DEVICE_NUM_NAPI_CONFIGS_OFFSET, maxqs as u32);

        // `alloc_netdev_mqs()` defaults.
        write_field(dev, 0, (1u64 << 5) | (1u64 << 17));
        write_field(dev, NET_DEVICE_GSO_MAX_SIZE_OFFSET, GSO_MAX_SIZE);
        write_field(dev, NET_DEVICE_GSO_IPV4_MAX_SIZE_OFFSET, GSO_MAX_SIZE);
        write_field(dev, NET_DEVICE_GSO_MAX_SEGS_OFFSET, u16::MAX);
        write_field(dev, 240, 65_536u32);
        write_field(dev, 244, 65_536u32);
        write_field(dev, 1100, 1u32);
        write_field(dev, NET_DEVICE_TSO_MAX_SIZE_OFFSET, GSO_MAX_SIZE);
        write_field(dev, NET_DEVICE_TSO_MAX_SEGS_OFFSET, u16::MAX);
        write_field(dev, 809, 1u8);
        write_field(dev, 810, 1u8);
        initialize_netdev_lists(dev);

        // `ether_setup()` defaults.
        let priv_flags = read_field::<u64>(dev, 0) | (1u64 << 11);
        write_field(dev, 0, priv_flags);
        write_field(dev, 544, 1u16);
        write_field(dev, 180, 14u16);
        write_field(dev, 546, 14u8);
        write_field(dev, NET_DEVICE_MTU_OFFSET, 1500u32);
        write_field(dev, 536, 68u32);
        write_field(dev, 540, 1500u32);
        write_field(dev, NET_DEVICE_ADDR_LEN_OFFSET, 6u8);
        write_field(dev, 1192, 1000u32);
        write_field(dev, NET_DEVICE_FLAGS_OFFSET, (1u32 << 1) | (1u32 << 12));
        write_field(dev, 547, 1u8);
        set_raw_netdev_name(dev, "eth%d");
        let address = dev.add(NET_DEVICE_DEV_ADDR_SHADOW_OFFSET);
        write_field(dev, NET_DEVICE_DEV_ADDR_OFFSET, address);
        core::ptr::write_bytes(dev.add(1120), 0xff, 6);

        for index in 0..txqs as usize {
            let queue = tx.add(index * NETDEV_QUEUE_SIZE);
            write_field(queue, 0, dev);
            write_field(queue, NETDEV_QUEUE_XMIT_OWNER_OFFSET, u32::MAX);
            write_field(queue, NETDEV_QUEUE_NUMA_NODE_OFFSET, -1i32);
            crate::lib::dynamic_queue_limits::dql_init(
                queue
                    .add(128)
                    .cast::<crate::lib::dynamic_queue_limits::LinuxDql>(),
                250,
            );
        }
        for index in 0..rxqs as usize {
            let queue = rx.add(index * NETDEV_RX_QUEUE_SIZE);
            // xdp_rxq_info fields and netdev_rx_queue::dev.
            write_field(queue, 0, dev);
            write_field(queue, 8, index as u32);
            write_field(queue, 12, 1u32);
            write_field(queue, 152, dev);
        }
    }
    dev
}

/// `devm_alloc_etherdev_mqs()` — `vendor/linux/net/devres.c:21`.
///
/// Lupos does not hot-unbind Linux-built drivers yet, so the managed lifetime
/// collapses to the netdev allocation lifetime.
#[unsafe(no_mangle)]
unsafe extern "C" fn devm_alloc_etherdev_mqs(
    _dev: *mut u8,
    sizeof_priv: i32,
    txqs: u32,
    rxqs: u32,
) -> *mut u8 {
    unsafe { alloc_etherdev_mqs(sizeof_priv, txqs, rxqs) }
}

/// `free_netdev()` — `vendor/linux/net/core/dev.c:12186` for an unregistered
/// or fully unregistered module-backed device.
#[unsafe(no_mangle)]
unsafe extern "C" fn free_netdev(dev: *mut u8) {
    if dev.is_null() {
        return;
    }
    let reg_state = unsafe { read_field::<u8>(dev, NET_DEVICE_REG_STATE_OFFSET) };
    if reg_state == 1 || reg_state == 2 {
        return;
    }
    for map_type in 0..2usize {
        let slot = unsafe {
            dev.add(NET_DEVICE_XPS_MAPS_OFFSET + map_type * core::mem::size_of::<usize>())
                .cast::<*mut LinuxXpsDevMaps>()
        };
        let maps = unsafe { slot.read() };
        if !maps.is_null() {
            unsafe { free_xps_maps(maps) };
            unsafe { slot.write(core::ptr::null_mut()) };
        }
    }
    unsafe { free_allocated_netdev_parts(dev) };
    unsafe { crate::mm::slab::kfree(dev) };
}

fn next_linux_netdev_name(template: &str) -> Option<String> {
    if !template.contains("%d") {
        return crate::net::device::lookup_netdevice(template)
            .is_none()
            .then(|| String::from(template));
    }
    (0..10_000u32)
        .map(|index| template.replacen("%d", &format!("{index}"), 1))
        .find(|name| crate::net::device::lookup_netdevice(name).is_none())
}

/// `register_netdevice()` — configured object validation, name/ifindex
/// assignment, link state publication, and integration with the Lupos rtnetlink
/// registry.
#[unsafe(no_mangle)]
unsafe extern "C" fn register_netdevice(dev: *mut u8) -> i32 {
    if dev.is_null()
        || unsafe { read_field::<u8>(dev, NET_DEVICE_REG_STATE_OFFSET) } != 0
        || unsafe { read_field::<usize>(dev, 8) } == 0
    {
        return -crate::include::uapi::errno::EINVAL;
    }
    let template = unsafe { raw_netdev_name(dev) };
    let Some(name) = next_linux_netdev_name(&template) else {
        return -crate::include::uapi::errno::EBUSY;
    };
    unsafe { set_raw_netdev_name(dev, &name) };
    let mtu = unsafe { read_field::<u32>(dev, NET_DEVICE_MTU_OFFSET) };
    let addr_ptr = unsafe { read_field::<*const u8>(dev, NET_DEVICE_DEV_ADDR_OFFSET) };
    if addr_ptr.is_null() {
        return -crate::include::uapi::errno::EINVAL;
    }
    let mut address = [0u8; 6];
    unsafe { core::ptr::copy_nonoverlapping(addr_ptr, address.as_mut_ptr(), address.len()) };
    if address == [0; 6] {
        unsafe { get_random_bytes(address.as_mut_ptr(), address.len()) };
        address[0] = (address[0] & 0xfe) | 0x02;
        unsafe { core::ptr::copy_nonoverlapping(address.as_ptr(), addr_ptr.cast_mut(), 6) };
    }
    let registered =
        match crate::net::device::register_linux_netdevice_locked(&name, mtu, address, dev) {
            Ok(registered) => registered,
            Err(errno) => return -errno,
        };
    let flags = unsafe { read_field::<u32>(dev, NET_DEVICE_FLAGS_OFFSET) };
    registered
        .flags
        .store(flags, core::sync::atomic::Ordering::Release);
    let mut state = unsafe { read_field::<u64>(dev, NET_DEVICE_STATE_OFFSET) };
    state |= 1 << 1;
    unsafe {
        write_field(dev, NET_DEVICE_STATE_OFFSET, state);
        write_field(dev, NET_DEVICE_IFINDEX_OFFSET, registered.ifindex as i32);
        write_field(dev, NET_DEVICE_REG_STATE_OFFSET, 1u8);
    }
    crate::net::device::set_carrier(&registered, state & (1 << 2) == 0);
    0
}

#[unsafe(no_mangle)]
unsafe extern "C" fn register_netdev(dev: *mut u8) -> i32 {
    rtnl_lock();
    let result = unsafe { register_netdevice(dev) };
    rtnl_unlock();
    result
}

#[unsafe(no_mangle)]
extern "C" fn rtnl_lock() {
    crate::net::device::linux_rtnl_lock();
}

#[unsafe(no_mangle)]
extern "C" fn rtnl_unlock() {
    crate::net::device::linux_rtnl_unlock();
}

/// `rtnl_is_locked()` — `vendor/linux/net/core/rtnetlink.c:167`.
#[unsafe(no_mangle)]
extern "C" fn rtnl_is_locked() -> i32 {
    if crate::net::device::linux_rtnl_is_locked() {
        1
    } else {
        0
    }
}

/// `register_netdevice_notifier()` — `vendor/linux/net/core/dev.c:1968`.
///
/// NETCONSOLE only needs registration lifetime tracking when it is loaded with
/// no configured targets. Device-change replay can be added when Lupos exposes
/// a full Linux notifier chain for configured netpoll users.
#[unsafe(no_mangle)]
unsafe extern "C" fn register_netdevice_notifier(nb: *mut u8) -> i32 {
    if !nb.is_null() {
        let addr = nb as usize;
        let mut notifiers = NETDEVICE_NOTIFIERS.lock();
        if !notifiers.contains(&addr) {
            notifiers.push(addr);
        }
    }
    0
}

/// `unregister_netdevice_notifier()` — `vendor/linux/net/core/dev.c:2023`.
#[unsafe(no_mangle)]
unsafe extern "C" fn unregister_netdevice_notifier(nb: *mut u8) -> i32 {
    let addr = nb as usize;
    NETDEVICE_NOTIFIERS
        .lock()
        .retain(|registered| *registered != addr);
    0
}

/// `netpoll_setup()` — `vendor/linux/net/core/netpoll.c:547`.
///
/// Lupos loads the vendor netconsole module, but does not yet expose Linux
/// netpoll transmission. A configured target therefore fails closed instead of
/// pretending packets can be sent.
#[unsafe(no_mangle)]
unsafe extern "C" fn netpoll_setup(_np: *mut u8) -> i32 {
    -ENODEV
}

/// `netpoll_cleanup()` — `vendor/linux/net/core/netpoll.c:692`.
#[unsafe(no_mangle)]
unsafe extern "C" fn netpoll_cleanup(_np: *mut u8) {}

/// `do_netpoll_cleanup()` — `vendor/linux/net/core/netpoll.c:684`.
#[unsafe(no_mangle)]
unsafe extern "C" fn do_netpoll_cleanup(_np: *mut u8) {}

/// `netpoll_poll_dev()` — `vendor/linux/include/linux/netpoll.h`.
#[unsafe(no_mangle)]
unsafe extern "C" fn netpoll_poll_dev(_dev: *mut u8) {}

/// `netpoll_send_skb()` — `vendor/linux/net/core/netpoll.c:337`.
#[unsafe(no_mangle)]
unsafe extern "C" fn netpoll_send_skb(_np: *mut u8, skb: *mut u8) -> i32 {
    unsafe { free_raw_skb(skb) };
    NET_XMIT_DROP
}

/// `netpoll_zap_completion_queue()` — `vendor/linux/net/core/netpoll.c:232`.
#[unsafe(no_mangle)]
unsafe extern "C" fn netpoll_zap_completion_queue() {}

unsafe fn unregister_netdevice_locked(dev: *mut u8) {
    if dev.is_null() {
        return;
    }
    let state = unsafe { read_field::<u8>(dev, NET_DEVICE_REG_STATE_OFFSET) };
    if state != 1 {
        return;
    }
    let _ = crate::net::device::unregister_linux_netdevice_locked(dev);
    unsafe { write_field(dev, NET_DEVICE_REG_STATE_OFFSET, 3u8) };
}

#[unsafe(no_mangle)]
unsafe extern "C" fn unregister_netdev(dev: *mut u8) {
    rtnl_lock();
    unsafe { unregister_netdevice_locked(dev) };
    rtnl_unlock();
}

unsafe fn update_registered_carrier(dev: *mut u8, up: bool) {
    if let Some(registered) = crate::net::device::lookup_linux_netdevice(dev) {
        crate::net::device::set_carrier(&registered, up);
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn netif_carrier_on(dev: *mut u8) {
    if dev.is_null() {
        return;
    }
    let state = unsafe {
        &*dev
            .add(NET_DEVICE_STATE_OFFSET)
            .cast::<core::sync::atomic::AtomicU64>()
    };
    state.fetch_and(!(1 << 2), core::sync::atomic::Ordering::AcqRel);
    unsafe { update_registered_carrier(dev, true) };
}

#[unsafe(no_mangle)]
unsafe extern "C" fn netif_carrier_off(dev: *mut u8) {
    if dev.is_null() {
        return;
    }
    let state = unsafe {
        &*dev
            .add(NET_DEVICE_STATE_OFFSET)
            .cast::<core::sync::atomic::AtomicU64>()
    };
    state.fetch_or(1 << 2, core::sync::atomic::Ordering::AcqRel);
    unsafe { update_registered_carrier(dev, false) };
}

/// `linkwatch_fire_event()` — `vendor/linux/net/core/link_watch.c:319`.
#[unsafe(no_mangle)]
unsafe extern "C" fn linkwatch_fire_event(dev: *mut u8) {
    if dev.is_null() {
        return;
    }

    let state = unsafe {
        &*dev
            .add(NET_DEVICE_STATE_OFFSET)
            .cast::<core::sync::atomic::AtomicU64>()
    };
    let carrier_up = state.load(core::sync::atomic::Ordering::Acquire) & (1 << 2) == 0;
    unsafe { update_registered_carrier(dev, carrier_up) };
}

#[unsafe(no_mangle)]
unsafe extern "C" fn netif_device_attach(dev: *mut u8) {
    if !dev.is_null() {
        let state = unsafe {
            &*dev
                .add(NET_DEVICE_STATE_OFFSET)
                .cast::<core::sync::atomic::AtomicU64>()
        };
        state.fetch_or(1 << 1, core::sync::atomic::Ordering::AcqRel);
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn netif_device_detach(dev: *mut u8) {
    if !dev.is_null() {
        let state = unsafe {
            &*dev
                .add(NET_DEVICE_STATE_OFFSET)
                .cast::<core::sync::atomic::AtomicU64>()
        };
        state.fetch_and(!(1 << 1), core::sync::atomic::Ordering::AcqRel);
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn netif_set_real_num_tx_queues(dev: *mut u8, count: u32) -> i32 {
    if dev.is_null()
        || count == 0
        || count > unsafe { read_field::<u32>(dev, NET_DEVICE_NUM_TX_QUEUES_OFFSET) }
    {
        return -crate::include::uapi::errno::EINVAL;
    }
    unsafe { write_field(dev, NET_DEVICE_REAL_NUM_TX_QUEUES_OFFSET, count) };
    0
}

#[unsafe(no_mangle)]
unsafe extern "C" fn netif_set_real_num_rx_queues(dev: *mut u8, count: u32) -> i32 {
    if dev.is_null()
        || count == 0
        || count > unsafe { read_field::<u32>(dev, NET_DEVICE_NUM_RX_QUEUES_OFFSET) }
    {
        return -crate::include::uapi::errno::EINVAL;
    }
    unsafe { write_field(dev, NET_DEVICE_REAL_NUM_RX_QUEUES_OFFSET, count) };
    0
}

/// `netif_set_tso_max_size()` — `vendor/linux/net/core/dev.c:3317`.
#[unsafe(no_mangle)]
unsafe extern "C" fn netif_set_tso_max_size(dev: *mut u8, size: u32) {
    if dev.is_null() {
        return;
    }
    unsafe { write_field(dev, NET_DEVICE_TSO_MAX_SIZE_OFFSET, size.min(GSO_MAX_SIZE)) };
    if size < unsafe { read_field::<u32>(dev, NET_DEVICE_GSO_MAX_SIZE_OFFSET) } {
        unsafe { write_field(dev, NET_DEVICE_GSO_MAX_SIZE_OFFSET, size) };
    }
    if size < unsafe { read_field::<u32>(dev, NET_DEVICE_GSO_IPV4_MAX_SIZE_OFFSET) } {
        unsafe { write_field(dev, NET_DEVICE_GSO_IPV4_MAX_SIZE_OFFSET, size) };
    }
}

/// `netif_set_tso_max_segs()` — `vendor/linux/net/core/dev.c:3336`.
#[unsafe(no_mangle)]
unsafe extern "C" fn netif_set_tso_max_segs(dev: *mut u8, segs: u32) {
    if dev.is_null() {
        return;
    }
    let segs = segs.min(u16::MAX as u32) as u16;
    unsafe { write_field(dev, NET_DEVICE_TSO_MAX_SEGS_OFFSET, segs) };
    if segs < unsafe { read_field::<u16>(dev, NET_DEVICE_GSO_MAX_SEGS_OFFSET) } {
        unsafe { write_field(dev, NET_DEVICE_GSO_MAX_SEGS_OFFSET, segs) };
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn dev_addr_mod(dev: *mut u8, offset: u32, addr: *const u8, len: usize) {
    if dev.is_null() || addr.is_null() {
        return;
    }
    let address = unsafe { read_field::<*mut u8>(dev, NET_DEVICE_DEV_ADDR_OFFSET) };
    let addr_len = unsafe { read_field::<u8>(dev, NET_DEVICE_ADDR_LEN_OFFSET) } as usize;
    let offset = offset as usize;
    if address.is_null() || offset > addr_len || len > addr_len - offset {
        return;
    }
    unsafe { core::ptr::copy_nonoverlapping(addr, address.add(offset), len) };
}

fn is_valid_ether_addr(addr: &[u8; 6]) -> bool {
    addr[0] & 1 == 0 && *addr != [0; 6]
}

/// `eth_mac_addr` - `vendor/linux/net/ethernet/eth.c:307`.
#[unsafe(no_mangle)]
unsafe extern "C" fn eth_mac_addr(dev: *mut u8, sockaddr: *mut u8) -> i32 {
    if dev.is_null() || sockaddr.is_null() {
        return -crate::include::uapi::errno::EINVAL;
    }
    let priv_flags = unsafe { read_field::<u64>(dev, 0) };
    let state = unsafe { read_field::<u64>(dev, NET_DEVICE_STATE_OFFSET) };
    if priv_flags & IFF_LIVE_ADDR_CHANGE == 0 && state & LINK_STATE_START != 0 {
        return -crate::include::uapi::errno::EBUSY;
    }

    let mut addr = [0u8; 6];
    unsafe { core::ptr::copy_nonoverlapping(sockaddr.add(2), addr.as_mut_ptr(), addr.len()) };
    if !is_valid_ether_addr(&addr) {
        return -crate::include::uapi::errno::EADDRNOTAVAIL;
    }
    unsafe { dev_addr_mod(dev, 0, addr.as_ptr(), addr.len()) };
    0
}

static RANDOM_STATE: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(0x9e37_79b9_7f4a_7c15);

fn next_random_u64() -> u64 {
    let mut current = RANDOM_STATE.load(core::sync::atomic::Ordering::Acquire);
    loop {
        let mut candidate = current ^ crate::kernel::time::clocksource::read_tsc();
        candidate ^= candidate << 13;
        candidate ^= candidate >> 7;
        candidate ^= candidate << 17;
        match RANDOM_STATE.compare_exchange(
            current,
            candidate,
            core::sync::atomic::Ordering::AcqRel,
            core::sync::atomic::Ordering::Acquire,
        ) {
            Ok(_) => return candidate,
            Err(observed) => current = observed,
        }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn get_random_bytes(buffer: *mut u8, len: usize) {
    if buffer.is_null() {
        return;
    }
    let mut written = 0usize;
    while written < len {
        let next = next_random_u64();
        let bytes = next.to_ne_bytes();
        let count = (len - written).min(bytes.len());
        unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer.add(written), count) };
        written += count;
    }
}

/// `get_random_u8` - `vendor/linux/drivers/char/random.c`.
unsafe extern "C" fn get_random_u8() -> u8 {
    unsafe { get_random_u64() as u8 }
}

/// `get_random_u16` - `vendor/linux/drivers/char/random.c`.
unsafe extern "C" fn get_random_u16() -> u16 {
    unsafe { get_random_u64() as u16 }
}

/// `get_random_u32` - `vendor/linux/drivers/char/random.c`.
unsafe extern "C" fn get_random_u32() -> u32 {
    unsafe { get_random_u64() as u32 }
}

/// `get_random_u64` - `vendor/linux/drivers/char/random.c`.
unsafe extern "C" fn get_random_u64() -> u64 {
    next_random_u64()
}

/// `__get_random_u32_below` - `vendor/linux/drivers/char/random.c:559`.
unsafe extern "C" fn __get_random_u32_below(ceil: u32) -> u32 {
    let rand = unsafe { get_random_u32() };
    if ceil == 0 {
        return rand;
    }
    (((ceil as u64) * (rand as u64)) >> 32) as u32
}

unsafe fn tx_queues(dev: *mut u8) -> (*mut u8, usize) {
    if dev.is_null() {
        return (core::ptr::null_mut(), 0);
    }
    let queues = unsafe { read_field::<*mut u8>(dev, NET_DEVICE_TX_PTR_OFFSET) };
    let count = unsafe { read_field::<u32>(dev, NET_DEVICE_NUM_TX_QUEUES_OFFSET) } as usize;
    (queues, count)
}

unsafe fn txq_lock(queue: *mut u8) {
    let lock = unsafe {
        queue
            .add(NETDEV_QUEUE_XMIT_LOCK_OFFSET)
            .cast::<crate::kernel::locking::qspinlock::QSpinLock>()
    };
    unsafe { crate::kernel::locking::raw_spinlock::linux_raw_spin_lock(lock) };
    unsafe {
        write_field(
            queue,
            NETDEV_QUEUE_XMIT_OWNER_OFFSET,
            crate::kernel::sched::current_cpu(),
        )
    };
}

unsafe fn txq_unlock(queue: *mut u8) {
    unsafe { write_field(queue, NETDEV_QUEUE_XMIT_OWNER_OFFSET, u32::MAX) };
    let lock = unsafe {
        queue
            .add(NETDEV_QUEUE_XMIT_LOCK_OFFSET)
            .cast::<crate::kernel::locking::qspinlock::QSpinLock>()
    };
    unsafe { crate::kernel::locking::raw_spinlock::linux_raw_spin_unlock(lock) };
}

/// `netif_tx_lock()` — `vendor/linux/net/sched/sch_generic.c:495`.
#[unsafe(no_mangle)]
unsafe extern "C" fn netif_tx_lock(dev: *mut u8) {
    if dev.is_null() {
        return;
    }
    let global = unsafe {
        dev.add(NET_DEVICE_TX_GLOBAL_LOCK_OFFSET)
            .cast::<crate::kernel::locking::qspinlock::QSpinLock>()
    };
    unsafe { crate::kernel::locking::raw_spinlock::linux_raw_spin_lock(global) };
    let (queues, count) = unsafe { tx_queues(dev) };
    if queues.is_null() {
        unsafe { crate::kernel::locking::raw_spinlock::linux_raw_spin_unlock(global) };
        return;
    }
    for index in 0..count {
        let queue = unsafe { queues.add(index * NETDEV_QUEUE_SIZE) };
        unsafe { txq_lock(queue) };
        let state = unsafe {
            &*queue
                .add(NETDEV_QUEUE_STATE_OFFSET)
                .cast::<core::sync::atomic::AtomicU64>()
        };
        state.fetch_or(QUEUE_STATE_FROZEN, core::sync::atomic::Ordering::AcqRel);
        unsafe { txq_unlock(queue) };
    }
}

/// `netif_schedule_queue()` — `vendor/linux/net/core/dev.c:3418`.
/// Lupos currently installs Linux's noqueue discipline, so no qdisc run list
/// exists after the stopped-state check.
#[unsafe(no_mangle)]
unsafe extern "C" fn netif_schedule_queue(queue: *mut u8) {
    if queue.is_null() {
        return;
    }
    crate::kernel::rcu::rcu_read_lock();
    let state = unsafe {
        &*queue
            .add(NETDEV_QUEUE_STATE_OFFSET)
            .cast::<core::sync::atomic::AtomicU64>()
    };
    let _running = state.load(core::sync::atomic::Ordering::Acquire) & 0x3 == 0;
    crate::kernel::rcu::rcu_read_unlock();
}

/// `netif_tx_unlock()` — `vendor/linux/net/sched/sch_generic.c:518`.
#[unsafe(no_mangle)]
unsafe extern "C" fn netif_tx_unlock(dev: *mut u8) {
    if dev.is_null() {
        return;
    }
    let (queues, count) = unsafe { tx_queues(dev) };
    if !queues.is_null() {
        for index in 0..count {
            let queue = unsafe { queues.add(index * NETDEV_QUEUE_SIZE) };
            let state = unsafe {
                &*queue
                    .add(NETDEV_QUEUE_STATE_OFFSET)
                    .cast::<core::sync::atomic::AtomicU64>()
            };
            state.fetch_and(!QUEUE_STATE_FROZEN, core::sync::atomic::Ordering::AcqRel);
            unsafe { netif_schedule_queue(queue) };
        }
    }
    let global = unsafe {
        dev.add(NET_DEVICE_TX_GLOBAL_LOCK_OFFSET)
            .cast::<crate::kernel::locking::qspinlock::QSpinLock>()
    };
    unsafe { crate::kernel::locking::raw_spinlock::linux_raw_spin_unlock(global) };
}

/// `netif_tx_wake_queue()` — `vendor/linux/net/core/dev.c:3430`.
#[unsafe(no_mangle)]
unsafe extern "C" fn netif_tx_wake_queue(queue: *mut u8) {
    if queue.is_null() {
        return;
    }
    let state = unsafe {
        &*queue
            .add(NETDEV_QUEUE_STATE_OFFSET)
            .cast::<core::sync::atomic::AtomicU64>()
    };
    if state.fetch_and(!QUEUE_STATE_DRV_XOFF, core::sync::atomic::Ordering::AcqRel)
        & QUEUE_STATE_DRV_XOFF
        != 0
    {
        unsafe { netif_schedule_queue(queue) };
    }
}

/// `netif_get_num_default_rss_queues()` — `vendor/linux/net/core/dev.c:3362`.
#[unsafe(no_mangle)]
unsafe extern "C" fn netif_get_num_default_rss_queues() -> i32 {
    let count = crate::kernel::cpuhotplug::cpu_online_mask()
        .count_ones()
        .max(1);
    if count > 2 {
        count.div_ceil(2) as i32
    } else {
        count as i32
    }
}

/// `netif_tx_stop_all_queues()` — `vendor/linux/net/core/dev.c:11241`.
#[unsafe(no_mangle)]
unsafe extern "C" fn netif_tx_stop_all_queues(dev: *mut u8) {
    let (queues, count) = unsafe { tx_queues(dev) };
    if queues.is_null() {
        return;
    }
    for index in 0..count {
        let queue = unsafe { queues.add(index * NETDEV_QUEUE_SIZE) };
        let state = unsafe {
            &*queue
                .add(NETDEV_QUEUE_STATE_OFFSET)
                .cast::<core::sync::atomic::AtomicU64>()
        };
        state.fetch_or(QUEUE_STATE_DRV_XOFF, core::sync::atomic::Ordering::AcqRel);
    }
}

#[repr(C)]
struct LinuxXpsDevMaps {
    rcu: [usize; 2],
    nr_ids: u32,
    num_tc: i16,
    _pad: u16,
}

#[repr(C)]
struct LinuxXpsMap {
    len: u32,
    alloc_len: u32,
    rcu: [usize; 2],
}

static XPS_MAP_LOCK: spin::Mutex<()> = spin::Mutex::new(());

unsafe fn free_xps_maps(maps: *mut LinuxXpsDevMaps) {
    if maps.is_null() {
        return;
    }
    let entries = unsafe { (*maps).nr_ids as usize * (*maps).num_tc.max(0) as usize };
    let attrs = unsafe {
        (maps as *mut u8)
            .add(core::mem::size_of::<LinuxXpsDevMaps>())
            .cast::<*mut LinuxXpsMap>()
    };
    for index in 0..entries {
        let map = unsafe { attrs.add(index).read() };
        if !map.is_null() {
            unsafe { crate::mm::slab::kfree(map.cast()) };
        }
    }
    unsafe { crate::mm::slab::kfree(maps.cast()) };
}

unsafe fn old_xps_queues(maps: *mut LinuxXpsDevMaps, id: usize, tc: usize) -> Vec<u16> {
    if maps.is_null()
        || id >= unsafe { (*maps).nr_ids as usize }
        || tc >= unsafe { (*maps).num_tc.max(0) as usize }
    {
        return Vec::new();
    }
    let attrs = unsafe {
        (maps as *mut u8)
            .add(core::mem::size_of::<LinuxXpsDevMaps>())
            .cast::<*mut LinuxXpsMap>()
    };
    let map = unsafe { attrs.add(id * (*maps).num_tc as usize + tc).read() };
    if map.is_null() {
        return Vec::new();
    }
    let len = unsafe { (*map).len as usize };
    let queues = unsafe {
        (map as *const u8)
            .add(core::mem::size_of::<LinuxXpsMap>())
            .cast::<u16>()
    };
    unsafe { core::slice::from_raw_parts(queues, len) }.to_vec()
}

unsafe fn allocate_xps_map(queues: &[u16]) -> *mut LinuxXpsMap {
    if queues.is_empty() {
        return core::ptr::null_mut();
    }
    // `XPS_MIN_MAP_ALLOC` is 20 for the configured 64-byte L1 cache line.
    let alloc_len = queues.len().max(20);
    let size = core::mem::size_of::<LinuxXpsMap>() + alloc_len * core::mem::size_of::<u16>();
    let raw = unsafe { crate::mm::slab::kzalloc_noprof(size, crate::mm::page_flags::GFP_KERNEL) };
    if raw.is_null() {
        return core::ptr::null_mut();
    }
    let map = raw.cast::<LinuxXpsMap>();
    unsafe {
        (*map).len = queues.len() as u32;
        (*map).alloc_len = alloc_len as u32;
        core::ptr::copy_nonoverlapping(
            queues.as_ptr(),
            raw.add(core::mem::size_of::<LinuxXpsMap>()).cast::<u16>(),
            queues.len(),
        );
    }
    map
}

/// `__netif_set_xps_queue()` — the RCU replace/update algorithm from
/// `vendor/linux/net/core/dev.c:2850`, for the normal `num_tc == 0` driver
/// configuration used during virtio-net probe.
#[unsafe(no_mangle)]
unsafe extern "C" fn __netif_set_xps_queue(
    dev: *mut u8,
    mask: *const u64,
    index: u16,
    map_type: i32,
) -> i32 {
    if dev.is_null() || !(0..=1).contains(&map_type) {
        return -crate::include::uapi::errno::EINVAL;
    }
    let num_tx = unsafe { read_field::<u32>(dev, NET_DEVICE_NUM_TX_QUEUES_OFFSET) };
    if index as u32 >= num_tx {
        return -crate::include::uapi::errno::EINVAL;
    }
    let configured_tc = unsafe { read_field::<i16>(dev, NET_DEVICE_NUM_TC_OFFSET) };
    if configured_tc != 0 {
        return -crate::include::uapi::errno::EINVAL;
    }
    let nr_ids = if map_type == 0 {
        crate::kernel::cpuhotplug::nr_cpu_ids() as usize
    } else {
        unsafe { read_field::<u32>(dev, NET_DEVICE_NUM_RX_QUEUES_OFFSET) as usize }
    }
    .max(1);
    let requested = if mask.is_null() {
        0
    } else {
        unsafe { mask.read() }
    };
    let online = if map_type == 0 {
        crate::kernel::cpuhotplug::cpu_online_mask()
    } else {
        u64::MAX
    };

    let _guard = XPS_MAP_LOCK.lock();
    let slot = unsafe {
        dev.add(NET_DEVICE_XPS_MAPS_OFFSET + map_type as usize * core::mem::size_of::<usize>())
            .cast::<*mut LinuxXpsDevMaps>()
    };
    let old = unsafe { slot.read() };
    let bytes =
        (core::mem::size_of::<LinuxXpsDevMaps>() + nr_ids * core::mem::size_of::<usize>()).max(64);
    let new_raw =
        unsafe { crate::mm::slab::kzalloc_noprof(bytes, crate::mm::page_flags::GFP_KERNEL) };
    if new_raw.is_null() {
        return -crate::include::uapi::errno::ENOMEM;
    }
    let new_maps = new_raw.cast::<LinuxXpsDevMaps>();
    unsafe {
        (*new_maps).nr_ids = nr_ids as u32;
        (*new_maps).num_tc = 1;
    }
    let attrs = unsafe {
        new_raw
            .add(core::mem::size_of::<LinuxXpsDevMaps>())
            .cast::<*mut LinuxXpsMap>()
    };
    for id in 0..nr_ids {
        let mut queues = unsafe { old_xps_queues(old, id, 0) };
        queues.retain(|queue| *queue != index);
        if id < 64 && requested & online & (1u64 << id) != 0 {
            queues.push(index);
        }
        let map = unsafe { allocate_xps_map(&queues) };
        if !queues.is_empty() && map.is_null() {
            unsafe { free_xps_maps(new_maps) };
            return -crate::include::uapi::errno::ENOMEM;
        }
        unsafe { attrs.add(id).write(map) };
    }
    core::sync::atomic::fence(core::sync::atomic::Ordering::Release);
    unsafe { slot.write(new_maps) };

    if map_type == 0 {
        let (queues, count) = unsafe { tx_queues(dev) };
        if !queues.is_null() && (index as usize) < count {
            unsafe {
                write_field(
                    queues.add(index as usize * NETDEV_QUEUE_SIZE),
                    NETDEV_QUEUE_NUMA_NODE_OFFSET,
                    -1i32,
                )
            };
        }
    }
    if !old.is_null() {
        crate::kernel::rcu::synchronize_rcu();
        unsafe { free_xps_maps(old) };
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtio_net_link_stubs_include_expected_core_symbols() {
        assert!(!FUNCTION_STUB_EXPORTS.contains(&"netif_tx_wake_queue"));
        assert!(!FUNCTION_STUB_EXPORTS.contains(&"register_netdevice"));
        assert!(FUNCTION_STUB_EXPORTS.contains(&"netdev_notice"));
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

    #[test]
    fn random_exports_include_scalar_helpers() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("get_random_u64"),
            Some(get_random_u64 as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("__get_random_u32_below"),
            Some(__get_random_u32_below as usize)
        );
    }

    #[test]
    fn legacy_nic_exports_include_napi_receive_and_stats_helpers() {
        register_module_exports();
        for (name, addr) in [
            ("__napi_schedule_irqoff", __napi_schedule_irqoff as usize),
            ("napi_enable_locked", napi_enable_locked as usize),
            ("linkwatch_fire_event", linkwatch_fire_event as usize),
            (
                "netif_get_num_default_rss_queues",
                netif_get_num_default_rss_queues as usize,
            ),
            ("__netdev_alloc_skb", __netdev_alloc_skb as usize),
            ("devm_alloc_etherdev_mqs", devm_alloc_etherdev_mqs as usize),
            ("__napi_alloc_frag_align", __napi_alloc_frag_align as usize),
            ("slab_build_skb", slab_build_skb as usize),
            ("netif_receive_skb", netif_receive_skb as usize),
            (
                "dev_kfree_skb_irq_reason",
                dev_kfree_skb_irq_reason as usize,
            ),
            ("device_get_mac_address", device_get_mac_address as usize),
            (
                "eth_platform_get_mac_address",
                eth_platform_get_mac_address as usize,
            ),
            ("skb_copy_and_csum_dev", skb_copy_and_csum_dev as usize),
            ("skb_copy_bits", skb_copy_bits as usize),
            ("skb_copy", skb_copy as usize),
            ("skb_copy_expand", skb_copy_expand as usize),
            ("skb_checksum_help", skb_checksum_help as usize),
            ("__skb_gso_segment", __skb_gso_segment as usize),
            ("dev_trans_start", dev_trans_start as usize),
            ("dev_close", dev_close as usize),
            ("netdev_update_features", netdev_update_features as usize),
            ("netif_set_tso_max_size", netif_set_tso_max_size as usize),
            ("netif_set_tso_max_segs", netif_set_tso_max_segs as usize),
            (
                "netdev_sw_irq_coalesce_default_on",
                netdev_sw_irq_coalesce_default_on as usize,
            ),
            ("dev_fetch_sw_netstats", dev_fetch_sw_netstats as usize),
            ("dev_get_stats", dev_get_stats as usize),
            ("eth_mac_addr", eth_mac_addr as usize),
            ("ethtool_puts", ethtool_puts as usize),
            ("ptp_clock_register", ptp_clock_register as usize),
            ("ptp_clock_unregister", ptp_clock_unregister as usize),
            ("ptp_clock_index", ptp_clock_index as usize),
            ("ptp_schedule_worker", ptp_schedule_worker as usize),
        ] {
            assert_eq!(crate::kernel::module::find_symbol(name), Some(addr));
        }
    }

    #[test]
    fn linkwatch_export_tracks_vendor_source() {
        let source = include_str!("../../vendor/linux/net/core/link_watch.c");

        assert!(source.contains("EXPORT_SYMBOL(linkwatch_fire_event);"));

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol_gpl_only("linkwatch_fire_event"),
            Some(false)
        );
    }

    #[test]
    fn netconsole_network_exports_track_vendor_sources() {
        let dev = include_str!("../../vendor/linux/net/core/dev.c");
        let rtnetlink = include_str!("../../vendor/linux/net/core/rtnetlink.c");
        let netpoll = include_str!("../../vendor/linux/net/core/netpoll.c");
        let skbuff = include_str!("../../vendor/linux/net/core/skbuff.c");

        assert!(dev.contains("EXPORT_SYMBOL(register_netdevice_notifier);"));
        assert!(dev.contains("EXPORT_SYMBOL(unregister_netdevice_notifier);"));
        assert!(rtnetlink.contains("EXPORT_SYMBOL(rtnl_is_locked);"));
        assert!(netpoll.contains("EXPORT_SYMBOL(netpoll_setup);"));
        assert!(netpoll.contains("EXPORT_SYMBOL(netpoll_cleanup);"));
        assert!(netpoll.contains("EXPORT_SYMBOL(do_netpoll_cleanup);"));
        assert!(netpoll.contains("EXPORT_SYMBOL(netpoll_poll_dev);"));
        assert!(netpoll.contains("EXPORT_SYMBOL(netpoll_send_skb);"));
        assert!(netpoll.contains("EXPORT_SYMBOL_NS_GPL(netpoll_zap_completion_queue"));
        assert!(skbuff.contains("EXPORT_SYMBOL(skb_push);"));
        assert!(skbuff.contains("EXPORT_SYMBOL(skb_dequeue);"));

        register_module_exports();
        for (name, addr, gpl_only) in [
            (
                "register_netdevice_notifier",
                register_netdevice_notifier as usize,
                false,
            ),
            (
                "unregister_netdevice_notifier",
                unregister_netdevice_notifier as usize,
                false,
            ),
            ("rtnl_is_locked", rtnl_is_locked as usize, false),
            ("netpoll_setup", netpoll_setup as usize, false),
            ("netpoll_cleanup", netpoll_cleanup as usize, false),
            ("do_netpoll_cleanup", do_netpoll_cleanup as usize, false),
            ("netpoll_poll_dev", netpoll_poll_dev as usize, false),
            ("netpoll_send_skb", netpoll_send_skb as usize, false),
            ("skb_push", linux_skb_push as usize, false),
            ("skb_dequeue", linux_skb_dequeue as usize, false),
            (
                "netpoll_zap_completion_queue",
                netpoll_zap_completion_queue as usize,
                true,
            ),
        ] {
            assert_eq!(crate::kernel::module::find_symbol(name), Some(addr));
            assert_eq!(
                crate::kernel::module::find_symbol_gpl_only(name),
                Some(gpl_only)
            );
        }
        assert_eq!(unsafe { netpoll_setup(core::ptr::null_mut()) }, -ENODEV);
    }

    #[test]
    fn ethernet_skb_exports_track_vendor_sources() {
        let ethernet = include_str!("../../vendor/linux/net/ethernet/eth.c");
        let net_devres = include_str!("../../vendor/linux/net/devres.c");
        let skbuff = include_str!("../../vendor/linux/net/core/skbuff.c");
        let dev = include_str!("../../vendor/linux/net/core/dev.c");
        let gso = include_str!("../../vendor/linux/net/core/gso.c");
        let ptp = include_str!("../../vendor/linux/drivers/ptp/ptp_clock.c");

        assert!(ethernet.contains("EXPORT_SYMBOL(device_get_mac_address);"));
        assert!(ethernet.contains("EXPORT_SYMBOL(eth_platform_get_mac_address);"));
        assert!(ethernet.contains("EXPORT_SYMBOL(eth_mac_addr);"));
        assert!(net_devres.contains("EXPORT_SYMBOL(devm_alloc_etherdev_mqs);"));
        assert!(skbuff.contains("EXPORT_SYMBOL(__napi_alloc_frag_align);"));
        assert!(skbuff.contains("EXPORT_SYMBOL(slab_build_skb);"));
        assert!(skbuff.contains("EXPORT_SYMBOL(skb_copy_bits);"));
        assert!(skbuff.contains("EXPORT_SYMBOL(skb_copy);"));
        assert!(skbuff.contains("EXPORT_SYMBOL(skb_copy_expand);"));
        assert!(dev.contains("EXPORT_SYMBOL(skb_checksum_help);"));
        assert!(gso.contains("EXPORT_SYMBOL(__skb_gso_segment);"));
        assert!(dev.contains("EXPORT_SYMBOL(netif_get_num_default_rss_queues);"));
        assert!(dev.contains("EXPORT_SYMBOL(netif_set_tso_max_size);"));
        assert!(dev.contains("EXPORT_SYMBOL(netif_set_tso_max_segs);"));
        assert!(dev.contains("EXPORT_SYMBOL_GPL(dev_fetch_sw_netstats);"));
        assert!(dev.contains("EXPORT_SYMBOL_GPL(netdev_sw_irq_coalesce_default_on);"));
        assert!(ptp.contains("EXPORT_SYMBOL(ptp_schedule_worker);"));

        register_module_exports();
        for name in [
            "device_get_mac_address",
            "eth_platform_get_mac_address",
            "eth_mac_addr",
            "devm_alloc_etherdev_mqs",
            "__napi_alloc_frag_align",
            "slab_build_skb",
            "skb_copy_bits",
            "skb_copy",
            "skb_copy_expand",
            "skb_checksum_help",
            "__skb_gso_segment",
            "netif_get_num_default_rss_queues",
            "netif_set_tso_max_size",
            "netif_set_tso_max_segs",
            "ptp_schedule_worker",
        ] {
            assert_eq!(
                crate::kernel::module::find_symbol_gpl_only(name),
                Some(false)
            );
        }
        assert_eq!(
            crate::kernel::module::find_symbol_gpl_only("netdev_sw_irq_coalesce_default_on"),
            Some(true)
        );
        assert_eq!(
            crate::kernel::module::find_symbol_gpl_only("dev_fetch_sw_netstats"),
            Some(true)
        );
    }

    #[test]
    fn netif_get_num_default_rss_queues_returns_at_least_one_queue() {
        assert!(unsafe { netif_get_num_default_rss_queues() } >= 1);
    }

    #[test]
    fn netif_set_tso_max_size_updates_tso_and_lower_gso_limits() {
        let mut dev = [0u8; NET_DEVICE_SIZE];
        let ptr = dev.as_mut_ptr();
        unsafe {
            write_field(ptr, NET_DEVICE_GSO_MAX_SIZE_OFFSET, GSO_MAX_SIZE);
            write_field(ptr, NET_DEVICE_GSO_IPV4_MAX_SIZE_OFFSET, GSO_MAX_SIZE);
            write_field(ptr, NET_DEVICE_TSO_MAX_SIZE_OFFSET, GSO_MAX_SIZE);
            netif_set_tso_max_size(ptr, 16_384);
            assert_eq!(
                read_field::<u32>(ptr, NET_DEVICE_TSO_MAX_SIZE_OFFSET),
                16_384
            );
            assert_eq!(
                read_field::<u32>(ptr, NET_DEVICE_GSO_MAX_SIZE_OFFSET),
                16_384
            );
            assert_eq!(
                read_field::<u32>(ptr, NET_DEVICE_GSO_IPV4_MAX_SIZE_OFFSET),
                16_384
            );
        }
    }

    #[test]
    fn netif_set_tso_max_segs_updates_tso_and_lower_gso_limits() {
        let mut dev = [0u8; NET_DEVICE_SIZE];
        let ptr = dev.as_mut_ptr();
        unsafe {
            write_field(ptr, NET_DEVICE_GSO_MAX_SEGS_OFFSET, u16::MAX);
            write_field(ptr, NET_DEVICE_TSO_MAX_SEGS_OFFSET, u16::MAX);
            netif_set_tso_max_segs(ptr, 128);
            assert_eq!(read_field::<u16>(ptr, NET_DEVICE_TSO_MAX_SEGS_OFFSET), 128);
            assert_eq!(read_field::<u16>(ptr, NET_DEVICE_GSO_MAX_SEGS_OFFSET), 128);
        }
    }

    #[test]
    fn skb_gso_segment_reports_unsupported_segmentation() {
        let err = unsafe { __skb_gso_segment(core::ptr::null_mut(), 0, true) };

        assert_eq!(
            err as isize,
            -(crate::include::uapi::errno::EOPNOTSUPP as isize)
        );
    }

    #[test]
    fn devm_alloc_etherdev_mqs_reuses_netdev_allocator_validation() {
        assert!(unsafe { devm_alloc_etherdev_mqs(core::ptr::null_mut(), 0, 0, 1) }.is_null());
    }

    #[test]
    fn eth_platform_get_mac_address_reports_no_platform_address() {
        let mut addr = [0u8; 6];

        assert_eq!(
            unsafe { eth_platform_get_mac_address(core::ptr::null_mut(), addr.as_mut_ptr()) },
            -crate::include::uapi::errno::ENODEV
        );
    }

    #[test]
    fn phylib_ethtool_exports_track_vendor_sources() {
        let common = include_str!("../../vendor/linux/net/ethtool/common.c");
        let cabletest = include_str!("../../vendor/linux/net/ethtool/cabletest.c");
        let stubs = include_str!("../../vendor/linux/drivers/net/phy/stubs.c");
        assert!(common.contains("EXPORT_SYMBOL_GPL(link_mode_params);"));
        assert!(common.contains("EXPORT_SYMBOL_GPL(ethtool_set_ethtool_phy_ops);"));
        assert!(common.contains("EXPORT_SYMBOL_GPL(ethtool_str_to_medium);"));
        assert!(cabletest.contains("EXPORT_SYMBOL_GPL(ethnl_cable_test_alloc);"));
        assert!(cabletest.contains("EXPORT_SYMBOL_GPL(ethnl_cable_test_free);"));
        assert!(cabletest.contains("EXPORT_SYMBOL_GPL(ethnl_cable_test_finished);"));
        assert!(cabletest.contains("EXPORT_SYMBOL_GPL(ethnl_cable_test_result_with_src);"));
        assert!(cabletest.contains("EXPORT_SYMBOL_GPL(ethnl_cable_test_fault_length_with_src);"));
        assert!(stubs.contains("EXPORT_SYMBOL_GPL(phylib_stubs);"));

        register_module_exports();
        for (name, addr) in [
            ("link_mode_params", LINUX_LINK_MODE_PARAMS.as_ptr() as usize),
            (
                "ethtool_set_ethtool_phy_ops",
                ethtool_set_ethtool_phy_ops as usize,
            ),
            ("ethtool_str_to_medium", ethtool_str_to_medium as usize),
            ("ethnl_cable_test_alloc", ethnl_cable_test_alloc as usize),
            ("ethnl_cable_test_free", ethnl_cable_test_free as usize),
            (
                "ethnl_cable_test_finished",
                ethnl_cable_test_finished as usize,
            ),
            (
                "ethnl_cable_test_result_with_src",
                ethnl_cable_test_result_with_src as usize,
            ),
            (
                "ethnl_cable_test_fault_length_with_src",
                ethnl_cable_test_fault_length_with_src as usize,
            ),
            (
                "phylib_stubs",
                core::ptr::addr_of_mut!(LINUX_PHYLIB_STUBS) as usize,
            ),
        ] {
            assert_eq!(crate::kernel::module::find_symbol(name), Some(addr));
        }
    }

    #[test]
    fn link_mode_params_table_matches_configured_vendor_abi_shape() {
        assert_eq!(core::mem::size_of::<LinuxLinkModeInfo>(), 12);
        assert_eq!(LINUX_LINK_MODE_PARAMS.len(), 125);
        assert!(
            LINUX_LINK_MODE_PARAMS
                .iter()
                .all(|info| info.speed == LINUX_SPEED_UNKNOWN)
        );
    }
}
