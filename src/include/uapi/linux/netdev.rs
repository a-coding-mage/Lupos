// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/uapi/linux/netdev.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016267

pub const NETDEV_FAMILY_NAME: &str = "netdev";
pub const NETDEV_FAMILY_VERSION: i32 = 1;

pub type netdev_xdp_act = i32;
pub const NETDEV_XDP_ACT_BASIC: netdev_xdp_act = 1;
pub const NETDEV_XDP_ACT_REDIRECT: netdev_xdp_act = 2;
pub const NETDEV_XDP_ACT_NDO_XMIT: netdev_xdp_act = 4;
pub const NETDEV_XDP_ACT_XSK_ZEROCOPY: netdev_xdp_act = 8;
pub const NETDEV_XDP_ACT_HW_OFFLOAD: netdev_xdp_act = 16;
pub const NETDEV_XDP_ACT_RX_SG: netdev_xdp_act = 32;
pub const NETDEV_XDP_ACT_NDO_XMIT_SG: netdev_xdp_act = 64;
pub const NETDEV_XDP_ACT_MASK: netdev_xdp_act = 127;

pub type netdev_xdp_rx_metadata = i32;
pub const NETDEV_XDP_RX_METADATA_TIMESTAMP: netdev_xdp_rx_metadata = 1;
pub const NETDEV_XDP_RX_METADATA_HASH: netdev_xdp_rx_metadata = 2;
pub const NETDEV_XDP_RX_METADATA_VLAN_TAG: netdev_xdp_rx_metadata = 4;

pub type netdev_xsk_flags = i32;
pub const NETDEV_XSK_FLAGS_TX_TIMESTAMP: netdev_xsk_flags = 1;
pub const NETDEV_XSK_FLAGS_TX_CHECKSUM: netdev_xsk_flags = 2;
pub const NETDEV_XSK_FLAGS_TX_LAUNCH_TIME_FIFO: netdev_xsk_flags = 4;

pub type netdev_queue_type = i32;
pub const NETDEV_QUEUE_TYPE_RX: netdev_queue_type = 0;
pub const NETDEV_QUEUE_TYPE_TX: netdev_queue_type = 1;

pub type netdev_qstats_scope = i32;
pub const NETDEV_QSTATS_SCOPE_QUEUE: netdev_qstats_scope = 1;

pub type netdev_napi_threaded = i32;
pub const NETDEV_NAPI_THREADED_DISABLED: netdev_napi_threaded = 0;
pub const NETDEV_NAPI_THREADED_ENABLED: netdev_napi_threaded = 1;
pub const NETDEV_NAPI_THREADED_BUSY_POLL: netdev_napi_threaded = 2;

pub const NETDEV_A_DEV_IFINDEX: i32 = 1;
pub const NETDEV_A_DEV_PAD: i32 = 2;
pub const NETDEV_A_DEV_XDP_FEATURES: i32 = 3;
pub const NETDEV_A_DEV_XDP_ZC_MAX_SEGS: i32 = 4;
pub const NETDEV_A_DEV_XDP_RX_METADATA_FEATURES: i32 = 5;
pub const NETDEV_A_DEV_XSK_FEATURES: i32 = 6;
pub const __NETDEV_A_DEV_MAX: i32 = 7;
pub const NETDEV_A_DEV_MAX: i32 = __NETDEV_A_DEV_MAX - 1;

pub const NETDEV_A_IO_URING_PROVIDER_INFO_RX_BUF_LEN: i32 = 1;
pub const __NETDEV_A_IO_URING_PROVIDER_INFO_MAX: i32 = 2;
pub const NETDEV_A_IO_URING_PROVIDER_INFO_MAX: i32 = __NETDEV_A_IO_URING_PROVIDER_INFO_MAX - 1;

pub const NETDEV_A_PAGE_POOL_ID: i32 = 1;
pub const NETDEV_A_PAGE_POOL_IFINDEX: i32 = 2;
pub const NETDEV_A_PAGE_POOL_NAPI_ID: i32 = 3;
pub const NETDEV_A_PAGE_POOL_INFLIGHT: i32 = 4;
pub const NETDEV_A_PAGE_POOL_INFLIGHT_MEM: i32 = 5;
pub const NETDEV_A_PAGE_POOL_DETACH_TIME: i32 = 6;
pub const NETDEV_A_PAGE_POOL_DMABUF: i32 = 7;
pub const NETDEV_A_PAGE_POOL_IO_URING: i32 = 8;
pub const __NETDEV_A_PAGE_POOL_MAX: i32 = 9;
pub const NETDEV_A_PAGE_POOL_MAX: i32 = __NETDEV_A_PAGE_POOL_MAX - 1;

