// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/netfilter/nf_tables.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016277

//! nf_tables UAPI definitions. The C enum tags are preserved as transparent
//! 32-bit integer types; the original global enumerator and macro identifiers
//! remain the fixed numeric values used in netlink payloads.

macro_rules! nft_uapi_enum {
    ($name:ident, $integer:ty) => {
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(pub $integer);
    };
}

// C enum tags, in pinned-source declaration order.  These are transparent
// integer wrappers rather than Rust enums because C permits enum objects to
// carry integer values outside their listed enumerators.
nft_uapi_enum!(nft_registers, u32);
nft_uapi_enum!(nft_verdicts, i32);
nft_uapi_enum!(nf_tables_msg_types, u32);
nft_uapi_enum!(nft_list_attributes, u32);
nft_uapi_enum!(nft_hook_attributes, u32);
nft_uapi_enum!(nft_table_flags, u32);
nft_uapi_enum!(nft_table_attributes, u32);
nft_uapi_enum!(nft_chain_flags, u32);
nft_uapi_enum!(nft_chain_attributes, u32);
nft_uapi_enum!(nft_rule_attributes, u32);
nft_uapi_enum!(nft_rule_compat_flags, u32);
nft_uapi_enum!(nft_rule_compat_attributes, u32);
nft_uapi_enum!(nft_set_flags, u32);
nft_uapi_enum!(nft_set_policies, u32);
nft_uapi_enum!(nft_set_desc_attributes, u32);
nft_uapi_enum!(nft_set_field_attributes, u32);
nft_uapi_enum!(nft_set_attributes, u32);
nft_uapi_enum!(nft_set_elem_flags, u32);
nft_uapi_enum!(nft_set_elem_attributes, u32);
nft_uapi_enum!(nft_set_elem_list_attributes, u32);
nft_uapi_enum!(nft_data_types, u32);
nft_uapi_enum!(nft_data_attributes, u32);
nft_uapi_enum!(nft_verdict_attributes, u32);
nft_uapi_enum!(nft_expr_attributes, u32);
nft_uapi_enum!(nft_immediate_attributes, u32);
nft_uapi_enum!(nft_bitwise_ops, u32);
nft_uapi_enum!(nft_bitwise_attributes, u32);
nft_uapi_enum!(nft_byteorder_ops, u32);
nft_uapi_enum!(nft_byteorder_attributes, u32);
nft_uapi_enum!(nft_cmp_ops, u32);
nft_uapi_enum!(nft_cmp_attributes, u32);
nft_uapi_enum!(nft_range_ops, u32);
nft_uapi_enum!(nft_range_attributes, u32);
nft_uapi_enum!(nft_lookup_flags, u32);
nft_uapi_enum!(nft_lookup_attributes, u32);
nft_uapi_enum!(nft_dynset_ops, u32);
nft_uapi_enum!(nft_dynset_flags, u32);
nft_uapi_enum!(nft_dynset_attributes, u32);
nft_uapi_enum!(nft_payload_bases, u32);
nft_uapi_enum!(nft_payload_csum_types, u32);
nft_uapi_enum!(nft_payload_csum_flags, u32);
nft_uapi_enum!(nft_inner_type, u32);
nft_uapi_enum!(nft_inner_flags, u32);
nft_uapi_enum!(nft_inner_attributes, u32);
nft_uapi_enum!(nft_payload_attributes, u32);
nft_uapi_enum!(nft_exthdr_flags, u32);
nft_uapi_enum!(nft_exthdr_op, u32);
nft_uapi_enum!(nft_exthdr_attributes, u32);
nft_uapi_enum!(nft_meta_keys, u32);
nft_uapi_enum!(nft_rt_keys, u32);
nft_uapi_enum!(nft_hash_types, u32);
nft_uapi_enum!(nft_hash_attributes, u32);
nft_uapi_enum!(nft_meta_attributes, u32);
nft_uapi_enum!(nft_rt_attributes, u32);
nft_uapi_enum!(nft_socket_attributes, u32);
nft_uapi_enum!(nft_socket_keys, u32);
nft_uapi_enum!(nft_ct_keys, u32);
nft_uapi_enum!(nft_ct_attributes, u32);
nft_uapi_enum!(nft_offload_attributes, u32);
nft_uapi_enum!(nft_limit_type, u32);
nft_uapi_enum!(nft_limit_flags, u32);
nft_uapi_enum!(nft_limit_attributes, u32);
nft_uapi_enum!(nft_connlimit_flags, u32);
nft_uapi_enum!(nft_connlimit_attributes, u32);
nft_uapi_enum!(nft_counter_attributes, u32);
nft_uapi_enum!(nft_last_attributes, u32);
nft_uapi_enum!(nft_log_attributes, u32);
nft_uapi_enum!(nft_log_level, u32);
nft_uapi_enum!(nft_queue_attributes, u32);
nft_uapi_enum!(nft_quota_flags, u32);
nft_uapi_enum!(nft_quota_attributes, u32);
nft_uapi_enum!(nft_secmark_attributes, u32);
nft_uapi_enum!(nft_reject_types, u32);
nft_uapi_enum!(nft_reject_inet_code, u32);
nft_uapi_enum!(nft_reject_attributes, u32);
nft_uapi_enum!(nft_nat_types, u32);
nft_uapi_enum!(nft_nat_attributes, u32);
nft_uapi_enum!(nft_tproxy_attributes, u32);
nft_uapi_enum!(nft_masq_attributes, u32);
nft_uapi_enum!(nft_redir_attributes, u32);
nft_uapi_enum!(nft_dup_attributes, u32);
nft_uapi_enum!(nft_fwd_attributes, u32);
nft_uapi_enum!(nft_objref_attributes, u32);
nft_uapi_enum!(nft_gen_attributes, u32);
nft_uapi_enum!(nft_fib_attributes, u32);
nft_uapi_enum!(nft_fib_result, u32);
nft_uapi_enum!(nft_fib_flags, u32);
nft_uapi_enum!(nft_ct_helper_attributes, u32);
nft_uapi_enum!(nft_ct_timeout_timeout_attributes, u32);
nft_uapi_enum!(nft_ct_expectation_attributes, u32);
nft_uapi_enum!(nft_object_attributes, u32);
nft_uapi_enum!(nft_flowtable_flags, u32);
nft_uapi_enum!(nft_flowtable_attributes, u32);
nft_uapi_enum!(nft_flowtable_hook_attributes, u32);
nft_uapi_enum!(nft_osf_attributes, u32);
nft_uapi_enum!(nft_osf_flags, u32);
nft_uapi_enum!(nft_synproxy_attributes, u32);
nft_uapi_enum!(nft_devices_attributes, u32);
nft_uapi_enum!(nft_xfrm_attributes, u32);
nft_uapi_enum!(nft_xfrm_keys, u32);
nft_uapi_enum!(nft_trace_attributes, u32);
nft_uapi_enum!(nft_trace_types, u32);
nft_uapi_enum!(nft_ng_attributes, u32);
nft_uapi_enum!(nft_ng_types, u32);
nft_uapi_enum!(nft_tunnel_key_ip_attributes, u32);
nft_uapi_enum!(nft_tunnel_ip6_attributes, u32);
nft_uapi_enum!(nft_tunnel_opts_attributes, u32);
nft_uapi_enum!(nft_tunnel_opts_vxlan_attributes, u32);
nft_uapi_enum!(nft_tunnel_opts_erspan_attributes, u32);
nft_uapi_enum!(nft_tunnel_opts_geneve_attributes, u32);
nft_uapi_enum!(nft_tunnel_flags, u32);
nft_uapi_enum!(nft_tunnel_key_attributes, u32);
nft_uapi_enum!(nft_tunnel_keys, u32);
nft_uapi_enum!(nft_tunnel_mode, u32);
nft_uapi_enum!(nft_tunnel_attributes, u32);

// enum nft_registers
pub const NFT_REG_VERDICT: u32 = 0;
pub const NFT_REG_1: u32 = 1;
pub const NFT_REG_2: u32 = 2;
pub const NFT_REG_3: u32 = 3;
pub const NFT_REG_4: u32 = 4;
pub const __NFT_REG_MAX: u32 = 5;
pub const NFT_REG32_00: u32 = 8;
pub const NFT_REG32_01: u32 = 9;
pub const NFT_REG32_02: u32 = 10;
pub const NFT_REG32_03: u32 = 11;
pub const NFT_REG32_04: u32 = 12;
pub const NFT_REG32_05: u32 = 13;
pub const NFT_REG32_06: u32 = 14;
pub const NFT_REG32_07: u32 = 15;
pub const NFT_REG32_08: u32 = 16;
pub const NFT_REG32_09: u32 = 17;
pub const NFT_REG32_10: u32 = 18;
pub const NFT_REG32_11: u32 = 19;
pub const NFT_REG32_12: u32 = 20;
pub const NFT_REG32_13: u32 = 21;
pub const NFT_REG32_14: u32 = 22;
pub const NFT_REG32_15: u32 = 23;

