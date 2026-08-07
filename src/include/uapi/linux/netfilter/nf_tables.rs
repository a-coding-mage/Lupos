// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/netfilter/nf_tables.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016277

pub const NFT_NAME_MAXLEN: u32 = 256;
pub const NFT_TABLE_MAXNAMELEN: u32 = NFT_NAME_MAXLEN;
pub const NFT_CHAIN_MAXNAMELEN: u32 = NFT_NAME_MAXLEN;
pub const NFT_SET_MAXNAMELEN: u32 = NFT_NAME_MAXLEN;
pub const NFT_OBJ_MAXNAMELEN: u32 = NFT_NAME_MAXLEN;
pub const NFT_USERDATA_MAXLEN: u32 = 256;
pub const NFT_OSF_MAXGENRELEN: i32 = 16;

/**
 * enum nft_registers - nf_tables registers
 *
 * nf_tables used to have five registers: a verdict register and four data
 * registers of size 16. The data registers have been changed to 16 registers
 * of size 4. For compatibility reasons, the NFT_REG_[1-4] registers still
 * map to areas of size 16, the 4 byte registers are addressed using
 * NFT_REG32_00 - NFT_REG32_15.
 */
pub type nft_registers = i32;
pub const NFT_REG_VERDICT: nft_registers = 0;
pub const NFT_REG_1: nft_registers = 1;
pub const NFT_REG_2: nft_registers = 2;
pub const NFT_REG_3: nft_registers = 3;
pub const NFT_REG_4: nft_registers = 4;
pub const __NFT_REG_MAX: nft_registers = 5;
pub const NFT_REG32_00: nft_registers = 8;
pub const NFT_REG32_01: nft_registers = 9;
pub const NFT_REG32_02: nft_registers = 10;
pub const NFT_REG32_03: nft_registers = 11;
pub const NFT_REG32_04: nft_registers = 12;
pub const NFT_REG32_05: nft_registers = 13;
pub const NFT_REG32_06: nft_registers = 14;
pub const NFT_REG32_07: nft_registers = 15;
pub const NFT_REG32_08: nft_registers = 16;
pub const NFT_REG32_09: nft_registers = 17;
pub const NFT_REG32_10: nft_registers = 18;
pub const NFT_REG32_11: nft_registers = 19;
pub const NFT_REG32_12: nft_registers = 20;
pub const NFT_REG32_13: nft_registers = 21;
pub const NFT_REG32_14: nft_registers = 22;
pub const NFT_REG32_15: nft_registers = 23;

pub const NFT_REG_MAX: i32 = (__NFT_REG_MAX - 1);

pub const NFT_REG32_MAX: nft_registers = NFT_REG32_15;

pub const NFT_REG_SIZE: i32 = 16;
pub const NFT_REG32_SIZE: i32 = 4;
pub const NFT_REG32_COUNT: i32 = (NFT_REG32_15 - NFT_REG32_00 + 1);

/**
 * enum nft_verdicts - nf_tables internal verdicts
 *
 * @NFT_CONTINUE: continue evaluation of the current rule
 * @NFT_BREAK: terminate evaluation of the current rule
 * @NFT_JUMP: push the current chain on the jump stack and jump to a chain
 * @NFT_GOTO: jump to a chain without pushing the current chain on the jump stack
 * @NFT_RETURN: return to the topmost chain on the jump stack
 *
 * The nf_tables verdicts share their numeric space with the netfilter verdicts.
 */
pub type nft_verdicts = i32;
pub const NFT_CONTINUE: nft_verdicts = -1;
pub const NFT_BREAK: nft_verdicts = -2;
pub const NFT_JUMP: nft_verdicts = -3;
pub const NFT_GOTO: nft_verdicts = -4;
pub const NFT_RETURN: nft_verdicts = -5;


/**
 * enum nf_tables_msg_types - nf_tables netlink message types
 *
 * @NFT_MSG_NEWTABLE: create a new table (enum nft_table_attributes)
 * @NFT_MSG_GETTABLE: get a table (enum nft_table_attributes)
 * @NFT_MSG_DELTABLE: delete a table (enum nft_table_attributes)
 * @NFT_MSG_NEWCHAIN: create a new chain (enum nft_chain_attributes)
 * @NFT_MSG_GETCHAIN: get a chain (enum nft_chain_attributes)
 * @NFT_MSG_DELCHAIN: delete a chain (enum nft_chain_attributes)
 * @NFT_MSG_NEWRULE: create a new rule (enum nft_rule_attributes)
 * @NFT_MSG_GETRULE: get a rule (enum nft_rule_attributes)
 * @NFT_MSG_DELRULE: delete a rule (enum nft_rule_attributes)
 * @NFT_MSG_NEWSET: create a new set (enum nft_set_attributes)
 * @NFT_MSG_GETSET: get a set (enum nft_set_attributes)
 * @NFT_MSG_DELSET: delete a set (enum nft_set_attributes)
 * @NFT_MSG_NEWSETELEM: create a new set element (enum nft_set_elem_attributes)
 * @NFT_MSG_GETSETELEM: get a set element (enum nft_set_elem_attributes)
 * @NFT_MSG_DELSETELEM: delete a set element (enum nft_set_elem_attributes)
 * @NFT_MSG_NEWGEN: announce a new generation, only for events (enum nft_gen_attributes)
 * @NFT_MSG_GETGEN: get the rule-set generation (enum nft_gen_attributes)
 * @NFT_MSG_TRACE: trace event (enum nft_trace_attributes)
 * @NFT_MSG_NEWOBJ: create a stateful object (enum nft_obj_attributes)
 * @NFT_MSG_GETOBJ: get a stateful object (enum nft_obj_attributes)
 * @NFT_MSG_DELOBJ: delete a stateful object (enum nft_obj_attributes)
 * @NFT_MSG_GETOBJ_RESET: get and reset a stateful object (enum nft_obj_attributes)
 * @NFT_MSG_NEWFLOWTABLE: add new flow table (enum nft_flowtable_attributes)
 * @NFT_MSG_GETFLOWTABLE: get flow table (enum nft_flowtable_attributes)
 * @NFT_MSG_DELFLOWTABLE: delete flow table (enum nft_flowtable_attributes)
 * @NFT_MSG_GETRULE_RESET: get rules and reset stateful expressions (enum nft_obj_attributes)
 * @NFT_MSG_DESTROYTABLE: destroy a table (enum nft_table_attributes)
 * @NFT_MSG_DESTROYCHAIN: destroy a chain (enum nft_chain_attributes)
 * @NFT_MSG_DESTROYRULE: destroy a rule (enum nft_rule_attributes)
 * @NFT_MSG_DESTROYSET: destroy a set (enum nft_set_attributes)
 * @NFT_MSG_DESTROYSETELEM: destroy a set element (enum nft_set_elem_attributes)
 * @NFT_MSG_DESTROYOBJ: destroy a stateful object (enum nft_object_attributes)
 * @NFT_MSG_DESTROYFLOWTABLE: destroy flow table (enum nft_flowtable_attributes)
 * @NFT_MSG_GETSETELEM_RESET: get set elements and reset attached stateful expressions (enum nft_set_elem_attributes)
 */
pub type nf_tables_msg_types = i32;
pub const NFT_MSG_NEWTABLE: nf_tables_msg_types = 0;
pub const NFT_MSG_GETTABLE: nf_tables_msg_types = 1;
pub const NFT_MSG_DELTABLE: nf_tables_msg_types = 2;
pub const NFT_MSG_NEWCHAIN: nf_tables_msg_types = 3;
pub const NFT_MSG_GETCHAIN: nf_tables_msg_types = 4;
pub const NFT_MSG_DELCHAIN: nf_tables_msg_types = 5;
pub const NFT_MSG_NEWRULE: nf_tables_msg_types = 6;
pub const NFT_MSG_GETRULE: nf_tables_msg_types = 7;
pub const NFT_MSG_DELRULE: nf_tables_msg_types = 8;
pub const NFT_MSG_NEWSET: nf_tables_msg_types = 9;
pub const NFT_MSG_GETSET: nf_tables_msg_types = 10;
pub const NFT_MSG_DELSET: nf_tables_msg_types = 11;
pub const NFT_MSG_NEWSETELEM: nf_tables_msg_types = 12;
pub const NFT_MSG_GETSETELEM: nf_tables_msg_types = 13;
pub const NFT_MSG_DELSETELEM: nf_tables_msg_types = 14;
pub const NFT_MSG_NEWGEN: nf_tables_msg_types = 15;
pub const NFT_MSG_GETGEN: nf_tables_msg_types = 16;
pub const NFT_MSG_TRACE: nf_tables_msg_types = 17;
pub const NFT_MSG_NEWOBJ: nf_tables_msg_types = 18;
pub const NFT_MSG_GETOBJ: nf_tables_msg_types = 19;
pub const NFT_MSG_DELOBJ: nf_tables_msg_types = 20;
pub const NFT_MSG_GETOBJ_RESET: nf_tables_msg_types = 21;
pub const NFT_MSG_NEWFLOWTABLE: nf_tables_msg_types = 22;
pub const NFT_MSG_GETFLOWTABLE: nf_tables_msg_types = 23;
pub const NFT_MSG_DELFLOWTABLE: nf_tables_msg_types = 24;
pub const NFT_MSG_GETRULE_RESET: nf_tables_msg_types = 25;
pub const NFT_MSG_DESTROYTABLE: nf_tables_msg_types = 26;
pub const NFT_MSG_DESTROYCHAIN: nf_tables_msg_types = 27;
pub const NFT_MSG_DESTROYRULE: nf_tables_msg_types = 28;
pub const NFT_MSG_DESTROYSET: nf_tables_msg_types = 29;
pub const NFT_MSG_DESTROYSETELEM: nf_tables_msg_types = 30;
pub const NFT_MSG_DESTROYOBJ: nf_tables_msg_types = 31;
pub const NFT_MSG_DESTROYFLOWTABLE: nf_tables_msg_types = 32;
pub const NFT_MSG_GETSETELEM_RESET: nf_tables_msg_types = 33;
pub const NFT_MSG_MAX: nf_tables_msg_types = 34;


/**
 * enum nft_list_attributes - nf_tables generic list netlink attributes
 *
 * @NFTA_LIST_ELEM: list element (NLA_NESTED)
 */
pub type nft_list_attributes = i32;
pub const NFTA_LIST_UNSPEC: nft_list_attributes = 0;
pub const NFTA_LIST_ELEM: nft_list_attributes = 1;
pub const __NFTA_LIST_MAX: nft_list_attributes = 2;

pub const NFTA_LIST_MAX: i32 = (__NFTA_LIST_MAX - 1);

/**
 * enum nft_hook_attributes - nf_tables netfilter hook netlink attributes
 *
 * @NFTA_HOOK_HOOKNUM: netfilter hook number (NLA_U32)
 * @NFTA_HOOK_PRIORITY: netfilter hook priority (NLA_U32)
 * @NFTA_HOOK_DEV: netdevice name (NLA_STRING)
 * @NFTA_HOOK_DEVS: list of netdevices (NLA_NESTED)
 */
pub type nft_hook_attributes = i32;
pub const NFTA_HOOK_UNSPEC: nft_hook_attributes = 0;
pub const NFTA_HOOK_HOOKNUM: nft_hook_attributes = 1;
pub const NFTA_HOOK_PRIORITY: nft_hook_attributes = 2;
pub const NFTA_HOOK_DEV: nft_hook_attributes = 3;
pub const NFTA_HOOK_DEVS: nft_hook_attributes = 4;
pub const __NFTA_HOOK_MAX: nft_hook_attributes = 5;

pub const NFTA_HOOK_MAX: i32 = (__NFTA_HOOK_MAX - 1);

/**
 * enum nft_table_flags - nf_tables table flags
 *
 * @NFT_TABLE_F_DORMANT: this table is not active
 * @NFT_TABLE_F_OWNER:   this table is owned by a process
 * @NFT_TABLE_F_PERSIST: this table shall outlive its owner
 */
pub type nft_table_flags = i32;
pub const NFT_TABLE_F_DORMANT: nft_table_flags = 1;
pub const NFT_TABLE_F_OWNER: nft_table_flags = 2;
pub const NFT_TABLE_F_PERSIST: nft_table_flags = 4;

pub const NFT_TABLE_F_MASK: i32 = (NFT_TABLE_F_DORMANT | NFT_TABLE_F_OWNER | NFT_TABLE_F_PERSIST);

/**
 * enum nft_table_attributes - nf_tables table netlink attributes
 *
 * @NFTA_TABLE_NAME: name of the table (NLA_STRING)
 * @NFTA_TABLE_FLAGS: bitmask of enum nft_table_flags (NLA_U32)
 * @NFTA_TABLE_USE: number of chains in this table (NLA_U32)
 * @NFTA_TABLE_USERDATA: user data (NLA_BINARY)
 * @NFTA_TABLE_OWNER: owner of this table through netlink portID (NLA_U32)
 */
pub type nft_table_attributes = i32;
pub const NFTA_TABLE_UNSPEC: nft_table_attributes = 0;
pub const NFTA_TABLE_NAME: nft_table_attributes = 1;
pub const NFTA_TABLE_FLAGS: nft_table_attributes = 2;
pub const NFTA_TABLE_USE: nft_table_attributes = 3;
pub const NFTA_TABLE_HANDLE: nft_table_attributes = 4;
pub const NFTA_TABLE_PAD: nft_table_attributes = 5;
pub const NFTA_TABLE_USERDATA: nft_table_attributes = 6;
pub const NFTA_TABLE_OWNER: nft_table_attributes = 7;
pub const __NFTA_TABLE_MAX: nft_table_attributes = 8;

pub const NFTA_TABLE_MAX: i32 = (__NFTA_TABLE_MAX - 1);

pub type nft_chain_flags = i32;
pub const NFT_CHAIN_BASE: nft_chain_flags = 1;
pub const NFT_CHAIN_HW_OFFLOAD: nft_chain_flags = 2;
pub const NFT_CHAIN_BINDING: nft_chain_flags = 4;

pub const NFT_CHAIN_FLAGS: i32 = (NFT_CHAIN_BASE | NFT_CHAIN_HW_OFFLOAD | NFT_CHAIN_BINDING);

/**
 * enum nft_chain_attributes - nf_tables chain netlink attributes
 *
 * @NFTA_CHAIN_TABLE: name of the table containing the chain (NLA_STRING)
 * @NFTA_CHAIN_HANDLE: numeric handle of the chain (NLA_U64)
 * @NFTA_CHAIN_NAME: name of the chain (NLA_STRING)
 * @NFTA_CHAIN_HOOK: hook specification for basechains (NLA_NESTED: nft_hook_attributes)
 * @NFTA_CHAIN_POLICY: numeric policy of the chain (NLA_U32)
 * @NFTA_CHAIN_USE: number of references to this chain (NLA_U32)
 * @NFTA_CHAIN_TYPE: type name of the string (NLA_NUL_STRING)
 * @NFTA_CHAIN_COUNTERS: counter specification of the chain (NLA_NESTED: nft_counter_attributes)
 * @NFTA_CHAIN_FLAGS: chain flags
 * @NFTA_CHAIN_ID: uniquely identifies a chain in a transaction (NLA_U32)
 * @NFTA_CHAIN_USERDATA: user data (NLA_BINARY)
 */
