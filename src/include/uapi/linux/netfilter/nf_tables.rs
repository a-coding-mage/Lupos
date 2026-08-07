// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/uapi/linux/netfilter/nf_tables.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016277
#![allow(non_camel_case_types, non_upper_case_globals)]

// source include guard: _LINUX_NF_TABLES_H
pub const NFT_NAME_MAXLEN: i32 = 256;
pub const NFT_TABLE_MAXNAMELEN: i32 = NFT_NAME_MAXLEN;
pub const NFT_CHAIN_MAXNAMELEN: i32 = NFT_NAME_MAXLEN;
pub const NFT_SET_MAXNAMELEN: i32 = NFT_NAME_MAXLEN;
pub const NFT_OBJ_MAXNAMELEN: i32 = NFT_NAME_MAXLEN;
pub const NFT_USERDATA_MAXLEN: i32 = 256;
pub const NFT_OSF_MAXGENRELEN: i32 = 16;
pub type nft_registers = i32;
pub const NFT_REG_VERDICT: i32 = 0;
pub const NFT_REG_1: i32 = 1;
pub const NFT_REG_2: i32 = 2;
pub const NFT_REG_3: i32 = 3;
pub const NFT_REG_4: i32 = 4;
pub const __NFT_REG_MAX: i32 = 5;
pub const NFT_REG32_00: i32 = 8;
pub const NFT_REG32_01: i32 = 9;
pub const NFT_REG32_02: i32 = 10;
pub const NFT_REG32_03: i32 = 11;
pub const NFT_REG32_04: i32 = 12;
pub const NFT_REG32_05: i32 = 13;
pub const NFT_REG32_06: i32 = 14;
pub const NFT_REG32_07: i32 = 15;
pub const NFT_REG32_08: i32 = 16;
pub const NFT_REG32_09: i32 = 17;
pub const NFT_REG32_10: i32 = 18;
pub const NFT_REG32_11: i32 = 19;
pub const NFT_REG32_12: i32 = 20;
pub const NFT_REG32_13: i32 = 21;
pub const NFT_REG32_14: i32 = 22;
pub const NFT_REG32_15: i32 = 23;

pub const NFT_REG_MAX: i32 = (__NFT_REG_MAX - 1);
pub const NFT_REG32_MAX: i32 = NFT_REG32_15;
pub const NFT_REG_SIZE: i32 = 16;
pub const NFT_REG32_SIZE: i32 = 4;
pub const NFT_REG32_COUNT: i32 = (NFT_REG32_15 - NFT_REG32_00 + 1);
pub type nft_verdicts = i32;
pub const NFT_CONTINUE: i32 = -1;
pub const NFT_BREAK: i32 = -2;
pub const NFT_JUMP: i32 = -3;
pub const NFT_GOTO: i32 = -4;
pub const NFT_RETURN: i32 = -5;

pub type nf_tables_msg_types = i32;
pub const NFT_MSG_NEWTABLE: i32 = 0;
pub const NFT_MSG_GETTABLE: i32 = 1;
pub const NFT_MSG_DELTABLE: i32 = 2;
pub const NFT_MSG_NEWCHAIN: i32 = 3;
pub const NFT_MSG_GETCHAIN: i32 = 4;
pub const NFT_MSG_DELCHAIN: i32 = 5;
pub const NFT_MSG_NEWRULE: i32 = 6;
pub const NFT_MSG_GETRULE: i32 = 7;
pub const NFT_MSG_DELRULE: i32 = 8;
pub const NFT_MSG_NEWSET: i32 = 9;
pub const NFT_MSG_GETSET: i32 = 10;
pub const NFT_MSG_DELSET: i32 = 11;
pub const NFT_MSG_NEWSETELEM: i32 = 12;
pub const NFT_MSG_GETSETELEM: i32 = 13;
pub const NFT_MSG_DELSETELEM: i32 = 14;
pub const NFT_MSG_NEWGEN: i32 = 15;
pub const NFT_MSG_GETGEN: i32 = 16;
pub const NFT_MSG_TRACE: i32 = 17;
pub const NFT_MSG_NEWOBJ: i32 = 18;
pub const NFT_MSG_GETOBJ: i32 = 19;
pub const NFT_MSG_DELOBJ: i32 = 20;
pub const NFT_MSG_GETOBJ_RESET: i32 = 21;
pub const NFT_MSG_NEWFLOWTABLE: i32 = 22;
pub const NFT_MSG_GETFLOWTABLE: i32 = 23;
pub const NFT_MSG_DELFLOWTABLE: i32 = 24;
pub const NFT_MSG_GETRULE_RESET: i32 = 25;
pub const NFT_MSG_DESTROYTABLE: i32 = 26;
pub const NFT_MSG_DESTROYCHAIN: i32 = 27;
pub const NFT_MSG_DESTROYRULE: i32 = 28;
pub const NFT_MSG_DESTROYSET: i32 = 29;
pub const NFT_MSG_DESTROYSETELEM: i32 = 30;
pub const NFT_MSG_DESTROYOBJ: i32 = 31;
pub const NFT_MSG_DESTROYFLOWTABLE: i32 = 32;
pub const NFT_MSG_GETSETELEM_RESET: i32 = 33;
pub const NFT_MSG_MAX: i32 = 34;

pub type nft_list_attributes = i32;
pub const NFTA_LIST_UNSPEC: i32 = 0;
pub const NFTA_LIST_ELEM: i32 = 1;
pub const __NFTA_LIST_MAX: i32 = 2;

pub const NFTA_LIST_MAX: i32 = (__NFTA_LIST_MAX - 1);
pub type nft_hook_attributes = i32;
pub const NFTA_HOOK_UNSPEC: i32 = 0;
pub const NFTA_HOOK_HOOKNUM: i32 = 1;
pub const NFTA_HOOK_PRIORITY: i32 = 2;
pub const NFTA_HOOK_DEV: i32 = 3;
pub const NFTA_HOOK_DEVS: i32 = 4;
pub const __NFTA_HOOK_MAX: i32 = 5;

pub const NFTA_HOOK_MAX: i32 = (__NFTA_HOOK_MAX - 1);
pub type nft_table_flags = i32;
pub const NFT_TABLE_F_DORMANT: i32 = 1;
pub const NFT_TABLE_F_OWNER: i32 = 2;
pub const NFT_TABLE_F_PERSIST: i32 = 4;

pub const NFT_TABLE_F_MASK: u32 = ((NFT_TABLE_F_DORMANT as u32) |  (NFT_TABLE_F_OWNER as u32) |  (NFT_TABLE_F_PERSIST as u32));
pub type nft_table_attributes = i32;
pub const NFTA_TABLE_UNSPEC: i32 = 0;
pub const NFTA_TABLE_NAME: i32 = 1;
pub const NFTA_TABLE_FLAGS: i32 = 2;
pub const NFTA_TABLE_USE: i32 = 3;
pub const NFTA_TABLE_HANDLE: i32 = 4;
pub const NFTA_TABLE_PAD: i32 = 5;
pub const NFTA_TABLE_USERDATA: i32 = 6;
pub const NFTA_TABLE_OWNER: i32 = 7;
pub const __NFTA_TABLE_MAX: i32 = 8;

pub const NFTA_TABLE_MAX: i32 = (__NFTA_TABLE_MAX - 1);
pub type nft_chain_flags = i32;
pub const NFT_CHAIN_BASE: i32 = 1;
pub const NFT_CHAIN_HW_OFFLOAD: i32 = 2;
pub const NFT_CHAIN_BINDING: i32 = 4;

pub const NFT_CHAIN_FLAGS: u32 = ((NFT_CHAIN_BASE as u32)		|  (NFT_CHAIN_HW_OFFLOAD as u32)	|  (NFT_CHAIN_BINDING as u32));
pub type nft_chain_attributes = i32;
pub const NFTA_CHAIN_UNSPEC: i32 = 0;
pub const NFTA_CHAIN_TABLE: i32 = 1;
pub const NFTA_CHAIN_HANDLE: i32 = 2;
pub const NFTA_CHAIN_NAME: i32 = 3;
pub const NFTA_CHAIN_HOOK: i32 = 4;
pub const NFTA_CHAIN_POLICY: i32 = 5;
pub const NFTA_CHAIN_USE: i32 = 6;
pub const NFTA_CHAIN_TYPE: i32 = 7;
pub const NFTA_CHAIN_COUNTERS: i32 = 8;
pub const NFTA_CHAIN_PAD: i32 = 9;
pub const NFTA_CHAIN_FLAGS: i32 = 10;
pub const NFTA_CHAIN_ID: i32 = 11;
pub const NFTA_CHAIN_USERDATA: i32 = 12;
pub const __NFTA_CHAIN_MAX: i32 = 13;