// enum nft_verdicts
pub const NFT_CONTINUE: i32 = -1;
pub const NFT_BREAK: i32 = -2;
pub const NFT_JUMP: i32 = -3;
pub const NFT_GOTO: i32 = -4;
pub const NFT_RETURN: i32 = -5;

// enum nf_tables_msg_types
pub const NFT_MSG_NEWTABLE: u32 = 0;
pub const NFT_MSG_GETTABLE: u32 = 1;
pub const NFT_MSG_DELTABLE: u32 = 2;
pub const NFT_MSG_NEWCHAIN: u32 = 3;
pub const NFT_MSG_GETCHAIN: u32 = 4;
pub const NFT_MSG_DELCHAIN: u32 = 5;
pub const NFT_MSG_NEWRULE: u32 = 6;
pub const NFT_MSG_GETRULE: u32 = 7;
pub const NFT_MSG_DELRULE: u32 = 8;
pub const NFT_MSG_NEWSET: u32 = 9;
pub const NFT_MSG_GETSET: u32 = 10;
pub const NFT_MSG_DELSET: u32 = 11;
pub const NFT_MSG_NEWSETELEM: u32 = 12;
pub const NFT_MSG_GETSETELEM: u32 = 13;
pub const NFT_MSG_DELSETELEM: u32 = 14;
pub const NFT_MSG_NEWGEN: u32 = 15;
pub const NFT_MSG_GETGEN: u32 = 16;
pub const NFT_MSG_TRACE: u32 = 17;
pub const NFT_MSG_NEWOBJ: u32 = 18;
pub const NFT_MSG_GETOBJ: u32 = 19;
pub const NFT_MSG_DELOBJ: u32 = 20;
pub const NFT_MSG_GETOBJ_RESET: u32 = 21;
pub const NFT_MSG_NEWFLOWTABLE: u32 = 22;
pub const NFT_MSG_GETFLOWTABLE: u32 = 23;
pub const NFT_MSG_DELFLOWTABLE: u32 = 24;
pub const NFT_MSG_GETRULE_RESET: u32 = 25;
pub const NFT_MSG_DESTROYTABLE: u32 = 26;
pub const NFT_MSG_DESTROYCHAIN: u32 = 27;
pub const NFT_MSG_DESTROYRULE: u32 = 28;
pub const NFT_MSG_DESTROYSET: u32 = 29;
pub const NFT_MSG_DESTROYSETELEM: u32 = 30;
pub const NFT_MSG_DESTROYOBJ: u32 = 31;
pub const NFT_MSG_DESTROYFLOWTABLE: u32 = 32;
pub const NFT_MSG_GETSETELEM_RESET: u32 = 33;
pub const NFT_MSG_MAX: u32 = 34;

// enum nft_list_attributes
pub const NFTA_LIST_UNSPEC: u32 = 0;
pub const NFTA_LIST_ELEM: u32 = 1;
pub const __NFTA_LIST_MAX: u32 = 2;

// enum nft_hook_attributes
pub const NFTA_HOOK_UNSPEC: u32 = 0;
pub const NFTA_HOOK_HOOKNUM: u32 = 1;
pub const NFTA_HOOK_PRIORITY: u32 = 2;
pub const NFTA_HOOK_DEV: u32 = 3;
pub const NFTA_HOOK_DEVS: u32 = 4;
pub const __NFTA_HOOK_MAX: u32 = 5;

// enum nft_table_flags
pub const NFT_TABLE_F_DORMANT: u32 = 1;
pub const NFT_TABLE_F_OWNER: u32 = 2;
pub const NFT_TABLE_F_PERSIST: u32 = 4;

// enum nft_table_attributes
pub const NFTA_TABLE_UNSPEC: u32 = 0;
pub const NFTA_TABLE_NAME: u32 = 1;
pub const NFTA_TABLE_FLAGS: u32 = 2;
pub const NFTA_TABLE_USE: u32 = 3;
pub const NFTA_TABLE_HANDLE: u32 = 4;
pub const NFTA_TABLE_PAD: u32 = 5;
pub const NFTA_TABLE_USERDATA: u32 = 6;
pub const NFTA_TABLE_OWNER: u32 = 7;
pub const __NFTA_TABLE_MAX: u32 = 8;

// enum nft_chain_flags
pub const NFT_CHAIN_BASE: u32 = 1;
pub const NFT_CHAIN_HW_OFFLOAD: u32 = 2;
pub const NFT_CHAIN_BINDING: u32 = 4;

// enum nft_chain_attributes
pub const NFTA_CHAIN_UNSPEC: u32 = 0;
pub const NFTA_CHAIN_TABLE: u32 = 1;
pub const NFTA_CHAIN_HANDLE: u32 = 2;
pub const NFTA_CHAIN_NAME: u32 = 3;
pub const NFTA_CHAIN_HOOK: u32 = 4;
pub const NFTA_CHAIN_POLICY: u32 = 5;
pub const NFTA_CHAIN_USE: u32 = 6;
pub const NFTA_CHAIN_TYPE: u32 = 7;
pub const NFTA_CHAIN_COUNTERS: u32 = 8;
pub const NFTA_CHAIN_PAD: u32 = 9;
pub const NFTA_CHAIN_FLAGS: u32 = 10;
pub const NFTA_CHAIN_ID: u32 = 11;
pub const NFTA_CHAIN_USERDATA: u32 = 12;
pub const __NFTA_CHAIN_MAX: u32 = 13;

// enum nft_rule_attributes
pub const NFTA_RULE_UNSPEC: u32 = 0;
pub const NFTA_RULE_TABLE: u32 = 1;
pub const NFTA_RULE_CHAIN: u32 = 2;
pub const NFTA_RULE_HANDLE: u32 = 3;
pub const NFTA_RULE_EXPRESSIONS: u32 = 4;
pub const NFTA_RULE_COMPAT: u32 = 5;
pub const NFTA_RULE_POSITION: u32 = 6;
pub const NFTA_RULE_USERDATA: u32 = 7;
pub const NFTA_RULE_PAD: u32 = 8;
pub const NFTA_RULE_ID: u32 = 9;
pub const NFTA_RULE_POSITION_ID: u32 = 10;
pub const NFTA_RULE_CHAIN_ID: u32 = 11;
pub const __NFTA_RULE_MAX: u32 = 12;

// enum nft_rule_compat_flags
pub const NFT_RULE_COMPAT_F_UNUSED: u32 = 1;
pub const NFT_RULE_COMPAT_F_INV: u32 = 2;
pub const NFT_RULE_COMPAT_F_MASK: u32 = 2;

// enum nft_rule_compat_attributes
pub const NFTA_RULE_COMPAT_UNSPEC: u32 = 0;
pub const NFTA_RULE_COMPAT_PROTO: u32 = 1;
pub const NFTA_RULE_COMPAT_FLAGS: u32 = 2;
pub const __NFTA_RULE_COMPAT_MAX: u32 = 3;

// enum nft_set_flags
pub const NFT_SET_ANONYMOUS: u32 = 1;
pub const NFT_SET_CONSTANT: u32 = 2;
pub const NFT_SET_INTERVAL: u32 = 4;
pub const NFT_SET_MAP: u32 = 8;
pub const NFT_SET_TIMEOUT: u32 = 16;
pub const NFT_SET_EVAL: u32 = 32;
pub const NFT_SET_OBJECT: u32 = 64;
pub const NFT_SET_CONCAT: u32 = 128;
pub const NFT_SET_EXPR: u32 = 256;

// enum nft_set_policies
pub const NFT_SET_POL_PERFORMANCE: u32 = 0;
pub const NFT_SET_POL_MEMORY: u32 = 1;

// enum nft_set_desc_attributes
pub const NFTA_SET_DESC_UNSPEC: u32 = 0;
pub const NFTA_SET_DESC_SIZE: u32 = 1;
pub const NFTA_SET_DESC_CONCAT: u32 = 2;
pub const __NFTA_SET_DESC_MAX: u32 = 3;

// enum nft_set_field_attributes
pub const NFTA_SET_FIELD_UNSPEC: u32 = 0;
pub const NFTA_SET_FIELD_LEN: u32 = 1;
pub const __NFTA_SET_FIELD_MAX: u32 = 2;

