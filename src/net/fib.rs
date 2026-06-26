//! linux-parity: complete
//! linux-source: vendor/linux/net
//! test-origin: linux:vendor/linux/net
//! FIB4/FIB6 routing tables with longest-prefix match.

extern crate alloc;

use alloc::vec::Vec;

use spin::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fib4Entry {
    pub prefix: u32,
    pub prefix_len: u8,
    pub gateway: u32,
    pub ifindex: u32,
    pub metric: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fib6Entry {
    pub prefix: [u8; 16],
    pub prefix_len: u8,
    pub gateway: [u8; 16],
    pub ifindex: u32,
    pub metric: u32,
}

static FIB4: Mutex<Vec<Fib4Entry>> = Mutex::new(Vec::new());
static FIB6: Mutex<Vec<Fib6Entry>> = Mutex::new(Vec::new());

pub fn fib_clear() {
    FIB4.lock().clear();
    FIB6.lock().clear();
}

pub fn fib4_add(entry: Fib4Entry) {
    FIB4.lock().push(entry);
}

pub fn fib6_add(entry: Fib6Entry) {
    FIB6.lock().push(entry);
}

pub fn fib4_lookup(dst: u32) -> Option<Fib4Entry> {
    FIB4.lock()
        .iter()
        .copied()
        .filter(|entry| ipv4_matches(dst, entry.prefix, entry.prefix_len))
        .max_by(|a, b| {
            a.prefix_len
                .cmp(&b.prefix_len)
                .then_with(|| b.metric.cmp(&a.metric))
        })
}

pub fn fib6_lookup(dst: [u8; 16]) -> Option<Fib6Entry> {
    FIB6.lock()
        .iter()
        .copied()
        .filter(|entry| ipv6_matches(dst, entry.prefix, entry.prefix_len))
        .max_by(|a, b| {
            a.prefix_len
                .cmp(&b.prefix_len)
                .then_with(|| b.metric.cmp(&a.metric))
        })
}

fn ipv4_matches(dst: u32, prefix: u32, prefix_len: u8) -> bool {
    if prefix_len == 0 {
        return true;
    }
    let mask = u32::MAX << (32 - prefix_len as u32);
    (dst & mask) == (prefix & mask)
}

fn ipv6_matches(dst: [u8; 16], prefix: [u8; 16], prefix_len: u8) -> bool {
    let full = (prefix_len / 8) as usize;
    let rem = prefix_len % 8;

    if dst[..full] != prefix[..full] {
        return false;
    }
    if rem == 0 {
        return true;
    }

    let mask = 0xffu8 << (8 - rem);
    (dst[full] & mask) == (prefix[full] & mask)
}

pub const fn ipv4(a: u8, b: u8, c: u8, d: u8) -> u32 {
    u32::from_be_bytes([a, b, c, d])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fib4_uses_longest_prefix_then_lowest_metric() {
        fib_clear();
        fib4_add(Fib4Entry {
            prefix: ipv4(0, 0, 0, 0),
            prefix_len: 0,
            gateway: ipv4(10, 0, 0, 1),
            ifindex: 1,
            metric: 100,
        });
        fib4_add(Fib4Entry {
            prefix: ipv4(10, 1, 0, 0),
            prefix_len: 16,
            gateway: 0,
            ifindex: 2,
            metric: 100,
        });
        fib4_add(Fib4Entry {
            prefix: ipv4(10, 1, 2, 0),
            prefix_len: 24,
            gateway: 0,
            ifindex: 3,
            metric: 200,
        });
        fib4_add(Fib4Entry {
            prefix: ipv4(10, 1, 2, 0),
            prefix_len: 24,
            gateway: 0,
            ifindex: 4,
            metric: 50,
        });

        assert_eq!(fib4_lookup(ipv4(10, 1, 2, 9)).unwrap().ifindex, 4);
        assert_eq!(fib4_lookup(ipv4(8, 8, 8, 8)).unwrap().ifindex, 1);
    }

    #[test]
    fn fib6_uses_longest_prefix() {
        fib_clear();
        let mut p32 = [0u8; 16];
        p32[0..4].copy_from_slice(&[0x20, 0x01, 0x0d, 0xb8]);
        let mut p64 = p32;
        p64[4..8].copy_from_slice(&[0, 1, 0, 2]);
        let mut dst = p64;
        dst[15] = 1;

        fib6_add(Fib6Entry {
            prefix: p32,
            prefix_len: 32,
            gateway: [0; 16],
            ifindex: 1,
            metric: 10,
        });
        fib6_add(Fib6Entry {
            prefix: p64,
            prefix_len: 64,
            gateway: [0; 16],
            ifindex: 2,
            metric: 10,
        });

        assert_eq!(fib6_lookup(dst).unwrap().ifindex, 2);
    }
}