pub type nft_chain_attributes = i32;
pub const NFTA_CHAIN_UNSPEC: nft_chain_attributes = 0;
pub const NFTA_CHAIN_TABLE: nft_chain_attributes = 1;
pub const NFTA_CHAIN_HANDLE: nft_chain_attributes = 2;
pub const NFTA_CHAIN_NAME: nft_chain_attributes = 3;
pub const NFTA_CHAIN_HOOK: nft_chain_attributes = 4;
pub const NFTA_CHAIN_POLICY: nft_chain_attributes = 5;
pub const NFTA_CHAIN_USE: nft_chain_attributes = 6;
pub const NFTA_CHAIN_TYPE: nft_chain_attributes = 7;
pub const NFTA_CHAIN_COUNTERS: nft_chain_attributes = 8;
pub const NFTA_CHAIN_PAD: nft_chain_attributes = 9;
pub const NFTA_CHAIN_FLAGS: nft_chain_attributes = 10;
pub const NFTA_CHAIN_ID: nft_chain_attributes = 11;
pub const NFTA_CHAIN_USERDATA: nft_chain_attributes = 12;
pub const __NFTA_CHAIN_MAX: nft_chain_attributes = 13;

pub const NFTA_CHAIN_MAX: i32 = (__NFTA_CHAIN_MAX - 1);

/**
 * enum nft_rule_attributes - nf_tables rule netlink attributes
 *
 * @NFTA_RULE_TABLE: name of the table containing the rule (NLA_STRING)
 * @NFTA_RULE_CHAIN: name of the chain containing the rule (NLA_STRING)
 * @NFTA_RULE_HANDLE: numeric handle of the rule (NLA_U64)
 * @NFTA_RULE_EXPRESSIONS: list of expressions (NLA_NESTED: nft_expr_attributes)
 * @NFTA_RULE_COMPAT: compatibility specifications of the rule (NLA_NESTED: nft_rule_compat_attributes)
 * @NFTA_RULE_POSITION: numeric handle of the previous rule (NLA_U64)
 * @NFTA_RULE_USERDATA: user data (NLA_BINARY, NFT_USERDATA_MAXLEN)
 * @NFTA_RULE_ID: uniquely identifies a rule in a transaction (NLA_U32)
 * @NFTA_RULE_POSITION_ID: transaction unique identifier of the previous rule (NLA_U32)
 * @NFTA_RULE_CHAIN_ID: add the rule to chain by ID, alternative to @NFTA_RULE_CHAIN (NLA_U32)
 */
pub type nft_rule_attributes = i32;
pub const NFTA_RULE_UNSPEC: nft_rule_attributes = 0;
pub const NFTA_RULE_TABLE: nft_rule_attributes = 1;
pub const NFTA_RULE_CHAIN: nft_rule_attributes = 2;
pub const NFTA_RULE_HANDLE: nft_rule_attributes = 3;
pub const NFTA_RULE_EXPRESSIONS: nft_rule_attributes = 4;
pub const NFTA_RULE_COMPAT: nft_rule_attributes = 5;
pub const NFTA_RULE_POSITION: nft_rule_attributes = 6;
pub const NFTA_RULE_USERDATA: nft_rule_attributes = 7;
pub const NFTA_RULE_PAD: nft_rule_attributes = 8;
pub const NFTA_RULE_ID: nft_rule_attributes = 9;
pub const NFTA_RULE_POSITION_ID: nft_rule_attributes = 10;
pub const NFTA_RULE_CHAIN_ID: nft_rule_attributes = 11;
pub const __NFTA_RULE_MAX: nft_rule_attributes = 12;

pub const NFTA_RULE_MAX: i32 = (__NFTA_RULE_MAX - 1);

/**
 * enum nft_rule_compat_flags - nf_tables rule compat flags
 *
 * @NFT_RULE_COMPAT_F_UNUSED: unused
 * @NFT_RULE_COMPAT_F_INV: invert the check result
 */
pub type nft_rule_compat_flags = i32;
pub const NFT_RULE_COMPAT_F_UNUSED: nft_rule_compat_flags = 1;
pub const NFT_RULE_COMPAT_F_INV: nft_rule_compat_flags = 2;
pub const NFT_RULE_COMPAT_F_MASK: nft_rule_compat_flags = 2;


/**
 * enum nft_rule_compat_attributes - nf_tables rule compat attributes
 *
 * @NFTA_RULE_COMPAT_PROTO: numeric value of handled protocol (NLA_U32)
 * @NFTA_RULE_COMPAT_FLAGS: bitmask of enum nft_rule_compat_flags (NLA_U32)
 */
pub type nft_rule_compat_attributes = i32;
pub const NFTA_RULE_COMPAT_UNSPEC: nft_rule_compat_attributes = 0;
pub const NFTA_RULE_COMPAT_PROTO: nft_rule_compat_attributes = 1;
pub const NFTA_RULE_COMPAT_FLAGS: nft_rule_compat_attributes = 2;
pub const __NFTA_RULE_COMPAT_MAX: nft_rule_compat_attributes = 3;

pub const NFTA_RULE_COMPAT_MAX: i32 = (__NFTA_RULE_COMPAT_MAX - 1);

/**
 * enum nft_set_flags - nf_tables set flags
 *
 * @NFT_SET_ANONYMOUS: name allocation, automatic cleanup on unlink
 * @NFT_SET_CONSTANT: set contents may not change while bound
 * @NFT_SET_INTERVAL: set contains intervals
 * @NFT_SET_MAP: set is used as a dictionary
 * @NFT_SET_TIMEOUT: set uses timeouts
 * @NFT_SET_EVAL: set can be updated from the evaluation path
 * @NFT_SET_OBJECT: set contains stateful objects
 * @NFT_SET_CONCAT: set contains a concatenation
 * @NFT_SET_EXPR: set contains expressions
 */
pub type nft_set_flags = i32;
pub const NFT_SET_ANONYMOUS: nft_set_flags = 1;
pub const NFT_SET_CONSTANT: nft_set_flags = 2;
pub const NFT_SET_INTERVAL: nft_set_flags = 4;
pub const NFT_SET_MAP: nft_set_flags = 8;
pub const NFT_SET_TIMEOUT: nft_set_flags = 16;
pub const NFT_SET_EVAL: nft_set_flags = 32;
pub const NFT_SET_OBJECT: nft_set_flags = 64;
pub const NFT_SET_CONCAT: nft_set_flags = 128;
pub const NFT_SET_EXPR: nft_set_flags = 256;


/**
 * enum nft_set_policies - set selection policy
 *
 * @NFT_SET_POL_PERFORMANCE: prefer high performance over low memory use
 * @NFT_SET_POL_MEMORY: prefer low memory use over high performance
 */
pub type nft_set_policies = i32;
pub const NFT_SET_POL_PERFORMANCE: nft_set_policies = 0;
pub const NFT_SET_POL_MEMORY: nft_set_policies = 1;


/**
 * enum nft_set_desc_attributes - set element description
 *
 * @NFTA_SET_DESC_SIZE: number of elements in set (NLA_U32)
 * @NFTA_SET_DESC_CONCAT: description of field concatenation (NLA_NESTED)
 */
pub type nft_set_desc_attributes = i32;
pub const NFTA_SET_DESC_UNSPEC: nft_set_desc_attributes = 0;
pub const NFTA_SET_DESC_SIZE: nft_set_desc_attributes = 1;
pub const NFTA_SET_DESC_CONCAT: nft_set_desc_attributes = 2;
pub const __NFTA_SET_DESC_MAX: nft_set_desc_attributes = 3;

pub const NFTA_SET_DESC_MAX: i32 = (__NFTA_SET_DESC_MAX - 1);

/**
 * enum nft_set_field_attributes - attributes of concatenated fields
 *
 * @NFTA_SET_FIELD_LEN: length of single field, in bits (NLA_U32)
 */
pub type nft_set_field_attributes = i32;
pub const NFTA_SET_FIELD_UNSPEC: nft_set_field_attributes = 0;
pub const NFTA_SET_FIELD_LEN: nft_set_field_attributes = 1;
pub const __NFTA_SET_FIELD_MAX: nft_set_field_attributes = 2;

pub const NFTA_SET_FIELD_MAX: i32 = (__NFTA_SET_FIELD_MAX - 1);

/**
 * enum nft_set_attributes - nf_tables set netlink attributes
 *
 * @NFTA_SET_TABLE: table name (NLA_STRING)
 * @NFTA_SET_NAME: set name (NLA_STRING)
 * @NFTA_SET_FLAGS: bitmask of enum nft_set_flags (NLA_U32)
 * @NFTA_SET_KEY_TYPE: key data type, informational purpose only (NLA_U32)
 * @NFTA_SET_KEY_LEN: key data length (NLA_U32)
 * @NFTA_SET_DATA_TYPE: mapping data type (NLA_U32)
 * @NFTA_SET_DATA_LEN: mapping data length (NLA_U32)
 * @NFTA_SET_POLICY: selection policy (NLA_U32)
 * @NFTA_SET_DESC: set description (NLA_NESTED)
 * @NFTA_SET_ID: uniquely identifies a set in a transaction (NLA_U32)
 * @NFTA_SET_TIMEOUT: default timeout value (NLA_U64)
 * @NFTA_SET_GC_INTERVAL: garbage collection interval (NLA_U32)
 * @NFTA_SET_USERDATA: user data (NLA_BINARY)
 * @NFTA_SET_OBJ_TYPE: stateful object type (NLA_U32: NFT_OBJECT_*)
 * @NFTA_SET_HANDLE: set handle (NLA_U64)
 * @NFTA_SET_EXPR: set expression (NLA_NESTED: nft_expr_attributes)
 * @NFTA_SET_EXPRESSIONS: list of expressions (NLA_NESTED: nft_list_attributes)
 * @NFTA_SET_TYPE: set backend type (NLA_STRING)
 * @NFTA_SET_COUNT: number of set elements (NLA_U32)
 */
pub type nft_set_attributes = i32;
pub const NFTA_SET_UNSPEC: nft_set_attributes = 0;
pub const NFTA_SET_TABLE: nft_set_attributes = 1;
pub const NFTA_SET_NAME: nft_set_attributes = 2;
pub const NFTA_SET_FLAGS: nft_set_attributes = 3;
pub const NFTA_SET_KEY_TYPE: nft_set_attributes = 4;
pub const NFTA_SET_KEY_LEN: nft_set_attributes = 5;
pub const NFTA_SET_DATA_TYPE: nft_set_attributes = 6;
pub const NFTA_SET_DATA_LEN: nft_set_attributes = 7;
pub const NFTA_SET_POLICY: nft_set_attributes = 8;
pub const NFTA_SET_DESC: nft_set_attributes = 9;
pub const NFTA_SET_ID: nft_set_attributes = 10;
pub const NFTA_SET_TIMEOUT: nft_set_attributes = 11;
pub const NFTA_SET_GC_INTERVAL: nft_set_attributes = 12;
pub const NFTA_SET_USERDATA: nft_set_attributes = 13;
pub const NFTA_SET_PAD: nft_set_attributes = 14;
pub const NFTA_SET_OBJ_TYPE: nft_set_attributes = 15;
pub const NFTA_SET_HANDLE: nft_set_attributes = 16;
pub const NFTA_SET_EXPR: nft_set_attributes = 17;
pub const NFTA_SET_EXPRESSIONS: nft_set_attributes = 18;
pub const NFTA_SET_TYPE: nft_set_attributes = 19;
pub const NFTA_SET_COUNT: nft_set_attributes = 20;
pub const __NFTA_SET_MAX: nft_set_attributes = 21;

pub const NFTA_SET_MAX: i32 = (__NFTA_SET_MAX - 1);

/**
 * enum nft_set_elem_flags - nf_tables set element flags
 *
 * @NFT_SET_ELEM_INTERVAL_END: element ends the previous interval
 * @NFT_SET_ELEM_CATCHALL: special catch-all element
 */
pub type nft_set_elem_flags = i32;
pub const NFT_SET_ELEM_INTERVAL_END: nft_set_elem_flags = 1;
pub const NFT_SET_ELEM_CATCHALL: nft_set_elem_flags = 2;


/**
 * enum nft_set_elem_attributes - nf_tables set element netlink attributes
 *
 * @NFTA_SET_ELEM_KEY: key value (NLA_NESTED: nft_data)
 * @NFTA_SET_ELEM_DATA: data value of mapping (NLA_NESTED: nft_data_attributes)
 * @NFTA_SET_ELEM_FLAGS: bitmask of nft_set_elem_flags (NLA_U32)
 * @NFTA_SET_ELEM_TIMEOUT: timeout value, zero means never times out (NLA_U64)
 * @NFTA_SET_ELEM_EXPIRATION: expiration time (NLA_U64)
 * @NFTA_SET_ELEM_USERDATA: user data (NLA_BINARY)
 * @NFTA_SET_ELEM_EXPR: expression (NLA_NESTED: nft_expr_attributes)
 * @NFTA_SET_ELEM_OBJREF: stateful object reference (NLA_STRING)
 * @NFTA_SET_ELEM_KEY_END: closing key value (NLA_NESTED: nft_data)
 * @NFTA_SET_ELEM_EXPRESSIONS: list of expressions (NLA_NESTED: nft_list_attributes)
 */
pub type nft_set_elem_attributes = i32;
pub const NFTA_SET_ELEM_UNSPEC: nft_set_elem_attributes = 0;
pub const NFTA_SET_ELEM_KEY: nft_set_elem_attributes = 1;
pub const NFTA_SET_ELEM_DATA: nft_set_elem_attributes = 2;
pub const NFTA_SET_ELEM_FLAGS: nft_set_elem_attributes = 3;
pub const NFTA_SET_ELEM_TIMEOUT: nft_set_elem_attributes = 4;
pub const NFTA_SET_ELEM_EXPIRATION: nft_set_elem_attributes = 5;
pub const NFTA_SET_ELEM_USERDATA: nft_set_elem_attributes = 6;
pub const NFTA_SET_ELEM_EXPR: nft_set_elem_attributes = 7;
pub const NFTA_SET_ELEM_PAD: nft_set_elem_attributes = 8;
pub const NFTA_SET_ELEM_OBJREF: nft_set_elem_attributes = 9;
pub const NFTA_SET_ELEM_KEY_END: nft_set_elem_attributes = 10;
pub const NFTA_SET_ELEM_EXPRESSIONS: nft_set_elem_attributes = 11;
pub const __NFTA_SET_ELEM_MAX: nft_set_elem_attributes = 12;

pub const NFTA_SET_ELEM_MAX: i32 = (__NFTA_SET_ELEM_MAX - 1);

/**
 * enum nft_set_elem_list_attributes - nf_tables set element list netlink attributes
 *
 * @NFTA_SET_ELEM_LIST_TABLE: table of the set to be changed (NLA_STRING)
 * @NFTA_SET_ELEM_LIST_SET: name of the set to be changed (NLA_STRING)
 * @NFTA_SET_ELEM_LIST_ELEMENTS: list of set elements (NLA_NESTED: nft_set_elem_attributes)
 * @NFTA_SET_ELEM_LIST_SET_ID: uniquely identifies a set in a transaction (NLA_U32)
 */
pub type nft_set_elem_list_attributes = i32;
pub const NFTA_SET_ELEM_LIST_UNSPEC: nft_set_elem_list_attributes = 0;
pub const NFTA_SET_ELEM_LIST_TABLE: nft_set_elem_list_attributes = 1;
pub const NFTA_SET_ELEM_LIST_SET: nft_set_elem_list_attributes = 2;
pub const NFTA_SET_ELEM_LIST_ELEMENTS: nft_set_elem_list_attributes = 3;
pub const NFTA_SET_ELEM_LIST_SET_ID: nft_set_elem_list_attributes = 4;
pub const __NFTA_SET_ELEM_LIST_MAX: nft_set_elem_list_attributes = 5;

