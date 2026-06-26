//! linux-parity: partial
//! linux-source: vendor/linux/block/partitions
//! GPT (GUID Partition Table) parser.
//!
//! Mirrors `vendor/linux/block/partitions/efi.c`.  GPT layout:
//!   * LBA 0: protective MBR (already validated by caller).
//!   * LBA 1: primary GPT header (96 bytes used + reserved).
//!   * LBA 2…: partition entry array, normally 128 entries × 128 bytes = 32 sectors.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::EINVAL;

use super::super::block_device::BlockDeviceRef;
use super::{Partition, read_sectors};

pub const GPT_SIGNATURE: &[u8; 8] = b"EFI PART";
pub const GPT_REVISION: u32 = 0x00010000;
pub const HEADER_LBA: u64 = 1;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct GptHeader {
    pub signature: [u8; 8],
    pub revision: u32,
    pub header_size: u32,
    pub header_crc32: u32,
    pub reserved: u32,
    pub current_lba: u64,
    pub backup_lba: u64,
    pub first_usable: u64,
    pub last_usable: u64,
    pub disk_guid: [u8; 16],
    pub partition_entry_lba: u64,
    pub num_partition_entries: u32,
    pub size_of_partition_entry: u32,
    pub partition_entry_array_crc32: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct GptPartitionEntry {
    pub partition_type_guid: [u8; 16],
    pub unique_partition_guid: [u8; 16],
    pub starting_lba: u64,
    pub ending_lba: u64,
    pub attributes: u64,
    pub partition_name: [u16; 36],
}

const TYPE_UNUSED_GUID: [u8; 16] = [0u8; 16];
const MIN_PARTITION_ENTRY_SIZE: usize = 48;

// CRC32 (IEEE polynomial 0xEDB88320).  Same constants Linux's `crc32_le` uses.
fn crc32_ieee(data: &[u8]) -> u32 {
    const POLY: u32 = 0xEDB88320;
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (POLY & mask);
        }
    }
    !crc
}

fn parse_header(sector1: &[u8]) -> Option<GptHeader> {
    if sector1.len() < 92 {
        return None;
    }
    if &sector1[0..8] != GPT_SIGNATURE {
        return None;
    }
    let header_size = u32::from_le_bytes([sector1[12], sector1[13], sector1[14], sector1[15]]);
    if header_size < 92 || (header_size as usize) > sector1.len() {
        return None;
    }

    // Verify header CRC: compute over header_size bytes with the CRC field zeroed.
    let mut tmp = sector1[..header_size as usize].to_vec();
    tmp[16..20].copy_from_slice(&[0u8; 4]);
    let want = u32::from_le_bytes([sector1[16], sector1[17], sector1[18], sector1[19]]);
    if crc32_ieee(&tmp) != want {
        return None;
    }

    Some(GptHeader {
        signature: [
            sector1[0], sector1[1], sector1[2], sector1[3], sector1[4], sector1[5], sector1[6],
            sector1[7],
        ],
        revision: u32::from_le_bytes([sector1[8], sector1[9], sector1[10], sector1[11]]),
        header_size,
        header_crc32: want,
        reserved: 0,
        current_lba: u64::from_le_bytes([
            sector1[24],
            sector1[25],
            sector1[26],
            sector1[27],
            sector1[28],
            sector1[29],
            sector1[30],
            sector1[31],
        ]),
        backup_lba: u64::from_le_bytes([
            sector1[32],
            sector1[33],
            sector1[34],
            sector1[35],
            sector1[36],
            sector1[37],
            sector1[38],
            sector1[39],
        ]),
        first_usable: u64::from_le_bytes([
            sector1[40],
            sector1[41],
            sector1[42],
            sector1[43],
            sector1[44],
            sector1[45],
            sector1[46],
            sector1[47],
        ]),
        last_usable: u64::from_le_bytes([
            sector1[48],
            sector1[49],
            sector1[50],
            sector1[51],
            sector1[52],
            sector1[53],
            sector1[54],
            sector1[55],
        ]),
        disk_guid: {
            let mut g = [0u8; 16];
            g.copy_from_slice(&sector1[56..72]);
            g
        },
        partition_entry_lba: u64::from_le_bytes([
            sector1[72],
            sector1[73],
            sector1[74],
            sector1[75],
            sector1[76],
            sector1[77],
            sector1[78],
            sector1[79],
        ]),
        num_partition_entries: u32::from_le_bytes([
            sector1[80],
            sector1[81],
            sector1[82],
            sector1[83],
        ]),
        size_of_partition_entry: u32::from_le_bytes([
            sector1[84],
            sector1[85],
            sector1[86],
            sector1[87],
        ]),
        partition_entry_array_crc32: u32::from_le_bytes([
            sector1[88],
            sector1[89],
            sector1[90],
            sector1[91],
        ]),
    })
}

