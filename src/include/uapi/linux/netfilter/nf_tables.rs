// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/netfilter/nf_tables.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016277

// UAPI constants and enum values translated from the pinned Linux header.

pub const NFT_NAME_MAXLEN = 256;
pub const NFT_TABLE_MAXNAMELEN = NFT_NAME_MAXLEN;
pub const NFT_CHAIN_MAXNAMELEN = NFT_NAME_MAXLEN;
pub const NFT_SET_MAXNAMELEN = NFT_NAME_MAXLEN;
pub const NFT_OBJ_MAXNAMELEN = NFT_NAME_MAXLEN;
pub const NFT_USERDATA_MAXLEN = 256;
pub const NFT_OSF_MAXGENRELEN = 16;
// C enum nft_registers
pub type nft_registers = i32;
pub const NFT_REG_VERDICT = 0;
pub const NFT_REG_1 = 1;
pub const NFT_REG_2 = 2;
pub const NFT_REG_3 = 3;
pub const NFT_REG_4 = 4;
pub const __NFT_REG_MAX = 5;
pub const NFT_REG32_00 = 8;
pub const NFT_REG32_01 = 9;
pub const NFT_REG32_02 = 10;
pub const NFT_REG32_03 = 11;
pub const NFT_REG32_04 = 12;
pub const NFT_REG32_05 = 13;
pub const NFT_REG32_06 = 14;
pub const NFT_REG32_07 = 15;
pub const NFT_REG32_08 = 16;
pub const NFT_REG32_09 = 17;
pub const NFT_REG32_10 = 18;
pub const NFT_REG32_11 = 19;
pub const NFT_REG32_12 = 20;
pub const NFT_REG32_13 = 21;
pub const NFT_REG32_14 = 22;
pub const NFT_REG32_15 = 23;

pub const NFT_REG_MAX = (__NFT_REG_MAX - 1);
#[cfg(feature = "__KERNEL__")]
pub const NFT_REG32_MAX = NFT_REG32_15;
pub const NFT_REG_SIZE = 16;
pub const NFT_REG32_SIZE = 4;
pub const NFT_REG32_COUNT = (NFT_REG32_15 - NFT_REG32_00 + 1);
// C enum nft_verdicts
pub type nft_verdicts = i32;
pub const NFT_CONTINUE = -1;
pub const NFT_BREAK = -2;
pub const NFT_JUMP = -3;
pub const NFT_GOTO = -4;
pub const NFT_RETURN = -5;

// C enum nf_tables_msg_types
pub type nf_tables_msg_types = i32;
pub const NFT_MSG_NEWTABLE = 0;
pub const NFT_MSG_GETTABLE = 1;
pub const NFT_MSG_DELTABLE = 2;
pub const NFT_MSG_NEWCHAIN = 3;
pub const NFT_MSG_GETCHAIN = 4;
pub const NFT_MSG_DELCHAIN = 5;
pub const NFT_MSG_NEWRULE = 6;
pub const NFT_MSG_GETRULE = 7;
pub const NFT_MSG_DELRULE = 8;
pub const NFT_MSG_NEWSET = 9;
pub const NFT_MSG_GETSET = 10;
pub const NFT_MSG_DELSET = 11;
pub const NFT_MSG_NEWSETELEM = 12;
pub const NFT_MSG_GETSETELEM = 13;
pub const NFT_MSG_DELSETELEM = 14;
pub const NFT_MSG_NEWGEN = 15;
pub const NFT_MSG_GETGEN = 16;
pub const NFT_MSG_TRACE = 17;
pub const NFT_MSG_NEWOBJ = 18;
pub const NFT_MSG_GETOBJ = 19;
pub const NFT_MSG_DELOBJ = 20;
pub const NFT_MSG_GETOBJ_RESET = 21;
pub const NFT_MSG_NEWFLOWTABLE = 22;
pub const NFT_MSG_GETFLOWTABLE = 23;
pub const NFT_MSG_DELFLOWTABLE = 24;
pub const NFT_MSG_GETRULE_RESET = 25;
pub const NFT_MSG_DESTROYTABLE = 26;
pub const NFT_MSG_DESTROYCHAIN = 27;
pub const NFT_MSG_DESTROYRULE = 28;
pub const NFT_MSG_DESTROYSET = 29;
pub const NFT_MSG_DESTROYSETELEM = 30;
pub const NFT_MSG_DESTROYOBJ = 31;
pub const NFT_MSG_DESTROYFLOWTABLE = 32;
pub const NFT_MSG_GETSETELEM_RESET = 33;
pub const NFT_MSG_MAX = 34;

// C enum nft_list_attributes
pub type nft_list_attributes = i32;
pub const NFTA_LIST_UNSPEC = 0;
pub const NFTA_LIST_ELEM = 1;
pub const __NFTA_LIST_MAX = 2;

pub const NFTA_LIST_MAX = (__NFTA_LIST_MAX - 1);
// C enum nft_hook_attributes
pub type nft_hook_attributes = i32;
pub const NFTA_HOOK_UNSPEC = 0;
pub const NFTA_HOOK_HOOKNUM = 1;
pub const NFTA_HOOK_PRIORITY = 2;
pub const NFTA_HOOK_DEV = 3;
pub const NFTA_HOOK_DEVS = 4;
pub const __NFTA_HOOK_MAX = 5;

pub const NFTA_HOOK_MAX = (__NFTA_HOOK_MAX - 1);
// C enum nft_table_flags
pub type nft_table_flags = i32;
pub const NFT_TABLE_F_DORMANT = 0x1;
pub const NFT_TABLE_F_OWNER = 0x2;
pub const NFT_TABLE_F_PERSIST = 0x4;

pub const NFT_TABLE_F_MASK = (NFT_TABLE_F_DORMANT |  NFT_TABLE_F_OWNER |  NFT_TABLE_F_PERSIST);
// C enum nft_table_attributes
pub type nft_table_attributes = i32;
pub const NFTA_TABLE_UNSPEC = 0;
pub const NFTA_TABLE_NAME = 1;
pub const NFTA_TABLE_FLAGS = 2;
pub const NFTA_TABLE_USE = 3;
pub const NFTA_TABLE_HANDLE = 4;
pub const NFTA_TABLE_PAD = 5;
pub const NFTA_TABLE_USERDATA = 6;
pub const NFTA_TABLE_OWNER = 7;
pub const __NFTA_TABLE_MAX = 8;

pub const NFTA_TABLE_MAX = (__NFTA_TABLE_MAX - 1);
// C enum nft_chain_flags
pub type nft_chain_flags = i32;
pub const NFT_CHAIN_BASE = (1 << 0);
pub const NFT_CHAIN_HW_OFFLOAD = (1 << 1);
pub const NFT_CHAIN_BINDING = (1 << 2);

pub const NFT_CHAIN_FLAGS = (NFT_CHAIN_BASE		|  NFT_CHAIN_HW_OFFLOAD	|  NFT_CHAIN_BINDING);
// C enum nft_chain_attributes
pub type nft_chain_attributes = i32;
pub const NFTA_CHAIN_UNSPEC = 0;
pub const NFTA_CHAIN_TABLE = 1;
pub const NFTA_CHAIN_HANDLE = 2;
pub const NFTA_CHAIN_NAME = 3;
pub const NFTA_CHAIN_HOOK = 4;
pub const NFTA_CHAIN_POLICY = 5;
pub const NFTA_CHAIN_USE = 6;
pub const NFTA_CHAIN_TYPE = 7;
pub const NFTA_CHAIN_COUNTERS = 8;
pub const NFTA_CHAIN_PAD = 9;
pub const NFTA_CHAIN_FLAGS = 10;
pub const NFTA_CHAIN_ID = 11;
pub const NFTA_CHAIN_USERDATA = 12;
pub const __NFTA_CHAIN_MAX = 13;

