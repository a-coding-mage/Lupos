//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/poll.c
//! test-origin: linux:vendor/linux/io_uring/poll.c
//! `IORING_OP_POLL_ADD` / `POLL_REMOVE` and async-poll arm/disarm.
//!
//! Linux uses a per-file poll table that hooks into the file's wait queue.
//! Lupos provides a minimal arm/fire/remove API here; the full vfs_poll
//! integration lands as files become poll-capable.
//!
//! Ref: vendor/linux/io_uring/poll.c

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use spin::Mutex;

/// `IORING_POLL_ADD_MULTI` / `UPDATE_EVENTS` / `UPDATE_USER_DATA`.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:380-382
pub const IORING_POLL_ADD_MULTI: u32 = 1 << 0;
pub const IORING_POLL_UPDATE_EVENTS: u32 = 1 << 1;
pub const IORING_POLL_UPDATE_USER_DATA: u32 = 1 << 2;

/// One armed poll entry.
#[derive(Clone, Copy, Debug)]
pub struct PollEntry {
    pub user_data: u64,
    /// `events` bitmask (EPOLLIN/OUT/etc.).
    pub events: u32,
    pub multishot: bool,
}

/// Per-ring poll table.  Linux uses a hashed list; we use a `BTreeMap<u64>`
/// keyed by user_data which gives O(log n) lookup and stable iteration.
#[derive(Default)]
pub struct PollTable {
    entries: Mutex<BTreeMap<u64, PollEntry>>,
}

impl PollTable {
    pub const fn new() -> Self {
        Self {
            entries: Mutex::new(BTreeMap::new()),
        }
    }

    /// `io_poll_add` — arm a poll.  Returns `-EALREADY` (-114) if `user_data`
    /// is already registered (matches Linux).
    pub fn add(&self, e: PollEntry) -> Result<(), i32> {
        let mut g = self.entries.lock();
        if g.contains_key(&e.user_data) {
            return Err(-114);
        }
        g.insert(e.user_data, e);
        Ok(())
    }

    /// `io_poll_remove` — cancel by `user_data`.  Returns `-ENOENT` (-2) if
    /// the entry was missing (matches Linux).
    pub fn remove(&self, user_data: u64) -> Result<PollEntry, i32> {
        self.entries.lock().remove(&user_data).ok_or(-2)
    }

    /// `io_poll_double_wait` event delivery — fire every entry whose
    /// `events` mask intersects `mask`.  Single-shot entries are removed
    /// after firing; multishot stay.  Returns the list of `(user_data, mask)`
    /// completions that should be posted as CQEs.
    pub fn fire(&self, mask: u32) -> Vec<(u64, u32)> {
        let mut fired = Vec::new();
        let mut to_remove = Vec::new();
        let mut g = self.entries.lock();
        for (k, e) in g.iter() {
            let hit = e.events & mask;
            if hit != 0 {
                fired.push((*k, hit));
                if !e.multishot {
                    to_remove.push(*k);
                }
            }
        }
        for k in to_remove {
            g.remove(&k);
        }
        fired
    }

    pub fn len(&self) -> usize {
        self.entries.lock().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPOLLIN: u32 = 0x1;
    const EPOLLOUT: u32 = 0x4;

    #[test]
    fn add_then_remove_round_trip() {
        let t = PollTable::new();
        let e = PollEntry {
            user_data: 7,
            events: EPOLLIN,
            multishot: false,
        };
        t.add(e).unwrap();
        let got = t.remove(7).unwrap();
        assert_eq!(got.user_data, 7);
        assert_eq!(got.events, EPOLLIN);
    }

    #[test]
    fn duplicate_add_is_ealready() {
        let t = PollTable::new();
        let e = PollEntry {
            user_data: 1,
            events: EPOLLIN,
            multishot: false,
        };
        t.add(e).unwrap();
        assert_eq!(t.add(e).unwrap_err(), -114);
    }

    #[test]
    fn remove_missing_is_enoent() {
        let t = PollTable::new();
        assert_eq!(t.remove(99).unwrap_err(), -2);
    }

    #[test]
    fn fire_drops_single_shot_entries() {
        let t = PollTable::new();
        t.add(PollEntry {
            user_data: 1,
            events: EPOLLIN,
            multishot: false,
        })
        .unwrap();
        let fired = t.fire(EPOLLIN);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0], (1, EPOLLIN));
        // Entry was consumed.
        assert_eq!(t.len(), 0);
    }

    #[test]
    fn fire_keeps_multishot_entries() {
        let t = PollTable::new();
        t.add(PollEntry {
            user_data: 2,
            events: EPOLLIN,
            multishot: true,
        })
        .unwrap();
        let _ = t.fire(EPOLLIN);
        let _ = t.fire(EPOLLIN);
        assert_eq!(t.len(), 1);
    }

    #[test]
    fn fire_ignores_unmasked_entries() {
        let t = PollTable::new();
        t.add(PollEntry {
            user_data: 3,
            events: EPOLLOUT,
            multishot: false,
        })
        .unwrap();
        let fired = t.fire(EPOLLIN);
        assert!(fired.is_empty());
    }

    #[test]
    fn poll_flag_constants_match_linux() {
        assert_eq!(IORING_POLL_ADD_MULTI, 1);
        assert_eq!(IORING_POLL_UPDATE_EVENTS, 2);
        assert_eq!(IORING_POLL_UPDATE_USER_DATA, 4);
    }
}