pub const NFTA_CHAIN_MAX: i32 = (__NFTA_CHAIN_MAX - 1);
pub type nft_rule_attributes = i32;
pub const NFTA_RULE_UNSPEC: i32 = 0;
pub const NFTA_RULE_TABLE: i32 = 1;
pub const NFTA_RULE_CHAIN: i32 = 2;
pub const NFTA_RULE_HANDLE: i32 = 3;
pub const NFTA_RULE_EXPRESSIONS: i32 = 4;
pub const NFTA_RULE_COMPAT: i32 = 5;
pub const NFTA_RULE_POSITION: i32 = 6;
pub const NFTA_RULE_USERDATA: i32 = 7;
pub const NFTA_RULE_PAD: i32 = 8;
pub const NFTA_RULE_ID: i32 = 9;
pub const NFTA_RULE_POSITION_ID: i32 = 10;
pub const NFTA_RULE_CHAIN_ID: i32 = 11;
pub const __NFTA_RULE_MAX: i32 = 12;

pub const NFTA_RULE_MAX: i32 = (__NFTA_RULE_MAX - 1);
pub type nft_rule_compat_flags = i32;
pub const NFT_RULE_COMPAT_F_UNUSED: i32 = 1;
pub const NFT_RULE_COMPAT_F_INV: i32 = 2;
pub const NFT_RULE_COMPAT_F_MASK: i32 = 2;

pub type nft_rule_compat_attributes = i32;
pub const NFTA_RULE_COMPAT_UNSPEC: i32 = 0;
pub const NFTA_RULE_COMPAT_PROTO: i32 = 1;
pub const NFTA_RULE_COMPAT_FLAGS: i32 = 2;
pub const __NFTA_RULE_COMPAT_MAX: i32 = 3;

pub const NFTA_RULE_COMPAT_MAX: i32 = (__NFTA_RULE_COMPAT_MAX - 1);
pub type nft_set_flags = i32;
pub const NFT_SET_ANONYMOUS: i32 = 1;
pub const NFT_SET_CONSTANT: i32 = 2;
pub const NFT_SET_INTERVAL: i32 = 4;
pub const NFT_SET_MAP: i32 = 8;
pub const NFT_SET_TIMEOUT: i32 = 16;
pub const NFT_SET_EVAL: i32 = 32;
pub const NFT_SET_OBJECT: i32 = 64;
pub const NFT_SET_CONCAT: i32 = 128;
pub const NFT_SET_EXPR: i32 = 256;

pub type nft_set_policies = i32;
pub const NFT_SET_POL_PERFORMANCE: i32 = 0;
pub const NFT_SET_POL_MEMORY: i32 = 1;

pub type nft_set_desc_attributes = i32;
pub const NFTA_SET_DESC_UNSPEC: i32 = 0;
pub const NFTA_SET_DESC_SIZE: i32 = 1;
pub const NFTA_SET_DESC_CONCAT: i32 = 2;
pub const __NFTA_SET_DESC_MAX: i32 = 3;

pub const NFTA_SET_DESC_MAX: i32 = (__NFTA_SET_DESC_MAX - 1);
pub type nft_set_field_attributes = i32;
pub const NFTA_SET_FIELD_UNSPEC: i32 = 0;
pub const NFTA_SET_FIELD_LEN: i32 = 1;
pub const __NFTA_SET_FIELD_MAX: i32 = 2;

pub const NFTA_SET_FIELD_MAX: i32 = (__NFTA_SET_FIELD_MAX - 1);
pub type nft_set_attributes = i32;
pub const NFTA_SET_UNSPEC: i32 = 0;
pub const NFTA_SET_TABLE: i32 = 1;
pub const NFTA_SET_NAME: i32 = 2;
pub const NFTA_SET_FLAGS: i32 = 3;
pub const NFTA_SET_KEY_TYPE: i32 = 4;
pub const NFTA_SET_KEY_LEN: i32 = 5;
pub const NFTA_SET_DATA_TYPE: i32 = 6;
pub const NFTA_SET_DATA_LEN: i32 = 7;
pub const NFTA_SET_POLICY: i32 = 8;
pub const NFTA_SET_DESC: i32 = 9;
pub const NFTA_SET_ID: i32 = 10;
pub const NFTA_SET_TIMEOUT: i32 = 11;
pub const NFTA_SET_GC_INTERVAL: i32 = 12;
pub const NFTA_SET_USERDATA: i32 = 13;
pub const NFTA_SET_PAD: i32 = 14;
pub const NFTA_SET_OBJ_TYPE: i32 = 15;
pub const NFTA_SET_HANDLE: i32 = 16;
pub const NFTA_SET_EXPR: i32 = 17;
pub const NFTA_SET_EXPRESSIONS: i32 = 18;
pub const NFTA_SET_TYPE: i32 = 19;
pub const NFTA_SET_COUNT: i32 = 20;
pub const __NFTA_SET_MAX: i32 = 21;

pub const NFTA_SET_MAX: i32 = (__NFTA_SET_MAX - 1);
pub type nft_set_elem_flags = i32;
pub const NFT_SET_ELEM_INTERVAL_END: i32 = 1;
pub const NFT_SET_ELEM_CATCHALL: i32 = 2;

pub type nft_set_elem_attributes = i32;
pub const NFTA_SET_ELEM_UNSPEC: i32 = 0;
pub const NFTA_SET_ELEM_KEY: i32 = 1;
pub const NFTA_SET_ELEM_DATA: i32 = 2;
pub const NFTA_SET_ELEM_FLAGS: i32 = 3;
pub const NFTA_SET_ELEM_TIMEOUT: i32 = 4;
pub const NFTA_SET_ELEM_EXPIRATION: i32 = 5;
pub const NFTA_SET_ELEM_USERDATA: i32 = 6;
pub const NFTA_SET_ELEM_EXPR: i32 = 7;
pub const NFTA_SET_ELEM_PAD: i32 = 8;
pub const NFTA_SET_ELEM_OBJREF: i32 = 9;
pub const NFTA_SET_ELEM_KEY_END: i32 = 10;
pub const NFTA_SET_ELEM_EXPRESSIONS: i32 = 11;
pub const __NFTA_SET_ELEM_MAX: i32 = 12;

pub const NFTA_SET_ELEM_MAX: i32 = (__NFTA_SET_ELEM_MAX - 1);
pub type nft_set_elem_list_attributes = i32;
pub const NFTA_SET_ELEM_LIST_UNSPEC: i32 = 0;
pub const NFTA_SET_ELEM_LIST_TABLE: i32 = 1;
pub const NFTA_SET_ELEM_LIST_SET: i32 = 2;
pub const NFTA_SET_ELEM_LIST_ELEMENTS: i32 = 3;
pub const NFTA_SET_ELEM_LIST_SET_ID: i32 = 4;
pub const __NFTA_SET_ELEM_LIST_MAX: i32 = 5;

pub const NFTA_SET_ELEM_LIST_MAX: i32 = (__NFTA_SET_ELEM_LIST_MAX - 1);
pub type nft_data_types = i32;
pub const NFT_DATA_VALUE: i32 = 0;
pub const NFT_DATA_VERDICT: i32 = 4294967040;

pub const NFT_DATA_RESERVED_MASK: u32 = 0xffffff00;
pub type nft_data_attributes = i32;
pub const NFTA_DATA_UNSPEC: i32 = 0;
pub const NFTA_DATA_VALUE: i32 = 1;
pub const NFTA_DATA_VERDICT: i32 = 2;
pub const __NFTA_DATA_MAX: i32 = 3;

pub const NFTA_DATA_MAX: i32 = (__NFTA_DATA_MAX - 1);
pub const NFT_DATA_VALUE_MAXLEN: i32 = 64;
pub type nft_verdict_attributes = i32;
pub const NFTA_VERDICT_UNSPEC: i32 = 0;
pub const NFTA_VERDICT_CODE: i32 = 1;
pub const NFTA_VERDICT_CHAIN: i32 = 2;
pub const NFTA_VERDICT_CHAIN_ID: i32 = 3;
pub const __NFTA_VERDICT_MAX: i32 = 4;

