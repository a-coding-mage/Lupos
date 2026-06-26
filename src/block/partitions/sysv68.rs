//! linux-parity: complete
//! linux-source: vendor/linux/block/partitions/sysv68.c
//! test-origin: linux:vendor/linux/block/partitions/sysv68.c
//! Motorola SYSV68 disk slice table parser.

extern crate alloc;

use alloc::vec::Vec;

use super::Partition;

pub const SYSV68_VOLUME_ID_OFFSET: usize = 248;
pub const SYSV68_VOLUME_ID: &[u8; 8] = b"MOTOROLA";
pub const SYSV68_CONFIG_OFFSET: usize = 256;
pub const SYSV68_SLICE_TABLE_BLOCK_OFFSET: usize = SYSV68_CONFIG_OFFSET + 128;
pub const SYSV68_SLICE_COUNT_OFFSET: usize = SYSV68_CONFIG_OFFSET + 132;
pub const SYSV68_SLICE_SIZE: usize = 8;

pub fn has_sysv68_volume_id(block0: &[u8]) -> bool {
    block0.get(SYSV68_VOLUME_ID_OFFSET..SYSV68_VOLUME_ID_OFFSET + SYSV68_VOLUME_ID.len())
        == Some(SYSV68_VOLUME_ID)
}

pub fn sysv68_slice_table_block(block0: &[u8]) -> Option<u32> {
    if !has_sysv68_volume_id(block0) {
        return None;
    }
    be32(block0, SYSV68_SLICE_TABLE_BLOCK_OFFSET)
}

pub fn sysv68_slice_count(block0: &[u8]) -> Option<usize> {
    if !has_sysv68_volume_id(block0) {
        return None;
    }
    be16(block0, SYSV68_SLICE_COUNT_OFFSET).map(usize::from)
}

pub fn parse_sysv68_slice_table(slice_table: &[u8], slices: usize, limit: usize) -> Vec<Partition> {
    let mut partitions = Vec::new();
    let usable_slices = slices.saturating_sub(1);
    let mut slot = 1usize;

    for index in 0..usable_slices {
        if slot == limit {
            break;
        }
        let Some(offset) = index.checked_mul(SYSV68_SLICE_SIZE) else {
            break;
        };
        let Some(nblocks) = be32(slice_table, offset) else {
            break;
        };
        let Some(blkoff) = be32(slice_table, offset + 4) else {
            break;
        };
        if nblocks != 0 {
            partitions.push(Partition {
                number: slot as u32,
                start_sector: u64::from(blkoff),
                nr_sectors: u64::from(nblocks),
                type_guid: None,
                type_byte: None,
            });
        }
        slot += 1;
    }

    partitions
}

fn be16(buf: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes([
        *buf.get(offset)?,
        *buf.get(offset + 1)?,
    ]))
}

fn be32(buf: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes([
        *buf.get(offset)?,
        *buf.get(offset + 1)?,
        *buf.get(offset + 2)?,
        *buf.get(offset + 3)?,
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_be16(buf: &mut [u8], offset: usize, value: u16) {
        buf[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
    }

    fn write_be32(buf: &mut [u8], offset: usize, value: u32) {
        buf[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    #[test]
    fn sysv68_parser_matches_linux_volume_and_slice_layout() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/partitions/sysv68.c"
        ));
        assert!(source.contains("u8\tvid_unused[248];"));
        assert!(source.contains("u8\tvid_mac[8];\t/* ASCII string \"MOTOROLA\" */"));
        assert!(source.contains("__be32\tios_slcblk;"));
        assert!(source.contains("__be16\tios_slccnt;"));
        assert!(source.contains("struct slice"));
        assert!(source.contains("memcmp(b->dk_vid.vid_mac, \"MOTOROLA\""));
        assert!(source.contains("slices -= 1; /* last slice is the whole disk */"));
        assert!(source.contains("put_partition(state, slot"));
        assert!(source.contains("be32_to_cpu(slice->blkoff)"));
        assert!(source.contains("be32_to_cpu(slice->nblocks)"));
        assert!(source.contains("seq_buf_puts(&state->pp_buf, \"\\n\");"));

        let mut block0 = [0u8; 512];
        block0[SYSV68_VOLUME_ID_OFFSET..SYSV68_VOLUME_ID_OFFSET + SYSV68_VOLUME_ID.len()]
            .copy_from_slice(SYSV68_VOLUME_ID);
        write_be32(&mut block0, SYSV68_SLICE_TABLE_BLOCK_OFFSET, 9);
        write_be16(&mut block0, SYSV68_SLICE_COUNT_OFFSET, 4);

        assert!(has_sysv68_volume_id(&block0));
        assert_eq!(sysv68_slice_table_block(&block0), Some(9));
        assert_eq!(sysv68_slice_count(&block0), Some(4));

        let mut table = [0u8; 32];
        write_be32(&mut table, 0, 100);
        write_be32(&mut table, 4, 63);
        write_be32(&mut table, 8, 0);
        write_be32(&mut table, 12, 1000);
        write_be32(&mut table, 16, 200);
        write_be32(&mut table, 20, 4096);

        let partitions = parse_sysv68_slice_table(&table, 4, 16);
        assert_eq!(partitions.len(), 2);
        assert_eq!(partitions[0].number, 1);
        assert_eq!(partitions[0].start_sector, 63);
        assert_eq!(partitions[0].nr_sectors, 100);
        assert_eq!(partitions[1].number, 3);
        assert_eq!(partitions[1].start_sector, 4096);
        assert_eq!(partitions[1].nr_sectors, 200);

        block0[SYSV68_VOLUME_ID_OFFSET] = b'X';
        assert!(!has_sysv68_volume_id(&block0));
        assert_eq!(sysv68_slice_count(&block0), None);
    }
}