pub const NFTA_CHAIN_MAX = (__NFTA_CHAIN_MAX - 1);
// C enum nft_rule_attributes
pub type nft_rule_attributes = i32;
pub const NFTA_RULE_UNSPEC = 0;
pub const NFTA_RULE_TABLE = 1;
pub const NFTA_RULE_CHAIN = 2;
pub const NFTA_RULE_HANDLE = 3;
pub const NFTA_RULE_EXPRESSIONS = 4;
pub const NFTA_RULE_COMPAT = 5;
pub const NFTA_RULE_POSITION = 6;
pub const NFTA_RULE_USERDATA = 7;
pub const NFTA_RULE_PAD = 8;
pub const NFTA_RULE_ID = 9;
pub const NFTA_RULE_POSITION_ID = 10;
pub const NFTA_RULE_CHAIN_ID = 11;
pub const __NFTA_RULE_MAX = 12;

pub const NFTA_RULE_MAX = (__NFTA_RULE_MAX - 1);
// C enum nft_rule_compat_flags
pub type nft_rule_compat_flags = i32;
pub const NFT_RULE_COMPAT_F_UNUSED = (1 << 0);
pub const NFT_RULE_COMPAT_F_INV = (1 << 1);
pub const NFT_RULE_COMPAT_F_MASK = NFT_RULE_COMPAT_F_INV;

// C enum nft_rule_compat_attributes
pub type nft_rule_compat_attributes = i32;
pub const NFTA_RULE_COMPAT_UNSPEC = 0;
pub const NFTA_RULE_COMPAT_PROTO = 1;
pub const NFTA_RULE_COMPAT_FLAGS = 2;
pub const __NFTA_RULE_COMPAT_MAX = 3;

pub const NFTA_RULE_COMPAT_MAX = (__NFTA_RULE_COMPAT_MAX - 1);
// C enum nft_set_flags
pub type nft_set_flags = i32;
pub const NFT_SET_ANONYMOUS = 0x1;
pub const NFT_SET_CONSTANT = 0x2;
pub const NFT_SET_INTERVAL = 0x4;
pub const NFT_SET_MAP = 0x8;
pub const NFT_SET_TIMEOUT = 0x10;
pub const NFT_SET_EVAL = 0x20;
pub const NFT_SET_OBJECT = 0x40;
pub const NFT_SET_CONCAT = 0x80;
pub const NFT_SET_EXPR = 0x100;

// C enum nft_set_policies
pub type nft_set_policies = i32;
pub const NFT_SET_POL_PERFORMANCE = 0;
pub const NFT_SET_POL_MEMORY = 1;

// C enum nft_set_desc_attributes
pub type nft_set_desc_attributes = i32;
pub const NFTA_SET_DESC_UNSPEC = 0;
pub const NFTA_SET_DESC_SIZE = 1;
pub const NFTA_SET_DESC_CONCAT = 2;
pub const __NFTA_SET_DESC_MAX = 3;

pub const NFTA_SET_DESC_MAX = (__NFTA_SET_DESC_MAX - 1);
// C enum nft_set_field_attributes
pub type nft_set_field_attributes = i32;
pub const NFTA_SET_FIELD_UNSPEC = 0;
pub const NFTA_SET_FIELD_LEN = 1;
pub const __NFTA_SET_FIELD_MAX = 2;

pub const NFTA_SET_FIELD_MAX = (__NFTA_SET_FIELD_MAX - 1);
// C enum nft_set_attributes
pub type nft_set_attributes = i32;
pub const NFTA_SET_UNSPEC = 0;
pub const NFTA_SET_TABLE = 1;
pub const NFTA_SET_NAME = 2;
pub const NFTA_SET_FLAGS = 3;
pub const NFTA_SET_KEY_TYPE = 4;
pub const NFTA_SET_KEY_LEN = 5;
pub const NFTA_SET_DATA_TYPE = 6;
pub const NFTA_SET_DATA_LEN = 7;
pub const NFTA_SET_POLICY = 8;
pub const NFTA_SET_DESC = 9;
pub const NFTA_SET_ID = 10;
pub const NFTA_SET_TIMEOUT = 11;
pub const NFTA_SET_GC_INTERVAL = 12;
pub const NFTA_SET_USERDATA = 13;
pub const NFTA_SET_PAD = 14;
pub const NFTA_SET_OBJ_TYPE = 15;
pub const NFTA_SET_HANDLE = 16;
pub const NFTA_SET_EXPR = 17;
pub const NFTA_SET_EXPRESSIONS = 18;
pub const NFTA_SET_TYPE = 19;
pub const NFTA_SET_COUNT = 20;
pub const __NFTA_SET_MAX = 21;

pub const NFTA_SET_MAX = (__NFTA_SET_MAX - 1);
// C enum nft_set_elem_flags
pub type nft_set_elem_flags = i32;
pub const NFT_SET_ELEM_INTERVAL_END = 0x1;
pub const NFT_SET_ELEM_CATCHALL = 0x2;

// C enum nft_set_elem_attributes
pub type nft_set_elem_attributes = i32;
pub const NFTA_SET_ELEM_UNSPEC = 0;
pub const NFTA_SET_ELEM_KEY = 1;
pub const NFTA_SET_ELEM_DATA = 2;
pub const NFTA_SET_ELEM_FLAGS = 3;
pub const NFTA_SET_ELEM_TIMEOUT = 4;
pub const NFTA_SET_ELEM_EXPIRATION = 5;
pub const NFTA_SET_ELEM_USERDATA = 6;
pub const NFTA_SET_ELEM_EXPR = 7;
pub const NFTA_SET_ELEM_PAD = 8;
pub const NFTA_SET_ELEM_OBJREF = 9;
pub const NFTA_SET_ELEM_KEY_END = 10;
pub const NFTA_SET_ELEM_EXPRESSIONS = 11;
pub const __NFTA_SET_ELEM_MAX = 12;

pub const NFTA_SET_ELEM_MAX = (__NFTA_SET_ELEM_MAX - 1);
// C enum nft_set_elem_list_attributes
pub type nft_set_elem_list_attributes = i32;
pub const NFTA_SET_ELEM_LIST_UNSPEC = 0;
pub const NFTA_SET_ELEM_LIST_TABLE = 1;
pub const NFTA_SET_ELEM_LIST_SET = 2;
pub const NFTA_SET_ELEM_LIST_ELEMENTS = 3;
pub const NFTA_SET_ELEM_LIST_SET_ID = 4;
pub const __NFTA_SET_ELEM_LIST_MAX = 5;

pub const NFTA_SET_ELEM_LIST_MAX = (__NFTA_SET_ELEM_LIST_MAX - 1);
// C enum nft_data_types
pub type nft_data_types = u32;
pub const NFT_DATA_VALUE: nft_data_types = 0;
pub const NFT_DATA_VERDICT: nft_data_types = 0xffffff00u32;

pub const NFT_DATA_RESERVED_MASK: nft_data_types = 0xffffff00u32;
// C enum nft_data_attributes
pub type nft_data_attributes = i32;
pub const NFTA_DATA_UNSPEC = 0;
pub const NFTA_DATA_VALUE = 1;
pub const NFTA_DATA_VERDICT = 2;
pub const __NFTA_DATA_MAX = 3;

pub const NFTA_DATA_MAX = (__NFTA_DATA_MAX - 1);
pub const NFT_DATA_VALUE_MAXLEN = 64;
// C enum nft_verdict_attributes
pub type nft_verdict_attributes = i32;
pub const NFTA_VERDICT_UNSPEC = 0;
pub const NFTA_VERDICT_CODE = 1;
pub const NFTA_VERDICT_CHAIN = 2;
pub const NFTA_VERDICT_CHAIN_ID = 3;
pub const __NFTA_VERDICT_MAX = 4;

pub const NFTA_VERDICT_MAX = (__NFTA_VERDICT_MAX - 1);
// C enum nft_expr_attributes
pub type nft_expr_attributes = i32;
pub const NFTA_EXPR_UNSPEC = 0;
pub const NFTA_EXPR_NAME = 1;
pub const NFTA_EXPR_DATA = 2;
pub const __NFTA_EXPR_MAX = 3;

