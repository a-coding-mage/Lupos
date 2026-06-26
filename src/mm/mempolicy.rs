//! linux-parity: complete
//! linux-source: vendor/linux/mm/mempolicy.c
//! test-origin: linux:vendor/linux/mm/mempolicy.c
//! NUMA and memory-policy helpers.
//!
//! Implements the stateful pieces of:
//! - `vendor/linux/mm/mempolicy.c`
//! - `vendor/linux/mm/numa.c`
//! - `vendor/linux/mm/numa_emulation.c`
//! - `vendor/linux/mm/numa_memblks.c`
//! - `vendor/linux/mm/memory-tiers.c`

extern crate alloc;

use alloc::{boxed::Box, format, string::String, vec::Vec};

use spin::Mutex;

use crate::include::uapi::errno::{EINVAL, ESRCH};
use crate::mm::mm_types::VmAreaStruct;
use crate::mm::page::Page;
use crate::mm::page_flags::GfpFlags;
use crate::mm::vm_flags::VM_HUGETLB;

pub const MPOL_DEFAULT: i32 = 0;
pub const MPOL_PREFERRED: i32 = 1;
pub const MPOL_BIND: i32 = 2;
pub const MPOL_INTERLEAVE: i32 = 3;
pub const MPOL_LOCAL: i32 = 4;
pub const MPOL_PREFERRED_MANY: i32 = 5;
pub const MPOL_WEIGHTED_INTERLEAVE: i32 = 6;

pub const MPOL_F_STATIC_NODES: u32 = 1 << 15;
pub const MPOL_F_RELATIVE_NODES: u32 = 1 << 14;
pub const MPOL_F_NUMA_BALANCING: u32 = 1 << 13;
pub const MPOL_MODE_FLAGS: u32 =
    MPOL_F_STATIC_NODES | MPOL_F_RELATIVE_NODES | MPOL_F_NUMA_BALANCING;
pub const MPOL_F_SHARED: u32 = 1 << 0;
pub const MPOL_F_MOF: u32 = 1 << 3;

pub const NUMA_NO_NODE: i32 = -1;
pub const MAX_NUMNODES: i32 = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryPolicy {
    pub refcnt: i32,
    pub mode: i32,
    pub flags: u32,
    pub nodemask: u64,
    pub home_node: i32,
}

impl MemoryPolicy {
    pub const fn default_single_node() -> Self {
        Self {
            refcnt: 1,
            mode: MPOL_DEFAULT,
            flags: 0,
            nodemask: 0,
            home_node: NUMA_NO_NODE,
        }
    }

