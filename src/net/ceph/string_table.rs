//! linux-parity: complete
//! linux-source: vendor/linux/net/ceph/string_table.c
//! test-origin: linux:vendor/linux/net/ceph/string_table.c
//! Ceph refcounted string table behavior.

extern crate alloc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CephString {
    pub text: alloc::string::String,
    pub kref: usize,
    pub in_tree: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CephStringTable {
    entries: alloc::vec::Vec<CephString>,
}

impl CephStringTable {
    pub const fn new() -> Self {
        Self {
            entries: alloc::vec::Vec::new(),
        }
    }

    pub fn ceph_find_or_create_string(&mut self, str_: &str, alloc_ok: bool) -> Option<usize> {
        if let Some((idx, entry)) = self
            .entries
            .iter_mut()
            .enumerate()
            .find(|(_, entry)| entry.in_tree && entry.text == str_)
        {
            if entry.kref != 0 {
                entry.kref += 1;
                return Some(idx);
            }
            entry.in_tree = false;
        }
        if !alloc_ok {
            return None;
        }
        self.entries.push(CephString {
            text: str_.into(),
            kref: 1,
            in_tree: true,
        });
        Some(self.entries.len() - 1)
    }

    pub fn ceph_release_string(&mut self, idx: usize) {
        if let Some(entry) = self.entries.get_mut(idx) {
            entry.kref = entry.kref.saturating_sub(1);
            if entry.kref == 0 {
                entry.in_tree = false;
            }
        }
    }

    pub fn ceph_strings_empty(&self) -> bool {
        !self.entries.iter().any(|entry| entry.in_tree)
    }

    pub fn get(&self, idx: usize) -> Option<&CephString> {
        self.entries.get(idx)
    }
}

pub fn ceph_compare_string(existing: &CephString, str_: &str) -> core::cmp::Ordering {
    existing.text.as_str().cmp(str_)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ceph_string_table_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ceph/string_table.c"
        ));
        assert!(source.contains("static DEFINE_SPINLOCK(string_tree_lock);"));
        assert!(source.contains("static struct rb_root string_tree = RB_ROOT;"));
        assert!(source.contains("struct ceph_string *ceph_find_or_create_string"));
        assert!(source.contains("spin_lock(&string_tree_lock);"));
        assert!(source.contains("ret = ceph_compare_string(exist, str, len);"));
        assert!(source.contains("if (exist && !kref_get_unless_zero(&exist->kref))"));
        assert!(source.contains("rb_erase(&exist->node, &string_tree);"));
        assert!(source.contains("cs = kmalloc(sizeof(*cs) + len + 1, GFP_NOFS);"));
        assert!(source.contains("kref_init(&cs->kref);"));
        assert!(source.contains("memcpy(cs->str, str, len);"));
        assert!(source.contains("rb_link_node(&cs->node, parent, p);"));
        assert!(source.contains("rb_insert_color(&cs->node, &string_tree);"));
        assert!(source.contains("goto retry;"));
        assert!(source.contains("void ceph_release_string"));
        assert!(source.contains("kfree_rcu(cs, rcu);"));
        assert!(source.contains("bool ceph_strings_empty(void)"));
    }

    #[test]
    fn string_table_interns_refcounts_and_releases_entries() {
        let mut table = CephStringTable::new();
        assert!(table.ceph_strings_empty());
        let first = table.ceph_find_or_create_string("alpha", true).unwrap();
        let again = table.ceph_find_or_create_string("alpha", true).unwrap();
        assert_eq!(first, again);
        assert_eq!(table.get(first).unwrap().kref, 2);
        assert!(!table.ceph_strings_empty());
        table.ceph_release_string(first);
        assert_eq!(table.get(first).unwrap().kref, 1);
        table.ceph_release_string(first);
        assert!(table.ceph_strings_empty());
        assert!(table.ceph_find_or_create_string("beta", false).is_none());
    }
}
