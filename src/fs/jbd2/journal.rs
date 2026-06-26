//! linux-parity: partial
//! linux-source: vendor/linux/fs/jbd2/journal.c
//! `journal_t` — JBD2 journal handle (skeleton).

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

use spin::Mutex;

#[repr(C)]
pub struct OnDiskJournalSuperBlockHeader {
    pub h_magic: u32,
    pub h_blocktype: u32,
    pub h_sequence: u32,
}

pub struct Journal {
    pub seq: AtomicU32,
    pub committing: Mutex<()>,
    pub committed_metadata: Mutex<Vec<CommittedMetadataBlock>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedMetadataBlock {
    pub sequence: u32,
    pub target_block: u64,
    pub block_size: u64,
    pub len: usize,
}

impl Journal {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            seq: AtomicU32::new(0),
            committing: Mutex::new(()),
            committed_metadata: Mutex::new(Vec::new()),
        })
    }
    pub fn next_sequence(&self) -> u32 {
        self.seq.fetch_add(1, Ordering::AcqRel)
    }
    pub fn record_committed_metadata(&self, block: CommittedMetadataBlock) {
        self.committed_metadata.lock().push(block);
    }
    pub fn committed_metadata_count(&self) -> usize {
        self.committed_metadata.lock().len()
    }
}
