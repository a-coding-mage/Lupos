// SPDX-License-Identifier: ((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)
//! linux-source: include/uapi/linux/ethtool_netlink_generated.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016119

//! Auto-generated ethtool generic-netlink UAPI numeric definitions.

macro_rules! ethtool_uapi_enum {
    ($name:ident) => {
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(pub u32);
    };
}

// Named C enum tags retain distinct type identity and their enumerator values.
ethtool_uapi_enum!(ethtool_header_flags);
ethtool_uapi_enum!(ethtool_tcp_data_split);
ethtool_uapi_enum!(hwtstamp_source);
ethtool_uapi_enum!(ethtool_pse_event);

// C string-literal macro: the terminating NUL is part of the macro value.
pub const ETHTOOL_GENL_NAME: &[u8; 8] = b"ethtool\0";
pub const ETHTOOL_GENL_VERSION: u32 = 1;

// anonymous C enum
pub const ETHTOOL_UDP_TUNNEL_TYPE_VXLAN: u32 = 0;
pub const ETHTOOL_UDP_TUNNEL_TYPE_GENEVE: u32 = 1;
pub const ETHTOOL_UDP_TUNNEL_TYPE_VXLAN_GPE: u32 = 2;
pub const __ETHTOOL_UDP_TUNNEL_TYPE_CNT: u32 = 3;
pub const ETHTOOL_UDP_TUNNEL_TYPE_MAX: u32 = __ETHTOOL_UDP_TUNNEL_TYPE_CNT - 1;

// enum ethtool_header_flags
pub const ETHTOOL_FLAG_COMPACT_BITSETS: ethtool_header_flags = ethtool_header_flags(1);
pub const ETHTOOL_FLAG_OMIT_REPLY: ethtool_header_flags = ethtool_header_flags(2);
pub const ETHTOOL_FLAG_STATS: ethtool_header_flags = ethtool_header_flags(4);

// enum ethtool_tcp_data_split
pub const ETHTOOL_TCP_DATA_SPLIT_UNKNOWN: ethtool_tcp_data_split = ethtool_tcp_data_split(0);
pub const ETHTOOL_TCP_DATA_SPLIT_DISABLED: ethtool_tcp_data_split = ethtool_tcp_data_split(1);
pub const ETHTOOL_TCP_DATA_SPLIT_ENABLED: ethtool_tcp_data_split = ethtool_tcp_data_split(2);

// enum hwtstamp_source
pub const HWTSTAMP_SOURCE_NETDEV: hwtstamp_source = hwtstamp_source(1);
pub const HWTSTAMP_SOURCE_PHYLIB: hwtstamp_source = hwtstamp_source(2);

// enum ethtool_pse_event
pub const ETHTOOL_PSE_EVENT_OVER_CURRENT: ethtool_pse_event = ethtool_pse_event(1);
pub const ETHTOOL_PSE_EVENT_OVER_TEMP: ethtool_pse_event = ethtool_pse_event(2);
pub const ETHTOOL_C33_PSE_EVENT_DETECTION: ethtool_pse_event = ethtool_pse_event(4);
pub const ETHTOOL_C33_PSE_EVENT_CLASSIFICATION: ethtool_pse_event = ethtool_pse_event(8);
pub const ETHTOOL_C33_PSE_EVENT_DISCONNECTION: ethtool_pse_event = ethtool_pse_event(16);
pub const ETHTOOL_PSE_EVENT_OVER_BUDGET: ethtool_pse_event = ethtool_pse_event(32);
pub const ETHTOOL_PSE_EVENT_SW_PW_CONTROL_ERROR: ethtool_pse_event = ethtool_pse_event(64);

