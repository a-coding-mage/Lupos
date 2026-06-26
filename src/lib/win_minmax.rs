//! linux-parity: complete
//! linux-source: vendor/linux/lib/win_minmax.c
//! test-origin: linux:vendor/linux/lib/win_minmax.c
//! Windowed min/max tracker.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MinmaxSample {
    pub t: u32,
    pub v: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Minmax {
    pub s: [MinmaxSample; 3],
}

pub const fn minmax_get(m: &Minmax) -> u32 {
    m.s[0].v
}

pub fn minmax_reset(m: &mut Minmax, t: u32, meas: u32) -> u32 {
    let val = MinmaxSample { t, v: meas };
    m.s = [val, val, val];
    m.s[0].v
}

fn minmax_subwin_update(m: &mut Minmax, win: u32, val: MinmaxSample) -> u32 {
    let dt = val.t.wrapping_sub(m.s[0].t);
    if dt > win {
        m.s[0] = m.s[1];
        m.s[1] = m.s[2];
        m.s[2] = val;
        if val.t.wrapping_sub(m.s[0].t) > win {
            m.s[0] = m.s[1];
            m.s[1] = m.s[2];
            m.s[2] = val;
        }
    } else if m.s[1].t == m.s[0].t && dt > win / 4 {
        m.s[1] = val;
        m.s[2] = val;
    } else if m.s[2].t == m.s[1].t && dt > win / 2 {
        m.s[2] = val;
    }
    m.s[0].v
}

pub fn minmax_running_max(m: &mut Minmax, win: u32, t: u32, meas: u32) -> u32 {
    let val = MinmaxSample { t, v: meas };
    if val.v >= m.s[0].v || val.t.wrapping_sub(m.s[2].t) > win {
        return minmax_reset(m, t, meas);
    }
    if val.v >= m.s[1].v {
        m.s[1] = val;
        m.s[2] = val;
    } else if val.v >= m.s[2].v {
        m.s[2] = val;
    }
    minmax_subwin_update(m, win, val)
}

pub fn minmax_running_min(m: &mut Minmax, win: u32, t: u32, meas: u32) -> u32 {
    let val = MinmaxSample { t, v: meas };
    if val.v <= m.s[0].v || val.t.wrapping_sub(m.s[2].t) > win {
        return minmax_reset(m, t, meas);
    }
    if val.v <= m.s[1].v {
        m.s[1] = val;
        m.s[2] = val;
    } else if val.v <= m.s[2].v {
        m.s[2] = val;
    }
    minmax_subwin_update(m, win, val)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn win_minmax_matches_linux_window_algorithm() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/win_minmax.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/win_minmax.h"
        ));
        assert!(source.contains("static u32 minmax_subwin_update(struct minmax *m, u32 win,"));
        assert!(source.contains("u32 dt = val->t - m->s[0].t;"));
        assert!(source.contains("if (unlikely(dt > win))"));
        assert!(source.contains("m->s[0] = m->s[1];"));
        assert!(source.contains("} else if (unlikely(m->s[1].t == m->s[0].t) && dt > win/4)"));
        assert!(source.contains("} else if (unlikely(m->s[2].t == m->s[1].t) && dt > win/2)"));
        assert!(source.contains("if (unlikely(val.v >= m->s[0].v) ||"));
        assert!(source.contains("if (unlikely(val.v <= m->s[0].v) ||"));
        assert!(source.contains("EXPORT_SYMBOL(minmax_running_max);"));
        assert!(source.contains("EXPORT_SYMBOL(minmax_running_min);"));
        assert!(header.contains("struct minmax_sample"));
        assert!(header.contains("m->s[2] = m->s[1] = m->s[0] = val;"));

        let mut max = Minmax::default();
        assert_eq!(minmax_reset(&mut max, 0, 10), 10);
        assert_eq!(minmax_running_max(&mut max, 10, 1, 8), 10);
        assert_eq!(minmax_running_max(&mut max, 10, 2, 12), 12);
        assert_eq!(minmax_get(&max), 12);

        let mut min = Minmax::default();
        assert_eq!(minmax_reset(&mut min, 0, 10), 10);
        assert_eq!(minmax_running_min(&mut min, 10, 1, 12), 10);
        assert_eq!(minmax_running_min(&mut min, 10, 2, 8), 8);
        assert_eq!(minmax_get(&min), 8);
    }
}