pub const NFTA_VERDICT_MAX: i32 = (__NFTA_VERDICT_MAX - 1);
pub type nft_expr_attributes = i32;
pub const NFTA_EXPR_UNSPEC: i32 = 0;
pub const NFTA_EXPR_NAME: i32 = 1;
pub const NFTA_EXPR_DATA: i32 = 2;
pub const __NFTA_EXPR_MAX: i32 = 3;

pub const NFTA_EXPR_MAX: i32 = (__NFTA_EXPR_MAX - 1);
pub type nft_immediate_attributes = i32;
pub const NFTA_IMMEDIATE_UNSPEC: i32 = 0;
pub const NFTA_IMMEDIATE_DREG: i32 = 1;
pub const NFTA_IMMEDIATE_DATA: i32 = 2;
pub const __NFTA_IMMEDIATE_MAX: i32 = 3;

pub const NFTA_IMMEDIATE_MAX: i32 = (__NFTA_IMMEDIATE_MAX - 1);
pub type nft_bitwise_ops = i32;
pub const NFT_BITWISE_MASK_XOR: i32 = 0;
pub const NFT_BITWISE_LSHIFT: i32 = 1;
pub const NFT_BITWISE_RSHIFT: i32 = 2;
pub const NFT_BITWISE_AND: i32 = 3;
pub const NFT_BITWISE_OR: i32 = 4;
pub const NFT_BITWISE_XOR: i32 = 5;

pub const NFT_BITWISE_BOOL: i32 = NFT_BITWISE_MASK_XOR;
pub type nft_bitwise_attributes = i32;
pub const NFTA_BITWISE_UNSPEC: i32 = 0;
pub const NFTA_BITWISE_SREG: i32 = 1;
pub const NFTA_BITWISE_DREG: i32 = 2;
pub const NFTA_BITWISE_LEN: i32 = 3;
pub const NFTA_BITWISE_MASK: i32 = 4;
pub const NFTA_BITWISE_XOR: i32 = 5;
pub const NFTA_BITWISE_OP: i32 = 6;
pub const NFTA_BITWISE_DATA: i32 = 7;
pub const NFTA_BITWISE_SREG2: i32 = 8;
pub const __NFTA_BITWISE_MAX: i32 = 9;

pub const NFTA_BITWISE_MAX: i32 = (__NFTA_BITWISE_MAX - 1);
pub type nft_byteorder_ops = i32;
pub const NFT_BYTEORDER_NTOH: i32 = 0;
pub const NFT_BYTEORDER_HTON: i32 = 1;

pub type nft_byteorder_attributes = i32;
pub const NFTA_BYTEORDER_UNSPEC: i32 = 0;
pub const NFTA_BYTEORDER_SREG: i32 = 1;
pub const NFTA_BYTEORDER_DREG: i32 = 2;
pub const NFTA_BYTEORDER_OP: i32 = 3;
pub const NFTA_BYTEORDER_LEN: i32 = 4;
pub const NFTA_BYTEORDER_SIZE: i32 = 5;
pub const __NFTA_BYTEORDER_MAX: i32 = 6;

pub const NFTA_BYTEORDER_MAX: i32 = (__NFTA_BYTEORDER_MAX - 1);
pub type nft_cmp_ops = i32;
pub const NFT_CMP_EQ: i32 = 0;
pub const NFT_CMP_NEQ: i32 = 1;
pub const NFT_CMP_LT: i32 = 2;
pub const NFT_CMP_LTE: i32 = 3;
pub const NFT_CMP_GT: i32 = 4;
pub const NFT_CMP_GTE: i32 = 5;

pub type nft_cmp_attributes = i32;
pub const NFTA_CMP_UNSPEC: i32 = 0;
pub const NFTA_CMP_SREG: i32 = 1;
pub const NFTA_CMP_OP: i32 = 2;
pub const NFTA_CMP_DATA: i32 = 3;
pub const __NFTA_CMP_MAX: i32 = 4;

pub const NFTA_CMP_MAX: i32 = (__NFTA_CMP_MAX - 1);
pub type nft_range_ops = i32;
pub const NFT_RANGE_EQ: i32 = 0;
pub const NFT_RANGE_NEQ: i32 = 1;

pub type nft_range_attributes = i32;
pub const NFTA_RANGE_UNSPEC: i32 = 0;
pub const NFTA_RANGE_SREG: i32 = 1;
pub const NFTA_RANGE_OP: i32 = 2;
pub const NFTA_RANGE_FROM_DATA: i32 = 3;
pub const NFTA_RANGE_TO_DATA: i32 = 4;
pub const __NFTA_RANGE_MAX: i32 = 5;

pub const NFTA_RANGE_MAX: i32 = (__NFTA_RANGE_MAX - 1);
pub type nft_lookup_flags = i32;
pub const NFT_LOOKUP_F_INV: i32 = 1;

pub type nft_lookup_attributes = i32;
pub const NFTA_LOOKUP_UNSPEC: i32 = 0;
pub const NFTA_LOOKUP_SET: i32 = 1;
pub const NFTA_LOOKUP_SREG: i32 = 2;
pub const NFTA_LOOKUP_DREG: i32 = 3;
pub const NFTA_LOOKUP_SET_ID: i32 = 4;
pub const NFTA_LOOKUP_FLAGS: i32 = 5;
pub const __NFTA_LOOKUP_MAX: i32 = 6;

pub const NFTA_LOOKUP_MAX: i32 = (__NFTA_LOOKUP_MAX - 1);
pub type nft_dynset_ops = i32;
pub const NFT_DYNSET_OP_ADD: i32 = 0;
pub const NFT_DYNSET_OP_UPDATE: i32 = 1;
pub const NFT_DYNSET_OP_DELETE: i32 = 2;

pub type nft_dynset_flags = i32;
pub const NFT_DYNSET_F_INV: i32 = 1;
pub const NFT_DYNSET_F_EXPR: i32 = 2;

pub type nft_dynset_attributes = i32;
pub const NFTA_DYNSET_UNSPEC: i32 = 0;
pub const NFTA_DYNSET_SET_NAME: i32 = 1;
pub const NFTA_DYNSET_SET_ID: i32 = 2;
pub const NFTA_DYNSET_OP: i32 = 3;
pub const NFTA_DYNSET_SREG_KEY: i32 = 4;
pub const NFTA_DYNSET_SREG_DATA: i32 = 5;
pub const NFTA_DYNSET_TIMEOUT: i32 = 6;
pub const NFTA_DYNSET_EXPR: i32 = 7;
pub const NFTA_DYNSET_PAD: i32 = 8;
pub const NFTA_DYNSET_FLAGS: i32 = 9;
pub const NFTA_DYNSET_EXPRESSIONS: i32 = 10;
pub const __NFTA_DYNSET_MAX: i32 = 11;

pub const NFTA_DYNSET_MAX: i32 = (__NFTA_DYNSET_MAX - 1);
pub type nft_payload_bases = i32;
pub const NFT_PAYLOAD_LL_HEADER: i32 = 0;
pub const NFT_PAYLOAD_NETWORK_HEADER: i32 = 1;
pub const NFT_PAYLOAD_TRANSPORT_HEADER: i32 = 2;
pub const NFT_PAYLOAD_INNER_HEADER: i32 = 3;
pub const NFT_PAYLOAD_TUN_HEADER: i32 = 4;

pub type nft_payload_csum_types = i32;
pub const NFT_PAYLOAD_CSUM_NONE: i32 = 0;
pub const NFT_PAYLOAD_CSUM_INET: i32 = 1;
pub const NFT_PAYLOAD_CSUM_SCTP: i32 = 2;

pub type nft_payload_csum_flags = i32;
pub const NFT_PAYLOAD_L4CSUM_PSEUDOHDR: i32 = 1;

pub type nft_inner_type = i32;
pub const NFT_INNER_UNSPEC: i32 = 0;
pub const NFT_INNER_VXLAN: i32 = 1;
pub const NFT_INNER_GENEVE: i32 = 2;

pub type nft_inner_flags = i32;
pub const NFT_INNER_HDRSIZE: i32 = 1;
pub const NFT_INNER_LL: i32 = 2;
pub const NFT_INNER_NH: i32 = 4;
pub const NFT_INNER_TH: i32 = 8;

pub const NFT_INNER_MASK: u32 = ((NFT_INNER_HDRSIZE as u32) | (NFT_INNER_LL as u32) |  (NFT_INNER_NH as u32) | (NFT_INNER_TH as u32));
pub type nft_inner_attributes = i32;
pub const NFTA_INNER_UNSPEC: i32 = 0;
pub const NFTA_INNER_NUM: i32 = 1;
pub const NFTA_INNER_TYPE: i32 = 2;
pub const NFTA_INNER_FLAGS: i32 = 3;
pub const NFTA_INNER_HDRSIZE: i32 = 4;
pub const NFTA_INNER_EXPR: i32 = 5;
pub const __NFTA_INNER_MAX: i32 = 6;

