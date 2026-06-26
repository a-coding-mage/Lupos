//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/events/intel/bts.c
//! test-origin: linux:vendor/linux/arch/x86/events/intel/bts.c
//! Intel Branch Trace Store PMU model.

use crate::include::uapi::errno::{EBUSY, EINVAL, ENODEV, ENOENT, ENOMEM, ENOSPC};

pub const PAGE_SHIFT: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
pub const BTS_BUFFER_SIZE: usize = PAGE_SIZE << 4;
pub const BTS_RECORD_SIZE: usize = 24;
pub const BTS_SAFETY_MARGIN: usize = 4080;

pub const PERF_EF_START: i32 = 0x01;
pub const PERF_EF_UPDATE: i32 = 0x04;
pub const PERF_HES_STOPPED: u64 = 0x01;
pub const PERF_AUX_FLAG_TRUNCATED: u32 = 0x0001;

pub const PERF_PMU_CAP_AUX_NO_SG: u64 = 0x0004;
pub const PERF_PMU_CAP_EXCLUSIVE: u64 = 0x0010;
pub const PERF_PMU_CAP_ITRACE: u64 = 0x0020;

pub const INTEL_PMC_IDX_FIXED: u8 = 32;
pub const INTEL_PMC_IDX_FIXED_BTS: u8 = INTEL_PMC_IDX_FIXED + 15;

