//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/a20.c
//! test-origin: linux:vendor/linux/arch/x86/boot/a20.c
//! A20 gate enable sequence.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/a20.c
//!
//! Real-mode setup must enable the A20 line before transitioning to
//! protected mode (otherwise the kernel image will alias to itself in
//! the bottom 1 MiB). Linux tries four strategies in a loop, oldest
//! first: BIOS (INT 15h AX=2401h), keyboard controller, "fast gate"
//! (port 0x92), and short-circuits if A20 is already on.
//!
//! Lupos' Linux boot-protocol loader handles A20 before protected-mode entry,
//! but the port preserves Linux's sequence and tunables for ABI parity.

/// Linux `MAX_8042_LOOPS`.
pub const MAX_8042_LOOPS: i32 = 100_000;
/// Linux `MAX_8042_FF` — bail after this many consecutive 0xFF reads
/// (no keyboard controller present).
pub const MAX_8042_FF: i32 = 32;
/// Linux `A20_ENABLE_LOOPS` — total retry budget across all strategies.
pub const A20_ENABLE_LOOPS: i32 = 255;
/// Linux `A20_TEST_ADDR` — int 0x80 vector address; aliases at +1 MiB
/// when A20 is gated.
pub const A20_TEST_ADDR: u32 = 4 * 0x80;
/// Short A20 test loop count.
pub const A20_TEST_SHORT: i32 = 32;
/// Long A20 test loop count (2^21).
pub const A20_TEST_LONG: i32 = 1 << 21;

/// Strategies Linux tries in order. Mirrors a20.c lines 89-120.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum A20Strategy {
    /// Already-enabled fast path.
    Already,
    /// BIOS INT 15h AX=2401h.
    Bios,
    /// Keyboard controller 8042.
    Kbc,
    /// Fast gate via port 0x92.
    Fast,
}

/// Iteration plan for `enable_a20()` — yields strategies in the order
/// Linux tries them.
pub fn strategy_order() -> [A20Strategy; 4] {
    [
        A20Strategy::Already,
        A20Strategy::Bios,
        A20Strategy::Kbc,
        A20Strategy::Fast,
    ]
}

/// Trait seam for the A20 platform-access primitives. Tests substitute
/// a deterministic backend; production wires this to real port I/O.
pub trait A20Platform {
    /// Probe whether A20 is already enabled (Linux `a20_test_short`).
    fn a20_test_short(&mut self) -> bool;
    /// Long A20 test (Linux `a20_test_long`).
    fn a20_test_long(&mut self) -> bool;
    /// BIOS INT 15h AX=2401h.
    fn enable_a20_bios(&mut self);
    /// Drain the 8042 PS/2 controller (Linux `empty_8042`). Returns
    /// `Ok(())` on success, `Err(())` if KBC is unresponsive.
    fn empty_8042(&mut self) -> Result<(), ()>;
    /// Use the keyboard controller to enable A20.
    fn enable_a20_kbc(&mut self);
    /// Use the legacy "fast gate" at port 0x92.
    fn enable_a20_fast(&mut self);
}

/// `enable_a20()` — Linux's full retry sequence. Returns `Ok(())` if any
/// strategy succeeded within `A20_ENABLE_LOOPS` iterations.
pub fn enable_a20<P: A20Platform>(platform: &mut P) -> Result<(), ()> {
    for _ in 0..A20_ENABLE_LOOPS {
        if platform.a20_test_short() {
            return Ok(());
        }
        platform.enable_a20_bios();
        if platform.a20_test_short() {
            return Ok(());
        }
        let kbc_ok = platform.empty_8042().is_ok();
        if platform.a20_test_short() {
            return Ok(());
        }
        if kbc_ok {
            platform.enable_a20_kbc();
            if platform.a20_test_long() {
                return Ok(());
            }
        }
        platform.enable_a20_fast();
        if platform.a20_test_long() {
            return Ok(());
        }
    }
    Err(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Track {
        already: bool,
        bios_works: bool,
        kbc_works: bool,
        fast_works: bool,
        bios_called: u32,
        kbc_called: u32,
        fast_called: u32,
        empty_returns_err: bool,
    }
    impl A20Platform for Track {
        fn a20_test_short(&mut self) -> bool {
            self.already || self.bios_works && self.bios_called > 0
        }
        fn a20_test_long(&mut self) -> bool {
            (self.kbc_works && self.kbc_called > 0) || (self.fast_works && self.fast_called > 0)
        }
        fn enable_a20_bios(&mut self) {
            self.bios_called += 1;
        }
        fn empty_8042(&mut self) -> Result<(), ()> {
            if self.empty_returns_err {
                Err(())
            } else {
                Ok(())
            }
        }
        fn enable_a20_kbc(&mut self) {
            self.kbc_called += 1;
        }
        fn enable_a20_fast(&mut self) {
            self.fast_called += 1;
        }
    }

    fn baseline() -> Track {
        Track {
            already: false,
            bios_works: false,
            kbc_works: false,
            fast_works: false,
            bios_called: 0,
            kbc_called: 0,
            fast_called: 0,
            empty_returns_err: false,
        }
    }

    #[test]
    fn enable_a20_short_circuits_when_already_enabled() {
        let mut t = baseline();
        t.already = true;
        assert_eq!(enable_a20(&mut t), Ok(()));
        // No strategy invoked.
        assert_eq!(t.bios_called, 0);
        assert_eq!(t.kbc_called, 0);
        assert_eq!(t.fast_called, 0);
    }

    #[test]
    fn enable_a20_tries_bios_then_returns_when_bios_works() {
        let mut t = baseline();
        t.bios_works = true;
        assert_eq!(enable_a20(&mut t), Ok(()));
        assert_eq!(t.bios_called, 1);
        assert_eq!(t.kbc_called, 0);
        assert_eq!(t.fast_called, 0);
    }

    #[test]
    fn enable_a20_falls_through_to_fast_when_others_fail() {
        let mut t = baseline();
        t.fast_works = true;
        assert_eq!(enable_a20(&mut t), Ok(()));
        // BIOS + KBC tried once before fast succeeded.
        assert_eq!(t.bios_called, 1);
        assert_eq!(t.kbc_called, 1);
        assert_eq!(t.fast_called, 1);
    }

    #[test]
    fn enable_a20_skips_kbc_when_empty_8042_fails() {
        let mut t = baseline();
        t.empty_returns_err = true;
        t.fast_works = true;
        assert_eq!(enable_a20(&mut t), Ok(()));
        // KBC must not be attempted when empty_8042 fails.
        assert_eq!(t.kbc_called, 0);
        assert_eq!(t.fast_called, 1);
    }

    #[test]
    fn enable_a20_returns_err_after_loop_budget_exhausted() {
        let mut t = baseline();
        // No strategy works → all 255 loops fail.
        assert_eq!(enable_a20(&mut t), Err(()));
    }

    #[test]
    fn constants_match_linux_a20_c() {
        assert_eq!(MAX_8042_LOOPS, 100_000);
        assert_eq!(MAX_8042_FF, 32);
        assert_eq!(A20_ENABLE_LOOPS, 255);
        assert_eq!(A20_TEST_ADDR, 0x200);
        assert_eq!(A20_TEST_SHORT, 32);
        assert_eq!(A20_TEST_LONG, 2_097_152);
    }
}
