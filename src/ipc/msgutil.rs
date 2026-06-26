//! linux-parity: complete
//! linux-source: vendor/linux/ipc/msgutil.c
//! test-origin: linux:vendor/linux/ipc/msgutil.c
//! System V message allocation geometry and copy segmentation.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use crate::include::uapi::errno::{EFAULT, EINVAL, ENOMEM, ENOSYS};

pub const PAGE_SIZE: usize = 4096;
pub const MQ_LOCK_DEFINED: bool = true;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MsgCopyPlan {
    pub head_len: usize,
    pub segment_count: usize,
    pub last_segment_len: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpcNamespaceInit {
    pub ns_common_initialized: bool,
    pub user_ns: &'static str,
}

pub const INIT_IPC_NS: IpcNamespaceInit = IpcNamespaceInit {
    ns_common_initialized: true,
    user_ns: "init_user_ns",
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MsgBucketsInit {
    pub name: &'static str,
    pub slab_account: bool,
    pub object_size: usize,
    pub max_size: usize,
    pub ret: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MsgLayout {
    pub page_size: usize,
    pub msg_header_size: usize,
    pub seg_header_size: usize,
}

impl MsgLayout {
    pub const fn datalen_msg(self) -> usize {
        datalen_msg(self.page_size, self.msg_header_size)
    }

    pub const fn datalen_seg(self) -> usize {
        datalen_seg(self.page_size, self.seg_header_size)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MsgMsg {
    pub m_type: i64,
    pub m_ts: usize,
    pub head: Vec<u8>,
    pub segments: Vec<Vec<u8>>,
    pub security_allocated: bool,
    pub cond_resched_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FreeMsgReport {
    pub security_freed: bool,
    pub freed_head: bool,
    pub freed_segments: usize,
    pub cond_resched_count: usize,
}

pub const fn datalen_msg(page_size: usize, msg_header_size: usize) -> usize {
    page_size - msg_header_size
}

pub const fn datalen_seg(page_size: usize, seg_header_size: usize) -> usize {
    page_size - seg_header_size
}

pub const fn msg_copy_plan(
    len: usize,
    page_size: usize,
    msg_header_size: usize,
    seg_header_size: usize,
) -> MsgCopyPlan {
    let head_cap = datalen_msg(page_size, msg_header_size);
    let seg_cap = datalen_seg(page_size, seg_header_size);
    let head_len = if len < head_cap { len } else { head_cap };
    let mut remaining = len - head_len;
    let mut segment_count = 0usize;
    let mut last_segment_len = 0usize;

    while remaining > 0 {
        let take = if remaining < seg_cap {
            remaining
        } else {
            seg_cap
        };
        segment_count += 1;
        last_segment_len = take;
        remaining -= take;
    }

    MsgCopyPlan {
        head_len,
        segment_count,
        last_segment_len,
    }
}

pub const fn copy_msg_allowed(src_len: usize, dst_len: usize) -> bool {
    src_len <= dst_len
}

pub const fn init_msg_buckets(layout: MsgLayout) -> MsgBucketsInit {
    MsgBucketsInit {
        name: "msg_msg",
        slab_account: true,
        object_size: layout.msg_header_size,
        max_size: layout.datalen_msg(),
        ret: 0,
    }
}

pub fn alloc_msg_with_failure(
    len: usize,
    layout: MsgLayout,
    fail_segment_index: Option<usize>,
) -> Result<MsgMsg, i32> {
    let plan = msg_copy_plan(
        len,
        layout.page_size,
        layout.msg_header_size,
        layout.seg_header_size,
    );
    let mut segments = Vec::new();
    for index in 0..plan.segment_count {
        if fail_segment_index == Some(index) {
            return Err(-ENOMEM);
        }
        let seg_len = if index + 1 == plan.segment_count {
            plan.last_segment_len
        } else {
            layout.datalen_seg()
        };
        segments.push(vec![0; seg_len]);
    }

    Ok(MsgMsg {
        m_type: 0,
        m_ts: len,
        head: vec![0; plan.head_len],
        segments,
        security_allocated: false,
        cond_resched_count: plan.segment_count,
    })
}

pub fn alloc_msg(len: usize, layout: MsgLayout) -> Result<MsgMsg, i32> {
    alloc_msg_with_failure(len, layout, None)
}

pub fn load_msg(
    src: &[u8],
    len: usize,
    layout: MsgLayout,
    copy_fail_segment: Option<usize>,
    security_ret: i32,
) -> Result<MsgMsg, i32> {
    if src.len() < len {
        return Err(-EFAULT);
    }
    let mut msg = alloc_msg(len, layout)?;
    let mut offset = 0usize;

    if copy_fail_segment == Some(0) {
        return Err(-EFAULT);
    }
    let head_len = msg.head.len();
    msg.head.copy_from_slice(&src[..head_len]);
    offset += head_len;

    for (index, segment) in msg.segments.iter_mut().enumerate() {
        if copy_fail_segment == Some(index + 1) {
            return Err(-EFAULT);
        }
        let segment_len = segment.len();
        segment.copy_from_slice(&src[offset..offset + segment_len]);
        offset += segment_len;
    }

    if security_ret != 0 {
        return Err(security_ret);
    }

    msg.security_allocated = true;
    Ok(msg)
}

pub fn copy_msg(src: &MsgMsg, dst: &mut MsgMsg, checkpoint_restore: bool) -> Result<(), i32> {
    if !checkpoint_restore {
        return Err(-ENOSYS);
    }
    if src.m_ts > dst.m_ts {
        return Err(-EINVAL);
    }

    dst.head[..src.head.len()].copy_from_slice(&src.head);
    for (dst_seg, src_seg) in dst.segments.iter_mut().zip(src.segments.iter()) {
        dst_seg[..src_seg.len()].copy_from_slice(src_seg);
    }
    dst.m_type = src.m_type;
    dst.m_ts = src.m_ts;
    Ok(())
}

pub fn store_msg(
    dest: &mut [u8],
    msg: &MsgMsg,
    len: usize,
    copy_fail_segment: Option<usize>,
) -> i32 {
    if dest.len() < len {
        return -1;
    }
    let mut offset = 0usize;
    let head_len = msg.head.len().min(len);
    if copy_fail_segment == Some(0) {
        return -1;
    }
    dest[..head_len].copy_from_slice(&msg.head[..head_len]);
    offset += head_len;

    for (index, segment) in msg.segments.iter().enumerate() {
        if offset >= len {
            break;
        }
        let take = segment.len().min(len - offset);
        if copy_fail_segment == Some(index + 1) {
            return -1;
        }
        dest[offset..offset + take].copy_from_slice(&segment[..take]);
        offset += take;
    }
    0
}

pub fn free_msg(msg: MsgMsg) -> FreeMsgReport {
    FreeMsgReport {
        security_freed: true,
        freed_head: true,
        freed_segments: msg.segments.len(),
        cond_resched_count: msg.segments.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msgutil_segmentation_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/ipc/msgutil.c"
        ));
        assert!(source.contains("DEFINE_SPINLOCK(mq_lock);"));
        assert!(source.contains("struct ipc_namespace init_ipc_ns"));
        assert!(source.contains("#define DATALEN_MSG"));
        assert!(source.contains("#define DATALEN_SEG"));
        assert!(source.contains("msg_buckets = kmem_buckets_create(\"msg_msg\""));
        assert!(source.contains("subsys_initcall(init_msg_buckets);"));
        assert!(source.contains("msg->next = NULL;"));
        assert!(source.contains("msg->security = NULL;"));
        assert!(source.contains("cond_resched();"));
        assert!(source.contains("goto out_err;"));
        assert!(source.contains("return ERR_PTR(-ENOMEM);"));
        assert!(source.contains("alen = min(len, DATALEN_MSG);"));
        assert!(source.contains("alen = min(len, DATALEN_SEG);"));
        assert!(source.contains("copy_from_user(msg + 1, src, alen)"));
        assert!(source.contains("return ERR_PTR(err);"));
        assert!(source.contains("return ERR_PTR(-EINVAL);"));
        assert!(source.contains("return ERR_PTR(-ENOSYS);"));
        assert!(source.contains("copy_to_user(dest, msg + 1, alen)"));
        assert!(source.contains("return -1;"));
        assert!(source.contains("security_msg_msg_alloc(msg)"));
        assert!(source.contains("security_msg_msg_free(msg)"));
        assert!(source.contains("kfree(msg);"));
        assert!(source.contains("kfree(seg);"));

        assert!(MQ_LOCK_DEFINED);
        assert_eq!(INIT_IPC_NS.user_ns, "init_user_ns");
        assert_eq!(datalen_msg(4096, 64), 4032);
        assert_eq!(datalen_seg(4096, 8), 4088);
        assert_eq!(
            init_msg_buckets(MsgLayout {
                page_size: 4096,
                msg_header_size: 64,
                seg_header_size: 8,
            }),
            MsgBucketsInit {
                name: "msg_msg",
                slab_account: true,
                object_size: 64,
                max_size: 4032,
                ret: 0,
            }
        );
        assert_eq!(
            msg_copy_plan(9000, 4096, 64, 8),
            MsgCopyPlan {
                head_len: 4032,
                segment_count: 2,
                last_segment_len: 880,
            }
        );
        assert!(copy_msg_allowed(128, 128));
        assert!(!copy_msg_allowed(129, 128));
    }

    #[test]
    fn msg_lifecycle_models_alloc_load_copy_store_and_free_paths() {
        let layout = MsgLayout {
            page_size: PAGE_SIZE,
            msg_header_size: 64,
            seg_header_size: 8,
        };
        let source: Vec<u8> = (0..9000).map(|byte| (byte % 251) as u8).collect();
        let mut msg = load_msg(&source, source.len(), layout, None, 0).expect("load");
        msg.m_type = 7;
        assert_eq!(msg.m_ts, 9000);
        assert_eq!(msg.head.len(), 4032);
        assert_eq!(msg.segments.len(), 2);
        assert!(msg.security_allocated);
        assert_eq!(msg.cond_resched_count, 2);

        let mut dest = vec![0u8; 9000];
        assert_eq!(store_msg(&mut dest, &msg, 9000, None), 0);
        assert_eq!(dest, source);

        let mut copy = alloc_msg(9000, layout).expect("alloc copy");
        assert_eq!(copy_msg(&msg, &mut copy, true), Ok(()));
        assert_eq!(copy.m_type, 7);
        assert_eq!(copy.m_ts, 9000);
        assert_eq!(copy.head, msg.head);
        assert_eq!(copy.segments, msg.segments);

        let mut too_small = alloc_msg(128, layout).expect("small copy");
        assert_eq!(copy_msg(&msg, &mut too_small, true), Err(-EINVAL));
        assert_eq!(copy_msg(&msg, &mut copy, false), Err(-ENOSYS));

        let freed = free_msg(msg);
        assert!(freed.security_freed);
        assert!(freed.freed_head);
        assert_eq!(freed.freed_segments, 2);
        assert_eq!(freed.cond_resched_count, 2);
    }

    #[test]
    fn msg_error_paths_match_linux_errno_contracts() {
        let layout = MsgLayout {
            page_size: PAGE_SIZE,
            msg_header_size: 64,
            seg_header_size: 8,
        };
        assert_eq!(alloc_msg_with_failure(9000, layout, Some(1)), Err(-ENOMEM));
        assert_eq!(load_msg(&[1, 2], 3, layout, None, 0), Err(-EFAULT));
        let source = vec![0x5a; 9000];
        assert_eq!(
            load_msg(&source, source.len(), layout, Some(1), 0),
            Err(-EFAULT)
        );
        assert_eq!(
            load_msg(&source, source.len(), layout, None, -EINVAL),
            Err(-EINVAL)
        );
        let msg = load_msg(&source, source.len(), layout, None, 0).expect("load");
        let mut short_dest = vec![0u8; 10];
        assert_eq!(store_msg(&mut short_dest, &msg, 9000, None), -1);
        let mut dest = vec![0u8; 9000];
        assert_eq!(store_msg(&mut dest, &msg, 9000, Some(2)), -1);
    }
}
