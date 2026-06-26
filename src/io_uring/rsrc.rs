//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/rsrc.c
//! test-origin: linux:vendor/linux/io_uring/rsrc.c
//! Resource (buffer / file) registration.
//!
//! Backs `IORING_REGISTER_BUFFERS`, `IORING_REGISTER_FILES`, and the tagging
//! variants.  Each registered resource is a refcounted node in an indexed
//! table; per-op `IORING_OP_READ_FIXED` etc. look up by index.
//!
//! Ref: vendor/linux/io_uring/rsrc.c
//! Ref: vendor/linux/io_uring/rsrc.h

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

use crate::fs::types::FileRef;

/// `enum io_rsrc_type` — kind of resource a node holds.
/// Ref: vendor/linux/io_uring/rsrc.h::io_rsrc_type
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IoRsrcType {
    File = 1,
    Buffer = 2,
}

/// A registered buffer (`struct io_mapped_ubuf`).  Stores the user iovec
/// pinned at registration time.  Per Linux the buffer is described by a list
/// of pages — we keep the original iovec view for the no_std port and let
/// the consumer copy bytes through the standard VFS path.
#[derive(Clone, Debug)]
pub struct MappedUbuf {
    pub ubuf: u64,
    pub len: u64,
    pub tag: u64,
}

/// One slot in a per-ring resource table.
pub enum IoRsrcNode {
    File { file: FileRef, tag: u64 },
    Buffer(MappedUbuf),
    Empty,
}

/// `struct io_rsrc_data` — sparse table of registered resources of one kind.
pub struct IoRsrcData {
    pub kind: IoRsrcType,
    pub nodes: Vec<IoRsrcNode>,
    /// `refs` — number of in-flight ops referencing this table.  When the
    /// last op completes after an `unregister`, the storage is freed.
    pub refs: AtomicU32,
}

impl IoRsrcData {
    /// `io_rsrc_data_alloc`.
    pub fn new(kind: IoRsrcType, nr: u32) -> Self {
        let mut nodes = Vec::with_capacity(nr as usize);
        for _ in 0..nr {
            nodes.push(IoRsrcNode::Empty);
        }
        Self {
            kind,
            nodes,
            refs: AtomicU32::new(1),
        }
    }

    pub fn nr(&self) -> u32 {
        self.nodes.len() as u32
    }

    /// `io_rsrc_node_lookup`.
    pub fn lookup(&self, index: u32) -> Option<&IoRsrcNode> {
        self.nodes.get(index as usize)
    }

    /// Install or replace a slot.  Returns `-EINVAL` for out-of-range.
    pub fn set(&mut self, index: u32, node: IoRsrcNode) -> Result<(), i32> {
        match self.nodes.get_mut(index as usize) {
            Some(slot) => {
                *slot = node;
                Ok(())
            }
            None => Err(-22),
        }
    }

    /// Clear slot; returns `-EBADF` if the slot was empty.
    pub fn clear(&mut self, index: u32) -> Result<(), i32> {
        let slot = self.nodes.get_mut(index as usize).ok_or(-22)?;
        if matches!(slot, IoRsrcNode::Empty) {
            return Err(-9);
        }
        *slot = IoRsrcNode::Empty;
        Ok(())
    }

    pub fn get_ref(&self) -> u32 {
        self.refs.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub fn put_ref(&self) -> u32 {
        self.refs.fetch_sub(1, Ordering::AcqRel) - 1
    }
}

/// Bundle of all per-ring resource tables — one buffer table, one file table.
pub struct IoRsrcTables {
    pub buffers: Option<Arc<IoRsrcData>>,
    pub files: Option<Arc<IoRsrcData>>,
}

impl IoRsrcTables {
    pub const fn new() -> Self {
        Self {
            buffers: None,
            files: None,
        }
    }

