//! linux-parity: partial
//! linux-source: vendor/linux/block/bfq-iosched.c
//! test-origin: linux:vendor/linux/block/bfq-iosched.c
//! BFQ scheduler tunables, queue flags, and request charging.

use crate::include::uapi::errno::EINVAL;

use super::bfq_wf2q::{BfqEntity, bfq_ioprio_to_weight};

pub const NSEC_PER_USEC: u64 = 1_000;
pub const NSEC_PER_MSEC: u64 = 1_000_000;
pub const NSEC_PER_SEC: u64 = 1_000_000_000;
pub const HZ: u64 = 100;

pub const BFQ_IOPRIO_CLASSES: usize = 3;
pub const BFQ_MIN_WEIGHT: u16 = 1;
pub const BFQ_MAX_WEIGHT: u16 = 1000;
pub const BFQ_DEFAULT_QUEUE_IOPRIO: u16 = 4;
pub const BFQ_DEFAULT_GRP_IOPRIO: u16 = 0;
pub const BFQ_DEFAULT_GRP_CLASS: u16 = 2;
pub const MAX_BFQQ_NAME_LENGTH: usize = 16;
pub const BFQ_SOFTRT_WEIGHT_FACTOR: u16 = 100;
pub const BFQ_MAX_ACTUATORS: usize = 8;