pub const NFTA_EXPR_MAX = (__NFTA_EXPR_MAX - 1);
// C enum nft_immediate_attributes
pub type nft_immediate_attributes = i32;
pub const NFTA_IMMEDIATE_UNSPEC = 0;
pub const NFTA_IMMEDIATE_DREG = 1;
pub const NFTA_IMMEDIATE_DATA = 2;
pub const __NFTA_IMMEDIATE_MAX = 3;

pub const NFTA_IMMEDIATE_MAX = (__NFTA_IMMEDIATE_MAX - 1);
// C enum nft_bitwise_ops
pub type nft_bitwise_ops = i32;
pub const NFT_BITWISE_MASK_XOR = 0;
pub const NFT_BITWISE_LSHIFT = 1;
pub const NFT_BITWISE_RSHIFT = 2;
pub const NFT_BITWISE_AND = 3;
pub const NFT_BITWISE_OR = 4;
pub const NFT_BITWISE_XOR = 5;

pub const NFT_BITWISE_BOOL = NFT_BITWISE_MASK_XOR;
// C enum nft_bitwise_attributes
pub type nft_bitwise_attributes = i32;
pub const NFTA_BITWISE_UNSPEC = 0;
pub const NFTA_BITWISE_SREG = 1;
pub const NFTA_BITWISE_DREG = 2;
pub const NFTA_BITWISE_LEN = 3;
pub const NFTA_BITWISE_MASK = 4;
pub const NFTA_BITWISE_XOR = 5;
pub const NFTA_BITWISE_OP = 6;
pub const NFTA_BITWISE_DATA = 7;
pub const NFTA_BITWISE_SREG2 = 8;
pub const __NFTA_BITWISE_MAX = 9;

pub const NFTA_BITWISE_MAX = (__NFTA_BITWISE_MAX - 1);
// C enum nft_byteorder_ops
pub type nft_byteorder_ops = i32;
pub const NFT_BYTEORDER_NTOH = 0;
pub const NFT_BYTEORDER_HTON = 1;

// C enum nft_byteorder_attributes
pub type nft_byteorder_attributes = i32;
pub const NFTA_BYTEORDER_UNSPEC = 0;
pub const NFTA_BYTEORDER_SREG = 1;
pub const NFTA_BYTEORDER_DREG = 2;
pub const NFTA_BYTEORDER_OP = 3;
pub const NFTA_BYTEORDER_LEN = 4;
pub const NFTA_BYTEORDER_SIZE = 5;
pub const __NFTA_BYTEORDER_MAX = 6;

pub const NFTA_BYTEORDER_MAX = (__NFTA_BYTEORDER_MAX - 1);
// C enum nft_cmp_ops
pub type nft_cmp_ops = i32;
pub const NFT_CMP_EQ = 0;
pub const NFT_CMP_NEQ = 1;
pub const NFT_CMP_LT = 2;
pub const NFT_CMP_LTE = 3;
pub const NFT_CMP_GT = 4;
pub const NFT_CMP_GTE = 5;

// C enum nft_cmp_attributes
pub type nft_cmp_attributes = i32;
pub const NFTA_CMP_UNSPEC = 0;
pub const NFTA_CMP_SREG = 1;
pub const NFTA_CMP_OP = 2;
pub const NFTA_CMP_DATA = 3;
pub const __NFTA_CMP_MAX = 4;

pub const NFTA_CMP_MAX = (__NFTA_CMP_MAX - 1);
// C enum nft_range_ops
pub type nft_range_ops = i32;
pub const NFT_RANGE_EQ = 0;
pub const NFT_RANGE_NEQ = 1;

// C enum nft_range_attributes
pub type nft_range_attributes = i32;
pub const NFTA_RANGE_UNSPEC = 0;
pub const NFTA_RANGE_SREG = 1;
pub const NFTA_RANGE_OP = 2;
pub const NFTA_RANGE_FROM_DATA = 3;
pub const NFTA_RANGE_TO_DATA = 4;
pub const __NFTA_RANGE_MAX = 5;

pub const NFTA_RANGE_MAX = (__NFTA_RANGE_MAX - 1);
// C enum nft_lookup_flags
pub type nft_lookup_flags = i32;
pub const NFT_LOOKUP_F_INV = (1 << 0);

// C enum nft_lookup_attributes
pub type nft_lookup_attributes = i32;
pub const NFTA_LOOKUP_UNSPEC = 0;
pub const NFTA_LOOKUP_SET = 1;
pub const NFTA_LOOKUP_SREG = 2;
pub const NFTA_LOOKUP_DREG = 3;
pub const NFTA_LOOKUP_SET_ID = 4;
pub const NFTA_LOOKUP_FLAGS = 5;
pub const __NFTA_LOOKUP_MAX = 6;

pub const NFTA_LOOKUP_MAX = (__NFTA_LOOKUP_MAX - 1);
// C enum nft_dynset_ops
pub type nft_dynset_ops = i32;
pub const NFT_DYNSET_OP_ADD = 0;
pub const NFT_DYNSET_OP_UPDATE = 1;
pub const NFT_DYNSET_OP_DELETE = 2;

// C enum nft_dynset_flags
pub type nft_dynset_flags = i32;
pub const NFT_DYNSET_F_INV = (1 << 0);
pub const NFT_DYNSET_F_EXPR = (1 << 1);

// C enum nft_dynset_attributes
pub type nft_dynset_attributes = i32;
pub const NFTA_DYNSET_UNSPEC = 0;
pub const NFTA_DYNSET_SET_NAME = 1;
pub const NFTA_DYNSET_SET_ID = 2;
pub const NFTA_DYNSET_OP = 3;
pub const NFTA_DYNSET_SREG_KEY = 4;
pub const NFTA_DYNSET_SREG_DATA = 5;
pub const NFTA_DYNSET_TIMEOUT = 6;
pub const NFTA_DYNSET_EXPR = 7;
pub const NFTA_DYNSET_PAD = 8;
pub const NFTA_DYNSET_FLAGS = 9;
pub const NFTA_DYNSET_EXPRESSIONS = 10;
pub const __NFTA_DYNSET_MAX = 11;

pub const NFTA_DYNSET_MAX = (__NFTA_DYNSET_MAX - 1);
// C enum nft_payload_bases
pub type nft_payload_bases = i32;
pub const NFT_PAYLOAD_LL_HEADER = 0;
pub const NFT_PAYLOAD_NETWORK_HEADER = 1;
pub const NFT_PAYLOAD_TRANSPORT_HEADER = 2;
pub const NFT_PAYLOAD_INNER_HEADER = 3;
pub const NFT_PAYLOAD_TUN_HEADER = 4;

// C enum nft_payload_csum_types
pub type nft_payload_csum_types = i32;
pub const NFT_PAYLOAD_CSUM_NONE = 0;
pub const NFT_PAYLOAD_CSUM_INET = 1;
pub const NFT_PAYLOAD_CSUM_SCTP = 2;

// C enum nft_payload_csum_flags
pub type nft_payload_csum_flags = i32;
pub const NFT_PAYLOAD_L4CSUM_PSEUDOHDR = (1 << 0);

// C enum nft_inner_type
pub type nft_inner_type = i32;
pub const NFT_INNER_UNSPEC = 0;
pub const NFT_INNER_VXLAN = 1;
pub const NFT_INNER_GENEVE = 2;

// C enum nft_inner_flags
pub type nft_inner_flags = i32;
pub const NFT_INNER_HDRSIZE = (1 << 0);
pub const NFT_INNER_LL = (1 << 1);
pub const NFT_INNER_NH = (1 << 2);
pub const NFT_INNER_TH = (1 << 3);

pub const NFT_INNER_MASK = (NFT_INNER_HDRSIZE | NFT_INNER_LL |  NFT_INNER_NH | NFT_INNER_TH);
// C enum nft_inner_attributes
pub type nft_inner_attributes = i32;
pub const NFTA_INNER_UNSPEC = 0;
pub const NFTA_INNER_NUM = 1;
pub const NFTA_INNER_TYPE = 2;
pub const NFTA_INNER_FLAGS = 3;
pub const NFTA_INNER_HDRSIZE = 4;
pub const NFTA_INNER_EXPR = 5;
pub const __NFTA_INNER_MAX = 6;

