//! linux-parity: partial
//! linux-source: vendor/linux/fs/jbd2/recovery.c
//! test-origin: linux:vendor/linux/fs/jbd2/recovery.c
//! JBD2 checkpoint, commit, recovery, and revoke helpers.
//!
//! Mirrors:
//! `vendor/linux/fs/jbd2/checkpoint.c`
//! `vendor/linux/fs/jbd2/commit.c`
//! `vendor/linux/fs/jbd2/recovery.c`
//! `vendor/linux/fs/jbd2/revoke.c`

extern crate alloc;

use alloc::vec::Vec;

use crate::block::bio::{BIO_OP_WRITE, BioOp, BioVec, bio_alloc, submit_bio};
use crate::block::block_device::BlockDeviceRef;
use crate::include::uapi::errno::EINVAL;

use super::{JBD2_COMMIT_BLOCK, JBD2_DESCRIPTOR_BLOCK, JBD2_REVOKE_BLOCK};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JournalBlock {
    pub sequence: u32,
    pub block_type: u32,
    pub target_block: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalReplayBlock {
    pub sequence: u32,
    pub block_type: u32,
    pub target_block: u64,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct RevokeTable {
    blocks: Vec<u64>,
}

impl RevokeTable {
    pub fn revoke(&mut self, block: u64) {
        if !self.blocks.contains(&block) {
            self.blocks.push(block);
        }
    }

    pub fn is_revoked(&self, block: u64) -> bool {
        self.blocks.contains(&block)
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReplayStats {
    pub descriptors: u32,
    pub commits: u32,
    pub revoked: u32,
    pub replayed: u32,
    pub last_sequence: u32,
}

pub fn replay(blocks: &[JournalBlock], revokes: &mut RevokeTable) -> Result<ReplayStats, i32> {
    let mut stats = ReplayStats::default();
    for block in blocks.iter().copied() {
        stats.last_sequence = block.sequence;
        match block.block_type {
            JBD2_DESCRIPTOR_BLOCK => {
                stats.descriptors += 1;
                if !revokes.is_revoked(block.target_block) {
                    stats.replayed += 1;
                }
            }
            JBD2_COMMIT_BLOCK => {
                stats.commits += 1;
            }
            JBD2_REVOKE_BLOCK => {
                revokes.revoke(block.target_block);
                stats.revoked += 1;
            }
            _ => return Err(EINVAL),
        }
    }
    Ok(stats)
}

pub fn replay_to_block_device(
    blocks: &[JournalReplayBlock],
    revokes: &mut RevokeTable,
    bdev: &BlockDeviceRef,
    block_size: u64,
) -> Result<ReplayStats, i32> {
    if block_size == 0 || block_size % 512 != 0 {
        return Err(EINVAL);
    }

    let mut stats = ReplayStats::default();
    for block in blocks {
        stats.last_sequence = block.sequence;
        match block.block_type {
            JBD2_DESCRIPTOR_BLOCK => {
                stats.descriptors += 1;
                if !revokes.is_revoked(block.target_block) {
                    write_replay_block(bdev, block.target_block, block_size, &block.payload)?;
                    stats.replayed += 1;
                }
            }
            JBD2_COMMIT_BLOCK => {
                stats.commits += 1;
            }
            JBD2_REVOKE_BLOCK => {
                revokes.revoke(block.target_block);
                stats.revoked += 1;
            }
            _ => return Err(EINVAL),
        }
    }
    Ok(stats)
}

fn write_replay_block(
    bdev: &BlockDeviceRef,
    target_block: u64,
    block_size: u64,
    payload: &[u8],
) -> Result<(), i32> {
    if payload.len() as u64 != block_size {
        return Err(EINVAL);
    }
    let sector = target_block.checked_mul(block_size / 512).ok_or(EINVAL)?;
    let bio = bio_alloc(bdev.clone(), BioOp(BIO_OP_WRITE), sector);
    bio.add_vec(BioVec::new(payload.to_vec()));
    submit_bio(bio)
}

pub fn checkpoint_can_drop(transaction_seq: u32, oldest_running_seq: u32) -> bool {
    transaction_seq < oldest_running_seq
}

pub fn commit_block(sequence: u32) -> JournalBlock {
    JournalBlock {
        sequence,
        block_type: JBD2_COMMIT_BLOCK,
        target_block: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::bio::{BIO_OP_READ, BioOp, BioVec, bio_alloc, submit_bio};
    use crate::block::block_device::BlockDevice;
    use crate::block::mem::{MemBlockDevice, mem_block_device_ops};

    #[test]
    fn replay_skips_revoked_blocks() {
        let blocks = [
            JournalBlock {
                sequence: 1,
                block_type: JBD2_REVOKE_BLOCK,
                target_block: 10,
            },
            JournalBlock {
                sequence: 1,
                block_type: JBD2_DESCRIPTOR_BLOCK,
                target_block: 10,
            },
            JournalBlock {
                sequence: 1,
                block_type: JBD2_DESCRIPTOR_BLOCK,
                target_block: 11,
            },
            commit_block(1),
        ];
        let mut revokes = RevokeTable::default();
        let stats = replay(&blocks, &mut revokes).unwrap();
        assert_eq!(stats.revoked, 1);
        assert_eq!(stats.replayed, 1);
        assert_eq!(stats.commits, 1);
        assert_eq!(revokes.len(), 1);
    }

    #[test]
    fn replay_to_block_device_writes_descriptor_payloads() {
        let mem = MemBlockDevice::new("jbd2-test0", 4096);
        let bdev = BlockDevice::wrap(mem, &mem_block_device_ops());
        let payload = alloc::vec![0x5a; 512];
        let blocks = [
            JournalReplayBlock {
                sequence: 7,
                block_type: JBD2_DESCRIPTOR_BLOCK,
                target_block: 3,
                payload: payload.clone(),
            },
            JournalReplayBlock {
                sequence: 7,
                block_type: JBD2_COMMIT_BLOCK,
                target_block: 0,
                payload: alloc::vec![],
            },
        ];

        let mut revokes = RevokeTable::default();
        let stats = replay_to_block_device(&blocks, &mut revokes, &bdev, 512).unwrap();
        assert_eq!(stats.descriptors, 1);
        assert_eq!(stats.commits, 1);
        assert_eq!(stats.replayed, 1);

        let read = bio_alloc(bdev, BioOp(BIO_OP_READ), 3);
        read.add_vec(BioVec::new(alloc::vec![0u8; 512]));
        submit_bio(read.clone()).unwrap();
        let vecs = read.vecs.lock();
        let data = vecs[0].data.lock();
        assert_eq!(&data[..], &payload[..]);
    }

    #[test]
    fn replay_to_block_device_skips_revoked_payloads() {
        let mem = MemBlockDevice::new("jbd2-test1", 4096);
        let bdev = BlockDevice::wrap(mem, &mem_block_device_ops());
        let blocks = [
            JournalReplayBlock {
                sequence: 8,
                block_type: JBD2_REVOKE_BLOCK,
                target_block: 2,
                payload: alloc::vec![],
            },
            JournalReplayBlock {
                sequence: 8,
                block_type: JBD2_DESCRIPTOR_BLOCK,
                target_block: 2,
                payload: alloc::vec![0xaa; 512],
            },
        ];

        let mut revokes = RevokeTable::default();
        let stats = replay_to_block_device(&blocks, &mut revokes, &bdev, 512).unwrap();
        assert_eq!(stats.revoked, 1);
        assert_eq!(stats.replayed, 0);

        let read = bio_alloc(bdev, BioOp(BIO_OP_READ), 2);
        read.add_vec(BioVec::new(alloc::vec![0xff; 512]));
        submit_bio(read.clone()).unwrap();
        let vecs = read.vecs.lock();
        let data = vecs[0].data.lock();
        assert!(data.iter().all(|&byte| byte == 0));
    }

    #[test]
    fn replay_to_block_device_rejects_short_payloads() {
        let mem = MemBlockDevice::new("jbd2-test2", 4096);
        let bdev = BlockDevice::wrap(mem, &mem_block_device_ops());
        let blocks = [JournalReplayBlock {
            sequence: 9,
            block_type: JBD2_DESCRIPTOR_BLOCK,
            target_block: 1,
            payload: alloc::vec![0u8; 511],
        }];

        let mut revokes = RevokeTable::default();
        assert_eq!(
            replay_to_block_device(&blocks, &mut revokes, &bdev, 512),
            Err(EINVAL)
        );
    }
}
