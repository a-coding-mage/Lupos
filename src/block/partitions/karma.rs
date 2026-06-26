//! linux-parity: complete
//! linux-source: vendor/linux/block/partitions/karma.c
//! test-origin: linux:vendor/linux/block/partitions/karma.c
//! Rio Karma disklabel parser.

extern crate alloc;

use alloc::vec::Vec;

use super::Partition;

pub const KARMA_LABEL_MAGIC: u16 = 0xAB56;
pub const KARMA_PARTITION_COUNT: usize = 2;
pub const KARMA_PARTITION_TYPE: u8 = 0x4d;

const KARMA_MAGIC_OFFSET: usize = 510;
const KARMA_PARTITIONS_OFFSET: usize = 270;
const KARMA_PARTITION_SIZE: usize = 16;

pub fn parse_karma_sector(sector: &[u8], limit: usize) -> Option<Vec<Partition>> {
    if sector.len() < 512 || le16(sector, KARMA_MAGIC_OFFSET)? != KARMA_LABEL_MAGIC {
        return None;
    }

    let mut partitions = Vec::new();
    let mut slot = 1usize;
    for index in 0..KARMA_PARTITION_COUNT {
        if slot == limit {
            break;
        }

        let offset = KARMA_PARTITIONS_OFFSET + index * KARMA_PARTITION_SIZE;
        let fstype = *sector.get(offset + 4)?;
        let start = le32(sector, offset + 8)? as u64;
        let size = le32(sector, offset + 12)? as u64;
        if fstype == KARMA_PARTITION_TYPE && size != 0 {
            partitions.push(Partition {
                number: slot as u32,
                start_sector: start,
                nr_sectors: size,
                type_guid: None,
                type_byte: Some(fstype),
            });
        }
        slot += 1;
    }

    Some(partitions)
}

fn le16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn le32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_le16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn write_le32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    #[test]
    fn karma_parser_matches_linux_label_layout() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/partitions/karma.c"
        ));
        assert!(source.contains("#define KARMA_LABEL_MAGIC\t\t0xAB56"));
        assert!(source.contains("u8 d_reserved[270];"));
        assert!(source.contains("} d_partitions[2];"));
        assert!(source.contains("p->p_fstype == 0x4d"));
        assert!(source.contains("put_partition(state, slot, le32_to_cpu(p->p_offset)"));

        let mut sector = [0u8; 512];
        write_le16(&mut sector, KARMA_MAGIC_OFFSET, KARMA_LABEL_MAGIC);
        let part0 = KARMA_PARTITIONS_OFFSET;
        sector[part0 + 4] = KARMA_PARTITION_TYPE;
        write_le32(&mut sector, part0 + 8, 63);
        write_le32(&mut sector, part0 + 12, 1000);
        let part1 = part0 + KARMA_PARTITION_SIZE;
        sector[part1 + 4] = 0x83;
        write_le32(&mut sector, part1 + 8, 2048);
        write_le32(&mut sector, part1 + 12, 4096);

        let partitions = parse_karma_sector(&sector, 16).expect("karma label");
        assert_eq!(partitions.len(), 1);
        assert_eq!(partitions[0].number, 1);
        assert_eq!(partitions[0].start_sector, 63);
        assert_eq!(partitions[0].nr_sectors, 1000);
        assert_eq!(partitions[0].type_byte, Some(KARMA_PARTITION_TYPE));

        sector[KARMA_MAGIC_OFFSET] = 0;
        assert_eq!(parse_karma_sector(&sector, 16), None);
    }
}
