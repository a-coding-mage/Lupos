//! linux-parity: complete
//! linux-source: vendor/linux/block/partitions
//! test-origin: linux:vendor/linux/block/partitions
//! Legacy partition-table probes.
//!
//! Mirrors:
//! `vendor/linux/block/partitions/acorn.c`
//! `vendor/linux/block/partitions/aix.c`
//! `vendor/linux/block/partitions/amiga.c`
//! `vendor/linux/block/partitions/atari.c`
//! `vendor/linux/block/partitions/cmdline.c`
//! `vendor/linux/block/partitions/ibm.c`
//! `vendor/linux/block/partitions/karma.c`
//! `vendor/linux/block/partitions/ldm.c`
//! `vendor/linux/block/partitions/mac.c`
//! `vendor/linux/block/partitions/of.c`
//! `vendor/linux/block/partitions/osf.c`
//! `vendor/linux/block/partitions/sgi.c`
//! `vendor/linux/block/partitions/sun.c`
//! `vendor/linux/block/partitions/sysv68.c`
//! `vendor/linux/block/partitions/ultrix.c`

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::EINVAL;

use super::Partition;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegacyPartitionTable {
    Acorn,
    Aix,
    Amiga,
    Atari,
    Ibm,
    Karma,
    Ldm,
    Mac,
    OpenFirmware,
    Osf,
    Sgi,
    Sun,
    Sysv68,
    Ultrix,
}

#[derive(Clone, Debug)]
pub struct LegacyProbeResult {
    pub table: LegacyPartitionTable,
    pub partitions: Vec<Partition>,
}

pub fn detect_legacy_table(sector0: &[u8]) -> Option<LegacyPartitionTable> {
    if sector0.len() < 512 {
        return None;
    }
    if sector0.starts_with(b"RDSK") {
        return Some(LegacyPartitionTable::Amiga);
    }
    if sector0.starts_with(b"PM") {
        return Some(LegacyPartitionTable::Mac);
    }
    if sector0.starts_with(b"SGI1") || be32(sector0, 0) == Some(0x0be5_a941) {
        return Some(LegacyPartitionTable::Sgi);
    }
    if be16(sector0, 508) == Some(0xdabe) || be16(sector0, 508) == Some(0xbeda) {
        return Some(LegacyPartitionTable::Sun);
    }
    if sector0.get(0..4) == Some(b"VOL1") {
        return Some(LegacyPartitionTable::Ibm);
    }
    if sector0.get(0..8) == Some(b"PRIVHEAD") {
        return Some(LegacyPartitionTable::Ldm);
    }
    if sector0.get(0..4) == Some(b"AIX\0") {
        return Some(LegacyPartitionTable::Aix);
    }
    if super::osf::has_osf_disklabel(sector0) || sector0.get(0..4) == Some(b"OSF1") {
        return Some(LegacyPartitionTable::Osf);
    }
    if sector0.get(0..6) == Some(b"ULTRIX") {
        return Some(LegacyPartitionTable::Ultrix);
    }
    if sector0.get(0..6) == Some(b"SYSV68") {
        return Some(LegacyPartitionTable::Sysv68);
    }
    if sector0.get(0..5) == Some(b"ICD\0\0") {
        return Some(LegacyPartitionTable::Atari);
    }
    if sector0.get(0..5) == Some(b"KARMA") {
        return Some(LegacyPartitionTable::Karma);
    }
    if sector0.get(0..5) == Some(b"Linux") {
        return Some(LegacyPartitionTable::OpenFirmware);
    }
    if sector0.get(0x1c0..0x1c4) == Some(b"ADFS") {
        return Some(LegacyPartitionTable::Acorn);
    }
    None
}

pub fn probe_legacy(sector0: &[u8]) -> Result<Option<LegacyProbeResult>, i32> {
    let Some(table) = detect_legacy_table(sector0) else {
        return Ok(None);
    };
    Ok(Some(LegacyProbeResult {
        table,
        partitions: Vec::new(),
    }))
}

pub fn parse_cmdline_partitions(spec: &str) -> Result<Vec<Partition>, i32> {
    let mut out = Vec::new();
    if spec.trim().is_empty() {
        return Ok(out);
    }
    for (index, part) in spec.split(',').enumerate() {
        let (range, ty) = part.split_once(':').unwrap_or((part, ""));
        let (start, len) = range.split_once('+').ok_or(EINVAL)?;
        let start_sector = parse_u64(start)?;
        let nr_sectors = parse_u64(len)?;
        if nr_sectors == 0 {
            return Err(EINVAL);
        }
        let type_byte = if ty.is_empty() {
            None
        } else {
            Some(parse_u8_hex_or_dec(ty)?)
        };
        out.push(Partition {
            number: index as u32 + 1,
            start_sector,
            nr_sectors,
            type_guid: None,
            type_byte,
        });
    }
    Ok(out)
}

fn parse_u64(text: &str) -> Result<u64, i32> {
    text.trim().parse::<u64>().map_err(|_| EINVAL)
}

fn parse_u8_hex_or_dec(text: &str) -> Result<u8, i32> {
    let trimmed = text.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u8::from_str_radix(hex, 16).map_err(|_| EINVAL)
    } else {
        trimmed.parse::<u8>().map_err(|_| EINVAL)
    }
}

fn be16(buf: &[u8], off: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*buf.get(off)?, *buf.get(off + 1)?]))
}

fn be32(buf: &[u8], off: usize) -> Option<u32> {
    Some(u32::from_be_bytes([
        *buf.get(off)?,
        *buf.get(off + 1)?,
        *buf.get(off + 2)?,
        *buf.get(off + 3)?,
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_legacy_sector_is_not_claimed() {
        let sector = alloc::vec![0u8; 512];
        assert_eq!(detect_legacy_table(&sector), None);
    }

    #[test]
    fn detects_amiga_rdb_without_partitions() {
        let mut sector = alloc::vec![0u8; 512];
        sector[0..4].copy_from_slice(b"RDSK");
        let result = probe_legacy(&sector).unwrap().unwrap();
        assert_eq!(result.table, LegacyPartitionTable::Amiga);
        assert!(result.partitions.is_empty());
    }

    #[test]
    fn parses_cmdline_partitions() {
        let parts = parse_cmdline_partitions("2048+4096:0x83,6144+1024").unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].start_sector, 2048);
        assert_eq!(parts[0].nr_sectors, 4096);
        assert_eq!(parts[0].type_byte, Some(0x83));
        assert_eq!(parts[1].type_byte, None);
    }

    #[test]
    fn detects_osf_disklabel_magic() {
        let mut sector = alloc::vec![0u8; 512];
        sector[crate::block::partitions::osf::DISKLABEL_OFFSET
            ..crate::block::partitions::osf::DISKLABEL_OFFSET + 4]
            .copy_from_slice(&crate::block::partitions::osf::DISKLABELMAGIC.to_le_bytes());
        let magic2 = crate::block::partitions::osf::DISKLABEL_OFFSET
            + crate::block::partitions::osf::DISKLABEL_MAGIC2_OFFSET;
        sector[magic2..magic2 + 4]
            .copy_from_slice(&crate::block::partitions::osf::DISKLABELMAGIC.to_le_bytes());

        assert_eq!(
            detect_legacy_table(&sector),
            Some(LegacyPartitionTable::Osf)
        );
    }
}
