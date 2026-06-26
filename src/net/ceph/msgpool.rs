//! linux-parity: complete
//! linux-source: vendor/linux/net/ceph/msgpool.c
//! test-origin: linux:vendor/linux/net/ceph/msgpool.c
//! Ceph message pool allocation and reset behavior.

use crate::include::uapi::errno::ENOMEM;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CephMsg {
    pub type_: i32,
    pub front_len: i32,
    pub hdr_front_len: i32,
    pub max_data_items: i32,
    pub data_length: usize,
    pub num_data_items: i32,
    pub kref: u32,
    pub pooled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CephMsgpool {
    pub type_: i32,
    pub front_len: i32,
    pub max_data_items: i32,
    pub size: i32,
    pub name: &'static str,
    pub available: i32,
    pub pool_created: bool,
}

pub fn msgpool_alloc(pool: &CephMsgpool) -> CephMsg {
    CephMsg {
        type_: pool.type_,
        front_len: pool.front_len,
        hdr_front_len: pool.front_len,
        max_data_items: pool.max_data_items,
        data_length: 0,
        num_data_items: 0,
        kref: 1,
        pooled: true,
    }
}

pub fn msgpool_free(msg: &mut CephMsg) {
    msg.pooled = false;
}

pub fn ceph_msgpool_init(
    type_: i32,
    front_len: i32,
    max_data_items: i32,
    size: i32,
    name: &'static str,
    mempool_ok: bool,
) -> Result<CephMsgpool, i32> {
    if !mempool_ok {
        return Err(-ENOMEM);
    }
    Ok(CephMsgpool {
        type_,
        front_len,
        max_data_items,
        size,
        name,
        available: size,
        pool_created: true,
    })
}

pub fn ceph_msgpool_destroy(pool: &mut CephMsgpool) {
    pool.pool_created = false;
    pool.available = 0;
}

pub fn ceph_msgpool_get(pool: &mut CephMsgpool, front_len: i32, max_data_items: i32) -> CephMsg {
    if front_len > pool.front_len || max_data_items > pool.max_data_items {
        return CephMsg {
            type_: pool.type_,
            front_len,
            hdr_front_len: front_len,
            max_data_items,
            data_length: 0,
            num_data_items: 0,
            kref: 1,
            pooled: false,
        };
    }

    pool.available = pool.available.saturating_sub(1);
    msgpool_alloc(pool)
}

pub fn ceph_msgpool_put(pool: &mut CephMsgpool, msg: &mut CephMsg) {
    msg.front_len = pool.front_len;
    msg.hdr_front_len = pool.front_len;
    msg.data_length = 0;
    msg.num_data_items = 0;
    msg.kref = 1;
    msg.pooled = true;
    pool.available = pool.available.saturating_add(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ceph_msgpool_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ceph/msgpool.c"
        ));
        assert!(source.contains("static void *msgpool_alloc"));
        assert!(source.contains("ceph_msg_new2(pool->type, pool->front_len, pool->max_data_items"));
        assert!(source.contains("msg->pool = pool;"));
        assert!(source.contains("static void msgpool_free"));
        assert!(source.contains("msg->pool = NULL;"));
        assert!(source.contains("ceph_msg_put(msg);"));
        assert!(source.contains("int ceph_msgpool_init"));
        assert!(source.contains("pool->type = type;"));
        assert!(source.contains("pool->front_len = front_len;"));
        assert!(source.contains("pool->max_data_items = max_data_items;"));
        assert!(source.contains("mempool_create(size, msgpool_alloc, msgpool_free, pool);"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("pool->name = name;"));
        assert!(source.contains("void ceph_msgpool_destroy"));
        assert!(source.contains("mempool_destroy(pool->pool);"));
        assert!(source.contains("struct ceph_msg *ceph_msgpool_get"));
        assert!(source.contains("if (front_len > pool->front_len ||"));
        assert!(source.contains("return ceph_msg_new2(pool->type, front_len, max_data_items"));
        assert!(source.contains("msg = mempool_alloc(pool->pool, GFP_NOFS);"));
        assert!(source.contains("void ceph_msgpool_put"));
        assert!(source.contains("msg->front.iov_len = pool->front_len;"));
        assert!(source.contains("msg->hdr.front_len = cpu_to_le32(pool->front_len);"));
        assert!(source.contains("msg->data_length = 0;"));
        assert!(source.contains("msg->num_data_items = 0;"));
        assert!(source.contains("kref_init(&msg->kref);"));
        assert!(source.contains("mempool_free(msg, pool->pool);"));
    }

    #[test]
    fn msgpool_get_falls_back_for_oversized_messages_and_put_resets() {
        assert_eq!(ceph_msgpool_init(1, 64, 2, 4, "pool", false), Err(-ENOMEM));
        let mut pool = ceph_msgpool_init(1, 64, 2, 4, "pool", true).unwrap();
        let mut pooled = ceph_msgpool_get(&mut pool, 32, 1);
        assert!(pooled.pooled);
        assert_eq!(pool.available, 3);
        let fresh = ceph_msgpool_get(&mut pool, 128, 1);
        assert!(!fresh.pooled);
        assert_eq!(fresh.front_len, 128);

        pooled.front_len = 10;
        pooled.hdr_front_len = 10;
        pooled.data_length = 99;
        pooled.num_data_items = 3;
        pooled.kref = 7;
        ceph_msgpool_put(&mut pool, &mut pooled);
        assert_eq!(pooled.front_len, 64);
        assert_eq!(pooled.hdr_front_len, 64);
        assert_eq!(pooled.data_length, 0);
        assert_eq!(pooled.num_data_items, 0);
        assert_eq!(pooled.kref, 1);
        assert_eq!(pool.available, 4);
        ceph_msgpool_destroy(&mut pool);
        assert!(!pool.pool_created);
    }
}
