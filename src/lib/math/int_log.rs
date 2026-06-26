//! linux-parity: complete
//! linux-source: vendor/linux/lib/math/int_log.c
//! test-origin: linux:vendor/linux/lib/math/int_log.c
//! Fixed-point integer logarithm helpers.

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("intlog2", intlog2 as usize, false);
    export_symbol_once("intlog10", intlog10 as usize, false);
}

pub const LOG_TABLE: [u16; 256] = [
    0x0000, 0x0171, 0x02e0, 0x044e, 0x05ba, 0x0725, 0x088e, 0x09f7, 0x0b5d, 0x0cc3, 0x0e27, 0x0f8a,
    0x10eb, 0x124b, 0x13aa, 0x1508, 0x1664, 0x17bf, 0x1919, 0x1a71, 0x1bc8, 0x1d1e, 0x1e73, 0x1fc6,
    0x2119, 0x226a, 0x23ba, 0x2508, 0x2656, 0x27a2, 0x28ed, 0x2a37, 0x2b80, 0x2cc8, 0x2e0f, 0x2f54,
    0x3098, 0x31dc, 0x331e, 0x345f, 0x359f, 0x36de, 0x381b, 0x3958, 0x3a94, 0x3bce, 0x3d08, 0x3e41,
    0x3f78, 0x40af, 0x41e4, 0x4319, 0x444c, 0x457f, 0x46b0, 0x47e1, 0x4910, 0x4a3f, 0x4b6c, 0x4c99,
    0x4dc5, 0x4eef, 0x5019, 0x5142, 0x526a, 0x5391, 0x54b7, 0x55dc, 0x5700, 0x5824, 0x5946, 0x5a68,
    0x5b89, 0x5ca8, 0x5dc7, 0x5ee5, 0x6003, 0x611f, 0x623a, 0x6355, 0x646f, 0x6588, 0x66a0, 0x67b7,
    0x68ce, 0x69e4, 0x6af8, 0x6c0c, 0x6d20, 0x6e32, 0x6f44, 0x7055, 0x7165, 0x7274, 0x7383, 0x7490,
    0x759d, 0x76aa, 0x77b5, 0x78c0, 0x79ca, 0x7ad3, 0x7bdb, 0x7ce3, 0x7dea, 0x7ef0, 0x7ff6, 0x80fb,
    0x81ff, 0x8302, 0x8405, 0x8507, 0x8608, 0x8709, 0x8809, 0x8908, 0x8a06, 0x8b04, 0x8c01, 0x8cfe,
    0x8dfa, 0x8ef5, 0x8fef, 0x90e9, 0x91e2, 0x92db, 0x93d2, 0x94ca, 0x95c0, 0x96b6, 0x97ab, 0x98a0,
    0x9994, 0x9a87, 0x9b7a, 0x9c6c, 0x9d5e, 0x9e4f, 0x9f3f, 0xa02e, 0xa11e, 0xa20c, 0xa2fa, 0xa3e7,
    0xa4d4, 0xa5c0, 0xa6ab, 0xa796, 0xa881, 0xa96a, 0xaa53, 0xab3c, 0xac24, 0xad0c, 0xadf2, 0xaed9,
    0xafbe, 0xb0a4, 0xb188, 0xb26c, 0xb350, 0xb433, 0xb515, 0xb5f7, 0xb6d9, 0xb7ba, 0xb89a, 0xb97a,
    0xba59, 0xbb38, 0xbc16, 0xbcf4, 0xbdd1, 0xbead, 0xbf8a, 0xc065, 0xc140, 0xc21b, 0xc2f5, 0xc3cf,
    0xc4a8, 0xc580, 0xc658, 0xc730, 0xc807, 0xc8de, 0xc9b4, 0xca8a, 0xcb5f, 0xcc34, 0xcd08, 0xcddc,
    0xceaf, 0xcf82, 0xd054, 0xd126, 0xd1f7, 0xd2c8, 0xd399, 0xd469, 0xd538, 0xd607, 0xd6d6, 0xd7a4,
    0xd872, 0xd93f, 0xda0c, 0xdad9, 0xdba5, 0xdc70, 0xdd3b, 0xde06, 0xded0, 0xdf9a, 0xe063, 0xe12c,
    0xe1f5, 0xe2bd, 0xe385, 0xe44c, 0xe513, 0xe5d9, 0xe69f, 0xe765, 0xe82a, 0xe8ef, 0xe9b3, 0xea77,
    0xeb3b, 0xebfe, 0xecc1, 0xed83, 0xee45, 0xef06, 0xefc8, 0xf088, 0xf149, 0xf209, 0xf2c8, 0xf387,
    0xf446, 0xf505, 0xf5c3, 0xf680, 0xf73e, 0xf7fb, 0xf8b7, 0xf973, 0xfa2f, 0xfaea, 0xfba5, 0xfc60,
    0xfd1a, 0xfdd4, 0xfe8e, 0xff47,
];

pub const fn intlog2(value: u32) -> u32 {
    if value == 0 {
        return 0;
    }

    let msb = 31 - value.leading_zeros();
    let significand = value << (31 - msb);
    let logentry = ((significand >> 23) as usize) % LOG_TABLE.len();
    let next = LOG_TABLE[(logentry + 1) % LOG_TABLE.len()];
    let current = LOG_TABLE[logentry];
    let delta = next.wrapping_sub(current) as u32;
    let interpolation = ((significand & 0x007f_ffff) * delta) >> 15;

    (msb << 24) + ((current as u32) << 8) + interpolation
}

pub const fn intlog10(value: u32) -> u32 {
    if value == 0 {
        return 0;
    }

    ((intlog2(value) as u64 * 646_456_993) >> 31) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_log_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/math/int_log.c"
        ));
        assert!(source.contains("static const unsigned short logtable[256]"));
        assert!(source.contains("unsigned int intlog2(u32 value)"));
        assert!(source.contains("if (unlikely(value == 0))"));
        assert!(source.contains("msb = fls(value) - 1;"));
        assert!(source.contains("significand = value << (31 - msb);"));
        assert!(source.contains("logentry = (significand >> 23) % ARRAY_SIZE(logtable);"));
        assert!(source.contains("((logtable[(logentry + 1) % ARRAY_SIZE(logtable)] -"));
        assert!(
            source.contains("return ((msb << 24) + (logtable[logentry] << 8) + interpolation);")
        );
        assert!(source.contains("return (log * 646456993) >> 31;"));
        assert!(source.contains("EXPORT_SYMBOL(intlog2);"));
        assert!(source.contains("EXPORT_SYMBOL(intlog10);"));
        assert_eq!(LOG_TABLE.len(), 256);
        assert_eq!(LOG_TABLE[0], 0);
        assert_eq!(LOG_TABLE[255], 0xff47);
    }

    #[test]
    fn int_log_values_follow_linux_fixed_point_contract() {
        assert_eq!(intlog2(0), 0);
        assert_eq!(intlog10(0), 0);
        assert_eq!(intlog2(1), 0);
        assert_eq!(intlog2(2), 1 << 24);
        assert_eq!(intlog2(4), 2 << 24);
        assert_eq!(intlog2(8), 3 << 24);
        assert_eq!(intlog10(1), 0);
        assert_eq!(intlog10(10), 16_777_225);
        assert_eq!(intlog10(100), 33_554_450);
        assert_eq!(intlog10(1000), 50_331_675);
    }

    #[test]
    fn int_log_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("intlog2"),
            Some(intlog2 as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("intlog10"),
            Some(intlog10 as usize)
        );
    }
}