pub const BFQ_FIFO_EXPIRE_ASYNC_NS: u64 = NSEC_PER_SEC / 4;
pub const BFQ_FIFO_EXPIRE_SYNC_NS: u64 = NSEC_PER_SEC / 8;
pub const BFQ_BACK_MAX: u32 = 16 * 1024;
pub const BFQ_BACK_PENALTY: u32 = 2;
pub const BFQ_SLICE_IDLE_NS: u64 = NSEC_PER_SEC / 125;
pub const BFQ_DEFAULT_MAX_BUDGET: u32 = 16 * 1024;
pub const BFQ_ASYNC_CHARGE_FACTOR: u32 = 3;
pub const BFQ_TIMEOUT_JIFFIES: u64 = HZ / 8;
pub const BFQ_MERGE_TIME_LIMIT_JIFFIES: u64 = HZ / 10;
pub const BFQ_MIN_TT_NS: u64 = 2 * NSEC_PER_MSEC;
pub const BFQ_HW_QUEUE_THRESHOLD: u32 = 3;
pub const BFQ_HW_QUEUE_SAMPLES: u32 = 32;
pub const BFQ_RATE_SHIFT: u32 = 16;
pub const BFQ_RATE_MIN_SAMPLES: u32 = 32;
pub const BFQ_RATE_MIN_INTERVAL_NS: u64 = 300 * NSEC_PER_MSEC;
pub const BFQ_RATE_REF_INTERVAL_NS: u64 = NSEC_PER_SEC;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum BfqqStateFlag {
    JustCreated = 0,
    Busy = 1,
    WaitRequest = 2,
    NonBlockingWaitRq = 3,
    FifoExpire = 4,
    HasShortTtime = 5,
    Sync = 6,
    IoBound = 7,
    InLargeBurst = 8,
    SoftrtUpdate = 9,
    Coop = 10,
    SplitCoop = 11,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum BfqqExpiration {
    TooIdle = 0,
    BudgetTimeout = 1,
    BudgetExhausted = 2,
    NoMoreRequests = 3,
    Preempted = 4,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BfqQueue {
    pub entity: BfqEntity,
    pub ioprio: u16,
    pub ioprio_class: u16,
    pub new_ioprio: u16,
    pub new_ioprio_class: u16,
    pub wr_coeff: u16,
    pub max_budget: u32,
    pub flags: u32,
    pub service_from_backlogged: i32,
    pub service_from_wr: i32,
    pub fifo_time_ns: u64,
    pub allocated: i32,
}

impl BfqQueue {
    pub fn new(ioprio: u16, ioprio_class: u16) -> Self {
        let weight = bfq_ioprio_to_weight(ioprio);
        Self {
            entity: BfqEntity::new(weight, BFQ_DEFAULT_MAX_BUDGET as i32),
            ioprio,
            ioprio_class,
            new_ioprio: ioprio,
            new_ioprio_class: ioprio_class,
            wr_coeff: 1,
            max_budget: (2 * BFQ_DEFAULT_MAX_BUDGET) / 3,
            flags: 0,
            service_from_backlogged: 0,
            service_from_wr: 0,
            fifo_time_ns: 0,
            allocated: 0,
        }
    }

    pub fn oom_queue() -> Self {
        let mut queue = Self::new(BFQ_DEFAULT_QUEUE_IOPRIO, BFQ_DEFAULT_GRP_CLASS);
        queue.entity.prio_changed = true;
        queue.clear_flag(BfqqStateFlag::JustCreated);
        queue
    }

    pub fn mark_flag(&mut self, flag: BfqqStateFlag) {
        self.flags |= 1 << flag as u8;
    }

    pub fn clear_flag(&mut self, flag: BfqqStateFlag) {
        self.flags &= !(1 << flag as u8);
    }

    pub fn has_flag(&self, flag: BfqqStateFlag) -> bool {
        self.flags & (1 << flag as u8) != 0
    }

    pub fn request_allocated(&mut self) {
        self.allocated = self.allocated.saturating_add(1);
    }

    pub fn request_freed(&mut self) {
        self.allocated = self.allocated.saturating_sub(1);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BfqData {
    pub num_actuators: usize,
    pub sector: [u64; BFQ_MAX_ACTUATORS],
    pub nr_sectors: [u64; BFQ_MAX_ACTUATORS],
    pub bfq_max_budget: u32,
    pub bfq_fifo_expire: [u64; 2],
    pub bfq_back_max: u32,
    pub bfq_back_penalty: u32,
    pub bfq_slice_idle: u64,
    pub bfq_timeout: u64,
    pub bfq_large_burst_thresh: u32,
    pub low_latency: bool,
    pub bfq_wr_coeff: u16,
    pub bfq_wr_rt_max_time: u64,
    pub bfq_wr_min_idle_time: u64,
    pub bfq_wr_min_inter_arr_async: u64,
    pub bfq_wr_max_softrt_rate: u32,
    pub peak_rate: u32,
    pub actuator_load_threshold: u32,
}

impl BfqData {
    pub fn new(capacity_sectors: u64, rotational: bool) -> Self {
        let reference = if rotational { 14_000 } else { 33_000 };
        let mut sector = [0; BFQ_MAX_ACTUATORS];
        let mut nr_sectors = [0; BFQ_MAX_ACTUATORS];
        nr_sectors[0] = capacity_sectors;
        sector[0] = 0;
        Self {
            num_actuators: 1,
            sector,
            nr_sectors,
            bfq_max_budget: BFQ_DEFAULT_MAX_BUDGET,
            bfq_fifo_expire: [BFQ_FIFO_EXPIRE_ASYNC_NS, BFQ_FIFO_EXPIRE_SYNC_NS],
            bfq_back_max: BFQ_BACK_MAX,
            bfq_back_penalty: BFQ_BACK_PENALTY,
            bfq_slice_idle: BFQ_SLICE_IDLE_NS,
            bfq_timeout: BFQ_TIMEOUT_JIFFIES,
            bfq_large_burst_thresh: 8,
            low_latency: true,
            bfq_wr_coeff: 30,
            bfq_wr_rt_max_time: 300,
            bfq_wr_min_idle_time: 2000,
            bfq_wr_min_inter_arr_async: 500,
            bfq_wr_max_softrt_rate: 7000,
            peak_rate: reference * 2 / 3,
            actuator_load_threshold: 4,
        }
    }

    pub fn set_actuator_ranges(&mut self, ranges: &[(u64, u64)]) {
        if ranges.is_empty() || ranges.len() > BFQ_MAX_ACTUATORS {
            return;
        }
        self.num_actuators = ranges.len();
        self.sector = [0; BFQ_MAX_ACTUATORS];
        self.nr_sectors = [0; BFQ_MAX_ACTUATORS];
        for (idx, (start, len)) in ranges.iter().copied().enumerate() {
            self.sector[idx] = start;
            self.nr_sectors[idx] = len;
        }
    }
}

pub fn bfq_serv_to_charge(sectors: u32, is_sync: bool) -> u32 {
    if is_sync {
        sectors
    } else {
        sectors.saturating_mul(BFQ_ASYNC_CHARGE_FACTOR)
    }
}

pub fn bfq_var_store(var: &mut u64, input: &str) -> Result<(), i32> {
    let value = input.trim().parse::<u64>().map_err(|_| -EINVAL)?;
    *var = value;
    Ok(())
}

pub fn store_clamped(var: &mut u64, input: &str, min: u64, max: u64, conv: u64) -> Result<(), i32> {
    let mut value = input.trim().parse::<u64>().map_err(|_| -EINVAL)?;
    value = value.clamp(min, max);
    *var = value.saturating_mul(conv);
    Ok(())
}

pub fn strict_guarantees_store(data: &mut BfqData, value: u64) -> bool {
    let enabled = value.min(1) != 0;
    if enabled && data.bfq_slice_idle < 8 * NSEC_PER_MSEC {
        data.bfq_slice_idle = 8 * NSEC_PER_MSEC;
    }
    enabled
}

pub fn low_latency_store(queue: &mut BfqData, value: u64) {
    queue.low_latency = value.min(1) != 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bfq_constants_and_flags_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/bfq-iosched.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/bfq-iosched.h"
        ));
        assert!(source.contains("BFQ_BFQQ_FNS(just_created);"));
        assert!(source.contains("static const u64 bfq_fifo_expire[2]"));
        assert!(source.contains("static const int bfq_async_charge_factor = 3;"));
        assert!(source.contains("bfqd->bfq_wr_coeff = 30;"));
        assert!(header.contains("#define BFQ_MAX_ACTUATORS 8"));
        assert!(header.contains("enum bfqq_state_flags"));

        assert_eq!(BFQ_FIFO_EXPIRE_ASYNC_NS, 250_000_000);
        assert_eq!(BFQ_FIFO_EXPIRE_SYNC_NS, 125_000_000);
        assert_eq!(BFQ_SLICE_IDLE_NS, 8_000_000);
        assert_eq!(BFQ_DEFAULT_MAX_BUDGET, 16 * 1024);

        let mut queue = BfqQueue::new(4, BFQ_DEFAULT_GRP_CLASS);
        queue.mark_flag(BfqqStateFlag::Busy);
        assert!(queue.has_flag(BfqqStateFlag::Busy));
        queue.clear_flag(BfqqStateFlag::Busy);
        assert!(!queue.has_flag(BfqqStateFlag::Busy));
    }

    #[test]
    fn init_queue_defaults_and_store_helpers_follow_bfq_init_queue() {
        let mut data = BfqData::new(1_000_000, false);
        assert_eq!(data.num_actuators, 1);
        assert_eq!(data.nr_sectors[0], 1_000_000);
        assert_eq!(data.bfq_wr_max_softrt_rate, 7000);
        assert_eq!(data.peak_rate, 22_000);
        assert_eq!(bfq_serv_to_charge(8, false), 24);
        assert_eq!(bfq_serv_to_charge(8, true), 8);

        let mut stored = 0;
        bfq_var_store(&mut stored, "12\n").unwrap();
        assert_eq!(stored, 12);
        store_clamped(&mut stored, "0", 1, 10, NSEC_PER_MSEC).unwrap();
        assert_eq!(stored, NSEC_PER_MSEC);
        assert!(strict_guarantees_store(&mut data, 1));
        assert_eq!(data.bfq_slice_idle, 8 * NSEC_PER_MSEC);
    }
}
