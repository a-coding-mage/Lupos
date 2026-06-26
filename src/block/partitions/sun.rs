//! linux-parity: complete
//! linux-source: vendor/linux/block/partitions/sun.c
//! test-origin: linux:vendor/linux/block/partitions/sun.c
//! Sun disklabel parser.

extern crate alloc;

use alloc::vec::Vec;

use super::Partition;

pub const SUN_LABEL_MAGIC: u16 = 0xdabe;
pub const SUN_VTOC_SANITY: u32 = 0x600d_deee;
pub const SUN_WHOLE_DISK: u16 = 5;
pub const LINUX_RAID_PARTITION: u16 = 0x00fd;
pub const SUN_PARTITION_COUNT: usize = 8;

const VTOC_VERSION_OFFSET: usize = 128;
const VTOC_NPARTS_OFFSET: usize = 140;
const VTOC_INFOS_OFFSET: usize = 142;
const VTOC_SANITY_OFFSET: usize = 188;
const NTRKS_OFFSET: usize = 436;
const NSECT_OFFSET: usize = 438;
const PARTITIONS_OFFSET: usize = 444;
const SUN_PARTITION_SIZE: usize = 8;
const MAGIC_OFFSET: usize = 508;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SunPartition {
    pub partition: Partition,
    pub raid: bool,
    pub whole_disk: bool,
}

pub fn parse_sun_sector(sector: &[u8]) -> Option<Vec<SunPartition>> {
    if sector.len() < 512 || be16(sector, MAGIC_OFFSET)? != SUN_LABEL_MAGIC {
        return None;
    }
    if sun_label_checksum(sector)? != 0 {
        return None;
    }

    let vtoc_sanity = be32(sector, VTOC_SANITY_OFFSET)?;
    let vtoc_version = be32(sector, VTOC_VERSION_OFFSET)?;
    let vtoc_nparts = be16(sector, VTOC_NPARTS_OFFSET)? as usize;
    let vtoc_fields_zero = vtoc_sanity == 0 && vtoc_version == 0 && vtoc_nparts == 0;
    let valid_vtoc = vtoc_sanity == SUN_VTOC_SANITY && vtoc_version == 1 && vtoc_nparts <= 8;
    let use_vtoc_for_count = valid_vtoc;
    let use_vtoc_flags = valid_vtoc || vtoc_fields_zero;
    let nparts = if use_vtoc_for_count {
        vtoc_nparts
    } else {
        SUN_PARTITION_COUNT
    };
    let spc = (be16(sector, NTRKS_OFFSET)? as u64) * (be16(sector, NSECT_OFFSET)? as u64);

    let mut partitions = Vec::new();
    for index in 0..nparts {
        let offset = PARTITIONS_OFFSET + index * SUN_PARTITION_SIZE;
        let start_sector = (be32(sector, offset)? as u64).saturating_mul(spc);
        let nr_sectors = be32(sector, offset + 4)? as u64;
        if nr_sectors == 0 {
            continue;
        }

        let mut raid = false;
        let mut whole_disk = false;
        if use_vtoc_flags {
            let id = be16(sector, VTOC_INFOS_OFFSET + index * 4)?;
            raid = id == LINUX_RAID_PARTITION;
            whole_disk = id == SUN_WHOLE_DISK;
        }

        partitions.push(SunPartition {
            partition: Partition {
                number: index as u32 + 1,
                start_sector,
                nr_sectors,
                type_guid: None,
                type_byte: None,
            },
            raid,
            whole_disk,
        });
    }
    Some(partitions)
}

pub fn sun_label_checksum(sector: &[u8]) -> Option<u16> {
    if sector.len() < 512 {
        return None;
    }
    let mut checksum = 0u16;
    for word in sector[..512].chunks_exact(2) {
        checksum ^= u16::from_be_bytes(word.try_into().ok()?);
    }
    Some(checksum)
}

fn be16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn be32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHECKSUM_OFFSET: usize = 510;

    fn write_be16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
    }

    fn write_be32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn write_partition(bytes: &mut [u8], index: usize, start_cyl: u32, sectors: u32) {
        let offset = PARTITIONS_OFFSET + index * SUN_PARTITION_SIZE;
        write_be32(bytes, offset, start_cyl);
        write_be32(bytes, offset + 4, sectors);
    }

    fn fix_checksum(bytes: &mut [u8]) {
        write_be16(bytes, CHECKSUM_OFFSET, 0);
        let checksum = sun_label_checksum(bytes).expect("checksum");
        write_be16(bytes, CHECKSUM_OFFSET, checksum);
    }

    #[test]
    fn sun_parser_matches_linux_magic_checksum_vtoc_and_flags() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/partitions/sun.c"
        ));
        assert!(source.contains("#define SUN_LABEL_MAGIC          0xDABE"));
        assert!(source.contains("#define SUN_VTOC_SANITY          0x600DDEEE"));
        assert!(source.contains("SUN_WHOLE_DISK = 5"));
        assert!(source.contains("LINUX_RAID_PARTITION = 0xfd"));
        assert!(source.contains("csum ^= *ush--;"));
        assert!(source.contains("put_partition(state, slot, st_sector, num_sectors);"));
        assert!(source.contains("state->parts[slot].flags |= ADDPART_FLAG_RAID;"));
        assert!(source.contains("state->parts[slot].flags |= ADDPART_FLAG_WHOLEDISK;"));

        let mut sector = [0u8; 512];
        write_be32(&mut sector, VTOC_VERSION_OFFSET, 1);
        write_be16(&mut sector, VTOC_NPARTS_OFFSET, 3);
        write_be32(&mut sector, VTOC_SANITY_OFFSET, SUN_VTOC_SANITY);
        write_be16(&mut sector, VTOC_INFOS_OFFSET, LINUX_RAID_PARTITION);
        write_be16(&mut sector, VTOC_INFOS_OFFSET + 4, SUN_WHOLE_DISK);
        write_be16(&mut sector, NTRKS_OFFSET, 4);
        write_be16(&mut sector, NSECT_OFFSET, 16);
        write_partition(&mut sector, 0, 2, 128);
        write_partition(&mut sector, 1, 4, 256);
        write_partition(&mut sector, 2, 0, 0);
        write_be16(&mut sector, MAGIC_OFFSET, SUN_LABEL_MAGIC);
        fix_checksum(&mut sector);

        let partitions = parse_sun_sector(&sector).expect("sun label");
        assert_eq!(partitions.len(), 2);
        assert_eq!(partitions[0].partition.number, 1);
        assert_eq!(partitions[0].partition.start_sector, 128);
        assert_eq!(partitions[0].partition.nr_sectors, 128);
        assert!(partitions[0].raid);
        assert!(!partitions[0].whole_disk);
        assert_eq!(partitions[1].partition.start_sector, 256);
        assert!(partitions[1].whole_disk);

        sector[CHECKSUM_OFFSET + 1] ^= 1;
        assert_eq!(parse_sun_sector(&sector), None);
    }
}
