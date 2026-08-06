// SPDX-License-Identifier: ((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)
//! linux-source: include/uapi/linux/netdev.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016267

//! YNL-generated netdev generic-netlink UAPI definitions.

use core::ffi::c_int;

macro_rules! netdev_uapi_enum {
    ($name:ident) => {
        /// C `enum $name`, represented by the C ABI integer type.
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(pub c_int);
    };
}

netdev_uapi_enum!(netdev_xdp_act);
netdev_uapi_enum!(netdev_xdp_rx_metadata);
netdev_uapi_enum!(netdev_xsk_flags);
netdev_uapi_enum!(netdev_queue_type);
netdev_uapi_enum!(netdev_qstats_scope);
netdev_uapi_enum!(netdev_napi_threaded);

// C string-literal macros retain their terminating NUL byte.
pub const NETDEV_FAMILY_NAME: &[u8; 7] = b"netdev\0";
pub const NETDEV_FAMILY_VERSION: c_int = 1;

pub const NETDEV_XDP_ACT_BASIC: netdev_xdp_act = netdev_xdp_act(1);
pub const NETDEV_XDP_ACT_REDIRECT: netdev_xdp_act = netdev_xdp_act(2);
pub const NETDEV_XDP_ACT_NDO_XMIT: netdev_xdp_act = netdev_xdp_act(4);
pub const NETDEV_XDP_ACT_XSK_ZEROCOPY: netdev_xdp_act = netdev_xdp_act(8);
pub const NETDEV_XDP_ACT_HW_OFFLOAD: netdev_xdp_act = netdev_xdp_act(16);
pub const NETDEV_XDP_ACT_RX_SG: netdev_xdp_act = netdev_xdp_act(32);
pub const NETDEV_XDP_ACT_NDO_XMIT_SG: netdev_xdp_act = netdev_xdp_act(64);
pub const NETDEV_XDP_ACT_MASK: netdev_xdp_act = netdev_xdp_act(127);

pub const NETDEV_XDP_RX_METADATA_TIMESTAMP: netdev_xdp_rx_metadata = netdev_xdp_rx_metadata(1);
pub const NETDEV_XDP_RX_METADATA_HASH: netdev_xdp_rx_metadata = netdev_xdp_rx_metadata(2);
pub const NETDEV_XDP_RX_METADATA_VLAN_TAG: netdev_xdp_rx_metadata = netdev_xdp_rx_metadata(4);

pub const NETDEV_XSK_FLAGS_TX_TIMESTAMP: netdev_xsk_flags = netdev_xsk_flags(1);
pub const NETDEV_XSK_FLAGS_TX_CHECKSUM: netdev_xsk_flags = netdev_xsk_flags(2);
pub const NETDEV_XSK_FLAGS_TX_LAUNCH_TIME_FIFO: netdev_xsk_flags = netdev_xsk_flags(4);

pub const NETDEV_QUEUE_TYPE_RX: netdev_queue_type = netdev_queue_type(0);
pub const NETDEV_QUEUE_TYPE_TX: netdev_queue_type = netdev_queue_type(1);

pub const NETDEV_QSTATS_SCOPE_QUEUE: netdev_qstats_scope = netdev_qstats_scope(1);

pub const NETDEV_NAPI_THREADED_DISABLED: netdev_napi_threaded = netdev_napi_threaded(0);
pub const NETDEV_NAPI_THREADED_ENABLED: netdev_napi_threaded = netdev_napi_threaded(1);
pub const NETDEV_NAPI_THREADED_BUSY_POLL: netdev_napi_threaded = netdev_napi_threaded(2);

pub const NETDEV_A_DEV_IFINDEX: c_int = 1;
pub const NETDEV_A_DEV_PAD: c_int = 2;
pub const NETDEV_A_DEV_XDP_FEATURES: c_int = 3;
pub const NETDEV_A_DEV_XDP_ZC_MAX_SEGS: c_int = 4;
pub const NETDEV_A_DEV_XDP_RX_METADATA_FEATURES: c_int = 5;
pub const NETDEV_A_DEV_XSK_FEATURES: c_int = 6;
pub const __NETDEV_A_DEV_MAX: c_int = 7;
pub const NETDEV_A_DEV_MAX: c_int = __NETDEV_A_DEV_MAX - 1;

pub const NETDEV_A_IO_URING_PROVIDER_INFO_RX_BUF_LEN: c_int = 1;
pub const __NETDEV_A_IO_URING_PROVIDER_INFO_MAX: c_int = 2;
pub const NETDEV_A_IO_URING_PROVIDER_INFO_MAX: c_int = __NETDEV_A_IO_URING_PROVIDER_INFO_MAX - 1;