pub const NFTA_INNER_MAX = (__NFTA_INNER_MAX - 1);
// C enum nft_payload_attributes
pub type nft_payload_attributes = i32;
pub const NFTA_PAYLOAD_UNSPEC = 0;
pub const NFTA_PAYLOAD_DREG = 1;
pub const NFTA_PAYLOAD_BASE = 2;
pub const NFTA_PAYLOAD_OFFSET = 3;
pub const NFTA_PAYLOAD_LEN = 4;
pub const NFTA_PAYLOAD_SREG = 5;
pub const NFTA_PAYLOAD_CSUM_TYPE = 6;
pub const NFTA_PAYLOAD_CSUM_OFFSET = 7;
pub const NFTA_PAYLOAD_CSUM_FLAGS = 8;
pub const __NFTA_PAYLOAD_MAX = 9;

pub const NFTA_PAYLOAD_MAX = (__NFTA_PAYLOAD_MAX - 1);
// C enum nft_exthdr_flags
pub type nft_exthdr_flags = i32;
pub const NFT_EXTHDR_F_PRESENT = (1 << 0);

// C enum nft_exthdr_op
pub type nft_exthdr_op = i32;
pub const NFT_EXTHDR_OP_IPV6 = 0;
pub const NFT_EXTHDR_OP_TCPOPT = 1;
pub const NFT_EXTHDR_OP_IPV4 = 2;
pub const NFT_EXTHDR_OP_SCTP = 3;
pub const NFT_EXTHDR_OP_DCCP = 4;
pub const __NFT_EXTHDR_OP_MAX = 5;

pub const NFT_EXTHDR_OP_MAX = (__NFT_EXTHDR_OP_MAX - 1);
// C enum nft_exthdr_attributes
pub type nft_exthdr_attributes = i32;
pub const NFTA_EXTHDR_UNSPEC = 0;
pub const NFTA_EXTHDR_DREG = 1;
pub const NFTA_EXTHDR_TYPE = 2;
pub const NFTA_EXTHDR_OFFSET = 3;
pub const NFTA_EXTHDR_LEN = 4;
pub const NFTA_EXTHDR_FLAGS = 5;
pub const NFTA_EXTHDR_OP = 6;
pub const NFTA_EXTHDR_SREG = 7;
pub const __NFTA_EXTHDR_MAX = 8;

pub const NFTA_EXTHDR_MAX = (__NFTA_EXTHDR_MAX - 1);
// C enum nft_meta_keys
pub type nft_meta_keys = i32;
pub const NFT_META_LEN = 0;
pub const NFT_META_PROTOCOL = 1;
pub const NFT_META_PRIORITY = 2;
pub const NFT_META_MARK = 3;
pub const NFT_META_IIF = 4;
pub const NFT_META_OIF = 5;
pub const NFT_META_IIFNAME = 6;
pub const NFT_META_OIFNAME = 7;
pub const NFT_META_IFTYPE = 8;
pub const NFT_META_IIFTYPE = NFT_META_IFTYPE;
pub const NFT_META_OIFTYPE = 9;
pub const NFT_META_SKUID = 10;
pub const NFT_META_SKGID = 11;
pub const NFT_META_NFTRACE = 12;
pub const NFT_META_RTCLASSID = 13;
pub const NFT_META_SECMARK = 14;
pub const NFT_META_NFPROTO = 15;
pub const NFT_META_L4PROTO = 16;
pub const NFT_META_BRI_IIFNAME = 17;
pub const NFT_META_BRI_OIFNAME = 18;
pub const NFT_META_PKTTYPE = 19;
pub const NFT_META_CPU = 20;
pub const NFT_META_IIFGROUP = 21;
pub const NFT_META_OIFGROUP = 22;
pub const NFT_META_CGROUP = 23;
pub const NFT_META_PRANDOM = 24;
pub const NFT_META_SECPATH = 25;
pub const NFT_META_IIFKIND = 26;
pub const NFT_META_OIFKIND = 27;
pub const NFT_META_BRI_IIFPVID = 28;
pub const NFT_META_BRI_IIFVPROTO = 29;
pub const NFT_META_TIME_NS = 30;
pub const NFT_META_TIME_DAY = 31;
pub const NFT_META_TIME_HOUR = 32;
pub const NFT_META_SDIF = 33;
pub const NFT_META_SDIFNAME = 34;
pub const NFT_META_BRI_BROUTE = 35;
pub const __NFT_META_IIFTYPE = 36;
pub const NFT_META_BRI_IIFHWADDR = 37;

// C enum nft_rt_keys
pub type nft_rt_keys = i32;
pub const NFT_RT_CLASSID = 0;
pub const NFT_RT_NEXTHOP4 = 1;
pub const NFT_RT_NEXTHOP6 = 2;
pub const NFT_RT_TCPMSS = 3;
pub const NFT_RT_XFRM = 4;
pub const __NFT_RT_MAX = 5;

pub const NFT_RT_MAX = (__NFT_RT_MAX - 1);
// C enum nft_hash_types
pub type nft_hash_types = i32;
pub const NFT_HASH_JENKINS = 0;
pub const NFT_HASH_SYM = 1;

// C enum nft_hash_attributes
pub type nft_hash_attributes = i32;
pub const NFTA_HASH_UNSPEC = 0;
pub const NFTA_HASH_SREG = 1;
pub const NFTA_HASH_DREG = 2;
pub const NFTA_HASH_LEN = 3;
pub const NFTA_HASH_MODULUS = 4;
pub const NFTA_HASH_SEED = 5;
pub const NFTA_HASH_OFFSET = 6;
pub const NFTA_HASH_TYPE = 7;
pub const NFTA_HASH_SET_NAME = 8;
pub const NFTA_HASH_SET_ID = 9;
pub const __NFTA_HASH_MAX = 10;

pub const NFTA_HASH_MAX = (__NFTA_HASH_MAX - 1);
// C enum nft_meta_attributes
pub type nft_meta_attributes = i32;
pub const NFTA_META_UNSPEC = 0;
pub const NFTA_META_DREG = 1;
pub const NFTA_META_KEY = 2;
pub const NFTA_META_SREG = 3;
pub const __NFTA_META_MAX = 4;

pub const NFTA_META_MAX = (__NFTA_META_MAX - 1);
// C enum nft_rt_attributes
pub type nft_rt_attributes = i32;
pub const NFTA_RT_UNSPEC = 0;
pub const NFTA_RT_DREG = 1;
pub const NFTA_RT_KEY = 2;
pub const __NFTA_RT_MAX = 3;

pub const NFTA_RT_MAX = (__NFTA_RT_MAX - 1);
// C enum nft_socket_attributes
pub type nft_socket_attributes = i32;
pub const NFTA_SOCKET_UNSPEC = 0;
pub const NFTA_SOCKET_KEY = 1;
pub const NFTA_SOCKET_DREG = 2;
pub const NFTA_SOCKET_LEVEL = 3;
pub const __NFTA_SOCKET_MAX = 4;

pub const NFTA_SOCKET_MAX = (__NFTA_SOCKET_MAX - 1);
// C enum nft_socket_keys
pub type nft_socket_keys = i32;
pub const NFT_SOCKET_TRANSPARENT = 0;
pub const NFT_SOCKET_MARK = 1;
pub const NFT_SOCKET_WILDCARD = 2;
pub const NFT_SOCKET_CGROUPV2 = 3;
pub const __NFT_SOCKET_MAX = 4;

