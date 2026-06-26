//! linux-parity: partial
//! linux-source: vendor/linux/fs/fat/dir.c
//! FAT32 directory entry parsing (8.3 short names + LFN aggregation).
//!
//! Mirrors `vendor/linux/fs/fat/dir.c` and `namei_vfat.c`.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use super::FatSbi;
use super::fatent;

#[derive(Clone, Debug)]
pub struct FatDirEntry {
    pub name: String,  // resolved long name when present, else 8.3
    pub short: String, // raw 8.3 always populated
    pub cluster: u32,
    pub size: u32,
    pub attr: u8,
}

pub const ATTR_RO: u8 = 0x01;
pub const ATTR_HIDDEN: u8 = 0x02;
pub const ATTR_SYSTEM: u8 = 0x04;
pub const ATTR_VOLUME: u8 = 0x08;
pub const ATTR_DIR: u8 = 0x10;
pub const ATTR_ARCH: u8 = 0x20;
pub const ATTR_LFN: u8 = 0x0F;

/// Read all entries from the directory whose first cluster is `start_cluster`.
pub fn read_all(sbi: &FatSbi, start_cluster: u32) -> Result<Vec<FatDirEntry>, i32> {
    let chain = fatent::cluster_chain(sbi, start_cluster)?;
    let mut bytes = Vec::new();
    for c in chain.iter() {
        bytes.extend(fatent::read_cluster(sbi, *c)?);
    }
    parse_entries(&bytes)
}

fn parse_entries(buf: &[u8]) -> Result<Vec<FatDirEntry>, i32> {
    let mut out = Vec::new();
    let mut lfn_acc: Vec<u16> = Vec::new();
    let mut off = 0;
    while off + 32 <= buf.len() {
        let raw = &buf[off..off + 32];
        let first = raw[0];
        if first == 0x00 {
            break;
        }
        if first == 0xE5 {
            off += 32;
            lfn_acc.clear();
            continue;
        }
        let attr = raw[11];
        if attr == ATTR_LFN {
            // LFN entries store name fragments in UCS-2.  Order is reverse —
            // last-fragment-first carries the highest sequence number.
            let mut frag = [0u16; 13];
            // Bytes 1..=10 = chars 0..=4 (5 wchars), 14..=25 = chars 5..=10 (6 wchars), 28..=31 = chars 11..=12.
            for i in 0..5 {
                frag[i] = u16::from_le_bytes([raw[1 + i * 2], raw[2 + i * 2]]);
            }
            for i in 0..6 {
                frag[5 + i] = u16::from_le_bytes([raw[14 + i * 2], raw[15 + i * 2]]);
            }
            for i in 0..2 {
                frag[11 + i] = u16::from_le_bytes([raw[28 + i * 2], raw[29 + i * 2]]);
            }
            // Prepend (LFN order is last-first).
            let mut new_acc = Vec::with_capacity(13 + lfn_acc.len());
            for &c in frag.iter() {
                if c == 0 || c == 0xFFFF {
                    break;
                }
                new_acc.push(c);
            }
            new_acc.extend_from_slice(&lfn_acc);
            lfn_acc = new_acc;
            off += 32;
            continue;
        }

        // 8.3 entry.
        let mut name83 = String::new();
        for i in 0..8 {
            let b = raw[i];
            if b == b' ' {
                break;
            }
            name83.push(b as char);
        }
        let mut ext83 = String::new();
        for i in 0..3 {
            let b = raw[8 + i];
            if b == b' ' {
                break;
            }
            ext83.push(b as char);
        }
        let mut short = name83.clone();
        if !ext83.is_empty() {
            short.push('.');
            short.push_str(&ext83);
        }

        let cluster_hi = u16::from_le_bytes([raw[20], raw[21]]) as u32;
        let cluster_lo = u16::from_le_bytes([raw[26], raw[27]]) as u32;
        let cluster = (cluster_hi << 16) | cluster_lo;
        let size = u32::from_le_bytes([raw[28], raw[29], raw[30], raw[31]]);

        let long: Option<String> = if !lfn_acc.is_empty() {
            let s: String = char::decode_utf16(lfn_acc.iter().copied())
                .filter_map(|r| r.ok())
                .collect();
            lfn_acc.clear();
            Some(s)
        } else {
            None
        };

        out.push(FatDirEntry {
            name: long.unwrap_or_else(|| short.clone()),
            short,
            cluster,
            size,
            attr,
        });
        off += 32;
    }
    Ok(out)
}
