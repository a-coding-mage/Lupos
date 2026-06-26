//! linux-parity: complete
//! linux-source: vendor/linux/lib/math/cordic.c
//! test-origin: linux:vendor/linux/lib/math/cordic.c
//! CORDIC sine/cosine coordinate helper.

use crate::kernel::module::{export_symbol, find_symbol};

pub const CORDIC_ANGLE_GEN: i32 = 39797;
pub const CORDIC_PRECISION_SHIFT: i32 = 16;
pub const CORDIC_NUM_ITER: usize = (CORDIC_PRECISION_SHIFT as usize) + 2;

pub const ARCTAN_TABLE: [i32; CORDIC_NUM_ITER] = [
    2949120, 1740967, 919879, 466945, 234379, 117304, 58666, 29335, 14668, 7334, 3667, 1833, 917,
    458, 229, 115, 57, 29,
];

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CordicIq {
    pub i: i32,
    pub q: i32,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("cordic_calc_iq", cordic_calc_iq as usize, false);
}

pub const fn cordic_fixed(value: i32) -> i32 {
    value << CORDIC_PRECISION_SHIFT
}

pub const fn cordic_float(value: i32) -> i32 {
    if value >= 0 {
        (((value >> (CORDIC_PRECISION_SHIFT - 1)) + 1) >> 1)
    } else {
        -((((-value) >> (CORDIC_PRECISION_SHIFT - 1)) + 1) >> 1)
    }
}

pub extern "C" fn cordic_calc_iq(mut theta: i32) -> CordicIq {
    let mut coord = CordicIq {
        i: CORDIC_ANGLE_GEN,
        q: 0,
    };
    let mut angle = 0i32;
    let mut signx = 1i32;

    theta = cordic_fixed(theta);
    let signtheta = if theta < 0 { -1 } else { 1 };
    theta = ((theta + cordic_fixed(180) * signtheta) % cordic_fixed(360))
        - cordic_fixed(180) * signtheta;

    if cordic_float(theta) > 90 {
        theta -= cordic_fixed(180);
        signx = -1;
    } else if cordic_float(theta) < -90 {
        theta += cordic_fixed(180);
        signx = -1;
    }

    let mut iter = 0usize;
    while iter < CORDIC_NUM_ITER {
        let valtmp;
        if theta > angle {
            valtmp = coord.i - (coord.q >> iter);
            coord.q += coord.i >> iter;
            angle += ARCTAN_TABLE[iter];
        } else {
            valtmp = coord.i + (coord.q >> iter);
            coord.q -= coord.i >> iter;
            angle -= ARCTAN_TABLE[iter];
        }
        coord.i = valtmp;
        iter += 1;
    }

    coord.i *= signx;
    coord.q *= signx;
    coord
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cordic_matches_linux_iteration_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/math/cordic.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/cordic.h"
        ));
        assert!(source.contains("static const s32 arctan_table[]"));
        assert!(source.contains("coord.i = CORDIC_ANGLE_GEN;"));
        assert!(source.contains("theta = CORDIC_FIXED(theta);"));
        assert!(source.contains("if (CORDIC_FLOAT(theta) > 90)"));
        assert!(source.contains("for (iter = 0; iter < CORDIC_NUM_ITER; iter++)"));
        assert!(source.contains("coord.i *= signx;"));
        assert!(source.contains("EXPORT_SYMBOL(cordic_calc_iq);"));
        assert!(header.contains("#define CORDIC_ANGLE_GEN\t39797"));
        assert!(header.contains("#define CORDIC_NUM_ITER\t(CORDIC_PRECISION_SHIFT + 2)"));

        assert_eq!(ARCTAN_TABLE.len(), 18);
        let zero = cordic_calc_iq(0);
        assert!((zero.i - 65536).abs() <= 8);
        assert!(zero.q.abs() <= 2);
        let ninety = cordic_calc_iq(90);
        assert!(ninety.i.abs() <= 4);
        assert!((ninety.q - 65536).abs() <= 4);
        let one_eighty = cordic_calc_iq(180);
        assert!((one_eighty.i + 65536).abs() <= 8);
    }
}