pub const NETDEV_A_PAGE_POOL_STATS_INFO: i32 = 1;
pub const NETDEV_A_PAGE_POOL_STATS_ALLOC_FAST: i32 = 8;
pub const NETDEV_A_PAGE_POOL_STATS_ALLOC_SLOW: i32 = 9;
pub const NETDEV_A_PAGE_POOL_STATS_ALLOC_SLOW_HIGH_ORDER: i32 = 10;
pub const NETDEV_A_PAGE_POOL_STATS_ALLOC_EMPTY: i32 = 11;
pub const NETDEV_A_PAGE_POOL_STATS_ALLOC_REFILL: i32 = 12;
pub const NETDEV_A_PAGE_POOL_STATS_ALLOC_WAIVE: i32 = 13;
pub const NETDEV_A_PAGE_POOL_STATS_RECYCLE_CACHED: i32 = 14;
pub const NETDEV_A_PAGE_POOL_STATS_RECYCLE_CACHE_FULL: i32 = 15;
pub const NETDEV_A_PAGE_POOL_STATS_RECYCLE_RING: i32 = 16;
pub const NETDEV_A_PAGE_POOL_STATS_RECYCLE_RING_FULL: i32 = 17;
pub const NETDEV_A_PAGE_POOL_STATS_RECYCLE_RELEASED_REFCNT: i32 = 18;
pub const __NETDEV_A_PAGE_POOL_STATS_MAX: i32 = 19;
pub const NETDEV_A_PAGE_POOL_STATS_MAX: i32 = __NETDEV_A_PAGE_POOL_STATS_MAX - 1;

pub const NETDEV_A_NAPI_IFINDEX: i32 = 1;
pub const NETDEV_A_NAPI_ID: i32 = 2;
pub const NETDEV_A_NAPI_IRQ: i32 = 3;
pub const NETDEV_A_NAPI_PID: i32 = 4;
pub const NETDEV_A_NAPI_DEFER_HARD_IRQS: i32 = 5;
pub const NETDEV_A_NAPI_GRO_FLUSH_TIMEOUT: i32 = 6;
pub const NETDEV_A_NAPI_IRQ_SUSPEND_TIMEOUT: i32 = 7;
pub const NETDEV_A_NAPI_THREADED: i32 = 8;
pub const __NETDEV_A_NAPI_MAX: i32 = 9;
pub const NETDEV_A_NAPI_MAX: i32 = __NETDEV_A_NAPI_MAX - 1;

pub const __NETDEV_A_XSK_INFO_MAX: i32 = 0;
pub const NETDEV_A_XSK_INFO_MAX: i32 = __NETDEV_A_XSK_INFO_MAX - 1;

pub const NETDEV_A_QUEUE_ID: i32 = 1;
pub const NETDEV_A_QUEUE_IFINDEX: i32 = 2;
pub const NETDEV_A_QUEUE_TYPE: i32 = 3;
pub const NETDEV_A_QUEUE_NAPI_ID: i32 = 4;
pub const NETDEV_A_QUEUE_DMABUF: i32 = 5;
pub const NETDEV_A_QUEUE_IO_URING: i32 = 6;
pub const NETDEV_A_QUEUE_XSK: i32 = 7;
pub const NETDEV_A_QUEUE_LEASE: i32 = 8;
pub const __NETDEV_A_QUEUE_MAX: i32 = 9;
pub const NETDEV_A_QUEUE_MAX: i32 = __NETDEV_A_QUEUE_MAX - 1;