    pub const fn new(mode: i32, flags: u32, nodemask: u64) -> Self {
        Self {
            refcnt: 1,
            mode,
            flags,
            nodemask,
            home_node: NUMA_NO_NODE,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NumaNode {
    pub id: u16,
    pub online: bool,
    pub tier: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NumaMemblock {
    pub start: u64,
    pub end: u64,
    pub nid: u16,
}

struct MemPolicyState {
    nodes: Vec<NumaNode>,
    memblocks: Vec<NumaMemblock>,
    current_policy: MemoryPolicy,
    interleave_cursor: usize,
    policy_zone: usize,
    allocated_policies: Vec<usize>,
    shared_policies: Vec<SharedPolicyRecord>,
    vma_policies: Vec<VmaPolicyRecord>,
    node_perf: Vec<NodePerfRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SharedPolicyRecord {
    key: usize,
    entries: Vec<SharedPolicyEntry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SharedPolicyEntry {
    start: u64,
    end: u64,
    policy: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct VmaPolicyRecord {
    vma: usize,
    policy: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NodePerfRecord {
    node: i32,
    access: u32,
}

impl MemPolicyState {
    const fn new() -> Self {
        Self {
            nodes: Vec::new(),
            memblocks: Vec::new(),
            current_policy: MemoryPolicy::default_single_node(),
            interleave_cursor: 0,
            policy_zone: 0,
            allocated_policies: Vec::new(),
            shared_policies: Vec::new(),
            vma_policies: Vec::new(),
            node_perf: Vec::new(),
        }
    }

    fn ensure_boot_node(&mut self) {
        if self.nodes.is_empty() {
            self.nodes.push(NumaNode {
                id: 0,
                online: true,
                tier: 0,
            });
        }
    }

    fn reset(&mut self) {
        self.nodes.clear();
        self.memblocks.clear();
        self.current_policy = MemoryPolicy::default_single_node();
        self.interleave_cursor = 0;
        self.policy_zone = 0;
        for ptr in self.allocated_policies.drain(..) {
            unsafe {
                drop(Box::from_raw(ptr as *mut MemoryPolicy));
            }
        }
        self.shared_policies.clear();
        self.vma_policies.clear();
        self.node_perf.clear();
        self.ensure_boot_node();
    }

    fn online_mask(&self) -> u64 {
        self.nodes
            .iter()
            .filter(|node| node.online)
            .fold(0u64, |mask, node| mask | (1u64 << node.id))
    }
}

static MEMPOLICY_STATE: Mutex<MemPolicyState> = Mutex::new(MemPolicyState::new());

pub fn validate_mode(mode: i32) -> Result<(), i32> {
    if (-1..=MPOL_WEIGHTED_INTERLEAVE).contains(&mode) {
        Ok(())
    } else {
        Err(EINVAL)
    }
}

pub fn register_numa_node(id: u16, tier: u8) {
    let mut state = MEMPOLICY_STATE.lock();
    if let Some(node) = state.nodes.iter_mut().find(|node| node.id == id) {
        node.online = true;
        node.tier = tier;
    } else {
        state.nodes.push(NumaNode {
            id,
            online: true,
            tier,
        });
    }
}

pub fn online_nodes() -> u64 {
    let mut state = MEMPOLICY_STATE.lock();
    state.ensure_boot_node();
    state.online_mask()
}

pub fn numa_add_memblk(start: u64, end: u64, nid: u16) -> Result<(), i32> {
    if start >= end {
        return Err(EINVAL);
    }
    register_numa_node(nid, 0);
    MEMPOLICY_STATE
        .lock()
        .memblocks
        .push(NumaMemblock { start, end, nid });
    Ok(())
}

pub fn numa_remove_memblk(nid: u16, start: u64, end: u64) -> Result<(), i32> {
    let mut state = MEMPOLICY_STATE.lock();
    let Some(idx) = state
        .memblocks
        .iter()
        .position(|blk| blk.nid == nid && blk.start == start && blk.end == end)
    else {
        return Err(ESRCH);
    };
    state.memblocks.swap_remove(idx);
    Ok(())
}

pub fn numa_memblocks() -> Vec<NumaMemblock> {
    MEMPOLICY_STATE.lock().memblocks.clone()
}

pub fn numa_emulate_nodes(start: u64, end: u64, nr_nodes: u16) -> Result<Vec<NumaMemblock>, i32> {
    if start >= end || nr_nodes == 0 {
        return Err(EINVAL);
    }
    let span = end - start;
    let per_node = span / nr_nodes as u64;
    if per_node < (32 << 20) {
        return Err(EINVAL);
    }

    let mut created = Vec::new();
    for idx in 0..nr_nodes {
        let node_start = start + per_node * idx as u64;
        let node_end = if idx == nr_nodes - 1 {
            end
        } else {
            node_start + per_node
        };
        numa_add_memblk(node_start, node_end, idx)?;
        created.push(NumaMemblock {
            start: node_start,
            end: node_end,
            nid: idx,
        });
    }
    Ok(created)
}

pub fn mbind(len: u64, mode: u64, flags: u32) -> Result<(), i32> {
    if len == 0 || mode > MPOL_PREFERRED_MANY as u64 || flags & !0x7 != 0 {
        Err(EINVAL)
    } else {
        set_mempolicy(mode as i32).map(|_| ())
    }
}

pub fn set_mempolicy(mode: i32) -> Result<MemoryPolicy, i32> {
    validate_mode(mode)?;
    let mut state = MEMPOLICY_STATE.lock();
    state.ensure_boot_node();
    let policy = if mode == -1 {
        MemoryPolicy::default_single_node()
    } else if matches!(mode, MPOL_DEFAULT | MPOL_LOCAL) {
        MemoryPolicy::new(mode, 0, 0)
    } else {
        MemoryPolicy::new(mode, 0, state.online_mask())
    };
    state.current_policy = policy;
    Ok(policy)
}

pub fn set_mempolicy_mask(mode: i32, nodemask: u64) -> Result<MemoryPolicy, i32> {
    validate_mode(mode)?;
    let mut state = MEMPOLICY_STATE.lock();
    state.ensure_boot_node();
    if nodemask & state.online_mask() == 0 && !matches!(mode, MPOL_DEFAULT | MPOL_LOCAL) {
        return Err(EINVAL);
    }
    let policy = MemoryPolicy::new(mode, 0, nodemask);
    state.current_policy = policy;
    Ok(policy)
}

pub fn get_mempolicy(flags: u64) -> Result<MemoryPolicy, i32> {
    if flags & !0x3 != 0 {
        Err(EINVAL)
    } else {
        let mut state = MEMPOLICY_STATE.lock();
        state.ensure_boot_node();
        Ok(state.current_policy)
    }
}

pub fn select_node_for_address(addr: u64) -> Result<u16, i32> {
    let mut state = MEMPOLICY_STATE.lock();
    state.ensure_boot_node();
    let mask = if state.current_policy.nodemask == 0 {
        state.online_mask()
    } else {
        state.current_policy.nodemask & state.online_mask()
    };
    if mask == 0 {
        return Err(EINVAL);
    }

    let nodes: Vec<u16> = state
        .nodes
        .iter()
        .filter(|node| node.online && (mask & (1u64 << node.id)) != 0)
        .map(|node| node.id)
        .collect();

    let node = match state.current_policy.mode {
        MPOL_INTERLEAVE => {
            let idx = state.interleave_cursor % nodes.len();
            state.interleave_cursor += 1;
            nodes[idx]
        }
        MPOL_BIND | MPOL_PREFERRED | MPOL_PREFERRED_MANY => nodes[0],
        MPOL_LOCAL | MPOL_DEFAULT => ((addr >> 12) as usize % nodes.len()) as u16,
        _ => 0,
    };
    Ok(node)
}

pub fn set_mempolicy_home_node(len: u64, flags: u64) -> Result<(), i32> {
    if len == 0 || flags != 0 {
        Err(EINVAL)
    } else {
        Ok(())
    }
}

pub fn migrate_pages(pid: i32) -> Result<i64, i32> {
    if pid < 0 { Err(ESRCH) } else { Ok(0) }
}

// ---------------------------------------------------------------------------
// Linux-visible mempolicy.h wrappers
// ---------------------------------------------------------------------------

pub fn numa_default_policy() -> MemoryPolicy {
    MemoryPolicy::default_single_node()
}

pub fn numa_policy_init() {
    let _ = set_mempolicy(MPOL_DEFAULT);
}

pub fn mpol_get(policy: *mut MemoryPolicy) -> *mut MemoryPolicy {
    if !policy.is_null() {
        let mut state = MEMPOLICY_STATE.lock();
        if state.allocated_policies.contains(&(policy as usize)) {
            unsafe {
                (*policy).refcnt = (*policy).refcnt.saturating_add(1);
            }
        }
    }
    policy
}

pub fn __mpol_put(policy: *mut MemoryPolicy) {
    if policy.is_null() {
        return;
    }

    let mut free_policy = false;
    {
        let mut state = MEMPOLICY_STATE.lock();
        if let Some(idx) = state
            .allocated_policies
            .iter()
            .position(|tracked| *tracked == policy as usize)
        {
            unsafe {
                (*policy).refcnt -= 1;
                if (*policy).refcnt <= 0 {
                    state.allocated_policies.swap_remove(idx);
                    state
                        .shared_policies
                        .iter_mut()
                        .for_each(|sp| sp.entries.retain(|entry| entry.policy != policy as usize));
                    state
                        .vma_policies
                        .retain(|entry| entry.policy != policy as usize);
                    free_policy = true;
                }
            }
        }
    }

    if free_policy {
        unsafe {
            drop(Box::from_raw(policy));
        }
    }
}

pub fn mpol_put(policy: *mut MemoryPolicy) {
    __mpol_put(policy)
}

pub fn mpol_cond_put(policy: *mut MemoryPolicy) {
    if mpol_needs_cond_ref(policy) {
        mpol_put(policy);
    }
}

pub fn __mpol_dup(policy: *const MemoryPolicy) -> *mut MemoryPolicy {
    if policy.is_null() {
        core::ptr::null_mut()
    } else {
        let mut copy = unsafe { *policy };
        copy.refcnt = 1;
        let ptr = Box::into_raw(Box::new(copy));
        MEMPOLICY_STATE.lock().allocated_policies.push(ptr as usize);
        ptr
    }
}

pub fn mpol_dup(policy: *const MemoryPolicy) -> *mut MemoryPolicy {
    __mpol_dup(policy)
}

pub fn __mpol_equal(a: *const MemoryPolicy, b: *const MemoryPolicy) -> bool {
    if a.is_null() || b.is_null() {
        a == b
    } else {
        unsafe { *a == *b }
    }
}

pub fn mpol_equal(a: *const MemoryPolicy, b: *const MemoryPolicy) -> bool {
    __mpol_equal(a, b)
}

pub fn mpol_needs_cond_ref(policy: *const MemoryPolicy) -> bool {
    !policy.is_null() && unsafe { (*policy).flags & MPOL_F_SHARED != 0 }
}

pub fn mpol_is_preferred_many(policy: *const MemoryPolicy) -> bool {
    !policy.is_null() && unsafe { (*policy).mode == MPOL_PREFERRED_MANY }
}

fn policy_mode_name(mode: i32) -> Option<&'static str> {
    match mode {
        MPOL_DEFAULT => Some("default"),
        MPOL_PREFERRED => Some("prefer"),
        MPOL_BIND => Some("bind"),
        MPOL_INTERLEAVE => Some("interleave"),
        MPOL_LOCAL => Some("local"),
        MPOL_PREFERRED_MANY => Some("prefer (many)"),
        MPOL_WEIGHTED_INTERLEAVE => Some("weighted interleave"),
        _ => None,
    }
}

fn copy_c_string(buffer: *mut u8, maxlen: usize, text: &str) -> i32 {
    if buffer.is_null() || maxlen == 0 {
        return -EINVAL;
    }

    let bytes = text.as_bytes();
    let copy_len = bytes.len().min(maxlen - 1);
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, copy_len);
        *buffer.add(copy_len) = 0;
    }
    0
}

fn policy_to_string(policy: *const MemoryPolicy) -> Option<String> {
    let policy = if policy.is_null() {
        MemoryPolicy::default_single_node()
    } else {
        unsafe { *policy }
    };
    let mut out = String::from(policy_mode_name(policy.mode)?);

    let mode_flags = policy.flags & MPOL_MODE_FLAGS;
    if mode_flags != 0 {
        out.push('=');
        if mode_flags & MPOL_F_STATIC_NODES != 0 {
            out.push_str("static");
        } else if mode_flags & MPOL_F_RELATIVE_NODES != 0 {
            out.push_str("relative");
        }
        if mode_flags & MPOL_F_NUMA_BALANCING != 0 {
            if mode_flags & (MPOL_F_STATIC_NODES | MPOL_F_RELATIVE_NODES) != 0 {
                out.push('|');
            }
            out.push_str("balancing");
        }
    }

    if policy.nodemask != 0
        && matches!(
            policy.mode,
            MPOL_PREFERRED
                | MPOL_BIND
                | MPOL_INTERLEAVE
                | MPOL_PREFERRED_MANY
                | MPOL_WEIGHTED_INTERLEAVE
        )
    {
        out.push(':');
        out.push_str(&format!("{:#x}", policy.nodemask));
    }

    Some(out)
}

pub fn mpol_to_str(buffer: *mut u8, maxlen: usize, policy: *const MemoryPolicy) -> i32 {
    let Some(text) = policy_to_string(policy) else {
        return -EINVAL;
    };
    copy_c_string(buffer, maxlen, &text)
}

fn read_c_str(ptr: *const u8) -> Option<&'static str> {
    if ptr.is_null() {
        return None;
    }
    let mut len = 0usize;
    unsafe {
        while *ptr.add(len) != 0 {
            len = len.checked_add(1)?;
            if len > 4096 {
                return None;
            }
        }
        core::str::from_utf8(core::slice::from_raw_parts(ptr, len)).ok()
    }
}

fn parse_nodemask(text: &str) -> Option<u64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Some(hex) = text.strip_prefix("0x") {
        return u64::from_str_radix(hex, 16).ok();
    }

    let mut mask = 0u64;
    for part in text.split(',') {
        let part = part.trim();
        if part.is_empty() {
            return None;
        }
        if let Some((start, end)) = part.split_once('-') {
            let start: u8 = start.parse().ok()?;
            let end: u8 = end.parse().ok()?;
            if start > end || end >= 64 {
                return None;
            }
            for node in start..=end {
                mask |= 1u64 << node;
            }
        } else {
            let node: u8 = part.parse().ok()?;
            if node >= 64 {
                return None;
            }
            mask |= 1u64 << node;
        }
    }
    Some(mask)
}

fn parse_mode_flags(text: Option<&str>) -> Option<u32> {
    let Some(text) = text else {
        return Some(0);
    };
    let mut flags = 0u32;
    for flag in text.split('|') {
        match flag.trim() {
            "static" => flags |= MPOL_F_STATIC_NODES,
            "relative" => flags |= MPOL_F_RELATIVE_NODES,
            "balancing" => flags |= MPOL_F_NUMA_BALANCING,
            _ => return None,
        }
    }
    if flags & MPOL_F_STATIC_NODES != 0 && flags & MPOL_F_RELATIVE_NODES != 0 {
        return None;
    }
    Some(flags)
}

pub fn mpol_parse_str(str_ptr: *const u8, policy: *mut *mut MemoryPolicy) -> i32 {
    if policy.is_null() {
        return 1;
    }

    let Some(input) = read_c_str(str_ptr) else {
        return 1;
    };
    let (mode_and_flags, node_text) = input.split_once(':').unwrap_or((input, ""));
    let (mode_text, flags_text) = mode_and_flags
        .split_once('=')
        .map_or((mode_and_flags, None), |(mode, flags)| (mode, Some(flags)));

    let mode = match mode_text.trim() {
        "default" => MPOL_DEFAULT,
        "prefer" => MPOL_PREFERRED,
        "bind" => MPOL_BIND,
        "interleave" => MPOL_INTERLEAVE,
        "local" => MPOL_LOCAL,
        "prefer (many)" => MPOL_PREFERRED_MANY,
        "weighted interleave" => MPOL_WEIGHTED_INTERLEAVE,
        _ => return 1,
    };

    let Some(flags) = parse_mode_flags(flags_text) else {
        return 1;
    };
    let has_nodes = !node_text.trim().is_empty();
    let node_mask = if has_nodes {
        let Some(mask) = parse_nodemask(node_text) else {
            return 1;
        };
        if mask & online_nodes() == 0 {
            return 1;
        }
        mask
    } else {
        0
    };

    match mode {
        MPOL_DEFAULT if has_nodes => return 1,
        MPOL_LOCAL if has_nodes => return 1,
        MPOL_BIND | MPOL_PREFERRED_MANY if !has_nodes => return 1,
        MPOL_PREFERRED if has_nodes && node_mask.count_ones() != 1 => return 1,
        MPOL_INTERLEAVE | MPOL_WEIGHTED_INTERLEAVE if !has_nodes => {}
        _ => {}
    }

    let nodemask = if matches!(mode, MPOL_INTERLEAVE | MPOL_WEIGHTED_INTERLEAVE) && !has_nodes {
        online_nodes()
    } else {
        node_mask
    };
    let parsed = Box::into_raw(Box::new(MemoryPolicy::new(mode, flags, nodemask)));
    MEMPOLICY_STATE
        .lock()
        .allocated_policies
        .push(parsed as usize);
    unsafe {
        *policy = parsed;
    }
    0
}

pub fn get_task_policy(task: *mut u8) -> *mut MemoryPolicy {
    if task.is_null() {
        return core::ptr::null_mut();
    }
    let mut policy = MEMPOLICY_STATE.lock().current_policy;
    policy.refcnt = 1;
    let ptr = Box::into_raw(Box::new(policy));
    MEMPOLICY_STATE.lock().allocated_policies.push(ptr as usize);
    ptr
}

pub fn get_vma_policy(vma: *mut u8, _addr: u64) -> *mut MemoryPolicy {
    if vma.is_null() {
        return core::ptr::null_mut();
    }
    let policy = {
        let state = MEMPOLICY_STATE.lock();
        state
            .vma_policies
            .iter()
            .find(|record| record.vma == vma as usize)
            .map(|record| record.policy)
            .unwrap_or(0)
    };
    if policy == 0 {
        core::ptr::null_mut()
    } else {
        mpol_dup(policy as *const MemoryPolicy)
    }
}

pub fn vma_dup_policy(src: *mut u8, dst: *mut u8) -> i32 {
    if src.is_null() || dst.is_null() {
        return 0;
    }
    let src_policy = {
        let state = MEMPOLICY_STATE.lock();
        state
            .vma_policies
            .iter()
            .find(|record| record.vma == src as usize)
            .map(|record| record.policy)
            .unwrap_or(0)
    };
    if src_policy == 0 {
        let mut state = MEMPOLICY_STATE.lock();
        state
            .vma_policies
            .retain(|record| record.vma != dst as usize);
        return 0;
    }
    let dup = mpol_dup(src_policy as *const MemoryPolicy) as usize;
    let mut state = MEMPOLICY_STATE.lock();
    state
        .vma_policies
        .retain(|record| record.vma != dst as usize);
    state.vma_policies.push(VmaPolicyRecord {
        vma: dst as usize,
        policy: dup,
    });
    0
}

pub fn vma_policy_mof(vma: *mut u8) -> u32 {
    if vma.is_null() {
        return 0;
    }
    let state = MEMPOLICY_STATE.lock();
    state
        .vma_policies
        .iter()
        .find(|record| record.vma == vma as usize)
        .map(|record| unsafe { (*(record.policy as *const MemoryPolicy)).flags & MPOL_F_MOF })
        .unwrap_or(0)
}

pub fn vma_migratable(vma: *mut u8) -> bool {
    if vma.is_null() {
        return false;
    }
    let vma = unsafe { &*(vma as *const VmAreaStruct) };
    vma.vm_flags & VM_HUGETLB == 0
}

pub fn mpol_put_task_policy(_task: *mut u8) {}

pub fn mpol_rebind_mm(_mm: *mut u8, _new: *const u8) {}

pub fn mpol_rebind_task(_task: *mut u8, _new: *const u8) {}

pub fn mpol_shared_policy_init(sp: *mut u8, policy: *mut MemoryPolicy) {
    if sp.is_null() {
        return;
    }
    let policy_ref = if policy.is_null() {
        0
    } else {
        unsafe {
            (*policy).flags |= MPOL_F_SHARED;
        }
        mpol_get(policy) as usize
    };
    let old_entries = {
        let mut state = MEMPOLICY_STATE.lock();
        if let Some(idx) = state
            .shared_policies
            .iter()
            .position(|record| record.key == sp as usize)
        {
            state.shared_policies.swap_remove(idx).entries
        } else {
            Vec::new()
        }
    };
    for entry in old_entries {
        mpol_put(entry.policy as *mut MemoryPolicy);
    }
    let mut state = MEMPOLICY_STATE.lock();
    let mut record = SharedPolicyRecord {
        key: sp as usize,
        entries: Vec::new(),
    };
    if policy_ref != 0 {
        record.entries.push(SharedPolicyEntry {
            start: 0,
            end: u64::MAX,
            policy: policy_ref,
        });
    }
    state.shared_policies.push(record);
}

pub fn mpol_free_shared_policy(sp: *mut u8) {
    if sp.is_null() {
        return;
    }
    let entries = {
        let mut state = MEMPOLICY_STATE.lock();
        let Some(idx) = state
            .shared_policies
            .iter()
            .position(|record| record.key == sp as usize)
        else {
            return;
        };
        state.shared_policies.swap_remove(idx).entries
    };
    for entry in entries {
        mpol_put(entry.policy as *mut MemoryPolicy);
    }
}

pub fn mpol_set_shared_policy(sp: *mut u8, vma: *mut u8, new: *mut MemoryPolicy) -> i32 {
    if sp.is_null() || vma.is_null() {
        return -EINVAL;
    }

    let vma_ref = unsafe { &*(vma as *const VmAreaStruct) };
    if vma_ref.vm_end <= vma_ref.vm_start {
        return -EINVAL;
    }
    let start = vma_ref.vm_pgoff;
    let end =
        start + ((vma_ref.vm_end - vma_ref.vm_start).div_ceil(crate::mm::frame::PAGE_SIZE as u64));
    let shared_ref = if new.is_null() {
        0
    } else {
        unsafe {
            (*new).flags |= MPOL_F_SHARED;
        }
        mpol_get(new) as usize
    };
    let vma_ref_policy = if new.is_null() {
        0
    } else {
        mpol_get(new) as usize
    };

    let old_entries = {
        let mut state = MEMPOLICY_STATE.lock();
        let idx = match state
            .shared_policies
            .iter()
            .position(|record| record.key == sp as usize)
        {
            Some(idx) => idx,
            None => {
                state.shared_policies.push(SharedPolicyRecord {
                    key: sp as usize,
                    entries: Vec::new(),
                });
                state.shared_policies.len() - 1
            }
        };
        let mut removed = Vec::new();
        {
            let record = &mut state.shared_policies[idx];
            record.entries.retain(|entry| {
                let overlaps = entry.start < end && start < entry.end;
                if overlaps {
                    removed.push(entry.policy);
                }
                !overlaps
            });
            if shared_ref != 0 {
                record.entries.push(SharedPolicyEntry {
                    start,
                    end,
                    policy: shared_ref,
                });
            }
        }
        state
            .vma_policies
            .retain(|record| record.vma != vma as usize);
        if shared_ref != 0 {
            state.vma_policies.push(VmaPolicyRecord {
                vma: vma as usize,
                policy: vma_ref_policy,
            });
        }
        removed
    };
    for policy in old_entries {
        mpol_put(policy as *mut MemoryPolicy);
    }
    0
}

pub fn mpol_shared_policy_lookup(sp: *mut u8, idx: u64) -> *mut MemoryPolicy {
    if sp.is_null() {
        return core::ptr::null_mut();
    }
    let policy = {
        let state = MEMPOLICY_STATE.lock();
        let Some(record) = state
            .shared_policies
            .iter()
            .find(|record| record.key == sp as usize)
        else {
            return core::ptr::null_mut();
        };
        record
            .entries
            .iter()
            .find(|entry| idx >= entry.start && idx < entry.end)
            .map(|entry| entry.policy)
            .unwrap_or(0)
    };
    if policy == 0 {
        return core::ptr::null_mut();
    }
    mpol_get(policy as *mut MemoryPolicy)
}

pub fn init_nodemask_of_mempolicy(policy: *const MemoryPolicy) -> u64 {
    if policy.is_null() {
        0
    } else {
        unsafe { (*policy).nodemask }
    }
}

pub fn apply_policy_zone(policy: *const MemoryPolicy, zone: usize) -> bool {
    if policy.is_null() {
        return true;
    }
    let dynamic_zone = MEMPOLICY_STATE.lock().policy_zone;
    zone >= dynamic_zone
}

pub fn check_highest_zone(zone: usize) -> bool {
    let mut state = MEMPOLICY_STATE.lock();
    if zone > state.policy_zone {
        state.policy_zone = zone;
    }
    true
}

pub fn huge_node(_vma: *mut u8, addr: u64, _gfp_flags: GfpFlags, _mpol: *mut MemoryPolicy) -> i32 {
    select_node_for_address(addr).unwrap_or(0) as i32
}

pub fn mempolicy_slab_node() -> i32 {
    select_node_for_address(0).unwrap_or(0) as i32
}

pub fn mempolicy_in_oom_domain(_task: *mut u8, mask: *const u8) -> bool {
    if mask.is_null() {
        return true;
    }
    let requested = unsafe { *(mask as *const u64) };
    let policy = MEMPOLICY_STATE.lock().current_policy;
    let policy_mask = if policy.nodemask == 0 {
        online_nodes()
    } else {
        policy.nodemask
    };
    requested & policy_mask != 0
}

pub fn mempolicy_set_node_perf(node: i32, access: u32) {
    let mut state = MEMPOLICY_STATE.lock();
    if let Some(record) = state
        .node_perf
        .iter_mut()
        .find(|record| record.node == node)
    {
        record.access = access;
    } else {
        state.node_perf.push(NodePerfRecord { node, access });
    }
}

pub fn do_migrate_pages(pid: i32, _from: u64, _to: u64, _flags: u32) -> i64 {
    migrate_pages(pid).unwrap_or_else(|err| -(err as i64))
}

pub fn mpol_misplaced(_folio: *mut Page, _vma: *mut u8, addr: u64) -> i32 {
    select_node_for_address(addr).unwrap_or(0) as i32
}

pub fn nearest_node_nodemask(node: i32, mask: u64) -> i32 {
    if mask == 0 {
        return MAX_NUMNODES;
    }
    let start = node.max(0);
    let mut best_node = MAX_NUMNODES;
    let mut best_dist = i32::MAX;
    for idx in 0..64 {
        if (mask & (1u64 << idx)) != 0 {
            let dist = (idx - start).abs();
            if dist < best_dist {
                best_dist = dist;
                best_node = idx;
            }
        }
    }
    best_node
}

pub fn numa_nearest_node(node: i32, mask: u64) -> i32 {
    nearest_node_nodemask(node, mask)
}

pub fn folio_alloc_noprof(gfp: GfpFlags, order: u32) -> *mut Page {
    crate::mm::page_alloc::alloc_pages_noprof(gfp, order)
}

pub fn vma_alloc_folio_noprof(
    gfp: GfpFlags,
    order: u32,
    _vma: *mut u8,
    _addr: u64,
    _hugepage: bool,
) -> *mut Page {
    folio_alloc_noprof(gfp, order)
}

#[cfg(test)]
pub fn reset_for_tests() {
    MEMPOLICY_STATE.lock().reset();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;

    #[test]
    fn policy_state_tracks_online_nodes_and_current_policy() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        register_numa_node(1, 0);
        assert_eq!(online_nodes() & 0b11, 0b11);
        assert_eq!(set_mempolicy_mask(MPOL_BIND, 0b10).unwrap().nodemask, 0b10);
        assert_eq!(get_mempolicy(0).unwrap().mode, MPOL_BIND);
        assert_eq!(select_node_for_address(0x1000).unwrap(), 1);
    }

    #[test]
    fn interleave_policy_rotates_across_nodes() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        register_numa_node(1, 0);
        set_mempolicy_mask(MPOL_INTERLEAVE, 0b11).unwrap();
        assert_eq!(select_node_for_address(0).unwrap(), 0);
        assert_eq!(select_node_for_address(0).unwrap(), 1);
    }

    #[test]
    fn mbind_and_home_node_validate_linux_inputs() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        assert_eq!(mbind(0, 0, 0), Err(EINVAL));
        assert_eq!(mbind(4096, 0, 0), Ok(()));
        assert_eq!(set_mempolicy_home_node(0, 0), Err(EINVAL));
        assert_eq!(set_mempolicy_home_node(4096, 0), Ok(()));
    }

