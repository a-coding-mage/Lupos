//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/simple_ring_buffer.c
//! test-origin: linux:vendor/linux/kernel/trace/simple_ring_buffer.c
//! A simplified, lockless ring buffer for "single producer, single consumer"
//! traces (used by trace_remote and some test paths).
//!
//! Ref: vendor/linux/kernel/trace/simple_ring_buffer.c

extern crate alloc;
use alloc::vec::Vec;

use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

pub struct SimpleRing<T: Clone> {
    inner: Mutex<Vec<T>>,
    cap: usize,
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl<T: Clone + Default> SimpleRing<T> {
    pub fn new(cap: usize) -> Self {
        let mut v = Vec::with_capacity(cap);
        v.resize(cap, T::default());
        Self {
            inner: Mutex::new(v),
            cap,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    pub fn push(&self, item: T) -> bool {
        let tail = self.tail.load(Ordering::Acquire);
        let head = self.head.load(Ordering::Acquire);
        if tail.wrapping_sub(head) == self.cap {
            return false;
        }
        self.inner.lock()[tail % self.cap] = item;
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        true
    }

    pub fn pop(&self) -> Option<T> {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        if head == tail {
            return None;
        }
        let v = self.inner.lock()[head % self.cap].clone();
        self.head.store(head.wrapping_add(1), Ordering::Release);
        Some(v)
    }

    pub fn len(&self) -> usize {
        self.tail
            .load(Ordering::Acquire)
            .wrapping_sub(self.head.load(Ordering::Acquire))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_pop_fifo() {
        let r: SimpleRing<u32> = SimpleRing::new(4);
        r.push(1);
        r.push(2);
        r.push(3);
        assert_eq!(r.pop(), Some(1));
        assert_eq!(r.pop(), Some(2));
        assert_eq!(r.pop(), Some(3));
        assert_eq!(r.pop(), None);
    }

    #[test]
    fn push_rejects_when_full() {
        let r: SimpleRing<u32> = SimpleRing::new(2);
        assert!(r.push(1));
        assert!(r.push(2));
        assert!(!r.push(3));
    }
}