// enum nft_set_attributes
pub const NFTA_SET_UNSPEC: u32 = 0;
pub const NFTA_SET_TABLE: u32 = 1;
pub const NFTA_SET_NAME: u32 = 2;
pub const NFTA_SET_FLAGS: u32 = 3;
pub const NFTA_SET_KEY_TYPE: u32 = 4;
pub const NFTA_SET_KEY_LEN: u32 = 5;
pub const NFTA_SET_DATA_TYPE: u32 = 6;
pub const NFTA_SET_DATA_LEN: u32 = 7;
pub const NFTA_SET_POLICY: u32 = 8;
pub const NFTA_SET_DESC: u32 = 9;
pub const NFTA_SET_ID: u32 = 10;
pub const NFTA_SET_TIMEOUT: u32 = 11;
pub const NFTA_SET_GC_INTERVAL: u32 = 12;
pub const NFTA_SET_USERDATA: u32 = 13;
pub const NFTA_SET_PAD: u32 = 14;
pub const NFTA_SET_OBJ_TYPE: u32 = 15;
pub const NFTA_SET_HANDLE: u32 = 16;
pub const NFTA_SET_EXPR: u32 = 17;
pub const NFTA_SET_EXPRESSIONS: u32 = 18;
pub const NFTA_SET_TYPE: u32 = 19;
pub const NFTA_SET_COUNT: u32 = 20;
pub const __NFTA_SET_MAX: u32 = 21;

// enum nft_set_elem_flags
pub const NFT_SET_ELEM_INTERVAL_END: u32 = 1;
pub const NFT_SET_ELEM_CATCHALL: u32 = 2;

// enum nft_set_elem_attributes
pub const NFTA_SET_ELEM_UNSPEC: u32 = 0;
pub const NFTA_SET_ELEM_KEY: u32 = 1;
pub const NFTA_SET_ELEM_DATA: u32 = 2;
pub const NFTA_SET_ELEM_FLAGS: u32 = 3;
pub const NFTA_SET_ELEM_TIMEOUT: u32 = 4;
pub const NFTA_SET_ELEM_EXPIRATION: u32 = 5;
pub const NFTA_SET_ELEM_USERDATA: u32 = 6;
pub const NFTA_SET_ELEM_EXPR: u32 = 7;
pub const NFTA_SET_ELEM_PAD: u32 = 8;
pub const NFTA_SET_ELEM_OBJREF: u32 = 9;
pub const NFTA_SET_ELEM_KEY_END: u32 = 10;
pub const NFTA_SET_ELEM_EXPRESSIONS: u32 = 11;
pub const __NFTA_SET_ELEM_MAX: u32 = 12;

// enum nft_set_elem_list_attributes
pub const NFTA_SET_ELEM_LIST_UNSPEC: u32 = 0;
pub const NFTA_SET_ELEM_LIST_TABLE: u32 = 1;
pub const NFTA_SET_ELEM_LIST_SET: u32 = 2;
pub const NFTA_SET_ELEM_LIST_ELEMENTS: u32 = 3;
pub const NFTA_SET_ELEM_LIST_SET_ID: u32 = 4;
pub const __NFTA_SET_ELEM_LIST_MAX: u32 = 5;

// enum nft_data_types
pub const NFT_DATA_VALUE: u32 = 0;
pub const NFT_DATA_VERDICT: u32 = 4294967040;

// enum nft_data_attributes
pub const NFTA_DATA_UNSPEC: u32 = 0;
pub const NFTA_DATA_VALUE: u32 = 1;
pub const NFTA_DATA_VERDICT: u32 = 2;
pub const __NFTA_DATA_MAX: u32 = 3;

// enum nft_verdict_attributes
pub const NFTA_VERDICT_UNSPEC: u32 = 0;
pub const NFTA_VERDICT_CODE: u32 = 1;
pub const NFTA_VERDICT_CHAIN: u32 = 2;
pub const NFTA_VERDICT_CHAIN_ID: u32 = 3;
pub const __NFTA_VERDICT_MAX: u32 = 4;

// enum nft_expr_attributes
pub const NFTA_EXPR_UNSPEC: u32 = 0;
pub const NFTA_EXPR_NAME: u32 = 1;
pub const NFTA_EXPR_DATA: u32 = 2;
pub const __NFTA_EXPR_MAX: u32 = 3;

// enum nft_immediate_attributes
pub const NFTA_IMMEDIATE_UNSPEC: u32 = 0;
pub const NFTA_IMMEDIATE_DREG: u32 = 1;
pub const NFTA_IMMEDIATE_DATA: u32 = 2;
pub const __NFTA_IMMEDIATE_MAX: u32 = 3;

// enum nft_bitwise_ops
pub const NFT_BITWISE_MASK_XOR: u32 = 0;
pub const NFT_BITWISE_LSHIFT: u32 = 1;
pub const NFT_BITWISE_RSHIFT: u32 = 2;
pub const NFT_BITWISE_AND: u32 = 3;
pub const NFT_BITWISE_OR: u32 = 4;
pub const NFT_BITWISE_XOR: u32 = 5;

// enum nft_bitwise_attributes
pub const NFTA_BITWISE_UNSPEC: u32 = 0;
pub const NFTA_BITWISE_SREG: u32 = 1;
pub const NFTA_BITWISE_DREG: u32 = 2;
pub const NFTA_BITWISE_LEN: u32 = 3;
pub const NFTA_BITWISE_MASK: u32 = 4;
pub const NFTA_BITWISE_XOR: u32 = 5;
pub const NFTA_BITWISE_OP: u32 = 6;
pub const NFTA_BITWISE_DATA: u32 = 7;
pub const NFTA_BITWISE_SREG2: u32 = 8;
pub const __NFTA_BITWISE_MAX: u32 = 9;

// enum nft_byteorder_ops
pub const NFT_BYTEORDER_NTOH: u32 = 0;
pub const NFT_BYTEORDER_HTON: u32 = 1;

// enum nft_byteorder_attributes
pub const NFTA_BYTEORDER_UNSPEC: u32 = 0;
pub const NFTA_BYTEORDER_SREG: u32 = 1;
pub const NFTA_BYTEORDER_DREG: u32 = 2;
pub const NFTA_BYTEORDER_OP: u32 = 3;
pub const NFTA_BYTEORDER_LEN: u32 = 4;
pub const NFTA_BYTEORDER_SIZE: u32 = 5;
pub const __NFTA_BYTEORDER_MAX: u32 = 6;

// enum nft_cmp_ops
pub const NFT_CMP_EQ: u32 = 0;
pub const NFT_CMP_NEQ: u32 = 1;
pub const NFT_CMP_LT: u32 = 2;
pub const NFT_CMP_LTE: u32 = 3;
pub const NFT_CMP_GT: u32 = 4;
pub const NFT_CMP_GTE: u32 = 5;

// enum nft_cmp_attributes
pub const NFTA_CMP_UNSPEC: u32 = 0;
pub const NFTA_CMP_SREG: u32 = 1;
pub const NFTA_CMP_OP: u32 = 2;
pub const NFTA_CMP_DATA: u32 = 3;
pub const __NFTA_CMP_MAX: u32 = 4;

// enum nft_range_ops
pub const NFT_RANGE_EQ: u32 = 0;
pub const NFT_RANGE_NEQ: u32 = 1;

// enum nft_range_attributes
pub const NFTA_RANGE_UNSPEC: u32 = 0;
pub const NFTA_RANGE_SREG: u32 = 1;
pub const NFTA_RANGE_OP: u32 = 2;
pub const NFTA_RANGE_FROM_DATA: u32 = 3;
pub const NFTA_RANGE_TO_DATA: u32 = 4;
pub const __NFTA_RANGE_MAX: u32 = 5;

// enum nft_lookup_flags
pub const NFT_LOOKUP_F_INV: u32 = 1;

// enum nft_lookup_attributes
pub const NFTA_LOOKUP_UNSPEC: u32 = 0;
pub const NFTA_LOOKUP_SET: u32 = 1;
pub const NFTA_LOOKUP_SREG: u32 = 2;
pub const NFTA_LOOKUP_DREG: u32 = 3;
pub const NFTA_LOOKUP_SET_ID: u32 = 4;
pub const NFTA_LOOKUP_FLAGS: u32 = 5;
pub const __NFTA_LOOKUP_MAX: u32 = 6;

// enum nft_dynset_ops
pub const NFT_DYNSET_OP_ADD: u32 = 0;
pub const NFT_DYNSET_OP_UPDATE: u32 = 1;
pub const NFT_DYNSET_OP_DELETE: u32 = 2;

// enum nft_dynset_flags
pub const NFT_DYNSET_F_INV: u32 = 1;
pub const NFT_DYNSET_F_EXPR: u32 = 2;