pub const NFTA_SET_ELEM_LIST_MAX: i32 = (__NFTA_SET_ELEM_LIST_MAX - 1);

/**
 * enum nft_data_types - nf_tables data types
 *
 * @NFT_DATA_VALUE: generic data
 * @NFT_DATA_VERDICT: netfilter verdict
 *
 * The type of data is usually determined by the kernel directly and is not
 * explicitly specified by userspace. The only difference are sets, where
 * userspace specifies the key and mapping data types.
 *
 * The values 0xffffff00-0xffffffff are reserved for internally used types.
 * The remaining range can be freely used by userspace to encode types, all
 * values are equivalent to NFT_DATA_VALUE.
 */
pub type nft_data_types = u32;
pub const NFT_DATA_VALUE: nft_data_types = 0u32;
pub const NFT_DATA_VERDICT: nft_data_types = 4294967040u32;


pub const NFT_DATA_RESERVED_MASK: u32 = 0xffffff00;

/**
 * enum nft_data_attributes - nf_tables data netlink attributes
 *
 * @NFTA_DATA_VALUE: generic data (NLA_BINARY)
 * @NFTA_DATA_VERDICT: nf_tables verdict (NLA_NESTED: nft_verdict_attributes)
 */
pub type nft_data_attributes = i32;
pub const NFTA_DATA_UNSPEC: nft_data_attributes = 0;
pub const NFTA_DATA_VALUE: nft_data_attributes = 1;
pub const NFTA_DATA_VERDICT: nft_data_attributes = 2;
pub const __NFTA_DATA_MAX: nft_data_attributes = 3;

pub const NFTA_DATA_MAX: i32 = (__NFTA_DATA_MAX - 1);

/* Maximum length of a value */
pub const NFT_DATA_VALUE_MAXLEN: u32 = 64;

/**
 * enum nft_verdict_attributes - nf_tables verdict netlink attributes
 *
 * @NFTA_VERDICT_CODE: nf_tables verdict (NLA_U32: enum nft_verdicts)
 * @NFTA_VERDICT_CHAIN: jump target chain name (NLA_STRING)
 * @NFTA_VERDICT_CHAIN_ID: jump target chain ID (NLA_U32)
 */
pub type nft_verdict_attributes = i32;
pub const NFTA_VERDICT_UNSPEC: nft_verdict_attributes = 0;
pub const NFTA_VERDICT_CODE: nft_verdict_attributes = 1;
pub const NFTA_VERDICT_CHAIN: nft_verdict_attributes = 2;
pub const NFTA_VERDICT_CHAIN_ID: nft_verdict_attributes = 3;
pub const __NFTA_VERDICT_MAX: nft_verdict_attributes = 4;

pub const NFTA_VERDICT_MAX: i32 = (__NFTA_VERDICT_MAX - 1);

/**
 * enum nft_expr_attributes - nf_tables expression netlink attributes
 *
 * @NFTA_EXPR_NAME: name of the expression type (NLA_STRING)
 * @NFTA_EXPR_DATA: type specific data (NLA_NESTED)
 */
pub type nft_expr_attributes = i32;
pub const NFTA_EXPR_UNSPEC: nft_expr_attributes = 0;
pub const NFTA_EXPR_NAME: nft_expr_attributes = 1;
pub const NFTA_EXPR_DATA: nft_expr_attributes = 2;
pub const __NFTA_EXPR_MAX: nft_expr_attributes = 3;

pub const NFTA_EXPR_MAX: i32 = (__NFTA_EXPR_MAX - 1);

/**
 * enum nft_immediate_attributes - nf_tables immediate expression netlink attributes
 *
 * @NFTA_IMMEDIATE_DREG: destination register to load data into (NLA_U32)
 * @NFTA_IMMEDIATE_DATA: data to load (NLA_NESTED: nft_data_attributes)
 */
pub type nft_immediate_attributes = i32;
pub const NFTA_IMMEDIATE_UNSPEC: nft_immediate_attributes = 0;
pub const NFTA_IMMEDIATE_DREG: nft_immediate_attributes = 1;
pub const NFTA_IMMEDIATE_DATA: nft_immediate_attributes = 2;
pub const __NFTA_IMMEDIATE_MAX: nft_immediate_attributes = 3;

pub const NFTA_IMMEDIATE_MAX: i32 = (__NFTA_IMMEDIATE_MAX - 1);

/**
 * enum nft_bitwise_ops - nf_tables bitwise operations
 *
 * @NFT_BITWISE_MASK_XOR: mask-and-xor operation used to implement NOT, AND, OR
 *                        and XOR boolean operations
 * @NFT_BITWISE_LSHIFT: left-shift operation
 * @NFT_BITWISE_RSHIFT: right-shift operation
 * @NFT_BITWISE_AND: and operation
 * @NFT_BITWISE_OR: or operation
 * @NFT_BITWISE_XOR: xor operation
 */
pub type nft_bitwise_ops = i32;
pub const NFT_BITWISE_MASK_XOR: nft_bitwise_ops = 0;
pub const NFT_BITWISE_LSHIFT: nft_bitwise_ops = 1;
pub const NFT_BITWISE_RSHIFT: nft_bitwise_ops = 2;
pub const NFT_BITWISE_AND: nft_bitwise_ops = 3;
pub const NFT_BITWISE_OR: nft_bitwise_ops = 4;
pub const NFT_BITWISE_XOR: nft_bitwise_ops = 5;

/*
 * Old name for NFT_BITWISE_MASK_XOR.  Retained for backwards-compatibility.
 */
pub const NFT_BITWISE_BOOL: i32 = NFT_BITWISE_MASK_XOR;

/**
 * enum nft_bitwise_attributes - nf_tables bitwise expression netlink attributes
 *
 * @NFTA_BITWISE_SREG: source register (NLA_U32: nft_registers)
 * @NFTA_BITWISE_DREG: destination register (NLA_U32: nft_registers)
 * @NFTA_BITWISE_LEN: length of operands (NLA_U32)
 * @NFTA_BITWISE_MASK: mask value (NLA_NESTED: nft_data_attributes)
 * @NFTA_BITWISE_XOR: xor value (NLA_NESTED: nft_data_attributes)
 * @NFTA_BITWISE_OP: type of operation (NLA_U32: nft_bitwise_ops)
 * @NFTA_BITWISE_DATA: argument for non-boolean operations
 *                     (NLA_NESTED: nft_data_attributes)
 * @NFTA_BITWISE_SREG2: second source register (NLA_U32: nft_registers)
 *
 * The bitwise expression supports boolean and shift operations.  It implements
 * the boolean operations by performing the following operation:
 *
 * dreg = (sreg & mask) ^ xor
 *
 * with these mask and xor values:
 *
 * 		mask	xor
 * NOT:		1	1
 * OR:		~x	x
 * XOR:		1	x
 * AND:		x	0
 */
pub type nft_bitwise_attributes = i32;
pub const NFTA_BITWISE_UNSPEC: nft_bitwise_attributes = 0;
pub const NFTA_BITWISE_SREG: nft_bitwise_attributes = 1;
pub const NFTA_BITWISE_DREG: nft_bitwise_attributes = 2;
pub const NFTA_BITWISE_LEN: nft_bitwise_attributes = 3;
pub const NFTA_BITWISE_MASK: nft_bitwise_attributes = 4;
pub const NFTA_BITWISE_XOR: nft_bitwise_attributes = 5;
pub const NFTA_BITWISE_OP: nft_bitwise_attributes = 6;
pub const NFTA_BITWISE_DATA: nft_bitwise_attributes = 7;
pub const NFTA_BITWISE_SREG2: nft_bitwise_attributes = 8;
pub const __NFTA_BITWISE_MAX: nft_bitwise_attributes = 9;

pub const NFTA_BITWISE_MAX: i32 = (__NFTA_BITWISE_MAX - 1);

/**
 * enum nft_byteorder_ops - nf_tables byteorder operators
 *
 * @NFT_BYTEORDER_NTOH: network to host operator
 * @NFT_BYTEORDER_HTON: host to network operator
 */
pub type nft_byteorder_ops = i32;
pub const NFT_BYTEORDER_NTOH: nft_byteorder_ops = 0;
pub const NFT_BYTEORDER_HTON: nft_byteorder_ops = 1;


/**
 * enum nft_byteorder_attributes - nf_tables byteorder expression netlink attributes
 *
 * @NFTA_BYTEORDER_SREG: source register (NLA_U32: nft_registers)
 * @NFTA_BYTEORDER_DREG: destination register (NLA_U32: nft_registers)
 * @NFTA_BYTEORDER_OP: operator (NLA_U32: enum nft_byteorder_ops)
 * @NFTA_BYTEORDER_LEN: length of the data (NLA_U32)
 * @NFTA_BYTEORDER_SIZE: data size in bytes (NLA_U32: 2 or 4)
 */
pub type nft_byteorder_attributes = i32;
pub const NFTA_BYTEORDER_UNSPEC: nft_byteorder_attributes = 0;
pub const NFTA_BYTEORDER_SREG: nft_byteorder_attributes = 1;
pub const NFTA_BYTEORDER_DREG: nft_byteorder_attributes = 2;
pub const NFTA_BYTEORDER_OP: nft_byteorder_attributes = 3;
pub const NFTA_BYTEORDER_LEN: nft_byteorder_attributes = 4;
pub const NFTA_BYTEORDER_SIZE: nft_byteorder_attributes = 5;
pub const __NFTA_BYTEORDER_MAX: nft_byteorder_attributes = 6;

pub const NFTA_BYTEORDER_MAX: i32 = (__NFTA_BYTEORDER_MAX - 1);

/**
 * enum nft_cmp_ops - nf_tables relational operator
 *
 * @NFT_CMP_EQ: equal
 * @NFT_CMP_NEQ: not equal
 * @NFT_CMP_LT: less than
 * @NFT_CMP_LTE: less than or equal to
 * @NFT_CMP_GT: greater than
 * @NFT_CMP_GTE: greater than or equal to
 */
pub type nft_cmp_ops = i32;
pub const NFT_CMP_EQ: nft_cmp_ops = 0;
pub const NFT_CMP_NEQ: nft_cmp_ops = 1;
pub const NFT_CMP_LT: nft_cmp_ops = 2;
pub const NFT_CMP_LTE: nft_cmp_ops = 3;
pub const NFT_CMP_GT: nft_cmp_ops = 4;
pub const NFT_CMP_GTE: nft_cmp_ops = 5;


/**
 * enum nft_cmp_attributes - nf_tables cmp expression netlink attributes
 *
 * @NFTA_CMP_SREG: source register of data to compare (NLA_U32: nft_registers)
 * @NFTA_CMP_OP: cmp operation (NLA_U32: nft_cmp_ops)
 * @NFTA_CMP_DATA: data to compare against (NLA_NESTED: nft_data_attributes)
 */
pub type nft_cmp_attributes = i32;
pub const NFTA_CMP_UNSPEC: nft_cmp_attributes = 0;
pub const NFTA_CMP_SREG: nft_cmp_attributes = 1;
pub const NFTA_CMP_OP: nft_cmp_attributes = 2;
pub const NFTA_CMP_DATA: nft_cmp_attributes = 3;
pub const __NFTA_CMP_MAX: nft_cmp_attributes = 4;

pub const NFTA_CMP_MAX: i32 = (__NFTA_CMP_MAX - 1);

/**
 * enum nft_range_ops - nf_tables range operator
 *
 * @NFT_RANGE_EQ: equal
 * @NFT_RANGE_NEQ: not equal
 */
pub type nft_range_ops = i32;
pub const NFT_RANGE_EQ: nft_range_ops = 0;
pub const NFT_RANGE_NEQ: nft_range_ops = 1;


/**
 * enum nft_range_attributes - nf_tables range expression netlink attributes
 *
 * @NFTA_RANGE_SREG: source register of data to compare (NLA_U32: nft_registers)
 * @NFTA_RANGE_OP: cmp operation (NLA_U32: nft_range_ops)
 * @NFTA_RANGE_FROM_DATA: data range from (NLA_NESTED: nft_data_attributes)
 * @NFTA_RANGE_TO_DATA: data range to (NLA_NESTED: nft_data_attributes)
 */
pub type nft_range_attributes = i32;
pub const NFTA_RANGE_UNSPEC: nft_range_attributes = 0;
pub const NFTA_RANGE_SREG: nft_range_attributes = 1;
pub const NFTA_RANGE_OP: nft_range_attributes = 2;
pub const NFTA_RANGE_FROM_DATA: nft_range_attributes = 3;
pub const NFTA_RANGE_TO_DATA: nft_range_attributes = 4;
pub const __NFTA_RANGE_MAX: nft_range_attributes = 5;

pub const NFTA_RANGE_MAX: i32 = (__NFTA_RANGE_MAX - 1);

pub type nft_lookup_flags = i32;
pub const NFT_LOOKUP_F_INV: nft_lookup_flags = 1;


/**
 * enum nft_lookup_attributes - nf_tables set lookup expression netlink attributes
 *
 * @NFTA_LOOKUP_SET: name of the set where to look for (NLA_STRING)
 * @NFTA_LOOKUP_SREG: source register of the data to look for (NLA_U32: nft_registers)
 * @NFTA_LOOKUP_DREG: destination register (NLA_U32: nft_registers)
 * @NFTA_LOOKUP_SET_ID: uniquely identifies a set in a transaction (NLA_U32)
 * @NFTA_LOOKUP_FLAGS: flags (NLA_U32: enum nft_lookup_flags)
 */
pub type nft_lookup_attributes = i32;
pub const NFTA_LOOKUP_UNSPEC: nft_lookup_attributes = 0;
pub const NFTA_LOOKUP_SET: nft_lookup_attributes = 1;
pub const NFTA_LOOKUP_SREG: nft_lookup_attributes = 2;
pub const NFTA_LOOKUP_DREG: nft_lookup_attributes = 3;
pub const NFTA_LOOKUP_SET_ID: nft_lookup_attributes = 4;
pub const NFTA_LOOKUP_FLAGS: nft_lookup_attributes = 5;
pub const __NFTA_LOOKUP_MAX: nft_lookup_attributes = 6;

pub const NFTA_LOOKUP_MAX: i32 = (__NFTA_LOOKUP_MAX - 1);

pub type nft_dynset_ops = i32;
pub const NFT_DYNSET_OP_ADD: nft_dynset_ops = 0;
pub const NFT_DYNSET_OP_UPDATE: nft_dynset_ops = 1;
pub const NFT_DYNSET_OP_DELETE: nft_dynset_ops = 2;


pub type nft_dynset_flags = i32;
pub const NFT_DYNSET_F_INV: nft_dynset_flags = 1;
pub const NFT_DYNSET_F_EXPR: nft_dynset_flags = 2;


/**
 * enum nft_dynset_attributes - dynset expression attributes
 *
 * @NFTA_DYNSET_SET_NAME: name of set the to add data to (NLA_STRING)
 * @NFTA_DYNSET_SET_ID: uniquely identifier of the set in the transaction (NLA_U32)
 * @NFTA_DYNSET_OP: operation (NLA_U32)
 * @NFTA_DYNSET_SREG_KEY: source register of the key (NLA_U32)
 * @NFTA_DYNSET_SREG_DATA: source register of the data (NLA_U32)
 * @NFTA_DYNSET_TIMEOUT: timeout value for the new element (NLA_U64)
 * @NFTA_DYNSET_EXPR: expression (NLA_NESTED: nft_expr_attributes)
 * @NFTA_DYNSET_FLAGS: flags (NLA_U32)
 * @NFTA_DYNSET_EXPRESSIONS: list of expressions (NLA_NESTED: nft_list_attributes)
 */
