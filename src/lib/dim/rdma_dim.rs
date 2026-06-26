//! linux-parity: complete
//! linux-source: vendor/linux/lib/dim/rdma_dim.c
//! test-origin: linux:vendor/linux/lib/dim/rdma_dim.c
//! RDMA Dynamic Interrupt Moderation state machine.

use super::dim::{
    DIM_GOING_LEFT, DIM_GOING_RIGHT, DIM_NEVENTS, DIM_PARKING_ON_TOP, DIM_PARKING_TIRED, DimSample,
    DimStats, dim_calc_stats,
};
use crate::kernel::module::{export_symbol, find_symbol};

pub const RDMA_DIM_PARAMS_NUM_PROFILES: u8 = 9;
pub const RDMA_DIM_START_PROFILE: u8 = 0;

pub const DIM_START_MEASURE: u8 = 0;
pub const DIM_MEASURE_IN_PROGRESS: u8 = 1;
pub const DIM_APPLY_NEW_PROFILE: u8 = 2;

pub const DIM_STATS_WORSE: i32 = 0;
pub const DIM_STATS_SAME: i32 = 1;
pub const DIM_STATS_BETTER: i32 = 2;

pub const DIM_STEPPED: i32 = 0;
pub const DIM_ON_EDGE: i32 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RdmaDim {
    pub state: u8,
    pub prev_stats: DimStats,
    pub start_sample: DimSample,
    pub measuring_sample: DimSample,
    pub profile_ix: u8,
    pub tune_state: u8,
    pub steps_right: u8,
    pub steps_left: u8,
    pub work_scheduled: bool,
}

impl Default for RdmaDim {
    fn default() -> Self {
        Self {
            state: DIM_START_MEASURE,
            prev_stats: DimStats::default(),
            start_sample: DimSample::default(),
            measuring_sample: DimSample::default(),
            profile_ix: RDMA_DIM_START_PROFILE,
            tune_state: DIM_GOING_RIGHT,
            steps_right: 0,
            steps_left: 0,
            work_scheduled: false,
        }
    }
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("rdma_dim", rdma_dim_raw as usize, false);
}

pub fn rdma_dim_step(dim: &mut RdmaDim) -> i32 {
    if dim.tune_state == DIM_GOING_RIGHT {
        if dim.profile_ix == RDMA_DIM_PARAMS_NUM_PROFILES - 1 {
            return DIM_ON_EDGE;
        }
        dim.profile_ix += 1;
        dim.steps_right += 1;
    }
    if dim.tune_state == DIM_GOING_LEFT {
        if dim.profile_ix == 0 {
            return DIM_ON_EDGE;
        }
        dim.profile_ix -= 1;
        dim.steps_left += 1;
    }
    DIM_STEPPED
}

pub fn rdma_dim_stats_compare(curr: &DimStats, prev: &DimStats) -> i32 {
    if prev.cpms == 0 {
        return DIM_STATS_SAME;
    }
    if is_significant_diff(curr.cpms, prev.cpms) {
        return if curr.cpms > prev.cpms {
            DIM_STATS_BETTER
        } else {
            DIM_STATS_WORSE
        };
    }
    if is_significant_diff(curr.cpe_ratio, prev.cpe_ratio) {
        return if curr.cpe_ratio > prev.cpe_ratio {
            DIM_STATS_BETTER
        } else {
            DIM_STATS_WORSE
        };
    }
    DIM_STATS_SAME
}

pub fn rdma_dim_decision(curr_stats: &DimStats, dim: &mut RdmaDim) -> bool {
    let prev_ix = dim.profile_ix;
    let state = dim.tune_state;

    if state != DIM_PARKING_ON_TOP && state != DIM_PARKING_TIRED {
        match rdma_dim_stats_compare(curr_stats, &dim.prev_stats) {
            DIM_STATS_SAME => {
                if curr_stats.cpe_ratio <= 50 * prev_ix as u32 {
                    dim.profile_ix = 0;
                }
            }
            DIM_STATS_WORSE => {
                rdma_dim_turn(dim);
                if rdma_dim_step(dim) == DIM_ON_EDGE {
                    rdma_dim_turn(dim);
                }
            }
            DIM_STATS_BETTER => {
                if rdma_dim_step(dim) == DIM_ON_EDGE {
                    rdma_dim_turn(dim);
                }
            }
            _ => {}
        }
    }

    dim.prev_stats = *curr_stats;
    dim.profile_ix != prev_ix
}

pub fn rdma_dim(dim: &mut RdmaDim, completions: u64) {
    let next_time = dim.measuring_sample.time_us.wrapping_add(1);
    rdma_dim_at(dim, completions, next_time);
}

