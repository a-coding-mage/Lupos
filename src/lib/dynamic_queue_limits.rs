//! linux-parity: complete
//! linux-source: vendor/linux/lib/dynamic_queue_limits.c
//! test-origin: linux:vendor/linux/lib/dynamic_queue_limits.c
//! Module-facing dynamic byte queue limits.

use crate::kernel::module::{export_symbol, find_symbol};

const DQL_HIST_LEN: usize = 4;
const DQL_MAX_OBJECT: u32 = u32::MAX / 16;
const DQL_MAX_LIMIT: u32 = (u32::MAX / 2) - DQL_MAX_OBJECT;

/// Configured x86-64 `struct dql` (`____cacheline_aligned_in_smp` makes the
/// completion half begin at offset 64 and the whole object 128 bytes).
#[repr(C, align(64))]
pub struct LinuxDql {
    num_queued: u32,
    adj_limit: u32,
    last_obj_cnt: u32,
    stall_thrs: u16,
    _pad0: u16,
    history_head: u64,
    history: [u64; DQL_HIST_LEN],
    _pad1: [u8; 8],
    limit: u32,
    num_completed: u32,
    prev_ovlimit: u32,
    prev_num_queued: u32,
    prev_last_obj_cnt: u32,
    lowest_slack: u32,
    slack_start_time: u64,
    max_limit: u32,
    min_limit: u32,
    slack_hold_time: u32,
    stall_max: u16,
    _pad2: u16,
    last_reap: u64,
    stall_cnt: u64,
}

const _: () = assert!(core::mem::size_of::<LinuxDql>() == 128);
const _: () = assert!(core::mem::align_of::<LinuxDql>() == 64);

fn export_symbol_once(name: &'static str, addr: usize) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, false);
    }
}

pub fn register_module_exports() {
    export_symbol_once("dql_completed", dql_completed as usize);
    export_symbol_once("dql_reset", dql_reset as usize);
}

#[inline]
fn posdiff(a: u32, b: u32) -> u32 {
    let diff = a.wrapping_sub(b);
    if (diff as i32) > 0 { diff } else { 0 }
}

#[inline]
fn after_eq(a: u32, b: u32) -> bool {
    (a.wrapping_sub(b) as i32) >= 0
}

#[inline]
fn time_after(a: u64, b: u64) -> bool {
    (b.wrapping_sub(a) as i64) < 0
}

#[inline]
fn time_after_eq(a: u64, b: u64) -> bool {
    (a.wrapping_sub(b) as i64) >= 0
}

#[inline]
fn time_before(a: u64, b: u64) -> bool {
    time_after(b, a)
}

#[inline]
fn time_before_eq(a: u64, b: u64) -> bool {
    time_after_eq(b, a)
}

unsafe fn dql_check_stall(dql: &mut LinuxDql, stall_thrs: u16) {
    if stall_thrs == 0 {
        return;
    }
    let now = crate::kernel::time::jiffies::jiffies();
    if !time_after_eq(now, dql.last_reap.wrapping_add(stall_thrs as u64)) {
        return;
    }

    loop {
        let hist_head = unsafe { core::ptr::read_volatile(&dql.history_head) };
        core::sync::atomic::fence(core::sync::atomic::Ordering::Acquire);
        let mut start = hist_head
            .wrapping_sub(DQL_HIST_LEN as u64)
            .wrapping_add(1)
            .wrapping_mul(64);
        if time_before(start, dql.last_reap.wrapping_add(1)) {
            start = dql.last_reap.wrapping_add(1);
        }
        let mut end = hist_head.wrapping_mul(64).wrapping_add(63);
        if time_before(now, end.wrapping_add((stall_thrs / 2) as u64)) {
            end = now.wrapping_sub((stall_thrs / 2) as u64);
        }
        let mut t = start;
        while time_before_eq(t, end) {
            let bit = (t % (DQL_HIST_LEN as u64 * 64)) as usize;
            if dql.history[bit / 64] & (1u64 << (bit % 64)) != 0 {
                break;
            }
            t = t.wrapping_add(1);
        }
        if !time_before_eq(t, end) {
            break;
        }
        if hist_head != unsafe { core::ptr::read_volatile(&dql.history_head) } {
            continue;
        }
        dql.stall_cnt = dql.stall_cnt.wrapping_add(1);
        dql.stall_max = dql.stall_max.max(now.wrapping_sub(t) as u16);
        break;
    }
    dql.last_reap = now;
}