pub type nft_dynset_attributes = i32;
pub const NFTA_DYNSET_UNSPEC: nft_dynset_attributes = 0;
pub const NFTA_DYNSET_SET_NAME: nft_dynset_attributes = 1;
pub const NFTA_DYNSET_SET_ID: nft_dynset_attributes = 2;
pub const NFTA_DYNSET_OP: nft_dynset_attributes = 3;
pub const NFTA_DYNSET_SREG_KEY: nft_dynset_attributes = 4;
pub const NFTA_DYNSET_SREG_DATA: nft_dynset_attributes = 5;
pub const NFTA_DYNSET_TIMEOUT: nft_dynset_attributes = 6;
pub const NFTA_DYNSET_EXPR: nft_dynset_attributes = 7;
pub const NFTA_DYNSET_PAD: nft_dynset_attributes = 8;
pub const NFTA_DYNSET_FLAGS: nft_dynset_attributes = 9;
pub const NFTA_DYNSET_EXPRESSIONS: nft_dynset_attributes = 10;
pub const __NFTA_DYNSET_MAX: nft_dynset_attributes = 11;

pub const NFTA_DYNSET_MAX: i32 = (__NFTA_DYNSET_MAX - 1);

/**
 * enum nft_payload_bases - nf_tables payload expression offset bases
 *
 * @NFT_PAYLOAD_LL_HEADER: link layer header
 * @NFT_PAYLOAD_NETWORK_HEADER: network header
 * @NFT_PAYLOAD_TRANSPORT_HEADER: transport header
 * @NFT_PAYLOAD_INNER_HEADER: inner header / payload
 */
pub type nft_payload_bases = i32;
pub const NFT_PAYLOAD_LL_HEADER: nft_payload_bases = 0;
pub const NFT_PAYLOAD_NETWORK_HEADER: nft_payload_bases = 1;
pub const NFT_PAYLOAD_TRANSPORT_HEADER: nft_payload_bases = 2;
pub const NFT_PAYLOAD_INNER_HEADER: nft_payload_bases = 3;
pub const NFT_PAYLOAD_TUN_HEADER: nft_payload_bases = 4;


/**
 * enum nft_payload_csum_types - nf_tables payload expression checksum types
 *
 * @NFT_PAYLOAD_CSUM_NONE: no checksumming
 * @NFT_PAYLOAD_CSUM_INET: internet checksum (RFC 791)
 * @NFT_PAYLOAD_CSUM_SCTP: CRC-32c, for use in SCTP header (RFC 3309)
 */
pub type nft_payload_csum_types = i32;
pub const NFT_PAYLOAD_CSUM_NONE: nft_payload_csum_types = 0;
pub const NFT_PAYLOAD_CSUM_INET: nft_payload_csum_types = 1;
pub const NFT_PAYLOAD_CSUM_SCTP: nft_payload_csum_types = 2;


pub type nft_payload_csum_flags = i32;
pub const NFT_PAYLOAD_L4CSUM_PSEUDOHDR: nft_payload_csum_flags = 1;


pub type nft_inner_type = i32;
pub const NFT_INNER_UNSPEC: nft_inner_type = 0;
pub const NFT_INNER_VXLAN: nft_inner_type = 1;
pub const NFT_INNER_GENEVE: nft_inner_type = 2;


pub type nft_inner_flags = i32;
pub const NFT_INNER_HDRSIZE: nft_inner_flags = 1;
pub const NFT_INNER_LL: nft_inner_flags = 2;
pub const NFT_INNER_NH: nft_inner_flags = 4;
pub const NFT_INNER_TH: nft_inner_flags = 8;

pub const NFT_INNER_MASK: i32 = (NFT_INNER_HDRSIZE | NFT_INNER_LL | NFT_INNER_NH | NFT_INNER_TH);

pub type nft_inner_attributes = i32;
pub const NFTA_INNER_UNSPEC: nft_inner_attributes = 0;
pub const NFTA_INNER_NUM: nft_inner_attributes = 1;
pub const NFTA_INNER_TYPE: nft_inner_attributes = 2;
pub const NFTA_INNER_FLAGS: nft_inner_attributes = 3;
pub const NFTA_INNER_HDRSIZE: nft_inner_attributes = 4;
pub const NFTA_INNER_EXPR: nft_inner_attributes = 5;
pub const __NFTA_INNER_MAX: nft_inner_attributes = 6;

pub const NFTA_INNER_MAX: i32 = (__NFTA_INNER_MAX - 1);

/**
 * enum nft_payload_attributes - nf_tables payload expression netlink attributes
 *
 * @NFTA_PAYLOAD_DREG: destination register to load data into (NLA_U32: nft_registers)
 * @NFTA_PAYLOAD_BASE: payload base (NLA_U32: nft_payload_bases)
 * @NFTA_PAYLOAD_OFFSET: payload offset relative to base (NLA_U32)
 * @NFTA_PAYLOAD_LEN: payload length (NLA_U32)
 * @NFTA_PAYLOAD_SREG: source register to load data from (NLA_U32: nft_registers)
 * @NFTA_PAYLOAD_CSUM_TYPE: checksum type (NLA_U32)
 * @NFTA_PAYLOAD_CSUM_OFFSET: checksum offset relative to base (NLA_U32)
 * @NFTA_PAYLOAD_CSUM_FLAGS: checksum flags (NLA_U32)
 */
pub type nft_payload_attributes = i32;
pub const NFTA_PAYLOAD_UNSPEC: nft_payload_attributes = 0;
pub const NFTA_PAYLOAD_DREG: nft_payload_attributes = 1;
pub const NFTA_PAYLOAD_BASE: nft_payload_attributes = 2;
pub const NFTA_PAYLOAD_OFFSET: nft_payload_attributes = 3;
pub const NFTA_PAYLOAD_LEN: nft_payload_attributes = 4;
pub const NFTA_PAYLOAD_SREG: nft_payload_attributes = 5;
pub const NFTA_PAYLOAD_CSUM_TYPE: nft_payload_attributes = 6;
pub const NFTA_PAYLOAD_CSUM_OFFSET: nft_payload_attributes = 7;
pub const NFTA_PAYLOAD_CSUM_FLAGS: nft_payload_attributes = 8;
pub const __NFTA_PAYLOAD_MAX: nft_payload_attributes = 9;

pub const NFTA_PAYLOAD_MAX: i32 = (__NFTA_PAYLOAD_MAX - 1);

pub type nft_exthdr_flags = i32;
pub const NFT_EXTHDR_F_PRESENT: nft_exthdr_flags = 1;


/**
 * enum nft_exthdr_op - nf_tables match options
 *
 * @NFT_EXTHDR_OP_IPV6: match against ipv6 extension headers
 * @NFT_EXTHDR_OP_TCPOPT: match against tcp options
 * @NFT_EXTHDR_OP_IPV4: match against ipv4 options
 * @NFT_EXTHDR_OP_SCTP: match against sctp chunks
 * @NFT_EXTHDR_OP_DCCP: match against dccp options
 */
pub type nft_exthdr_op = i32;
pub const NFT_EXTHDR_OP_IPV6: nft_exthdr_op = 0;
pub const NFT_EXTHDR_OP_TCPOPT: nft_exthdr_op = 1;
pub const NFT_EXTHDR_OP_IPV4: nft_exthdr_op = 2;
pub const NFT_EXTHDR_OP_SCTP: nft_exthdr_op = 3;
pub const NFT_EXTHDR_OP_DCCP: nft_exthdr_op = 4;
pub const __NFT_EXTHDR_OP_MAX: nft_exthdr_op = 5;

pub const NFT_EXTHDR_OP_MAX: i32 = (__NFT_EXTHDR_OP_MAX - 1);

/**
 * enum nft_exthdr_attributes - nf_tables extension header expression netlink attributes
 *
 * @NFTA_EXTHDR_DREG: destination register (NLA_U32: nft_registers)
 * @NFTA_EXTHDR_TYPE: extension header type (NLA_U8)
 * @NFTA_EXTHDR_OFFSET: extension header offset (NLA_U32)
 * @NFTA_EXTHDR_LEN: extension header length (NLA_U32)
 * @NFTA_EXTHDR_FLAGS: extension header flags (NLA_U32)
 * @NFTA_EXTHDR_OP: option match type (NLA_U32)
 * @NFTA_EXTHDR_SREG: source register (NLA_U32: nft_registers)
 */
pub type nft_exthdr_attributes = i32;
pub const NFTA_EXTHDR_UNSPEC: nft_exthdr_attributes = 0;
pub const NFTA_EXTHDR_DREG: nft_exthdr_attributes = 1;
pub const NFTA_EXTHDR_TYPE: nft_exthdr_attributes = 2;
pub const NFTA_EXTHDR_OFFSET: nft_exthdr_attributes = 3;
pub const NFTA_EXTHDR_LEN: nft_exthdr_attributes = 4;
pub const NFTA_EXTHDR_FLAGS: nft_exthdr_attributes = 5;
pub const NFTA_EXTHDR_OP: nft_exthdr_attributes = 6;
pub const NFTA_EXTHDR_SREG: nft_exthdr_attributes = 7;
pub const __NFTA_EXTHDR_MAX: nft_exthdr_attributes = 8;

pub const NFTA_EXTHDR_MAX: i32 = (__NFTA_EXTHDR_MAX - 1);

/**
 * enum nft_meta_keys - nf_tables meta expression keys
 *
 * @NFT_META_LEN: packet length (skb->len)
 * @NFT_META_PROTOCOL: packet ethertype protocol (skb->protocol), invalid in OUTPUT
 * @NFT_META_PRIORITY: packet priority (skb->priority)
 * @NFT_META_MARK: packet mark (skb->mark)
 * @NFT_META_IIF: packet input interface index (dev->ifindex)
 * @NFT_META_OIF: packet output interface index (dev->ifindex)
 * @NFT_META_IIFNAME: packet input interface name (dev->name)
 * @NFT_META_OIFNAME: packet output interface name (dev->name)
 * @NFT_META_IIFTYPE: packet input interface type (dev->type)
 * @NFT_META_OIFTYPE: packet output interface type (dev->type)
 * @NFT_META_SKUID: originating socket UID (fsuid)
 * @NFT_META_SKGID: originating socket GID (fsgid)
 * @NFT_META_NFTRACE: packet nftrace bit
 * @NFT_META_RTCLASSID: realm value of packet's route (skb->dst->tclassid)
 * @NFT_META_SECMARK: packet secmark (skb->secmark)
 * @NFT_META_NFPROTO: netfilter protocol
 * @NFT_META_L4PROTO: layer 4 protocol number
 * @NFT_META_BRI_IIFNAME: packet input bridge interface name
 * @NFT_META_BRI_OIFNAME: packet output bridge interface name
 * @NFT_META_PKTTYPE: packet type (skb->pkt_type), special handling for loopback
 * @NFT_META_CPU: cpu id through smp_processor_id()
 * @NFT_META_IIFGROUP: packet input interface group
 * @NFT_META_OIFGROUP: packet output interface group
 * @NFT_META_CGROUP: socket control group (skb->sk->sk_classid)
 * @NFT_META_PRANDOM: a 32bit pseudo-random number
 * @NFT_META_SECPATH: boolean, secpath_exists (!!skb->sp)
 * @NFT_META_IIFKIND: packet input interface kind name (dev->rtnl_link_ops->kind)
 * @NFT_META_OIFKIND: packet output interface kind name (dev->rtnl_link_ops->kind)
 * @NFT_META_BRI_IIFPVID: packet input bridge port pvid
 * @NFT_META_BRI_IIFVPROTO: packet input bridge vlan proto
 * @NFT_META_TIME_NS: time since epoch (in nanoseconds)
 * @NFT_META_TIME_DAY: day of week (from 0 = Sunday to 6 = Saturday)
 * @NFT_META_TIME_HOUR: hour of day (in seconds)
 * @NFT_META_SDIF: slave device interface index
 * @NFT_META_SDIFNAME: slave device interface name
 * @NFT_META_BRI_BROUTE: packet br_netfilter_broute bit
 * @NFT_META_BRI_IIFHWADDR: packet input bridge interface ethernet address
 */
pub type nft_meta_keys = i32;
pub const NFT_META_LEN: nft_meta_keys = 0;
pub const NFT_META_PROTOCOL: nft_meta_keys = 1;
pub const NFT_META_PRIORITY: nft_meta_keys = 2;
pub const NFT_META_MARK: nft_meta_keys = 3;
pub const NFT_META_IIF: nft_meta_keys = 4;
pub const NFT_META_OIF: nft_meta_keys = 5;
pub const NFT_META_IIFNAME: nft_meta_keys = 6;
pub const NFT_META_OIFNAME: nft_meta_keys = 7;
pub const NFT_META_IFTYPE: nft_meta_keys = 8;
pub const NFT_META_IIFTYPE: nft_meta_keys = NFT_META_IFTYPE;
pub const NFT_META_OIFTYPE: nft_meta_keys = 9;
pub const NFT_META_SKUID: nft_meta_keys = 10;
pub const NFT_META_SKGID: nft_meta_keys = 11;
pub const NFT_META_NFTRACE: nft_meta_keys = 12;
pub const NFT_META_RTCLASSID: nft_meta_keys = 13;
pub const NFT_META_SECMARK: nft_meta_keys = 14;
pub const NFT_META_NFPROTO: nft_meta_keys = 15;
pub const NFT_META_L4PROTO: nft_meta_keys = 16;
pub const NFT_META_BRI_IIFNAME: nft_meta_keys = 17;
pub const NFT_META_BRI_OIFNAME: nft_meta_keys = 18;
pub const NFT_META_PKTTYPE: nft_meta_keys = 19;
pub const NFT_META_CPU: nft_meta_keys = 20;
pub const NFT_META_IIFGROUP: nft_meta_keys = 21;
pub const NFT_META_OIFGROUP: nft_meta_keys = 22;
pub const NFT_META_CGROUP: nft_meta_keys = 23;
pub const NFT_META_PRANDOM: nft_meta_keys = 24;
pub const NFT_META_SECPATH: nft_meta_keys = 25;
pub const NFT_META_IIFKIND: nft_meta_keys = 26;
pub const NFT_META_OIFKIND: nft_meta_keys = 27;
pub const NFT_META_BRI_IIFPVID: nft_meta_keys = 28;
pub const NFT_META_BRI_IIFVPROTO: nft_meta_keys = 29;
pub const NFT_META_TIME_NS: nft_meta_keys = 30;
pub const NFT_META_TIME_DAY: nft_meta_keys = 31;
pub const NFT_META_TIME_HOUR: nft_meta_keys = 32;
pub const NFT_META_SDIF: nft_meta_keys = 33;
pub const NFT_META_SDIFNAME: nft_meta_keys = 34;
pub const NFT_META_BRI_BROUTE: nft_meta_keys = 35;
pub const __NFT_META_IIFTYPE: nft_meta_keys = 36;
pub const NFT_META_BRI_IIFHWADDR: nft_meta_keys = 37;


/**
 * enum nft_rt_keys - nf_tables routing expression keys
 *
 * @NFT_RT_CLASSID: realm value of packet's route (skb->dst->tclassid)
 * @NFT_RT_NEXTHOP4: routing nexthop for IPv4
 * @NFT_RT_NEXTHOP6: routing nexthop for IPv6
 * @NFT_RT_TCPMSS: fetch current path tcp mss
 * @NFT_RT_XFRM: boolean, skb->dst->xfrm != NULL
 */