pub const NFT_SOCKET_MAX = (__NFT_SOCKET_MAX - 1);
// C enum nft_ct_keys
pub type nft_ct_keys = i32;
pub const NFT_CT_STATE = 0;
pub const NFT_CT_DIRECTION = 1;
pub const NFT_CT_STATUS = 2;
pub const NFT_CT_MARK = 3;
pub const NFT_CT_SECMARK = 4;
pub const NFT_CT_EXPIRATION = 5;
pub const NFT_CT_HELPER = 6;
pub const NFT_CT_L3PROTOCOL = 7;
pub const NFT_CT_SRC = 8;
pub const NFT_CT_DST = 9;
pub const NFT_CT_PROTOCOL = 10;
pub const NFT_CT_PROTO_SRC = 11;
pub const NFT_CT_PROTO_DST = 12;
pub const NFT_CT_LABELS = 13;
pub const NFT_CT_PKTS = 14;
pub const NFT_CT_BYTES = 15;
pub const NFT_CT_AVGPKT = 16;
pub const NFT_CT_ZONE = 17;
pub const NFT_CT_EVENTMASK = 18;
pub const NFT_CT_SRC_IP = 19;
pub const NFT_CT_DST_IP = 20;
pub const NFT_CT_SRC_IP6 = 21;
pub const NFT_CT_DST_IP6 = 22;
pub const NFT_CT_ID = 23;
pub const __NFT_CT_MAX = 24;

pub const NFT_CT_MAX = (__NFT_CT_MAX - 1);
// C enum nft_ct_attributes
pub type nft_ct_attributes = i32;
pub const NFTA_CT_UNSPEC = 0;
pub const NFTA_CT_DREG = 1;
pub const NFTA_CT_KEY = 2;
pub const NFTA_CT_DIRECTION = 3;
pub const NFTA_CT_SREG = 4;
pub const __NFTA_CT_MAX = 5;

pub const NFTA_CT_MAX = (__NFTA_CT_MAX - 1);
// C enum nft_offload_attributes
pub type nft_offload_attributes = i32;
pub const NFTA_FLOW_UNSPEC = 0;
pub const NFTA_FLOW_TABLE_NAME = 1;
pub const __NFTA_FLOW_MAX = 2;

pub const NFTA_FLOW_MAX = (__NFTA_FLOW_MAX - 1);
// C enum nft_limit_type
pub type nft_limit_type = i32;
pub const NFT_LIMIT_PKTS = 0;
pub const NFT_LIMIT_PKT_BYTES = 1;

// C enum nft_limit_flags
pub type nft_limit_flags = i32;
pub const NFT_LIMIT_F_INV = (1 << 0);

// C enum nft_limit_attributes
pub type nft_limit_attributes = i32;
pub const NFTA_LIMIT_UNSPEC = 0;
pub const NFTA_LIMIT_RATE = 1;
pub const NFTA_LIMIT_UNIT = 2;
pub const NFTA_LIMIT_BURST = 3;
pub const NFTA_LIMIT_TYPE = 4;
pub const NFTA_LIMIT_FLAGS = 5;
pub const NFTA_LIMIT_PAD = 6;
pub const __NFTA_LIMIT_MAX = 7;

pub const NFTA_LIMIT_MAX = (__NFTA_LIMIT_MAX - 1);
// C enum nft_connlimit_flags
pub type nft_connlimit_flags = i32;
pub const NFT_CONNLIMIT_F_INV = (1 << 0);

// C enum nft_connlimit_attributes
pub type nft_connlimit_attributes = i32;
pub const NFTA_CONNLIMIT_UNSPEC = 0;
pub const NFTA_CONNLIMIT_COUNT = 1;
pub const NFTA_CONNLIMIT_FLAGS = 2;
pub const __NFTA_CONNLIMIT_MAX = 3;

pub const NFTA_CONNLIMIT_MAX = (__NFTA_CONNLIMIT_MAX - 1);
// C enum nft_counter_attributes
pub type nft_counter_attributes = i32;
pub const NFTA_COUNTER_UNSPEC = 0;
pub const NFTA_COUNTER_BYTES = 1;
pub const NFTA_COUNTER_PACKETS = 2;
pub const NFTA_COUNTER_PAD = 3;
pub const __NFTA_COUNTER_MAX = 4;

pub const NFTA_COUNTER_MAX = (__NFTA_COUNTER_MAX - 1);
// C enum nft_last_attributes
pub type nft_last_attributes = i32;
pub const NFTA_LAST_UNSPEC = 0;
pub const NFTA_LAST_SET = 1;
pub const NFTA_LAST_MSECS = 2;
pub const NFTA_LAST_PAD = 3;
pub const __NFTA_LAST_MAX = 4;

pub const NFTA_LAST_MAX = (__NFTA_LAST_MAX - 1);
// C enum nft_log_attributes
pub type nft_log_attributes = i32;
pub const NFTA_LOG_UNSPEC = 0;
pub const NFTA_LOG_GROUP = 1;
pub const NFTA_LOG_PREFIX = 2;
pub const NFTA_LOG_SNAPLEN = 3;
pub const NFTA_LOG_QTHRESHOLD = 4;
pub const NFTA_LOG_LEVEL = 5;
pub const NFTA_LOG_FLAGS = 6;
pub const __NFTA_LOG_MAX = 7;

pub const NFTA_LOG_MAX = (__NFTA_LOG_MAX - 1);
// C enum nft_log_level
pub type nft_log_level = i32;
pub const NFT_LOGLEVEL_EMERG = 0;
pub const NFT_LOGLEVEL_ALERT = 1;
pub const NFT_LOGLEVEL_CRIT = 2;
pub const NFT_LOGLEVEL_ERR = 3;
pub const NFT_LOGLEVEL_WARNING = 4;
pub const NFT_LOGLEVEL_NOTICE = 5;
pub const NFT_LOGLEVEL_INFO = 6;
pub const NFT_LOGLEVEL_DEBUG = 7;
pub const NFT_LOGLEVEL_AUDIT = 8;
pub const __NFT_LOGLEVEL_MAX = 9;

pub const NFT_LOGLEVEL_MAX = (__NFT_LOGLEVEL_MAX - 1);
// C enum nft_queue_attributes
pub type nft_queue_attributes = i32;
pub const NFTA_QUEUE_UNSPEC = 0;
pub const NFTA_QUEUE_NUM = 1;
pub const NFTA_QUEUE_TOTAL = 2;
pub const NFTA_QUEUE_FLAGS = 3;
pub const NFTA_QUEUE_SREG_QNUM = 4;
pub const __NFTA_QUEUE_MAX = 5;

pub const NFTA_QUEUE_MAX = (__NFTA_QUEUE_MAX - 1);
pub const NFT_QUEUE_FLAG_BYPASS = 0x01;
pub const NFT_QUEUE_FLAG_CPU_FANOUT = 0x02;
pub const NFT_QUEUE_FLAG_MASK = 0x03;
// C enum nft_quota_flags
pub type nft_quota_flags = i32;
pub const NFT_QUOTA_F_INV = (1 << 0);
pub const NFT_QUOTA_F_DEPLETED = (1 << 1);

// C enum nft_quota_attributes
pub type nft_quota_attributes = i32;
pub const NFTA_QUOTA_UNSPEC = 0;
pub const NFTA_QUOTA_BYTES = 1;
pub const NFTA_QUOTA_FLAGS = 2;
pub const NFTA_QUOTA_PAD = 3;
pub const NFTA_QUOTA_CONSUMED = 4;
pub const __NFTA_QUOTA_MAX = 5;

pub const NFTA_QUOTA_MAX = (__NFTA_QUOTA_MAX - 1);
// C enum nft_secmark_attributes
pub type nft_secmark_attributes = i32;
pub const NFTA_SECMARK_UNSPEC = 0;
pub const NFTA_SECMARK_CTX = 1;
pub const __NFTA_SECMARK_MAX = 2;

pub const NFTA_SECMARK_MAX = (__NFTA_SECMARK_MAX - 1);
pub const NFT_SECMARK_CTX_MAXLEN = 4096;
// C enum nft_reject_types
pub type nft_reject_types = i32;
pub const NFT_REJECT_ICMP_UNREACH = 0;
pub const NFT_REJECT_TCP_RST = 1;
pub const NFT_REJECT_ICMPX_UNREACH = 2;

