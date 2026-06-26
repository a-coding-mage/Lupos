//! linux-parity: complete
//! linux-source: vendor/linux/virt/kvm/binary_stats.c
//! test-origin: linux:vendor/linux/virt/kvm/binary_stats.c
//! KVM binary statistics file layout and read cursor semantics.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::EFAULT;

pub const KVM_STATS_NAME_SIZE: usize = 48;
pub const KVM_STATS_HEADER_SIZE: usize = 24;
pub const KVM_STATS_DESC_SIZE: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmStatsHeader {
    pub flags: u32,
    pub name_size: u32,
    pub num_desc: u32,
    pub id_offset: u32,
    pub desc_offset: u32,
    pub data_offset: u32,
}

impl KvmStatsHeader {
    pub const fn new(num_desc: u32, data_size: usize) -> Self {
        let desc_offset = KVM_STATS_HEADER_SIZE + KVM_STATS_NAME_SIZE;
        Self {
            flags: 0,
            name_size: KVM_STATS_NAME_SIZE as u32,
            num_desc,
            id_offset: KVM_STATS_HEADER_SIZE as u32,
            desc_offset: desc_offset as u32,
            data_offset: (desc_offset + num_desc as usize * KVM_STATS_DESC_SIZE) as u32,
        }
    }

    pub const fn total_size(&self, stats_size: usize) -> usize {
        KVM_STATS_HEADER_SIZE
            + KVM_STATS_NAME_SIZE
            + self.num_desc as usize * KVM_STATS_DESC_SIZE
            + stats_size
    }