pub type nft_rt_keys = i32;
pub const NFT_RT_CLASSID: nft_rt_keys = 0;
pub const NFT_RT_NEXTHOP4: nft_rt_keys = 1;
pub const NFT_RT_NEXTHOP6: nft_rt_keys = 2;
pub const NFT_RT_TCPMSS: nft_rt_keys = 3;
pub const NFT_RT_XFRM: nft_rt_keys = 4;
pub const __NFT_RT_MAX: nft_rt_keys = 5;

pub const NFT_RT_MAX: i32 = (__NFT_RT_MAX - 1);

/**
 * enum nft_hash_types - nf_tables hash expression types
 *
 * @NFT_HASH_JENKINS: Jenkins Hash
 * @NFT_HASH_SYM: Symmetric Hash
 */
pub type nft_hash_types = i32;
pub const NFT_HASH_JENKINS: nft_hash_types = 0;
pub const NFT_HASH_SYM: nft_hash_types = 1;


/**
 * enum nft_hash_attributes - nf_tables hash expression netlink attributes
 *
 * @NFTA_HASH_SREG: source register (NLA_U32)
 * @NFTA_HASH_DREG: destination register (NLA_U32)
 * @NFTA_HASH_LEN: source data length (NLA_U32)
 * @NFTA_HASH_MODULUS: modulus value (NLA_U32)
 * @NFTA_HASH_SEED: seed value (NLA_U32)
 * @NFTA_HASH_OFFSET: add this offset value to hash result (NLA_U32)
 * @NFTA_HASH_TYPE: hash operation (NLA_U32: nft_hash_types)
 * @NFTA_HASH_SET_NAME: name of the map to lookup (NLA_STRING)
 * @NFTA_HASH_SET_ID: id of the map (NLA_U32)
 */
pub type nft_hash_attributes = i32;
pub const NFTA_HASH_UNSPEC: nft_hash_attributes = 0;
pub const NFTA_HASH_SREG: nft_hash_attributes = 1;
pub const NFTA_HASH_DREG: nft_hash_attributes = 2;
pub const NFTA_HASH_LEN: nft_hash_attributes = 3;
pub const NFTA_HASH_MODULUS: nft_hash_attributes = 4;
pub const NFTA_HASH_SEED: nft_hash_attributes = 5;
pub const NFTA_HASH_OFFSET: nft_hash_attributes = 6;
pub const NFTA_HASH_TYPE: nft_hash_attributes = 7;
pub const NFTA_HASH_SET_NAME: nft_hash_attributes = 8;
pub const NFTA_HASH_SET_ID: nft_hash_attributes = 9;
pub const __NFTA_HASH_MAX: nft_hash_attributes = 10;

pub const NFTA_HASH_MAX: i32 = (__NFTA_HASH_MAX - 1);

/**
 * enum nft_meta_attributes - nf_tables meta expression netlink attributes
 *
 * @NFTA_META_DREG: destination register (NLA_U32)
 * @NFTA_META_KEY: meta data item to load (NLA_U32: nft_meta_keys)
 * @NFTA_META_SREG: source register (NLA_U32)
 */
pub type nft_meta_attributes = i32;
pub const NFTA_META_UNSPEC: nft_meta_attributes = 0;
pub const NFTA_META_DREG: nft_meta_attributes = 1;
pub const NFTA_META_KEY: nft_meta_attributes = 2;
pub const NFTA_META_SREG: nft_meta_attributes = 3;
pub const __NFTA_META_MAX: nft_meta_attributes = 4;

pub const NFTA_META_MAX: i32 = (__NFTA_META_MAX - 1);

/**
 * enum nft_rt_attributes - nf_tables routing expression netlink attributes
 *
 * @NFTA_RT_DREG: destination register (NLA_U32)
 * @NFTA_RT_KEY: routing data item to load (NLA_U32: nft_rt_keys)
 */
pub type nft_rt_attributes = i32;
pub const NFTA_RT_UNSPEC: nft_rt_attributes = 0;
pub const NFTA_RT_DREG: nft_rt_attributes = 1;
pub const NFTA_RT_KEY: nft_rt_attributes = 2;
pub const __NFTA_RT_MAX: nft_rt_attributes = 3;

pub const NFTA_RT_MAX: i32 = (__NFTA_RT_MAX - 1);

/**
 * enum nft_socket_attributes - nf_tables socket expression netlink attributes
 *
 * @NFTA_SOCKET_KEY: socket key to match
 * @NFTA_SOCKET_DREG: destination register
 * @NFTA_SOCKET_LEVEL: cgroups2 ancestor level (only for cgroupsv2)
 */
pub type nft_socket_attributes = i32;
pub const NFTA_SOCKET_UNSPEC: nft_socket_attributes = 0;
pub const NFTA_SOCKET_KEY: nft_socket_attributes = 1;
pub const NFTA_SOCKET_DREG: nft_socket_attributes = 2;
pub const NFTA_SOCKET_LEVEL: nft_socket_attributes = 3;
pub const __NFTA_SOCKET_MAX: nft_socket_attributes = 4;

pub const NFTA_SOCKET_MAX: i32 = (__NFTA_SOCKET_MAX - 1);

/*
 * enum nft_socket_keys - nf_tables socket expression keys
 *
 * @NFT_SOCKET_TRANSPARENT: Value of the IP(V6)_TRANSPARENT socket option
 * @NFT_SOCKET_MARK: Value of the socket mark
 * @NFT_SOCKET_WILDCARD: Whether the socket is zero-bound (e.g. 0.0.0.0 or ::0)
 * @NFT_SOCKET_CGROUPV2: Match on cgroups version 2
 */
pub type nft_socket_keys = i32;
pub const NFT_SOCKET_TRANSPARENT: nft_socket_keys = 0;
pub const NFT_SOCKET_MARK: nft_socket_keys = 1;
pub const NFT_SOCKET_WILDCARD: nft_socket_keys = 2;
pub const NFT_SOCKET_CGROUPV2: nft_socket_keys = 3;
pub const __NFT_SOCKET_MAX: nft_socket_keys = 4;

pub const NFT_SOCKET_MAX: i32 = (__NFT_SOCKET_MAX - 1);

/**
 * enum nft_ct_keys - nf_tables ct expression keys
 *
 * @NFT_CT_STATE: conntrack state (bitmask of enum ip_conntrack_info)
 * @NFT_CT_DIRECTION: conntrack direction (enum ip_conntrack_dir)
 * @NFT_CT_STATUS: conntrack status (bitmask of enum ip_conntrack_status)
 * @NFT_CT_MARK: conntrack mark value
 * @NFT_CT_SECMARK: conntrack secmark value
 * @NFT_CT_EXPIRATION: relative conntrack expiration time in ms
 * @NFT_CT_HELPER: connection tracking helper assigned to conntrack
 * @NFT_CT_L3PROTOCOL: conntrack layer 3 protocol
 * @NFT_CT_SRC: conntrack layer 3 protocol source (IPv4/IPv6 address, deprecated)
 * @NFT_CT_DST: conntrack layer 3 protocol destination (IPv4/IPv6 address, deprecated)
 * @NFT_CT_PROTOCOL: conntrack layer 4 protocol
 * @NFT_CT_PROTO_SRC: conntrack layer 4 protocol source
 * @NFT_CT_PROTO_DST: conntrack layer 4 protocol destination
 * @NFT_CT_LABELS: conntrack labels
 * @NFT_CT_PKTS: conntrack packets
 * @NFT_CT_BYTES: conntrack bytes
 * @NFT_CT_AVGPKT: conntrack average bytes per packet
 * @NFT_CT_ZONE: conntrack zone
 * @NFT_CT_EVENTMASK: ctnetlink events to be generated for this conntrack
 * @NFT_CT_SRC_IP: conntrack layer 3 protocol source (IPv4 address)
 * @NFT_CT_DST_IP: conntrack layer 3 protocol destination (IPv4 address)
 * @NFT_CT_SRC_IP6: conntrack layer 3 protocol source (IPv6 address)
 * @NFT_CT_DST_IP6: conntrack layer 3 protocol destination (IPv6 address)
 * @NFT_CT_ID: conntrack id
 */
pub type nft_ct_keys = i32;
pub const NFT_CT_STATE: nft_ct_keys = 0;
pub const NFT_CT_DIRECTION: nft_ct_keys = 1;
pub const NFT_CT_STATUS: nft_ct_keys = 2;
pub const NFT_CT_MARK: nft_ct_keys = 3;
pub const NFT_CT_SECMARK: nft_ct_keys = 4;
pub const NFT_CT_EXPIRATION: nft_ct_keys = 5;
pub const NFT_CT_HELPER: nft_ct_keys = 6;
pub const NFT_CT_L3PROTOCOL: nft_ct_keys = 7;
pub const NFT_CT_SRC: nft_ct_keys = 8;
pub const NFT_CT_DST: nft_ct_keys = 9;
pub const NFT_CT_PROTOCOL: nft_ct_keys = 10;
pub const NFT_CT_PROTO_SRC: nft_ct_keys = 11;
pub const NFT_CT_PROTO_DST: nft_ct_keys = 12;
pub const NFT_CT_LABELS: nft_ct_keys = 13;
pub const NFT_CT_PKTS: nft_ct_keys = 14;
pub const NFT_CT_BYTES: nft_ct_keys = 15;
pub const NFT_CT_AVGPKT: nft_ct_keys = 16;
pub const NFT_CT_ZONE: nft_ct_keys = 17;
pub const NFT_CT_EVENTMASK: nft_ct_keys = 18;
pub const NFT_CT_SRC_IP: nft_ct_keys = 19;
pub const NFT_CT_DST_IP: nft_ct_keys = 20;
pub const NFT_CT_SRC_IP6: nft_ct_keys = 21;
pub const NFT_CT_DST_IP6: nft_ct_keys = 22;
pub const NFT_CT_ID: nft_ct_keys = 23;
pub const __NFT_CT_MAX: nft_ct_keys = 24;

pub const NFT_CT_MAX: i32 = (__NFT_CT_MAX - 1);

/**
 * enum nft_ct_attributes - nf_tables ct expression netlink attributes
 *
 * @NFTA_CT_DREG: destination register (NLA_U32)
 * @NFTA_CT_KEY: conntrack data item to load (NLA_U32: nft_ct_keys)
 * @NFTA_CT_DIRECTION: direction in case of directional keys (NLA_U8)
 * @NFTA_CT_SREG: source register (NLA_U32)
 */
pub type nft_ct_attributes = i32;
pub const NFTA_CT_UNSPEC: nft_ct_attributes = 0;
pub const NFTA_CT_DREG: nft_ct_attributes = 1;
pub const NFTA_CT_KEY: nft_ct_attributes = 2;
pub const NFTA_CT_DIRECTION: nft_ct_attributes = 3;
pub const NFTA_CT_SREG: nft_ct_attributes = 4;
pub const __NFTA_CT_MAX: nft_ct_attributes = 5;

pub const NFTA_CT_MAX: i32 = (__NFTA_CT_MAX - 1);

/**
 * enum nft_offload_attributes - ct offload expression attributes
 * @NFTA_FLOW_TABLE_NAME: flow table name (NLA_STRING)
 */
pub type nft_offload_attributes = i32;
pub const NFTA_FLOW_UNSPEC: nft_offload_attributes = 0;
pub const NFTA_FLOW_TABLE_NAME: nft_offload_attributes = 1;
pub const __NFTA_FLOW_MAX: nft_offload_attributes = 2;

pub const NFTA_FLOW_MAX: i32 = (__NFTA_FLOW_MAX - 1);

pub type nft_limit_type = i32;
pub const NFT_LIMIT_PKTS: nft_limit_type = 0;
pub const NFT_LIMIT_PKT_BYTES: nft_limit_type = 1;


pub type nft_limit_flags = i32;
pub const NFT_LIMIT_F_INV: nft_limit_flags = 1;


/**
 * enum nft_limit_attributes - nf_tables limit expression netlink attributes
 *
 * @NFTA_LIMIT_RATE: refill rate (NLA_U64)
 * @NFTA_LIMIT_UNIT: refill unit (NLA_U64)
 * @NFTA_LIMIT_BURST: burst (NLA_U32)
 * @NFTA_LIMIT_TYPE: type of limit (NLA_U32: enum nft_limit_type)
 * @NFTA_LIMIT_FLAGS: flags (NLA_U32: enum nft_limit_flags)
 */
pub type nft_limit_attributes = i32;
pub const NFTA_LIMIT_UNSPEC: nft_limit_attributes = 0;
pub const NFTA_LIMIT_RATE: nft_limit_attributes = 1;
pub const NFTA_LIMIT_UNIT: nft_limit_attributes = 2;
pub const NFTA_LIMIT_BURST: nft_limit_attributes = 3;
pub const NFTA_LIMIT_TYPE: nft_limit_attributes = 4;
pub const NFTA_LIMIT_FLAGS: nft_limit_attributes = 5;
pub const NFTA_LIMIT_PAD: nft_limit_attributes = 6;
pub const __NFTA_LIMIT_MAX: nft_limit_attributes = 7;

pub const NFTA_LIMIT_MAX: i32 = (__NFTA_LIMIT_MAX - 1);

pub type nft_connlimit_flags = i32;
pub const NFT_CONNLIMIT_F_INV: nft_connlimit_flags = 1;


/**
 * enum nft_connlimit_attributes - nf_tables connlimit expression netlink attributes
 *
 * @NFTA_CONNLIMIT_COUNT: number of connections (NLA_U32)
 * @NFTA_CONNLIMIT_FLAGS: flags (NLA_U32: enum nft_connlimit_flags)
 */
pub type nft_connlimit_attributes = i32;
pub const NFTA_CONNLIMIT_UNSPEC: nft_connlimit_attributes = 0;
pub const NFTA_CONNLIMIT_COUNT: nft_connlimit_attributes = 1;
pub const NFTA_CONNLIMIT_FLAGS: nft_connlimit_attributes = 2;
pub const __NFTA_CONNLIMIT_MAX: nft_connlimit_attributes = 3;

pub const NFTA_CONNLIMIT_MAX: i32 = (__NFTA_CONNLIMIT_MAX - 1);

/**
 * enum nft_counter_attributes - nf_tables counter expression netlink attributes
 *
 * @NFTA_COUNTER_BYTES: number of bytes (NLA_U64)
 * @NFTA_COUNTER_PACKETS: number of packets (NLA_U64)
 */
pub type nft_counter_attributes = i32;
pub const NFTA_COUNTER_UNSPEC: nft_counter_attributes = 0;
pub const NFTA_COUNTER_BYTES: nft_counter_attributes = 1;
pub const NFTA_COUNTER_PACKETS: nft_counter_attributes = 2;
pub const NFTA_COUNTER_PAD: nft_counter_attributes = 3;
pub const __NFTA_COUNTER_MAX: nft_counter_attributes = 4;

pub const NFTA_COUNTER_MAX: i32 = (__NFTA_COUNTER_MAX - 1);

/**
 * enum nft_last_attributes - nf_tables last expression netlink attributes
 *
 * @NFTA_LAST_SET: last update has been set, zero means never updated (NLA_U32)
 * @NFTA_LAST_MSECS: milliseconds since last update (NLA_U64)
 */