// C enum nft_reject_inet_code
pub type nft_reject_inet_code = i32;
pub const NFT_REJECT_ICMPX_NO_ROUTE = 0;
pub const NFT_REJECT_ICMPX_PORT_UNREACH = 1;
pub const NFT_REJECT_ICMPX_HOST_UNREACH = 2;
pub const NFT_REJECT_ICMPX_ADMIN_PROHIBITED = 3;
pub const __NFT_REJECT_ICMPX_MAX = 4;

pub const NFT_REJECT_ICMPX_MAX = (__NFT_REJECT_ICMPX_MAX - 1);
// C enum nft_reject_attributes
pub type nft_reject_attributes = i32;
pub const NFTA_REJECT_UNSPEC = 0;
pub const NFTA_REJECT_TYPE = 1;
pub const NFTA_REJECT_ICMP_CODE = 2;
pub const __NFTA_REJECT_MAX = 3;

pub const NFTA_REJECT_MAX = (__NFTA_REJECT_MAX - 1);
// C enum nft_nat_types
pub type nft_nat_types = i32;
pub const NFT_NAT_SNAT = 0;
pub const NFT_NAT_DNAT = 1;

// C enum nft_nat_attributes
pub type nft_nat_attributes = i32;
pub const NFTA_NAT_UNSPEC = 0;
pub const NFTA_NAT_TYPE = 1;
pub const NFTA_NAT_FAMILY = 2;
pub const NFTA_NAT_REG_ADDR_MIN = 3;
pub const NFTA_NAT_REG_ADDR_MAX = 4;
pub const NFTA_NAT_REG_PROTO_MIN = 5;
pub const NFTA_NAT_REG_PROTO_MAX = 6;
pub const NFTA_NAT_FLAGS = 7;
pub const __NFTA_NAT_MAX = 8;

pub const NFTA_NAT_MAX = (__NFTA_NAT_MAX - 1);
// C enum nft_tproxy_attributes
pub type nft_tproxy_attributes = i32;
pub const NFTA_TPROXY_UNSPEC = 0;
pub const NFTA_TPROXY_FAMILY = 1;
pub const NFTA_TPROXY_REG_ADDR = 2;
pub const NFTA_TPROXY_REG_PORT = 3;
pub const __NFTA_TPROXY_MAX = 4;

pub const NFTA_TPROXY_MAX = (__NFTA_TPROXY_MAX - 1);
// C enum nft_masq_attributes
pub type nft_masq_attributes = i32;
pub const NFTA_MASQ_UNSPEC = 0;
pub const NFTA_MASQ_FLAGS = 1;
pub const NFTA_MASQ_REG_PROTO_MIN = 2;
pub const NFTA_MASQ_REG_PROTO_MAX = 3;
pub const __NFTA_MASQ_MAX = 4;

pub const NFTA_MASQ_MAX = (__NFTA_MASQ_MAX - 1);
// C enum nft_redir_attributes
pub type nft_redir_attributes = i32;
pub const NFTA_REDIR_UNSPEC = 0;
pub const NFTA_REDIR_REG_PROTO_MIN = 1;
pub const NFTA_REDIR_REG_PROTO_MAX = 2;
pub const NFTA_REDIR_FLAGS = 3;
pub const __NFTA_REDIR_MAX = 4;

pub const NFTA_REDIR_MAX = (__NFTA_REDIR_MAX - 1);
// C enum nft_dup_attributes
pub type nft_dup_attributes = i32;
pub const NFTA_DUP_UNSPEC = 0;
pub const NFTA_DUP_SREG_ADDR = 1;
pub const NFTA_DUP_SREG_DEV = 2;
pub const __NFTA_DUP_MAX = 3;

pub const NFTA_DUP_MAX = (__NFTA_DUP_MAX - 1);
// C enum nft_fwd_attributes
pub type nft_fwd_attributes = i32;
pub const NFTA_FWD_UNSPEC = 0;
pub const NFTA_FWD_SREG_DEV = 1;
pub const NFTA_FWD_SREG_ADDR = 2;
pub const NFTA_FWD_NFPROTO = 3;
pub const __NFTA_FWD_MAX = 4;

pub const NFTA_FWD_MAX = (__NFTA_FWD_MAX - 1);
// C enum nft_objref_attributes
pub type nft_objref_attributes = i32;
pub const NFTA_OBJREF_UNSPEC = 0;
pub const NFTA_OBJREF_IMM_TYPE = 1;
pub const NFTA_OBJREF_IMM_NAME = 2;
pub const NFTA_OBJREF_SET_SREG = 3;
pub const NFTA_OBJREF_SET_NAME = 4;
pub const NFTA_OBJREF_SET_ID = 5;
pub const __NFTA_OBJREF_MAX = 6;

pub const NFTA_OBJREF_MAX = (__NFTA_OBJREF_MAX - 1);
// C enum nft_gen_attributes
pub type nft_gen_attributes = i32;
pub const NFTA_GEN_UNSPEC = 0;
pub const NFTA_GEN_ID = 1;
pub const NFTA_GEN_PROC_PID = 2;
pub const NFTA_GEN_PROC_NAME = 3;
pub const __NFTA_GEN_MAX = 4;

pub const NFTA_GEN_MAX = (__NFTA_GEN_MAX - 1);
// C enum nft_fib_attributes
pub type nft_fib_attributes = i32;
pub const NFTA_FIB_UNSPEC = 0;
pub const NFTA_FIB_DREG = 1;
pub const NFTA_FIB_RESULT = 2;
pub const NFTA_FIB_FLAGS = 3;
pub const __NFTA_FIB_MAX = 4;

pub const NFTA_FIB_MAX = (__NFTA_FIB_MAX - 1);
// C enum nft_fib_result
pub type nft_fib_result = i32;
pub const NFT_FIB_RESULT_UNSPEC = 0;
pub const NFT_FIB_RESULT_OIF = 1;
pub const NFT_FIB_RESULT_OIFNAME = 2;
pub const NFT_FIB_RESULT_ADDRTYPE = 3;
pub const __NFT_FIB_RESULT_MAX = 4;

pub const NFT_FIB_RESULT_MAX = (__NFT_FIB_RESULT_MAX - 1);
// C enum nft_fib_flags
pub type nft_fib_flags = i32;
pub const NFTA_FIB_F_SADDR = 1 << 0;
pub const NFTA_FIB_F_DADDR = 1 << 1;
pub const NFTA_FIB_F_MARK = 1 << 2;
pub const NFTA_FIB_F_IIF = 1 << 3;
pub const NFTA_FIB_F_OIF = 1 << 4;
pub const NFTA_FIB_F_PRESENT = 1 << 5;

// C enum nft_ct_helper_attributes
pub type nft_ct_helper_attributes = i32;
pub const NFTA_CT_HELPER_UNSPEC = 0;
pub const NFTA_CT_HELPER_NAME = 1;
pub const NFTA_CT_HELPER_L3PROTO = 2;
pub const NFTA_CT_HELPER_L4PROTO = 3;
pub const __NFTA_CT_HELPER_MAX = 4;

pub const NFTA_CT_HELPER_MAX = (__NFTA_CT_HELPER_MAX - 1);
// C enum nft_ct_timeout_timeout_attributes
pub type nft_ct_timeout_timeout_attributes = i32;
pub const NFTA_CT_TIMEOUT_UNSPEC = 0;
pub const NFTA_CT_TIMEOUT_L3PROTO = 1;
pub const NFTA_CT_TIMEOUT_L4PROTO = 2;
pub const NFTA_CT_TIMEOUT_DATA = 3;
pub const __NFTA_CT_TIMEOUT_MAX = 4;

pub const NFTA_CT_TIMEOUT_MAX = (__NFTA_CT_TIMEOUT_MAX - 1);
// C enum nft_ct_expectation_attributes
pub type nft_ct_expectation_attributes = i32;
pub const NFTA_CT_EXPECT_UNSPEC = 0;
pub const NFTA_CT_EXPECT_L3PROTO = 1;
pub const NFTA_CT_EXPECT_L4PROTO = 2;
pub const NFTA_CT_EXPECT_DPORT = 3;
pub const NFTA_CT_EXPECT_TIMEOUT = 4;
pub const NFTA_CT_EXPECT_SIZE = 5;
pub const __NFTA_CT_EXPECT_MAX = 6;

