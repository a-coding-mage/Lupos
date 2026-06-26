//! linux-parity: complete
//! linux-source: vendor/linux/fs/btrfs/orphan.c
//! test-origin: linux:vendor/linux/fs/btrfs/orphan.c
//! Btrfs orphan item insertion and deletion.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::{EEXIST, ENOENT, ENOMEM};

pub const BTRFS_ORPHAN_OBJECTID: u64 = u64::MAX - 4;
pub const BTRFS_ORPHAN_ITEM_KEY: u8 = 48;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BtrfsKey {
    pub objectid: u64,
    pub item_type: u8,
    pub offset: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BtrfsRoot {
    orphan_items: Vec<BtrfsKey>,
    path_allocation_fails: bool,
}

impl BtrfsRoot {
    pub const fn new() -> Self {
        Self {
            orphan_items: Vec::new(),
            path_allocation_fails: false,
        }
    }

    pub const fn with_path_allocation_failure() -> Self {
        Self {
            orphan_items: Vec::new(),
            path_allocation_fails: true,
        }
    }

    pub fn orphan_items(&self) -> &[BtrfsKey] {
        &self.orphan_items
    }
}

pub const fn orphan_item_key(offset: u64) -> BtrfsKey {
    BtrfsKey {
        objectid: BTRFS_ORPHAN_OBJECTID,
        item_type: BTRFS_ORPHAN_ITEM_KEY,
        offset,
    }
}

pub fn btrfs_insert_orphan_item(root: &mut BtrfsRoot, offset: u64) -> i32 {
    if root.path_allocation_fails {
        return -ENOMEM;
    }
    let key = orphan_item_key(offset);
    if root.orphan_items.iter().any(|item| *item == key) {
        return -EEXIST;
    }
    root.orphan_items.push(key);
    0
}

pub fn btrfs_del_orphan_item(root: &mut BtrfsRoot, offset: u64) -> i32 {
    if root.path_allocation_fails {
        return -ENOMEM;
    }
    let key = orphan_item_key(offset);
    let Some(index) = root.orphan_items.iter().position(|item| *item == key) else {
        return -ENOENT;
    };
    root.orphan_items.remove(index);
    0
}

pub fn btrfs_insert_orphan_item_outcome(
    path_allocated: bool,
    offset: u64,
) -> Result<BtrfsKey, i32> {
    if !path_allocated {
        return Err(-ENOMEM);
    }
    Ok(orphan_item_key(offset))
}

pub fn btrfs_del_orphan_item_outcome(
    path_allocated: bool,
    search_slot_ret: i32,
    offset: u64,
) -> Result<BtrfsKey, i32> {
    if !path_allocated {
        return Err(-ENOMEM);
    }
    if search_slot_ret < 0 {
        return Err(search_slot_ret);
    }
    if search_slot_ret != 0 {
        return Err(-ENOENT);
    }
    Ok(orphan_item_key(offset))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn btrfs_orphan_item_key_and_errors_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/btrfs/orphan.c"
        ));
        assert!(source.contains("BTRFS_PATH_AUTO_FREE(path);"));
        assert!(source.contains("key.objectid = BTRFS_ORPHAN_OBJECTID;"));
        assert!(source.contains("key.type = BTRFS_ORPHAN_ITEM_KEY;"));
        assert!(source.contains("key.offset = offset;"));
        assert!(source.contains("path = btrfs_alloc_path();"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("btrfs_insert_empty_item(trans, root, path, &key, 0);"));
        assert!(source.contains("btrfs_search_slot(trans, root, &key, path, -1, 1);"));
        assert!(source.contains("return -ENOENT;"));
        assert!(source.contains("return btrfs_del_item(trans, root, path);"));

        let key = orphan_item_key(123);
        assert_eq!(key.objectid, u64::MAX - 4);
        assert_eq!(key.item_type, 48);
        assert_eq!(btrfs_insert_orphan_item_outcome(false, 1), Err(-ENOMEM));
        assert_eq!(btrfs_del_orphan_item_outcome(true, -5, 1), Err(-5));
        assert_eq!(btrfs_del_orphan_item_outcome(true, 1, 1), Err(-ENOENT));
        assert_eq!(btrfs_del_orphan_item_outcome(true, 0, 123), Ok(key));

        let mut root = BtrfsRoot::new();
        assert_eq!(btrfs_insert_orphan_item(&mut root, 123), 0);
        assert_eq!(root.orphan_items(), &[key]);
        assert_eq!(btrfs_insert_orphan_item(&mut root, 123), -EEXIST);
        assert_eq!(btrfs_del_orphan_item(&mut root, 124), -ENOENT);
        assert_eq!(btrfs_del_orphan_item(&mut root, 123), 0);
        assert!(root.orphan_items().is_empty());

        let mut failed = BtrfsRoot::with_path_allocation_failure();
        assert_eq!(btrfs_insert_orphan_item(&mut failed, 1), -ENOMEM);
        assert_eq!(btrfs_del_orphan_item(&mut failed, 1), -ENOMEM);
    }
}