pub type nft_last_attributes = i32;
pub const NFTA_LAST_UNSPEC: nft_last_attributes = 0;
pub const NFTA_LAST_SET: nft_last_attributes = 1;
pub const NFTA_LAST_MSECS: nft_last_attributes = 2;
pub const NFTA_LAST_PAD: nft_last_attributes = 3;
pub const __NFTA_LAST_MAX: nft_last_attributes = 4;

pub const NFTA_LAST_MAX: i32 = (__NFTA_LAST_MAX - 1);

/**
 * enum nft_log_attributes - nf_tables log expression netlink attributes
 *
 * @NFTA_LOG_GROUP: netlink group to send messages to (NLA_U16)
 * @NFTA_LOG_PREFIX: prefix to prepend to log messages (NLA_STRING)
 * @NFTA_LOG_SNAPLEN: length of payload to include in netlink message (NLA_U32)
 * @NFTA_LOG_QTHRESHOLD: queue threshold (NLA_U16)
 * @NFTA_LOG_LEVEL: log level (NLA_U32)
 * @NFTA_LOG_FLAGS: logging flags (NLA_U32)
 */
pub type nft_log_attributes = i32;
pub const NFTA_LOG_UNSPEC: nft_log_attributes = 0;
pub const NFTA_LOG_GROUP: nft_log_attributes = 1;
pub const NFTA_LOG_PREFIX: nft_log_attributes = 2;
pub const NFTA_LOG_SNAPLEN: nft_log_attributes = 3;
pub const NFTA_LOG_QTHRESHOLD: nft_log_attributes = 4;
pub const NFTA_LOG_LEVEL: nft_log_attributes = 5;
pub const NFTA_LOG_FLAGS: nft_log_attributes = 6;
pub const __NFTA_LOG_MAX: nft_log_attributes = 7;

pub const NFTA_LOG_MAX: i32 = (__NFTA_LOG_MAX - 1);

/**
 * enum nft_log_level - nf_tables log levels
 *
 * @NFT_LOGLEVEL_EMERG: system is unusable
 * @NFT_LOGLEVEL_ALERT: action must be taken immediately
 * @NFT_LOGLEVEL_CRIT: critical conditions
 * @NFT_LOGLEVEL_ERR: error conditions
 * @NFT_LOGLEVEL_WARNING: warning conditions
 * @NFT_LOGLEVEL_NOTICE: normal but significant condition
 * @NFT_LOGLEVEL_INFO: informational
 * @NFT_LOGLEVEL_DEBUG: debug-level messages
 * @NFT_LOGLEVEL_AUDIT: enabling audit logging
 */
pub type nft_log_level = i32;
pub const NFT_LOGLEVEL_EMERG: nft_log_level = 0;
pub const NFT_LOGLEVEL_ALERT: nft_log_level = 1;
pub const NFT_LOGLEVEL_CRIT: nft_log_level = 2;
pub const NFT_LOGLEVEL_ERR: nft_log_level = 3;
pub const NFT_LOGLEVEL_WARNING: nft_log_level = 4;
pub const NFT_LOGLEVEL_NOTICE: nft_log_level = 5;
pub const NFT_LOGLEVEL_INFO: nft_log_level = 6;
pub const NFT_LOGLEVEL_DEBUG: nft_log_level = 7;
pub const NFT_LOGLEVEL_AUDIT: nft_log_level = 8;
pub const __NFT_LOGLEVEL_MAX: nft_log_level = 9;

pub const NFT_LOGLEVEL_MAX: i32 = (__NFT_LOGLEVEL_MAX - 1);

/**
 * enum nft_queue_attributes - nf_tables queue expression netlink attributes
 *
 * @NFTA_QUEUE_NUM: netlink queue to send messages to (NLA_U16)
 * @NFTA_QUEUE_TOTAL: number of queues to load balance packets on (NLA_U16)
 * @NFTA_QUEUE_FLAGS: various flags (NLA_U16)
 * @NFTA_QUEUE_SREG_QNUM: source register of queue number (NLA_U32: nft_registers)
 */
pub type nft_queue_attributes = i32;
pub const NFTA_QUEUE_UNSPEC: nft_queue_attributes = 0;
pub const NFTA_QUEUE_NUM: nft_queue_attributes = 1;
pub const NFTA_QUEUE_TOTAL: nft_queue_attributes = 2;
pub const NFTA_QUEUE_FLAGS: nft_queue_attributes = 3;
pub const NFTA_QUEUE_SREG_QNUM: nft_queue_attributes = 4;
pub const __NFTA_QUEUE_MAX: nft_queue_attributes = 5;

pub const NFTA_QUEUE_MAX: i32 = (__NFTA_QUEUE_MAX - 1);

pub const NFT_QUEUE_FLAG_BYPASS: i32 = 0x01 /* for compatibility with v2 */;
pub const NFT_QUEUE_FLAG_CPU_FANOUT: i32 = 0x02 /* use crrent CPU (no hashing) */;
pub const NFT_QUEUE_FLAG_MASK: i32 = 0x03;

pub type nft_quota_flags = i32;
pub const NFT_QUOTA_F_INV: nft_quota_flags = 1;
pub const NFT_QUOTA_F_DEPLETED: nft_quota_flags = 2;


/**
 * enum nft_quota_attributes - nf_tables quota expression netlink attributes
 *
 * @NFTA_QUOTA_BYTES: quota in bytes (NLA_U16)
 * @NFTA_QUOTA_FLAGS: flags (NLA_U32)
 * @NFTA_QUOTA_CONSUMED: quota already consumed in bytes (NLA_U64)
 */
pub type nft_quota_attributes = i32;
pub const NFTA_QUOTA_UNSPEC: nft_quota_attributes = 0;
pub const NFTA_QUOTA_BYTES: nft_quota_attributes = 1;
pub const NFTA_QUOTA_FLAGS: nft_quota_attributes = 2;
pub const NFTA_QUOTA_PAD: nft_quota_attributes = 3;
pub const NFTA_QUOTA_CONSUMED: nft_quota_attributes = 4;
pub const __NFTA_QUOTA_MAX: nft_quota_attributes = 5;

pub const NFTA_QUOTA_MAX: i32 = (__NFTA_QUOTA_MAX - 1);

/**
 * enum nft_secmark_attributes - nf_tables secmark object netlink attributes
 *
 * @NFTA_SECMARK_CTX: security context (NLA_STRING)
 */
pub type nft_secmark_attributes = i32;
pub const NFTA_SECMARK_UNSPEC: nft_secmark_attributes = 0;
pub const NFTA_SECMARK_CTX: nft_secmark_attributes = 1;
pub const __NFTA_SECMARK_MAX: nft_secmark_attributes = 2;

pub const NFTA_SECMARK_MAX: i32 = (__NFTA_SECMARK_MAX - 1);

/* Max security context length */
pub const NFT_SECMARK_CTX_MAXLEN: u32 = 4096;

/**
 * enum nft_reject_types - nf_tables reject expression reject types
 *
 * @NFT_REJECT_ICMP_UNREACH: reject using ICMP unreachable
 * @NFT_REJECT_TCP_RST: reject using TCP RST
 * @NFT_REJECT_ICMPX_UNREACH: abstracted ICMP unreachable for bridge and inet
 */
pub type nft_reject_types = i32;
pub const NFT_REJECT_ICMP_UNREACH: nft_reject_types = 0;
pub const NFT_REJECT_TCP_RST: nft_reject_types = 1;
pub const NFT_REJECT_ICMPX_UNREACH: nft_reject_types = 2;


/**
 * enum nft_reject_inet_code - Generic reject codes for IPv4/IPv6
 *
 * @NFT_REJECT_ICMPX_NO_ROUTE: no route to host / network unreachable
 * @NFT_REJECT_ICMPX_PORT_UNREACH: port unreachable
 * @NFT_REJECT_ICMPX_HOST_UNREACH: host unreachable
 * @NFT_REJECT_ICMPX_ADMIN_PROHIBITED: administratively prohibited
 *
 * These codes are mapped to real ICMP and ICMPv6 codes.
 */
pub type nft_reject_inet_code = i32;
pub const NFT_REJECT_ICMPX_NO_ROUTE: nft_reject_inet_code = 0;
pub const NFT_REJECT_ICMPX_PORT_UNREACH: nft_reject_inet_code = 1;
pub const NFT_REJECT_ICMPX_HOST_UNREACH: nft_reject_inet_code = 2;
pub const NFT_REJECT_ICMPX_ADMIN_PROHIBITED: nft_reject_inet_code = 3;
pub const __NFT_REJECT_ICMPX_MAX: nft_reject_inet_code = 4;

pub const NFT_REJECT_ICMPX_MAX: i32 = (__NFT_REJECT_ICMPX_MAX - 1);

/**
 * enum nft_reject_attributes - nf_tables reject expression netlink attributes
 *
 * @NFTA_REJECT_TYPE: packet type to use (NLA_U32: nft_reject_types)
 * @NFTA_REJECT_ICMP_CODE: ICMP code to use (NLA_U8)
 */
pub type nft_reject_attributes = i32;
pub const NFTA_REJECT_UNSPEC: nft_reject_attributes = 0;
pub const NFTA_REJECT_TYPE: nft_reject_attributes = 1;
pub const NFTA_REJECT_ICMP_CODE: nft_reject_attributes = 2;
pub const __NFTA_REJECT_MAX: nft_reject_attributes = 3;

pub const NFTA_REJECT_MAX: i32 = (__NFTA_REJECT_MAX - 1);

/**
 * enum nft_nat_types - nf_tables nat expression NAT types
 *
 * @NFT_NAT_SNAT: source NAT
 * @NFT_NAT_DNAT: destination NAT
 */
pub type nft_nat_types = i32;
pub const NFT_NAT_SNAT: nft_nat_types = 0;
pub const NFT_NAT_DNAT: nft_nat_types = 1;


/**
 * enum nft_nat_attributes - nf_tables nat expression netlink attributes
 *
 * @NFTA_NAT_TYPE: NAT type (NLA_U32: nft_nat_types)
 * @NFTA_NAT_FAMILY: NAT family (NLA_U32)
 * @NFTA_NAT_REG_ADDR_MIN: source register of address range start (NLA_U32: nft_registers)
 * @NFTA_NAT_REG_ADDR_MAX: source register of address range end (NLA_U32: nft_registers)
 * @NFTA_NAT_REG_PROTO_MIN: source register of proto range start (NLA_U32: nft_registers)
 * @NFTA_NAT_REG_PROTO_MAX: source register of proto range end (NLA_U32: nft_registers)
 * @NFTA_NAT_FLAGS: NAT flags (see NF_NAT_RANGE_* in linux/netfilter/nf_nat.h) (NLA_U32)
 */
pub type nft_nat_attributes = i32;
pub const NFTA_NAT_UNSPEC: nft_nat_attributes = 0;
pub const NFTA_NAT_TYPE: nft_nat_attributes = 1;
pub const NFTA_NAT_FAMILY: nft_nat_attributes = 2;
pub const NFTA_NAT_REG_ADDR_MIN: nft_nat_attributes = 3;
pub const NFTA_NAT_REG_ADDR_MAX: nft_nat_attributes = 4;
pub const NFTA_NAT_REG_PROTO_MIN: nft_nat_attributes = 5;
pub const NFTA_NAT_REG_PROTO_MAX: nft_nat_attributes = 6;
pub const NFTA_NAT_FLAGS: nft_nat_attributes = 7;
pub const __NFTA_NAT_MAX: nft_nat_attributes = 8;

pub const NFTA_NAT_MAX: i32 = (__NFTA_NAT_MAX - 1);

/**
 * enum nft_tproxy_attributes - nf_tables tproxy expression netlink attributes
 *
 * @NFTA_TPROXY_FAMILY: Target address family (NLA_U32: nft_registers)
 * @NFTA_TPROXY_REG_ADDR: Target address register (NLA_U32: nft_registers)
 * @NFTA_TPROXY_REG_PORT: Target port register (NLA_U32: nft_registers)
 */
pub type nft_tproxy_attributes = i32;
pub const NFTA_TPROXY_UNSPEC: nft_tproxy_attributes = 0;
pub const NFTA_TPROXY_FAMILY: nft_tproxy_attributes = 1;
pub const NFTA_TPROXY_REG_ADDR: nft_tproxy_attributes = 2;
pub const NFTA_TPROXY_REG_PORT: nft_tproxy_attributes = 3;
pub const __NFTA_TPROXY_MAX: nft_tproxy_attributes = 4;

pub const NFTA_TPROXY_MAX: i32 = (__NFTA_TPROXY_MAX - 1);

/**
 * enum nft_masq_attributes - nf_tables masquerade expression attributes
 *
 * @NFTA_MASQ_FLAGS: NAT flags (see NF_NAT_RANGE_* in linux/netfilter/nf_nat.h) (NLA_U32)
 * @NFTA_MASQ_REG_PROTO_MIN: source register of proto range start (NLA_U32: nft_registers)
 * @NFTA_MASQ_REG_PROTO_MAX: source register of proto range end (NLA_U32: nft_registers)
 */
pub type nft_masq_attributes = i32;
pub const NFTA_MASQ_UNSPEC: nft_masq_attributes = 0;
pub const NFTA_MASQ_FLAGS: nft_masq_attributes = 1;
pub const NFTA_MASQ_REG_PROTO_MIN: nft_masq_attributes = 2;
pub const NFTA_MASQ_REG_PROTO_MAX: nft_masq_attributes = 3;
pub const __NFTA_MASQ_MAX: nft_masq_attributes = 4;

pub const NFTA_MASQ_MAX: i32 = (__NFTA_MASQ_MAX - 1);

/**
 * enum nft_redir_attributes - nf_tables redirect expression netlink attributes
 *
 * @NFTA_REDIR_REG_PROTO_MIN: source register of proto range start (NLA_U32: nft_registers)
 * @NFTA_REDIR_REG_PROTO_MAX: source register of proto range end (NLA_U32: nft_registers)
 * @NFTA_REDIR_FLAGS: NAT flags (see NF_NAT_RANGE_* in linux/netfilter/nf_nat.h) (NLA_U32)
 */
pub type nft_redir_attributes = i32;
pub const NFTA_REDIR_UNSPEC: nft_redir_attributes = 0;
pub const NFTA_REDIR_REG_PROTO_MIN: nft_redir_attributes = 1;
pub const NFTA_REDIR_REG_PROTO_MAX: nft_redir_attributes = 2;
pub const NFTA_REDIR_FLAGS: nft_redir_attributes = 3;
pub const __NFTA_REDIR_MAX: nft_redir_attributes = 4;

pub const NFTA_REDIR_MAX: i32 = (__NFTA_REDIR_MAX - 1);

/**
 * enum nft_dup_attributes - nf_tables dup expression netlink attributes
 *
 * @NFTA_DUP_SREG_ADDR: source register of address (NLA_U32: nft_registers)
 * @NFTA_DUP_SREG_DEV: source register of output interface (NLA_U32: nft_register)
 */
pub type nft_dup_attributes = i32;
pub const NFTA_DUP_UNSPEC: nft_dup_attributes = 0;
pub const NFTA_DUP_SREG_ADDR: nft_dup_attributes = 1;
pub const NFTA_DUP_SREG_DEV: nft_dup_attributes = 2;
pub const __NFTA_DUP_MAX: nft_dup_attributes = 3;

pub const NFTA_DUP_MAX: i32 = (__NFTA_DUP_MAX - 1);

/**
 * enum nft_fwd_attributes - nf_tables fwd expression netlink attributes
 *
 * @NFTA_FWD_SREG_DEV: source register of output interface (NLA_U32: nft_register)
 * @NFTA_FWD_SREG_ADDR: source register of destination address (NLA_U32: nft_register)
 * @NFTA_FWD_NFPROTO: layer 3 family of source register address (NLA_U32: enum nfproto)
 */