pub fn parse(bdev: &BlockDeviceRef) -> Result<Vec<Partition>, i32> {
    let sector1 = read_sectors(bdev, HEADER_LBA, 1)?;
    let hdr = parse_header(&sector1).ok_or(EINVAL)?;

    let stride = hdr.size_of_partition_entry as usize;
    if stride < MIN_PARTITION_ENTRY_SIZE {
        return Err(EINVAL);
    }

    let entry_bytes = (hdr.num_partition_entries as usize)
        .checked_mul(stride)
        .ok_or(EINVAL)?;
    let nr_sectors = entry_bytes.div_ceil(512) as u64;
    let entries_buf = read_sectors(bdev, hdr.partition_entry_lba, nr_sectors)?;
    if entries_buf.len() < entry_bytes {
        return Err(EINVAL);
    }

    if crc32_ieee(&entries_buf[..entry_bytes]) != hdr.partition_entry_array_crc32 {
        return Err(EINVAL);
    }

    let mut out = Vec::new();
    for i in 0..(hdr.num_partition_entries as usize) {
        let off = i * stride;
        let mut type_guid = [0u8; 16];
        type_guid.copy_from_slice(&entries_buf[off..off + 16]);
        if type_guid == TYPE_UNUSED_GUID {
            continue;
        }
        let start = u64::from_le_bytes([
            entries_buf[off + 32],
            entries_buf[off + 33],
            entries_buf[off + 34],
            entries_buf[off + 35],
            entries_buf[off + 36],
            entries_buf[off + 37],
            entries_buf[off + 38],
            entries_buf[off + 39],
        ]);
        let end = u64::from_le_bytes([
            entries_buf[off + 40],
            entries_buf[off + 41],
            entries_buf[off + 42],
            entries_buf[off + 43],
            entries_buf[off + 44],
            entries_buf[off + 45],
            entries_buf[off + 46],
            entries_buf[off + 47],
        ]);
        out.push(Partition {
            number: (i as u32) + 1,
            start_sector: start,
            nr_sectors: end.saturating_sub(start) + 1,
            type_guid: Some(type_guid),
            type_byte: None,
        });
    }
    Ok(out)
}

/// Build a minimal-but-valid GPT header in `out`.  Used for test fixtures.
pub fn build_header(
    out: &mut [u8],
    current_lba: u64,
    backup_lba: u64,
    entries_lba: u64,
    num_entries: u32,
    entry_size: u32,
    entries_crc: u32,
) -> u32 {
    out.fill(0);
    out[0..8].copy_from_slice(GPT_SIGNATURE);
    out[8..12].copy_from_slice(&GPT_REVISION.to_le_bytes());
    out[12..16].copy_from_slice(&92u32.to_le_bytes()); // header size
    // CRC zero for now
    out[20..24].copy_from_slice(&0u32.to_le_bytes());
    out[24..32].copy_from_slice(&current_lba.to_le_bytes());
    out[32..40].copy_from_slice(&backup_lba.to_le_bytes());
    out[40..48].copy_from_slice(&34u64.to_le_bytes()); // first_usable
    out[48..56].copy_from_slice(&((backup_lba - 1) as u64).to_le_bytes()); // last_usable
    // disk_guid left as zeros
    out[72..80].copy_from_slice(&entries_lba.to_le_bytes());
    out[80..84].copy_from_slice(&num_entries.to_le_bytes());
    out[84..88].copy_from_slice(&entry_size.to_le_bytes());
    out[88..92].copy_from_slice(&entries_crc.to_le_bytes());
    let crc = crc32_ieee(&out[..92]);
    out[16..20].copy_from_slice(&crc.to_le_bytes());
    crc
}

pub fn build_partition_entry(
    out: &mut [u8],
    type_guid: [u8; 16],
    starting_lba: u64,
    ending_lba: u64,
) {
    out.fill(0);
    out[0..16].copy_from_slice(&type_guid);
    // unique guid left zero
    out[32..40].copy_from_slice(&starting_lba.to_le_bytes());
    out[40..48].copy_from_slice(&ending_lba.to_le_bytes());
}

pub fn entries_crc(entries: &[u8]) -> u32 {
    crc32_ieee(entries)
}