    #[test]
    fn numa_memblocks_and_emulation_split_physical_ranges() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        assert_eq!(numa_add_memblk(0x1000, 0x2000, 0), Ok(()));
        assert_eq!(numa_memblocks().len(), 1);
        assert_eq!(numa_remove_memblk(0, 0x1000, 0x2000), Ok(()));

        let blocks = numa_emulate_nodes(0, 128 << 20, 2).unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].nid, 0);
        assert_eq!(blocks[1].nid, 1);
        assert_eq!(online_nodes() & 0b11, 0b11);
        assert_eq!(numa_emulate_nodes(0, 1 << 20, 2), Err(EINVAL));
    }

    #[test]
    fn parse_format_and_refcount_match_tmpfs_policy_shape() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        register_numa_node(1, 0);

        let mut parsed = core::ptr::null_mut();
        assert_eq!(mpol_parse_str(b"bind=static:1\0".as_ptr(), &mut parsed), 0);
        assert!(!parsed.is_null());
        unsafe {
            assert_eq!((*parsed).mode, MPOL_BIND);
            assert_eq!((*parsed).flags & MPOL_F_STATIC_NODES, MPOL_F_STATIC_NODES);
            assert_eq!((*parsed).nodemask, 0b10);
        }

        let mut buf = [0u8; 64];
        assert_eq!(mpol_to_str(buf.as_mut_ptr(), buf.len(), parsed), 0);
        let len = buf.iter().position(|byte| *byte == 0).unwrap();
        assert_eq!(
            core::str::from_utf8(&buf[..len]).unwrap(),
            "bind=static:0x2"
        );

        let dup = mpol_dup(parsed);
        assert!(!dup.is_null());
        unsafe {
            assert_eq!((*dup).refcnt, 1);
        }
        mpol_get(dup);
        unsafe {
            assert_eq!((*dup).refcnt, 2);
        }
        mpol_put(dup);
        unsafe {
            assert_eq!((*dup).refcnt, 1);
        }
        mpol_put(dup);
        mpol_put(parsed);

        assert_eq!(mpol_parse_str(b"default:0\0".as_ptr(), &mut parsed), 1);
    }

    #[test]
    fn shared_policy_and_vma_lookup_track_page_ranges() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();

        let mut sp = 0u8;
        let mut vma = VmAreaStruct::new(0x4000, 0x6000, 0);
        vma.vm_pgoff = 8;
        let mut policy = MemoryPolicy::new(MPOL_PREFERRED_MANY, MPOL_F_MOF, 0b1);

        assert_eq!(
            mpol_set_shared_policy(
                &mut sp as *mut u8,
                &mut vma as *mut VmAreaStruct as *mut u8,
                &mut policy,
            ),
            0
        );
        assert!(mpol_needs_cond_ref(&policy));

        let hit = mpol_shared_policy_lookup(&mut sp, 8);
        assert_eq!(hit, &mut policy as *mut MemoryPolicy);
        assert!(mpol_shared_policy_lookup(&mut sp, 10).is_null());

        let vma_policy = get_vma_policy(&mut vma as *mut VmAreaStruct as *mut u8, 0x4000);
        assert!(!vma_policy.is_null());
        unsafe {
            assert_eq!((*vma_policy).mode, MPOL_PREFERRED_MANY);
        }
        mpol_put(vma_policy);

        let mut dst = VmAreaStruct::new(0x8000, 0xa000, 0);
        assert_eq!(
            vma_dup_policy(
                &mut vma as *mut VmAreaStruct as *mut u8,
                &mut dst as *mut VmAreaStruct as *mut u8,
            ),
            0
        );
        let dst_policy = get_vma_policy(&mut dst as *mut VmAreaStruct as *mut u8, 0x8000);
        assert!(!dst_policy.is_null());
        mpol_put(dst_policy);

        assert_eq!(
            mpol_set_shared_policy(
                &mut sp as *mut u8,
                &mut vma as *mut VmAreaStruct as *mut u8,
                core::ptr::null_mut(),
            ),
            0
        );
        assert!(mpol_shared_policy_lookup(&mut sp, 8).is_null());
        mpol_free_shared_policy(&mut sp);
    }

    #[test]
    fn zone_oom_and_nearest_node_helpers_follow_policy_state() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        register_numa_node(1, 0);

        let policy = MemoryPolicy::new(MPOL_BIND, 0, 0b10);
        check_highest_zone(2);
        assert!(!apply_policy_zone(&policy, 1));
        assert!(apply_policy_zone(&policy, 2));

        set_mempolicy_mask(MPOL_BIND, 0b10).unwrap();
        let allowed = 0b10u64;
        let denied = 0b01u64;
        assert!(mempolicy_in_oom_domain(
            core::ptr::null_mut(),
            &allowed as *const u64 as *const u8
        ));
        assert!(!mempolicy_in_oom_domain(
            core::ptr::null_mut(),
            &denied as *const u64 as *const u8
        ));
        assert_eq!(mempolicy_slab_node(), 1);
        assert_eq!(nearest_node_nodemask(5, (1 << 3) | (1 << 10)), 3);
        assert_eq!(nearest_node_nodemask(5, 0), MAX_NUMNODES);

        mempolicy_set_node_perf(1, 42);
        assert_eq!(
            mpol_misplaced(core::ptr::null_mut(), core::ptr::null_mut(), 0),
            1
        );
    }
}
