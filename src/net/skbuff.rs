//! linux-parity: complete
//! linux-source: vendor/linux/net
//! test-origin: linux:vendor/linux/net
//! `struct sk_buff` core.
//!
//! This mirrors the Linux packet-buffer model closely enough for the rest of
//! the networking stack: packet data lives between `data..tail`, headroom is
//! available before `data`, tailroom is available after `tail`, and clones use
//! copy-on-write before mutation.

extern crate alloc;

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::include::uapi::errno::{EINVAL, ENOMEM, ENOSPC};

pub const SKB_CB_LEN: usize = 48;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SkbSharedInfo {
    pub nr_frags: u8,
    pub gso_size: u16,
    pub gso_type: u16,
    pub tx_flags: u32,
}

#[derive(Clone)]
pub struct SkBuff {
    storage: Arc<Vec<u8>>,
    pub head: usize,
    pub data: usize,
    pub tail: usize,
    pub end: usize,
    pub len: usize,
    pub data_len: usize,
    pub truesize: usize,
    pub cb: [u8; SKB_CB_LEN],
    pub shared_info: SkbSharedInfo,
    pub frag_list: Option<Box<SkBuff>>,
}

impl SkBuff {
    pub fn headroom(&self) -> usize {
        self.data - self.head
    }

    pub fn tailroom(&self) -> usize {
        self.end - self.tail
    }

    pub fn cloned(&self) -> bool {
        Arc::strong_count(&self.storage) > 1
    }

    pub fn data(&self) -> &[u8] {
        &self.storage[self.data..self.tail]
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        self.ensure_unique();
        let storage = Arc::get_mut(&mut self.storage).expect("skb storage is unique");
        &mut storage[self.data..self.tail]
    }

    fn ensure_unique(&mut self) {
        if self.cloned() {
            self.storage = Arc::new((*self.storage).clone());
        }
    }
}

pub fn alloc_skb(size: usize) -> Result<SkBuff, i32> {
    let mut storage = Vec::new();
    storage.try_reserve_exact(size).map_err(|_| ENOMEM)?;
    storage.resize(size, 0);
    Ok(SkBuff {
        storage: Arc::new(storage),
        head: 0,
        data: 0,
        tail: 0,
        end: size,
        len: 0,
        data_len: 0,
        truesize: core::mem::size_of::<SkBuff>() + size,
        cb: [0; SKB_CB_LEN],
        shared_info: SkbSharedInfo::default(),
        frag_list: None,
    })
}

pub fn skb_reserve(skb: &mut SkBuff, len: usize) -> Result<(), i32> {
    if skb.len != 0 || len > skb.tailroom() {
        return Err(EINVAL);
    }
    skb.data += len;
    skb.tail += len;
    Ok(())
}

pub fn skb_put(skb: &mut SkBuff, len: usize) -> Result<&mut [u8], i32> {
    if len > skb.tailroom() {
        return Err(ENOSPC);
    }
    skb.ensure_unique();
    let start = skb.tail;
    skb.tail += len;
    skb.len += len;
    let storage = Arc::get_mut(&mut skb.storage).expect("skb storage is unique");
    Ok(&mut storage[start..start + len])
}

pub fn skb_push(skb: &mut SkBuff, len: usize) -> Result<&mut [u8], i32> {
    if len > skb.headroom() {
        return Err(ENOSPC);
    }
    skb.ensure_unique();
    skb.data -= len;
    skb.len += len;
    let storage = Arc::get_mut(&mut skb.storage).expect("skb storage is unique");
    Ok(&mut storage[skb.data..skb.data + len])
}

pub fn skb_pull(skb: &mut SkBuff, len: usize) -> Result<&[u8], i32> {
    if len > skb.len {
        return Err(EINVAL);
    }
    skb.data += len;
    skb.len -= len;
    Ok(skb.data())
}

pub fn skb_trim(skb: &mut SkBuff, len: usize) -> Result<(), i32> {
    if len > skb.len {
        return Err(EINVAL);
    }
    skb.tail = skb.data + len;
    skb.len = len;
    Ok(())
}

pub fn skb_pad(skb: &mut SkBuff, pad: usize) -> Result<(), i32> {
    if pad > skb.tailroom() {
        pskb_expand_head(skb, 0, pad - skb.tailroom())?;
    }
    let out = skb_put(skb, pad)?;
    out.fill(0);
    Ok(())
}

pub fn skb_clone(skb: &SkBuff) -> SkBuff {
    skb.clone()
}

pub fn skb_copy(skb: &SkBuff) -> Result<SkBuff, i32> {
    let mut storage = Vec::new();
    storage
        .try_reserve_exact(skb.storage.len())
        .map_err(|_| ENOMEM)?;
    storage.extend_from_slice(&skb.storage);
    Ok(SkBuff {
        storage: Arc::new(storage),
        head: skb.head,
        data: skb.data,
        tail: skb.tail,
        end: skb.end,
        len: skb.len,
        data_len: skb.data_len,
        truesize: skb.truesize,
        cb: skb.cb,
        shared_info: skb.shared_info.clone(),
        frag_list: skb
            .frag_list
            .as_ref()
            .map(|frag| Box::new((**frag).clone())),
    })
}

pub fn skb_share_check(skb: SkBuff) -> Result<SkBuff, i32> {
    if skb.cloned() {
        skb_copy(&skb)
    } else {
        Ok(skb)
    }
}

pub fn pskb_expand_head(skb: &mut SkBuff, extra_head: usize, extra_tail: usize) -> Result<(), i32> {
    let packet_len = skb.len;
    let new_end = extra_head
        .checked_add(packet_len)
        .and_then(|v| v.checked_add(extra_tail))
        .ok_or(ENOMEM)?;

    let mut storage = Vec::new();
    storage.try_reserve_exact(new_end).map_err(|_| ENOMEM)?;
    storage.resize(new_end, 0);
    storage[extra_head..extra_head + packet_len].copy_from_slice(skb.data());

    skb.storage = Arc::new(storage);
    skb.head = 0;
    skb.data = extra_head;
    skb.tail = extra_head + packet_len;
    skb.end = new_end;
    skb.truesize = core::mem::size_of::<SkBuff>() + new_end;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skb_clone_share_check_preserves_original_on_mutation() {
        let mut skb = alloc_skb(64).unwrap();
        skb_reserve(&mut skb, 16).unwrap();
        skb_put(&mut skb, 4).unwrap().copy_from_slice(&[1, 2, 3, 4]);

        let clone = skb_clone(&skb);
        assert!(skb.cloned());

        let mut writable = skb_share_check(clone).unwrap();
        writable.data_mut()[0] = 9;

        assert_eq!(skb.data(), &[1, 2, 3, 4]);
        assert_eq!(writable.data(), &[9, 2, 3, 4]);
    }

    #[test]
    fn pskb_expand_head_preserves_packet_bytes() {
        let mut skb = alloc_skb(8).unwrap();
        skb_put(&mut skb, 3)
            .unwrap()
            .copy_from_slice(&[0xaa, 0xbb, 0xcc]);
        pskb_expand_head(&mut skb, 8, 8).unwrap();

        assert_eq!(skb.headroom(), 8);
        assert_eq!(skb.tailroom(), 8);
        assert_eq!(skb.data(), &[0xaa, 0xbb, 0xcc]);
    }
}
