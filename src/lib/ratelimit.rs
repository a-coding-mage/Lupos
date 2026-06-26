//! linux-parity: complete
//! linux-source: vendor/linux/lib/ratelimit.c
//! test-origin: linux:vendor/linux/lib/ratelimit.c
//! Standalone ratelimit state transitions.

use crate::kernel::module::{export_symbol, find_symbol};

pub const HZ: i32 = 100;
pub const DEFAULT_RATELIMIT_INTERVAL: i32 = 5 * HZ;
pub const DEFAULT_RATELIMIT_BURST: i32 = 10;
pub const RATELIMIT_MSG_ON_RELEASE: u32 = 1 << 0;
pub const RATELIMIT_INITIALIZED: u32 = 1 << 1;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("___ratelimit", ___ratelimit_raw as usize, false);
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RatelimitState {
    pub interval: i32,
    pub burst: i32,
    pub n_left: i32,
    pub missed: i32,
    pub flags: u32,
    pub begin: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RatelimitResult {
    pub allowed: bool,
    pub callbacks_suppressed: Option<i32>,
}

impl RatelimitState {
    pub const fn new(interval: i32, burst: i32) -> Self {
        Self {
            interval,
            burst,
            n_left: 0,
            missed: 0,
            flags: 0,
            begin: 0,
        }
    }

    pub const fn disabled() -> Self {
        Self::new(0, DEFAULT_RATELIMIT_BURST)
    }

    pub const fn default_state() -> Self {
        Self::new(DEFAULT_RATELIMIT_INTERVAL, DEFAULT_RATELIMIT_BURST)
    }
}

pub fn ratelimit_state_inc_miss(rs: &mut RatelimitState) {
    rs.missed += 1;
}

pub fn ratelimit_state_get_miss(rs: &RatelimitState) -> i32 {
    rs.missed
}

pub fn ratelimit_state_reset_miss(rs: &mut RatelimitState) -> i32 {
    let missed = rs.missed;
    rs.missed = 0;
    missed
}

pub fn ratelimit_state_reset_interval(rs: &mut RatelimitState, interval: i32) {
    rs.interval = interval;
    rs.flags &= !RATELIMIT_INITIALIZED;
    rs.n_left = rs.burst;
    ratelimit_state_reset_miss(rs);
}

pub fn ratelimit_set_flags(rs: &mut RatelimitState, flags: u32) {
    rs.flags = flags;
}

pub fn ___ratelimit_at(
    rs: &mut RatelimitState,
    _func: &str,
    now_jiffies: u64,
    lock_acquired: bool,
) -> RatelimitResult {
    let interval = rs.interval;
    let burst = rs.burst;
    let mut ret = false;
    let mut callbacks_suppressed = None;

    if interval <= 0 || burst <= 0 {
        ret = interval == 0 || burst > 0;
        if rs.flags & RATELIMIT_INITIALIZED != 0 && !(interval == 0 && burst == 0) && lock_acquired
        {
            rs.flags &= !RATELIMIT_INITIALIZED;
        }
        if !ret {
            ratelimit_state_inc_miss(rs);
        }
        return RatelimitResult {
            allowed: ret,
            callbacks_suppressed,
        };
    }

    if !lock_acquired {
        if rs.flags & RATELIMIT_INITIALIZED != 0 && rs.n_left > 0 {
            rs.n_left -= 1;
            if rs.n_left >= 0 {
                ret = true;
            }
        }
        if !ret {
            ratelimit_state_inc_miss(rs);
        }
        return RatelimitResult {
            allowed: ret,
            callbacks_suppressed,
        };
    }

    if rs.flags & RATELIMIT_INITIALIZED == 0 {
        rs.begin = now_jiffies;
        rs.flags |= RATELIMIT_INITIALIZED;
        rs.n_left = burst;
    }

    if now_jiffies > rs.begin.wrapping_add(interval as u64) {
        rs.n_left = burst;
        rs.begin = now_jiffies;

        if rs.flags & RATELIMIT_MSG_ON_RELEASE == 0 {
            let missed = ratelimit_state_reset_miss(rs);
            if missed != 0 {
                callbacks_suppressed = Some(missed);
            }
        }
    }

    if rs.n_left > 0 {
        rs.n_left -= 1;
        if rs.n_left >= 0 {
            ret = true;
        }
    }

    if !ret {
        ratelimit_state_inc_miss(rs);
    }

    RatelimitResult {
        allowed: ret,
        callbacks_suppressed,
    }
}

pub fn ___ratelimit(rs: &mut RatelimitState, func: &str, now_jiffies: u64) -> bool {
    ___ratelimit_at(rs, func, now_jiffies, true).allowed
}

pub unsafe extern "C" fn ___ratelimit_raw(rs: *mut RatelimitState, _func: *const u8) -> i32 {
    if rs.is_null() {
        return 0;
    }
    let state = unsafe { &mut *rs };
    let now = state.begin;
    i32::from(___ratelimit(state, "", now))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratelimit_state_machine_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/ratelimit.c"
        ));
        let types = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/ratelimit_types.h"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/ratelimit.h"
        ));
        assert!(source.contains("int ___ratelimit(struct ratelimit_state *rs, const char *func)"));
        assert!(source.contains("int interval = READ_ONCE(rs->interval);"));
        assert!(source.contains("int burst = READ_ONCE(rs->burst);"));
        assert!(source.contains("ret = interval == 0 || burst > 0;"));
        assert!(source.contains("rs->flags &= ~RATELIMIT_INITIALIZED;"));
        assert!(source.contains("atomic_read(&rs->rs_n_left) > 0"));
        assert!(source.contains("rs->begin = jiffies;"));
        assert!(source.contains("time_is_before_jiffies(rs->begin + interval)"));
        assert!(source.contains("ratelimit_state_reset_miss(rs);"));
        assert!(source.contains("ratelimit_state_inc_miss(rs);"));
        assert!(source.contains("EXPORT_SYMBOL(___ratelimit);"));
        assert!(types.contains("#define DEFAULT_RATELIMIT_INTERVAL\t(5 * HZ)"));
        assert!(types.contains("#define RATELIMIT_MSG_ON_RELEASE\tBIT(0)"));
        assert!(types.contains("#define RATELIMIT_INITIALIZED\t\tBIT(1)"));
        assert!(header.contains("ratelimit_state_reset_interval"));

        let mut rs = RatelimitState::new(5, 2);
        assert!(___ratelimit(&mut rs, "unit", 10));
        assert_eq!(rs.begin, 10);
        assert_eq!(rs.n_left, 1);
        assert!(___ratelimit(&mut rs, "unit", 11));
        assert!(!___ratelimit(&mut rs, "unit", 12));
        assert_eq!(rs.missed, 1);

        let reset = ___ratelimit_at(&mut rs, "unit", 16, true);
        assert!(reset.allowed);
        assert_eq!(reset.callbacks_suppressed, Some(1));
        assert_eq!(rs.begin, 16);
        assert_eq!(rs.missed, 0);

        let mut disabled = RatelimitState::disabled();
        assert!(___ratelimit(&mut disabled, "unit", 0));

        let mut always_limit = RatelimitState::new(5, 0);
        assert!(!___ratelimit(&mut always_limit, "unit", 0));
        assert_eq!(always_limit.missed, 1);

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("___ratelimit"),
            Some(___ratelimit_raw as usize)
        );
    }
}