// anonymous C enum
pub const ETHTOOL_A_HEADER_UNSPEC: u32 = 0;
pub const ETHTOOL_A_HEADER_DEV_INDEX: u32 = 1;
pub const ETHTOOL_A_HEADER_DEV_NAME: u32 = 2;
pub const ETHTOOL_A_HEADER_FLAGS: u32 = 3;
pub const ETHTOOL_A_HEADER_PHY_INDEX: u32 = 4;
pub const __ETHTOOL_A_HEADER_CNT: u32 = 5;
pub const ETHTOOL_A_HEADER_MAX: u32 = __ETHTOOL_A_HEADER_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_BITSET_BIT_UNSPEC: u32 = 0;
pub const ETHTOOL_A_BITSET_BIT_INDEX: u32 = 1;
pub const ETHTOOL_A_BITSET_BIT_NAME: u32 = 2;
pub const ETHTOOL_A_BITSET_BIT_VALUE: u32 = 3;
pub const __ETHTOOL_A_BITSET_BIT_CNT: u32 = 4;
pub const ETHTOOL_A_BITSET_BIT_MAX: u32 = __ETHTOOL_A_BITSET_BIT_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_BITSET_BITS_UNSPEC: u32 = 0;
pub const ETHTOOL_A_BITSET_BITS_BIT: u32 = 1;
pub const __ETHTOOL_A_BITSET_BITS_CNT: u32 = 2;
pub const ETHTOOL_A_BITSET_BITS_MAX: u32 = __ETHTOOL_A_BITSET_BITS_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_BITSET_UNSPEC: u32 = 0;
pub const ETHTOOL_A_BITSET_NOMASK: u32 = 1;
pub const ETHTOOL_A_BITSET_SIZE: u32 = 2;
pub const ETHTOOL_A_BITSET_BITS: u32 = 3;
pub const ETHTOOL_A_BITSET_VALUE: u32 = 4;
pub const ETHTOOL_A_BITSET_MASK: u32 = 5;
pub const __ETHTOOL_A_BITSET_CNT: u32 = 6;
pub const ETHTOOL_A_BITSET_MAX: u32 = __ETHTOOL_A_BITSET_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_STRING_UNSPEC: u32 = 0;
pub const ETHTOOL_A_STRING_INDEX: u32 = 1;
pub const ETHTOOL_A_STRING_VALUE: u32 = 2;
pub const __ETHTOOL_A_STRING_CNT: u32 = 3;
pub const ETHTOOL_A_STRING_MAX: u32 = __ETHTOOL_A_STRING_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_STRINGS_UNSPEC: u32 = 0;
pub const ETHTOOL_A_STRINGS_STRING: u32 = 1;
pub const __ETHTOOL_A_STRINGS_CNT: u32 = 2;
pub const ETHTOOL_A_STRINGS_MAX: u32 = __ETHTOOL_A_STRINGS_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_STRINGSET_UNSPEC: u32 = 0;
pub const ETHTOOL_A_STRINGSET_ID: u32 = 1;
pub const ETHTOOL_A_STRINGSET_COUNT: u32 = 2;
pub const ETHTOOL_A_STRINGSET_STRINGS: u32 = 3;
pub const __ETHTOOL_A_STRINGSET_CNT: u32 = 4;
pub const ETHTOOL_A_STRINGSET_MAX: u32 = __ETHTOOL_A_STRINGSET_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_STRINGSETS_UNSPEC: u32 = 0;
pub const ETHTOOL_A_STRINGSETS_STRINGSET: u32 = 1;
pub const __ETHTOOL_A_STRINGSETS_CNT: u32 = 2;
pub const ETHTOOL_A_STRINGSETS_MAX: u32 = __ETHTOOL_A_STRINGSETS_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_STRSET_UNSPEC: u32 = 0;
pub const ETHTOOL_A_STRSET_HEADER: u32 = 1;
pub const ETHTOOL_A_STRSET_STRINGSETS: u32 = 2;
pub const ETHTOOL_A_STRSET_COUNTS_ONLY: u32 = 3;
pub const __ETHTOOL_A_STRSET_CNT: u32 = 4;
pub const ETHTOOL_A_STRSET_MAX: u32 = __ETHTOOL_A_STRSET_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_PRIVFLAGS_UNSPEC: u32 = 0;
pub const ETHTOOL_A_PRIVFLAGS_HEADER: u32 = 1;
pub const ETHTOOL_A_PRIVFLAGS_FLAGS: u32 = 2;
pub const __ETHTOOL_A_PRIVFLAGS_CNT: u32 = 3;
pub const ETHTOOL_A_PRIVFLAGS_MAX: u32 = __ETHTOOL_A_PRIVFLAGS_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_RINGS_UNSPEC: u32 = 0;
pub const ETHTOOL_A_RINGS_HEADER: u32 = 1;
pub const ETHTOOL_A_RINGS_RX_MAX: u32 = 2;
pub const ETHTOOL_A_RINGS_RX_MINI_MAX: u32 = 3;
pub const ETHTOOL_A_RINGS_RX_JUMBO_MAX: u32 = 4;
pub const ETHTOOL_A_RINGS_TX_MAX: u32 = 5;
pub const ETHTOOL_A_RINGS_RX: u32 = 6;
pub const ETHTOOL_A_RINGS_RX_MINI: u32 = 7;
pub const ETHTOOL_A_RINGS_RX_JUMBO: u32 = 8;
pub const ETHTOOL_A_RINGS_TX: u32 = 9;
pub const ETHTOOL_A_RINGS_RX_BUF_LEN: u32 = 10;
pub const ETHTOOL_A_RINGS_TCP_DATA_SPLIT: u32 = 11;
pub const ETHTOOL_A_RINGS_CQE_SIZE: u32 = 12;
pub const ETHTOOL_A_RINGS_TX_PUSH: u32 = 13;
pub const ETHTOOL_A_RINGS_RX_PUSH: u32 = 14;
pub const ETHTOOL_A_RINGS_TX_PUSH_BUF_LEN: u32 = 15;
pub const ETHTOOL_A_RINGS_TX_PUSH_BUF_LEN_MAX: u32 = 16;
pub const ETHTOOL_A_RINGS_HDS_THRESH: u32 = 17;
pub const ETHTOOL_A_RINGS_HDS_THRESH_MAX: u32 = 18;
pub const __ETHTOOL_A_RINGS_CNT: u32 = 19;
pub const ETHTOOL_A_RINGS_MAX: u32 = __ETHTOOL_A_RINGS_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_MM_STAT_UNSPEC: u32 = 0;
pub const ETHTOOL_A_MM_STAT_PAD: u32 = 1;
pub const ETHTOOL_A_MM_STAT_REASSEMBLY_ERRORS: u32 = 2;
pub const ETHTOOL_A_MM_STAT_SMD_ERRORS: u32 = 3;
pub const ETHTOOL_A_MM_STAT_REASSEMBLY_OK: u32 = 4;
pub const ETHTOOL_A_MM_STAT_RX_FRAG_COUNT: u32 = 5;
pub const ETHTOOL_A_MM_STAT_TX_FRAG_COUNT: u32 = 6;
pub const ETHTOOL_A_MM_STAT_HOLD_COUNT: u32 = 7;
pub const __ETHTOOL_A_MM_STAT_CNT: u32 = 8;
pub const ETHTOOL_A_MM_STAT_MAX: u32 = __ETHTOOL_A_MM_STAT_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_MM_UNSPEC: u32 = 0;
pub const ETHTOOL_A_MM_HEADER: u32 = 1;
pub const ETHTOOL_A_MM_PMAC_ENABLED: u32 = 2;
pub const ETHTOOL_A_MM_TX_ENABLED: u32 = 3;
pub const ETHTOOL_A_MM_TX_ACTIVE: u32 = 4;
pub const ETHTOOL_A_MM_TX_MIN_FRAG_SIZE: u32 = 5;
pub const ETHTOOL_A_MM_RX_MIN_FRAG_SIZE: u32 = 6;
pub const ETHTOOL_A_MM_VERIFY_ENABLED: u32 = 7;
pub const ETHTOOL_A_MM_VERIFY_STATUS: u32 = 8;
pub const ETHTOOL_A_MM_VERIFY_TIME: u32 = 9;
pub const ETHTOOL_A_MM_MAX_VERIFY_TIME: u32 = 10;
pub const ETHTOOL_A_MM_STATS: u32 = 11;
pub const __ETHTOOL_A_MM_CNT: u32 = 12;
pub const ETHTOOL_A_MM_MAX: u32 = __ETHTOOL_A_MM_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_LINKINFO_UNSPEC: u32 = 0;
pub const ETHTOOL_A_LINKINFO_HEADER: u32 = 1;
pub const ETHTOOL_A_LINKINFO_PORT: u32 = 2;
pub const ETHTOOL_A_LINKINFO_PHYADDR: u32 = 3;
pub const ETHTOOL_A_LINKINFO_TP_MDIX: u32 = 4;
pub const ETHTOOL_A_LINKINFO_TP_MDIX_CTRL: u32 = 5;
pub const ETHTOOL_A_LINKINFO_TRANSCEIVER: u32 = 6;
pub const __ETHTOOL_A_LINKINFO_CNT: u32 = 7;
pub const ETHTOOL_A_LINKINFO_MAX: u32 = __ETHTOOL_A_LINKINFO_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_LINKMODES_UNSPEC: u32 = 0;
pub const ETHTOOL_A_LINKMODES_HEADER: u32 = 1;
pub const ETHTOOL_A_LINKMODES_AUTONEG: u32 = 2;
pub const ETHTOOL_A_LINKMODES_OURS: u32 = 3;
pub const ETHTOOL_A_LINKMODES_PEER: u32 = 4;
pub const ETHTOOL_A_LINKMODES_SPEED: u32 = 5;
pub const ETHTOOL_A_LINKMODES_DUPLEX: u32 = 6;
pub const ETHTOOL_A_LINKMODES_MASTER_SLAVE_CFG: u32 = 7;
pub const ETHTOOL_A_LINKMODES_MASTER_SLAVE_STATE: u32 = 8;
pub const ETHTOOL_A_LINKMODES_LANES: u32 = 9;
pub const ETHTOOL_A_LINKMODES_RATE_MATCHING: u32 = 10;
pub const __ETHTOOL_A_LINKMODES_CNT: u32 = 11;
pub const ETHTOOL_A_LINKMODES_MAX: u32 = __ETHTOOL_A_LINKMODES_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_LINKSTATE_UNSPEC: u32 = 0;
pub const ETHTOOL_A_LINKSTATE_HEADER: u32 = 1;
pub const ETHTOOL_A_LINKSTATE_LINK: u32 = 2;
pub const ETHTOOL_A_LINKSTATE_SQI: u32 = 3;
pub const ETHTOOL_A_LINKSTATE_SQI_MAX: u32 = 4;
pub const ETHTOOL_A_LINKSTATE_EXT_STATE: u32 = 5;
pub const ETHTOOL_A_LINKSTATE_EXT_SUBSTATE: u32 = 6;
pub const ETHTOOL_A_LINKSTATE_EXT_DOWN_CNT: u32 = 7;
pub const __ETHTOOL_A_LINKSTATE_CNT: u32 = 8;
pub const ETHTOOL_A_LINKSTATE_MAX: u32 = __ETHTOOL_A_LINKSTATE_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_DEBUG_UNSPEC: u32 = 0;
pub const ETHTOOL_A_DEBUG_HEADER: u32 = 1;
pub const ETHTOOL_A_DEBUG_MSGMASK: u32 = 2;
pub const __ETHTOOL_A_DEBUG_CNT: u32 = 3;
pub const ETHTOOL_A_DEBUG_MAX: u32 = __ETHTOOL_A_DEBUG_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_WOL_UNSPEC: u32 = 0;
pub const ETHTOOL_A_WOL_HEADER: u32 = 1;
pub const ETHTOOL_A_WOL_MODES: u32 = 2;
pub const ETHTOOL_A_WOL_SOPASS: u32 = 3;
pub const __ETHTOOL_A_WOL_CNT: u32 = 4;
pub const ETHTOOL_A_WOL_MAX: u32 = __ETHTOOL_A_WOL_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_FEATURES_UNSPEC: u32 = 0;
pub const ETHTOOL_A_FEATURES_HEADER: u32 = 1;
pub const ETHTOOL_A_FEATURES_HW: u32 = 2;
pub const ETHTOOL_A_FEATURES_WANTED: u32 = 3;
pub const ETHTOOL_A_FEATURES_ACTIVE: u32 = 4;
pub const ETHTOOL_A_FEATURES_NOCHANGE: u32 = 5;
pub const __ETHTOOL_A_FEATURES_CNT: u32 = 6;
pub const ETHTOOL_A_FEATURES_MAX: u32 = __ETHTOOL_A_FEATURES_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_CHANNELS_UNSPEC: u32 = 0;
pub const ETHTOOL_A_CHANNELS_HEADER: u32 = 1;
pub const ETHTOOL_A_CHANNELS_RX_MAX: u32 = 2;
pub const ETHTOOL_A_CHANNELS_TX_MAX: u32 = 3;
pub const ETHTOOL_A_CHANNELS_OTHER_MAX: u32 = 4;
pub const ETHTOOL_A_CHANNELS_COMBINED_MAX: u32 = 5;
pub const ETHTOOL_A_CHANNELS_RX_COUNT: u32 = 6;
pub const ETHTOOL_A_CHANNELS_TX_COUNT: u32 = 7;
pub const ETHTOOL_A_CHANNELS_OTHER_COUNT: u32 = 8;
pub const ETHTOOL_A_CHANNELS_COMBINED_COUNT: u32 = 9;
pub const __ETHTOOL_A_CHANNELS_CNT: u32 = 10;
pub const ETHTOOL_A_CHANNELS_MAX: u32 = __ETHTOOL_A_CHANNELS_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_IRQ_MODERATION_UNSPEC: u32 = 0;
pub const ETHTOOL_A_IRQ_MODERATION_USEC: u32 = 1;
pub const ETHTOOL_A_IRQ_MODERATION_PKTS: u32 = 2;
pub const ETHTOOL_A_IRQ_MODERATION_COMPS: u32 = 3;
pub const __ETHTOOL_A_IRQ_MODERATION_CNT: u32 = 4;
pub const ETHTOOL_A_IRQ_MODERATION_MAX: u32 = __ETHTOOL_A_IRQ_MODERATION_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_PROFILE_UNSPEC: u32 = 0;
pub const ETHTOOL_A_PROFILE_IRQ_MODERATION: u32 = 1;
pub const __ETHTOOL_A_PROFILE_CNT: u32 = 2;
pub const ETHTOOL_A_PROFILE_MAX: u32 = __ETHTOOL_A_PROFILE_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_COALESCE_UNSPEC: u32 = 0;
pub const ETHTOOL_A_COALESCE_HEADER: u32 = 1;
pub const ETHTOOL_A_COALESCE_RX_USECS: u32 = 2;
pub const ETHTOOL_A_COALESCE_RX_MAX_FRAMES: u32 = 3;
pub const ETHTOOL_A_COALESCE_RX_USECS_IRQ: u32 = 4;
pub const ETHTOOL_A_COALESCE_RX_MAX_FRAMES_IRQ: u32 = 5;
pub const ETHTOOL_A_COALESCE_TX_USECS: u32 = 6;
pub const ETHTOOL_A_COALESCE_TX_MAX_FRAMES: u32 = 7;
pub const ETHTOOL_A_COALESCE_TX_USECS_IRQ: u32 = 8;
pub const ETHTOOL_A_COALESCE_TX_MAX_FRAMES_IRQ: u32 = 9;
pub const ETHTOOL_A_COALESCE_STATS_BLOCK_USECS: u32 = 10;
pub const ETHTOOL_A_COALESCE_USE_ADAPTIVE_RX: u32 = 11;
pub const ETHTOOL_A_COALESCE_USE_ADAPTIVE_TX: u32 = 12;
pub const ETHTOOL_A_COALESCE_PKT_RATE_LOW: u32 = 13;
pub const ETHTOOL_A_COALESCE_RX_USECS_LOW: u32 = 14;
pub const ETHTOOL_A_COALESCE_RX_MAX_FRAMES_LOW: u32 = 15;
pub const ETHTOOL_A_COALESCE_TX_USECS_LOW: u32 = 16;
pub const ETHTOOL_A_COALESCE_TX_MAX_FRAMES_LOW: u32 = 17;
pub const ETHTOOL_A_COALESCE_PKT_RATE_HIGH: u32 = 18;
pub const ETHTOOL_A_COALESCE_RX_USECS_HIGH: u32 = 19;
pub const ETHTOOL_A_COALESCE_RX_MAX_FRAMES_HIGH: u32 = 20;
pub const ETHTOOL_A_COALESCE_TX_USECS_HIGH: u32 = 21;
pub const ETHTOOL_A_COALESCE_TX_MAX_FRAMES_HIGH: u32 = 22;
pub const ETHTOOL_A_COALESCE_RATE_SAMPLE_INTERVAL: u32 = 23;
pub const ETHTOOL_A_COALESCE_USE_CQE_MODE_TX: u32 = 24;
pub const ETHTOOL_A_COALESCE_USE_CQE_MODE_RX: u32 = 25;
pub const ETHTOOL_A_COALESCE_TX_AGGR_MAX_BYTES: u32 = 26;
pub const ETHTOOL_A_COALESCE_TX_AGGR_MAX_FRAMES: u32 = 27;
pub const ETHTOOL_A_COALESCE_TX_AGGR_TIME_USECS: u32 = 28;
pub const ETHTOOL_A_COALESCE_RX_PROFILE: u32 = 29;
pub const ETHTOOL_A_COALESCE_TX_PROFILE: u32 = 30;
pub const ETHTOOL_A_COALESCE_RX_CQE_FRAMES: u32 = 31;
pub const ETHTOOL_A_COALESCE_RX_CQE_NSECS: u32 = 32;
pub const __ETHTOOL_A_COALESCE_CNT: u32 = 33;
pub const ETHTOOL_A_COALESCE_MAX: u32 = __ETHTOOL_A_COALESCE_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_PAUSE_STAT_UNSPEC: u32 = 0;
pub const ETHTOOL_A_PAUSE_STAT_PAD: u32 = 1;
pub const ETHTOOL_A_PAUSE_STAT_TX_FRAMES: u32 = 2;
pub const ETHTOOL_A_PAUSE_STAT_RX_FRAMES: u32 = 3;
pub const ETHTOOL_A_PAUSE_STAT_TX_PAUSE_STORM_EVENTS: u32 = 4;
pub const __ETHTOOL_A_PAUSE_STAT_CNT: u32 = 5;
pub const ETHTOOL_A_PAUSE_STAT_MAX: u32 = __ETHTOOL_A_PAUSE_STAT_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_PAUSE_UNSPEC: u32 = 0;
pub const ETHTOOL_A_PAUSE_HEADER: u32 = 1;
pub const ETHTOOL_A_PAUSE_AUTONEG: u32 = 2;
pub const ETHTOOL_A_PAUSE_RX: u32 = 3;
pub const ETHTOOL_A_PAUSE_TX: u32 = 4;
pub const ETHTOOL_A_PAUSE_STATS: u32 = 5;
pub const ETHTOOL_A_PAUSE_STATS_SRC: u32 = 6;
pub const __ETHTOOL_A_PAUSE_CNT: u32 = 7;
pub const ETHTOOL_A_PAUSE_MAX: u32 = __ETHTOOL_A_PAUSE_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_EEE_UNSPEC: u32 = 0;
pub const ETHTOOL_A_EEE_HEADER: u32 = 1;
pub const ETHTOOL_A_EEE_MODES_OURS: u32 = 2;
pub const ETHTOOL_A_EEE_MODES_PEER: u32 = 3;
pub const ETHTOOL_A_EEE_ACTIVE: u32 = 4;
pub const ETHTOOL_A_EEE_ENABLED: u32 = 5;
pub const ETHTOOL_A_EEE_TX_LPI_ENABLED: u32 = 6;
pub const ETHTOOL_A_EEE_TX_LPI_TIMER: u32 = 7;
pub const __ETHTOOL_A_EEE_CNT: u32 = 8;
pub const ETHTOOL_A_EEE_MAX: u32 = __ETHTOOL_A_EEE_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_TS_STAT_UNSPEC: u32 = 0;
pub const ETHTOOL_A_TS_STAT_TX_PKTS: u32 = 1;
pub const ETHTOOL_A_TS_STAT_TX_LOST: u32 = 2;
pub const ETHTOOL_A_TS_STAT_TX_ERR: u32 = 3;
pub const ETHTOOL_A_TS_STAT_TX_ONESTEP_PKTS_UNCONFIRMED: u32 = 4;
pub const __ETHTOOL_A_TS_STAT_CNT: u32 = 5;
pub const ETHTOOL_A_TS_STAT_MAX: u32 = __ETHTOOL_A_TS_STAT_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_TS_HWTSTAMP_PROVIDER_UNSPEC: u32 = 0;
pub const ETHTOOL_A_TS_HWTSTAMP_PROVIDER_INDEX: u32 = 1;
pub const ETHTOOL_A_TS_HWTSTAMP_PROVIDER_QUALIFIER: u32 = 2;
pub const __ETHTOOL_A_TS_HWTSTAMP_PROVIDER_CNT: u32 = 3;
pub const ETHTOOL_A_TS_HWTSTAMP_PROVIDER_MAX: u32 = __ETHTOOL_A_TS_HWTSTAMP_PROVIDER_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_TSINFO_UNSPEC: u32 = 0;
pub const ETHTOOL_A_TSINFO_HEADER: u32 = 1;
pub const ETHTOOL_A_TSINFO_TIMESTAMPING: u32 = 2;
pub const ETHTOOL_A_TSINFO_TX_TYPES: u32 = 3;
pub const ETHTOOL_A_TSINFO_RX_FILTERS: u32 = 4;
pub const ETHTOOL_A_TSINFO_PHC_INDEX: u32 = 5;
pub const ETHTOOL_A_TSINFO_STATS: u32 = 6;
pub const ETHTOOL_A_TSINFO_HWTSTAMP_PROVIDER: u32 = 7;
pub const ETHTOOL_A_TSINFO_HWTSTAMP_SOURCE: u32 = 8;
pub const ETHTOOL_A_TSINFO_HWTSTAMP_PHYINDEX: u32 = 9;
pub const __ETHTOOL_A_TSINFO_CNT: u32 = 10;
pub const ETHTOOL_A_TSINFO_MAX: u32 = __ETHTOOL_A_TSINFO_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_CABLE_RESULT_UNSPEC: u32 = 0;
pub const ETHTOOL_A_CABLE_RESULT_PAIR: u32 = 1;
pub const ETHTOOL_A_CABLE_RESULT_CODE: u32 = 2;
pub const ETHTOOL_A_CABLE_RESULT_SRC: u32 = 3;
pub const __ETHTOOL_A_CABLE_RESULT_CNT: u32 = 4;
pub const ETHTOOL_A_CABLE_RESULT_MAX: u32 = __ETHTOOL_A_CABLE_RESULT_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_CABLE_FAULT_LENGTH_UNSPEC: u32 = 0;
pub const ETHTOOL_A_CABLE_FAULT_LENGTH_PAIR: u32 = 1;
pub const ETHTOOL_A_CABLE_FAULT_LENGTH_CM: u32 = 2;
pub const ETHTOOL_A_CABLE_FAULT_LENGTH_SRC: u32 = 3;
pub const __ETHTOOL_A_CABLE_FAULT_LENGTH_CNT: u32 = 4;
pub const ETHTOOL_A_CABLE_FAULT_LENGTH_MAX: u32 = __ETHTOOL_A_CABLE_FAULT_LENGTH_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_CABLE_NEST_UNSPEC: u32 = 0;
pub const ETHTOOL_A_CABLE_NEST_RESULT: u32 = 1;
pub const ETHTOOL_A_CABLE_NEST_FAULT_LENGTH: u32 = 2;
pub const __ETHTOOL_A_CABLE_NEST_CNT: u32 = 3;
pub const ETHTOOL_A_CABLE_NEST_MAX: u32 = __ETHTOOL_A_CABLE_NEST_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_CABLE_TEST_UNSPEC: u32 = 0;
pub const ETHTOOL_A_CABLE_TEST_HEADER: u32 = 1;
pub const __ETHTOOL_A_CABLE_TEST_CNT: u32 = 2;
pub const ETHTOOL_A_CABLE_TEST_MAX: u32 = __ETHTOOL_A_CABLE_TEST_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_CABLE_TEST_NTF_UNSPEC: u32 = 0;
pub const ETHTOOL_A_CABLE_TEST_NTF_HEADER: u32 = 1;
pub const ETHTOOL_A_CABLE_TEST_NTF_STATUS: u32 = 2;
pub const ETHTOOL_A_CABLE_TEST_NTF_NEST: u32 = 3;
pub const __ETHTOOL_A_CABLE_TEST_NTF_CNT: u32 = 4;
pub const ETHTOOL_A_CABLE_TEST_NTF_MAX: u32 = __ETHTOOL_A_CABLE_TEST_NTF_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_CABLE_TEST_TDR_CFG_UNSPEC: u32 = 0;
pub const ETHTOOL_A_CABLE_TEST_TDR_CFG_FIRST: u32 = 1;
pub const ETHTOOL_A_CABLE_TEST_TDR_CFG_LAST: u32 = 2;
pub const ETHTOOL_A_CABLE_TEST_TDR_CFG_STEP: u32 = 3;
pub const ETHTOOL_A_CABLE_TEST_TDR_CFG_PAIR: u32 = 4;
pub const __ETHTOOL_A_CABLE_TEST_TDR_CFG_CNT: u32 = 5;
pub const ETHTOOL_A_CABLE_TEST_TDR_CFG_MAX: u32 = __ETHTOOL_A_CABLE_TEST_TDR_CFG_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_CABLE_TEST_TDR_NTF_UNSPEC: u32 = 0;
pub const ETHTOOL_A_CABLE_TEST_TDR_NTF_HEADER: u32 = 1;
pub const ETHTOOL_A_CABLE_TEST_TDR_NTF_STATUS: u32 = 2;
pub const ETHTOOL_A_CABLE_TEST_TDR_NTF_NEST: u32 = 3;
pub const __ETHTOOL_A_CABLE_TEST_TDR_NTF_CNT: u32 = 4;
pub const ETHTOOL_A_CABLE_TEST_TDR_NTF_MAX: u32 = __ETHTOOL_A_CABLE_TEST_TDR_NTF_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_CABLE_TEST_TDR_UNSPEC: u32 = 0;
pub const ETHTOOL_A_CABLE_TEST_TDR_HEADER: u32 = 1;
pub const ETHTOOL_A_CABLE_TEST_TDR_CFG: u32 = 2;
pub const __ETHTOOL_A_CABLE_TEST_TDR_CNT: u32 = 3;
pub const ETHTOOL_A_CABLE_TEST_TDR_MAX: u32 = __ETHTOOL_A_CABLE_TEST_TDR_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_TUNNEL_UDP_ENTRY_UNSPEC: u32 = 0;
pub const ETHTOOL_A_TUNNEL_UDP_ENTRY_PORT: u32 = 1;
pub const ETHTOOL_A_TUNNEL_UDP_ENTRY_TYPE: u32 = 2;
pub const __ETHTOOL_A_TUNNEL_UDP_ENTRY_CNT: u32 = 3;
pub const ETHTOOL_A_TUNNEL_UDP_ENTRY_MAX: u32 = __ETHTOOL_A_TUNNEL_UDP_ENTRY_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_TUNNEL_UDP_TABLE_UNSPEC: u32 = 0;
pub const ETHTOOL_A_TUNNEL_UDP_TABLE_SIZE: u32 = 1;
pub const ETHTOOL_A_TUNNEL_UDP_TABLE_TYPES: u32 = 2;
pub const ETHTOOL_A_TUNNEL_UDP_TABLE_ENTRY: u32 = 3;
pub const __ETHTOOL_A_TUNNEL_UDP_TABLE_CNT: u32 = 4;
pub const ETHTOOL_A_TUNNEL_UDP_TABLE_MAX: u32 = __ETHTOOL_A_TUNNEL_UDP_TABLE_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_TUNNEL_UDP_UNSPEC: u32 = 0;
pub const ETHTOOL_A_TUNNEL_UDP_TABLE: u32 = 1;
pub const __ETHTOOL_A_TUNNEL_UDP_CNT: u32 = 2;
pub const ETHTOOL_A_TUNNEL_UDP_MAX: u32 = __ETHTOOL_A_TUNNEL_UDP_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_TUNNEL_INFO_UNSPEC: u32 = 0;
pub const ETHTOOL_A_TUNNEL_INFO_HEADER: u32 = 1;
pub const ETHTOOL_A_TUNNEL_INFO_UDP_PORTS: u32 = 2;
pub const __ETHTOOL_A_TUNNEL_INFO_CNT: u32 = 3;
pub const ETHTOOL_A_TUNNEL_INFO_MAX: u32 = __ETHTOOL_A_TUNNEL_INFO_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_FEC_HIST_PAD: u32 = 1;
pub const ETHTOOL_A_FEC_HIST_BIN_LOW: u32 = 2;
pub const ETHTOOL_A_FEC_HIST_BIN_HIGH: u32 = 3;
pub const ETHTOOL_A_FEC_HIST_BIN_VAL: u32 = 4;
pub const ETHTOOL_A_FEC_HIST_BIN_VAL_PER_LANE: u32 = 5;
pub const __ETHTOOL_A_FEC_HIST_CNT: u32 = 6;
pub const ETHTOOL_A_FEC_HIST_MAX: u32 = __ETHTOOL_A_FEC_HIST_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_FEC_STAT_UNSPEC: u32 = 0;
pub const ETHTOOL_A_FEC_STAT_PAD: u32 = 1;
pub const ETHTOOL_A_FEC_STAT_CORRECTED: u32 = 2;
pub const ETHTOOL_A_FEC_STAT_UNCORR: u32 = 3;
pub const ETHTOOL_A_FEC_STAT_CORR_BITS: u32 = 4;
pub const ETHTOOL_A_FEC_STAT_HIST: u32 = 5;
pub const __ETHTOOL_A_FEC_STAT_CNT: u32 = 6;
pub const ETHTOOL_A_FEC_STAT_MAX: u32 = __ETHTOOL_A_FEC_STAT_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_FEC_UNSPEC: u32 = 0;
pub const ETHTOOL_A_FEC_HEADER: u32 = 1;
pub const ETHTOOL_A_FEC_MODES: u32 = 2;
pub const ETHTOOL_A_FEC_AUTO: u32 = 3;
pub const ETHTOOL_A_FEC_ACTIVE: u32 = 4;
pub const ETHTOOL_A_FEC_STATS: u32 = 5;
pub const __ETHTOOL_A_FEC_CNT: u32 = 6;
pub const ETHTOOL_A_FEC_MAX: u32 = __ETHTOOL_A_FEC_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_MODULE_EEPROM_UNSPEC: u32 = 0;
pub const ETHTOOL_A_MODULE_EEPROM_HEADER: u32 = 1;
pub const ETHTOOL_A_MODULE_EEPROM_OFFSET: u32 = 2;
pub const ETHTOOL_A_MODULE_EEPROM_LENGTH: u32 = 3;
pub const ETHTOOL_A_MODULE_EEPROM_PAGE: u32 = 4;
pub const ETHTOOL_A_MODULE_EEPROM_BANK: u32 = 5;
pub const ETHTOOL_A_MODULE_EEPROM_I2C_ADDRESS: u32 = 6;
pub const ETHTOOL_A_MODULE_EEPROM_DATA: u32 = 7;
pub const __ETHTOOL_A_MODULE_EEPROM_CNT: u32 = 8;
pub const ETHTOOL_A_MODULE_EEPROM_MAX: u32 = __ETHTOOL_A_MODULE_EEPROM_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_STATS_GRP_UNSPEC: u32 = 0;
pub const ETHTOOL_A_STATS_GRP_PAD: u32 = 1;
pub const ETHTOOL_A_STATS_GRP_ID: u32 = 2;
pub const ETHTOOL_A_STATS_GRP_SS_ID: u32 = 3;
pub const ETHTOOL_A_STATS_GRP_STAT: u32 = 4;
pub const ETHTOOL_A_STATS_GRP_HIST_RX: u32 = 5;
pub const ETHTOOL_A_STATS_GRP_HIST_TX: u32 = 6;
pub const ETHTOOL_A_STATS_GRP_HIST_BKT_LOW: u32 = 7;
pub const ETHTOOL_A_STATS_GRP_HIST_BKT_HI: u32 = 8;
pub const ETHTOOL_A_STATS_GRP_HIST_VAL: u32 = 9;
pub const __ETHTOOL_A_STATS_GRP_CNT: u32 = 10;
pub const ETHTOOL_A_STATS_GRP_MAX: u32 = __ETHTOOL_A_STATS_GRP_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_STATS_UNSPEC: u32 = 0;
pub const ETHTOOL_A_STATS_PAD: u32 = 1;
pub const ETHTOOL_A_STATS_HEADER: u32 = 2;
pub const ETHTOOL_A_STATS_GROUPS: u32 = 3;
pub const ETHTOOL_A_STATS_GRP: u32 = 4;
pub const ETHTOOL_A_STATS_SRC: u32 = 5;
pub const __ETHTOOL_A_STATS_CNT: u32 = 6;
pub const ETHTOOL_A_STATS_MAX: u32 = __ETHTOOL_A_STATS_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_PHC_VCLOCKS_UNSPEC: u32 = 0;
pub const ETHTOOL_A_PHC_VCLOCKS_HEADER: u32 = 1;
pub const ETHTOOL_A_PHC_VCLOCKS_NUM: u32 = 2;
pub const ETHTOOL_A_PHC_VCLOCKS_INDEX: u32 = 3;
pub const __ETHTOOL_A_PHC_VCLOCKS_CNT: u32 = 4;
pub const ETHTOOL_A_PHC_VCLOCKS_MAX: u32 = __ETHTOOL_A_PHC_VCLOCKS_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_MODULE_UNSPEC: u32 = 0;
pub const ETHTOOL_A_MODULE_HEADER: u32 = 1;
pub const ETHTOOL_A_MODULE_POWER_MODE_POLICY: u32 = 2;
pub const ETHTOOL_A_MODULE_POWER_MODE: u32 = 3;
pub const __ETHTOOL_A_MODULE_CNT: u32 = 4;
pub const ETHTOOL_A_MODULE_MAX: u32 = __ETHTOOL_A_MODULE_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_C33_PSE_PW_LIMIT_UNSPEC: u32 = 0;
pub const ETHTOOL_A_C33_PSE_PW_LIMIT_MIN: u32 = 1;
pub const ETHTOOL_A_C33_PSE_PW_LIMIT_MAX: u32 = 2;
pub const __ETHTOOL_A_C33_PSE_PW_LIMIT_CNT: u32 = 3;
pub const __ETHTOOL_A_C33_PSE_PW_LIMIT_MAX: u32 = __ETHTOOL_A_C33_PSE_PW_LIMIT_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_PSE_UNSPEC: u32 = 0;
pub const ETHTOOL_A_PSE_HEADER: u32 = 1;
pub const ETHTOOL_A_PODL_PSE_ADMIN_STATE: u32 = 2;
pub const ETHTOOL_A_PODL_PSE_ADMIN_CONTROL: u32 = 3;
pub const ETHTOOL_A_PODL_PSE_PW_D_STATUS: u32 = 4;
pub const ETHTOOL_A_C33_PSE_ADMIN_STATE: u32 = 5;
pub const ETHTOOL_A_C33_PSE_ADMIN_CONTROL: u32 = 6;
pub const ETHTOOL_A_C33_PSE_PW_D_STATUS: u32 = 7;
pub const ETHTOOL_A_C33_PSE_PW_CLASS: u32 = 8;
pub const ETHTOOL_A_C33_PSE_ACTUAL_PW: u32 = 9;
pub const ETHTOOL_A_C33_PSE_EXT_STATE: u32 = 10;
pub const ETHTOOL_A_C33_PSE_EXT_SUBSTATE: u32 = 11;
pub const ETHTOOL_A_C33_PSE_AVAIL_PW_LIMIT: u32 = 12;
pub const ETHTOOL_A_C33_PSE_PW_LIMIT_RANGES: u32 = 13;
pub const ETHTOOL_A_PSE_PW_D_ID: u32 = 14;
pub const ETHTOOL_A_PSE_PRIO_MAX: u32 = 15;
pub const ETHTOOL_A_PSE_PRIO: u32 = 16;
pub const __ETHTOOL_A_PSE_CNT: u32 = 17;
pub const ETHTOOL_A_PSE_MAX: u32 = __ETHTOOL_A_PSE_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_FLOW_ETHER: u32 = 1;
pub const ETHTOOL_A_FLOW_IP4: u32 = 2;
pub const ETHTOOL_A_FLOW_IP6: u32 = 3;
pub const ETHTOOL_A_FLOW_TCP4: u32 = 4;
pub const ETHTOOL_A_FLOW_TCP6: u32 = 5;
pub const ETHTOOL_A_FLOW_UDP4: u32 = 6;
pub const ETHTOOL_A_FLOW_UDP6: u32 = 7;
pub const ETHTOOL_A_FLOW_SCTP4: u32 = 8;
pub const ETHTOOL_A_FLOW_SCTP6: u32 = 9;
pub const ETHTOOL_A_FLOW_AH4: u32 = 10;
pub const ETHTOOL_A_FLOW_AH6: u32 = 11;
pub const ETHTOOL_A_FLOW_ESP4: u32 = 12;
pub const ETHTOOL_A_FLOW_ESP6: u32 = 13;
pub const ETHTOOL_A_FLOW_AH_ESP4: u32 = 14;
pub const ETHTOOL_A_FLOW_AH_ESP6: u32 = 15;
pub const ETHTOOL_A_FLOW_GTPU4: u32 = 16;
pub const ETHTOOL_A_FLOW_GTPU6: u32 = 17;
pub const ETHTOOL_A_FLOW_GTPC4: u32 = 18;
pub const ETHTOOL_A_FLOW_GTPC6: u32 = 19;
pub const ETHTOOL_A_FLOW_GTPC_TEID4: u32 = 20;
pub const ETHTOOL_A_FLOW_GTPC_TEID6: u32 = 21;
pub const ETHTOOL_A_FLOW_GTPU_EH4: u32 = 22;
pub const ETHTOOL_A_FLOW_GTPU_EH6: u32 = 23;
pub const ETHTOOL_A_FLOW_GTPU_UL4: u32 = 24;
pub const ETHTOOL_A_FLOW_GTPU_UL6: u32 = 25;
pub const ETHTOOL_A_FLOW_GTPU_DL4: u32 = 26;
pub const ETHTOOL_A_FLOW_GTPU_DL6: u32 = 27;
pub const __ETHTOOL_A_FLOW_CNT: u32 = 28;
pub const ETHTOOL_A_FLOW_MAX: u32 = __ETHTOOL_A_FLOW_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_RSS_UNSPEC: u32 = 0;
pub const ETHTOOL_A_RSS_HEADER: u32 = 1;
pub const ETHTOOL_A_RSS_CONTEXT: u32 = 2;
pub const ETHTOOL_A_RSS_HFUNC: u32 = 3;
pub const ETHTOOL_A_RSS_INDIR: u32 = 4;
pub const ETHTOOL_A_RSS_HKEY: u32 = 5;
pub const ETHTOOL_A_RSS_INPUT_XFRM: u32 = 6;
pub const ETHTOOL_A_RSS_START_CONTEXT: u32 = 7;
pub const ETHTOOL_A_RSS_FLOW_HASH: u32 = 8;
pub const __ETHTOOL_A_RSS_CNT: u32 = 9;
pub const ETHTOOL_A_RSS_MAX: u32 = __ETHTOOL_A_RSS_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_PLCA_UNSPEC: u32 = 0;
pub const ETHTOOL_A_PLCA_HEADER: u32 = 1;
pub const ETHTOOL_A_PLCA_VERSION: u32 = 2;
pub const ETHTOOL_A_PLCA_ENABLED: u32 = 3;
pub const ETHTOOL_A_PLCA_STATUS: u32 = 4;
pub const ETHTOOL_A_PLCA_NODE_CNT: u32 = 5;
pub const ETHTOOL_A_PLCA_NODE_ID: u32 = 6;
pub const ETHTOOL_A_PLCA_TO_TMR: u32 = 7;
pub const ETHTOOL_A_PLCA_BURST_CNT: u32 = 8;
pub const ETHTOOL_A_PLCA_BURST_TMR: u32 = 9;
pub const __ETHTOOL_A_PLCA_CNT: u32 = 10;
pub const ETHTOOL_A_PLCA_MAX: u32 = __ETHTOOL_A_PLCA_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_MODULE_FW_FLASH_UNSPEC: u32 = 0;
pub const ETHTOOL_A_MODULE_FW_FLASH_HEADER: u32 = 1;
pub const ETHTOOL_A_MODULE_FW_FLASH_FILE_NAME: u32 = 2;
pub const ETHTOOL_A_MODULE_FW_FLASH_PASSWORD: u32 = 3;
pub const ETHTOOL_A_MODULE_FW_FLASH_STATUS: u32 = 4;
pub const ETHTOOL_A_MODULE_FW_FLASH_STATUS_MSG: u32 = 5;
pub const ETHTOOL_A_MODULE_FW_FLASH_DONE: u32 = 6;
pub const ETHTOOL_A_MODULE_FW_FLASH_TOTAL: u32 = 7;
pub const __ETHTOOL_A_MODULE_FW_FLASH_CNT: u32 = 8;
pub const ETHTOOL_A_MODULE_FW_FLASH_MAX: u32 = __ETHTOOL_A_MODULE_FW_FLASH_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_PHY_UNSPEC: u32 = 0;
pub const ETHTOOL_A_PHY_HEADER: u32 = 1;
pub const ETHTOOL_A_PHY_INDEX: u32 = 2;
pub const ETHTOOL_A_PHY_DRVNAME: u32 = 3;
pub const ETHTOOL_A_PHY_NAME: u32 = 4;
pub const ETHTOOL_A_PHY_UPSTREAM_TYPE: u32 = 5;
pub const ETHTOOL_A_PHY_UPSTREAM_INDEX: u32 = 6;
pub const ETHTOOL_A_PHY_UPSTREAM_SFP_NAME: u32 = 7;
pub const ETHTOOL_A_PHY_DOWNSTREAM_SFP_NAME: u32 = 8;
pub const __ETHTOOL_A_PHY_CNT: u32 = 9;
pub const ETHTOOL_A_PHY_MAX: u32 = __ETHTOOL_A_PHY_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_TSCONFIG_UNSPEC: u32 = 0;
pub const ETHTOOL_A_TSCONFIG_HEADER: u32 = 1;
pub const ETHTOOL_A_TSCONFIG_HWTSTAMP_PROVIDER: u32 = 2;
pub const ETHTOOL_A_TSCONFIG_TX_TYPES: u32 = 3;
pub const ETHTOOL_A_TSCONFIG_RX_FILTERS: u32 = 4;
pub const ETHTOOL_A_TSCONFIG_HWTSTAMP_FLAGS: u32 = 5;
pub const __ETHTOOL_A_TSCONFIG_CNT: u32 = 6;
pub const ETHTOOL_A_TSCONFIG_MAX: u32 = __ETHTOOL_A_TSCONFIG_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_PSE_NTF_HEADER: u32 = 1;
pub const ETHTOOL_A_PSE_NTF_EVENTS: u32 = 2;
pub const __ETHTOOL_A_PSE_NTF_CNT: u32 = 3;
pub const ETHTOOL_A_PSE_NTF_MAX: u32 = __ETHTOOL_A_PSE_NTF_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_MSE_CAPABILITIES_MAX_AVERAGE_MSE: u32 = 1;
pub const ETHTOOL_A_MSE_CAPABILITIES_MAX_PEAK_MSE: u32 = 2;
pub const ETHTOOL_A_MSE_CAPABILITIES_REFRESH_RATE_PS: u32 = 3;
pub const ETHTOOL_A_MSE_CAPABILITIES_NUM_SYMBOLS: u32 = 4;
pub const __ETHTOOL_A_MSE_CAPABILITIES_CNT: u32 = 5;
pub const ETHTOOL_A_MSE_CAPABILITIES_MAX: u32 = __ETHTOOL_A_MSE_CAPABILITIES_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_MSE_SNAPSHOT_AVERAGE_MSE: u32 = 1;
pub const ETHTOOL_A_MSE_SNAPSHOT_PEAK_MSE: u32 = 2;
pub const ETHTOOL_A_MSE_SNAPSHOT_WORST_PEAK_MSE: u32 = 3;
pub const __ETHTOOL_A_MSE_SNAPSHOT_CNT: u32 = 4;
pub const ETHTOOL_A_MSE_SNAPSHOT_MAX: u32 = __ETHTOOL_A_MSE_SNAPSHOT_CNT - 1;

