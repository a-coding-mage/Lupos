//! linux-parity: partial
//! linux-source: vendor/linux/fs/jbd2/transaction.c
//! test-origin: linux:vendor/linux/fs/jbd2/transaction.c
//! `transaction_t` — JBD2 transaction record and metadata commit shim.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;

use lazy_static::lazy_static;
use spin::Mutex;

use crate::block::bio::{BIO_OP_WRITE, BioOp, BioVec, bio_alloc, submit_bio};
use crate::block::block_device::BlockDeviceRef;
use crate::include::uapi::errno::{EINVAL, ENOSPC};

use super::journal::{CommittedMetadataBlock, Journal};

pub struct Transaction {
    pub journal: Arc<Journal>,
    pub seq: u32,
    pub credits: u32,
    metadata: Mutex<Vec<JournaledMetadataBlock>>,
}

struct JournaledMetadataBlock {
    bdev: BlockDeviceRef,
    target_block: u64,
    block_size: u64,
    payload: Vec<u8>,
}

lazy_static! {
    static ref BDEV_JOURNALS: Mutex<BTreeMap<u64, Arc<Journal>>> = Mutex::new(BTreeMap::new());
}

pub fn journal_for_block_device(bdev: &BlockDeviceRef) -> Arc<Journal> {
    let mut journals = BDEV_JOURNALS.lock();
    journals.entry(bdev.id).or_insert_with(Journal::new).clone()
}

pub fn jbd2_journal_start(journal: &Arc<Journal>, nblocks: u32) -> Arc<Transaction> {
    Arc::new(Transaction {
        journal: journal.clone(),
        seq: journal.next_sequence(),
        credits: nblocks,
        metadata: Mutex::new(Vec::new()),
    })
}

pub fn jbd2_journal_dirty_metadata(
    transaction: &Arc<Transaction>,
    bdev: &BlockDeviceRef,
    target_block: u64,
    block_size: u64,
    payload: &[u8],
) -> Result<(), i32> {
    if block_size == 0 || block_size % 512 != 0 || payload.len() as u64 != block_size {
        return Err(EINVAL);
    }
    let mut metadata = transaction.metadata.lock();
    if metadata.len() >= transaction.credits as usize {
        return Err(ENOSPC);
    }
    metadata.push(JournaledMetadataBlock {
        bdev: bdev.clone(),
        target_block,
        block_size,
        payload: payload.to_vec(),
    });
    Ok(())
}

pub fn jbd2_journal_stop(transaction: Arc<Transaction>) -> Result<(), i32> {
    let _commit = transaction.journal.committing.lock();
    let metadata = transaction.metadata.lock();
    for block in metadata.iter() {
        write_metadata_block(block)?;
        transaction
            .journal
            .record_committed_metadata(CommittedMetadataBlock {
                sequence: transaction.seq,
                target_block: block.target_block,
                block_size: block.block_size,
                len: block.payload.len(),
            });
    }
    Ok(())
}

pub fn jbd2_journal_write_metadata_block(
    bdev: &BlockDeviceRef,
    target_block: u64,
    block_size: u64,
    payload: &[u8],
) -> Result<(), i32> {
    let journal = journal_for_block_device(bdev);
    let transaction = jbd2_journal_start(&journal, 1);
    jbd2_journal_dirty_metadata(&transaction, bdev, target_block, block_size, payload)?;
    jbd2_journal_stop(transaction)
}

pub fn committed_metadata_count_for_block_device(bdev: &BlockDeviceRef) -> usize {
    BDEV_JOURNALS
        .lock()
        .get(&bdev.id)
        .map(|journal| journal.committed_metadata_count())
        .unwrap_or(0)
}

fn write_metadata_block(block: &JournaledMetadataBlock) -> Result<(), i32> {
    let sector = block
        .target_block
        .checked_mul(block.block_size / 512)
        .ok_or(EINVAL)?;
    let bio = bio_alloc(block.bdev.clone(), BioOp(BIO_OP_WRITE), sector);
    bio.add_vec(BioVec::new(block.payload.clone()));
    submit_bio(bio)
}

#[cfg(test)]
pub fn reset_journals_for_test() {
    BDEV_JOURNALS.lock().clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::bio::{BIO_OP_READ, BioOp, BioVec, bio_alloc, submit_bio};
    use crate::block::block_device::BlockDevice;
    use crate::block::mem::{MemBlockDevice, mem_block_device_ops};

    #[test]
    fn jbd2_transaction_commits_metadata_blocks_in_credit_order() {
        reset_journals_for_test();
        let mem = MemBlockDevice::new("jbd2-transaction0", 4096);
        let bdev = BlockDevice::wrap(mem, mem_block_device_ops());
        let journal = journal_for_block_device(&bdev);
        let transaction = jbd2_journal_start(&journal, 2);

        jbd2_journal_dirty_metadata(&transaction, &bdev, 1, 512, &[0x11; 512]).unwrap();
        jbd2_journal_dirty_metadata(&transaction, &bdev, 2, 512, &[0x22; 512]).unwrap();
        assert_eq!(
            jbd2_journal_dirty_metadata(&transaction, &bdev, 3, 512, &[0x33; 512]),
            Err(ENOSPC)
        );
        jbd2_journal_stop(transaction).unwrap();

        assert_eq!(committed_metadata_count_for_block_device(&bdev), 2);
        let read = bio_alloc(bdev, BioOp(BIO_OP_READ), 1);
        read.add_vec(BioVec::new(alloc::vec![0u8; 1024]));
        submit_bio(read.clone()).unwrap();
        let vecs = read.vecs.lock();
        let data = vecs[0].data.lock();
        assert_eq!(&data[..512], &[0x11; 512]);
        assert_eq!(&data[512..], &[0x22; 512]);
    }
}