// enum nft_dynset_attributes
pub const NFTA_DYNSET_UNSPEC: u32 = 0;
pub const NFTA_DYNSET_SET_NAME: u32 = 1;
pub const NFTA_DYNSET_SET_ID: u32 = 2;
pub const NFTA_DYNSET_OP: u32 = 3;
pub const NFTA_DYNSET_SREG_KEY: u32 = 4;
pub const NFTA_DYNSET_SREG_DATA: u32 = 5;
pub const NFTA_DYNSET_TIMEOUT: u32 = 6;
pub const NFTA_DYNSET_EXPR: u32 = 7;
pub const NFTA_DYNSET_PAD: u32 = 8;
pub const NFTA_DYNSET_FLAGS: u32 = 9;
pub const NFTA_DYNSET_EXPRESSIONS: u32 = 10;
pub const __NFTA_DYNSET_MAX: u32 = 11;

// enum nft_payload_bases
pub const NFT_PAYLOAD_LL_HEADER: u32 = 0;
pub const NFT_PAYLOAD_NETWORK_HEADER: u32 = 1;
pub const NFT_PAYLOAD_TRANSPORT_HEADER: u32 = 2;
pub const NFT_PAYLOAD_INNER_HEADER: u32 = 3;
pub const NFT_PAYLOAD_TUN_HEADER: u32 = 4;

// enum nft_payload_csum_types
pub const NFT_PAYLOAD_CSUM_NONE: u32 = 0;
pub const NFT_PAYLOAD_CSUM_INET: u32 = 1;
pub const NFT_PAYLOAD_CSUM_SCTP: u32 = 2;

// enum nft_payload_csum_flags
pub const NFT_PAYLOAD_L4CSUM_PSEUDOHDR: u32 = 1;

// enum nft_inner_type
pub const NFT_INNER_UNSPEC: u32 = 0;
pub const NFT_INNER_VXLAN: u32 = 1;
pub const NFT_INNER_GENEVE: u32 = 2;

// enum nft_inner_flags
pub const NFT_INNER_HDRSIZE: u32 = 1;
pub const NFT_INNER_LL: u32 = 2;
pub const NFT_INNER_NH: u32 = 4;
pub const NFT_INNER_TH: u32 = 8;

// enum nft_inner_attributes
pub const NFTA_INNER_UNSPEC: u32 = 0;
pub const NFTA_INNER_NUM: u32 = 1;
pub const NFTA_INNER_TYPE: u32 = 2;
pub const NFTA_INNER_FLAGS: u32 = 3;
pub const NFTA_INNER_HDRSIZE: u32 = 4;
pub const NFTA_INNER_EXPR: u32 = 5;
pub const __NFTA_INNER_MAX: u32 = 6;

// enum nft_payload_attributes
pub const NFTA_PAYLOAD_UNSPEC: u32 = 0;
pub const NFTA_PAYLOAD_DREG: u32 = 1;
pub const NFTA_PAYLOAD_BASE: u32 = 2;
pub const NFTA_PAYLOAD_OFFSET: u32 = 3;
pub const NFTA_PAYLOAD_LEN: u32 = 4;
pub const NFTA_PAYLOAD_SREG: u32 = 5;
pub const NFTA_PAYLOAD_CSUM_TYPE: u32 = 6;
pub const NFTA_PAYLOAD_CSUM_OFFSET: u32 = 7;
pub const NFTA_PAYLOAD_CSUM_FLAGS: u32 = 8;
pub const __NFTA_PAYLOAD_MAX: u32 = 9;

// enum nft_exthdr_flags
pub const NFT_EXTHDR_F_PRESENT: u32 = 1;

// enum nft_exthdr_op
pub const NFT_EXTHDR_OP_IPV6: u32 = 0;
pub const NFT_EXTHDR_OP_TCPOPT: u32 = 1;
pub const NFT_EXTHDR_OP_IPV4: u32 = 2;
pub const NFT_EXTHDR_OP_SCTP: u32 = 3;
pub const NFT_EXTHDR_OP_DCCP: u32 = 4;
pub const __NFT_EXTHDR_OP_MAX: u32 = 5;

// enum nft_exthdr_attributes
pub const NFTA_EXTHDR_UNSPEC: u32 = 0;
pub const NFTA_EXTHDR_DREG: u32 = 1;
pub const NFTA_EXTHDR_TYPE: u32 = 2;
pub const NFTA_EXTHDR_OFFSET: u32 = 3;
pub const NFTA_EXTHDR_LEN: u32 = 4;
pub const NFTA_EXTHDR_FLAGS: u32 = 5;
pub const NFTA_EXTHDR_OP: u32 = 6;
pub const NFTA_EXTHDR_SREG: u32 = 7;
pub const __NFTA_EXTHDR_MAX: u32 = 8;

// enum nft_meta_keys
pub const NFT_META_LEN: u32 = 0;
pub const NFT_META_PROTOCOL: u32 = 1;
pub const NFT_META_PRIORITY: u32 = 2;
pub const NFT_META_MARK: u32 = 3;
pub const NFT_META_IIF: u32 = 4;
pub const NFT_META_OIF: u32 = 5;
pub const NFT_META_IIFNAME: u32 = 6;
pub const NFT_META_OIFNAME: u32 = 7;
pub const NFT_META_IFTYPE: u32 = 8;
pub const NFT_META_OIFTYPE: u32 = 9;
pub const NFT_META_SKUID: u32 = 10;
pub const NFT_META_SKGID: u32 = 11;
pub const NFT_META_NFTRACE: u32 = 12;
pub const NFT_META_RTCLASSID: u32 = 13;
pub const NFT_META_SECMARK: u32 = 14;
pub const NFT_META_NFPROTO: u32 = 15;
pub const NFT_META_L4PROTO: u32 = 16;
pub const NFT_META_BRI_IIFNAME: u32 = 17;
pub const NFT_META_BRI_OIFNAME: u32 = 18;
pub const NFT_META_PKTTYPE: u32 = 19;
pub const NFT_META_CPU: u32 = 20;
pub const NFT_META_IIFGROUP: u32 = 21;
pub const NFT_META_OIFGROUP: u32 = 22;
pub const NFT_META_CGROUP: u32 = 23;
pub const NFT_META_PRANDOM: u32 = 24;
pub const NFT_META_SECPATH: u32 = 25;
pub const NFT_META_IIFKIND: u32 = 26;
pub const NFT_META_OIFKIND: u32 = 27;
pub const NFT_META_BRI_IIFPVID: u32 = 28;
pub const NFT_META_BRI_IIFVPROTO: u32 = 29;
pub const NFT_META_TIME_NS: u32 = 30;
pub const NFT_META_TIME_DAY: u32 = 31;
pub const NFT_META_TIME_HOUR: u32 = 32;
pub const NFT_META_SDIF: u32 = 33;
pub const NFT_META_SDIFNAME: u32 = 34;
pub const NFT_META_BRI_BROUTE: u32 = 35;
pub const __NFT_META_IIFTYPE: u32 = 36;
pub const NFT_META_BRI_IIFHWADDR: u32 = 37;

// enum nft_rt_keys
pub const NFT_RT_CLASSID: u32 = 0;
pub const NFT_RT_NEXTHOP4: u32 = 1;
pub const NFT_RT_NEXTHOP6: u32 = 2;
pub const NFT_RT_TCPMSS: u32 = 3;
pub const NFT_RT_XFRM: u32 = 4;
pub const __NFT_RT_MAX: u32 = 5;

// enum nft_hash_types
pub const NFT_HASH_JENKINS: u32 = 0;
pub const NFT_HASH_SYM: u32 = 1;

// enum nft_hash_attributes
pub const NFTA_HASH_UNSPEC: u32 = 0;
pub const NFTA_HASH_SREG: u32 = 1;
pub const NFTA_HASH_DREG: u32 = 2;
pub const NFTA_HASH_LEN: u32 = 3;
pub const NFTA_HASH_MODULUS: u32 = 4;
pub const NFTA_HASH_SEED: u32 = 5;
pub const NFTA_HASH_OFFSET: u32 = 6;
pub const NFTA_HASH_TYPE: u32 = 7;
pub const NFTA_HASH_SET_NAME: u32 = 8;
pub const NFTA_HASH_SET_ID: u32 = 9;
pub const __NFTA_HASH_MAX: u32 = 10;

// enum nft_meta_attributes
pub const NFTA_META_UNSPEC: u32 = 0;
pub const NFTA_META_DREG: u32 = 1;
pub const NFTA_META_KEY: u32 = 2;
pub const NFTA_META_SREG: u32 = 3;
pub const __NFTA_META_MAX: u32 = 4;

// enum nft_rt_attributes
pub const NFTA_RT_UNSPEC: u32 = 0;
pub const NFTA_RT_DREG: u32 = 1;
pub const NFTA_RT_KEY: u32 = 2;
pub const __NFTA_RT_MAX: u32 = 3;