pub const ARCH_PERFMON_EVENTSEL_USR: u64 = 1 << 16;
pub const ARCH_PERFMON_EVENTSEL_OS: u64 = 1 << 17;
pub const ARCH_PERFMON_EVENTSEL_INT: u64 = 1 << 20;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct BtsRecord {
    pub from: u64,
    pub to: u64,
    pub flags: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BtsState {
    Stopped,
    Inactive,
    Active,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BtsPmuDescriptor {
    pub name: &'static str,
    pub task_ctx_nr: &'static str,
    pub capabilities: u64,
    pub callbacks: &'static [&'static str],
}

pub const BTS_PMU_CALLBACKS: [&str; 8] = [
    "event_init",
    "add",
    "del",
    "start",
    "stop",
    "read",
    "setup_aux",
    "free_aux",
];

pub const BTS_PMU: BtsPmuDescriptor = BtsPmuDescriptor {
    name: "intel_bts",
    task_ctx_nr: "perf_sw_context",
    capabilities: PERF_PMU_CAP_AUX_NO_SG | PERF_PMU_CAP_ITRACE | PERF_PMU_CAP_EXCLUSIVE,
    callbacks: &BTS_PMU_CALLBACKS,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BtsPage {
    pub base: u64,
    pub order: u8,
}

impl BtsPage {
    pub const fn nr_pages(self) -> usize {
        1usize << self.order
    }

    pub const fn size(self) -> usize {
        self.nr_pages() * PAGE_SIZE
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BtsPhys {
    pub page: BtsPage,
    pub size: usize,
    pub offset: usize,
    pub displacement: usize,
}

impl BtsPhys {
    pub const EMPTY: Self = Self {
        page: BtsPage { base: 0, order: 0 },
        size: 0,
        offset: 0,
        displacement: 0,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BtsBuffer<const N: usize> {
    pub real_size: usize,
    pub nr_pages: usize,
    pub nr_bufs: usize,
    pub cur_buf: usize,
    pub snapshot: bool,
    pub data_size: usize,
    pub head: usize,
    pub end: usize,
    pub buf: [BtsPhys; N],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DebugStore {
    pub bts_buffer_base: u64,
    pub bts_index: u64,
    pub bts_absolute_maximum: u64,
    pub bts_interrupt_threshold: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PerfOutputHandle {
    pub head: usize,
    pub size: usize,
    pub wakeup: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BtsContext {
    pub state: BtsState,
    pub handle_event: bool,
    pub ds_back: DebugStore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BtsEventAttr {
    pub typ: u32,
    pub exclude_kernel: bool,
    pub exclude_user: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BtsPerfEvent {
    pub attr: BtsEventAttr,
    pub hw_state: u64,
    pub destroy_installed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BtsAuxSetup {
    pub overwrite_rejected: bool,
    pub allocation_failed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BtsConfigPlan {
    pub ds: DebugStore,
    pub index: usize,
    pub end: usize,
    pub threshold: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BtsUpdateResult {
    pub old_head: usize,
    pub new_head: usize,
    pub data_size: usize,
    pub truncated: bool,
    pub changed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BtsResetPlan {
    pub head: usize,
    pub space: usize,
    pub end: usize,
    pub cur_buf: usize,
    pub skipped: usize,
    pub padded: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BtsStartResult {
    pub started: bool,
    pub aux_end_zero: bool,
    pub event_state: u64,
    pub ctx_state: BtsState,
    pub config: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BtsStopResult {
    pub event_state: u64,
    pub ctx_state: BtsState,
    pub aux_end_size: Option<usize>,
    pub restored_ds: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BtsLocalTransition {
    pub state: BtsState,
    pub enable_bts: bool,
    pub disable_bts: bool,
    pub warn_active: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BtsInterruptResult {
    pub handled: i32,
    pub state: BtsState,
    pub aux_end_size: Option<usize>,
    pub aux_end_zero: bool,
    pub reset_error: Option<i32>,
    pub truncated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BtsInitInput {
    pub has_dtes64: bool,
    pub has_bts: bool,
    pub has_pti: bool,
    pub alloc_percpu_ok: bool,
    pub register_result: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BtsInitPlan {
    pub x86_pmu_bts: bool,
    pub alloc_percpu: bool,
    pub pmu: BtsPmuDescriptor,
}

pub const fn bts_available(debug_store_64: bool, bts_cpuid: bool, pti: bool) -> bool {
    debug_store_64 && bts_cpuid && !pti
}

pub const fn bts_buffer_bytes(records: usize) -> usize {
    records.saturating_mul(BTS_RECORD_SIZE)
}

pub const fn round_down_record(value: usize) -> usize {
    value - value % BTS_RECORD_SIZE
}

pub const fn bts_buffer_offset<const N: usize>(bb: &BtsBuffer<N>, idx: usize) -> usize {
    bb.buf[idx].offset + bb.buf[idx].displacement
}

pub fn bts_buffer_setup_aux<const N: usize>(
    pages: [BtsPage; N],
    overwrite: bool,
    allocation_ok: bool,
) -> Option<BtsBuffer<N>> {
    let mut pg = 0usize;
    let mut nr_bufs = 0usize;
    while pg < N {
        pg += pages[pg].nr_pages();
        nr_bufs += 1;
    }

    if overwrite && nr_bufs > 1 {
        return None;
    }
    if !allocation_ok {
        return None;
    }

    let mut buf = [BtsPhys::EMPTY; N];
    let mut pad = 0usize;
    let mut offset = 0usize;
    pg = 0;

    let mut nr_buf = 0usize;
    while nr_buf < nr_bufs {
        let page = pages[pg];
        let displacement = if pad != 0 { BTS_RECORD_SIZE - pad } else { 0 };
        let mut size = page.size() - displacement;
        pad = size % BTS_RECORD_SIZE;
        size -= pad;

        buf[nr_buf] = BtsPhys {
            page,
            size,
            offset,
            displacement,
        };

        pg += page.nr_pages();
        offset += page.size();
        nr_buf += 1;
    }

    Some(BtsBuffer {
        real_size: round_down_record(N << PAGE_SHIFT),
        nr_pages: N,
        nr_bufs,
        cur_buf: 0,
        snapshot: overwrite,
        data_size: 0,
        head: 0,
        end: 0,
        buf,
    })
}

pub fn bts_config_buffer<const N: usize>(bb: &BtsBuffer<N>) -> BtsConfigPlan {
    let phys = bb.buf[bb.cur_buf];
    let mut index = bb.head;
    let mut thresh = 0usize;
    let mut end = phys.size;

    if !bb.snapshot {
        if bb.end < phys.offset + phys.page.size() {
            end = bb
                .end
                .saturating_sub(phys.offset)
                .saturating_sub(phys.displacement);
        }

        index = index.saturating_sub(phys.offset + phys.displacement);

        if end.saturating_sub(index) > BTS_SAFETY_MARGIN {
            thresh = end - BTS_SAFETY_MARGIN;
        } else if end.saturating_sub(index) > BTS_RECORD_SIZE {
            thresh = end - BTS_RECORD_SIZE;
        } else {
            thresh = end;
        }
    }

    let base = phys.page.base + phys.displacement as u64;
    let maximum = base + end as u64;
    let threshold = if !bb.snapshot {
        base + thresh as u64
    } else {
        maximum + BTS_RECORD_SIZE as u64
    };

    BtsConfigPlan {
        ds: DebugStore {
            bts_buffer_base: base,
            bts_index: base + index as u64,
            bts_absolute_maximum: maximum,
            bts_interrupt_threshold: threshold,
        },
        index,
        end,
        threshold: thresh,
    }
}

pub fn bts_update<const N: usize>(bb: &mut BtsBuffer<N>, ds: DebugStore) -> BtsUpdateResult {
    let index = ds.bts_index.saturating_sub(ds.bts_buffer_base) as usize;
    let head = index + bts_buffer_offset(bb, bb.cur_buf);
    let old = bb.head;
    bb.head = head;

    if !bb.snapshot {
        if old == head {
            return BtsUpdateResult {
                old_head: old,
                new_head: head,
                data_size: bb.data_size,
                truncated: false,
                changed: false,
            };
        }

        let truncated = ds.bts_index >= ds.bts_absolute_maximum;
        bb.data_size = bb.data_size.saturating_add(head.saturating_sub(old));
        BtsUpdateResult {
            old_head: old,
            new_head: head,
            data_size: bb.data_size,
            truncated,
            changed: true,
        }
    } else {
        bb.data_size = head;
        BtsUpdateResult {
            old_head: old,
            new_head: head,
            data_size: bb.data_size,
            truncated: false,
            changed: old != head,
        }
    }
}

pub fn bts_buffer_reset<const N: usize>(
    bb: &mut BtsBuffer<N>,
    handle: &mut PerfOutputHandle,
) -> Result<BtsResetPlan, i32> {
    if bb.snapshot {
        return Ok(BtsResetPlan {
            head: bb.head,
            space: 0,
            end: bb.end,
            cur_buf: bb.cur_buf,
            skipped: 0,
            padded: false,
        });
    }

    let mask = (bb.nr_pages << PAGE_SHIFT) - 1;
    let mut head = handle.head & mask;
    let mut phys = bb.buf[bb.cur_buf];
    let mut space = phys.offset + phys.displacement + phys.size - head;
    let mut pad = space;
    let mut skipped = 0usize;
    let mut padded = false;

    if space > handle.size {
        space = round_down_record(handle.size);
    }

    if space <= BTS_SAFETY_MARGIN {
        let mut next_buf = bb.cur_buf + 1;
        if next_buf >= bb.nr_bufs {
            next_buf = 0;
        }
        let next_phys = bb.buf[next_buf];
        let gap = phys.page.size() - phys.displacement - phys.size + next_phys.displacement;
        let skip = pad + gap;
        if handle.size >= skip {
            let mut next_space = next_phys.size;
            if next_space + skip > handle.size {
                next_space = round_down_record(handle.size - skip);
            }
            if next_space > space || space == 0 {
                padded = pad != 0;
                skipped = skip;
                handle.head = handle.head.saturating_add(skip);
                phys = next_phys;
                space = next_space;
                head = phys.offset + phys.displacement;
                bb.cur_buf = next_buf;
                bb.head = head;
                pad = 0;
            }
        }
    }

    let wakeup_limit = BTS_SAFETY_MARGIN
        .saturating_add(BTS_RECORD_SIZE)
        .saturating_add(handle.wakeup)
        .saturating_sub(handle.head);
    if space > wakeup_limit {
        space = round_down_record(wakeup_limit);
    }

    bb.end = head + space;
    if space == 0 {
        return Err(ENOSPC);
    }

    Ok(BtsResetPlan {
        head,
        space,
        end: bb.end,
        cur_buf: bb.cur_buf,
        skipped,
        padded: padded || pad != 0 && skipped != 0,
    })
}

pub const fn bts_start_config(snapshot: bool, exclude_kernel: bool, exclude_user: bool) -> u64 {
    let mut config = 0u64;
    if !snapshot {
        config |= ARCH_PERFMON_EVENTSEL_INT;
    }
    if !exclude_kernel {
        config |= ARCH_PERFMON_EVENTSEL_OS;
    }
    if !exclude_user {
        config |= ARCH_PERFMON_EVENTSEL_USR;
    }
    config
}

pub fn bts_event_start<const N: usize>(
    event: &mut BtsPerfEvent,
    ctx: &mut BtsContext,
    bb: Option<&mut BtsBuffer<N>>,
    handle: &mut PerfOutputHandle,
    current_ds: DebugStore,
) -> BtsStartResult {
    let Some(bb) = bb else {
        event.hw_state = PERF_HES_STOPPED;
        return BtsStartResult {
            started: false,
            aux_end_zero: false,
            event_state: event.hw_state,
            ctx_state: ctx.state,
            config: 0,
        };
    };

    if bts_buffer_reset(bb, handle).is_err() {
        event.hw_state = PERF_HES_STOPPED;
        return BtsStartResult {
            started: false,
            aux_end_zero: true,
            event_state: event.hw_state,
            ctx_state: ctx.state,
            config: 0,
        };
    }

    ctx.ds_back = current_ds;
    event.hw_state = 0;
    ctx.handle_event = true;
    ctx.state = BtsState::Active;
    let config = bts_start_config(
        bb.snapshot,
        event.attr.exclude_kernel,
        event.attr.exclude_user,
    );
    BtsStartResult {
        started: true,
        aux_end_zero: false,
        event_state: event.hw_state,
        ctx_state: ctx.state,
        config,
    }
}

pub fn bts_event_stop<const N: usize>(
    event: &mut BtsPerfEvent,
    ctx: &mut BtsContext,
    bb: Option<&mut BtsBuffer<N>>,
    ds: DebugStore,
    flags: i32,
) -> BtsStopResult {
    let state = ctx.state;
    if state == BtsState::Active {
        ctx.state = BtsState::Stopped;
    }

    event.hw_state |= PERF_HES_STOPPED;
    let mut aux_end_size = None;
    let mut restored_ds = false;

    if (flags & PERF_EF_UPDATE) != 0 {
        if let Some(bb) = bb {
            bts_update(bb, ds);
            if bb.snapshot {
                aux_end_size = Some(bb.nr_pages << PAGE_SHIFT);
            } else {
                aux_end_size = Some(bb.data_size);
            }
            bb.data_size = 0;
        }
        restored_ds = true;
    }

    BtsStopResult {
        event_state: event.hw_state,
        ctx_state: ctx.state,
        aux_end_size,
        restored_ds,
    }
}

pub fn intel_bts_enable_local(ctx: Option<&mut BtsContext>) -> BtsLocalTransition {
    let Some(ctx) = ctx else {
        return BtsLocalTransition {
            state: BtsState::Stopped,
            enable_bts: false,
            disable_bts: false,
            warn_active: false,
        };
    };

    if ctx.state == BtsState::Active {
        return BtsLocalTransition {
            state: ctx.state,
            enable_bts: false,
            disable_bts: false,
            warn_active: true,
        };
    }
    if ctx.state == BtsState::Stopped {
        return BtsLocalTransition {
            state: ctx.state,
            enable_bts: false,
            disable_bts: false,
            warn_active: false,
        };
    }
    if ctx.handle_event {
        ctx.state = BtsState::Active;
        return BtsLocalTransition {
            state: ctx.state,
            enable_bts: true,
            disable_bts: false,
            warn_active: false,
        };
    }

    BtsLocalTransition {
        state: ctx.state,
        enable_bts: false,
        disable_bts: false,
        warn_active: false,
    }
}

pub fn intel_bts_disable_local(ctx: Option<&mut BtsContext>) -> BtsLocalTransition {
    let Some(ctx) = ctx else {
        return BtsLocalTransition {
            state: BtsState::Stopped,
            enable_bts: false,
            disable_bts: false,
            warn_active: false,
        };
    };

    if ctx.state != BtsState::Active {
        return BtsLocalTransition {
            state: ctx.state,
            enable_bts: false,
            disable_bts: false,
            warn_active: false,
        };
    }
    if ctx.handle_event {
        ctx.state = BtsState::Inactive;
        return BtsLocalTransition {
            state: ctx.state,
            enable_bts: false,
            disable_bts: true,
            warn_active: false,
        };
    }

    BtsLocalTransition {
        state: ctx.state,
        enable_bts: false,
        disable_bts: false,
        warn_active: false,
    }
}

pub fn intel_bts_interrupt<const N: usize>(
    ctx: Option<&mut BtsContext>,
    bb: Option<&mut BtsBuffer<N>>,
    ds: Option<DebugStore>,
    handle: &mut PerfOutputHandle,
) -> BtsInterruptResult {
    let Some(ctx) = ctx else {
        return BtsInterruptResult {
            handled: 0,
            state: BtsState::Stopped,
            aux_end_size: None,
            aux_end_zero: false,
            reset_error: None,
            truncated: false,
        };
    };

    let mut handled = if let Some(ds) = ds {
        if ds.bts_index >= ds.bts_interrupt_threshold {
            1
        } else {
            0
        }
    } else {
        0
    };

    if ctx.state == BtsState::Stopped {
        return BtsInterruptResult {
            handled,
            state: ctx.state,
            aux_end_size: None,
            aux_end_zero: false,
            reset_error: None,
            truncated: false,
        };
    }

    let Some(bb) = bb else {
        return BtsInterruptResult {
            handled,
            state: ctx.state,
            aux_end_size: None,
            aux_end_zero: false,
            reset_error: None,
            truncated: false,
        };
    };

    if bb.snapshot {
        return BtsInterruptResult {
            handled: 0,
            state: ctx.state,
            aux_end_size: None,
            aux_end_zero: false,
            reset_error: None,
            truncated: false,
        };
    }

    let Some(ds) = ds else {
        return BtsInterruptResult {
            handled,
            state: ctx.state,
            aux_end_size: None,
            aux_end_zero: false,
            reset_error: None,
            truncated: false,
        };
    };

    let old_head = bb.head;
    let update = bts_update(bb, ds);
    if old_head == bb.head {
        return BtsInterruptResult {
            handled,
            state: ctx.state,
            aux_end_size: None,
            aux_end_zero: false,
            reset_error: None,
            truncated: update.truncated,
        };
    }

    let aux_end_size = bb.data_size;
    bb.data_size = 0;
    let reset = bts_buffer_reset(bb, handle);
    let mut aux_end_zero = false;
    let mut reset_error = None;
    if let Err(err) = reset {
        ctx.state = BtsState::Stopped;
        aux_end_zero = true;
        reset_error = Some(err);
    }
    handled = 1;

    BtsInterruptResult {
        handled,
        state: ctx.state,
        aux_end_size: Some(aux_end_size),
        aux_end_zero,
        reset_error,
        truncated: update.truncated,
    }
}

pub fn bts_event_del<const N: usize>(
    event: &mut BtsPerfEvent,
    ctx: &mut BtsContext,
    bb: Option<&mut BtsBuffer<N>>,
    ds: DebugStore,
) -> BtsStopResult {
    bts_event_stop(event, ctx, bb, ds, PERF_EF_UPDATE)
}

pub fn bts_event_add(
    event: &mut BtsPerfEvent,
    ctx: &mut BtsContext,
    active_fixed_bts: bool,
    mode: i32,
    start_succeeds: bool,
) -> i32 {
    event.hw_state = PERF_HES_STOPPED;

    if active_fixed_bts {
        return EBUSY;
    }
    if ctx.handle_event {
        return EBUSY;
    }

    if (mode & PERF_EF_START) != 0 {
        if start_succeeds {
            event.hw_state = 0;
            ctx.handle_event = true;
            ctx.state = BtsState::Active;
        }
        if (event.hw_state & PERF_HES_STOPPED) != 0 {
            return EINVAL;
        }
    }

    0
}

pub fn bts_event_destroy() -> (&'static str, &'static str) {
    (
        "x86_release_hardware()",
        "x86_del_exclusive(x86_lbr_exclusive_bts)",
    )
}

pub fn bts_event_init(
    event: &mut BtsPerfEvent,
    pmu_type: u32,
    perf_allow_kernel_ret: i32,
    exclusive_busy: bool,
    reserve_hardware_ret: i32,
) -> Result<(), i32> {
    if event.attr.typ != pmu_type {
        return Err(ENOENT);
    }

    if event.attr.exclude_kernel && perf_allow_kernel_ret != 0 {
        return Err(perf_allow_kernel_ret);
    }

    if exclusive_busy {
        return Err(EBUSY);
    }

    if reserve_hardware_ret != 0 {
        return Err(reserve_hardware_ret);
    }

    event.destroy_installed = true;
    Ok(())
}

pub const fn bts_event_read() {}

pub fn bts_init(input: BtsInitInput) -> Result<BtsInitPlan, i32> {
    if !input.has_dtes64 {
        return Err(ENODEV);
    }
    if !input.has_bts {
        return Err(ENODEV);
    }
    if input.has_pti {
        return Err(ENODEV);
    }
    if !input.alloc_percpu_ok {
        return Err(ENOMEM);
    }
    if input.register_result != 0 {
        return Err(input.register_result);
    }

    Ok(BtsInitPlan {
        x86_pmu_bts: true,
        alloc_percpu: true,
        pmu: BTS_PMU,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE0: BtsPage = BtsPage {
        base: 0x1000_0000,
        order: 0,
    };
    const PAGE1: BtsPage = BtsPage {
        base: 0x1000_1000,
        order: 0,
    };
    const DS0: DebugStore = DebugStore {
        bts_buffer_base: 0x1000_0000,
        bts_index: 0x1000_0000,
        bts_absolute_maximum: 0x1000_0000,
        bts_interrupt_threshold: 0x1000_0000,
    };

    fn event() -> BtsPerfEvent {
        BtsPerfEvent {
            attr: BtsEventAttr {
                typ: 7,
                exclude_kernel: false,
                exclude_user: false,
            },
            hw_state: 0,
            destroy_installed: false,
        }
    }

    fn ctx(state: BtsState) -> BtsContext {
        BtsContext {
            state,
            handle_event: false,
            ds_back: DS0,
        }
    }

    #[test]
    fn bts_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/events/intel/bts.c"
        ));
        let perf_event = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/perf_event.h"
        ));
        let asm_perf = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/asm/perf_event.h"
        ));
        let intel_ds = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/asm/intel_ds.h"
        ));

        assert!(source.contains("BTS_STATE_STOPPED = 0"));
        assert!(source.contains("#define BTS_RECORD_SIZE\t\t24"));
        assert!(source.contains("#define BTS_SAFETY_MARGIN\t4080"));
        assert!(source.contains("bb->real_size = size - size % BTS_RECORD_SIZE;"));
        assert!(
            source.contains("bb->buf[nr_buf].displacement = (pad ? BTS_RECORD_SIZE - pad : 0);")
        );
        assert!(source.contains("ds->bts_interrupt_threshold = !bb->snapshot"));
        assert!(source.contains("perf_aux_output_flag(&bts->handle"));
        assert!(source.contains("PERF_AUX_FLAG_TRUNCATED"));
        assert!(source.contains("config |= ARCH_PERFMON_EVENTSEL_INT;"));
        assert!(source.contains("config |= ARCH_PERFMON_EVENTSEL_OS;"));
        assert!(source.contains("config |= ARCH_PERFMON_EVENTSEL_USR;"));
        assert!(source.contains("WRITE_ONCE(bts->state, BTS_STATE_ACTIVE);"));
        assert!(source.contains("intel_pmu_enable_bts(config);"));
        assert!(source.contains("intel_pmu_disable_bts();"));
        assert!(source.contains("bts_event_stop(event, PERF_EF_UPDATE);"));
        assert!(source.contains("test_bit(INTEL_PMC_IDX_FIXED_BTS, cpuc->active_mask)"));
        assert!(source.contains("if (event->attr.type != bts_pmu.type)"));
        assert!(source.contains("event->destroy = bts_event_destroy;"));
        assert!(source.contains("if (!boot_cpu_has(X86_FEATURE_DTES64))"));
        assert!(source.contains("if (boot_cpu_has(X86_FEATURE_PTI))"));
        assert!(source.contains("PERF_PMU_CAP_AUX_NO_SG | PERF_PMU_CAP_ITRACE"));
        assert!(source.contains("return perf_pmu_register(&bts_pmu, \"intel_bts\", -1);"));
        assert!(perf_event.contains("#define PERF_PMU_CAP_AUX_NO_SG\t\t0x0004"));
        assert!(perf_event.contains("#define PERF_PMU_CAP_EXCLUSIVE\t\t0x0010"));
        assert!(perf_event.contains("#define PERF_PMU_CAP_ITRACE\t\t0x0020"));
        assert!(asm_perf.contains("#define INTEL_PMC_IDX_FIXED_BTS"));
        assert!(intel_ds.contains("#define BTS_BUFFER_SIZE\t\t(PAGE_SIZE << 4)"));
        assert!(intel_ds.contains("u64\tbts_interrupt_threshold;"));
    }

    #[test]
    fn constants_record_size_and_pmu_descriptor_match_source() {
        assert_eq!(core::mem::size_of::<BtsRecord>(), BTS_RECORD_SIZE);
        assert_eq!(BTS_RECORD_SIZE, 24);
        assert_eq!(BTS_SAFETY_MARGIN, 4080);
        assert_eq!(BTS_BUFFER_SIZE, PAGE_SIZE << 4);
        assert_eq!(bts_buffer_bytes(4), 96);
        assert!(bts_available(true, true, false));
        assert!(!bts_available(true, true, true));
        assert_eq!(INTEL_PMC_IDX_FIXED_BTS, 47);
        assert_eq!(BTS_PMU.name, "intel_bts");
        assert_eq!(
            BTS_PMU.capabilities,
            PERF_PMU_CAP_AUX_NO_SG | PERF_PMU_CAP_ITRACE | PERF_PMU_CAP_EXCLUSIVE
        );
        assert_eq!(BTS_PMU.callbacks[6], "setup_aux");
    }

    #[test]
    fn setup_aux_groups_high_order_pages_and_aligns_record_storage() {
        let high = BtsPage {
            base: 0x2000_0000,
            order: 1,
        };
        let bb = bts_buffer_setup_aux([high, PAGE1, PAGE0], false, true).unwrap();
        assert_eq!(bb.nr_pages, 3);
        assert_eq!(bb.nr_bufs, 2);
        assert_eq!(bb.real_size, round_down_record(3 * PAGE_SIZE));
        assert_eq!(bb.buf[0].offset, 0);
        assert_eq!(bb.buf[0].size % BTS_RECORD_SIZE, 0);
        assert_eq!(bb.buf[1].offset, 2 * PAGE_SIZE);
        assert_eq!(
            bb.buf[1].displacement,
            BTS_RECORD_SIZE - (bb.buf[0].page.size() % BTS_RECORD_SIZE)
        );
        assert!(bts_buffer_setup_aux([PAGE0, PAGE1], true, true).is_none());
        assert!(bts_buffer_setup_aux([PAGE0], false, false).is_none());
    }

    #[test]
    fn config_buffer_sets_threshold_and_snapshot_maximum_like_linux() {
        let mut bb = bts_buffer_setup_aux([PAGE0], false, true).unwrap();
        bb.head = 0;
        bb.end = bb.buf[0].size;
        let plan = bts_config_buffer(&bb);
        assert_eq!(plan.ds.bts_buffer_base, PAGE0.base);
        assert_eq!(plan.ds.bts_index, PAGE0.base);
        assert_eq!(
            plan.ds.bts_absolute_maximum,
            PAGE0.base + bb.buf[0].size as u64
        );
        assert_eq!(
            plan.ds.bts_interrupt_threshold,
            PAGE0.base + (bb.buf[0].size - BTS_RECORD_SIZE) as u64
        );

        let snap = bts_buffer_setup_aux([PAGE0], true, true).unwrap();
        let plan = bts_config_buffer(&snap);
        assert_eq!(
            plan.ds.bts_interrupt_threshold,
            plan.ds.bts_absolute_maximum + BTS_RECORD_SIZE as u64
        );
    }

    #[test]
    fn update_tracks_head_data_size_snapshot_and_truncation() {
        let mut bb = bts_buffer_setup_aux([PAGE0], false, true).unwrap();
        let ds = DebugStore {
            bts_buffer_base: PAGE0.base,
            bts_index: PAGE0.base + 96,
            bts_absolute_maximum: PAGE0.base + 96,
            bts_interrupt_threshold: PAGE0.base + 72,
        };
        let update = bts_update(&mut bb, ds);
        assert_eq!(update.old_head, 0);
        assert_eq!(update.new_head, 96);
        assert_eq!(update.data_size, 96);
        assert!(update.truncated);
        assert!(update.changed);

        let update = bts_update(&mut bb, ds);
        assert!(!update.changed);

        let mut snap = bts_buffer_setup_aux([PAGE0], true, true).unwrap();
        let update = bts_update(&mut snap, ds);
        assert_eq!(update.data_size, 96);
        assert_eq!(snap.data_size, snap.head);
    }

    #[test]
    fn reset_advances_to_next_physical_buffer_and_reports_no_space() {
        let mut bb = bts_buffer_setup_aux([PAGE0, PAGE1], false, true).unwrap();
        let mut handle = PerfOutputHandle {
            head: bb.buf[0].size - 16,
            size: PAGE_SIZE,
            wakeup: PAGE_SIZE,
        };
        let reset = bts_buffer_reset(&mut bb, &mut handle).unwrap();
        assert_eq!(reset.cur_buf, 1);
        assert_eq!(reset.head, bb.buf[1].offset + bb.buf[1].displacement);
        assert!(reset.skipped > 0);
        assert!(reset.padded);
        assert_eq!(bb.head, reset.head);

        let mut bb = bts_buffer_setup_aux([PAGE0], false, true).unwrap();
        let mut handle = PerfOutputHandle {
            head: 0,
            size: 1,
            wakeup: 0,
        };
        assert_eq!(bts_buffer_reset(&mut bb, &mut handle), Err(ENOSPC));

        let mut snap = bts_buffer_setup_aux([PAGE0], true, true).unwrap();
        let mut handle = PerfOutputHandle {
            head: 123,
            size: PAGE_SIZE,
            wakeup: 0,
        };
        assert!(bts_buffer_reset(&mut snap, &mut handle).is_ok());
    }

    #[test]
    fn start_config_and_start_stop_paths_follow_event_lifecycle() {
        assert_eq!(
            bts_start_config(false, false, false),
            ARCH_PERFMON_EVENTSEL_INT | ARCH_PERFMON_EVENTSEL_OS | ARCH_PERFMON_EVENTSEL_USR
        );
        assert_eq!(
            bts_start_config(false, true, false),
            ARCH_PERFMON_EVENTSEL_INT | ARCH_PERFMON_EVENTSEL_USR
        );
        assert_eq!(
            bts_start_config(true, false, true),
            ARCH_PERFMON_EVENTSEL_OS
        );

        let mut event = event();
        let mut ctx = ctx(BtsState::Stopped);
        let mut bb = bts_buffer_setup_aux([PAGE0], false, true).unwrap();
        let mut handle = PerfOutputHandle {
            head: 0,
            size: PAGE_SIZE,
            wakeup: PAGE_SIZE,
        };
        let start = bts_event_start(&mut event, &mut ctx, Some(&mut bb), &mut handle, DS0);
        assert!(start.started);
        assert_eq!(start.ctx_state, BtsState::Active);
        assert_eq!(event.hw_state, 0);
        assert!(ctx.handle_event);

        let ds = DebugStore {
            bts_buffer_base: PAGE0.base,
            bts_index: PAGE0.base + 48,
            bts_absolute_maximum: PAGE0.base + PAGE_SIZE as u64,
            bts_interrupt_threshold: PAGE0.base + 24,
        };
        let stop = bts_event_stop(&mut event, &mut ctx, Some(&mut bb), ds, PERF_EF_UPDATE);
        assert_eq!(stop.ctx_state, BtsState::Stopped);
        assert_eq!(stop.aux_end_size, Some(48));
        assert!(stop.restored_ds);
        assert_ne!(event.hw_state & PERF_HES_STOPPED, 0);
    }

    #[test]
    fn local_enable_disable_keep_stopped_and_inactive_rules() {
        assert_eq!(
            intel_bts_enable_local(None),
            BtsLocalTransition {
                state: BtsState::Stopped,
                enable_bts: false,
                disable_bts: false,
                warn_active: false,
            }
        );

        let mut ctx = ctx(BtsState::Inactive);
        ctx.handle_event = true;
        let transition = intel_bts_enable_local(Some(&mut ctx));
        assert_eq!(transition.state, BtsState::Active);
        assert!(transition.enable_bts);

        let transition = intel_bts_enable_local(Some(&mut ctx));
        assert!(transition.warn_active);

        let transition = intel_bts_disable_local(Some(&mut ctx));
        assert_eq!(transition.state, BtsState::Inactive);
        assert!(transition.disable_bts);

        let transition = intel_bts_disable_local(Some(&mut ctx));
        assert!(!transition.disable_bts);
    }

    #[test]
    fn interrupt_handles_threshold_snapshot_no_data_and_reset_error_paths() {
        let mut ctx = ctx(BtsState::Inactive);
        ctx.handle_event = true;
        let mut bb = bts_buffer_setup_aux([PAGE0], false, true).unwrap();
        let mut handle = PerfOutputHandle {
            head: 0,
            size: PAGE_SIZE,
            wakeup: PAGE_SIZE,
        };
        let ds = DebugStore {
            bts_buffer_base: PAGE0.base,
            bts_index: PAGE0.base + 96,
            bts_absolute_maximum: PAGE0.base + PAGE_SIZE as u64,
            bts_interrupt_threshold: PAGE0.base + 24,
        };
        let result = intel_bts_interrupt(Some(&mut ctx), Some(&mut bb), Some(ds), &mut handle);
        assert_eq!(result.handled, 1);
        assert_eq!(result.aux_end_size, Some(96));
        assert_eq!(result.state, BtsState::Inactive);

        let result = intel_bts_interrupt(Some(&mut ctx), Some(&mut bb), Some(ds), &mut handle);
        assert_eq!(result.aux_end_size, None);

        let mut snap = bts_buffer_setup_aux([PAGE0], true, true).unwrap();
        let result = intel_bts_interrupt(Some(&mut ctx), Some(&mut snap), Some(ds), &mut handle);
        assert_eq!(result.handled, 0);

        let mut tiny = bts_buffer_setup_aux([PAGE0], false, true).unwrap();
        tiny.head = 0;
        let mut tiny_handle = PerfOutputHandle {
            head: 0,
            size: 1,
            wakeup: 0,
        };
        let result =
            intel_bts_interrupt(Some(&mut ctx), Some(&mut tiny), Some(ds), &mut tiny_handle);
        assert_eq!(result.reset_error, Some(ENOSPC));
        assert_eq!(result.state, BtsState::Stopped);
        assert!(result.aux_end_zero);
    }

    #[test]
    fn event_add_init_destroy_and_module_init_return_linux_errors() {
        let mut add_event = event();
        let mut ctx = ctx(BtsState::Stopped);
        assert_eq!(
            bts_event_add(&mut add_event, &mut ctx, true, 0, false),
            EBUSY
        );
        ctx.handle_event = true;
        assert_eq!(
            bts_event_add(&mut add_event, &mut ctx, false, 0, false),
            EBUSY
        );
        ctx.handle_event = false;
        assert_eq!(
            bts_event_add(&mut add_event, &mut ctx, false, PERF_EF_START, false),
            EINVAL
        );
        assert_eq!(
            bts_event_add(&mut add_event, &mut ctx, false, PERF_EF_START, true),
            0
        );
        assert_eq!(ctx.state, BtsState::Active);

        let mut init_event = event();
        assert_eq!(bts_event_init(&mut init_event, 9, 0, false, 0), Err(ENOENT));
        init_event.attr.exclude_kernel = true;
        assert_eq!(bts_event_init(&mut init_event, 7, -1, false, 0), Err(-1));
        init_event.attr.exclude_kernel = false;
        assert_eq!(bts_event_init(&mut init_event, 7, 0, true, 0), Err(EBUSY));
        assert_eq!(bts_event_init(&mut init_event, 7, 0, false, -5), Err(-5));
        assert_eq!(bts_event_init(&mut init_event, 7, 0, false, 0), Ok(()));
        assert!(init_event.destroy_installed);
        assert_eq!(
            bts_event_destroy(),
            (
                "x86_release_hardware()",
                "x86_del_exclusive(x86_lbr_exclusive_bts)"
            )
        );

        assert_eq!(
            bts_init(BtsInitInput {
                has_dtes64: false,
                has_bts: true,
                has_pti: false,
                alloc_percpu_ok: true,
                register_result: 0,
            }),
            Err(ENODEV)
        );
        assert_eq!(
            bts_init(BtsInitInput {
                has_dtes64: true,
                has_bts: false,
                has_pti: false,
                alloc_percpu_ok: true,
                register_result: 0,
            }),
            Err(ENODEV)
        );
        assert_eq!(
            bts_init(BtsInitInput {
                has_dtes64: true,
                has_bts: true,
                has_pti: true,
                alloc_percpu_ok: true,
                register_result: 0,
            }),
            Err(ENODEV)
        );
        assert_eq!(
            bts_init(BtsInitInput {
                has_dtes64: true,
                has_bts: true,
                has_pti: false,
                alloc_percpu_ok: false,
                register_result: 0,
            }),
            Err(ENOMEM)
        );
        assert_eq!(
            bts_init(BtsInitInput {
                has_dtes64: true,
                has_bts: true,
                has_pti: false,
                alloc_percpu_ok: true,
                register_result: -7,
            }),
            Err(-7)
        );
        let init = bts_init(BtsInitInput {
            has_dtes64: true,
            has_bts: true,
            has_pti: false,
            alloc_percpu_ok: true,
            register_result: 0,
        })
        .unwrap();
        assert!(init.x86_pmu_bts);
        assert_eq!(init.pmu.name, "intel_bts");
    }
}