pub const NETDEV_A_PAGE_POOL_ID: c_int = 1;
pub const NETDEV_A_PAGE_POOL_IFINDEX: c_int = 2;
pub const NETDEV_A_PAGE_POOL_NAPI_ID: c_int = 3;
pub const NETDEV_A_PAGE_POOL_INFLIGHT: c_int = 4;
pub const NETDEV_A_PAGE_POOL_INFLIGHT_MEM: c_int = 5;
pub const NETDEV_A_PAGE_POOL_DETACH_TIME: c_int = 6;
pub const NETDEV_A_PAGE_POOL_DMABUF: c_int = 7;
pub const NETDEV_A_PAGE_POOL_IO_URING: c_int = 8;
pub const __NETDEV_A_PAGE_POOL_MAX: c_int = 9;
pub const NETDEV_A_PAGE_POOL_MAX: c_int = __NETDEV_A_PAGE_POOL_MAX - 1;

pub const NETDEV_A_PAGE_POOL_STATS_INFO: c_int = 1;
pub const NETDEV_A_PAGE_POOL_STATS_ALLOC_FAST: c_int = 8;
pub const NETDEV_A_PAGE_POOL_STATS_ALLOC_SLOW: c_int = 9;
pub const NETDEV_A_PAGE_POOL_STATS_ALLOC_SLOW_HIGH_ORDER: c_int = 10;
pub const NETDEV_A_PAGE_POOL_STATS_ALLOC_EMPTY: c_int = 11;
pub const NETDEV_A_PAGE_POOL_STATS_ALLOC_REFILL: c_int = 12;
pub const NETDEV_A_PAGE_POOL_STATS_ALLOC_WAIVE: c_int = 13;
pub const NETDEV_A_PAGE_POOL_STATS_RECYCLE_CACHED: c_int = 14;
pub const NETDEV_A_PAGE_POOL_STATS_RECYCLE_CACHE_FULL: c_int = 15;
pub const NETDEV_A_PAGE_POOL_STATS_RECYCLE_RING: c_int = 16;
pub const NETDEV_A_PAGE_POOL_STATS_RECYCLE_RING_FULL: c_int = 17;
pub const NETDEV_A_PAGE_POOL_STATS_RECYCLE_RELEASED_REFCNT: c_int = 18;
pub const __NETDEV_A_PAGE_POOL_STATS_MAX: c_int = 19;
pub const NETDEV_A_PAGE_POOL_STATS_MAX: c_int = __NETDEV_A_PAGE_POOL_STATS_MAX - 1;

pub const NETDEV_A_NAPI_IFINDEX: c_int = 1;
pub const NETDEV_A_NAPI_ID: c_int = 2;
pub const NETDEV_A_NAPI_IRQ: c_int = 3;
pub const NETDEV_A_NAPI_PID: c_int = 4;
pub const NETDEV_A_NAPI_DEFER_HARD_IRQS: c_int = 5;
pub const NETDEV_A_NAPI_GRO_FLUSH_TIMEOUT: c_int = 6;
pub const NETDEV_A_NAPI_IRQ_SUSPEND_TIMEOUT: c_int = 7;
pub const NETDEV_A_NAPI_THREADED: c_int = 8;
pub const __NETDEV_A_NAPI_MAX: c_int = 9;
pub const NETDEV_A_NAPI_MAX: c_int = __NETDEV_A_NAPI_MAX - 1;

pub const __NETDEV_A_XSK_INFO_MAX: c_int = 0;
pub const NETDEV_A_XSK_INFO_MAX: c_int = __NETDEV_A_XSK_INFO_MAX - 1;

pub const NETDEV_A_QUEUE_ID: c_int = 1;
pub const NETDEV_A_QUEUE_IFINDEX: c_int = 2;
pub const NETDEV_A_QUEUE_TYPE: c_int = 3;
pub const NETDEV_A_QUEUE_NAPI_ID: c_int = 4;
pub const NETDEV_A_QUEUE_DMABUF: c_int = 5;
pub const NETDEV_A_QUEUE_IO_URING: c_int = 6;
pub const NETDEV_A_QUEUE_XSK: c_int = 7;
pub const NETDEV_A_QUEUE_LEASE: c_int = 8;
pub const __NETDEV_A_QUEUE_MAX: c_int = 9;
pub const NETDEV_A_QUEUE_MAX: c_int = __NETDEV_A_QUEUE_MAX - 1;