pub const NFTA_INNER_MAX: i32 = (__NFTA_INNER_MAX - 1);
pub type nft_payload_attributes = i32;
pub const NFTA_PAYLOAD_UNSPEC: i32 = 0;
pub const NFTA_PAYLOAD_DREG: i32 = 1;
pub const NFTA_PAYLOAD_BASE: i32 = 2;
pub const NFTA_PAYLOAD_OFFSET: i32 = 3;
pub const NFTA_PAYLOAD_LEN: i32 = 4;
pub const NFTA_PAYLOAD_SREG: i32 = 5;
pub const NFTA_PAYLOAD_CSUM_TYPE: i32 = 6;
pub const NFTA_PAYLOAD_CSUM_OFFSET: i32 = 7;
pub const NFTA_PAYLOAD_CSUM_FLAGS: i32 = 8;
pub const __NFTA_PAYLOAD_MAX: i32 = 9;

pub const NFTA_PAYLOAD_MAX: i32 = (__NFTA_PAYLOAD_MAX - 1);
pub type nft_exthdr_flags = i32;
pub const NFT_EXTHDR_F_PRESENT: i32 = 1;

pub type nft_exthdr_op = i32;
pub const NFT_EXTHDR_OP_IPV6: i32 = 0;
pub const NFT_EXTHDR_OP_TCPOPT: i32 = 1;
pub const NFT_EXTHDR_OP_IPV4: i32 = 2;
pub const NFT_EXTHDR_OP_SCTP: i32 = 3;
pub const NFT_EXTHDR_OP_DCCP: i32 = 4;
pub const __NFT_EXTHDR_OP_MAX: i32 = 5;

pub const NFT_EXTHDR_OP_MAX: i32 = (__NFT_EXTHDR_OP_MAX - 1);
pub type nft_exthdr_attributes = i32;
pub const NFTA_EXTHDR_UNSPEC: i32 = 0;
pub const NFTA_EXTHDR_DREG: i32 = 1;
pub const NFTA_EXTHDR_TYPE: i32 = 2;
pub const NFTA_EXTHDR_OFFSET: i32 = 3;
pub const NFTA_EXTHDR_LEN: i32 = 4;
pub const NFTA_EXTHDR_FLAGS: i32 = 5;
pub const NFTA_EXTHDR_OP: i32 = 6;
pub const NFTA_EXTHDR_SREG: i32 = 7;
pub const __NFTA_EXTHDR_MAX: i32 = 8;

pub const NFTA_EXTHDR_MAX: i32 = (__NFTA_EXTHDR_MAX - 1);
pub type nft_meta_keys = i32;
pub const NFT_META_LEN: i32 = 0;
pub const NFT_META_PROTOCOL: i32 = 1;
pub const NFT_META_PRIORITY: i32 = 2;
pub const NFT_META_MARK: i32 = 3;
pub const NFT_META_IIF: i32 = 4;
pub const NFT_META_OIF: i32 = 5;
pub const NFT_META_IIFNAME: i32 = 6;
pub const NFT_META_OIFNAME: i32 = 7;
pub const NFT_META_IFTYPE: i32 = 8;
pub const NFT_META_IIFTYPE: i32 = NFT_META_IFTYPE;
pub const NFT_META_OIFTYPE: i32 = 9;
pub const NFT_META_SKUID: i32 = 10;
pub const NFT_META_SKGID: i32 = 11;
pub const NFT_META_NFTRACE: i32 = 12;
pub const NFT_META_RTCLASSID: i32 = 13;
pub const NFT_META_SECMARK: i32 = 14;
pub const NFT_META_NFPROTO: i32 = 15;
pub const NFT_META_L4PROTO: i32 = 16;
pub const NFT_META_BRI_IIFNAME: i32 = 17;
pub const NFT_META_BRI_OIFNAME: i32 = 18;
pub const NFT_META_PKTTYPE: i32 = 19;
pub const NFT_META_CPU: i32 = 20;
pub const NFT_META_IIFGROUP: i32 = 21;
pub const NFT_META_OIFGROUP: i32 = 22;
pub const NFT_META_CGROUP: i32 = 23;
pub const NFT_META_PRANDOM: i32 = 24;
pub const NFT_META_SECPATH: i32 = 25;
pub const NFT_META_IIFKIND: i32 = 26;
pub const NFT_META_OIFKIND: i32 = 27;
pub const NFT_META_BRI_IIFPVID: i32 = 28;
pub const NFT_META_BRI_IIFVPROTO: i32 = 29;
pub const NFT_META_TIME_NS: i32 = 30;
pub const NFT_META_TIME_DAY: i32 = 31;
pub const NFT_META_TIME_HOUR: i32 = 32;
pub const NFT_META_SDIF: i32 = 33;
pub const NFT_META_SDIFNAME: i32 = 34;
pub const NFT_META_BRI_BROUTE: i32 = 35;
pub const __NFT_META_IIFTYPE: i32 = 36;
pub const NFT_META_BRI_IIFHWADDR: i32 = 37;

pub type nft_rt_keys = i32;
pub const NFT_RT_CLASSID: i32 = 0;
pub const NFT_RT_NEXTHOP4: i32 = 1;
pub const NFT_RT_NEXTHOP6: i32 = 2;
pub const NFT_RT_TCPMSS: i32 = 3;
pub const NFT_RT_XFRM: i32 = 4;
pub const __NFT_RT_MAX: i32 = 5;

pub const NFT_RT_MAX: i32 = (__NFT_RT_MAX - 1);
pub type nft_hash_types = i32;
pub const NFT_HASH_JENKINS: i32 = 0;
pub const NFT_HASH_SYM: i32 = 1;

pub type nft_hash_attributes = i32;
pub const NFTA_HASH_UNSPEC: i32 = 0;
pub const NFTA_HASH_SREG: i32 = 1;
pub const NFTA_HASH_DREG: i32 = 2;
pub const NFTA_HASH_LEN: i32 = 3;
pub const NFTA_HASH_MODULUS: i32 = 4;
pub const NFTA_HASH_SEED: i32 = 5;
pub const NFTA_HASH_OFFSET: i32 = 6;
pub const NFTA_HASH_TYPE: i32 = 7;
pub const NFTA_HASH_SET_NAME: i32 = 8;
pub const NFTA_HASH_SET_ID: i32 = 9;
pub const __NFTA_HASH_MAX: i32 = 10;

pub const NFTA_HASH_MAX: i32 = (__NFTA_HASH_MAX - 1);
pub type nft_meta_attributes = i32;
pub const NFTA_META_UNSPEC: i32 = 0;
pub const NFTA_META_DREG: i32 = 1;
pub const NFTA_META_KEY: i32 = 2;
pub const NFTA_META_SREG: i32 = 3;
pub const __NFTA_META_MAX: i32 = 4;

pub const NFTA_META_MAX: i32 = (__NFTA_META_MAX - 1);
pub type nft_rt_attributes = i32;
pub const NFTA_RT_UNSPEC: i32 = 0;
pub const NFTA_RT_DREG: i32 = 1;
pub const NFTA_RT_KEY: i32 = 2;
pub const __NFTA_RT_MAX: i32 = 3;

pub const NFTA_RT_MAX: i32 = (__NFTA_RT_MAX - 1);
pub type nft_socket_attributes = i32;
pub const NFTA_SOCKET_UNSPEC: i32 = 0;
pub const NFTA_SOCKET_KEY: i32 = 1;
pub const NFTA_SOCKET_DREG: i32 = 2;
pub const NFTA_SOCKET_LEVEL: i32 = 3;
pub const __NFTA_SOCKET_MAX: i32 = 4;

pub const NFTA_SOCKET_MAX: i32 = (__NFTA_SOCKET_MAX - 1);
pub type nft_socket_keys = i32;
pub const NFT_SOCKET_TRANSPARENT: i32 = 0;
pub const NFT_SOCKET_MARK: i32 = 1;
pub const NFT_SOCKET_WILDCARD: i32 = 2;
pub const NFT_SOCKET_CGROUPV2: i32 = 3;
pub const __NFT_SOCKET_MAX: i32 = 4;

