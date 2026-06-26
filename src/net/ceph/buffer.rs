//! linux-parity: complete
//! linux-source: vendor/linux/net/ceph/buffer.c
//! test-origin: linux:vendor/linux/net/ceph/buffer.c
//! Ceph refcounted buffer allocation and decode helper.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::{EINVAL, ENOMEM};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CephBuffer {
    pub alloc_len: usize,
    pub vec: Vec<u8>,
    pub kref: usize,
}

pub fn ceph_buffer_new(len: usize) -> Result<CephBuffer, i32> {
    let mut vec = Vec::new();
    vec.try_reserve_exact(len).map_err(|_| ENOMEM)?;
    vec.resize(len, 0);
    Ok(CephBuffer {
        alloc_len: len,
        vec,
        kref: 1,
    })
}

pub fn ceph_buffer_release(buffer: &mut Option<CephBuffer>) {
    *buffer = None;
}

pub fn ceph_decode_buffer(cursor: &mut &[u8]) -> Result<CephBuffer, i32> {
    if cursor.len() < 4 {
        return Err(EINVAL);
    }
    let len = u32::from_le_bytes([cursor[0], cursor[1], cursor[2], cursor[3]]) as usize;
    *cursor = &cursor[4..];
    if cursor.len() < len {
        return Err(EINVAL);
    }
    let mut buffer = ceph_buffer_new(len)?;
    buffer.vec.copy_from_slice(&cursor[..len]);
    *cursor = &cursor[len..];
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ceph_buffer_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ceph/buffer.c"
        ));
        assert!(source.contains("struct ceph_buffer *ceph_buffer_new(size_t len"));
        assert!(source.contains("b = kmalloc_obj(*b, gfp);"));
        assert!(source.contains("b->vec.iov_base = kvmalloc(len, gfp);"));
        assert!(source.contains("kref_init(&b->kref);"));
        assert!(source.contains("b->alloc_len = len;"));
        assert!(source.contains("b->vec.iov_len = len;"));
        assert!(source.contains("void ceph_buffer_release(struct kref *kref)"));
        assert!(source.contains("kvfree(b->vec.iov_base);"));
        assert!(source.contains("int ceph_decode_buffer(struct ceph_buffer **b"));
        assert!(source.contains("len = ceph_decode_32(p);"));
        assert!(source.contains("ceph_decode_copy(p, (*b)->vec.iov_base, len);"));
        assert!(source.contains("return -EINVAL;"));

        let buffer = ceph_buffer_new(4).unwrap();
        assert_eq!(buffer.alloc_len, 4);
        assert_eq!(buffer.vec, alloc::vec![0, 0, 0, 0]);
        assert_eq!(buffer.kref, 1);

        let mut bytes: &[u8] = &[3, 0, 0, 0, 9, 8, 7, 6];
        let decoded = ceph_decode_buffer(&mut bytes).unwrap();
        assert_eq!(decoded.vec, alloc::vec![9, 8, 7]);
        assert_eq!(bytes, &[6]);
        assert_eq!(ceph_decode_buffer(&mut &[4, 0, 0, 0, 1][..]), Err(EINVAL));

        let mut owned = Some(decoded);
        ceph_buffer_release(&mut owned);
        assert_eq!(owned, None);
    }
}