pub type nft_fwd_attributes = i32;
pub const NFTA_FWD_UNSPEC: nft_fwd_attributes = 0;
pub const NFTA_FWD_SREG_DEV: nft_fwd_attributes = 1;
pub const NFTA_FWD_SREG_ADDR: nft_fwd_attributes = 2;
pub const NFTA_FWD_NFPROTO: nft_fwd_attributes = 3;
pub const __NFTA_FWD_MAX: nft_fwd_attributes = 4;

pub const NFTA_FWD_MAX: i32 = (__NFTA_FWD_MAX - 1);

/**
 * enum nft_objref_attributes - nf_tables stateful object expression netlink attributes
 *
 * @NFTA_OBJREF_IMM_TYPE: object type for immediate reference (NLA_U32: nft_register)
 * @NFTA_OBJREF_IMM_NAME: object name for immediate reference (NLA_STRING)
 * @NFTA_OBJREF_SET_SREG: source register of the data to look for (NLA_U32: nft_registers)
 * @NFTA_OBJREF_SET_NAME: name of the set where to look for (NLA_STRING)
 * @NFTA_OBJREF_SET_ID: id of the set where to look for in this transaction (NLA_U32)
 */
pub type nft_objref_attributes = i32;
pub const NFTA_OBJREF_UNSPEC: nft_objref_attributes = 0;
pub const NFTA_OBJREF_IMM_TYPE: nft_objref_attributes = 1;
pub const NFTA_OBJREF_IMM_NAME: nft_objref_attributes = 2;
pub const NFTA_OBJREF_SET_SREG: nft_objref_attributes = 3;
pub const NFTA_OBJREF_SET_NAME: nft_objref_attributes = 4;
pub const NFTA_OBJREF_SET_ID: nft_objref_attributes = 5;
pub const __NFTA_OBJREF_MAX: nft_objref_attributes = 6;

pub const NFTA_OBJREF_MAX: i32 = (__NFTA_OBJREF_MAX - 1);

/**
 * enum nft_gen_attributes - nf_tables ruleset generation attributes
 *
 * @NFTA_GEN_ID: Ruleset generation ID (NLA_U32)
 */
pub type nft_gen_attributes = i32;
pub const NFTA_GEN_UNSPEC: nft_gen_attributes = 0;
pub const NFTA_GEN_ID: nft_gen_attributes = 1;
pub const NFTA_GEN_PROC_PID: nft_gen_attributes = 2;
pub const NFTA_GEN_PROC_NAME: nft_gen_attributes = 3;
pub const __NFTA_GEN_MAX: nft_gen_attributes = 4;

pub const NFTA_GEN_MAX: i32 = (__NFTA_GEN_MAX - 1);

/*
 * enum nft_fib_attributes - nf_tables fib expression netlink attributes
 *
 * @NFTA_FIB_DREG: destination register (NLA_U32)
 * @NFTA_FIB_RESULT: desired result (NLA_U32)
 * @NFTA_FIB_FLAGS: flowi fields to initialize when querying the FIB (NLA_U32)
 *
 * The FIB expression performs a route lookup according
 * to the packet data.
 */
pub type nft_fib_attributes = i32;
pub const NFTA_FIB_UNSPEC: nft_fib_attributes = 0;
pub const NFTA_FIB_DREG: nft_fib_attributes = 1;
pub const NFTA_FIB_RESULT: nft_fib_attributes = 2;
pub const NFTA_FIB_FLAGS: nft_fib_attributes = 3;
pub const __NFTA_FIB_MAX: nft_fib_attributes = 4;

pub const NFTA_FIB_MAX: i32 = (__NFTA_FIB_MAX - 1);

pub type nft_fib_result = i32;
pub const NFT_FIB_RESULT_UNSPEC: nft_fib_result = 0;
pub const NFT_FIB_RESULT_OIF: nft_fib_result = 1;
pub const NFT_FIB_RESULT_OIFNAME: nft_fib_result = 2;
pub const NFT_FIB_RESULT_ADDRTYPE: nft_fib_result = 3;
pub const __NFT_FIB_RESULT_MAX: nft_fib_result = 4;

pub const NFT_FIB_RESULT_MAX: i32 = (__NFT_FIB_RESULT_MAX - 1);

pub type nft_fib_flags = i32;
pub const NFTA_FIB_F_SADDR: nft_fib_flags = 1;
pub const NFTA_FIB_F_DADDR: nft_fib_flags = 2;
pub const NFTA_FIB_F_MARK: nft_fib_flags = 4;
pub const NFTA_FIB_F_IIF: nft_fib_flags = 8;
pub const NFTA_FIB_F_OIF: nft_fib_flags = 16;
pub const NFTA_FIB_F_PRESENT: nft_fib_flags = 32;


pub type nft_ct_helper_attributes = i32;
pub const NFTA_CT_HELPER_UNSPEC: nft_ct_helper_attributes = 0;
pub const NFTA_CT_HELPER_NAME: nft_ct_helper_attributes = 1;
pub const NFTA_CT_HELPER_L3PROTO: nft_ct_helper_attributes = 2;
pub const NFTA_CT_HELPER_L4PROTO: nft_ct_helper_attributes = 3;
pub const __NFTA_CT_HELPER_MAX: nft_ct_helper_attributes = 4;

pub const NFTA_CT_HELPER_MAX: i32 = (__NFTA_CT_HELPER_MAX - 1);

pub type nft_ct_timeout_timeout_attributes = i32;
pub const NFTA_CT_TIMEOUT_UNSPEC: nft_ct_timeout_timeout_attributes = 0;
pub const NFTA_CT_TIMEOUT_L3PROTO: nft_ct_timeout_timeout_attributes = 1;
pub const NFTA_CT_TIMEOUT_L4PROTO: nft_ct_timeout_timeout_attributes = 2;
pub const NFTA_CT_TIMEOUT_DATA: nft_ct_timeout_timeout_attributes = 3;
pub const __NFTA_CT_TIMEOUT_MAX: nft_ct_timeout_timeout_attributes = 4;

pub const NFTA_CT_TIMEOUT_MAX: i32 = (__NFTA_CT_TIMEOUT_MAX - 1);

pub type nft_ct_expectation_attributes = i32;
pub const NFTA_CT_EXPECT_UNSPEC: nft_ct_expectation_attributes = 0;
pub const NFTA_CT_EXPECT_L3PROTO: nft_ct_expectation_attributes = 1;
pub const NFTA_CT_EXPECT_L4PROTO: nft_ct_expectation_attributes = 2;
pub const NFTA_CT_EXPECT_DPORT: nft_ct_expectation_attributes = 3;
pub const NFTA_CT_EXPECT_TIMEOUT: nft_ct_expectation_attributes = 4;
pub const NFTA_CT_EXPECT_SIZE: nft_ct_expectation_attributes = 5;
pub const __NFTA_CT_EXPECT_MAX: nft_ct_expectation_attributes = 6;

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

/**
 * enum nft_object_attributes - nf_tables stateful object netlink attributes
 *
 * @NFTA_OBJ_TABLE: name of the table containing the expression (NLA_STRING)
 * @NFTA_OBJ_NAME: name of this expression type (NLA_STRING)
 * @NFTA_OBJ_TYPE: stateful object type (NLA_U32)
 * @NFTA_OBJ_DATA: stateful object data (NLA_NESTED)
 * @NFTA_OBJ_USE: number of references to this expression (NLA_U32)
 * @NFTA_OBJ_HANDLE: object handle (NLA_U64)
 * @NFTA_OBJ_USERDATA: user data (NLA_BINARY)
 */
pub type nft_object_attributes = i32;
pub const NFTA_OBJ_UNSPEC: nft_object_attributes = 0;
pub const NFTA_OBJ_TABLE: nft_object_attributes = 1;
pub const NFTA_OBJ_NAME: nft_object_attributes = 2;
pub const NFTA_OBJ_TYPE: nft_object_attributes = 3;
pub const NFTA_OBJ_DATA: nft_object_attributes = 4;
pub const NFTA_OBJ_USE: nft_object_attributes = 5;
pub const NFTA_OBJ_HANDLE: nft_object_attributes = 6;
pub const NFTA_OBJ_PAD: nft_object_attributes = 7;
pub const NFTA_OBJ_USERDATA: nft_object_attributes = 8;
pub const __NFTA_OBJ_MAX: nft_object_attributes = 9;

pub const NFTA_OBJ_MAX: i32 = (__NFTA_OBJ_MAX - 1);

/**
 * enum nft_flowtable_flags - nf_tables flowtable flags
 *
 * @NFT_FLOWTABLE_HW_OFFLOAD: flowtable hardware offload is enabled
 * @NFT_FLOWTABLE_COUNTER: enable flow counters
 */
pub type nft_flowtable_flags = i32;
pub const NFT_FLOWTABLE_HW_OFFLOAD: nft_flowtable_flags = 1;
pub const NFT_FLOWTABLE_COUNTER: nft_flowtable_flags = 2;
pub const NFT_FLOWTABLE_MASK: nft_flowtable_flags = 3;


/**
 * enum nft_flowtable_attributes - nf_tables flow table netlink attributes
 *
 * @NFTA_FLOWTABLE_TABLE: name of the table containing the expression (NLA_STRING)
 * @NFTA_FLOWTABLE_NAME: name of this flow table (NLA_STRING)
 * @NFTA_FLOWTABLE_HOOK: netfilter hook configuration (NLA_NESTED)
 * @NFTA_FLOWTABLE_USE: number of references to this flow table (NLA_U32)
 * @NFTA_FLOWTABLE_HANDLE: object handle (NLA_U64)
 * @NFTA_FLOWTABLE_FLAGS: flags (NLA_U32)
 */
pub type nft_flowtable_attributes = i32;
pub const NFTA_FLOWTABLE_UNSPEC: nft_flowtable_attributes = 0;
pub const NFTA_FLOWTABLE_TABLE: nft_flowtable_attributes = 1;
pub const NFTA_FLOWTABLE_NAME: nft_flowtable_attributes = 2;
pub const NFTA_FLOWTABLE_HOOK: nft_flowtable_attributes = 3;
pub const NFTA_FLOWTABLE_USE: nft_flowtable_attributes = 4;
pub const NFTA_FLOWTABLE_HANDLE: nft_flowtable_attributes = 5;
pub const NFTA_FLOWTABLE_PAD: nft_flowtable_attributes = 6;
pub const NFTA_FLOWTABLE_FLAGS: nft_flowtable_attributes = 7;
pub const __NFTA_FLOWTABLE_MAX: nft_flowtable_attributes = 8;

pub const NFTA_FLOWTABLE_MAX: i32 = (__NFTA_FLOWTABLE_MAX - 1);

/**
 * enum nft_flowtable_hook_attributes - nf_tables flow table hook netlink attributes
 *
 * @NFTA_FLOWTABLE_HOOK_NUM: netfilter hook number (NLA_U32)
 * @NFTA_FLOWTABLE_HOOK_PRIORITY: netfilter hook priority (NLA_U32)
 * @NFTA_FLOWTABLE_HOOK_DEVS: input devices this flow table is bound to (NLA_NESTED)
 */
pub type nft_flowtable_hook_attributes = i32;
pub const NFTA_FLOWTABLE_HOOK_UNSPEC: nft_flowtable_hook_attributes = 0;
pub const NFTA_FLOWTABLE_HOOK_NUM: nft_flowtable_hook_attributes = 1;
pub const NFTA_FLOWTABLE_HOOK_PRIORITY: nft_flowtable_hook_attributes = 2;
pub const NFTA_FLOWTABLE_HOOK_DEVS: nft_flowtable_hook_attributes = 3;
pub const __NFTA_FLOWTABLE_HOOK_MAX: nft_flowtable_hook_attributes = 4;

pub const NFTA_FLOWTABLE_HOOK_MAX: i32 = (__NFTA_FLOWTABLE_HOOK_MAX - 1);

/**
 * enum nft_osf_attributes - nftables osf expression netlink attributes
 *
 * @NFTA_OSF_DREG: destination register (NLA_U32: nft_registers)
 * @NFTA_OSF_TTL: Value of the TTL osf option (NLA_U8)
 * @NFTA_OSF_FLAGS: flags (NLA_U32)
 */
pub type nft_osf_attributes = i32;
pub const NFTA_OSF_UNSPEC: nft_osf_attributes = 0;
pub const NFTA_OSF_DREG: nft_osf_attributes = 1;
pub const NFTA_OSF_TTL: nft_osf_attributes = 2;
pub const NFTA_OSF_FLAGS: nft_osf_attributes = 3;
pub const __NFTA_OSF_MAX: nft_osf_attributes = 4;

pub const NFTA_OSF_MAX: i32 = (__NFTA_OSF_MAX - 1);

pub type nft_osf_flags = i32;
pub const NFT_OSF_F_VERSION: nft_osf_flags = 1;


/**
 * enum nft_synproxy_attributes - nf_tables synproxy expression netlink attributes
 *
 * @NFTA_SYNPROXY_MSS: mss value sent to the backend (NLA_U16)
 * @NFTA_SYNPROXY_WSCALE: wscale value sent to the backend (NLA_U8)
 * @NFTA_SYNPROXY_FLAGS: flags (NLA_U32)
 */
pub type nft_synproxy_attributes = i32;
pub const NFTA_SYNPROXY_UNSPEC: nft_synproxy_attributes = 0;
pub const NFTA_SYNPROXY_MSS: nft_synproxy_attributes = 1;
pub const NFTA_SYNPROXY_WSCALE: nft_synproxy_attributes = 2;
pub const NFTA_SYNPROXY_FLAGS: nft_synproxy_attributes = 3;
pub const __NFTA_SYNPROXY_MAX: nft_synproxy_attributes = 4;

pub const NFTA_SYNPROXY_MAX: i32 = (__NFTA_SYNPROXY_MAX - 1);

/**
 * enum nft_devices_attributes - nf_tables device netlink attributes
 *
 * @NFTA_DEVICE_NAME: name of this device (NLA_STRING)
 * @NFTA_DEVICE_PREFIX: device name prefix, a simple wildcard (NLA_STRING)
 */
pub type nft_devices_attributes = i32;
pub const NFTA_DEVICE_UNSPEC: nft_devices_attributes = 0;
pub const NFTA_DEVICE_NAME: nft_devices_attributes = 1;
pub const NFTA_DEVICE_PREFIX: nft_devices_attributes = 2;
pub const __NFTA_DEVICE_MAX: nft_devices_attributes = 3;

pub const NFTA_DEVICE_MAX: i32 = (__NFTA_DEVICE_MAX - 1);

/*
 * enum nft_xfrm_attributes - nf_tables xfrm expr netlink attributes
 *
 * @NFTA_XFRM_DREG: destination register (NLA_U32)
 * @NFTA_XFRM_KEY: enum nft_xfrm_keys (NLA_U32)
 * @NFTA_XFRM_DIR: direction (NLA_U8)
 * @NFTA_XFRM_SPNUM: index in secpath array (NLA_U32)
 */
pub type nft_xfrm_attributes = i32;
pub const NFTA_XFRM_UNSPEC: nft_xfrm_attributes = 0;
pub const NFTA_XFRM_DREG: nft_xfrm_attributes = 1;
pub const NFTA_XFRM_KEY: nft_xfrm_attributes = 2;
pub const NFTA_XFRM_DIR: nft_xfrm_attributes = 3;
pub const NFTA_XFRM_SPNUM: nft_xfrm_attributes = 4;
pub const __NFTA_XFRM_MAX: nft_xfrm_attributes = 5;