pub const NFT_SOCKET_MAX: i32 = (__NFT_SOCKET_MAX - 1);
pub type nft_ct_keys = i32;
pub const NFT_CT_STATE: i32 = 0;
pub const NFT_CT_DIRECTION: i32 = 1;
pub const NFT_CT_STATUS: i32 = 2;
pub const NFT_CT_MARK: i32 = 3;
pub const NFT_CT_SECMARK: i32 = 4;
pub const NFT_CT_EXPIRATION: i32 = 5;
pub const NFT_CT_HELPER: i32 = 6;
pub const NFT_CT_L3PROTOCOL: i32 = 7;
pub const NFT_CT_SRC: i32 = 8;
pub const NFT_CT_DST: i32 = 9;
pub const NFT_CT_PROTOCOL: i32 = 10;
pub const NFT_CT_PROTO_SRC: i32 = 11;
pub const NFT_CT_PROTO_DST: i32 = 12;
pub const NFT_CT_LABELS: i32 = 13;
pub const NFT_CT_PKTS: i32 = 14;
pub const NFT_CT_BYTES: i32 = 15;
pub const NFT_CT_AVGPKT: i32 = 16;
pub const NFT_CT_ZONE: i32 = 17;
pub const NFT_CT_EVENTMASK: i32 = 18;
pub const NFT_CT_SRC_IP: i32 = 19;
pub const NFT_CT_DST_IP: i32 = 20;
pub const NFT_CT_SRC_IP6: i32 = 21;
pub const NFT_CT_DST_IP6: i32 = 22;
pub const NFT_CT_ID: i32 = 23;
pub const __NFT_CT_MAX: i32 = 24;

pub const NFT_CT_MAX: i32 = (__NFT_CT_MAX - 1);
pub type nft_ct_attributes = i32;
pub const NFTA_CT_UNSPEC: i32 = 0;
pub const NFTA_CT_DREG: i32 = 1;
pub const NFTA_CT_KEY: i32 = 2;
pub const NFTA_CT_DIRECTION: i32 = 3;
pub const NFTA_CT_SREG: i32 = 4;
pub const __NFTA_CT_MAX: i32 = 5;

pub const NFTA_CT_MAX: i32 = (__NFTA_CT_MAX - 1);
pub type nft_offload_attributes = i32;
pub const NFTA_FLOW_UNSPEC: i32 = 0;
pub const NFTA_FLOW_TABLE_NAME: i32 = 1;
pub const __NFTA_FLOW_MAX: i32 = 2;

pub const NFTA_FLOW_MAX: i32 = (__NFTA_FLOW_MAX - 1);
pub type nft_limit_type = i32;
pub const NFT_LIMIT_PKTS: i32 = 0;
pub const NFT_LIMIT_PKT_BYTES: i32 = 1;

pub type nft_limit_flags = i32;
pub const NFT_LIMIT_F_INV: i32 = 1;

pub type nft_limit_attributes = i32;
pub const NFTA_LIMIT_UNSPEC: i32 = 0;
pub const NFTA_LIMIT_RATE: i32 = 1;
pub const NFTA_LIMIT_UNIT: i32 = 2;
pub const NFTA_LIMIT_BURST: i32 = 3;
pub const NFTA_LIMIT_TYPE: i32 = 4;
pub const NFTA_LIMIT_FLAGS: i32 = 5;
pub const NFTA_LIMIT_PAD: i32 = 6;
pub const __NFTA_LIMIT_MAX: i32 = 7;

pub const NFTA_LIMIT_MAX: i32 = (__NFTA_LIMIT_MAX - 1);
pub type nft_connlimit_flags = i32;
pub const NFT_CONNLIMIT_F_INV: i32 = 1;

pub type nft_connlimit_attributes = i32;
pub const NFTA_CONNLIMIT_UNSPEC: i32 = 0;
pub const NFTA_CONNLIMIT_COUNT: i32 = 1;
pub const NFTA_CONNLIMIT_FLAGS: i32 = 2;
pub const __NFTA_CONNLIMIT_MAX: i32 = 3;

pub const NFTA_CONNLIMIT_MAX: i32 = (__NFTA_CONNLIMIT_MAX - 1);
pub type nft_counter_attributes = i32;
pub const NFTA_COUNTER_UNSPEC: i32 = 0;
pub const NFTA_COUNTER_BYTES: i32 = 1;
pub const NFTA_COUNTER_PACKETS: i32 = 2;
pub const NFTA_COUNTER_PAD: i32 = 3;
pub const __NFTA_COUNTER_MAX: i32 = 4;

pub const NFTA_COUNTER_MAX: i32 = (__NFTA_COUNTER_MAX - 1);
pub type nft_last_attributes = i32;
pub const NFTA_LAST_UNSPEC: i32 = 0;
pub const NFTA_LAST_SET: i32 = 1;
pub const NFTA_LAST_MSECS: i32 = 2;
pub const NFTA_LAST_PAD: i32 = 3;
pub const __NFTA_LAST_MAX: i32 = 4;

pub const NFTA_LAST_MAX: i32 = (__NFTA_LAST_MAX - 1);
pub type nft_log_attributes = i32;
pub const NFTA_LOG_UNSPEC: i32 = 0;
pub const NFTA_LOG_GROUP: i32 = 1;
pub const NFTA_LOG_PREFIX: i32 = 2;
pub const NFTA_LOG_SNAPLEN: i32 = 3;
pub const NFTA_LOG_QTHRESHOLD: i32 = 4;
pub const NFTA_LOG_LEVEL: i32 = 5;
pub const NFTA_LOG_FLAGS: i32 = 6;
pub const __NFTA_LOG_MAX: i32 = 7;

pub const NFTA_LOG_MAX: i32 = (__NFTA_LOG_MAX - 1);
pub type nft_log_level = i32;
pub const NFT_LOGLEVEL_EMERG: i32 = 0;
pub const NFT_LOGLEVEL_ALERT: i32 = 1;
pub const NFT_LOGLEVEL_CRIT: i32 = 2;
pub const NFT_LOGLEVEL_ERR: i32 = 3;
pub const NFT_LOGLEVEL_WARNING: i32 = 4;
pub const NFT_LOGLEVEL_NOTICE: i32 = 5;
pub const NFT_LOGLEVEL_INFO: i32 = 6;
pub const NFT_LOGLEVEL_DEBUG: i32 = 7;
pub const NFT_LOGLEVEL_AUDIT: i32 = 8;
pub const __NFT_LOGLEVEL_MAX: i32 = 9;

pub const NFT_LOGLEVEL_MAX: i32 = (__NFT_LOGLEVEL_MAX - 1);
pub type nft_queue_attributes = i32;
pub const NFTA_QUEUE_UNSPEC: i32 = 0;
pub const NFTA_QUEUE_NUM: i32 = 1;
pub const NFTA_QUEUE_TOTAL: i32 = 2;
pub const NFTA_QUEUE_FLAGS: i32 = 3;
pub const NFTA_QUEUE_SREG_QNUM: i32 = 4;
pub const __NFTA_QUEUE_MAX: i32 = 5;

pub const NFTA_QUEUE_MAX: i32 = (__NFTA_QUEUE_MAX - 1);
pub const NFT_QUEUE_FLAG_BYPASS: i32 = 0x01;
pub const NFT_QUEUE_FLAG_CPU_FANOUT: i32 = 0x02;
pub const NFT_QUEUE_FLAG_MASK: i32 = 0x03;
pub type nft_quota_flags = i32;
pub const NFT_QUOTA_F_INV: i32 = 1;
pub const NFT_QUOTA_F_DEPLETED: i32 = 2;

pub type nft_quota_attributes = i32;
pub const NFTA_QUOTA_UNSPEC: i32 = 0;
pub const NFTA_QUOTA_BYTES: i32 = 1;
pub const NFTA_QUOTA_FLAGS: i32 = 2;
pub const NFTA_QUOTA_PAD: i32 = 3;
pub const NFTA_QUOTA_CONSUMED: i32 = 4;
pub const __NFTA_QUOTA_MAX: i32 = 5;

pub const NFTA_QUOTA_MAX: i32 = (__NFTA_QUOTA_MAX - 1);
pub type nft_secmark_attributes = i32;
pub const NFTA_SECMARK_UNSPEC: i32 = 0;
pub const NFTA_SECMARK_CTX: i32 = 1;
pub const __NFTA_SECMARK_MAX: i32 = 2;