pub const NETDEV_A_QSTATS_IFINDEX: c_int = 1;
pub const NETDEV_A_QSTATS_QUEUE_TYPE: c_int = 2;
pub const NETDEV_A_QSTATS_QUEUE_ID: c_int = 3;
pub const NETDEV_A_QSTATS_SCOPE: c_int = 4;
pub const NETDEV_A_QSTATS_RX_PACKETS: c_int = 8;
pub const NETDEV_A_QSTATS_RX_BYTES: c_int = 9;
pub const NETDEV_A_QSTATS_TX_PACKETS: c_int = 10;
pub const NETDEV_A_QSTATS_TX_BYTES: c_int = 11;
pub const NETDEV_A_QSTATS_RX_ALLOC_FAIL: c_int = 12;
pub const NETDEV_A_QSTATS_RX_HW_DROPS: c_int = 13;
pub const NETDEV_A_QSTATS_RX_HW_DROP_OVERRUNS: c_int = 14;
pub const NETDEV_A_QSTATS_RX_CSUM_COMPLETE: c_int = 15;
pub const NETDEV_A_QSTATS_RX_CSUM_UNNECESSARY: c_int = 16;
pub const NETDEV_A_QSTATS_RX_CSUM_NONE: c_int = 17;
pub const NETDEV_A_QSTATS_RX_CSUM_BAD: c_int = 18;
pub const NETDEV_A_QSTATS_RX_HW_GRO_PACKETS: c_int = 19;
pub const NETDEV_A_QSTATS_RX_HW_GRO_BYTES: c_int = 20;
pub const NETDEV_A_QSTATS_RX_HW_GRO_WIRE_PACKETS: c_int = 21;
pub const NETDEV_A_QSTATS_RX_HW_GRO_WIRE_BYTES: c_int = 22;
pub const NETDEV_A_QSTATS_RX_HW_DROP_RATELIMITS: c_int = 23;
pub const NETDEV_A_QSTATS_TX_HW_DROPS: c_int = 24;
pub const NETDEV_A_QSTATS_TX_HW_DROP_ERRORS: c_int = 25;
pub const NETDEV_A_QSTATS_TX_CSUM_NONE: c_int = 26;
pub const NETDEV_A_QSTATS_TX_NEEDS_CSUM: c_int = 27;
pub const NETDEV_A_QSTATS_TX_HW_GSO_PACKETS: c_int = 28;
pub const NETDEV_A_QSTATS_TX_HW_GSO_BYTES: c_int = 29;
pub const NETDEV_A_QSTATS_TX_HW_GSO_WIRE_PACKETS: c_int = 30;
pub const NETDEV_A_QSTATS_TX_HW_GSO_WIRE_BYTES: c_int = 31;
pub const NETDEV_A_QSTATS_TX_HW_DROP_RATELIMITS: c_int = 32;
pub const NETDEV_A_QSTATS_TX_STOP: c_int = 33;
pub const NETDEV_A_QSTATS_TX_WAKE: c_int = 34;
pub const __NETDEV_A_QSTATS_MAX: c_int = 35;
pub const NETDEV_A_QSTATS_MAX: c_int = __NETDEV_A_QSTATS_MAX - 1;

pub const NETDEV_A_LEASE_IFINDEX: c_int = 1;
pub const NETDEV_A_LEASE_QUEUE: c_int = 2;
pub const NETDEV_A_LEASE_NETNS_ID: c_int = 3;
pub const __NETDEV_A_LEASE_MAX: c_int = 4;
pub const NETDEV_A_LEASE_MAX: c_int = __NETDEV_A_LEASE_MAX - 1;

pub const NETDEV_A_DMABUF_IFINDEX: c_int = 1;
pub const NETDEV_A_DMABUF_QUEUES: c_int = 2;
pub const NETDEV_A_DMABUF_FD: c_int = 3;
pub const NETDEV_A_DMABUF_ID: c_int = 4;
pub const __NETDEV_A_DMABUF_MAX: c_int = 5;
pub const NETDEV_A_DMABUF_MAX: c_int = __NETDEV_A_DMABUF_MAX - 1;

pub const NETDEV_CMD_DEV_GET: c_int = 1;
pub const NETDEV_CMD_DEV_ADD_NTF: c_int = 2;
pub const NETDEV_CMD_DEV_DEL_NTF: c_int = 3;
pub const NETDEV_CMD_DEV_CHANGE_NTF: c_int = 4;
pub const NETDEV_CMD_PAGE_POOL_GET: c_int = 5;
pub const NETDEV_CMD_PAGE_POOL_ADD_NTF: c_int = 6;
pub const NETDEV_CMD_PAGE_POOL_DEL_NTF: c_int = 7;
pub const NETDEV_CMD_PAGE_POOL_CHANGE_NTF: c_int = 8;
pub const NETDEV_CMD_PAGE_POOL_STATS_GET: c_int = 9;
pub const NETDEV_CMD_QUEUE_GET: c_int = 10;
pub const NETDEV_CMD_NAPI_GET: c_int = 11;
pub const NETDEV_CMD_QSTATS_GET: c_int = 12;
pub const NETDEV_CMD_BIND_RX: c_int = 13;
pub const NETDEV_CMD_NAPI_SET: c_int = 14;
pub const NETDEV_CMD_BIND_TX: c_int = 15;
pub const NETDEV_CMD_QUEUE_CREATE: c_int = 16;
pub const __NETDEV_CMD_MAX: c_int = 17;
pub const NETDEV_CMD_MAX: c_int = __NETDEV_CMD_MAX - 1;

// C string-literal macros retain their terminating NUL byte.
pub const NETDEV_MCGRP_MGMT: &[u8; 5] = b"mgmt\0";
pub const NETDEV_MCGRP_PAGE_POOL: &[u8; 10] = b"page-pool\0";
