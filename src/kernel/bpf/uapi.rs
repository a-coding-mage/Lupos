//! linux-parity: partial
//! linux-source: vendor/linux/kernel/bpf
//! UAPI constants from `vendor/linux/include/uapi/linux/bpf.h`.

// ── BPF syscall subcommands (`enum bpf_cmd`) ────────────────────────────────
pub const BPF_MAP_CREATE: u32 = 0;
pub const BPF_MAP_LOOKUP_ELEM: u32 = 1;
pub const BPF_MAP_UPDATE_ELEM: u32 = 2;
pub const BPF_MAP_DELETE_ELEM: u32 = 3;
pub const BPF_MAP_GET_NEXT_KEY: u32 = 4;
pub const BPF_PROG_LOAD: u32 = 5;
pub const BPF_OBJ_PIN: u32 = 6;
pub const BPF_OBJ_GET: u32 = 7;
pub const BPF_PROG_ATTACH: u32 = 8;
pub const BPF_PROG_DETACH: u32 = 9;
pub const BPF_PROG_TEST_RUN: u32 = 10;
pub const BPF_PROG_RUN: u32 = BPF_PROG_TEST_RUN;
pub const BPF_MAP_LOOKUP_AND_DELETE_ELEM: u32 = 21;
pub const BPF_MAP_FREEZE: u32 = 22;

// ── BPF_MAP_TYPE_* ─────────────────────────────────────────────────────────
pub const BPF_MAP_TYPE_UNSPEC: u32 = 0;
pub const BPF_MAP_TYPE_HASH: u32 = 1;
pub const BPF_MAP_TYPE_ARRAY: u32 = 2;
pub const BPF_MAP_TYPE_STACK_TRACE: u32 = 7;
pub const BPF_MAP_TYPE_LPM_TRIE: u32 = 11;
pub const BPF_MAP_TYPE_QUEUE: u32 = 22;
pub const BPF_MAP_TYPE_STACK: u32 = 23;
pub const BPF_MAP_TYPE_RINGBUF: u32 = 27;
pub const BPF_MAP_TYPE_BLOOM_FILTER: u32 = 30;

pub const BPF_ANY: u64 = 0;
pub const BPF_NOEXIST: u64 = 1;
pub const BPF_EXIST: u64 = 2;
pub const BPF_F_LOCK: u64 = 4;

// BPF_MAP_CREATE flags used by the translated kernel/bpf map families.
pub const BPF_F_NO_PREALLOC: u64 = 1 << 0;
pub const BPF_F_NUMA_NODE: u64 = 1 << 2;
pub const BPF_F_RDONLY: u64 = 1 << 3;
pub const BPF_F_WRONLY: u64 = 1 << 4;
pub const BPF_F_STACK_BUILD_ID: u64 = 1 << 5;
pub const BPF_F_ZERO_SEED: u64 = 1 << 6;
pub const BPF_F_RDONLY_PROG: u64 = 1 << 7;
pub const BPF_F_WRONLY_PROG: u64 = 1 << 8;
pub const BPF_F_MMAPABLE: u64 = 1 << 10;
pub const BPF_F_PRESERVE_ELEMS: u64 = 1 << 11;
pub const BPF_F_INNER_MAP: u64 = 1 << 12;
pub const BPF_F_RB_OVERWRITE: u64 = 1 << 19;
pub const BPF_F_ACCESS_MASK: u64 =
    BPF_F_RDONLY | BPF_F_RDONLY_PROG | BPF_F_WRONLY | BPF_F_WRONLY_PROG;

// Ring-buffer helper flags and query selectors.
pub const BPF_RB_NO_WAKEUP: u64 = 1 << 0;
pub const BPF_RB_FORCE_WAKEUP: u64 = 1 << 1;
pub const BPF_RB_AVAIL_DATA: u64 = 0;
pub const BPF_RB_RING_SIZE: u64 = 1;
pub const BPF_RB_CONS_POS: u64 = 2;
pub const BPF_RB_PROD_POS: u64 = 3;
pub const BPF_RB_OVERWRITE_POS: u64 = 4;
pub const BPF_RINGBUF_BUSY_BIT: u32 = 1 << 31;
pub const BPF_RINGBUF_DISCARD_BIT: u32 = 1 << 30;
pub const BPF_RINGBUF_HDR_SZ: u64 = 8;

// ── BPF_PROG_TYPE_* ────────────────────────────────────────────────────────
pub const BPF_PROG_TYPE_UNSPEC: u32 = 0;
pub const BPF_PROG_TYPE_SOCKET_FILTER: u32 = 1;
pub const BPF_PROG_TYPE_KPROBE: u32 = 2;
pub const BPF_PROG_TYPE_TRACEPOINT: u32 = 5;
pub const BPF_PROG_TYPE_CGROUP_SKB: u32 = 8;
pub const BPF_PROG_TYPE_CGROUP_DEVICE: u32 = 15;

// ── BPF attach types (`enum bpf_attach_type`) ───────────────────────────────
pub const BPF_CGROUP_INET_INGRESS: u32 = 0;
pub const BPF_CGROUP_INET_EGRESS: u32 = 1;
pub const BPF_CGROUP_DEVICE: u32 = 6;

pub const BPF_F_ALLOW_OVERRIDE: u32 = 1 << 0;
pub const BPF_F_ALLOW_MULTI: u32 = 1 << 1;
pub const BPF_F_REPLACE: u32 = 1 << 2;

// ── Helper function IDs (`enum bpf_func_id`) ────────────────────────────────
pub const BPF_FUNC_unspec: u32 = 0;
pub const BPF_FUNC_map_lookup_elem: u32 = 1;
pub const BPF_FUNC_map_update_elem: u32 = 2;
pub const BPF_FUNC_map_delete_elem: u32 = 3;
pub const BPF_FUNC_get_current_pid_tgid: u32 = 14;
pub const BPF_FUNC_ktime_get_ns: u32 = 5;
pub const BPF_FUNC_trace_printk: u32 = 6;
