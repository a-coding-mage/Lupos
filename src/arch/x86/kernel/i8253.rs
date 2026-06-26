//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/i8253.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/i8253.c
//! 8253 / PIT clock-event setup.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/i8253.c
//!
//! The 8253 Programmable Interval Timer ticks at 1.193182 MHz (a third of
//! the original 3.579545 MHz colour-burst oscillator). It still acts as
//! the calibration source for the local-APIC timer on systems without a
//! discoverable TSC frequency, so even on modern boxes we keep the
//! programming sequence faithful.
//!
//! Intel SDM Vol. 3 §10.5 mentions the PIT; the canonical reference is
//! the original 8254 datasheet (Intel order # 231164).

#![allow(dead_code)]

extern crate alloc;

// === Register set — mirror vendor/linux/include/linux/i8253.h ===

pub const PIT_MODE: u16 = 0x43;
pub const PIT_CH0: u16 = 0x40;
pub const PIT_CH2: u16 = 0x42;

// === Frequencies — mirror vendor/linux/include/linux/timex.h ===

/// Native PIT input frequency in Hz.
pub const PIT_TICK_RATE: u32 = 1_193_182;

/// Default kernel tick rate used to derive `PIT_LATCH`.
pub const HZ: u32 = 250;

/// Reload-register value for the periodic tick. Mirrors
/// `((PIT_TICK_RATE + HZ/2) / HZ)` from `linux/i8253.h`.
pub const fn pit_latch(hz: u32) -> u32 {
    (PIT_TICK_RATE + hz / 2) / hz
}

// === Mode word bit fields — Intel 8254 datasheet §3.2 ===

pub const PIT_MODE_BINARY: u8 = 0x00;
pub const PIT_MODE_BCD: u8 = 0x01;

pub const PIT_MODE_INT_ON_TERMINAL: u8 = 0x00; // mode 0
pub const PIT_MODE_HW_ONESHOT: u8 = 0x02; // mode 1
pub const PIT_MODE_RATE_GENERATOR: u8 = 0x04; // mode 2
pub const PIT_MODE_SQUARE_WAVE: u8 = 0x06; // mode 3
pub const PIT_MODE_SW_STROBE: u8 = 0x08; // mode 4
pub const PIT_MODE_HW_STROBE: u8 = 0x0A; // mode 5

pub const PIT_ACCESS_LATCH: u8 = 0x00;
pub const PIT_ACCESS_LOBYTE: u8 = 0x10;
pub const PIT_ACCESS_HIBYTE: u8 = 0x20;
pub const PIT_ACCESS_LO_HI: u8 = 0x30;

pub const PIT_SELECT_CH0: u8 = 0x00;
pub const PIT_SELECT_CH1: u8 = 0x40;
pub const PIT_SELECT_CH2: u8 = 0x80;
pub const PIT_SELECT_READBACK: u8 = 0xC0;

/// Compose the mode-control byte written to `PIT_MODE`.
pub const fn mode_word(channel: u8, access: u8, mode: u8, bcd: u8) -> u8 {
    channel | access | mode | bcd
}

/// Port-I/O seam used by `pit_timer_init` and `clockevent_i8253_disable`.
pub trait PitPort {
    fn outb(&self, port: u16, value: u8);
}

/// Trait seam for `apic_needs_pit()` / TSC availability.
pub trait PitCalibrationContext {
    fn has_tsc(&self) -> bool;
    fn apic_needs_pit(&self) -> bool;
}

/// Linux's `use_pit`: PIT is required if there is no TSC, or if the APIC
/// path explicitly asks for it (also covers the "APIC disabled" case).
pub fn use_pit<C: PitCalibrationContext>(ctx: &C) -> bool {
    if !ctx.has_tsc() {
        return true;
    }
    ctx.apic_needs_pit()
}