pub const NFTA_XFRM_MAX: i32 = (__NFTA_XFRM_MAX - 1);

pub type nft_xfrm_keys = i32;
pub const NFT_XFRM_KEY_UNSPEC: nft_xfrm_keys = 0;
pub const NFT_XFRM_KEY_DADDR_IP4: nft_xfrm_keys = 1;
pub const NFT_XFRM_KEY_DADDR_IP6: nft_xfrm_keys = 2;
pub const NFT_XFRM_KEY_SADDR_IP4: nft_xfrm_keys = 3;
pub const NFT_XFRM_KEY_SADDR_IP6: nft_xfrm_keys = 4;
pub const NFT_XFRM_KEY_REQID: nft_xfrm_keys = 5;
pub const NFT_XFRM_KEY_SPI: nft_xfrm_keys = 6;
pub const __NFT_XFRM_KEY_MAX: nft_xfrm_keys = 7;

pub const NFT_XFRM_KEY_MAX: i32 = (__NFT_XFRM_KEY_MAX - 1);

/**
 * enum nft_trace_attributes - nf_tables trace netlink attributes
 *
 * @NFTA_TRACE_TABLE: name of the table (NLA_STRING)
 * @NFTA_TRACE_CHAIN: name of the chain (NLA_STRING)
 * @NFTA_TRACE_RULE_HANDLE: numeric handle of the rule (NLA_U64)
 * @NFTA_TRACE_TYPE: type of the event (NLA_U32: nft_trace_types)
 * @NFTA_TRACE_VERDICT: verdict returned by hook (NLA_NESTED: nft_verdicts)
 * @NFTA_TRACE_ID: pseudo-id, same for each skb traced (NLA_U32)
 * @NFTA_TRACE_LL_HEADER: linklayer header (NLA_BINARY)
 * @NFTA_TRACE_NETWORK_HEADER: network header (NLA_BINARY)
 * @NFTA_TRACE_TRANSPORT_HEADER: transport header (NLA_BINARY)
 * @NFTA_TRACE_IIF: indev ifindex (NLA_U32)
 * @NFTA_TRACE_IIFTYPE: netdev->type of indev (NLA_U16)
 * @NFTA_TRACE_OIF: outdev ifindex (NLA_U32)
 * @NFTA_TRACE_OIFTYPE: netdev->type of outdev (NLA_U16)
 * @NFTA_TRACE_MARK: nfmark (NLA_U32)
 * @NFTA_TRACE_NFPROTO: nf protocol processed (NLA_U32)
 * @NFTA_TRACE_POLICY: policy that decided fate of packet (NLA_U32)
 * @NFTA_TRACE_CT_ID: conntrack id (NLA_U32)
 * @NFTA_TRACE_CT_DIRECTION: packets direction (NLA_U8)
 * @NFTA_TRACE_CT_STATUS: conntrack status (NLA_U32)
 * @NFTA_TRACE_CT_STATE: packet state (new, established, ...) (NLA_U32)
 */
pub type nft_trace_attributes = i32;
pub const NFTA_TRACE_UNSPEC: nft_trace_attributes = 0;
pub const NFTA_TRACE_TABLE: nft_trace_attributes = 1;
pub const NFTA_TRACE_CHAIN: nft_trace_attributes = 2;
pub const NFTA_TRACE_RULE_HANDLE: nft_trace_attributes = 3;
pub const NFTA_TRACE_TYPE: nft_trace_attributes = 4;
pub const NFTA_TRACE_VERDICT: nft_trace_attributes = 5;
pub const NFTA_TRACE_ID: nft_trace_attributes = 6;
pub const NFTA_TRACE_LL_HEADER: nft_trace_attributes = 7;
pub const NFTA_TRACE_NETWORK_HEADER: nft_trace_attributes = 8;
pub const NFTA_TRACE_TRANSPORT_HEADER: nft_trace_attributes = 9;
pub const NFTA_TRACE_IIF: nft_trace_attributes = 10;
pub const NFTA_TRACE_IIFTYPE: nft_trace_attributes = 11;
pub const NFTA_TRACE_OIF: nft_trace_attributes = 12;
pub const NFTA_TRACE_OIFTYPE: nft_trace_attributes = 13;
pub const NFTA_TRACE_MARK: nft_trace_attributes = 14;
pub const NFTA_TRACE_NFPROTO: nft_trace_attributes = 15;
pub const NFTA_TRACE_POLICY: nft_trace_attributes = 16;
pub const NFTA_TRACE_PAD: nft_trace_attributes = 17;
pub const NFTA_TRACE_CT_ID: nft_trace_attributes = 18;
pub const NFTA_TRACE_CT_DIRECTION: nft_trace_attributes = 19;
pub const NFTA_TRACE_CT_STATUS: nft_trace_attributes = 20;
pub const NFTA_TRACE_CT_STATE: nft_trace_attributes = 21;
pub const __NFTA_TRACE_MAX: nft_trace_attributes = 22;

pub const NFTA_TRACE_MAX: i32 = (__NFTA_TRACE_MAX - 1);

pub type nft_trace_types = i32;
pub const NFT_TRACETYPE_UNSPEC: nft_trace_types = 0;
pub const NFT_TRACETYPE_POLICY: nft_trace_types = 1;
pub const NFT_TRACETYPE_RETURN: nft_trace_types = 2;
pub const NFT_TRACETYPE_RULE: nft_trace_types = 3;
pub const __NFT_TRACETYPE_MAX: nft_trace_types = 4;

pub const NFT_TRACETYPE_MAX: i32 = (__NFT_TRACETYPE_MAX - 1);

/**
 * enum nft_ng_attributes - nf_tables number generator expression netlink attributes
 *
 * @NFTA_NG_DREG: destination register (NLA_U32)
 * @NFTA_NG_MODULUS: maximum counter value (NLA_U32)
 * @NFTA_NG_TYPE: operation type (NLA_U32)
 * @NFTA_NG_OFFSET: offset to be added to the counter (NLA_U32)
 * @NFTA_NG_SET_NAME: name of the map to lookup (NLA_STRING)
 * @NFTA_NG_SET_ID: id of the map (NLA_U32)
 */
pub type nft_ng_attributes = i32;
pub const NFTA_NG_UNSPEC: nft_ng_attributes = 0;
pub const NFTA_NG_DREG: nft_ng_attributes = 1;
pub const NFTA_NG_MODULUS: nft_ng_attributes = 2;
pub const NFTA_NG_TYPE: nft_ng_attributes = 3;
pub const NFTA_NG_OFFSET: nft_ng_attributes = 4;
pub const NFTA_NG_SET_NAME: nft_ng_attributes = 5;
pub const NFTA_NG_SET_ID: nft_ng_attributes = 6;
pub const __NFTA_NG_MAX: nft_ng_attributes = 7;

pub const NFTA_NG_MAX: i32 = (__NFTA_NG_MAX - 1);

pub type nft_ng_types = i32;
pub const NFT_NG_INCREMENTAL: nft_ng_types = 0;
pub const NFT_NG_RANDOM: nft_ng_types = 1;
pub const __NFT_NG_MAX: nft_ng_types = 2;

pub const NFT_NG_MAX: i32 = (__NFT_NG_MAX - 1);

pub type nft_tunnel_key_ip_attributes = i32;
pub const NFTA_TUNNEL_KEY_IP_UNSPEC: nft_tunnel_key_ip_attributes = 0;
pub const NFTA_TUNNEL_KEY_IP_SRC: nft_tunnel_key_ip_attributes = 1;
pub const NFTA_TUNNEL_KEY_IP_DST: nft_tunnel_key_ip_attributes = 2;
pub const __NFTA_TUNNEL_KEY_IP_MAX: nft_tunnel_key_ip_attributes = 3;

pub const NFTA_TUNNEL_KEY_IP_MAX: i32 = (__NFTA_TUNNEL_KEY_IP_MAX - 1);

pub type nft_tunnel_ip6_attributes = i32;
pub const NFTA_TUNNEL_KEY_IP6_UNSPEC: nft_tunnel_ip6_attributes = 0;
pub const NFTA_TUNNEL_KEY_IP6_SRC: nft_tunnel_ip6_attributes = 1;
pub const NFTA_TUNNEL_KEY_IP6_DST: nft_tunnel_ip6_attributes = 2;
pub const NFTA_TUNNEL_KEY_IP6_FLOWLABEL: nft_tunnel_ip6_attributes = 3;
pub const __NFTA_TUNNEL_KEY_IP6_MAX: nft_tunnel_ip6_attributes = 4;

pub const NFTA_TUNNEL_KEY_IP6_MAX: i32 = (__NFTA_TUNNEL_KEY_IP6_MAX - 1);

pub type nft_tunnel_opts_attributes = i32;
pub const NFTA_TUNNEL_KEY_OPTS_UNSPEC: nft_tunnel_opts_attributes = 0;
pub const NFTA_TUNNEL_KEY_OPTS_VXLAN: nft_tunnel_opts_attributes = 1;
pub const NFTA_TUNNEL_KEY_OPTS_ERSPAN: nft_tunnel_opts_attributes = 2;
pub const NFTA_TUNNEL_KEY_OPTS_GENEVE: nft_tunnel_opts_attributes = 3;
pub const __NFTA_TUNNEL_KEY_OPTS_MAX: nft_tunnel_opts_attributes = 4;

pub const NFTA_TUNNEL_KEY_OPTS_MAX: i32 = (__NFTA_TUNNEL_KEY_OPTS_MAX - 1);

pub type nft_tunnel_opts_vxlan_attributes = i32;
pub const NFTA_TUNNEL_KEY_VXLAN_UNSPEC: nft_tunnel_opts_vxlan_attributes = 0;
pub const NFTA_TUNNEL_KEY_VXLAN_GBP: nft_tunnel_opts_vxlan_attributes = 1;
pub const __NFTA_TUNNEL_KEY_VXLAN_MAX: nft_tunnel_opts_vxlan_attributes = 2;

pub const NFTA_TUNNEL_KEY_VXLAN_MAX: i32 = (__NFTA_TUNNEL_KEY_VXLAN_MAX - 1);

pub type nft_tunnel_opts_erspan_attributes = i32;
pub const NFTA_TUNNEL_KEY_ERSPAN_UNSPEC: nft_tunnel_opts_erspan_attributes = 0;
pub const NFTA_TUNNEL_KEY_ERSPAN_VERSION: nft_tunnel_opts_erspan_attributes = 1;
pub const NFTA_TUNNEL_KEY_ERSPAN_V1_INDEX: nft_tunnel_opts_erspan_attributes = 2;
pub const NFTA_TUNNEL_KEY_ERSPAN_V2_HWID: nft_tunnel_opts_erspan_attributes = 3;
pub const NFTA_TUNNEL_KEY_ERSPAN_V2_DIR: nft_tunnel_opts_erspan_attributes = 4;
pub const __NFTA_TUNNEL_KEY_ERSPAN_MAX: nft_tunnel_opts_erspan_attributes = 5;

pub const NFTA_TUNNEL_KEY_ERSPAN_MAX: i32 = (__NFTA_TUNNEL_KEY_ERSPAN_MAX - 1);

pub type nft_tunnel_opts_geneve_attributes = i32;
pub const NFTA_TUNNEL_KEY_GENEVE_UNSPEC: nft_tunnel_opts_geneve_attributes = 0;
pub const NFTA_TUNNEL_KEY_GENEVE_CLASS: nft_tunnel_opts_geneve_attributes = 1;
pub const NFTA_TUNNEL_KEY_GENEVE_TYPE: nft_tunnel_opts_geneve_attributes = 2;
pub const NFTA_TUNNEL_KEY_GENEVE_DATA: nft_tunnel_opts_geneve_attributes = 3;
pub const __NFTA_TUNNEL_KEY_GENEVE_MAX: nft_tunnel_opts_geneve_attributes = 4;

pub const NFTA_TUNNEL_KEY_GENEVE_MAX: i32 = (__NFTA_TUNNEL_KEY_GENEVE_MAX - 1);

pub type nft_tunnel_flags = i32;
pub const NFT_TUNNEL_F_ZERO_CSUM_TX: nft_tunnel_flags = 1;
pub const NFT_TUNNEL_F_DONT_FRAGMENT: nft_tunnel_flags = 2;
pub const NFT_TUNNEL_F_SEQ_NUMBER: nft_tunnel_flags = 4;

pub const NFT_TUNNEL_F_MASK: i32 = (NFT_TUNNEL_F_ZERO_CSUM_TX | NFT_TUNNEL_F_DONT_FRAGMENT | NFT_TUNNEL_F_SEQ_NUMBER);

pub type nft_tunnel_key_attributes = i32;
pub const NFTA_TUNNEL_KEY_UNSPEC: nft_tunnel_key_attributes = 0;
pub const NFTA_TUNNEL_KEY_ID: nft_tunnel_key_attributes = 1;
pub const NFTA_TUNNEL_KEY_IP: nft_tunnel_key_attributes = 2;
pub const NFTA_TUNNEL_KEY_IP6: nft_tunnel_key_attributes = 3;
pub const NFTA_TUNNEL_KEY_FLAGS: nft_tunnel_key_attributes = 4;
pub const NFTA_TUNNEL_KEY_TOS: nft_tunnel_key_attributes = 5;
pub const NFTA_TUNNEL_KEY_TTL: nft_tunnel_key_attributes = 6;
pub const NFTA_TUNNEL_KEY_SPORT: nft_tunnel_key_attributes = 7;
pub const NFTA_TUNNEL_KEY_DPORT: nft_tunnel_key_attributes = 8;
pub const NFTA_TUNNEL_KEY_OPTS: nft_tunnel_key_attributes = 9;
pub const __NFTA_TUNNEL_KEY_MAX: nft_tunnel_key_attributes = 10;

pub const NFTA_TUNNEL_KEY_MAX: i32 = (__NFTA_TUNNEL_KEY_MAX - 1);

pub type nft_tunnel_keys = i32;
pub const NFT_TUNNEL_PATH: nft_tunnel_keys = 0;
pub const NFT_TUNNEL_ID: nft_tunnel_keys = 1;
pub const __NFT_TUNNEL_MAX: nft_tunnel_keys = 2;

pub const NFT_TUNNEL_MAX: i32 = (__NFT_TUNNEL_MAX - 1);

pub type nft_tunnel_mode = i32;
pub const NFT_TUNNEL_MODE_NONE: nft_tunnel_mode = 0;
pub const NFT_TUNNEL_MODE_RX: nft_tunnel_mode = 1;
pub const NFT_TUNNEL_MODE_TX: nft_tunnel_mode = 2;
pub const __NFT_TUNNEL_MODE_MAX: nft_tunnel_mode = 3;

pub const NFT_TUNNEL_MODE_MAX: i32 = (__NFT_TUNNEL_MODE_MAX - 1);

pub type nft_tunnel_attributes = i32;
pub const NFTA_TUNNEL_UNSPEC: nft_tunnel_attributes = 0;
pub const NFTA_TUNNEL_KEY: nft_tunnel_attributes = 1;
pub const NFTA_TUNNEL_DREG: nft_tunnel_attributes = 2;
pub const NFTA_TUNNEL_MODE: nft_tunnel_attributes = 3;
pub const __NFTA_TUNNEL_MAX: nft_tunnel_attributes = 4;

pub const NFTA_TUNNEL_MAX: i32 = (__NFTA_TUNNEL_MAX - 1);
