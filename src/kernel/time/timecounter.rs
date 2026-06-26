//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/timecounter.c
//! test-origin: linux:vendor/linux/kernel/time/timecounter.c
//! Timecounter coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/timecounter.c`.

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Timecounter {
    pub cycle_last: u64,
    pub nsec: u64,
    pub mask: u64,
    pub mult: u32,
    pub shift: u32,
}

impl Timecounter {
    pub const fn new(cycle_last: u64, mask: u64, mult: u32, shift: u32) -> Self {
        Self {
            cycle_last,
            nsec: 0,
            mask,
            mult,
            shift,
        }
    }

    pub fn read(&mut self, cycle_now: u64) -> u64 {
        let delta = cycle_now.wrapping_sub(self.cycle_last) & self.mask;
        self.cycle_last = cycle_now;
        self.nsec = self
            .nsec
            .saturating_add(delta.saturating_mul(self.mult as u64) >> self.shift);
        self.nsec
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timecounter_accumulates_scaled_delta() {
        let mut tc = Timecounter::new(10, u64::MAX, 2, 0);
        assert_eq!(tc.read(15), 10);
        assert_eq!(tc.read(20), 20);
    }
}
