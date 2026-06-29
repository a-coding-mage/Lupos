//! linux-parity: complete
//! linux-source: vendor/linux/drivers/input/misc/pcspkr.c
//! test-origin: linux:vendor/linux/drivers/input/misc/pcspkr.c
//! PC Speaker beeper driver.
//!
//! Ports / mirrors:
//! - vendor/linux/drivers/input/misc/pcspkr.c (`pcspkr_event`)
//!
//! Mirrors the `pcspkr_event` programming sequence: counter 2 of the 8254
//! PIT (ports `0x42`/`0x43`) generates the square wave, and bits 0-1 of the
//! keyboard-controller status port `0x61` gate that output onto the speaker.
//! The virtual-console bell drives this through [`kd_mksound`].

#![allow(dead_code)]

use crate::arch::x86::kernel::i8253::{PIT_CH2, PIT_MODE, PIT_TICK_RATE};

/// Keyboard-controller / NMI status port. Bits 0-1 gate the PIT channel-2
/// output onto the speaker. Mirrors the bare `0x61` in `pcspkr_event`.
pub const SPEAKER_PORT: u16 = 0x61;

/// PIT control word for `pcspkr_event`: select counter 2, lobyte/hibyte
/// access, mode 3 (square-wave generator), binary. Mirrors the `0xB6`
/// written to port `0x43`.
pub const PIT_CH2_SQUARE_WAVE: u8 = 0xB6;

/// Speaker gate bits in [`SPEAKER_PORT`]: timer-2 gate enable + speaker data
/// enable. Mirrors the `| 3` / `& 0xFC` masks in `pcspkr_event`.
pub const SPEAKER_GATE_BITS: u8 = 0x03;

/// Lowest tone `pcspkr_event` will play (`value > 20`).
pub const PCSPKR_MIN_HZ: u32 = 20;

/// Highest tone `pcspkr_event` will play (`value < 32767`).
pub const PCSPKR_MAX_HZ: u32 = 32767;

/// Compute the PIT reload count for a tone of `hz`. Mirrors
/// `if (value > 20 && value < 32767) count = PIT_TICK_RATE / value;` from
/// `pcspkr_event`. Returns `0` when the speaker should fall silent.
pub fn pcspkr_count(hz: u32) -> u32 {
    if hz > PCSPKR_MIN_HZ && hz < PCSPKR_MAX_HZ {
        PIT_TICK_RATE / hz
    } else {
        0
    }
}

/// Drive the PC speaker at `hz`, or silence it when `hz` maps to a zero
/// count. Mirrors the port-I/O body of `pcspkr_event`.
///
/// On the host (`cfg(test)`) build this is a no-op: the `out`/`in`
/// instructions would fault outside the kernel. The count computation is
/// still exercised so the math stays under test.
pub fn kd_mksound(hz: u32) {
    let count = pcspkr_count(hz);
    #[cfg(not(test))]
    unsafe {
        use crate::arch::x86::include::asm::io::{inb_p, outb, outb_p};
        if count != 0 {
            // set command for counter 2, 2 byte write
            outb_p(PIT_MODE, PIT_CH2_SQUARE_WAVE);
            // select desired HZ
            outb_p(PIT_CH2, (count & 0xff) as u8);
            outb(PIT_CH2, ((count >> 8) & 0xff) as u8);
            // enable counter 2
            outb_p(SPEAKER_PORT, inb_p(SPEAKER_PORT) | SPEAKER_GATE_BITS);
        } else {
            // disable counter 2
            outb(SPEAKER_PORT, inb_p(SPEAKER_PORT) & !SPEAKER_GATE_BITS);
        }
    }
    #[cfg(test)]
    let _ = count;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_matches_linux_pit_division() {
        // 750 Hz is the virtual-console DEFAULT_BELL_PITCH.
        assert_eq!(pcspkr_count(750), PIT_TICK_RATE / 750);
        assert_eq!(pcspkr_count(1000), PIT_TICK_RATE / 1000);
    }

    #[test]
    fn out_of_range_tones_silence_the_speaker() {
        assert_eq!(pcspkr_count(0), 0);
        assert_eq!(pcspkr_count(PCSPKR_MIN_HZ), 0);
        assert_eq!(pcspkr_count(PCSPKR_MAX_HZ), 0);
        assert_eq!(pcspkr_count(40_000), 0);
    }

    #[test]
    fn gate_mask_is_low_two_bits() {
        assert_eq!(SPEAKER_GATE_BITS, 0x03);
        assert_eq!(!SPEAKER_GATE_BITS, 0xFC);
        assert_eq!(PIT_CH2_SQUARE_WAVE, 0xB6);
    }
}