pub const NFTA_SECMARK_MAX: i32 = (__NFTA_SECMARK_MAX - 1);
pub const NFT_SECMARK_CTX_MAXLEN: i32 = 4096;
pub type nft_reject_types = i32;
pub const NFT_REJECT_ICMP_UNREACH: i32 = 0;
pub const NFT_REJECT_TCP_RST: i32 = 1;
pub const NFT_REJECT_ICMPX_UNREACH: i32 = 2;

pub type nft_reject_inet_code = i32;
pub const NFT_REJECT_ICMPX_NO_ROUTE: i32 = 0;
pub const NFT_REJECT_ICMPX_PORT_UNREACH: i32 = 1;
pub const NFT_REJECT_ICMPX_HOST_UNREACH: i32 = 2;
pub const NFT_REJECT_ICMPX_ADMIN_PROHIBITED: i32 = 3;
pub const __NFT_REJECT_ICMPX_MAX: i32 = 4;

pub const NFT_REJECT_ICMPX_MAX: i32 = (__NFT_REJECT_ICMPX_MAX - 1);
pub type nft_reject_attributes = i32;
pub const NFTA_REJECT_UNSPEC: i32 = 0;
pub const NFTA_REJECT_TYPE: i32 = 1;
pub const NFTA_REJECT_ICMP_CODE: i32 = 2;
pub const __NFTA_REJECT_MAX: i32 = 3;

pub const NFTA_REJECT_MAX: i32 = (__NFTA_REJECT_MAX - 1);
pub type nft_nat_types = i32;
pub const NFT_NAT_SNAT: i32 = 0;
pub const NFT_NAT_DNAT: i32 = 1;

pub type nft_nat_attributes = i32;
pub const NFTA_NAT_UNSPEC: i32 = 0;
pub const NFTA_NAT_TYPE: i32 = 1;
pub const NFTA_NAT_FAMILY: i32 = 2;
pub const NFTA_NAT_REG_ADDR_MIN: i32 = 3;
pub const NFTA_NAT_REG_ADDR_MAX: i32 = 4;
pub const NFTA_NAT_REG_PROTO_MIN: i32 = 5;
pub const NFTA_NAT_REG_PROTO_MAX: i32 = 6;
pub const NFTA_NAT_FLAGS: i32 = 7;
pub const __NFTA_NAT_MAX: i32 = 8;

pub const NFTA_NAT_MAX: i32 = (__NFTA_NAT_MAX - 1);
pub type nft_tproxy_attributes = i32;
pub const NFTA_TPROXY_UNSPEC: i32 = 0;
pub const NFTA_TPROXY_FAMILY: i32 = 1;
pub const NFTA_TPROXY_REG_ADDR: i32 = 2;
pub const NFTA_TPROXY_REG_PORT: i32 = 3;
pub const __NFTA_TPROXY_MAX: i32 = 4;

pub const NFTA_TPROXY_MAX: i32 = (__NFTA_TPROXY_MAX - 1);
pub type nft_masq_attributes = i32;
pub const NFTA_MASQ_UNSPEC: i32 = 0;
pub const NFTA_MASQ_FLAGS: i32 = 1;
pub const NFTA_MASQ_REG_PROTO_MIN: i32 = 2;
pub const NFTA_MASQ_REG_PROTO_MAX: i32 = 3;
pub const __NFTA_MASQ_MAX: i32 = 4;

pub const NFTA_MASQ_MAX: i32 = (__NFTA_MASQ_MAX - 1);
pub type nft_redir_attributes = i32;
pub const NFTA_REDIR_UNSPEC: i32 = 0;
pub const NFTA_REDIR_REG_PROTO_MIN: i32 = 1;
pub const NFTA_REDIR_REG_PROTO_MAX: i32 = 2;
pub const NFTA_REDIR_FLAGS: i32 = 3;
pub const __NFTA_REDIR_MAX: i32 = 4;

pub const NFTA_REDIR_MAX: i32 = (__NFTA_REDIR_MAX - 1);
pub type nft_dup_attributes = i32;
pub const NFTA_DUP_UNSPEC: i32 = 0;
pub const NFTA_DUP_SREG_ADDR: i32 = 1;
pub const NFTA_DUP_SREG_DEV: i32 = 2;
pub const __NFTA_DUP_MAX: i32 = 3;

pub const NFTA_DUP_MAX: i32 = (__NFTA_DUP_MAX - 1);
pub type nft_fwd_attributes = i32;
pub const NFTA_FWD_UNSPEC: i32 = 0;
pub const NFTA_FWD_SREG_DEV: i32 = 1;
pub const NFTA_FWD_SREG_ADDR: i32 = 2;
pub const NFTA_FWD_NFPROTO: i32 = 3;
pub const __NFTA_FWD_MAX: i32 = 4;

pub const NFTA_FWD_MAX: i32 = (__NFTA_FWD_MAX - 1);
pub type nft_objref_attributes = i32;
pub const NFTA_OBJREF_UNSPEC: i32 = 0;
pub const NFTA_OBJREF_IMM_TYPE: i32 = 1;
pub const NFTA_OBJREF_IMM_NAME: i32 = 2;
pub const NFTA_OBJREF_SET_SREG: i32 = 3;
pub const NFTA_OBJREF_SET_NAME: i32 = 4;
pub const NFTA_OBJREF_SET_ID: i32 = 5;
pub const __NFTA_OBJREF_MAX: i32 = 6;

pub const NFTA_OBJREF_MAX: i32 = (__NFTA_OBJREF_MAX - 1);
pub type nft_gen_attributes = i32;
pub const NFTA_GEN_UNSPEC: i32 = 0;
pub const NFTA_GEN_ID: i32 = 1;
pub const NFTA_GEN_PROC_PID: i32 = 2;
pub const NFTA_GEN_PROC_NAME: i32 = 3;
pub const __NFTA_GEN_MAX: i32 = 4;

pub const NFTA_GEN_MAX: i32 = (__NFTA_GEN_MAX - 1);
pub type nft_fib_attributes = i32;
pub const NFTA_FIB_UNSPEC: i32 = 0;
pub const NFTA_FIB_DREG: i32 = 1;
pub const NFTA_FIB_RESULT: i32 = 2;
pub const NFTA_FIB_FLAGS: i32 = 3;
pub const __NFTA_FIB_MAX: i32 = 4;

pub const NFTA_FIB_MAX: i32 = (__NFTA_FIB_MAX - 1);
pub type nft_fib_result = i32;
pub const NFT_FIB_RESULT_UNSPEC: i32 = 0;
pub const NFT_FIB_RESULT_OIF: i32 = 1;
pub const NFT_FIB_RESULT_OIFNAME: i32 = 2;
pub const NFT_FIB_RESULT_ADDRTYPE: i32 = 3;
pub const __NFT_FIB_RESULT_MAX: i32 = 4;

pub const NFT_FIB_RESULT_MAX: i32 = (__NFT_FIB_RESULT_MAX - 1);
pub type nft_fib_flags = i32;
pub const NFTA_FIB_F_SADDR: i32 = 1;
pub const NFTA_FIB_F_DADDR: i32 = 2;
pub const NFTA_FIB_F_MARK: i32 = 4;
pub const NFTA_FIB_F_IIF: i32 = 8;
pub const NFTA_FIB_F_OIF: i32 = 16;
pub const NFTA_FIB_F_PRESENT: i32 = 32;

pub type nft_ct_helper_attributes = i32;
pub const NFTA_CT_HELPER_UNSPEC: i32 = 0;
pub const NFTA_CT_HELPER_NAME: i32 = 1;
pub const NFTA_CT_HELPER_L3PROTO: i32 = 2;
pub const NFTA_CT_HELPER_L4PROTO: i32 = 3;
pub const __NFTA_CT_HELPER_MAX: i32 = 4;

pub const NFTA_CT_HELPER_MAX: i32 = (__NFTA_CT_HELPER_MAX - 1);
pub type nft_ct_timeout_timeout_attributes = i32;
pub const NFTA_CT_TIMEOUT_UNSPEC: i32 = 0;
pub const NFTA_CT_TIMEOUT_L3PROTO: i32 = 1;
pub const NFTA_CT_TIMEOUT_L4PROTO: i32 = 2;
pub const NFTA_CT_TIMEOUT_DATA: i32 = 3;
pub const __NFTA_CT_TIMEOUT_MAX: i32 = 4;