// anonymous C enum
pub const ETHTOOL_A_MSE_HEADER: u32 = 1;
pub const ETHTOOL_A_MSE_CAPABILITIES: u32 = 2;
pub const ETHTOOL_A_MSE_CHANNEL_A: u32 = 3;
pub const ETHTOOL_A_MSE_CHANNEL_B: u32 = 4;
pub const ETHTOOL_A_MSE_CHANNEL_C: u32 = 5;
pub const ETHTOOL_A_MSE_CHANNEL_D: u32 = 6;
pub const ETHTOOL_A_MSE_WORST_CHANNEL: u32 = 7;
pub const ETHTOOL_A_MSE_LINK: u32 = 8;
pub const __ETHTOOL_A_MSE_CNT: u32 = 9;
pub const ETHTOOL_A_MSE_MAX: u32 = __ETHTOOL_A_MSE_CNT - 1;

// anonymous C enum
pub const ETHTOOL_MSG_USER_NONE: u32 = 0;
pub const ETHTOOL_MSG_STRSET_GET: u32 = 1;
pub const ETHTOOL_MSG_LINKINFO_GET: u32 = 2;
pub const ETHTOOL_MSG_LINKINFO_SET: u32 = 3;
pub const ETHTOOL_MSG_LINKMODES_GET: u32 = 4;
pub const ETHTOOL_MSG_LINKMODES_SET: u32 = 5;
pub const ETHTOOL_MSG_LINKSTATE_GET: u32 = 6;
pub const ETHTOOL_MSG_DEBUG_GET: u32 = 7;
pub const ETHTOOL_MSG_DEBUG_SET: u32 = 8;
pub const ETHTOOL_MSG_WOL_GET: u32 = 9;
pub const ETHTOOL_MSG_WOL_SET: u32 = 10;
pub const ETHTOOL_MSG_FEATURES_GET: u32 = 11;
pub const ETHTOOL_MSG_FEATURES_SET: u32 = 12;
pub const ETHTOOL_MSG_PRIVFLAGS_GET: u32 = 13;
pub const ETHTOOL_MSG_PRIVFLAGS_SET: u32 = 14;
pub const ETHTOOL_MSG_RINGS_GET: u32 = 15;
pub const ETHTOOL_MSG_RINGS_SET: u32 = 16;
pub const ETHTOOL_MSG_CHANNELS_GET: u32 = 17;
pub const ETHTOOL_MSG_CHANNELS_SET: u32 = 18;
pub const ETHTOOL_MSG_COALESCE_GET: u32 = 19;
pub const ETHTOOL_MSG_COALESCE_SET: u32 = 20;
pub const ETHTOOL_MSG_PAUSE_GET: u32 = 21;
pub const ETHTOOL_MSG_PAUSE_SET: u32 = 22;
pub const ETHTOOL_MSG_EEE_GET: u32 = 23;
pub const ETHTOOL_MSG_EEE_SET: u32 = 24;
pub const ETHTOOL_MSG_TSINFO_GET: u32 = 25;
pub const ETHTOOL_MSG_CABLE_TEST_ACT: u32 = 26;
pub const ETHTOOL_MSG_CABLE_TEST_TDR_ACT: u32 = 27;
pub const ETHTOOL_MSG_TUNNEL_INFO_GET: u32 = 28;
pub const ETHTOOL_MSG_FEC_GET: u32 = 29;
pub const ETHTOOL_MSG_FEC_SET: u32 = 30;
pub const ETHTOOL_MSG_MODULE_EEPROM_GET: u32 = 31;
pub const ETHTOOL_MSG_STATS_GET: u32 = 32;
pub const ETHTOOL_MSG_PHC_VCLOCKS_GET: u32 = 33;
pub const ETHTOOL_MSG_MODULE_GET: u32 = 34;
pub const ETHTOOL_MSG_MODULE_SET: u32 = 35;
pub const ETHTOOL_MSG_PSE_GET: u32 = 36;
pub const ETHTOOL_MSG_PSE_SET: u32 = 37;
pub const ETHTOOL_MSG_RSS_GET: u32 = 38;
pub const ETHTOOL_MSG_PLCA_GET_CFG: u32 = 39;
pub const ETHTOOL_MSG_PLCA_SET_CFG: u32 = 40;
pub const ETHTOOL_MSG_PLCA_GET_STATUS: u32 = 41;
pub const ETHTOOL_MSG_MM_GET: u32 = 42;
pub const ETHTOOL_MSG_MM_SET: u32 = 43;
pub const ETHTOOL_MSG_MODULE_FW_FLASH_ACT: u32 = 44;
pub const ETHTOOL_MSG_PHY_GET: u32 = 45;
pub const ETHTOOL_MSG_TSCONFIG_GET: u32 = 46;
pub const ETHTOOL_MSG_TSCONFIG_SET: u32 = 47;
pub const ETHTOOL_MSG_RSS_SET: u32 = 48;
pub const ETHTOOL_MSG_RSS_CREATE_ACT: u32 = 49;
pub const ETHTOOL_MSG_RSS_DELETE_ACT: u32 = 50;
pub const ETHTOOL_MSG_MSE_GET: u32 = 51;
pub const __ETHTOOL_MSG_USER_CNT: u32 = 52;
pub const ETHTOOL_MSG_USER_MAX: u32 = __ETHTOOL_MSG_USER_CNT - 1;