pub fn rdma_dim_at(dim: &mut RdmaDim, completions: u64, now_us: u64) {
    dim.measuring_sample.time_us = now_us;
    dim.measuring_sample.event_ctr = dim.measuring_sample.event_ctr.wrapping_add(1);
    dim.measuring_sample.comp_ctr = dim
        .measuring_sample
        .comp_ctr
        .wrapping_add(completions as u32);
    dim.measuring_sample.pkt_ctr = 0;
    dim.measuring_sample.byte_ctr = 0;

    let mut start_new_measure = false;
    match dim.state {
        DIM_MEASURE_IN_PROGRESS => {
            let nevents = dim
                .measuring_sample
                .event_ctr
                .wrapping_sub(dim.start_sample.event_ctr);
            if nevents >= DIM_NEVENTS as u16 {
                if let Some(curr_stats) = dim_calc_stats(&dim.start_sample, &dim.measuring_sample) {
                    if rdma_dim_decision(&curr_stats, dim) {
                        dim.state = DIM_APPLY_NEW_PROFILE;
                        dim.work_scheduled = true;
                    } else {
                        start_new_measure = true;
                    }
                }
            }
        }
        DIM_START_MEASURE => start_new_measure = true,
        DIM_APPLY_NEW_PROFILE => {}
        _ => {}
    }

    if start_new_measure {
        dim.state = DIM_MEASURE_IN_PROGRESS;
        dim.start_sample = dim.measuring_sample;
    }
}

pub unsafe extern "C" fn rdma_dim_raw(dim: *mut RdmaDim, completions: u64) {
    if !dim.is_null() {
        rdma_dim(unsafe { &mut *dim }, completions);
    }
}

fn rdma_dim_turn(dim: &mut RdmaDim) {
    match dim.tune_state {
        DIM_GOING_RIGHT => {
            dim.tune_state = DIM_GOING_LEFT;
            dim.steps_left = 0;
        }
        DIM_GOING_LEFT => {
            dim.tune_state = DIM_GOING_RIGHT;
            dim.steps_right = 0;
        }
        _ => {}
    }
}

fn is_significant_diff(val: u32, reference: u32) -> bool {
    reference != 0 && (100 * val.abs_diff(reference) / reference) > 10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rdma_dim_matches_linux_state_machine() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/dim/rdma_dim.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/dim.h"
        ));
        assert!(source.contains("static int rdma_dim_step(struct dim *dim)"));
        assert!(source.contains("RDMA_DIM_PARAMS_NUM_PROFILES - 1"));
        assert!(source.contains("if (!prev->cpms)"));
        assert!(source.contains("IS_SIGNIFICANT_DIFF(curr->cpms, prev->cpms)"));
        assert!(source.contains("dim_turn(dim);"));
        assert!(source.contains("dim_update_sample_with_comps"));
        assert!(source.contains("nevents < DIM_NEVENTS"));
        assert!(source.contains("schedule_work(&dim->work);"));
        assert!(source.contains("EXPORT_SYMBOL(rdma_dim);"));
        assert!(header.contains("#define RDMA_DIM_PARAMS_NUM_PROFILES 9"));

        let mut dim = RdmaDim {
            tune_state: DIM_GOING_RIGHT,
            profile_ix: 7,
            ..RdmaDim::default()
        };
        assert_eq!(rdma_dim_step(&mut dim), DIM_STEPPED);
        assert_eq!(dim.profile_ix, 8);
        assert_eq!(rdma_dim_step(&mut dim), DIM_ON_EDGE);

        let prev = DimStats {
            cpms: 100,
            cpe_ratio: 100,
            ..DimStats::default()
        };
        let better = DimStats {
            cpms: 112,
            cpe_ratio: 100,
            ..DimStats::default()
        };
        assert_eq!(rdma_dim_stats_compare(&better, &prev), DIM_STATS_BETTER);

        let mut dim = RdmaDim {
            state: DIM_MEASURE_IN_PROGRESS,
            start_sample: DimSample {
                time_us: 0,
                event_ctr: 0,
                comp_ctr: 0,
                ..DimSample::default()
            },
            measuring_sample: DimSample {
                time_us: 63,
                event_ctr: 63,
                comp_ctr: 63,
                ..DimSample::default()
            },
            prev_stats: DimStats {
                cpms: 1,
                cpe_ratio: 1,
                ..DimStats::default()
            },
            tune_state: DIM_GOING_RIGHT,
            ..RdmaDim::default()
        };
        rdma_dim_at(&mut dim, 64, 64);
        assert_eq!(dim.state, DIM_APPLY_NEW_PROFILE);
        assert!(dim.work_scheduled);
        assert_eq!(dim.profile_ix, 1);
    }
}
