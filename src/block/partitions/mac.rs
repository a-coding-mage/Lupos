//! linux-parity: complete
//! linux-source: vendor/linux/block/partitions/mac.c
//! test-origin: linux:vendor/linux/block/partitions/mac.c
//! Apple partition map parser.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::EINVAL;

use super::Partition;

pub const MAC_DRIVER_MAGIC: u16 = 0x4552;
pub const MAC_PARTITION_MAGIC: u16 = 0x504d;
pub const MAC_STATUS_BOOTABLE: u32 = 8;
pub const APPLE_AUX_TYPE: &[u8; 15] = b"Apple_UNIX_SVR2";
pub const DISK_MAX_PARTS: usize = 256;

const MAC_PARTITION_ENTRY_SIZE: usize = 136;
const MAC_NAME_OFFSET: usize = 16;
const MAC_NAME_LEN: usize = 32;
const MAC_TYPE_OFFSET: usize = 48;
const MAC_TYPE_LEN: usize = 32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MacPartition {
    pub partition: Partition,
    pub name: [u8; MAC_NAME_LEN],
    pub ty: [u8; MAC_TYPE_LEN],
    pub status: u32,
    pub raid: bool,
}

pub fn parse_mac_partitions(disk: &[u8], limit: usize) -> Result<Option<Vec<MacPartition>>, i32> {
    if disk.len() < 512 {
        return Err(EINVAL);
    }
    if be16(disk, 0).ok_or(EINVAL)? != MAC_DRIVER_MAGIC {
        return Ok(None);
    }

    let secsize = be16(disk, 2).ok_or(EINVAL)? as usize;
    if !is_power_of_two(secsize) {
        return Err(EINVAL);
    }

    let datasize = secsize & !511usize;
    let partoffset = secsize % 512;
    if partoffset + MAC_PARTITION_ENTRY_SIZE > datasize {
        return Err(EINVAL);
    }

    let first = partition_entry(disk, secsize)?;
    if be16(first, 0).ok_or(EINVAL)? != MAC_PARTITION_MAGIC {
        return Ok(None);
    }

    let map_count = be32(first, 4).ok_or(EINVAL)? as usize;
    if map_count >= DISK_MAX_PARTS {
        return Ok(None);
    }
    let blocks_in_map = if map_count >= limit {
        limit.saturating_sub(1)
    } else {
        map_count
    };

    let mut partitions = Vec::new();
    let sector_factor = secsize / 512;
    for slot in 1..=blocks_in_map {
        let pos = slot.checked_mul(secsize).ok_or(EINVAL)?;
        let entry = partition_entry(disk, pos)?;
        if be16(entry, 0).ok_or(EINVAL)? != MAC_PARTITION_MAGIC {
            break;
        }

        let start_block = be32(entry, 8).ok_or(EINVAL)? as u64;
        let block_count = be32(entry, 12).ok_or(EINVAL)? as u64;
        let mut name = [0u8; MAC_NAME_LEN];
        name.copy_from_slice(&entry[MAC_NAME_OFFSET..MAC_NAME_OFFSET + MAC_NAME_LEN]);
        let mut ty = [0u8; MAC_TYPE_LEN];
        ty.copy_from_slice(&entry[MAC_TYPE_OFFSET..MAC_TYPE_OFFSET + MAC_TYPE_LEN]);
        partitions.push(MacPartition {
            partition: Partition {
                number: slot as u32,
                start_sector: start_block * sector_factor as u64,
                nr_sectors: block_count * sector_factor as u64,
                type_guid: None,
                type_byte: None,
            },
            name,
            raid: ascii_case_prefix(&ty, b"Linux_RAID"),
            ty,
            status: be32(entry, 88).ok_or(EINVAL)?,
        });
    }

    Ok(Some(partitions))
}

pub const fn is_power_of_two(value: usize) -> bool {
    value != 0 && (value & (value - 1)) == 0
}

fn partition_entry(disk: &[u8], pos: usize) -> Result<&[u8], i32> {
    disk.get(pos..pos + MAC_PARTITION_ENTRY_SIZE).ok_or(EINVAL)
}

fn ascii_case_prefix(field: &[u8], prefix: &[u8]) -> bool {
    field.len() >= prefix.len()
        && field[..prefix.len()]
            .iter()
            .zip(prefix.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
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

    fn write_be16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
    }

    fn write_be32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn write_driver(bytes: &mut [u8], block_size: u16) {
        write_be16(bytes, 0, MAC_DRIVER_MAGIC);
        write_be16(bytes, 2, block_size);
    }

    fn write_partition(
        bytes: &mut [u8],
        offset: usize,
        map_count: u32,
        start_block: u32,
        block_count: u32,
        ty: &[u8],
    ) {
        write_be16(bytes, offset, MAC_PARTITION_MAGIC);
        write_be32(bytes, offset + 4, map_count);
        write_be32(bytes, offset + 8, start_block);
        write_be32(bytes, offset + 12, block_count);
        bytes[offset + MAC_TYPE_OFFSET..offset + MAC_TYPE_OFFSET + ty.len()].copy_from_slice(ty);
        write_be32(bytes, offset + 88, MAC_STATUS_BOOTABLE);
    }

    #[test]
    fn mac_parser_matches_linux_driver_descriptor_and_map_entries() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/partitions/mac.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/partitions/mac.h"
        ));
        assert!(source.contains("be16_to_cpu(md->signature) != MAC_DRIVER_MAGIC"));
        assert!(source.contains("if (!is_power_of_2(secsize))"));
        assert!(source.contains("be16_to_cpu(part->signature) != MAC_PARTITION_MAGIC"));
        assert!(source.contains("put_partition(state, slot,"));
        assert!(source.contains("!strncasecmp(part->type, \"Linux_RAID\", 10)"));
        assert!(header.contains("#define MAC_PARTITION_MAGIC\t0x504d"));
        assert!(header.contains("#define MAC_DRIVER_MAGIC\t0x4552"));

        let mut disk = alloc::vec![0u8; 4 * 512];
        write_driver(&mut disk, 512);
        write_partition(&mut disk, 512, 2, 16, 32, b"Linux_RAID");
        write_partition(&mut disk, 1024, 2, 48, 64, APPLE_AUX_TYPE);

        let partitions = parse_mac_partitions(&disk, 16)
            .expect("parse")
            .expect("mac map");
        assert_eq!(partitions.len(), 2);
        assert_eq!(partitions[0].partition.number, 1);
        assert_eq!(partitions[0].partition.start_sector, 16);
        assert_eq!(partitions[0].partition.nr_sectors, 32);
        assert!(partitions[0].raid);
        assert_eq!(partitions[1].partition.number, 2);
        assert!(!partitions[1].raid);
        assert_eq!(&partitions[1].ty[..APPLE_AUX_TYPE.len()], APPLE_AUX_TYPE);

        write_be16(&mut disk, 2, 768);
        assert_eq!(parse_mac_partitions(&disk, 16), Err(EINVAL));
    }
}
