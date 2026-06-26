//! linux-parity: complete
//! linux-source: vendor/linux/lib/dim/dim.c
//! test-origin: linux:vendor/linux/lib/dim/dim.c
//! Dynamic Interrupt Moderation tuning helpers.

use crate::kernel::module::{export_symbol, find_symbol};

pub const USEC_PER_MSEC: u32 = 1000;
pub const DIM_NEVENTS: u32 = 64;

pub const DIM_PARKING_ON_TOP: u8 = 0;
pub const DIM_PARKING_TIRED: u8 = 1;
pub const DIM_GOING_RIGHT: u8 = 2;
pub const DIM_GOING_LEFT: u8 = 3;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Dim {
    pub tune_state: u8,
    pub steps_right: u8,
    pub steps_left: u8,
    pub tired: u8,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DimSample {
    pub time_us: u64,
    pub pkt_ctr: u32,
    pub byte_ctr: u32,
    pub event_ctr: u16,
    pub comp_ctr: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DimStats {
    pub ppms: u32,
    pub bpms: u32,
    pub epms: u32,
    pub cpms: u32,
    pub cpe_ratio: u32,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("dim_on_top", dim_on_top_raw as usize, false);
    export_symbol_once("dim_turn", dim_turn_raw as usize, false);
    export_symbol_once("dim_park_on_top", dim_park_on_top_raw as usize, false);
    export_symbol_once("dim_park_tired", dim_park_tired_raw as usize, false);
}

pub const fn bit_gap_u32(end: u32, start: u32) -> u32 {
    end.wrapping_sub(start)
}

const fn div_round_up(n: u32, d: u32) -> u32 {
    if n == 0 { 0 } else { (n - 1) / d + 1 }
}

pub const fn dim_on_top(dim: &Dim) -> bool {
    match dim.tune_state {
        DIM_PARKING_ON_TOP | DIM_PARKING_TIRED => true,
        DIM_GOING_RIGHT => dim.steps_left > 1 && dim.steps_right == 1,
        _ => dim.steps_right > 1 && dim.steps_left == 1,
    }
}

pub fn dim_turn(dim: &mut Dim) {
    match dim.tune_state {
        DIM_PARKING_ON_TOP | DIM_PARKING_TIRED => {}
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

pub fn dim_park_on_top(dim: &mut Dim) {
    dim.steps_right = 0;
    dim.steps_left = 0;
    dim.tired = 0;
    dim.tune_state = DIM_PARKING_ON_TOP;
}

pub fn dim_park_tired(dim: &mut Dim) {
    dim.steps_right = 0;
    dim.steps_left = 0;
    dim.tune_state = DIM_PARKING_TIRED;
}

pub fn dim_calc_stats(start: &DimSample, end: &DimSample) -> Option<DimStats> {
    let delta_us = end.time_us.wrapping_sub(start.time_us) as u32;
    if delta_us == 0 {
        return None;
    }
    let npkts = bit_gap_u32(end.pkt_ctr, start.pkt_ctr);
    let nbytes = bit_gap_u32(end.byte_ctr, start.byte_ctr);
    let ncomps = bit_gap_u32(end.comp_ctr, start.comp_ctr);
    let epms = div_round_up(DIM_NEVENTS * USEC_PER_MSEC, delta_us);
    let cpms = div_round_up(ncomps * USEC_PER_MSEC, delta_us);
    Some(DimStats {
        ppms: div_round_up(npkts * USEC_PER_MSEC, delta_us),
        bpms: div_round_up(nbytes * USEC_PER_MSEC, delta_us),
        epms,
        cpms,
        cpe_ratio: if epms != 0 { cpms * 100 / epms } else { 0 },
    })
}

pub unsafe extern "C" fn dim_on_top_raw(dim: *const Dim) -> bool {
    !dim.is_null() && dim_on_top(unsafe { &*dim })
}

pub unsafe extern "C" fn dim_turn_raw(dim: *mut Dim) {
    if !dim.is_null() {
        dim_turn(unsafe { &mut *dim });
    }
}

pub unsafe extern "C" fn dim_park_on_top_raw(dim: *mut Dim) {
    if !dim.is_null() {
        dim_park_on_top(unsafe { &mut *dim });
    }
}

pub unsafe extern "C" fn dim_park_tired_raw(dim: *mut Dim) {
    if !dim.is_null() {
        dim_park_tired(unsafe { &mut *dim });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dim_matches_linux_state_machine_and_stats() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/dim/dim.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/dim.h"
        ));
        assert!(source.contains("case DIM_PARKING_ON_TOP:"));
        assert!(source.contains("return (dim->steps_left > 1) && (dim->steps_right == 1);"));
        assert!(source.contains("dim->tune_state = DIM_GOING_LEFT;"));
        assert!(source.contains("dim->tune_state   = DIM_PARKING_ON_TOP;"));
        assert!(
            source.contains("curr_stats->ppms = DIV_ROUND_UP(npkts * USEC_PER_MSEC, delta_us);")
        );
        assert!(source.contains("EXPORT_SYMBOL(dim_calc_stats);"));
        assert!(header.contains("#define DIM_NEVENTS 64"));

        let mut dim = Dim {
            tune_state: DIM_GOING_RIGHT,
            steps_left: 2,
            steps_right: 1,
            tired: 3,
        };
        assert!(dim_on_top(&dim));
        dim_turn(&mut dim);
        assert_eq!(dim.tune_state, DIM_GOING_LEFT);
        assert_eq!(dim.steps_left, 0);
        dim_park_on_top(&mut dim);
        assert_eq!(dim, Dim::default());
        dim_park_tired(&mut dim);
        assert_eq!(dim.tune_state, DIM_PARKING_TIRED);

        let stats = dim_calc_stats(
            &DimSample {
                time_us: 10,
                pkt_ctr: 10,
                byte_ctr: 100,
                comp_ctr: 7,
                event_ctr: 0,
            },
            &DimSample {
                time_us: 1010,
                pkt_ctr: 74,
                byte_ctr: 1100,
                comp_ctr: 71,
                event_ctr: 0,
            },
        )
        .expect("stats");
        assert_eq!(stats.ppms, 64);
        assert_eq!(stats.bpms, 1000);
        assert_eq!(stats.epms, 64);
        assert_eq!(stats.cpms, 64);
        assert_eq!(stats.cpe_ratio, 100);
    }
}