/// Disable the PIT by programming channel 0 in mode 0 with a zero latch
/// (no further interrupts). Mirrors `clockevent_i8253_disable` for the
/// purposes of the test/host build — the production path lives in
/// `drivers/clocksource/i8253.c` which is out of scope for arch/x86.
pub fn clockevent_i8253_disable<P: PitPort>(pit: &P) {
    pit.outb(
        PIT_MODE,
        mode_word(
            PIT_SELECT_CH0,
            PIT_ACCESS_LO_HI,
            PIT_MODE_INT_ON_TERMINAL,
            PIT_MODE_BINARY,
        ),
    );
    pit.outb(PIT_CH0, 0);
    pit.outb(PIT_CH0, 0);
}

/// Initialise the PIT for periodic interrupts at `HZ`. Mirrors
/// `clockevent_i8253_init(true)` followed by the body of `pit_timer_init`
/// when the PIT is needed.
pub fn pit_timer_init<C: PitCalibrationContext, P: PitPort>(ctx: &C, pit: &P) -> bool {
    if !use_pit(ctx) {
        clockevent_i8253_disable(pit);
        return false;
    }
    let latch = pit_latch(HZ);
    pit.outb(
        PIT_MODE,
        mode_word(
            PIT_SELECT_CH0,
            PIT_ACCESS_LO_HI,
            PIT_MODE_RATE_GENERATOR,
            PIT_MODE_BINARY,
        ),
    );
    pit.outb(PIT_CH0, (latch & 0xFF) as u8);
    pit.outb(PIT_CH0, ((latch >> 8) & 0xFF) as u8);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::RefCell;

    #[derive(Default)]
    struct MockPit {
        writes: RefCell<alloc::vec::Vec<(u16, u8)>>,
    }

    impl PitPort for MockPit {
        fn outb(&self, port: u16, value: u8) {
            self.writes.borrow_mut().push((port, value));
        }
    }

    struct Ctx {
        has_tsc: bool,
        needs_pit: bool,
    }

    impl PitCalibrationContext for Ctx {
        fn has_tsc(&self) -> bool {
            self.has_tsc
        }
        fn apic_needs_pit(&self) -> bool {
            self.needs_pit
        }
    }

    #[test]
    fn pit_tick_rate_matches_linux() {
        assert_eq!(PIT_TICK_RATE, 1_193_182);
    }

    #[test]
    fn pit_latch_for_hz_250_is_4773() {
        assert_eq!(pit_latch(250), 4773);
    }

    #[test]
    fn use_pit_when_no_tsc() {
        let ctx = Ctx {
            has_tsc: false,
            needs_pit: false,
        };
        assert!(use_pit(&ctx));
    }

    #[test]
    fn skip_pit_when_tsc_and_apic_self_sufficient() {
        let ctx = Ctx {
            has_tsc: true,
            needs_pit: false,
        };
        assert!(!use_pit(&ctx));
    }

    #[test]
    fn pit_timer_init_writes_latch_lo_hi_in_order() {
        let ctx = Ctx {
            has_tsc: false,
            needs_pit: false,
        };
        let pit = MockPit::default();
        let started = pit_timer_init(&ctx, &pit);
        assert!(started);
        let writes = pit.writes.borrow();
        assert_eq!(writes[0].0, PIT_MODE);
        assert_eq!(writes[1].0, PIT_CH0);
        assert_eq!(writes[2].0, PIT_CH0);
        // 4773 = 0x12A5 → lo 0xA5, hi 0x12.
        assert_eq!(writes[1].1, 0xA5);
        assert_eq!(writes[2].1, 0x12);
    }

    #[test]
    fn pit_timer_init_disables_when_unneeded() {
        let ctx = Ctx {
            has_tsc: true,
            needs_pit: false,
        };
        let pit = MockPit::default();
        let started = pit_timer_init(&ctx, &pit);
        assert!(!started);
        let writes = pit.writes.borrow();
        // First write is the disable mode word.
        assert_eq!(writes[0].0, PIT_MODE);
        assert_eq!(
            writes[0].1,
            mode_word(
                PIT_SELECT_CH0,
                PIT_ACCESS_LO_HI,
                PIT_MODE_INT_ON_TERMINAL,
                PIT_MODE_BINARY,
            )
        );
    }
}
