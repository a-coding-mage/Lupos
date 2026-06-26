//! linux-parity: complete
//! linux-source: vendor/linux/lib/union_find.c
//! test-origin: linux:vendor/linux/lib/union_find.c
//! Union-find nodes with path compression and union by rank.

use core::ptr;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UfNode {
    pub parent: *mut UfNode,
    pub rank: u32,
}

impl UfNode {
    pub const fn uninit() -> Self {
        Self {
            parent: ptr::null_mut(),
            rank: 0,
        }
    }
}

pub unsafe fn uf_node_init(node: *mut UfNode) {
    if !node.is_null() {
        unsafe {
            (*node).parent = node;
            (*node).rank = 0;
        }
    }
}

pub unsafe extern "C" fn uf_find(mut node: *mut UfNode) -> *mut UfNode {
    if node.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        while (*node).parent != node {
            let parent = (*node).parent;
            (*node).parent = (*parent).parent;
            node = parent;
        }
    }
    node
}

pub unsafe extern "C" fn uf_union(node1: *mut UfNode, node2: *mut UfNode) {
    if node1.is_null() || node2.is_null() {
        return;
    }

    let root1 = unsafe { uf_find(node1) };
    let root2 = unsafe { uf_find(node2) };
    if root1 == root2 {
        return;
    }

    unsafe {
        if (*root1).rank < (*root2).rank {
            (*root1).parent = root2;
        } else if (*root1).rank > (*root2).rank {
            (*root2).parent = root1;
        } else {
            (*root2).parent = root1;
            (*root1).rank += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_find_matches_linux_path_compression_and_rank() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/union_find.c"
        ));
        assert!(source.contains("struct uf_node *uf_find(struct uf_node *node)"));
        assert!(source.contains("node->parent = parent->parent;"));
        assert!(source.contains("void uf_union(struct uf_node *node1, struct uf_node *node2)"));
        assert!(source.contains("root2->parent = root1;"));
        assert!(source.contains("root1->rank++;"));

        let mut nodes = [UfNode::uninit(); 3];
        for node in &mut nodes {
            unsafe { uf_node_init(node) };
        }

        unsafe { uf_union(&mut nodes[0], &mut nodes[1]) };
        assert_eq!(unsafe { uf_find(&mut nodes[0]) }, unsafe {
            uf_find(&mut nodes[1])
        });
        assert_eq!(unsafe { (*uf_find(&mut nodes[0])).rank }, 1);

        nodes[2].parent = &mut nodes[1];
        let root = unsafe { uf_find(&mut nodes[0]) };
        assert_eq!(unsafe { uf_find(&mut nodes[2]) }, root);
        assert_eq!(nodes[2].parent, root);
    }
}
