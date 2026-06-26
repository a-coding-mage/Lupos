//! linux-parity: complete
//! linux-source: vendor/linux/fs/file_table.c
//! test-origin: linux:vendor/linux/fs/file_table.c
//! Global file table accounting.
//!
//! Ref: `vendor/linux/fs/file_table.c`

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};

use spin::Mutex;

use crate::include::uapi::errno::{ENFILE, ENOENT};

use super::types::FileRef;

const DEFAULT_MAX_FILES: usize = 1 << 20;

lazy_static::lazy_static! {
    static ref FILE_TABLE: Mutex<FileTable> = Mutex::new(FileTable::new(DEFAULT_MAX_FILES));
}

static ALLOCATED_FILES: AtomicUsize = AtomicUsize::new(0);

pub struct FileTable {
    next_id: usize,
    max_files: usize,
    entries: BTreeMap<usize, FileRef>,
    refs: BTreeMap<usize, usize>,
}

impl FileTable {
    pub fn new(max_files: usize) -> Self {
        Self {
            next_id: 1,
            max_files,
            entries: BTreeMap::new(),
            refs: BTreeMap::new(),
        }
    }

    pub fn alloc(&mut self, file: FileRef) -> Result<usize, i32> {
        if self.entries.len() >= self.max_files {
            return Err(ENFILE);
        }
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1).max(1);
        self.entries.insert(id, file);
        self.refs.insert(id, 1);
        Ok(id)
    }

    pub fn get(&self, id: usize) -> Result<FileRef, i32> {
        self.entries.get(&id).cloned().ok_or(ENOENT)
    }

    pub fn get_file_rcu(&mut self, id: usize) -> Result<FileRef, i32> {
        let file = self.entries.get(&id).cloned().ok_or(ENOENT)?;
        *self.refs.entry(id).or_insert(0) += 1;
        Ok(file)
    }

    pub fn fput(&mut self, id: usize) -> Result<bool, i32> {
        let refs = self.refs.get_mut(&id).ok_or(ENOENT)?;
        if *refs > 1 {
            *refs -= 1;
            return Ok(false);
        }
        self.refs.remove(&id);
        self.entries.remove(&id);
        Ok(true)
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    pub fn refcount(&self, id: usize) -> Result<usize, i32> {
        self.refs.get(&id).copied().ok_or(ENOENT)
    }
}

pub(crate) fn account_allocated_file() {
    ALLOCATED_FILES.fetch_add(1, Ordering::AcqRel);
}

pub(crate) fn account_released_file() {
    let _ = ALLOCATED_FILES.fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
        Some(value.saturating_sub(1))
    });
}

pub fn allocated_files() -> usize {
    ALLOCATED_FILES.load(Ordering::Acquire)
}

pub fn file_table_alloc(file: FileRef) -> Result<usize, i32> {
    FILE_TABLE.lock().alloc(file)
}

pub fn get_file(id: usize) -> Result<FileRef, i32> {
    FILE_TABLE.lock().get(id)
}

pub fn get_file_rcu(id: usize) -> Result<FileRef, i32> {
    FILE_TABLE.lock().get_file_rcu(id)
}

pub fn fput_file(id: usize) -> Result<bool, i32> {
    FILE_TABLE.lock().fput(id)
}

pub fn table_count() -> usize {
    FILE_TABLE.lock().count()
}

pub fn file_refcount(id: usize) -> Result<usize, i32> {
    FILE_TABLE.lock().refcount(id)
}

pub fn same_file(left: &FileRef, right: &FileRef) -> bool {
    Arc::ptr_eq(left, right)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::dcache::d_alloc;
    use crate::fs::file::alloc_file;
    use crate::fs::ops::NOOP_FILE_OPS;

    #[test]
    fn file_table_tracks_refcounts_until_last_put() {
        let file = alloc_file(d_alloc("tracked"), 0, 0, &NOOP_FILE_OPS);
        let mut table = FileTable::new(4);
        let id = table.alloc(file.clone()).unwrap();
        assert_eq!(table.count(), 1);
        assert!(same_file(&file, &table.get_file_rcu(id).unwrap()));
        assert_eq!(table.refcount(id), Ok(2));
        assert_eq!(table.fput(id), Ok(false));
        assert_eq!(table.fput(id), Ok(true));
        assert_eq!(table.count(), 0);
    }

    #[test]
    fn file_table_enforces_global_limit() {
        let mut table = FileTable::new(1);
        table
            .alloc(alloc_file(d_alloc("one"), 0, 0, &NOOP_FILE_OPS))
            .unwrap();
        assert!(matches!(
            table.alloc(alloc_file(d_alloc("two"), 0, 0, &NOOP_FILE_OPS)),
            Err(ENFILE)
        ));
    }
}
