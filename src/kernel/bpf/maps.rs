//! linux-parity: partial
//! linux-source: vendor/linux/kernel/bpf
//! test-origin: linux:vendor/linux/kernel/bpf
//! Legacy in-kernel BPF map subset used by the syscall tests.
//!
//! This is not a 1:1 migration of any individual Linux BPF map source file.

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use spin::Mutex;

use super::uapi::{
    BPF_ANY, BPF_EXIST, BPF_MAP_TYPE_ARRAY, BPF_MAP_TYPE_HASH, BPF_MAP_TYPE_QUEUE,
    BPF_MAP_TYPE_STACK, BPF_NOEXIST,
};

pub struct Map {
    pub id: i32,
    pub map_type: u32,
    pub key_size: u32,
    pub value_size: u32,
    pub max_entries: u32,
    frozen: AtomicBool,
    inner: Mutex<MapInner>,
}

enum MapInner {
    Hash(Vec<(Vec<u8>, Vec<u8>)>),
    Array(Vec<Vec<u8>>),
    Queue(Vec<Vec<u8>>),
    Stack(Vec<Vec<u8>>),
}

impl Map {
    pub fn new(
        map_type: u32,
        key_size: u32,
        value_size: u32,
        max_entries: u32,
    ) -> Result<Self, i32> {
        if value_size == 0 || max_entries == 0 {
            return Err(-22);
        }
        let id = NEXT_MAP_ID.fetch_add(1, Ordering::AcqRel);
        let inner = match map_type {
            BPF_MAP_TYPE_HASH => {
                if key_size == 0 {
                    return Err(-22);
                }
                MapInner::Hash(Vec::new())
            }
            BPF_MAP_TYPE_ARRAY => {
                if key_size != 4 {
                    return Err(-22);
                }
                let mut a = Vec::with_capacity(max_entries as usize);
                for _ in 0..max_entries {
                    a.push(alloc::vec![0u8; value_size as usize]);
                }
                MapInner::Array(a)
            }
            BPF_MAP_TYPE_QUEUE => {
                if key_size != 0 {
                    return Err(-22);
                }
                MapInner::Queue(Vec::new())
            }
            BPF_MAP_TYPE_STACK => {
                if key_size != 0 {
                    return Err(-22);
                }
                MapInner::Stack(Vec::new())
            }
            _ => return Err(-22),
        };
        Ok(Self {
            id,
            map_type,
            key_size,
            value_size,
            max_entries,
            frozen: AtomicBool::new(false),
            inner: Mutex::new(inner),
        })
    }

    pub fn lookup(&self, key: &[u8]) -> Option<Vec<u8>> {
        let g = self.inner.lock();
        match &*g {
            MapInner::Hash(buckets) => buckets
                .iter()
                .find(|(k, _)| k.as_slice() == key)
                .map(|(_, v)| v.clone()),
            MapInner::Array(elems) => {
                if key.len() != 4 {
                    return None;
                }
                let idx = u32::from_ne_bytes([key[0], key[1], key[2], key[3]]) as usize;
                elems.get(idx).cloned()
            }
            MapInner::Queue(elems) => elems.first().cloned(),
            MapInner::Stack(elems) => elems.last().cloned(),
        }
    }

    pub fn update(&self, key: &[u8], value: &[u8]) -> Result<(), i32> {
        self.update_with_flags(key, value, BPF_ANY)
    }

    pub fn update_with_flags(&self, key: &[u8], value: &[u8], flags: u64) -> Result<(), i32> {
        if self.frozen.load(Ordering::Acquire) {
            return Err(-1);
        }
        if value.len() != self.value_size as usize {
            return Err(-22);
        }
        let mut g = self.inner.lock();
        match &mut *g {
            MapInner::Hash(buckets) => {
                if key.len() != self.key_size as usize {
                    return Err(-22);
                }
                let existing = buckets.iter_mut().find(|(k, _)| k.as_slice() == key);
                match (flags, existing) {
                    (BPF_NOEXIST, Some(_)) => Err(-17),
                    (BPF_EXIST, None) => Err(-2),
                    (BPF_ANY | BPF_EXIST, Some(slot)) => {
                        slot.1 = value.to_vec();
                        Ok(())
                    }
                    (BPF_ANY | BPF_NOEXIST, None) => {
                        if buckets.len() >= self.max_entries as usize {
                            return Err(-7);
                        }
                        buckets.push((key.to_vec(), value.to_vec()));
                        Ok(())
                    }
                    _ => Err(-22),
                }
            }
            MapInner::Array(elems) => {
                if flags == BPF_NOEXIST {
                    return Err(-17);
                }
                if flags != BPF_ANY && flags != BPF_EXIST {
                    return Err(-22);
                }
                if key.len() != 4 {
                    return Err(-22);
                }
                let idx = u32::from_ne_bytes([key[0], key[1], key[2], key[3]]) as usize;
                if idx >= elems.len() {
                    return Err(-22);
                }
                elems[idx].copy_from_slice(value);
                Ok(())
            }
            MapInner::Queue(elems) | MapInner::Stack(elems) => {
                if flags != BPF_ANY {
                    return Err(-22);
                }
                if elems.len() >= self.max_entries as usize {
                    return Err(-7);
                }
                elems.push(value.to_vec());
                Ok(())
            }
        }
    }

    pub fn delete(&self, key: &[u8]) -> Result<(), i32> {
        let mut g = self.inner.lock();
        match &mut *g {
            MapInner::Hash(buckets) => {
                let len_before = buckets.len();
                buckets.retain(|(k, _)| k.as_slice() != key);
                if buckets.len() == len_before {
                    Err(-2)
                } else {
                    Ok(())
                }
            }
            MapInner::Array(_) => Err(-22),
            MapInner::Queue(_) | MapInner::Stack(_) => Err(-22),
        }
    }

