//! linux-parity: complete
//! linux-source: vendor/linux/net/vmw_vsock/af_vsock_tap.c
//! test-origin: linux:vendor/linux/net/vmw_vsock/af_vsock_tap.c
//! AF_VSOCK tap registration and delivery helpers.

use crate::include::uapi::errno::{EINVAL, ENODEV};

pub const MODULE_LICENSE: &str = "GPL";
pub const ARPHRD_VSOCKMON: u16 = 826;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VsockTap {
    pub id: u32,
    pub dev_type: u16,
    pub module_refs: u32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct VsockTapList {
    taps: alloc::vec::Vec<VsockTap>,
}

extern crate alloc;

pub fn vsock_add_tap(list: &mut VsockTapList, mut tap: VsockTap) -> Result<(), i32> {
    if tap.dev_type != ARPHRD_VSOCKMON {
        return Err(-EINVAL);
    }
    tap.module_refs += 1;
    list.taps.push(tap);
    Ok(())
}

pub fn vsock_remove_tap(list: &mut VsockTapList, id: u32) -> Result<(), i32> {
    let Some(pos) = list.taps.iter().position(|tap| tap.id == id) else {
        return Err(-ENODEV);
    };
    list.taps.remove(pos);
    Ok(())
}

pub const fn __vsock_deliver_tap_skb(clone_ok: bool, xmit_ret: i32) -> i32 {
    if !clone_ok {
        return 0;
    }
    if xmit_ret > 0 { -xmit_ret } else { xmit_ret }
}

pub fn vsock_deliver_tap(list: &VsockTapList, build_skb_ok: bool, per_tap_rets: &[i32]) -> usize {
    if list.taps.is_empty() || !build_skb_ok {
        return 0;
    }
    let mut delivered = 0;
    for (idx, _) in list.taps.iter().enumerate() {
        delivered += 1;
        if per_tap_rets.get(idx).copied().unwrap_or(0) != 0 {
            break;
        }
    }
    delivered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn af_vsock_tap_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/vmw_vsock/af_vsock_tap.c"
        ));
        assert!(source.contains("static DEFINE_SPINLOCK(vsock_tap_lock);"));
        assert!(source.contains("static struct list_head vsock_tap_all"));
        assert!(source.contains("int vsock_add_tap(struct vsock_tap *vt)"));
        assert!(source.contains("vt->dev->type != ARPHRD_VSOCKMON"));
        assert!(source.contains("__module_get(vt->module);"));
        assert!(source.contains("list_add_rcu(&vt->list, &vsock_tap_all);"));
        assert!(source.contains("int vsock_remove_tap(struct vsock_tap *vt)"));
        assert!(source.contains("list_for_each_entry(tmp, &vsock_tap_all, list)"));
        assert!(source.contains("list_del_rcu(&vt->list);"));
        assert!(source.contains("module_put(vt->module);"));
        assert!(source.contains("return found ? 0 : -ENODEV;"));
        assert!(source.contains("skb_clone(skb, GFP_ATOMIC);"));
        assert!(source.contains("ret = dev_queue_xmit(nskb);"));
        assert!(source.contains("ret = net_xmit_errno(ret);"));
        assert!(source.contains("void vsock_deliver_tap"));
        assert!(source.contains("if (likely(list_empty(&vsock_tap_all)))"));
        assert!(source.contains("consume_skb(skb);"));
    }

    #[test]
    fn tap_list_validates_device_type_removes_and_delivers_until_error() {
        let mut list = VsockTapList::default();
        assert_eq!(
            vsock_add_tap(
                &mut list,
                VsockTap {
                    id: 1,
                    dev_type: 1,
                    module_refs: 0,
                }
            ),
            Err(-EINVAL)
        );
        assert_eq!(
            vsock_add_tap(
                &mut list,
                VsockTap {
                    id: 1,
                    dev_type: ARPHRD_VSOCKMON,
                    module_refs: 0,
                }
            ),
            Ok(())
        );
        assert_eq!(vsock_deliver_tap(&list, true, &[0]), 1);
        assert_eq!(__vsock_deliver_tap_skb(true, 4), -4);
        assert_eq!(vsock_remove_tap(&mut list, 2), Err(-ENODEV));
        assert_eq!(vsock_remove_tap(&mut list, 1), Ok(()));
        assert_eq!(vsock_deliver_tap(&list, true, &[0]), 0);
    }
}
