//! linux-parity: complete
//! linux-source: vendor/linux/security/selinux/ss/symtab.c
//! test-origin: linux:vendor/linux/security/selinux/ss/symtab.c
//! SELinux symbol table helpers.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::include::uapi::errno::{EEXIST, EINVAL};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymtabEntry<T> {
    pub name: String,
    pub datum: T,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Symtab<T> {
    pub size: u32,
    pub nprim: u32,
    entries: Vec<SymtabEntry<T>>,
}

impl<T> Symtab<T> {
    pub fn init(size: u32) -> Result<Self, i32> {
        if size == 0 || !size.is_power_of_two() {
            return Err(-EINVAL);
        }
        Ok(Self {
            size,
            nprim: 0,
            entries: Vec::new(),
        })
    }

    pub fn insert(&mut self, name: &str, datum: T) -> Result<(), i32> {
        if self.entries.iter().any(|entry| entry.name == name) {
            return Err(-EEXIST);
        }

        let index = self
            .entries
            .binary_search_by(|entry| entry.name.as_str().cmp(name))
            .unwrap_or_else(|index| index);
        self.entries.insert(
            index,
            SymtabEntry {
                name: String::from(name),
                datum,
            },
        );
        Ok(())
    }

    pub fn search(&self, name: &str) -> Option<&T> {
        self.entries
            .binary_search_by(|entry| entry.name.as_str().cmp(name))
            .ok()
            .map(|index| &self.entries[index].datum)
    }

    pub fn bucket(&self, name: &str) -> u32 {
        symhash(name.as_bytes()) & (self.size - 1)
    }
}

pub fn symhash(key: &[u8]) -> u32 {
    let mut hash = 5381u32;
    for byte in key {
        if *byte == 0 {
            break;
        }
        hash = hash.wrapping_shl(5).wrapping_add(hash) ^ (*byte as u32);
    }
    hash
}

pub fn symcmp(key1: &str, key2: &str) -> core::cmp::Ordering {
    key1.cmp(key2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symtab_hash_and_lookup_follow_linux_source_shape() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/selinux/ss/symtab.c"
        ));
        assert!(source.contains("unsigned int hash = 5381;"));
        assert!(source.contains("hash = ((hash << 5) + hash) ^ c;"));
        assert!(source.contains("return strcmp(keyp1, keyp2);"));
        assert!(source.contains("return hashtab_insert(&s->table, name, datum"));
        assert!(source.contains("return hashtab_search(&s->table, name"));

        assert_eq!(symhash(b"user\0ignored"), symhash(b"user"));
        assert_eq!(symhash(b""), 5381);
        assert_eq!(
            symhash(b"system_u"),
            b"system_u".iter().fold(5381u32, |hash, byte| {
                hash.wrapping_shl(5).wrapping_add(hash) ^ (*byte as u32)
            })
        );

        let mut symtab = Symtab::init(8).expect("symtab");
        assert_eq!(symtab.nprim, 0);
        symtab.insert("user", 10).expect("insert user");
        symtab.insert("role", 20).expect("insert role");
        assert_eq!(symtab.search("user"), Some(&10));
        assert_eq!(symtab.search("role"), Some(&20));
        assert_eq!(symtab.search("type"), None);
        assert_eq!(symtab.insert("user", 30), Err(-EEXIST));
        assert_eq!(symtab.bucket("user"), symhash(b"user") & 7);
    }
}