// enum nft_socket_attributes
pub const NFTA_SOCKET_UNSPEC: u32 = 0;
pub const NFTA_SOCKET_KEY: u32 = 1;
pub const NFTA_SOCKET_DREG: u32 = 2;
pub const NFTA_SOCKET_LEVEL: u32 = 3;
pub const __NFTA_SOCKET_MAX: u32 = 4;

// enum nft_socket_keys
pub const NFT_SOCKET_TRANSPARENT: u32 = 0;
pub const NFT_SOCKET_MARK: u32 = 1;
pub const NFT_SOCKET_WILDCARD: u32 = 2;
pub const NFT_SOCKET_CGROUPV2: u32 = 3;
pub const __NFT_SOCKET_MAX: u32 = 4;

// enum nft_ct_keys
pub const NFT_CT_STATE: u32 = 0;
pub const NFT_CT_DIRECTION: u32 = 1;
pub const NFT_CT_STATUS: u32 = 2;
pub const NFT_CT_MARK: u32 = 3;
pub const NFT_CT_SECMARK: u32 = 4;
pub const NFT_CT_EXPIRATION: u32 = 5;
pub const NFT_CT_HELPER: u32 = 6;
pub const NFT_CT_L3PROTOCOL: u32 = 7;
pub const NFT_CT_SRC: u32 = 8;
pub const NFT_CT_DST: u32 = 9;
pub const NFT_CT_PROTOCOL: u32 = 10;
pub const NFT_CT_PROTO_SRC: u32 = 11;
pub const NFT_CT_PROTO_DST: u32 = 12;
pub const NFT_CT_LABELS: u32 = 13;
pub const NFT_CT_PKTS: u32 = 14;
pub const NFT_CT_BYTES: u32 = 15;
pub const NFT_CT_AVGPKT: u32 = 16;
pub const NFT_CT_ZONE: u32 = 17;
pub const NFT_CT_EVENTMASK: u32 = 18;
pub const NFT_CT_SRC_IP: u32 = 19;
pub const NFT_CT_DST_IP: u32 = 20;
pub const NFT_CT_SRC_IP6: u32 = 21;
pub const NFT_CT_DST_IP6: u32 = 22;
pub const NFT_CT_ID: u32 = 23;
pub const __NFT_CT_MAX: u32 = 24;

// enum nft_ct_attributes
pub const NFTA_CT_UNSPEC: u32 = 0;
pub const NFTA_CT_DREG: u32 = 1;
pub const NFTA_CT_KEY: u32 = 2;
pub const NFTA_CT_DIRECTION: u32 = 3;
pub const NFTA_CT_SREG: u32 = 4;
pub const __NFTA_CT_MAX: u32 = 5;

// enum nft_offload_attributes
pub const NFTA_FLOW_UNSPEC: u32 = 0;
pub const NFTA_FLOW_TABLE_NAME: u32 = 1;
pub const __NFTA_FLOW_MAX: u32 = 2;

// enum nft_limit_type
pub const NFT_LIMIT_PKTS: u32 = 0;
pub const NFT_LIMIT_PKT_BYTES: u32 = 1;

// enum nft_limit_flags
pub const NFT_LIMIT_F_INV: u32 = 1;

// enum nft_limit_attributes
pub const NFTA_LIMIT_UNSPEC: u32 = 0;
pub const NFTA_LIMIT_RATE: u32 = 1;
pub const NFTA_LIMIT_UNIT: u32 = 2;
pub const NFTA_LIMIT_BURST: u32 = 3;
pub const NFTA_LIMIT_TYPE: u32 = 4;
pub const NFTA_LIMIT_FLAGS: u32 = 5;
pub const NFTA_LIMIT_PAD: u32 = 6;
pub const __NFTA_LIMIT_MAX: u32 = 7;

// enum nft_connlimit_flags
pub const NFT_CONNLIMIT_F_INV: u32 = 1;

// enum nft_connlimit_attributes
pub const NFTA_CONNLIMIT_UNSPEC: u32 = 0;
pub const NFTA_CONNLIMIT_COUNT: u32 = 1;
pub const NFTA_CONNLIMIT_FLAGS: u32 = 2;
pub const __NFTA_CONNLIMIT_MAX: u32 = 3;

// enum nft_counter_attributes
pub const NFTA_COUNTER_UNSPEC: u32 = 0;
pub const NFTA_COUNTER_BYTES: u32 = 1;
pub const NFTA_COUNTER_PACKETS: u32 = 2;
pub const NFTA_COUNTER_PAD: u32 = 3;
pub const __NFTA_COUNTER_MAX: u32 = 4;

// enum nft_last_attributes
pub const NFTA_LAST_UNSPEC: u32 = 0;
pub const NFTA_LAST_SET: u32 = 1;
pub const NFTA_LAST_MSECS: u32 = 2;
pub const NFTA_LAST_PAD: u32 = 3;
pub const __NFTA_LAST_MAX: u32 = 4;

// enum nft_log_attributes
pub const NFTA_LOG_UNSPEC: u32 = 0;
pub const NFTA_LOG_GROUP: u32 = 1;
pub const NFTA_LOG_PREFIX: u32 = 2;
pub const NFTA_LOG_SNAPLEN: u32 = 3;
pub const NFTA_LOG_QTHRESHOLD: u32 = 4;
pub const NFTA_LOG_LEVEL: u32 = 5;
pub const NFTA_LOG_FLAGS: u32 = 6;
pub const __NFTA_LOG_MAX: u32 = 7;

// enum nft_log_level
pub const NFT_LOGLEVEL_EMERG: u32 = 0;
pub const NFT_LOGLEVEL_ALERT: u32 = 1;
pub const NFT_LOGLEVEL_CRIT: u32 = 2;
pub const NFT_LOGLEVEL_ERR: u32 = 3;
pub const NFT_LOGLEVEL_WARNING: u32 = 4;
pub const NFT_LOGLEVEL_NOTICE: u32 = 5;
pub const NFT_LOGLEVEL_INFO: u32 = 6;
pub const NFT_LOGLEVEL_DEBUG: u32 = 7;
pub const NFT_LOGLEVEL_AUDIT: u32 = 8;
pub const __NFT_LOGLEVEL_MAX: u32 = 9;

// enum nft_queue_attributes
pub const NFTA_QUEUE_UNSPEC: u32 = 0;
pub const NFTA_QUEUE_NUM: u32 = 1;
pub const NFTA_QUEUE_TOTAL: u32 = 2;
pub const NFTA_QUEUE_FLAGS: u32 = 3;
pub const NFTA_QUEUE_SREG_QNUM: u32 = 4;
pub const __NFTA_QUEUE_MAX: u32 = 5;

// enum nft_quota_flags
pub const NFT_QUOTA_F_INV: u32 = 1;
pub const NFT_QUOTA_F_DEPLETED: u32 = 2;

// enum nft_quota_attributes
pub const NFTA_QUOTA_UNSPEC: u32 = 0;
pub const NFTA_QUOTA_BYTES: u32 = 1;
pub const NFTA_QUOTA_FLAGS: u32 = 2;
pub const NFTA_QUOTA_PAD: u32 = 3;
pub const NFTA_QUOTA_CONSUMED: u32 = 4;
pub const __NFTA_QUOTA_MAX: u32 = 5;

// enum nft_secmark_attributes
pub const NFTA_SECMARK_UNSPEC: u32 = 0;
pub const NFTA_SECMARK_CTX: u32 = 1;
pub const __NFTA_SECMARK_MAX: u32 = 2;

// enum nft_reject_types
pub const NFT_REJECT_ICMP_UNREACH: u32 = 0;
pub const NFT_REJECT_TCP_RST: u32 = 1;
pub const NFT_REJECT_ICMPX_UNREACH: u32 = 2;

// enum nft_reject_inet_code
pub const NFT_REJECT_ICMPX_NO_ROUTE: u32 = 0;
pub const NFT_REJECT_ICMPX_PORT_UNREACH: u32 = 1;
pub const NFT_REJECT_ICMPX_HOST_UNREACH: u32 = 2;
pub const NFT_REJECT_ICMPX_ADMIN_PROHIBITED: u32 = 3;
pub const __NFT_REJECT_ICMPX_MAX: u32 = 4;

// enum nft_reject_attributes
pub const NFTA_REJECT_UNSPEC: u32 = 0;
pub const NFTA_REJECT_TYPE: u32 = 1;
pub const NFTA_REJECT_ICMP_CODE: u32 = 2;
pub const __NFTA_REJECT_MAX: u32 = 3;

// enum nft_nat_types
pub const NFT_NAT_SNAT: u32 = 0;
pub const NFT_NAT_DNAT: u32 = 1;

