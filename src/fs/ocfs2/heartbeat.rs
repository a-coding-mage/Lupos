//! linux-parity: complete
//! linux-source: vendor/linux/fs/ocfs2/heartbeat.c
//! test-origin: linux:vendor/linux/fs/ocfs2/heartbeat.c
//! OCFS2 node-map heartbeat helpers.

pub const OCFS2_NODE_MAP_MAX_NODES: usize = 256;
pub const OCFS2_NODE_MAP_WORDS: usize = OCFS2_NODE_MAP_MAX_NODES / 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ocfs2NodeMap {
    pub num_nodes: u16,
    pub map: [u64; OCFS2_NODE_MAP_WORDS],
}

impl Ocfs2NodeMap {
    pub const fn new() -> Self {
        Self {
            num_nodes: OCFS2_NODE_MAP_MAX_NODES as u16,
            map: [0; OCFS2_NODE_MAP_WORDS],
        }
    }

    pub fn set_bit(&mut self, bit: i32) {
        if bit == -1 {
            return;
        }
        assert!((bit as u16) < self.num_nodes);
        let bit = bit as usize;
        self.map[bit / 64] |= 1u64 << (bit % 64);
    }

    pub fn clear_bit(&mut self, bit: i32) {
        if bit == -1 {
            return;
        }
        assert!((bit as u16) < self.num_nodes);
        let bit = bit as usize;
        self.map[bit / 64] &= !(1u64 << (bit % 64));
    }

    pub fn test_bit(&self, bit: usize) -> bool {
        assert!(bit < self.num_nodes as usize);
        self.map[bit / 64] & (1u64 << (bit % 64)) != 0
    }
}

impl Default for Ocfs2NodeMap {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ocfs2NodeDownAction {
    IgnoreUntilClusterConnects,
    StartRecoveryThread,
}

pub const fn ocfs2_do_node_down_action(
    local_node: u32,
    down_node: u32,
    has_cluster_connection: bool,
) -> Ocfs2NodeDownAction {
    assert!(local_node != down_node);
    if has_cluster_connection {
        Ocfs2NodeDownAction::StartRecoveryThread
    } else {
        Ocfs2NodeDownAction::IgnoreUntilClusterConnects
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ocfs2_heartbeat_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ocfs2/heartbeat.c"
        ));
        assert!(source.contains("#include <linux/bitmap.h>"));
        assert!(source.contains("#include \"heartbeat.h\""));
        assert!(source.contains("map->num_nodes = OCFS2_NODE_MAP_MAX_NODES;"));
        assert!(source.contains("bitmap_zero(map->map, OCFS2_NODE_MAP_MAX_NODES);"));
        assert!(source.contains("spin_lock_init(&osb->node_map_lock);"));
        assert!(source.contains("BUG_ON(osb->node_num == node_num);"));
        assert!(source.contains("if (!osb->cconn)"));
        assert!(source.contains("ocfs2_recovery_thread(osb, node_num);"));
        assert!(source.contains("if (bit==-1)"));
        assert!(source.contains("BUG_ON(bit >= map->num_nodes);"));
        assert!(source.contains("set_bit(bit, map->map);"));
        assert!(source.contains("clear_bit(bit, map->map);"));
        assert!(source.contains("ret = test_bit(bit, map->map);"));

        let mut map = Ocfs2NodeMap::new();
        assert_eq!(map.num_nodes, 256);
        assert!(!map.test_bit(42));
        map.set_bit(42);
        assert!(map.test_bit(42));
        map.clear_bit(42);
        assert!(!map.test_bit(42));
        map.set_bit(-1);
        assert_eq!(map.map, [0; OCFS2_NODE_MAP_WORDS]);
        assert_eq!(
            ocfs2_do_node_down_action(1, 2, false),
            Ocfs2NodeDownAction::IgnoreUntilClusterConnects
        );
        assert_eq!(
            ocfs2_do_node_down_action(1, 2, true),
            Ocfs2NodeDownAction::StartRecoveryThread
        );
    }
}
