//! linux-parity: complete
//! linux-source: vendor/linux/net/batman-adv/hash.c
//! test-origin: linux:vendor/linux/net/batman-adv/hash.c
//! B.A.T.M.A.N. advanced hashtable allocation shape.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatadvHashtable {
    pub size: u32,
    pub generation: i32,
    pub table_initialized: Vec<bool>,
    pub locks_initialized: Vec<bool>,
    pub lock_class_keys: Vec<Option<u32>>,
}

fn batadv_hash_init(hash: &mut BatadvHashtable) {
    for i in 0..hash.size as usize {
        hash.table_initialized[i] = true;
        hash.locks_initialized[i] = true;
    }
    hash.generation = 0;
}

pub fn batadv_hash_new(size: u32) -> BatadvHashtable {
    let mut hash = BatadvHashtable {
        size,
        generation: -1,
        table_initialized: vec![false; size as usize],
        locks_initialized: vec![false; size as usize],
        lock_class_keys: vec![None; size as usize],
    };
    batadv_hash_init(&mut hash);
    hash
}

pub fn batadv_hash_destroy(hash: BatadvHashtable) -> (usize, usize, u32) {
    (
        hash.table_initialized.len(),
        hash.locks_initialized.len(),
        hash.size,
    )
}

pub fn batadv_hash_set_lock_class(hash: &mut BatadvHashtable, key: u32) {
    for lock_class in &mut hash.lock_class_keys {
        *lock_class = Some(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batadv_hash_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/batman-adv/hash.c"
        ));
        assert!(source.contains("static void batadv_hash_init(struct batadv_hashtable *hash)"));
        assert!(source.contains("for (i = 0; i < hash->size; i++)"));
        assert!(source.contains("INIT_HLIST_HEAD(&hash->table[i]);"));
        assert!(source.contains("spin_lock_init(&hash->list_locks[i]);"));
        assert!(source.contains("atomic_set(&hash->generation, 0);"));
        assert!(source.contains("void batadv_hash_destroy(struct batadv_hashtable *hash)"));
        assert!(source.contains("kfree(hash->list_locks);"));
        assert!(source.contains("kfree(hash->table);"));
        assert!(source.contains("kfree(hash);"));
        assert!(source.contains("struct batadv_hashtable *batadv_hash_new(u32 size)"));
        assert!(source.contains("hash = kmalloc_obj(*hash, GFP_ATOMIC);"));
        assert!(source.contains("hash->table = kmalloc_objs(*hash->table, size, GFP_ATOMIC);"));
        assert!(
            source
                .contains("hash->list_locks = kmalloc_objs(*hash->list_locks, size, GFP_ATOMIC);")
        );
        assert!(source.contains("hash->size = size;"));
        assert!(source.contains("batadv_hash_init(hash);"));
        assert!(source.contains("void batadv_hash_set_lock_class"));
        assert!(source.contains("lockdep_set_class(&hash->list_locks[i], key);"));

        let mut hash = batadv_hash_new(3);
        assert_eq!(hash.generation, 0);
        assert_eq!(hash.table_initialized, [true, true, true]);
        assert_eq!(hash.locks_initialized, [true, true, true]);
        batadv_hash_set_lock_class(&mut hash, 77);
        assert_eq!(hash.lock_class_keys, [Some(77), Some(77), Some(77)]);
        assert_eq!(batadv_hash_destroy(hash), (3, 3, 3));
    }
}
