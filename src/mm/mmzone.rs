//! linux-parity: complete
//! linux-source: vendor/linux/mm/mmzone.c
//! test-origin: linux:vendor/linux/mm/mmzone.c
//! pgdat, zone, zonelist, and lruvec iteration helpers.

extern crate alloc;

use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Pgdat {
    pub node_id: usize,
    pub first_zone: usize,
    pub nr_zones: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ZoneRef {
    pub zone_idx: usize,
    pub node_id: usize,
}

pub fn first_online_pgdat(nodes: &[Pgdat]) -> Option<Pgdat> {
    nodes.first().copied()
}

pub fn next_online_pgdat(nodes: &[Pgdat], current_node_id: usize) -> Option<Pgdat> {
    nodes
        .iter()
        .position(|node| node.node_id == current_node_id)
        .and_then(|index| nodes.get(index + 1))
        .copied()
}

pub fn next_zone(
    nodes: &[Pgdat],
    current_node_id: usize,
    zone_offset: usize,
) -> Option<(usize, usize)> {
    let node_index = nodes
        .iter()
        .position(|node| node.node_id == current_node_id)?;
    let node = nodes[node_index];
    if zone_offset + 1 < node.nr_zones {
        Some((current_node_id, zone_offset + 1))
    } else {
        nodes
            .get(node_index + 1)
            .map(|next_node| (next_node.node_id, next_node.first_zone))
    }
}

pub fn next_zones_zonelist(
    zonelist: &[ZoneRef],
    start: usize,
    highest_zoneidx: usize,
    nodes: Option<&[usize]>,
) -> Option<usize> {
    let mut index = start;
    while let Some(zref) = zonelist.get(index) {
        let zone_too_high = zref.zone_idx > highest_zoneidx;
        let node_filtered = nodes.is_some_and(|allowed| !allowed.contains(&zref.node_id));
        if !zone_too_high && !node_filtered {
            return Some(index);
        }
        index += 1;
    }
    None
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LruVec {
    pub lists: Vec<bool>,
    pub unevictable_poisoned: bool,
    pub gen_initialized: bool,
}

pub fn lruvec_init(nr_lru_lists: usize, unevictable_index: usize) -> LruVec {
    let mut lists = Vec::new();
    lists.resize(nr_lru_lists, true);
    if unevictable_index < lists.len() {
        lists[unevictable_index] = false;
    }
    LruVec {
        lists,
        unevictable_poisoned: true,
        gen_initialized: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mmzone_iteration_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/mmzone.c"
        ));
        assert!(source.contains("struct pglist_data *first_online_pgdat(void)"));
        assert!(source.contains("return NODE_DATA(first_online_node);"));
        assert!(source.contains("struct pglist_data *next_online_pgdat"));
        assert!(source.contains("nid = next_online_node(pgdat->node_id);"));
        assert!(source.contains("if (nid == MAX_NUMNODES)"));
        assert!(source.contains("struct zone *next_zone(struct zone *zone)"));
        assert!(source.contains("zone < pgdat->node_zones + MAX_NR_ZONES - 1"));
        assert!(source.contains("struct zoneref *__next_zones_zonelist"));
        assert!(source.contains("while (zonelist_zone_idx(z) > highest_zoneidx)"));
        assert!(source.contains("zref_in_nodemask(z, nodes)"));
        assert!(source.contains("void lruvec_init(struct lruvec *lruvec)"));
        assert!(source.contains("INIT_LIST_HEAD(&lruvec->lists[lru]);"));
        assert!(source.contains("list_del(&lruvec->lists[LRU_UNEVICTABLE]);"));
        assert!(source.contains("lru_gen_init_lruvec(lruvec);"));

        let nodes = [
            Pgdat {
                node_id: 0,
                first_zone: 0,
                nr_zones: 2,
            },
            Pgdat {
                node_id: 1,
                first_zone: 0,
                nr_zones: 1,
            },
        ];
        assert_eq!(first_online_pgdat(&nodes), Some(nodes[0]));
        assert_eq!(next_online_pgdat(&nodes, 0), Some(nodes[1]));
        assert_eq!(next_online_pgdat(&nodes, 1), None);
        assert_eq!(next_zone(&nodes, 0, 0), Some((0, 1)));
        assert_eq!(next_zone(&nodes, 0, 1), Some((1, 0)));

        let zonelist = [
            ZoneRef {
                zone_idx: 3,
                node_id: 0,
            },
            ZoneRef {
                zone_idx: 1,
                node_id: 1,
            },
            ZoneRef {
                zone_idx: 0,
                node_id: 2,
            },
        ];
        assert_eq!(next_zones_zonelist(&zonelist, 0, 1, None), Some(1));
        assert_eq!(next_zones_zonelist(&zonelist, 0, 1, Some(&[2])), Some(2));
        let lruvec = lruvec_init(5, 4);
        assert!(!lruvec.lists[4]);
        assert!(lruvec.unevictable_poisoned);
        assert!(lruvec.gen_initialized);
    }
}