    pub fn to_le_bytes(self) -> [u8; KVM_STATS_HEADER_SIZE] {
        let mut out = [0u8; KVM_STATS_HEADER_SIZE];
        write_u32(&mut out[0..4], self.flags);
        write_u32(&mut out[4..8], self.name_size);
        write_u32(&mut out[8..12], self.num_desc);
        write_u32(&mut out[12..16], self.id_offset);
        write_u32(&mut out[16..20], self.desc_offset);
        write_u32(&mut out[20..24], self.data_offset);
        out
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KvmStatsDesc {
    pub flags: u32,
    pub exponent: i16,
    pub size: u16,
    pub offset: u32,
    pub bucket_size: u32,
    pub name: [u8; KVM_STATS_NAME_SIZE],
}

impl KvmStatsDesc {
    pub fn new(name: &str, offset: u32, size: u16) -> Self {
        let mut padded = [0u8; KVM_STATS_NAME_SIZE];
        let bytes = name.as_bytes();
        let len = bytes.len().min(KVM_STATS_NAME_SIZE.saturating_sub(1));
        padded[..len].copy_from_slice(&bytes[..len]);
        Self {
            flags: 0,
            exponent: 0,
            size,
            offset,
            bucket_size: 0,
            name: padded,
        }
    }

    pub fn to_le_bytes(&self) -> [u8; KVM_STATS_DESC_SIZE] {
        let mut out = [0u8; KVM_STATS_DESC_SIZE];
        write_u32(&mut out[0..4], self.flags);
        out[4..6].copy_from_slice(&self.exponent.to_le_bytes());
        out[6..8].copy_from_slice(&self.size.to_le_bytes());
        write_u32(&mut out[8..12], self.offset);
        write_u32(&mut out[12..16], self.bucket_size);
        out[16..].copy_from_slice(&self.name);
        out
    }
}

pub fn pack_descs(descs: &[KvmStatsDesc]) -> Vec<u8> {
    let mut out = Vec::with_capacity(descs.len() * KVM_STATS_DESC_SIZE);
    for desc in descs {
        out.extend_from_slice(&desc.to_le_bytes());
    }
    out
}

pub fn kvm_stats_read(
    id: &str,
    header: &KvmStatsHeader,
    desc: &[u8],
    stats: &[u8],
    offset: &mut usize,
    user_buffer: &mut [u8],
) -> usize {
    let size_desc = header.num_desc as usize * KVM_STATS_DESC_SIZE;
    let file_len = kvm_stats_file_len(size_desc, stats.len());
    if *offset >= file_len || user_buffer.is_empty() {
        return 0;
    }

    let len = (file_len - *offset).min(user_buffer.len());
    user_buffer[..len].fill(0);

    let header_bytes = header.to_le_bytes();
    let mut id_bytes = [0u8; KVM_STATS_NAME_SIZE];
    let id_src = id.as_bytes();
    let id_len = id_src.len().min(KVM_STATS_NAME_SIZE);
    id_bytes[..id_len].copy_from_slice(&id_src[..id_len]);

    copy_segment(user_buffer, *offset, 0, &header_bytes);
    copy_segment(user_buffer, *offset, header.id_offset as usize, &id_bytes);
    copy_segment(
        user_buffer,
        *offset,
        header.desc_offset as usize,
        &desc[..desc.len().min(size_desc)],
    );
    copy_segment(user_buffer, *offset, header.data_offset as usize, stats);

    *offset += len;
    len
}

pub fn kvm_stats_read_checked(
    id: &str,
    header: &KvmStatsHeader,
    desc: &[u8],
    stats: &[u8],
    offset: &mut usize,
    user_buffer: &mut [u8],
    copy_to_user_ok: bool,
) -> Result<usize, i32> {
    let size_desc = header.num_desc as usize * KVM_STATS_DESC_SIZE;
    let file_len = kvm_stats_file_len(size_desc, stats.len());
    if *offset >= file_len || user_buffer.is_empty() {
        return Ok(0);
    }
    if !copy_to_user_ok {
        return Err(-EFAULT);
    }
    Ok(kvm_stats_read(id, header, desc, stats, offset, user_buffer))
}

const fn kvm_stats_file_len(size_desc: usize, stats_len: usize) -> usize {
    KVM_STATS_NAME_SIZE + KVM_STATS_HEADER_SIZE + size_desc + stats_len
}

fn copy_segment(out: &mut [u8], file_offset: usize, segment_offset: usize, segment: &[u8]) {
    let out_end = file_offset.saturating_add(out.len());
    let segment_end = segment_offset.saturating_add(segment.len());
    let start = file_offset.max(segment_offset);
    let end = out_end.min(segment_end);
    if start >= end {
        return;
    }

    let dst = start - file_offset;
    let src = start - segment_offset;
    out[dst..dst + (end - start)].copy_from_slice(&segment[src..src + (end - start)]);
}

fn write_u32(out: &mut [u8], value: u32) {
    out.copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kvm_stats_read_shape_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/virt/kvm/binary_stats.c"
        ));
        assert!(source.contains("ssize_t kvm_stats_read(char *id"));
        assert!(source.contains("size_header = sizeof(*header);"));
        assert!(source.contains("size_desc = header->num_desc * sizeof(*desc);"));
        assert!(
            source.contains(
                "len = KVM_STATS_NAME_SIZE + size_header + size_desc + size_stats - pos;"
            )
        );
        assert!(source.contains("src = id + pos - header->id_offset;"));
        assert!(source.contains("if (copy_to_user(dest, src, copylen))"));
        assert!(source.contains("return -EFAULT;"));
        assert!(source.contains("src = (void *)desc + pos - header->desc_offset;"));
        assert!(source.contains("src = stats + pos - header->data_offset;"));
        assert!(source.contains("*offset = pos;"));
    }

    #[test]
    fn read_cursor_can_cross_header_id_desc_and_stats() {
        let desc = KvmStatsDesc::new("exits", 0, 1);
        let desc_bytes = pack_descs(&[desc]);
        let header = KvmStatsHeader::new(1, 8);
        let mut offset = 20;
        let mut out = [0u8; 96];
        let n = kvm_stats_read(
            "vm0",
            &header,
            &desc_bytes,
            &[9, 8, 7, 6],
            &mut offset,
            &mut out,
        );

        assert_eq!(n, 96);
        assert_eq!(offset, 116);
        assert_eq!(&out[4..7], b"vm0");
        let desc_name = header.desc_offset as usize + 16 - 20;
        assert_eq!(&out[desc_name..desc_name + 5], b"exits");
    }

    #[test]
    fn read_returns_zero_at_eof() {
        let header = KvmStatsHeader::new(0, 4);
        let mut offset = header.total_size(4);
        let mut out = [0u8; 8];
        assert_eq!(
            kvm_stats_read("vm", &header, &[], &[1, 2, 3, 4], &mut offset, &mut out),
            0
        );
    }

    #[test]
    fn read_reports_copy_to_user_fault_before_advancing_offset() {
        let header = KvmStatsHeader::new(0, 4);
        let mut offset = 0;
        let mut out = [0u8; 8];
        assert_eq!(
            kvm_stats_read_checked(
                "vm",
                &header,
                &[],
                &[1, 2, 3, 4],
                &mut offset,
                &mut out,
                false,
            ),
            Err(-EFAULT)
        );
        assert_eq!(offset, 0);
    }
}