pub const NFTA_CT_EXPECT_MAX = (__NFTA_CT_EXPECT_MAX - 1);
pub const NFT_OBJECT_UNSPEC = 0;
pub const NFT_OBJECT_COUNTER = 1;
pub const NFT_OBJECT_QUOTA = 2;
pub const NFT_OBJECT_CT_HELPER = 3;
pub const NFT_OBJECT_LIMIT = 4;
pub const NFT_OBJECT_CONNLIMIT = 5;
pub const NFT_OBJECT_TUNNEL = 6;
pub const NFT_OBJECT_CT_TIMEOUT = 7;
pub const NFT_OBJECT_SECMARK = 8;
pub const NFT_OBJECT_CT_EXPECT = 9;
pub const NFT_OBJECT_SYNPROXY = 10;
pub const __NFT_OBJECT_MAX = 11;
pub const NFT_OBJECT_MAX = (__NFT_OBJECT_MAX - 1);
// C enum nft_object_attributes
pub type nft_object_attributes = i32;
pub const NFTA_OBJ_UNSPEC = 0;
pub const NFTA_OBJ_TABLE = 1;
pub const NFTA_OBJ_NAME = 2;
pub const NFTA_OBJ_TYPE = 3;
pub const NFTA_OBJ_DATA = 4;
pub const NFTA_OBJ_USE = 5;
pub const NFTA_OBJ_HANDLE = 6;
pub const NFTA_OBJ_PAD = 7;
pub const NFTA_OBJ_USERDATA = 8;
pub const __NFTA_OBJ_MAX = 9;

pub const NFTA_OBJ_MAX = (__NFTA_OBJ_MAX - 1);
// C enum nft_flowtable_flags
pub type nft_flowtable_flags = i32;
pub const NFT_FLOWTABLE_HW_OFFLOAD = 0x1;
pub const NFT_FLOWTABLE_COUNTER = 0x2;
pub const NFT_FLOWTABLE_MASK = (NFT_FLOWTABLE_HW_OFFLOAD | NFT_FLOWTABLE_COUNTER);

// C enum nft_flowtable_attributes
pub type nft_flowtable_attributes = i32;
pub const NFTA_FLOWTABLE_UNSPEC = 0;
pub const NFTA_FLOWTABLE_TABLE = 1;
pub const NFTA_FLOWTABLE_NAME = 2;
pub const NFTA_FLOWTABLE_HOOK = 3;
pub const NFTA_FLOWTABLE_USE = 4;
pub const NFTA_FLOWTABLE_HANDLE = 5;
pub const NFTA_FLOWTABLE_PAD = 6;
pub const NFTA_FLOWTABLE_FLAGS = 7;
pub const __NFTA_FLOWTABLE_MAX = 8;

pub const NFTA_FLOWTABLE_MAX = (__NFTA_FLOWTABLE_MAX - 1);
// C enum nft_flowtable_hook_attributes
pub type nft_flowtable_hook_attributes = i32;
pub const NFTA_FLOWTABLE_HOOK_UNSPEC = 0;
pub const NFTA_FLOWTABLE_HOOK_NUM = 1;
pub const NFTA_FLOWTABLE_HOOK_PRIORITY = 2;
pub const NFTA_FLOWTABLE_HOOK_DEVS = 3;
pub const __NFTA_FLOWTABLE_HOOK_MAX = 4;

pub const NFTA_FLOWTABLE_HOOK_MAX = (__NFTA_FLOWTABLE_HOOK_MAX - 1);
// C enum nft_osf_attributes
pub type nft_osf_attributes = i32;
pub const NFTA_OSF_UNSPEC = 0;
pub const NFTA_OSF_DREG = 1;
pub const NFTA_OSF_TTL = 2;
pub const NFTA_OSF_FLAGS = 3;
pub const __NFTA_OSF_MAX = 4;

pub const NFTA_OSF_MAX = (__NFTA_OSF_MAX - 1);
// C enum nft_osf_flags
pub type nft_osf_flags = i32;
pub const NFT_OSF_F_VERSION = (1 << 0);

// C enum nft_synproxy_attributes
pub type nft_synproxy_attributes = i32;
pub const NFTA_SYNPROXY_UNSPEC = 0;
pub const NFTA_SYNPROXY_MSS = 1;
pub const NFTA_SYNPROXY_WSCALE = 2;
pub const NFTA_SYNPROXY_FLAGS = 3;
pub const __NFTA_SYNPROXY_MAX = 4;

pub const NFTA_SYNPROXY_MAX = (__NFTA_SYNPROXY_MAX - 1);
// C enum nft_devices_attributes
pub type nft_devices_attributes = i32;
pub const NFTA_DEVICE_UNSPEC = 0;
pub const NFTA_DEVICE_NAME = 1;
pub const NFTA_DEVICE_PREFIX = 2;
pub const __NFTA_DEVICE_MAX = 3;

pub const NFTA_DEVICE_MAX = (__NFTA_DEVICE_MAX - 1);
// C enum nft_xfrm_attributes
pub type nft_xfrm_attributes = i32;
pub const NFTA_XFRM_UNSPEC = 0;
pub const NFTA_XFRM_DREG = 1;
pub const NFTA_XFRM_KEY = 2;
pub const NFTA_XFRM_DIR = 3;
pub const NFTA_XFRM_SPNUM = 4;
pub const __NFTA_XFRM_MAX = 5;

pub const NFTA_XFRM_MAX = (__NFTA_XFRM_MAX - 1);
// C enum nft_xfrm_keys
pub type nft_xfrm_keys = i32;
pub const NFT_XFRM_KEY_UNSPEC = 0;
pub const NFT_XFRM_KEY_DADDR_IP4 = 1;
pub const NFT_XFRM_KEY_DADDR_IP6 = 2;
pub const NFT_XFRM_KEY_SADDR_IP4 = 3;
pub const NFT_XFRM_KEY_SADDR_IP6 = 4;
pub const NFT_XFRM_KEY_REQID = 5;
pub const NFT_XFRM_KEY_SPI = 6;
pub const __NFT_XFRM_KEY_MAX = 7;

pub const NFT_XFRM_KEY_MAX = (__NFT_XFRM_KEY_MAX - 1);
// C enum nft_trace_attributes
pub type nft_trace_attributes = i32;
pub const NFTA_TRACE_UNSPEC = 0;
pub const NFTA_TRACE_TABLE = 1;
pub const NFTA_TRACE_CHAIN = 2;
pub const NFTA_TRACE_RULE_HANDLE = 3;
pub const NFTA_TRACE_TYPE = 4;
pub const NFTA_TRACE_VERDICT = 5;
pub const NFTA_TRACE_ID = 6;
pub const NFTA_TRACE_LL_HEADER = 7;
pub const NFTA_TRACE_NETWORK_HEADER = 8;
pub const NFTA_TRACE_TRANSPORT_HEADER = 9;
pub const NFTA_TRACE_IIF = 10;
pub const NFTA_TRACE_IIFTYPE = 11;
pub const NFTA_TRACE_OIF = 12;
pub const NFTA_TRACE_OIFTYPE = 13;
pub const NFTA_TRACE_MARK = 14;
pub const NFTA_TRACE_NFPROTO = 15;
pub const NFTA_TRACE_POLICY = 16;
pub const NFTA_TRACE_PAD = 17;
pub const NFTA_TRACE_CT_ID = 18;
pub const NFTA_TRACE_CT_DIRECTION = 19;
pub const NFTA_TRACE_CT_STATUS = 20;
pub const NFTA_TRACE_CT_STATE = 21;
pub const __NFTA_TRACE_MAX = 22;