/// `dql_completed()` — direct port of
/// `vendor/linux/lib/dynamic_queue_limits.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dql_completed(dql: *mut LinuxDql, count: u32) {
    let Some(dql) = (unsafe { dql.as_mut() }) else {
        return;
    };
    let num_queued = unsafe { core::ptr::read_volatile(&dql.num_queued) };
    let stall_thrs = unsafe { core::ptr::read_volatile(&dql.stall_thrs) };
    assert!(count <= num_queued.wrapping_sub(dql.num_completed));

    let completed = dql.num_completed.wrapping_add(count);
    let mut limit = dql.limit;
    let mut ovlimit = posdiff(num_queued.wrapping_sub(dql.num_completed), limit);
    let inprogress = num_queued.wrapping_sub(completed);
    let prev_inprogress = dql.prev_num_queued.wrapping_sub(dql.num_completed);
    let all_prev_completed = after_eq(completed, dql.prev_num_queued);
    let now = crate::kernel::time::jiffies::jiffies();

    if (ovlimit != 0 && inprogress == 0) || (dql.prev_ovlimit != 0 && all_prev_completed) {
        limit = limit
            .wrapping_add(posdiff(completed, dql.prev_num_queued))
            .wrapping_add(dql.prev_ovlimit);
        dql.slack_start_time = now;
        dql.lowest_slack = u32::MAX;
    } else if inprogress != 0 && prev_inprogress != 0 && !all_prev_completed {
        let mut slack = posdiff(
            limit.wrapping_add(dql.prev_ovlimit),
            completed.wrapping_sub(dql.num_completed).wrapping_mul(2),
        );
        let slack_last_objs = if dql.prev_ovlimit != 0 {
            posdiff(dql.prev_last_obj_cnt, dql.prev_ovlimit)
        } else {
            0
        };
        slack = slack.max(slack_last_objs);
        dql.lowest_slack = dql.lowest_slack.min(slack);
        if time_after(
            now,
            dql.slack_start_time
                .wrapping_add(dql.slack_hold_time as u64),
        ) {
            limit = posdiff(limit, dql.lowest_slack);
            dql.slack_start_time = now;
            dql.lowest_slack = u32::MAX;
        }
    }

    limit = limit.clamp(dql.min_limit, dql.max_limit);
    if limit != dql.limit {
        dql.limit = limit;
        ovlimit = 0;
    }
    dql.adj_limit = limit.wrapping_add(completed);
    dql.prev_ovlimit = ovlimit;
    dql.prev_last_obj_cnt = unsafe { core::ptr::read_volatile(&dql.last_obj_cnt) };
    dql.num_completed = completed;
    dql.prev_num_queued = num_queued;
    unsafe { dql_check_stall(dql, stall_thrs) };
}

/// `dql_reset()` — direct port of
/// `vendor/linux/lib/dynamic_queue_limits.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dql_reset(dql: *mut LinuxDql) {
    let Some(dql) = (unsafe { dql.as_mut() }) else {
        return;
    };
    let now = crate::kernel::time::jiffies::jiffies();
    dql.limit = dql.min_limit;
    dql.num_queued = 0;
    dql.num_completed = 0;
    dql.last_obj_cnt = 0;
    dql.prev_num_queued = 0;
    dql.prev_last_obj_cnt = 0;
    dql.prev_ovlimit = 0;
    dql.lowest_slack = u32::MAX;
    dql.slack_start_time = now;
    dql.last_reap = now;
    dql.history_head = now / 64;
    dql.history.fill(0);
}

/// `dql_init()` — `vendor/linux/lib/dynamic_queue_limits.c`.
pub unsafe fn dql_init(dql: *mut LinuxDql, hold_time: u32) {
    let Some(dql_ref) = (unsafe { dql.as_mut() }) else {
        return;
    };
    dql_ref.max_limit = DQL_MAX_LIMIT;
    dql_ref.min_limit = 0;
    dql_ref.slack_hold_time = hold_time;
    dql_ref.stall_thrs = 0;
    unsafe { dql_reset(dql) };
}