    pub fn lookup_and_delete(&self, key: &[u8]) -> Result<Vec<u8>, i32> {
        let mut g = self.inner.lock();
        match &mut *g {
            MapInner::Hash(buckets) => {
                let Some(pos) = buckets.iter().position(|(k, _)| k.as_slice() == key) else {
                    return Err(-2);
                };
                Ok(buckets.remove(pos).1)
            }
            MapInner::Queue(elems) => {
                if elems.is_empty() {
                    Err(-2)
                } else {
                    Ok(elems.remove(0))
                }
            }
            MapInner::Stack(elems) => elems.pop().ok_or(-2),
            MapInner::Array(_) => Err(-22),
        }
    }

    pub fn get_next_key(&self, key: Option<&[u8]>) -> Result<Vec<u8>, i32> {
        let g = self.inner.lock();
        match &*g {
            MapInner::Hash(buckets) => {
                if key.is_none() {
                    return buckets.first().map(|(k, _)| k.clone()).ok_or(-2);
                }
                let key = key.unwrap();
                let Some(pos) = buckets.iter().position(|(k, _)| k.as_slice() == key) else {
                    return Err(-2);
                };
                buckets.get(pos + 1).map(|(k, _)| k.clone()).ok_or(-2)
            }
            MapInner::Array(elems) => {
                let next = match key {
                    None => 0,
                    Some(key) => {
                        if key.len() != 4 {
                            return Err(-22);
                        }
                        u32::from_ne_bytes([key[0], key[1], key[2], key[3]]) as usize + 1
                    }
                };
                if next >= elems.len() {
                    return Err(-2);
                }
                Ok((next as u32).to_ne_bytes().to_vec())
            }
            MapInner::Queue(_) | MapInner::Stack(_) => Err(-22),
        }
    }

    pub fn freeze(&self) {
        self.frozen.store(true, Ordering::Release);
    }
}

static NEXT_MAP_ID: AtomicI32 = AtomicI32::new(1);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_create_lookup_update_delete() {
        let m = Map::new(BPF_MAP_TYPE_HASH, 4, 8, 16).unwrap();
        let k = 7u32.to_ne_bytes();
        let v = 42u64.to_ne_bytes();
        m.update(&k, &v).unwrap();
        let got = m.lookup(&k).unwrap();
        assert_eq!(got, v);
        m.delete(&k).unwrap();
        assert!(m.lookup(&k).is_none());
    }

    #[test]
    fn hash_full_returns_e2big() {
        let m = Map::new(BPF_MAP_TYPE_HASH, 4, 4, 2).unwrap();
        m.update(&1u32.to_ne_bytes(), &1u32.to_ne_bytes()).unwrap();
        m.update(&2u32.to_ne_bytes(), &2u32.to_ne_bytes()).unwrap();
        assert_eq!(m.update(&3u32.to_ne_bytes(), &3u32.to_ne_bytes()), Err(-7));
    }

    #[test]
    fn array_round_trip() {
        let m = Map::new(BPF_MAP_TYPE_ARRAY, 4, 8, 8).unwrap();
        let v = 99u64.to_ne_bytes();
        m.update(&3u32.to_ne_bytes(), &v).unwrap();
        let got = m.lookup(&3u32.to_ne_bytes()).unwrap();
        assert_eq!(got, v);
    }

    #[test]
    fn array_oob_returns_einval() {
        let m = Map::new(BPF_MAP_TYPE_ARRAY, 4, 4, 4).unwrap();
        assert_eq!(
            m.update(&99u32.to_ne_bytes(), &0u32.to_ne_bytes()),
            Err(-22)
        );
    }

    #[test]
    fn get_next_and_lookup_delete_match_linux_map_ops() {
        let m = Map::new(BPF_MAP_TYPE_HASH, 4, 4, 4).unwrap();
        let k1 = 1u32.to_ne_bytes();
        let k2 = 2u32.to_ne_bytes();
        m.update(&k1, &10u32.to_ne_bytes()).unwrap();
        m.update(&k2, &20u32.to_ne_bytes()).unwrap();
        assert_eq!(m.get_next_key(None).unwrap(), k1);
        assert_eq!(m.get_next_key(Some(&k1)).unwrap(), k2);
        assert_eq!(m.lookup_and_delete(&k1).unwrap(), 10u32.to_ne_bytes());
        assert!(m.lookup(&k1).is_none());
    }

    #[test]
    fn queue_and_stack_maps_pop_in_linux_order() {
        let q = Map::new(BPF_MAP_TYPE_QUEUE, 0, 4, 4).unwrap();
        q.update(&[], &1u32.to_ne_bytes()).unwrap();
        q.update(&[], &2u32.to_ne_bytes()).unwrap();
        assert_eq!(q.lookup_and_delete(&[]).unwrap(), 1u32.to_ne_bytes());

        let s = Map::new(BPF_MAP_TYPE_STACK, 0, 4, 4).unwrap();
        s.update(&[], &1u32.to_ne_bytes()).unwrap();
        s.update(&[], &2u32.to_ne_bytes()).unwrap();
        assert_eq!(s.lookup_and_delete(&[]).unwrap(), 2u32.to_ne_bytes());
    }

    #[test]
    fn frozen_map_rejects_updates() {
        let m = Map::new(BPF_MAP_TYPE_ARRAY, 4, 4, 1).unwrap();
        m.freeze();
        assert_eq!(m.update(&0u32.to_ne_bytes(), &1u32.to_ne_bytes()), Err(-1));
    }
}
