//! linux-parity: partial
//! linux-source: vendor/linux/block/badblocks.c
//! test-origin: linux:vendor/linux/block/badblocks.c
//! Linux bad-block extent table.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::{max, min};

use crate::include::uapi::errno::{EINVAL, ENOSPC};

pub const BB_LEN_MASK: u64 = 0x0000_0000_0000_01ff;
pub const BB_OFFSET_MASK: u64 = 0x7fff_ffff_ffff_fe00;
pub const BB_ACK_MASK: u64 = 0x8000_0000_0000_0000;
pub const BB_MAX_LEN: u64 = 512;
pub const PAGE_SIZE: usize = 4096;
pub const MAX_BADBLOCKS: usize = PAGE_SIZE / 8;

pub const fn bb_offset(entry: u64) -> u64 {
    (entry & BB_OFFSET_MASK) >> 9
}

pub const fn bb_len(entry: u64) -> u64 {
    (entry & BB_LEN_MASK) + 1
}

pub const fn bb_ack(entry: u64) -> bool {
    (entry & BB_ACK_MASK) != 0
}

pub const fn bb_end(entry: u64) -> u64 {
    bb_offset(entry) + bb_len(entry)
}

pub const fn bb_make(offset: u64, len: u64, ack: bool) -> u64 {
    (offset << 9) | (len - 1) | ((ack as u64) << 63)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BadBlockEntry {
    pub start: u64,
    pub len: u64,
    pub acknowledged: bool,
}

impl BadBlockEntry {
    pub const fn end(self) -> u64 {
        self.start + self.len
    }

    pub const fn encode(self) -> u64 {
        bb_make(self.start, self.len, self.acknowledged)
    }

    pub const fn decode(entry: u64) -> Self {
        Self {
            start: bb_offset(entry),
            len: bb_len(entry),
            acknowledged: bb_ack(entry),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BadBlockHit {
    pub first_bad: u64,
    pub bad_sectors: u64,
    pub status: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BadBlocks {
    shift: i32,
    entries: Vec<BadBlockEntry>,
    changed: bool,
    unacked_exist: bool,
}

impl BadBlocks {
    pub fn new(enable: bool) -> Self {
        Self {
            shift: if enable { 0 } else { -1 },
            entries: Vec::new(),
            changed: false,
            unacked_exist: false,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.shift >= 0
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    pub fn changed(&self) -> bool {
        self.changed
    }

    pub fn unacked_exist(&self) -> bool {
        self.unacked_exist
    }

    pub fn entries(&self) -> &[BadBlockEntry] {
        &self.entries
    }

    pub fn set(&mut self, start: u64, sectors: u64, acknowledged: bool) -> bool {
        if self.shift < 0 || sectors == 0 {
            return false;
        }
        let Some(end) = start.checked_add(sectors) else {
            return false;
        };

        let mut points = Vec::new();
        points.push(start);
        points.push(end);
        for entry in &self.entries {
            points.push(entry.start);
            points.push(entry.end());
        }
        points.sort_unstable();
        points.dedup();

        let mut out = Vec::new();
        for window in points.windows(2) {
            let seg_start = window[0];
            let seg_end = window[1];
            if seg_start == seg_end {
                continue;
            }

            let old = self
                .entries
                .iter()
                .copied()
                .find(|entry| seg_start >= entry.start && seg_start < entry.end());
            let covered_by_new = seg_start >= start && seg_start < end;

            let next = match (old, covered_by_new) {
                (Some(entry), true) if acknowledged || !entry.acknowledged => Some(BadBlockEntry {
                    start: seg_start,
                    len: seg_end - seg_start,
                    acknowledged,
                }),
                (Some(entry), _) => Some(BadBlockEntry {
                    start: seg_start,
                    len: seg_end - seg_start,
                    acknowledged: entry.acknowledged,
                }),
                (None, true) => Some(BadBlockEntry {
                    start: seg_start,
                    len: seg_end - seg_start,
                    acknowledged,
                }),
                (None, false) => None,
            };

            if let Some(next) = next {
                push_split_merge(&mut out, next);
            }
        }

        if out.len() > MAX_BADBLOCKS {
            return false;
        }
        self.entries = out;
        self.changed = true;
        self.update_unacked();
        true
    }

    pub fn clear(&mut self, start: u64, sectors: u64) -> bool {
        if self.shift < 0 || sectors == 0 {
            return false;
        }
        let Some(end) = start.checked_add(sectors) else {
            return false;
        };
        let mut out = Vec::new();
        let mut cleared = false;

        for entry in self.entries.iter().copied() {
            if end <= entry.start || start >= entry.end() {
                out.push(entry);
                continue;
            }

            cleared = true;
            if start > entry.start {
                push_split_merge(
                    &mut out,
                    BadBlockEntry {
                        start: entry.start,
                        len: start - entry.start,
                        acknowledged: entry.acknowledged,
                    },
                );
            }
            if end < entry.end() {
                push_split_merge(
                    &mut out,
                    BadBlockEntry {
                        start: end,
                        len: entry.end() - end,
                        acknowledged: entry.acknowledged,
                    },
                );
            }
        }

        if out.len() > MAX_BADBLOCKS {
            return false;
        }
        if cleared {
            self.entries = out;
            self.changed = true;
            self.update_unacked();
        }
        true
    }

    pub fn check(&mut self, start: u64, sectors: u64) -> Option<BadBlockHit> {
        if self.shift < 0 || sectors == 0 {
            return None;
        }

        let (start, sectors) = self.shifted_range(start, sectors)?;
        let end = start.checked_add(sectors)?;
        let mut first = None;
        let mut saw_ack = false;
        let mut saw_unack = false;

        for entry in &self.entries {
            if entry.end() <= start {
                continue;
            }
            if entry.start >= end {
                break;
            }
            if first.is_none() {
                first = Some(*entry);
            }
            if entry.acknowledged {
                saw_ack = true;
            } else {
                saw_unack = true;
            }
        }

        match (first, saw_unack, saw_ack) {
            (Some(entry), true, _) => Some(BadBlockHit {
                first_bad: entry.start,
                bad_sectors: entry.len,
                status: -1,
            }),
            (Some(entry), false, true) => Some(BadBlockHit {
                first_bad: entry.start,
                bad_sectors: entry.len,
                status: 1,
            }),
            _ => {
                self.unacked_exist = false;
                None
            }
        }
    }

    pub fn ack_all(&mut self) {
        if self.changed || self.entries.is_empty() || !self.unacked_exist {
            return;
        }
        for entry in &mut self.entries {
            entry.acknowledged = true;
        }
        self.compact();
        self.unacked_exist = false;
    }

    pub fn clear_changed(&mut self) {
        self.changed = false;
    }

    pub fn show(&mut self, unack: bool) -> String {
        let mut out = String::new();
        if self.shift < 0 {
            return out;
        }
        for entry in &self.entries {
            if unack && entry.acknowledged {
                continue;
            }
            let scale = if self.shift == 0 {
                1
            } else {
                1u64 << self.shift as u32
            };
            use core::fmt::Write;
            let _ = writeln!(&mut out, "{} {}", entry.start * scale, entry.len * scale);
        }
        if unack && out.is_empty() {
            self.unacked_exist = false;
        }
        out
    }

    pub fn store(&mut self, input: &str, unack: bool) -> Result<usize, i32> {
        let len = input.len();
        let mut parts = input.split_whitespace();
        let sector = parts
            .next()
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or(-EINVAL)?;
        let sectors = parts
            .next()
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or(-EINVAL)?;
        if sectors == 0 || parts.next().is_some() {
            return Err(-EINVAL);
        }
        if self.set(sector, sectors, !unack) {
            Ok(len)
        } else {
            Err(-ENOSPC)
        }
    }

    fn shifted_range(&self, start: u64, sectors: u64) -> Option<(u64, u64)> {
        if self.shift <= 0 {
            return Some((start, sectors));
        }
        let block = 1u64 << self.shift as u32;
        let target = start.checked_add(sectors)?;
        let rounded_start = start / block * block;
        let rounded_end = target.checked_add(block - 1)? / block * block;
        Some((rounded_start, rounded_end - rounded_start))
    }

    fn compact(&mut self) {
        let mut out = Vec::new();
        for entry in self.entries.iter().copied() {
            push_split_merge(&mut out, entry);
        }
        self.entries = out;
    }

    fn update_unacked(&mut self) {
        self.unacked_exist = self.entries.iter().any(|entry| !entry.acknowledged);
    }
}

impl Default for BadBlocks {
    fn default() -> Self {
        Self::new(true)
    }
}

fn push_split_merge(out: &mut Vec<BadBlockEntry>, mut entry: BadBlockEntry) {
    if entry.len == 0 {
        return;
    }
    while entry.len > BB_MAX_LEN {
        let head = BadBlockEntry {
            start: entry.start,
            len: BB_MAX_LEN,
            acknowledged: entry.acknowledged,
        };
        push_one(out, head);
        entry.start += BB_MAX_LEN;
        entry.len -= BB_MAX_LEN;
    }
    push_one(out, entry);
}

fn push_one(out: &mut Vec<BadBlockEntry>, entry: BadBlockEntry) {
    if let Some(last) = out.last_mut() {
        if last.acknowledged == entry.acknowledged
            && last.end() == entry.start
            && last.len + entry.len <= BB_MAX_LEN
        {
            last.len += entry.len;
            return;
        }
        if last.start <= entry.start && entry.end() <= last.end() {
            return;
        }
        if last.acknowledged == entry.acknowledged
            && max(last.start, entry.start) <= min(last.end(), entry.end())
        {
            let end = max(last.end(), entry.end());
            last.start = min(last.start, entry.start);
            last.len = end - last.start;
            return;
        }
    }
    out.push(entry);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badblocks_encoding_matches_linux_header() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/badblocks.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/badblocks.h"
        ));
        assert!(header.contains("#define BB_LEN_MASK"));
        assert!(header.contains("#define BB_MAX_LEN\t512"));
        assert!(header.contains("#define MAX_BADBLOCKS\t(PAGE_SIZE/8)"));
        assert!(source.contains("badblocks_check(struct badblocks *bb"));
        assert!(source.contains("badblocks_set(struct badblocks *bb"));
        assert!(source.contains("ack_all_badblocks(struct badblocks *bb)"));

        let entry = BadBlockEntry {
            start: 42,
            len: 7,
            acknowledged: true,
        };
        assert_eq!(BadBlockEntry::decode(entry.encode()), entry);
        assert_eq!(BB_MAX_LEN, 512);
        assert_eq!(MAX_BADBLOCKS, 512);
    }

    #[test]
    fn set_check_clear_and_ack_preserve_linux_semantics() {
        let mut bb = BadBlocks::default();
        assert!(bb.set(10, 4, false));
        assert_eq!(
            bb.check(12, 1),
            Some(BadBlockHit {
                first_bad: 10,
                bad_sectors: 4,
                status: -1,
            })
        );

        assert!(bb.set(12, 2, true));
        assert_eq!(
            bb.entries(),
            &[
                BadBlockEntry {
                    start: 10,
                    len: 2,
                    acknowledged: false,
                },
                BadBlockEntry {
                    start: 12,
                    len: 2,
                    acknowledged: true,
                },
            ]
        );
        assert_eq!(
            bb.check(12, 1).unwrap(),
            BadBlockHit {
                first_bad: 12,
                bad_sectors: 2,
                status: 1,
            }
        );

        assert!(bb.clear(11, 2));
        assert_eq!(
            bb.entries(),
            &[
                BadBlockEntry {
                    start: 10,
                    len: 1,
                    acknowledged: false,
                },
                BadBlockEntry {
                    start: 13,
                    len: 1,
                    acknowledged: true,
                },
            ]
        );

        bb.clear_changed();
        bb.ack_all();
        assert!(bb.entries().iter().all(|entry| entry.acknowledged));
    }

    #[test]
    fn store_and_show_follow_sysfs_shape() {
        let mut bb = BadBlocks::default();
        assert_eq!(bb.store("100 3\n", true), Ok(6));
        assert_eq!(bb.show(true), "100 3\n");
        assert_eq!(bb.store("100 0\n", true), Err(-EINVAL));
    }
}