// anonymous C enum
pub const ETHTOOL_MSG_KERNEL_NONE: u32 = 0;
pub const ETHTOOL_MSG_STRSET_GET_REPLY: u32 = 1;
pub const ETHTOOL_MSG_LINKINFO_GET_REPLY: u32 = 2;
pub const ETHTOOL_MSG_LINKINFO_NTF: u32 = 3;
pub const ETHTOOL_MSG_LINKMODES_GET_REPLY: u32 = 4;
pub const ETHTOOL_MSG_LINKMODES_NTF: u32 = 5;
pub const ETHTOOL_MSG_LINKSTATE_GET_REPLY: u32 = 6;
pub const ETHTOOL_MSG_DEBUG_GET_REPLY: u32 = 7;
pub const ETHTOOL_MSG_DEBUG_NTF: u32 = 8;
pub const ETHTOOL_MSG_WOL_GET_REPLY: u32 = 9;
pub const ETHTOOL_MSG_WOL_NTF: u32 = 10;
pub const ETHTOOL_MSG_FEATURES_GET_REPLY: u32 = 11;
pub const ETHTOOL_MSG_FEATURES_SET_REPLY: u32 = 12;
pub const ETHTOOL_MSG_FEATURES_NTF: u32 = 13;
pub const ETHTOOL_MSG_PRIVFLAGS_GET_REPLY: u32 = 14;
pub const ETHTOOL_MSG_PRIVFLAGS_NTF: u32 = 15;
pub const ETHTOOL_MSG_RINGS_GET_REPLY: u32 = 16;
pub const ETHTOOL_MSG_RINGS_NTF: u32 = 17;
pub const ETHTOOL_MSG_CHANNELS_GET_REPLY: u32 = 18;
pub const ETHTOOL_MSG_CHANNELS_NTF: u32 = 19;
pub const ETHTOOL_MSG_COALESCE_GET_REPLY: u32 = 20;
pub const ETHTOOL_MSG_COALESCE_NTF: u32 = 21;
pub const ETHTOOL_MSG_PAUSE_GET_REPLY: u32 = 22;
pub const ETHTOOL_MSG_PAUSE_NTF: u32 = 23;
pub const ETHTOOL_MSG_EEE_GET_REPLY: u32 = 24;
pub const ETHTOOL_MSG_EEE_NTF: u32 = 25;
pub const ETHTOOL_MSG_TSINFO_GET_REPLY: u32 = 26;
pub const ETHTOOL_MSG_CABLE_TEST_NTF: u32 = 27;
pub const ETHTOOL_MSG_CABLE_TEST_TDR_NTF: u32 = 28;
pub const ETHTOOL_MSG_TUNNEL_INFO_GET_REPLY: u32 = 29;
pub const ETHTOOL_MSG_FEC_GET_REPLY: u32 = 30;
pub const ETHTOOL_MSG_FEC_NTF: u32 = 31;
pub const ETHTOOL_MSG_MODULE_EEPROM_GET_REPLY: u32 = 32;
pub const ETHTOOL_MSG_STATS_GET_REPLY: u32 = 33;
pub const ETHTOOL_MSG_PHC_VCLOCKS_GET_REPLY: u32 = 34;
pub const ETHTOOL_MSG_MODULE_GET_REPLY: u32 = 35;
pub const ETHTOOL_MSG_MODULE_NTF: u32 = 36;
pub const ETHTOOL_MSG_PSE_GET_REPLY: u32 = 37;
pub const ETHTOOL_MSG_RSS_GET_REPLY: u32 = 38;
pub const ETHTOOL_MSG_PLCA_GET_CFG_REPLY: u32 = 39;
pub const ETHTOOL_MSG_PLCA_GET_STATUS_REPLY: u32 = 40;
pub const ETHTOOL_MSG_PLCA_NTF: u32 = 41;
pub const ETHTOOL_MSG_MM_GET_REPLY: u32 = 42;
pub const ETHTOOL_MSG_MM_NTF: u32 = 43;
pub const ETHTOOL_MSG_MODULE_FW_FLASH_NTF: u32 = 44;
pub const ETHTOOL_MSG_PHY_GET_REPLY: u32 = 45;
pub const ETHTOOL_MSG_PHY_NTF: u32 = 46;
pub const ETHTOOL_MSG_TSCONFIG_GET_REPLY: u32 = 47;
pub const ETHTOOL_MSG_TSCONFIG_SET_REPLY: u32 = 48;
pub const ETHTOOL_MSG_PSE_NTF: u32 = 49;
pub const ETHTOOL_MSG_RSS_NTF: u32 = 50;
pub const ETHTOOL_MSG_RSS_CREATE_ACT_REPLY: u32 = 51;
pub const ETHTOOL_MSG_RSS_CREATE_NTF: u32 = 52;
pub const ETHTOOL_MSG_RSS_DELETE_NTF: u32 = 53;
pub const ETHTOOL_MSG_MSE_GET_REPLY: u32 = 54;
pub const __ETHTOOL_MSG_KERNEL_CNT: u32 = 55;
pub const ETHTOOL_MSG_KERNEL_MAX: u32 = __ETHTOOL_MSG_KERNEL_CNT - 1;

// C string-literal macro: the terminating NUL is part of the macro value.
pub const ETHTOOL_MCGRP_MONITOR_NAME: &[u8; 8] = b"monitor\0";