pub const NETDEV_A_QSTATS_IFINDEX: i32 = 1;
pub const NETDEV_A_QSTATS_QUEUE_TYPE: i32 = 2;
pub const NETDEV_A_QSTATS_QUEUE_ID: i32 = 3;
pub const NETDEV_A_QSTATS_SCOPE: i32 = 4;
pub const NETDEV_A_QSTATS_RX_PACKETS: i32 = 8;
pub const NETDEV_A_QSTATS_RX_BYTES: i32 = 9;
pub const NETDEV_A_QSTATS_TX_PACKETS: i32 = 10;
pub const NETDEV_A_QSTATS_TX_BYTES: i32 = 11;
pub const NETDEV_A_QSTATS_RX_ALLOC_FAIL: i32 = 12;
pub const NETDEV_A_QSTATS_RX_HW_DROPS: i32 = 13;
pub const NETDEV_A_QSTATS_RX_HW_DROP_OVERRUNS: i32 = 14;
pub const NETDEV_A_QSTATS_RX_CSUM_COMPLETE: i32 = 15;
pub const NETDEV_A_QSTATS_RX_CSUM_UNNECESSARY: i32 = 16;
pub const NETDEV_A_QSTATS_RX_CSUM_NONE: i32 = 17;
pub const NETDEV_A_QSTATS_RX_CSUM_BAD: i32 = 18;
pub const NETDEV_A_QSTATS_RX_HW_GRO_PACKETS: i32 = 19;
pub const NETDEV_A_QSTATS_RX_HW_GRO_BYTES: i32 = 20;
pub const NETDEV_A_QSTATS_RX_HW_GRO_WIRE_PACKETS: i32 = 21;
pub const NETDEV_A_QSTATS_RX_HW_GRO_WIRE_BYTES: i32 = 22;
pub const NETDEV_A_QSTATS_RX_HW_DROP_RATELIMITS: i32 = 23;
pub const NETDEV_A_QSTATS_TX_HW_DROPS: i32 = 24;
pub const NETDEV_A_QSTATS_TX_HW_DROP_ERRORS: i32 = 25;
pub const NETDEV_A_QSTATS_TX_CSUM_NONE: i32 = 26;
pub const NETDEV_A_QSTATS_TX_NEEDS_CSUM: i32 = 27;
pub const NETDEV_A_QSTATS_TX_HW_GSO_PACKETS: i32 = 28;
pub const NETDEV_A_QSTATS_TX_HW_GSO_BYTES: i32 = 29;
pub const NETDEV_A_QSTATS_TX_HW_GSO_WIRE_PACKETS: i32 = 30;
pub const NETDEV_A_QSTATS_TX_HW_GSO_WIRE_BYTES: i32 = 31;
pub const NETDEV_A_QSTATS_TX_HW_DROP_RATELIMITS: i32 = 32;
pub const NETDEV_A_QSTATS_TX_STOP: i32 = 33;
pub const NETDEV_A_QSTATS_TX_WAKE: i32 = 34;
pub const __NETDEV_A_QSTATS_MAX: i32 = 35;
pub const NETDEV_A_QSTATS_MAX: i32 = __NETDEV_A_QSTATS_MAX - 1;

pub const NETDEV_A_LEASE_IFINDEX: i32 = 1;
pub const NETDEV_A_LEASE_QUEUE: i32 = 2;
pub const NETDEV_A_LEASE_NETNS_ID: i32 = 3;
pub const __NETDEV_A_LEASE_MAX: i32 = 4;
pub const NETDEV_A_LEASE_MAX: i32 = __NETDEV_A_LEASE_MAX - 1;

pub const NETDEV_A_DMABUF_IFINDEX: i32 = 1;
pub const NETDEV_A_DMABUF_QUEUES: i32 = 2;
pub const NETDEV_A_DMABUF_FD: i32 = 3;
pub const NETDEV_A_DMABUF_ID: i32 = 4;
pub const __NETDEV_A_DMABUF_MAX: i32 = 5;
pub const NETDEV_A_DMABUF_MAX: i32 = __NETDEV_A_DMABUF_MAX - 1;

pub const NETDEV_CMD_DEV_GET: i32 = 1;
pub const NETDEV_CMD_DEV_ADD_NTF: i32 = 2;
pub const NETDEV_CMD_DEV_DEL_NTF: i32 = 3;
pub const NETDEV_CMD_DEV_CHANGE_NTF: i32 = 4;
pub const NETDEV_CMD_PAGE_POOL_GET: i32 = 5;
pub const NETDEV_CMD_PAGE_POOL_ADD_NTF: i32 = 6;
pub const NETDEV_CMD_PAGE_POOL_DEL_NTF: i32 = 7;
pub const NETDEV_CMD_PAGE_POOL_CHANGE_NTF: i32 = 8;
pub const NETDEV_CMD_PAGE_POOL_STATS_GET: i32 = 9;
pub const NETDEV_CMD_QUEUE_GET: i32 = 10;
pub const NETDEV_CMD_NAPI_GET: i32 = 11;
pub const NETDEV_CMD_QSTATS_GET: i32 = 12;
pub const NETDEV_CMD_BIND_RX: i32 = 13;
pub const NETDEV_CMD_NAPI_SET: i32 = 14;
pub const NETDEV_CMD_BIND_TX: i32 = 15;
pub const NETDEV_CMD_QUEUE_CREATE: i32 = 16;
pub const __NETDEV_CMD_MAX: i32 = 17;
pub const NETDEV_CMD_MAX: i32 = __NETDEV_CMD_MAX - 1;

pub const NETDEV_MCGRP_MGMT: &str = "mgmt";
pub const NETDEV_MCGRP_PAGE_POOL: &str = "page-pool";
