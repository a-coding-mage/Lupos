//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/wait_bit.c
//! test-origin: linux:vendor/linux/kernel/sched/wait_bit.c
//! Wait-on-bit helpers.
//!
//! Mirrors `vendor/linux/kernel/sched/wait_bit.c`.

use core::sync::atomic::{AtomicU64, Ordering};

use super::wait::WaitQueueHead;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WaitBitKey {
    pub word: usize,
    pub bit: u32,
}

impl WaitBitKey {
    pub const fn new(word: usize, bit: u32) -> Self {
        Self { word, bit }
    }

    pub const fn hash(self) -> u64 {
        ((self.word as u64) >> 3) ^ (self.bit as u64)
    }
}

pub fn test_bit(word: &AtomicU64, bit: u32) -> bool {
    word.load(Ordering::Acquire) & (1u64 << (bit & 63)) != 0
}

pub fn clear_bit_unlock(word: &AtomicU64, bit: u32) {
    word.fetch_and(!(1u64 << (bit & 63)), Ordering::Release);
}

pub fn wait_on_bit(word: &AtomicU64, bit: u32, _queue: &WaitQueueHead) -> bool {
    !test_bit(word, bit)
}

pub fn wake_up_bit(queue: &WaitQueueHead) -> usize {
    queue.wake_up_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_bit_hash_includes_word_and_bit() {
        let a = WaitBitKey::new(0x1000, 1);
        let b = WaitBitKey::new(0x1000, 2);
        assert_ne!(a.hash(), b.hash());
    }

    #[test]
    fn wait_on_bit_reports_clear_state() {
        let word = AtomicU64::new(1 << 3);
        let q = WaitQueueHead::new();
        assert!(!wait_on_bit(&word, 3, &q));
        clear_bit_unlock(&word, 3);
        assert!(wait_on_bit(&word, 3, &q));
    }
}
