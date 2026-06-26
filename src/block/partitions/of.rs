//! linux-parity: complete
//! linux-source: vendor/linux/block/partitions/of.c
//! test-origin: linux:vendor/linux/block/partitions/of.c
//! Open Firmware fixed-partitions parser helpers.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::include::uapi::errno::EINVAL;

use super::Partition;

pub const SECTOR_SIZE: u64 = 512;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OfPartitionNode {
    pub offset_bytes: u64,
    pub size_bytes: u64,
    pub read_only: bool,
    pub label: Option<String>,
    pub name: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OfPartition {
    pub partition: Partition,
    pub read_only: bool,
    pub volname: String,
}

pub const fn validate_of_partition(offset_bytes: u64, size_bytes: u64) -> Result<(), i32> {
    if offset_bytes % SECTOR_SIZE != 0 {
        return Err(EINVAL);
    }
    if size_bytes == 0 || size_bytes % SECTOR_SIZE != 0 {
        return Err(EINVAL);
    }
    Ok(())
}

pub fn parse_of_partitions(
    nodes: &[OfPartitionNode],
    limit: usize,
) -> Result<Vec<OfPartition>, i32> {
    for node in nodes {
        validate_of_partition(node.offset_bytes, node.size_bytes)?;
    }

    let mut out = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        let slot = index + 1;
        if slot >= limit {
            break;
        }
        let volname = node
            .label
            .as_ref()
            .or(node.name.as_ref())
            .cloned()
            .unwrap_or_default();
        out.push(OfPartition {
            partition: Partition {
                number: slot as u32,
                start_sector: node.offset_bytes / SECTOR_SIZE,
                nr_sectors: node.size_bytes / SECTOR_SIZE,
                type_guid: None,
                type_byte: None,
            },
            read_only: node.read_only,
            volname,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn open_firmware_partitions_match_linux_sector_and_label_rules() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/partitions/of.c"
        ));
        assert!(source.contains("static int validate_of_partition"));
        assert!(source.contains("of_get_property(np, \"reg\", &len);"));
        assert!(source.contains("len / sizeof(*reg) != a_cells + s_cells"));
        assert!(source.contains("offset % SECTOR_SIZE"));
        assert!(source.contains("!size || size % SECTOR_SIZE"));
        assert!(source.contains("put_partition(state, slot, offset, size);"));
        assert!(source.contains("of_property_read_bool(np, \"read-only\")"));
        assert!(source.contains("of_get_property(np, \"label\", &len);"));
        assert!(source.contains("of_get_property(np, \"name\", &len);"));
        assert!(source.contains("of_device_is_compatible(partitions_np, \"fixed-partitions\")"));

        assert_eq!(validate_of_partition(0, 512), Ok(()));
        assert_eq!(validate_of_partition(1, 512), Err(EINVAL));
        assert_eq!(validate_of_partition(0, 0), Err(EINVAL));
        assert_eq!(validate_of_partition(0, 513), Err(EINVAL));

        let nodes = [
            OfPartitionNode {
                offset_bytes: 1024,
                size_bytes: 4096,
                read_only: true,
                label: Some("firmware".to_string()),
                name: Some("fallback".to_string()),
            },
            OfPartitionNode {
                offset_bytes: 8192,
                size_bytes: 512,
                read_only: false,
                label: None,
                name: Some("env".to_string()),
            },
        ];
        let parts = parse_of_partitions(&nodes, 16).expect("valid fixed partitions");
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].partition.start_sector, 2);
        assert_eq!(parts[0].partition.nr_sectors, 8);
        assert!(parts[0].read_only);
        assert_eq!(parts[0].volname, "firmware");
        assert_eq!(parts[1].volname, "env");
    }
}