pub const NFTA_CT_TIMEOUT_MAX: i32 = (__NFTA_CT_TIMEOUT_MAX - 1);
pub type nft_ct_expectation_attributes = i32;
pub const NFTA_CT_EXPECT_UNSPEC: i32 = 0;
pub const NFTA_CT_EXPECT_L3PROTO: i32 = 1;
pub const NFTA_CT_EXPECT_L4PROTO: i32 = 2;
pub const NFTA_CT_EXPECT_DPORT: i32 = 3;
pub const NFTA_CT_EXPECT_TIMEOUT: i32 = 4;
pub const NFTA_CT_EXPECT_SIZE: i32 = 5;
pub const __NFTA_CT_EXPECT_MAX: i32 = 6;

pub const NFTA_CT_EXPECT_MAX: i32 = (__NFTA_CT_EXPECT_MAX - 1);
pub const NFT_OBJECT_UNSPEC: i32 = 0;
pub const NFT_OBJECT_COUNTER: i32 = 1;
pub const NFT_OBJECT_QUOTA: i32 = 2;
pub const NFT_OBJECT_CT_HELPER: i32 = 3;
pub const NFT_OBJECT_LIMIT: i32 = 4;
pub const NFT_OBJECT_CONNLIMIT: i32 = 5;
pub const NFT_OBJECT_TUNNEL: i32 = 6;
pub const NFT_OBJECT_CT_TIMEOUT: i32 = 7;
pub const NFT_OBJECT_SECMARK: i32 = 8;
pub const NFT_OBJECT_CT_EXPECT: i32 = 9;
pub const NFT_OBJECT_SYNPROXY: i32 = 10;
pub const __NFT_OBJECT_MAX: i32 = 11;
pub const NFT_OBJECT_MAX: i32 = (__NFT_OBJECT_MAX - 1);
pub type nft_object_attributes = i32;
pub const NFTA_OBJ_UNSPEC: i32 = 0;
pub const NFTA_OBJ_TABLE: i32 = 1;
pub const NFTA_OBJ_NAME: i32 = 2;
pub const NFTA_OBJ_TYPE: i32 = 3;
pub const NFTA_OBJ_DATA: i32 = 4;
pub const NFTA_OBJ_USE: i32 = 5;
pub const NFTA_OBJ_HANDLE: i32 = 6;
pub const NFTA_OBJ_PAD: i32 = 7;
pub const NFTA_OBJ_USERDATA: i32 = 8;
pub const __NFTA_OBJ_MAX: i32 = 9;

pub const NFTA_OBJ_MAX: i32 = (__NFTA_OBJ_MAX - 1);
pub type nft_flowtable_flags = i32;
pub const NFT_FLOWTABLE_HW_OFFLOAD: i32 = 1;
pub const NFT_FLOWTABLE_COUNTER: i32 = 2;
pub const NFT_FLOWTABLE_MASK: i32 = (NFT_FLOWTABLE_HW_OFFLOAD |;
pub const NFT_FLOWTABLE_COUNTER): i32 = 3;

pub type nft_flowtable_attributes = i32;
pub const NFTA_FLOWTABLE_UNSPEC: i32 = 0;
pub const NFTA_FLOWTABLE_TABLE: i32 = 1;
pub const NFTA_FLOWTABLE_NAME: i32 = 2;
pub const NFTA_FLOWTABLE_HOOK: i32 = 3;
pub const NFTA_FLOWTABLE_USE: i32 = 4;
pub const NFTA_FLOWTABLE_HANDLE: i32 = 5;
pub const NFTA_FLOWTABLE_PAD: i32 = 6;
pub const NFTA_FLOWTABLE_FLAGS: i32 = 7;
pub const __NFTA_FLOWTABLE_MAX: i32 = 8;

pub const NFTA_FLOWTABLE_MAX: i32 = (__NFTA_FLOWTABLE_MAX - 1);
pub type nft_flowtable_hook_attributes = i32;
pub const NFTA_FLOWTABLE_HOOK_UNSPEC: i32 = 0;
pub const NFTA_FLOWTABLE_HOOK_NUM: i32 = 1;
pub const NFTA_FLOWTABLE_HOOK_PRIORITY: i32 = 2;
pub const NFTA_FLOWTABLE_HOOK_DEVS: i32 = 3;
pub const __NFTA_FLOWTABLE_HOOK_MAX: i32 = 4;

pub const NFTA_FLOWTABLE_HOOK_MAX: i32 = (__NFTA_FLOWTABLE_HOOK_MAX - 1);
pub type nft_osf_attributes = i32;
pub const NFTA_OSF_UNSPEC: i32 = 0;
pub const NFTA_OSF_DREG: i32 = 1;
pub const NFTA_OSF_TTL: i32 = 2;
pub const NFTA_OSF_FLAGS: i32 = 3;
pub const __NFTA_OSF_MAX: i32 = 4;

pub const NFTA_OSF_MAX: i32 = (__NFTA_OSF_MAX - 1);
pub type nft_osf_flags = i32;
pub const NFT_OSF_F_VERSION: i32 = 1;

pub type nft_synproxy_attributes = i32;
pub const NFTA_SYNPROXY_UNSPEC: i32 = 0;
pub const NFTA_SYNPROXY_MSS: i32 = 1;
pub const NFTA_SYNPROXY_WSCALE: i32 = 2;
pub const NFTA_SYNPROXY_FLAGS: i32 = 3;
pub const __NFTA_SYNPROXY_MAX: i32 = 4;

pub const NFTA_SYNPROXY_MAX: i32 = (__NFTA_SYNPROXY_MAX - 1);
pub type nft_devices_attributes = i32;
pub const NFTA_DEVICE_UNSPEC: i32 = 0;
pub const NFTA_DEVICE_NAME: i32 = 1;
pub const NFTA_DEVICE_PREFIX: i32 = 2;
pub const __NFTA_DEVICE_MAX: i32 = 3;

pub const NFTA_DEVICE_MAX: i32 = (__NFTA_DEVICE_MAX - 1);
pub type nft_xfrm_attributes = i32;
pub const NFTA_XFRM_UNSPEC: i32 = 0;
pub const NFTA_XFRM_DREG: i32 = 1;
pub const NFTA_XFRM_KEY: i32 = 2;
pub const NFTA_XFRM_DIR: i32 = 3;
pub const NFTA_XFRM_SPNUM: i32 = 4;
pub const __NFTA_XFRM_MAX: i32 = 5;

pub const NFTA_XFRM_MAX: i32 = (__NFTA_XFRM_MAX - 1);
pub type nft_xfrm_keys = i32;
pub const NFT_XFRM_KEY_UNSPEC: i32 = 0;
pub const NFT_XFRM_KEY_DADDR_IP4: i32 = 1;
pub const NFT_XFRM_KEY_DADDR_IP6: i32 = 2;
pub const NFT_XFRM_KEY_SADDR_IP4: i32 = 3;
pub const NFT_XFRM_KEY_SADDR_IP6: i32 = 4;
pub const NFT_XFRM_KEY_REQID: i32 = 5;
pub const NFT_XFRM_KEY_SPI: i32 = 6;
pub const __NFT_XFRM_KEY_MAX: i32 = 7;

pub const NFT_XFRM_KEY_MAX: i32 = (__NFT_XFRM_KEY_MAX - 1);
pub type nft_trace_attributes = i32;
pub const NFTA_TRACE_UNSPEC: i32 = 0;
pub const NFTA_TRACE_TABLE: i32 = 1;
pub const NFTA_TRACE_CHAIN: i32 = 2;
pub const NFTA_TRACE_RULE_HANDLE: i32 = 3;
pub const NFTA_TRACE_TYPE: i32 = 4;
pub const NFTA_TRACE_VERDICT: i32 = 5;
pub const NFTA_TRACE_ID: i32 = 6;
pub const NFTA_TRACE_LL_HEADER: i32 = 7;
pub const NFTA_TRACE_NETWORK_HEADER: i32 = 8;
pub const NFTA_TRACE_TRANSPORT_HEADER: i32 = 9;
pub const NFTA_TRACE_IIF: i32 = 10;
pub const NFTA_TRACE_IIFTYPE: i32 = 11;
pub const NFTA_TRACE_OIF: i32 = 12;
pub const NFTA_TRACE_OIFTYPE: i32 = 13;
pub const NFTA_TRACE_MARK: i32 = 14;
pub const NFTA_TRACE_NFPROTO: i32 = 15;
pub const NFTA_TRACE_POLICY: i32 = 16;
pub const NFTA_TRACE_PAD: i32 = 17;
pub const NFTA_TRACE_CT_ID: i32 = 18;
pub const NFTA_TRACE_CT_DIRECTION: i32 = 19;
pub const NFTA_TRACE_CT_STATUS: i32 = 20;
pub const NFTA_TRACE_CT_STATE: i32 = 21;
pub const __NFTA_TRACE_MAX: i32 = 22;