// enum nft_nat_attributes
pub const NFTA_NAT_UNSPEC: u32 = 0;
pub const NFTA_NAT_TYPE: u32 = 1;
pub const NFTA_NAT_FAMILY: u32 = 2;
pub const NFTA_NAT_REG_ADDR_MIN: u32 = 3;
pub const NFTA_NAT_REG_ADDR_MAX: u32 = 4;
pub const NFTA_NAT_REG_PROTO_MIN: u32 = 5;
pub const NFTA_NAT_REG_PROTO_MAX: u32 = 6;
pub const NFTA_NAT_FLAGS: u32 = 7;
pub const __NFTA_NAT_MAX: u32 = 8;

// enum nft_tproxy_attributes
pub const NFTA_TPROXY_UNSPEC: u32 = 0;
pub const NFTA_TPROXY_FAMILY: u32 = 1;
pub const NFTA_TPROXY_REG_ADDR: u32 = 2;
pub const NFTA_TPROXY_REG_PORT: u32 = 3;
pub const __NFTA_TPROXY_MAX: u32 = 4;

// enum nft_masq_attributes
pub const NFTA_MASQ_UNSPEC: u32 = 0;
pub const NFTA_MASQ_FLAGS: u32 = 1;
pub const NFTA_MASQ_REG_PROTO_MIN: u32 = 2;
pub const NFTA_MASQ_REG_PROTO_MAX: u32 = 3;
pub const __NFTA_MASQ_MAX: u32 = 4;

// enum nft_redir_attributes
pub const NFTA_REDIR_UNSPEC: u32 = 0;
pub const NFTA_REDIR_REG_PROTO_MIN: u32 = 1;
pub const NFTA_REDIR_REG_PROTO_MAX: u32 = 2;
pub const NFTA_REDIR_FLAGS: u32 = 3;
pub const __NFTA_REDIR_MAX: u32 = 4;

// enum nft_dup_attributes
pub const NFTA_DUP_UNSPEC: u32 = 0;
pub const NFTA_DUP_SREG_ADDR: u32 = 1;
pub const NFTA_DUP_SREG_DEV: u32 = 2;
pub const __NFTA_DUP_MAX: u32 = 3;

// enum nft_fwd_attributes
pub const NFTA_FWD_UNSPEC: u32 = 0;
pub const NFTA_FWD_SREG_DEV: u32 = 1;
pub const NFTA_FWD_SREG_ADDR: u32 = 2;
pub const NFTA_FWD_NFPROTO: u32 = 3;
pub const __NFTA_FWD_MAX: u32 = 4;

// enum nft_objref_attributes
pub const NFTA_OBJREF_UNSPEC: u32 = 0;
pub const NFTA_OBJREF_IMM_TYPE: u32 = 1;
pub const NFTA_OBJREF_IMM_NAME: u32 = 2;
pub const NFTA_OBJREF_SET_SREG: u32 = 3;
pub const NFTA_OBJREF_SET_NAME: u32 = 4;
pub const NFTA_OBJREF_SET_ID: u32 = 5;
pub const __NFTA_OBJREF_MAX: u32 = 6;

// enum nft_gen_attributes
pub const NFTA_GEN_UNSPEC: u32 = 0;
pub const NFTA_GEN_ID: u32 = 1;
pub const NFTA_GEN_PROC_PID: u32 = 2;
pub const NFTA_GEN_PROC_NAME: u32 = 3;
pub const __NFTA_GEN_MAX: u32 = 4;

// enum nft_fib_attributes
pub const NFTA_FIB_UNSPEC: u32 = 0;
pub const NFTA_FIB_DREG: u32 = 1;
pub const NFTA_FIB_RESULT: u32 = 2;
pub const NFTA_FIB_FLAGS: u32 = 3;
pub const __NFTA_FIB_MAX: u32 = 4;

// enum nft_fib_result
pub const NFT_FIB_RESULT_UNSPEC: u32 = 0;
pub const NFT_FIB_RESULT_OIF: u32 = 1;
pub const NFT_FIB_RESULT_OIFNAME: u32 = 2;
pub const NFT_FIB_RESULT_ADDRTYPE: u32 = 3;
pub const __NFT_FIB_RESULT_MAX: u32 = 4;

// enum nft_fib_flags
pub const NFTA_FIB_F_SADDR: u32 = 1;
pub const NFTA_FIB_F_DADDR: u32 = 2;
pub const NFTA_FIB_F_MARK: u32 = 4;
pub const NFTA_FIB_F_IIF: u32 = 8;
pub const NFTA_FIB_F_OIF: u32 = 16;
pub const NFTA_FIB_F_PRESENT: u32 = 32;

// enum nft_ct_helper_attributes
pub const NFTA_CT_HELPER_UNSPEC: u32 = 0;
pub const NFTA_CT_HELPER_NAME: u32 = 1;
pub const NFTA_CT_HELPER_L3PROTO: u32 = 2;
pub const NFTA_CT_HELPER_L4PROTO: u32 = 3;
pub const __NFTA_CT_HELPER_MAX: u32 = 4;

// enum nft_ct_timeout_timeout_attributes
pub const NFTA_CT_TIMEOUT_UNSPEC: u32 = 0;
pub const NFTA_CT_TIMEOUT_L3PROTO: u32 = 1;
pub const NFTA_CT_TIMEOUT_L4PROTO: u32 = 2;
pub const NFTA_CT_TIMEOUT_DATA: u32 = 3;
pub const __NFTA_CT_TIMEOUT_MAX: u32 = 4;

// enum nft_ct_expectation_attributes
pub const NFTA_CT_EXPECT_UNSPEC: u32 = 0;
pub const NFTA_CT_EXPECT_L3PROTO: u32 = 1;
pub const NFTA_CT_EXPECT_L4PROTO: u32 = 2;
pub const NFTA_CT_EXPECT_DPORT: u32 = 3;
pub const NFTA_CT_EXPECT_TIMEOUT: u32 = 4;
pub const NFTA_CT_EXPECT_SIZE: u32 = 5;
pub const __NFTA_CT_EXPECT_MAX: u32 = 6;

// enum nft_object_attributes
pub const NFTA_OBJ_UNSPEC: u32 = 0;
pub const NFTA_OBJ_TABLE: u32 = 1;
pub const NFTA_OBJ_NAME: u32 = 2;
pub const NFTA_OBJ_TYPE: u32 = 3;
pub const NFTA_OBJ_DATA: u32 = 4;
pub const NFTA_OBJ_USE: u32 = 5;
pub const NFTA_OBJ_HANDLE: u32 = 6;
pub const NFTA_OBJ_PAD: u32 = 7;
pub const NFTA_OBJ_USERDATA: u32 = 8;
pub const __NFTA_OBJ_MAX: u32 = 9;

// enum nft_flowtable_flags
pub const NFT_FLOWTABLE_HW_OFFLOAD: u32 = 1;
pub const NFT_FLOWTABLE_COUNTER: u32 = 2;
pub const NFT_FLOWTABLE_MASK: u32 = 3;

// enum nft_flowtable_attributes
pub const NFTA_FLOWTABLE_UNSPEC: u32 = 0;
pub const NFTA_FLOWTABLE_TABLE: u32 = 1;
pub const NFTA_FLOWTABLE_NAME: u32 = 2;
pub const NFTA_FLOWTABLE_HOOK: u32 = 3;
pub const NFTA_FLOWTABLE_USE: u32 = 4;
pub const NFTA_FLOWTABLE_HANDLE: u32 = 5;
pub const NFTA_FLOWTABLE_PAD: u32 = 6;
pub const NFTA_FLOWTABLE_FLAGS: u32 = 7;
pub const __NFTA_FLOWTABLE_MAX: u32 = 8;

// enum nft_flowtable_hook_attributes
pub const NFTA_FLOWTABLE_HOOK_UNSPEC: u32 = 0;
pub const NFTA_FLOWTABLE_HOOK_NUM: u32 = 1;
pub const NFTA_FLOWTABLE_HOOK_PRIORITY: u32 = 2;
pub const NFTA_FLOWTABLE_HOOK_DEVS: u32 = 3;
pub const __NFTA_FLOWTABLE_HOOK_MAX: u32 = 4;

// enum nft_osf_attributes
pub const NFTA_OSF_UNSPEC: u32 = 0;
pub const NFTA_OSF_DREG: u32 = 1;
pub const NFTA_OSF_TTL: u32 = 2;
pub const NFTA_OSF_FLAGS: u32 = 3;
pub const __NFTA_OSF_MAX: u32 = 4;

// enum nft_osf_flags
pub const NFT_OSF_F_VERSION: u32 = 1;

// enum nft_synproxy_attributes
pub const NFTA_SYNPROXY_UNSPEC: u32 = 0;
pub const NFTA_SYNPROXY_MSS: u32 = 1;
pub const NFTA_SYNPROXY_WSCALE: u32 = 2;
pub const NFTA_SYNPROXY_FLAGS: u32 = 3;
pub const __NFTA_SYNPROXY_MAX: u32 = 4;

