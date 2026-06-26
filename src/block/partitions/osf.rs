//! linux-parity: complete
//! linux-source: vendor/linux/block/partitions/osf.c
//! test-origin: linux:vendor/linux/block/partitions/osf.c
//! OSF/1 disklabel partition parser.

extern crate alloc;

use alloc::vec::Vec;

use super::Partition;

pub const MAX_OSF_PARTITIONS: usize = 18;
pub const DISKLABELMAGIC: u32 = 0x8256_4557;
pub const DISKLABEL_OFFSET: usize = 64;
pub const DISKLABEL_MAGIC2_OFFSET: usize = 132;
pub const DISKLABEL_NPARTITIONS_OFFSET: usize = 138;
pub const DISKLABEL_PARTITIONS_OFFSET: usize = 148;
pub const DISKLABEL_PARTITION_SIZE: usize = 16;

pub fn has_osf_disklabel(sector: &[u8]) -> bool {
    osf_partition_count(sector).is_some()
}

pub fn parse_osf_sector(sector: &[u8], limit: usize) -> Option<Vec<Partition>> {
    let npartitions = osf_partition_count(sector)?;
    let mut partitions = Vec::new();
    let mut slot = 1usize;

    for index in 0..npartitions {
        if slot == limit {
            break;
        }
        let offset = DISKLABEL_OFFSET
            .checked_add(DISKLABEL_PARTITIONS_OFFSET)?
            .checked_add(index.checked_mul(DISKLABEL_PARTITION_SIZE)?)?;
        let size = le32(sector, offset)? as u64;
        let start = le32(sector, offset + 4)? as u64;
        if size != 0 {
            partitions.push(Partition {
                number: slot as u32,
                start_sector: start,
                nr_sectors: size,
                type_guid: None,
                type_byte: None,
            });
        }
        slot += 1;
    }

    Some(partitions)
}

fn osf_partition_count(sector: &[u8]) -> Option<usize> {
    if sector.len() < 512 {
        return None;
    }
    if le32(sector, DISKLABEL_OFFSET)? != DISKLABELMAGIC {
        return None;
    }
    if le32(sector, DISKLABEL_OFFSET + DISKLABEL_MAGIC2_OFFSET)? != DISKLABELMAGIC {
        return None;
    }
    let npartitions = le16(sector, DISKLABEL_OFFSET + DISKLABEL_NPARTITIONS_OFFSET)? as usize;
    (npartitions <= MAX_OSF_PARTITIONS).then_some(npartitions)
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
    fn osf_partition_parser_matches_linux_disklabel_layout() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/partitions/osf.c"
        ));
        assert!(source.contains("#define MAX_OSF_PARTITIONS 18"));
        assert!(source.contains("#define DISKLABELMAGIC (0x82564557UL)"));
        assert!(source.contains("label = (struct disklabel *) (data+64);"));
        assert!(source.contains("if (le32_to_cpu(label->d_magic) != DISKLABELMAGIC)"));
        assert!(source.contains("if (le32_to_cpu(label->d_magic2) != DISKLABELMAGIC)"));
        assert!(source.contains("npartitions = le16_to_cpu(label->d_npartitions);"));
        assert!(source.contains("put_partition(state, slot"));
        assert!(source.contains("le32_to_cpu(partition->p_offset)"));
        assert!(source.contains("le32_to_cpu(partition->p_size)"));
        assert!(source.contains("seq_buf_puts(&state->pp_buf, \"\\n\");"));

        let mut sector = [0u8; 512];
        write_le32(&mut sector, DISKLABEL_OFFSET, DISKLABELMAGIC);
        write_le32(
            &mut sector,
            DISKLABEL_OFFSET + DISKLABEL_MAGIC2_OFFSET,
            DISKLABELMAGIC,
        );
        write_le16(
            &mut sector,
            DISKLABEL_OFFSET + DISKLABEL_NPARTITIONS_OFFSET,
            3,
        );
        let part0 = DISKLABEL_OFFSET + DISKLABEL_PARTITIONS_OFFSET;
        write_le32(&mut sector, part0, 100);
        write_le32(&mut sector, part0 + 4, 63);
        let part1 = part0 + DISKLABEL_PARTITION_SIZE;
        write_le32(&mut sector, part1, 0);
        write_le32(&mut sector, part1 + 4, 1000);
        let part2 = part1 + DISKLABEL_PARTITION_SIZE;
        write_le32(&mut sector, part2, 200);
        write_le32(&mut sector, part2 + 4, 4096);

        assert!(has_osf_disklabel(&sector));
        let partitions = parse_osf_sector(&sector, 16).expect("osf disklabel");
        assert_eq!(partitions.len(), 2);
        assert_eq!(partitions[0].number, 1);
        assert_eq!(partitions[0].start_sector, 63);
        assert_eq!(partitions[0].nr_sectors, 100);
        assert_eq!(partitions[1].number, 3);
        assert_eq!(partitions[1].start_sector, 4096);
        assert_eq!(partitions[1].nr_sectors, 200);

        sector[DISKLABEL_OFFSET] = 0;
        assert!(!has_osf_disklabel(&sector));
        assert_eq!(parse_osf_sector(&sector, 16), None);
    }
}
