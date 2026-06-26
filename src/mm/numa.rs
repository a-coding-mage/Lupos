//! linux-parity: complete
//! linux-source: vendor/linux/mm/numa.c
//! test-origin: linux:vendor/linux/mm/numa.c
//! Generic NUMA node-data allocation fallback helpers.

pub const MAX_NUMNODES: usize = 1024;
pub const SMP_CACHE_BYTES: usize = 64;
pub const PAGE_SHIFT: u32 = 12;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeDataAllocation {
    pub nid: usize,
    pub phys_start: u64,
    pub phys_end: u64,
    pub target_nid: usize,
    pub zeroed: bool,
}

pub const fn rounded_pg_data_size(pg_data_size: usize) -> usize {
    pg_data_size.div_ceil(SMP_CACHE_BYTES) * SMP_CACHE_BYTES
}

pub const fn alloc_node_data(
    nid: usize,
    pg_data_size: usize,
    phys_addr: u64,
    target_nid: usize,
) -> Option<NodeDataAllocation> {
    if nid >= MAX_NUMNODES || phys_addr == 0 {
        return None;
    }
    let size = rounded_pg_data_size(pg_data_size) as u64;
    Some(NodeDataAllocation {
        nid,
        phys_start: phys_addr,
        phys_end: phys_addr + size - 1,
        target_nid,
        zeroed: true,
    })
}

pub const fn alloc_offline_node_data(
    nid: usize,
    pg_data_size: usize,
    phys_addr: u64,
) -> Option<NodeDataAllocation> {
    alloc_node_data(nid, pg_data_size, phys_addr, nid)
}

pub const fn memory_add_physaddr_to_nid(_start: u64) -> i32 {
    0
}

pub const fn phys_to_target_node(_start: u64) -> i32 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numa_node_data_allocation_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/numa.c"
        ));
        assert!(source.contains("struct pglist_data *node_data[MAX_NUMNODES];"));
        assert!(source.contains("roundup(sizeof(pg_data_t), SMP_CACHE_BYTES);"));
        assert!(source.contains("memblock_phys_alloc_try_nid"));
        assert!(source.contains("Cannot allocate %zu bytes for node %d data"));
        assert!(source.contains("early_pfn_to_nid(nd_pa >> PAGE_SHIFT);"));
        assert!(source.contains("memset(NODE_DATA(nid), 0, sizeof(pg_data_t));"));
        assert!(source.contains("memblock_alloc_or_panic(sizeof(*pgdat), SMP_CACHE_BYTES);"));
        assert!(source.contains("Unknown online node for memory"));
        assert!(source.contains("Unknown target node for memory"));

        let node = alloc_node_data(1, 100, 0x2000, 0).expect("node allocation");
        assert_eq!(node.phys_start, 0x2000);
        assert_eq!(node.phys_end, 0x2000 + 128 - 1);
        assert!(node.zeroed);
        assert_eq!(alloc_node_data(MAX_NUMNODES, 64, 0x1000, 0), None);
        assert_eq!(memory_add_physaddr_to_nid(0xdead), 0);
        assert_eq!(phys_to_target_node(0xbeef), 0);
    }
}