// enum nft_devices_attributes
pub const NFTA_DEVICE_UNSPEC: u32 = 0;
pub const NFTA_DEVICE_NAME: u32 = 1;
pub const NFTA_DEVICE_PREFIX: u32 = 2;
pub const __NFTA_DEVICE_MAX: u32 = 3;

// enum nft_xfrm_attributes
pub const NFTA_XFRM_UNSPEC: u32 = 0;
pub const NFTA_XFRM_DREG: u32 = 1;
pub const NFTA_XFRM_KEY: u32 = 2;
pub const NFTA_XFRM_DIR: u32 = 3;
pub const NFTA_XFRM_SPNUM: u32 = 4;
pub const __NFTA_XFRM_MAX: u32 = 5;

// enum nft_xfrm_keys
pub const NFT_XFRM_KEY_UNSPEC: u32 = 0;
pub const NFT_XFRM_KEY_DADDR_IP4: u32 = 1;
pub const NFT_XFRM_KEY_DADDR_IP6: u32 = 2;
pub const NFT_XFRM_KEY_SADDR_IP4: u32 = 3;
pub const NFT_XFRM_KEY_SADDR_IP6: u32 = 4;
pub const NFT_XFRM_KEY_REQID: u32 = 5;
pub const NFT_XFRM_KEY_SPI: u32 = 6;
pub const __NFT_XFRM_KEY_MAX: u32 = 7;

// enum nft_trace_attributes
pub const NFTA_TRACE_UNSPEC: u32 = 0;
pub const NFTA_TRACE_TABLE: u32 = 1;
pub const NFTA_TRACE_CHAIN: u32 = 2;
pub const NFTA_TRACE_RULE_HANDLE: u32 = 3;
pub const NFTA_TRACE_TYPE: u32 = 4;
pub const NFTA_TRACE_VERDICT: u32 = 5;
pub const NFTA_TRACE_ID: u32 = 6;
pub const NFTA_TRACE_LL_HEADER: u32 = 7;
pub const NFTA_TRACE_NETWORK_HEADER: u32 = 8;
pub const NFTA_TRACE_TRANSPORT_HEADER: u32 = 9;
pub const NFTA_TRACE_IIF: u32 = 10;
pub const NFTA_TRACE_IIFTYPE: u32 = 11;
pub const NFTA_TRACE_OIF: u32 = 12;
pub const NFTA_TRACE_OIFTYPE: u32 = 13;
pub const NFTA_TRACE_MARK: u32 = 14;
pub const NFTA_TRACE_NFPROTO: u32 = 15;
pub const NFTA_TRACE_POLICY: u32 = 16;
pub const NFTA_TRACE_PAD: u32 = 17;
pub const NFTA_TRACE_CT_ID: u32 = 18;
pub const NFTA_TRACE_CT_DIRECTION: u32 = 19;
pub const NFTA_TRACE_CT_STATUS: u32 = 20;
pub const NFTA_TRACE_CT_STATE: u32 = 21;
pub const __NFTA_TRACE_MAX: u32 = 22;

// enum nft_trace_types
pub const NFT_TRACETYPE_UNSPEC: u32 = 0;
pub const NFT_TRACETYPE_POLICY: u32 = 1;
pub const NFT_TRACETYPE_RETURN: u32 = 2;
pub const NFT_TRACETYPE_RULE: u32 = 3;
pub const __NFT_TRACETYPE_MAX: u32 = 4;

// enum nft_ng_attributes
pub const NFTA_NG_UNSPEC: u32 = 0;
pub const NFTA_NG_DREG: u32 = 1;
pub const NFTA_NG_MODULUS: u32 = 2;
pub const NFTA_NG_TYPE: u32 = 3;
pub const NFTA_NG_OFFSET: u32 = 4;
pub const NFTA_NG_SET_NAME: u32 = 5;
pub const NFTA_NG_SET_ID: u32 = 6;
pub const __NFTA_NG_MAX: u32 = 7;

// enum nft_ng_types
pub const NFT_NG_INCREMENTAL: u32 = 0;
pub const NFT_NG_RANDOM: u32 = 1;
pub const __NFT_NG_MAX: u32 = 2;

// enum nft_tunnel_key_ip_attributes
pub const NFTA_TUNNEL_KEY_IP_UNSPEC: u32 = 0;
pub const NFTA_TUNNEL_KEY_IP_SRC: u32 = 1;
pub const NFTA_TUNNEL_KEY_IP_DST: u32 = 2;
pub const __NFTA_TUNNEL_KEY_IP_MAX: u32 = 3;

// enum nft_tunnel_ip6_attributes
pub const NFTA_TUNNEL_KEY_IP6_UNSPEC: u32 = 0;
pub const NFTA_TUNNEL_KEY_IP6_SRC: u32 = 1;
pub const NFTA_TUNNEL_KEY_IP6_DST: u32 = 2;
pub const NFTA_TUNNEL_KEY_IP6_FLOWLABEL: u32 = 3;
pub const __NFTA_TUNNEL_KEY_IP6_MAX: u32 = 4;

// enum nft_tunnel_opts_attributes
pub const NFTA_TUNNEL_KEY_OPTS_UNSPEC: u32 = 0;
pub const NFTA_TUNNEL_KEY_OPTS_VXLAN: u32 = 1;
pub const NFTA_TUNNEL_KEY_OPTS_ERSPAN: u32 = 2;
pub const NFTA_TUNNEL_KEY_OPTS_GENEVE: u32 = 3;
pub const __NFTA_TUNNEL_KEY_OPTS_MAX: u32 = 4;

// enum nft_tunnel_opts_vxlan_attributes
pub const NFTA_TUNNEL_KEY_VXLAN_UNSPEC: u32 = 0;
pub const NFTA_TUNNEL_KEY_VXLAN_GBP: u32 = 1;
pub const __NFTA_TUNNEL_KEY_VXLAN_MAX: u32 = 2;

// enum nft_tunnel_opts_erspan_attributes
pub const NFTA_TUNNEL_KEY_ERSPAN_UNSPEC: u32 = 0;
pub const NFTA_TUNNEL_KEY_ERSPAN_VERSION: u32 = 1;
pub const NFTA_TUNNEL_KEY_ERSPAN_V1_INDEX: u32 = 2;
pub const NFTA_TUNNEL_KEY_ERSPAN_V2_HWID: u32 = 3;
pub const NFTA_TUNNEL_KEY_ERSPAN_V2_DIR: u32 = 4;
pub const __NFTA_TUNNEL_KEY_ERSPAN_MAX: u32 = 5;

// enum nft_tunnel_opts_geneve_attributes
pub const NFTA_TUNNEL_KEY_GENEVE_UNSPEC: u32 = 0;
pub const NFTA_TUNNEL_KEY_GENEVE_CLASS: u32 = 1;
pub const NFTA_TUNNEL_KEY_GENEVE_TYPE: u32 = 2;
pub const NFTA_TUNNEL_KEY_GENEVE_DATA: u32 = 3;
pub const __NFTA_TUNNEL_KEY_GENEVE_MAX: u32 = 4;

// enum nft_tunnel_flags
pub const NFT_TUNNEL_F_ZERO_CSUM_TX: u32 = 1;
pub const NFT_TUNNEL_F_DONT_FRAGMENT: u32 = 2;
pub const NFT_TUNNEL_F_SEQ_NUMBER: u32 = 4;

// enum nft_tunnel_key_attributes
pub const NFTA_TUNNEL_KEY_UNSPEC: u32 = 0;
pub const NFTA_TUNNEL_KEY_ID: u32 = 1;
pub const NFTA_TUNNEL_KEY_IP: u32 = 2;
pub const NFTA_TUNNEL_KEY_IP6: u32 = 3;
pub const NFTA_TUNNEL_KEY_FLAGS: u32 = 4;
pub const NFTA_TUNNEL_KEY_TOS: u32 = 5;
pub const NFTA_TUNNEL_KEY_TTL: u32 = 6;
pub const NFTA_TUNNEL_KEY_SPORT: u32 = 7;
pub const NFTA_TUNNEL_KEY_DPORT: u32 = 8;
pub const NFTA_TUNNEL_KEY_OPTS: u32 = 9;
pub const __NFTA_TUNNEL_KEY_MAX: u32 = 10;

// enum nft_tunnel_keys
pub const NFT_TUNNEL_PATH: u32 = 0;
pub const NFT_TUNNEL_ID: u32 = 1;
pub const __NFT_TUNNEL_MAX: u32 = 2;

