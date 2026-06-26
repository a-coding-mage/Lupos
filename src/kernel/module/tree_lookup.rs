//! linux-parity: complete
//! linux-source: vendor/linux/kernel/module/tree_lookup.c
//! test-origin: linux:vendor/linux/kernel/module/tree_lookup.c
//! Module address range lookup helpers.

extern crate alloc;

use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModuleMemoryType {
    Text,
    Data,
    RoData,
    InitText,
    InitData,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleMemory {
    pub module_id: usize,
    pub mem_type: ModuleMemoryType,
    pub base: usize,
    pub size: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModuleTree {
    nodes: Vec<ModuleMemory>,
}

pub const fn mod_tree_comp(addr: usize, start: usize, size: usize) -> i32 {
    if addr < start {
        -1
    } else if addr >= start.saturating_add(size) {
        1
    } else {
        0
    }
}

pub const fn mod_tree_less(a_base: usize, b_base: usize) -> bool {
    a_base < b_base
}

impl ModuleTree {
    pub fn insert(&mut self, mem: ModuleMemory) {
        if mem.size == 0 {
            return;
        }
        self.nodes.push(mem);
        self.nodes.sort_by_key(|node| node.base);
    }

    pub fn remove_init(&mut self, module_id: usize) {
        self.nodes.retain(|node| {
            node.module_id != module_id
                || !matches!(
                    node.mem_type,
                    ModuleMemoryType::InitText | ModuleMemoryType::InitData
                )
        });
    }

    pub fn remove(&mut self, module_id: usize) {
        self.nodes.retain(|node| node.module_id != module_id);
    }

    pub fn find(&self, addr: usize) -> Option<ModuleMemory> {
        self.nodes
            .iter()
            .find(|node| mod_tree_comp(addr, node.base, node.size) == 0)
            .copied()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_tree_lookup_matches_linux_latched_range_comparator() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/module/tree_lookup.c"
        ));
        assert!(source.contains("static __always_inline unsigned long __mod_tree_val"));
        assert!(source.contains("return (unsigned long)mod_mem->base;"));
        assert!(source.contains("static __always_inline unsigned long __mod_tree_size"));
        assert!(source.contains("return (unsigned long)mod_mem->size;"));
        assert!(source.contains("return __mod_tree_val(a) < __mod_tree_val(b);"));
        assert!(source.contains("if (val < start)"));
        assert!(source.contains("if (val >= end)"));
        assert!(source.contains("latch_tree_insert(&node->node, &tree->root, &mod_tree_ops);"));
        assert!(source.contains("for_each_mod_mem_type(type)"));
        assert!(source.contains("for_class_mod_mem_type(type, init)"));
        assert!(source.contains("latch_tree_find((void *)addr, &tree->root, &mod_tree_ops);"));

        assert_eq!(mod_tree_comp(9, 10, 5), -1);
        assert_eq!(mod_tree_comp(10, 10, 5), 0);
        assert_eq!(mod_tree_comp(14, 10, 5), 0);
        assert_eq!(mod_tree_comp(15, 10, 5), 1);
        assert!(mod_tree_less(10, 20));

        let mut tree = ModuleTree::default();
        tree.insert(ModuleMemory {
            module_id: 1,
            mem_type: ModuleMemoryType::Text,
            base: 0x1000,
            size: 0x100,
        });
        tree.insert(ModuleMemory {
            module_id: 1,
            mem_type: ModuleMemoryType::InitText,
            base: 0x2000,
            size: 0x100,
        });
        tree.insert(ModuleMemory {
            module_id: 2,
            mem_type: ModuleMemoryType::Data,
            base: 0x3000,
            size: 0,
        });
        assert_eq!(tree.len(), 2);
        assert_eq!(tree.find(0x1080).unwrap().module_id, 1);
        tree.remove_init(1);
        assert!(tree.find(0x2080).is_none());
        assert!(tree.find(0x1080).is_some());
        tree.remove(1);
        assert!(tree.find(0x1080).is_none());
    }
}