pub const NFTA_TRACE_MAX: i32 = (__NFTA_TRACE_MAX - 1);
pub type nft_trace_types = i32;
pub const NFT_TRACETYPE_UNSPEC: i32 = 0;
pub const NFT_TRACETYPE_POLICY: i32 = 1;
pub const NFT_TRACETYPE_RETURN: i32 = 2;
pub const NFT_TRACETYPE_RULE: i32 = 3;
pub const __NFT_TRACETYPE_MAX: i32 = 4;

pub const NFT_TRACETYPE_MAX: i32 = (__NFT_TRACETYPE_MAX - 1);
pub type nft_ng_attributes = i32;
pub const NFTA_NG_UNSPEC: i32 = 0;
pub const NFTA_NG_DREG: i32 = 1;
pub const NFTA_NG_MODULUS: i32 = 2;
pub const NFTA_NG_TYPE: i32 = 3;
pub const NFTA_NG_OFFSET: i32 = 4;
pub const NFTA_NG_SET_NAME: i32 = 5;
pub const NFTA_NG_SET_ID: i32 = 6;
pub const __NFTA_NG_MAX: i32 = 7;

pub const NFTA_NG_MAX: i32 = (__NFTA_NG_MAX - 1);
pub type nft_ng_types = i32;
pub const NFT_NG_INCREMENTAL: i32 = 0;
pub const NFT_NG_RANDOM: i32 = 1;
pub const __NFT_NG_MAX: i32 = 2;

pub const NFT_NG_MAX: i32 = (__NFT_NG_MAX - 1);
pub type nft_tunnel_key_ip_attributes = i32;
pub const NFTA_TUNNEL_KEY_IP_UNSPEC: i32 = 0;
pub const NFTA_TUNNEL_KEY_IP_SRC: i32 = 1;
pub const NFTA_TUNNEL_KEY_IP_DST: i32 = 2;
pub const __NFTA_TUNNEL_KEY_IP_MAX: i32 = 3;

pub const NFTA_TUNNEL_KEY_IP_MAX: i32 = (__NFTA_TUNNEL_KEY_IP_MAX - 1);
pub type nft_tunnel_ip6_attributes = i32;
pub const NFTA_TUNNEL_KEY_IP6_UNSPEC: i32 = 0;
pub const NFTA_TUNNEL_KEY_IP6_SRC: i32 = 1;
pub const NFTA_TUNNEL_KEY_IP6_DST: i32 = 2;
pub const NFTA_TUNNEL_KEY_IP6_FLOWLABEL: i32 = 3;
pub const __NFTA_TUNNEL_KEY_IP6_MAX: i32 = 4;

pub const NFTA_TUNNEL_KEY_IP6_MAX: i32 = (__NFTA_TUNNEL_KEY_IP6_MAX - 1);
pub type nft_tunnel_opts_attributes = i32;
pub const NFTA_TUNNEL_KEY_OPTS_UNSPEC: i32 = 0;
pub const NFTA_TUNNEL_KEY_OPTS_VXLAN: i32 = 1;
pub const NFTA_TUNNEL_KEY_OPTS_ERSPAN: i32 = 2;
pub const NFTA_TUNNEL_KEY_OPTS_GENEVE: i32 = 3;
pub const __NFTA_TUNNEL_KEY_OPTS_MAX: i32 = 4;

pub const NFTA_TUNNEL_KEY_OPTS_MAX: i32 = (__NFTA_TUNNEL_KEY_OPTS_MAX - 1);
pub type nft_tunnel_opts_vxlan_attributes = i32;
pub const NFTA_TUNNEL_KEY_VXLAN_UNSPEC: i32 = 0;
pub const NFTA_TUNNEL_KEY_VXLAN_GBP: i32 = 1;
pub const __NFTA_TUNNEL_KEY_VXLAN_MAX: i32 = 2;

pub const NFTA_TUNNEL_KEY_VXLAN_MAX: i32 = (__NFTA_TUNNEL_KEY_VXLAN_MAX - 1);
pub type nft_tunnel_opts_erspan_attributes = i32;
pub const NFTA_TUNNEL_KEY_ERSPAN_UNSPEC: i32 = 0;
pub const NFTA_TUNNEL_KEY_ERSPAN_VERSION: i32 = 1;
pub const NFTA_TUNNEL_KEY_ERSPAN_V1_INDEX: i32 = 2;
pub const NFTA_TUNNEL_KEY_ERSPAN_V2_HWID: i32 = 3;
pub const NFTA_TUNNEL_KEY_ERSPAN_V2_DIR: i32 = 4;
pub const __NFTA_TUNNEL_KEY_ERSPAN_MAX: i32 = 5;

pub const NFTA_TUNNEL_KEY_ERSPAN_MAX: i32 = (__NFTA_TUNNEL_KEY_ERSPAN_MAX - 1);
pub type nft_tunnel_opts_geneve_attributes = i32;
pub const NFTA_TUNNEL_KEY_GENEVE_UNSPEC: i32 = 0;
pub const NFTA_TUNNEL_KEY_GENEVE_CLASS: i32 = 1;
pub const NFTA_TUNNEL_KEY_GENEVE_TYPE: i32 = 2;
pub const NFTA_TUNNEL_KEY_GENEVE_DATA: i32 = 3;
pub const __NFTA_TUNNEL_KEY_GENEVE_MAX: i32 = 4;

pub const NFTA_TUNNEL_KEY_GENEVE_MAX: i32 = (__NFTA_TUNNEL_KEY_GENEVE_MAX - 1);
pub type nft_tunnel_flags = i32;
pub const NFT_TUNNEL_F_ZERO_CSUM_TX: i32 = 1;
pub const NFT_TUNNEL_F_DONT_FRAGMENT: i32 = 2;
pub const NFT_TUNNEL_F_SEQ_NUMBER: i32 = 4;

pub const NFT_TUNNEL_F_MASK: u32 = ((NFT_TUNNEL_F_ZERO_CSUM_TX as u32) |  (NFT_TUNNEL_F_DONT_FRAGMENT as u32) |  (NFT_TUNNEL_F_SEQ_NUMBER as u32));
pub type nft_tunnel_key_attributes = i32;
pub const NFTA_TUNNEL_KEY_UNSPEC: i32 = 0;
pub const NFTA_TUNNEL_KEY_ID: i32 = 1;
pub const NFTA_TUNNEL_KEY_IP: i32 = 2;
pub const NFTA_TUNNEL_KEY_IP6: i32 = 3;
pub const NFTA_TUNNEL_KEY_FLAGS: i32 = 4;
pub const NFTA_TUNNEL_KEY_TOS: i32 = 5;
pub const NFTA_TUNNEL_KEY_TTL: i32 = 6;
pub const NFTA_TUNNEL_KEY_SPORT: i32 = 7;
pub const NFTA_TUNNEL_KEY_DPORT: i32 = 8;
pub const NFTA_TUNNEL_KEY_OPTS: i32 = 9;
pub const __NFTA_TUNNEL_KEY_MAX: i32 = 10;

pub const NFTA_TUNNEL_KEY_MAX: i32 = (__NFTA_TUNNEL_KEY_MAX - 1);
pub type nft_tunnel_keys = i32;
pub const NFT_TUNNEL_PATH: i32 = 0;
pub const NFT_TUNNEL_ID: i32 = 1;
pub const __NFT_TUNNEL_MAX: i32 = 2;

pub const NFT_TUNNEL_MAX: i32 = (__NFT_TUNNEL_MAX - 1);
pub type nft_tunnel_mode = i32;
pub const NFT_TUNNEL_MODE_NONE: i32 = 0;
pub const NFT_TUNNEL_MODE_RX: i32 = 1;
pub const NFT_TUNNEL_MODE_TX: i32 = 2;
pub const __NFT_TUNNEL_MODE_MAX: i32 = 3;

pub const NFT_TUNNEL_MODE_MAX: i32 = (__NFT_TUNNEL_MODE_MAX - 1);
pub type nft_tunnel_attributes = i32;
pub const NFTA_TUNNEL_UNSPEC: i32 = 0;
pub const NFTA_TUNNEL_KEY: i32 = 1;
pub const NFTA_TUNNEL_DREG: i32 = 2;
pub const NFTA_TUNNEL_MODE: i32 = 3;
pub const __NFTA_TUNNEL_MAX: i32 = 4;

pub const NFTA_TUNNEL_MAX: i32 = (__NFTA_TUNNEL_MAX - 1);