// enum nft_tunnel_mode
pub const NFT_TUNNEL_MODE_NONE: u32 = 0;
pub const NFT_TUNNEL_MODE_RX: u32 = 1;
pub const NFT_TUNNEL_MODE_TX: u32 = 2;
pub const __NFT_TUNNEL_MODE_MAX: u32 = 3;

// enum nft_tunnel_attributes
pub const NFTA_TUNNEL_UNSPEC: u32 = 0;
pub const NFTA_TUNNEL_KEY: u32 = 1;
pub const NFTA_TUNNEL_DREG: u32 = 2;
pub const NFTA_TUNNEL_MODE: u32 = 3;
pub const __NFTA_TUNNEL_MAX: u32 = 4;

// Object-like macros. `NFT_REG32_MAX` is available only when the Rust build
// configuration carries the source `__KERNEL__` define.
pub const NFT_NAME_MAXLEN: u32 = 256;
pub const NFT_TABLE_MAXNAMELEN: u32 = 256;
pub const NFT_CHAIN_MAXNAMELEN: u32 = 256;
pub const NFT_SET_MAXNAMELEN: u32 = 256;
pub const NFT_OBJ_MAXNAMELEN: u32 = 256;
pub const NFT_USERDATA_MAXLEN: u32 = 256;
pub const NFT_OSF_MAXGENRELEN: u32 = 16;
pub const NFT_REG_MAX: u32 = 4;
#[cfg(feature = "__KERNEL__")]
pub const NFT_REG32_MAX: u32 = 23;
pub const NFT_REG_SIZE: u32 = 16;
pub const NFT_REG32_SIZE: u32 = 4;
pub const NFT_REG32_COUNT: u32 = 16;
pub const NFTA_LIST_MAX: u32 = 1;
pub const NFTA_HOOK_MAX: u32 = 4;
pub const NFT_TABLE_F_MASK: u32 = 7;
pub const NFTA_TABLE_MAX: u32 = 7;
pub const NFT_CHAIN_FLAGS: u32 = 7;
pub const NFTA_CHAIN_MAX: u32 = 12;
pub const NFTA_RULE_MAX: u32 = 11;
pub const NFTA_RULE_COMPAT_MAX: u32 = 2;
pub const NFTA_SET_DESC_MAX: u32 = 2;
pub const NFTA_SET_FIELD_MAX: u32 = 1;
pub const NFTA_SET_MAX: u32 = 20;
pub const NFTA_SET_ELEM_MAX: u32 = 11;
pub const NFTA_SET_ELEM_LIST_MAX: u32 = 4;
pub const NFT_DATA_RESERVED_MASK: u32 = 4294967040;
pub const NFTA_DATA_MAX: u32 = 2;
pub const NFT_DATA_VALUE_MAXLEN: u32 = 64;
pub const NFTA_VERDICT_MAX: u32 = 3;
pub const NFTA_EXPR_MAX: u32 = 2;
pub const NFTA_IMMEDIATE_MAX: u32 = 2;
pub const NFT_BITWISE_BOOL: u32 = 0;
pub const NFTA_BITWISE_MAX: u32 = 8;
pub const NFTA_BYTEORDER_MAX: u32 = 5;
pub const NFTA_CMP_MAX: u32 = 3;
pub const NFTA_RANGE_MAX: u32 = 4;
pub const NFTA_LOOKUP_MAX: u32 = 5;
pub const NFTA_DYNSET_MAX: u32 = 10;
pub const NFTA_INNER_MAX: u32 = 5;
pub const NFTA_PAYLOAD_MAX: u32 = 8;
pub const NFT_EXTHDR_OP_MAX: u32 = 4;
pub const NFTA_EXTHDR_MAX: u32 = 7;
pub const NFT_META_IIFTYPE: u32 = 8;
pub const NFT_RT_MAX: u32 = 4;
pub const NFTA_HASH_MAX: u32 = 9;
pub const NFTA_META_MAX: u32 = 3;
pub const NFTA_RT_MAX: u32 = 2;
pub const NFTA_SOCKET_MAX: u32 = 3;
pub const NFT_SOCKET_MAX: u32 = 3;
pub const NFT_CT_MAX: u32 = 23;
pub const NFTA_CT_MAX: u32 = 4;
pub const NFTA_FLOW_MAX: u32 = 1;
pub const NFTA_LIMIT_MAX: u32 = 6;
pub const NFTA_CONNLIMIT_MAX: u32 = 2;
pub const NFTA_COUNTER_MAX: u32 = 3;
pub const NFTA_LAST_MAX: u32 = 3;
pub const NFTA_LOG_MAX: u32 = 6;
pub const NFT_LOGLEVEL_MAX: u32 = 8;
pub const NFTA_QUEUE_MAX: u32 = 4;
pub const NFT_QUEUE_FLAG_BYPASS: u32 = 1;
pub const NFT_QUEUE_FLAG_CPU_FANOUT: u32 = 2;
pub const NFT_QUEUE_FLAG_MASK: u32 = 3;
pub const NFTA_QUOTA_MAX: u32 = 4;
pub const NFTA_SECMARK_MAX: u32 = 1;
pub const NFT_SECMARK_CTX_MAXLEN: u32 = 4096;
pub const NFT_REJECT_ICMPX_MAX: u32 = 3;
pub const NFTA_REJECT_MAX: u32 = 2;
pub const NFTA_NAT_MAX: u32 = 7;
pub const NFTA_TPROXY_MAX: u32 = 3;
pub const NFTA_MASQ_MAX: u32 = 3;
pub const NFTA_REDIR_MAX: u32 = 3;
pub const NFTA_DUP_MAX: u32 = 2;
pub const NFTA_FWD_MAX: u32 = 3;
pub const NFTA_OBJREF_MAX: u32 = 5;
pub const NFTA_GEN_MAX: u32 = 3;
pub const NFTA_FIB_MAX: u32 = 3;
pub const NFT_FIB_RESULT_MAX: u32 = 3;
pub const NFTA_CT_HELPER_MAX: u32 = 3;
pub const NFTA_CT_TIMEOUT_MAX: u32 = 3;
pub const NFTA_CT_EXPECT_MAX: u32 = 5;
pub const NFT_OBJECT_UNSPEC: u32 = 0;
pub const NFT_OBJECT_COUNTER: u32 = 1;
pub const NFT_OBJECT_QUOTA: u32 = 2;
pub const NFT_OBJECT_CT_HELPER: u32 = 3;
pub const NFT_OBJECT_LIMIT: u32 = 4;
pub const NFT_OBJECT_CONNLIMIT: u32 = 5;
pub const NFT_OBJECT_TUNNEL: u32 = 6;
pub const NFT_OBJECT_CT_TIMEOUT: u32 = 7;
pub const NFT_OBJECT_SECMARK: u32 = 8;
pub const NFT_OBJECT_CT_EXPECT: u32 = 9;
pub const NFT_OBJECT_SYNPROXY: u32 = 10;
pub const __NFT_OBJECT_MAX: u32 = 11;
pub const NFT_OBJECT_MAX: u32 = 10;
pub const NFTA_OBJ_MAX: u32 = 8;
pub const NFTA_FLOWTABLE_MAX: u32 = 7;
pub const NFTA_FLOWTABLE_HOOK_MAX: u32 = 3;
pub const NFTA_OSF_MAX: u32 = 3;
pub const NFTA_SYNPROXY_MAX: u32 = 3;
pub const NFTA_DEVICE_MAX: u32 = 2;
pub const NFTA_XFRM_MAX: u32 = 4;
pub const NFT_XFRM_KEY_MAX: u32 = 6;
pub const NFTA_TRACE_MAX: u32 = 21;
pub const NFT_TRACETYPE_MAX: u32 = 3;
pub const NFTA_NG_MAX: u32 = 6;
pub const NFT_NG_MAX: u32 = 1;
pub const NFTA_TUNNEL_KEY_IP_MAX: u32 = 2;
pub const NFTA_TUNNEL_KEY_IP6_MAX: u32 = 3;
pub const NFTA_TUNNEL_KEY_OPTS_MAX: u32 = 3;
pub const NFTA_TUNNEL_KEY_VXLAN_MAX: u32 = 1;
pub const NFTA_TUNNEL_KEY_ERSPAN_MAX: u32 = 4;
pub const NFTA_TUNNEL_KEY_GENEVE_MAX: u32 = 3;
pub const NFT_TUNNEL_F_MASK: u32 = 7;
pub const NFTA_TUNNEL_KEY_MAX: u32 = 9;
pub const NFT_TUNNEL_MAX: u32 = 1;
pub const NFT_TUNNEL_MODE_MAX: u32 = 2;
pub const NFTA_TUNNEL_MAX: u32 = 3;
pub const NFT_INNER_MASK: u32 = 15;
