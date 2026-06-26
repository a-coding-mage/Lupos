//! linux-parity: complete
//! linux-source: vendor/linux/block/partitions/ultrix.c
//! test-origin: linux:vendor/linux/block/partitions/ultrix.c
//! Ultrix disklabel partition parser.

extern crate alloc;

use alloc::vec::Vec;

use super::Partition;

pub const ULTRIX_LABEL_SECTOR: u64 = 31;
pub const ULTRIX_LABEL_SIZE: usize = 72;
pub const ULTRIX_LABEL_OFFSET: usize = 512 - ULTRIX_LABEL_SIZE;
pub const PT_MAGIC: i32 = 0x032957;
pub const PT_VALID: i32 = 1;
pub const ULTRIX_PARTITION_COUNT: usize = 8;

pub fn parse_ultrix_sector(sector: &[u8]) -> Option<Vec<Partition>> {
    if sector.len() < 512 {
        return None;
    }
    let label = &sector[ULTRIX_LABEL_OFFSET..512];
    let magic = read_i32_le(label, 0)?;
    let valid = read_i32_le(label, 4)?;
    if magic != PT_MAGIC || valid != PT_VALID {
        return None;
    }

    let mut partitions = Vec::new();
    let mut offset = 8usize;
    for number in 1..=ULTRIX_PARTITION_COUNT {
        let nblocks = read_i32_le(label, offset)?;
        let blkoff = read_u32_le(label, offset + 4)?;
        if nblocks > 0 {
            partitions.push(Partition {
                number: number as u32,
                start_sector: blkoff as u64,
                nr_sectors: nblocks as u64,
                type_guid: None,
                type_byte: None,
            });
        }
        offset += 8;
    }
    Some(partitions)
}

fn read_i32_le(bytes: &[u8], offset: usize) -> Option<i32> {
    Some(i32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_i32(bytes: &mut [u8], offset: usize, value: i32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    #[test]
    fn ultrix_partition_parser_matches_linux_label_location_and_magic() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/partitions/ultrix.c"
        ));
        assert!(source.contains("read_part_sector(state, (16384 - sizeof(*label))/512"));
        assert!(source.contains("#define PT_MAGIC\t0x032957"));
        assert!(source.contains("#define PT_VALID\t1"));
        assert!(
            source.contains("label = (struct ultrix_disklabel *)(data + 512 - sizeof(*label));")
        );
        assert!(source.contains("for (i=0; i<8; i++)"));
        assert!(source.contains("put_partition(state, i+1"));
        assert!(source.contains("seq_buf_puts(&state->pp_buf, \"\\n\");"));

        assert_eq!(ULTRIX_LABEL_SECTOR, 31);
        assert_eq!(ULTRIX_LABEL_OFFSET, 440);

        let mut sector = [0u8; 512];
        {
            let label = &mut sector[ULTRIX_LABEL_OFFSET..];
            write_i32(label, 0, PT_MAGIC);
            write_i32(label, 4, PT_VALID);
            write_i32(label, 8, 10);
            write_u32(label, 12, 42);
            write_i32(label, 16, 0);
        }
        let partitions = parse_ultrix_sector(&sector).expect("valid ultrix label");
        assert_eq!(partitions.len(), 1);
        assert_eq!(partitions[0].number, 1);
        assert_eq!(partitions[0].start_sector, 42);
        assert_eq!(partitions[0].nr_sectors, 10);

        sector[ULTRIX_LABEL_OFFSET] = 0;
        assert_eq!(parse_ultrix_sector(&sector), None);
    }
}