pub const NFTA_TRACE_MAX = (__NFTA_TRACE_MAX - 1);
// C enum nft_trace_types
pub type nft_trace_types = i32;
pub const NFT_TRACETYPE_UNSPEC = 0;
pub const NFT_TRACETYPE_POLICY = 1;
pub const NFT_TRACETYPE_RETURN = 2;
pub const NFT_TRACETYPE_RULE = 3;
pub const __NFT_TRACETYPE_MAX = 4;

pub const NFT_TRACETYPE_MAX = (__NFT_TRACETYPE_MAX - 1);
// C enum nft_ng_attributes
pub type nft_ng_attributes = i32;
pub const NFTA_NG_UNSPEC = 0;
pub const NFTA_NG_DREG = 1;
pub const NFTA_NG_MODULUS = 2;
pub const NFTA_NG_TYPE = 3;
pub const NFTA_NG_OFFSET = 4;
pub const NFTA_NG_SET_NAME = 5;
pub const NFTA_NG_SET_ID = 6;
pub const __NFTA_NG_MAX = 7;

pub const NFTA_NG_MAX = (__NFTA_NG_MAX - 1);
// C enum nft_ng_types
pub type nft_ng_types = i32;
pub const NFT_NG_INCREMENTAL = 0;
pub const NFT_NG_RANDOM = 1;
pub const __NFT_NG_MAX = 2;

pub const NFT_NG_MAX = (__NFT_NG_MAX - 1);
// C enum nft_tunnel_key_ip_attributes
pub type nft_tunnel_key_ip_attributes = i32;
pub const NFTA_TUNNEL_KEY_IP_UNSPEC = 0;
pub const NFTA_TUNNEL_KEY_IP_SRC = 1;
pub const NFTA_TUNNEL_KEY_IP_DST = 2;
pub const __NFTA_TUNNEL_KEY_IP_MAX = 3;

pub const NFTA_TUNNEL_KEY_IP_MAX = (__NFTA_TUNNEL_KEY_IP_MAX - 1);
// C enum nft_tunnel_ip6_attributes
pub type nft_tunnel_ip6_attributes = i32;
pub const NFTA_TUNNEL_KEY_IP6_UNSPEC = 0;
pub const NFTA_TUNNEL_KEY_IP6_SRC = 1;
pub const NFTA_TUNNEL_KEY_IP6_DST = 2;
pub const NFTA_TUNNEL_KEY_IP6_FLOWLABEL = 3;
pub const __NFTA_TUNNEL_KEY_IP6_MAX = 4;

pub const NFTA_TUNNEL_KEY_IP6_MAX = (__NFTA_TUNNEL_KEY_IP6_MAX - 1);
// C enum nft_tunnel_opts_attributes
pub type nft_tunnel_opts_attributes = i32;
pub const NFTA_TUNNEL_KEY_OPTS_UNSPEC = 0;
pub const NFTA_TUNNEL_KEY_OPTS_VXLAN = 1;
pub const NFTA_TUNNEL_KEY_OPTS_ERSPAN = 2;
pub const NFTA_TUNNEL_KEY_OPTS_GENEVE = 3;
pub const __NFTA_TUNNEL_KEY_OPTS_MAX = 4;

pub const NFTA_TUNNEL_KEY_OPTS_MAX = (__NFTA_TUNNEL_KEY_OPTS_MAX - 1);
// C enum nft_tunnel_opts_vxlan_attributes
pub type nft_tunnel_opts_vxlan_attributes = i32;
pub const NFTA_TUNNEL_KEY_VXLAN_UNSPEC = 0;
pub const NFTA_TUNNEL_KEY_VXLAN_GBP = 1;
pub const __NFTA_TUNNEL_KEY_VXLAN_MAX = 2;

pub const NFTA_TUNNEL_KEY_VXLAN_MAX = (__NFTA_TUNNEL_KEY_VXLAN_MAX - 1);
// C enum nft_tunnel_opts_erspan_attributes
pub type nft_tunnel_opts_erspan_attributes = i32;
pub const NFTA_TUNNEL_KEY_ERSPAN_UNSPEC = 0;
pub const NFTA_TUNNEL_KEY_ERSPAN_VERSION = 1;
pub const NFTA_TUNNEL_KEY_ERSPAN_V1_INDEX = 2;
pub const NFTA_TUNNEL_KEY_ERSPAN_V2_HWID = 3;
pub const NFTA_TUNNEL_KEY_ERSPAN_V2_DIR = 4;
pub const __NFTA_TUNNEL_KEY_ERSPAN_MAX = 5;

pub const NFTA_TUNNEL_KEY_ERSPAN_MAX = (__NFTA_TUNNEL_KEY_ERSPAN_MAX - 1);
// C enum nft_tunnel_opts_geneve_attributes
pub type nft_tunnel_opts_geneve_attributes = i32;
pub const NFTA_TUNNEL_KEY_GENEVE_UNSPEC = 0;
pub const NFTA_TUNNEL_KEY_GENEVE_CLASS = 1;
pub const NFTA_TUNNEL_KEY_GENEVE_TYPE = 2;
pub const NFTA_TUNNEL_KEY_GENEVE_DATA = 3;
pub const __NFTA_TUNNEL_KEY_GENEVE_MAX = 4;

pub const NFTA_TUNNEL_KEY_GENEVE_MAX = (__NFTA_TUNNEL_KEY_GENEVE_MAX - 1);
// C enum nft_tunnel_flags
pub type nft_tunnel_flags = i32;
pub const NFT_TUNNEL_F_ZERO_CSUM_TX = (1 << 0);
pub const NFT_TUNNEL_F_DONT_FRAGMENT = (1 << 1);
pub const NFT_TUNNEL_F_SEQ_NUMBER = (1 << 2);

pub const NFT_TUNNEL_F_MASK = (NFT_TUNNEL_F_ZERO_CSUM_TX |  NFT_TUNNEL_F_DONT_FRAGMENT |  NFT_TUNNEL_F_SEQ_NUMBER);
// C enum nft_tunnel_key_attributes
pub type nft_tunnel_key_attributes = i32;
pub const NFTA_TUNNEL_KEY_UNSPEC = 0;
pub const NFTA_TUNNEL_KEY_ID = 1;
pub const NFTA_TUNNEL_KEY_IP = 2;
pub const NFTA_TUNNEL_KEY_IP6 = 3;
pub const NFTA_TUNNEL_KEY_FLAGS = 4;
pub const NFTA_TUNNEL_KEY_TOS = 5;
pub const NFTA_TUNNEL_KEY_TTL = 6;
pub const NFTA_TUNNEL_KEY_SPORT = 7;
pub const NFTA_TUNNEL_KEY_DPORT = 8;
pub const NFTA_TUNNEL_KEY_OPTS = 9;
pub const __NFTA_TUNNEL_KEY_MAX = 10;

pub const NFTA_TUNNEL_KEY_MAX = (__NFTA_TUNNEL_KEY_MAX - 1);
// C enum nft_tunnel_keys
pub type nft_tunnel_keys = i32;
pub const NFT_TUNNEL_PATH = 0;
pub const NFT_TUNNEL_ID = 1;
pub const __NFT_TUNNEL_MAX = 2;

pub const NFT_TUNNEL_MAX = (__NFT_TUNNEL_MAX - 1);
// C enum nft_tunnel_mode
pub type nft_tunnel_mode = i32;
pub const NFT_TUNNEL_MODE_NONE = 0;
pub const NFT_TUNNEL_MODE_RX = 1;
pub const NFT_TUNNEL_MODE_TX = 2;
pub const __NFT_TUNNEL_MODE_MAX = 3;

pub const NFT_TUNNEL_MODE_MAX = (__NFT_TUNNEL_MODE_MAX - 1);
// C enum nft_tunnel_attributes
pub type nft_tunnel_attributes = i32;
pub const NFTA_TUNNEL_UNSPEC = 0;
pub const NFTA_TUNNEL_KEY = 1;
pub const NFTA_TUNNEL_DREG = 2;
pub const NFTA_TUNNEL_MODE = 3;
pub const __NFTA_TUNNEL_MAX = 4;

pub const NFTA_TUNNEL_MAX = (__NFTA_TUNNEL_MAX - 1);
