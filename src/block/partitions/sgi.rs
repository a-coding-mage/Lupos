//! linux-parity: complete
//! linux-source: vendor/linux/block/partitions/sgi.c
//! test-origin: linux:vendor/linux/block/partitions/sgi.c
//! SGI disklabel parser.

extern crate alloc;

use alloc::vec::Vec;

use super::Partition;

pub const SGI_LABEL_MAGIC: u32 = 0x0be5_a941;
pub const SGI_PARTITION_COUNT: usize = 16;
pub const LINUX_RAID_PARTITION: u32 = 0xfd;

const SGI_PARTITIONS_OFFSET: usize = 312;
const SGI_PARTITION_SIZE: usize = 12;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SgiPartition {
    pub partition: Partition,
    pub raid: bool,
}

pub fn parse_sgi_sector(sector: &[u8]) -> Option<Vec<SgiPartition>> {
    if sector.len() < 512 || be32(sector, 0)? != SGI_LABEL_MAGIC {
        return None;
    }
    if sgi_checksum(sector)? != 0 {
        return None;
    }

    let mut partitions = Vec::new();
    for index in 0..SGI_PARTITION_COUNT {
        let offset = SGI_PARTITIONS_OFFSET + index * SGI_PARTITION_SIZE;
        let blocks = be32(sector, offset)? as u64;
        let start = be32(sector, offset + 4)? as u64;
        let ty = be32(sector, offset + 8)?;
        if blocks != 0 {
            partitions.push(SgiPartition {
                partition: Partition {
                    number: index as u32 + 1,
                    start_sector: start,
                    nr_sectors: blocks,
                    type_guid: None,
                    type_byte: None,
                },
                raid: ty == LINUX_RAID_PARTITION,
            });
        }
    }
    Some(partitions)
}

pub fn sgi_checksum(sector: &[u8]) -> Option<u32> {
    if sector.len() < 512 {
        return None;
    }
    let mut sum = 0u32;
    for word in sector[..512].chunks_exact(4) {
        sum = sum.wrapping_add(u32::from_be_bytes(word.try_into().ok()?));
    }
    Some(sum)
}

fn be32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SGI_CHECKSUM_OFFSET: usize = 504;

    fn write_be32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn write_partition(bytes: &mut [u8], index: usize, blocks: u32, start: u32, ty: u32) {
        let offset = SGI_PARTITIONS_OFFSET + index * SGI_PARTITION_SIZE;
        write_be32(bytes, offset, blocks);
        write_be32(bytes, offset + 4, start);
        write_be32(bytes, offset + 8, ty);
    }

    fn fix_checksum(bytes: &mut [u8]) {
        write_be32(bytes, SGI_CHECKSUM_OFFSET, 0);
        let sum = sgi_checksum(bytes).expect("checksum");
        write_be32(bytes, SGI_CHECKSUM_OFFSET, (!sum).wrapping_add(1));
    }

    #[test]
    fn sgi_parser_matches_linux_magic_checksum_and_raid_flag() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/partitions/sgi.c"
        ));
        assert!(source.contains("#define SGI_LABEL_MAGIC 0x0be5a941"));
        assert!(source.contains("struct sgi_partition"));
        assert!(source.contains("for(csum = 0; ui >= ((__be32 *) label);)"));
        assert!(source.contains("put_partition(state, slot, start, blocks);"));
        assert!(source.contains("state->parts[slot].flags = ADDPART_FLAG_RAID;"));

        let mut sector = [0u8; 512];
        write_be32(&mut sector, 0, SGI_LABEL_MAGIC);
        write_partition(&mut sector, 0, 128, 4096, LINUX_RAID_PARTITION);
        write_partition(&mut sector, 1, 64, 8192, 0x83);
        fix_checksum(&mut sector);

        let partitions = parse_sgi_sector(&sector).expect("sgi label");
        assert_eq!(partitions.len(), 2);
        assert_eq!(partitions[0].partition.number, 1);
        assert_eq!(partitions[0].partition.start_sector, 4096);
        assert_eq!(partitions[0].partition.nr_sectors, 128);
        assert!(partitions[0].raid);
        assert_eq!(partitions[1].partition.number, 2);
        assert!(!partitions[1].raid);

        sector[SGI_CHECKSUM_OFFSET + 3] ^= 1;
        assert_eq!(parse_sgi_sector(&sector), None);
    }
}