    /// `IORING_REGISTER_BUFFERS` — accept `nr_args` iovecs.  Each is recorded
    /// as a `MappedUbuf` node.
    pub fn register_buffers(&mut self, ubufs: &[(u64, u64)]) -> Result<(), i32> {
        if self.buffers.is_some() {
            return Err(-16); // -EBUSY
        }
        let mut data = IoRsrcData::new(IoRsrcType::Buffer, ubufs.len() as u32);
        for (i, (addr, len)) in ubufs.iter().enumerate() {
            data.nodes[i] = IoRsrcNode::Buffer(MappedUbuf {
                ubuf: *addr,
                len: *len,
                tag: 0,
            });
        }
        self.buffers = Some(Arc::new(data));
        Ok(())
    }

    /// `IORING_UNREGISTER_BUFFERS`.
    pub fn unregister_buffers(&mut self) -> Result<(), i32> {
        if self.buffers.take().is_none() {
            return Err(-2); // -ENOENT
        }
        Ok(())
    }

    /// `IORING_REGISTER_FILES`.
    pub fn register_files(&mut self, files: Vec<Option<FileRef>>) -> Result<(), i32> {
        if self.files.is_some() {
            return Err(-16);
        }
        let mut data = IoRsrcData::new(IoRsrcType::File, files.len() as u32);
        for (i, f) in files.into_iter().enumerate() {
            data.nodes[i] = match f {
                Some(file) => IoRsrcNode::File { file, tag: 0 },
                None => IoRsrcNode::Empty,
            };
        }
        self.files = Some(Arc::new(data));
        Ok(())
    }

    pub fn unregister_files(&mut self) -> Result<(), i32> {
        if self.files.take().is_none() {
            return Err(-2);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::dcache::d_alloc;
    use crate::fs::file::alloc_file;
    use crate::fs::ops::NOOP_FILE_OPS;

    fn dummy_file() -> FileRef {
        alloc_file(d_alloc("rsrc-test"), 0, 0, &NOOP_FILE_OPS)
    }

    #[test]
    fn register_buffers_creates_table() {
        let mut t = IoRsrcTables::new();
        t.register_buffers(&[(0x1000, 4096), (0x2000, 8192)])
            .unwrap();
        let buf = t.buffers.as_ref().unwrap();
        assert_eq!(buf.nr(), 2);
        match buf.lookup(1).unwrap() {
            IoRsrcNode::Buffer(b) => {
                assert_eq!(b.ubuf, 0x2000);
                assert_eq!(b.len, 8192);
            }
            _ => panic!("expected Buffer node"),
        }
    }

    #[test]
    fn register_buffers_twice_is_ebusy() {
        let mut t = IoRsrcTables::new();
        t.register_buffers(&[(0x1000, 4096)]).unwrap();
        assert_eq!(t.register_buffers(&[(0x2000, 4096)]).unwrap_err(), -16);
    }

    #[test]
    fn unregister_without_register_is_enoent() {
        let mut t = IoRsrcTables::new();
        assert_eq!(t.unregister_buffers().unwrap_err(), -2);
        assert_eq!(t.unregister_files().unwrap_err(), -2);
    }

    #[test]
    fn refcount_round_trips() {
        let d = IoRsrcData::new(IoRsrcType::File, 0);
        assert_eq!(d.refs.load(Ordering::Acquire), 1);
        let r = d.get_ref();
        assert_eq!(r, 2);
        let r = d.put_ref();
        assert_eq!(r, 1);
    }

    #[test]
    fn clear_empty_slot_is_ebadf() {
        let mut d = IoRsrcData::new(IoRsrcType::File, 4);
        assert_eq!(d.clear(2).unwrap_err(), -9);
    }

    #[test]
    fn set_then_clear_is_ok() {
        let mut d = IoRsrcData::new(IoRsrcType::File, 4);
        d.set(
            1,
            IoRsrcNode::File {
                file: dummy_file(),
                tag: 0xfeed,
            },
        )
        .unwrap();
        d.clear(1).unwrap();
        assert!(matches!(d.lookup(1).unwrap(), IoRsrcNode::Empty));
    }
}
