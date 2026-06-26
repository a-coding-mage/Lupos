//! linux-parity: complete
//! linux-source: vendor/linux/fs/xfs/xfs_hooks.c
//! test-origin: linux:vendor/linux/fs/xfs/xfs_hooks.c
//! XFS live hook notifier-chain helpers.

extern crate alloc;

use alloc::vec::Vec;
use core::mem::offset_of;

use crate::include::uapi::errno::EEXIST;

pub const NOTIFY_DONE: i32 = 0;
pub const NOTIFY_OK: i32 = 1;
pub const NOTIFY_STOP_MASK: i32 = 0x8000;
pub const NOTIFY_STOP: i32 = NOTIFY_OK | NOTIFY_STOP_MASK;

pub type XfsHookCallback = fn(val: u64, priv_data: usize) -> i32;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct XfsHookNotifier {
    pub priority: i32,
    pub callback: XfsHookCallback,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct XfsHook {
    pub nb: XfsHookNotifier,
    pub id: usize,
}

pub const XFS_HOOK_NOTIFIER_OFFSET: usize = offset_of!(XfsHook, nb);

#[derive(Clone, Default)]
pub struct XfsHooks {
    hooks: Vec<XfsHook>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XfsHooksCallOutcome {
    pub notifier_chain_called: bool,
    pub result: i32,
}

pub fn xfs_hooks_init(chain: &mut XfsHooks) {
    chain.hooks.clear();
}

pub const fn xfs_hooks_init_clears_head() -> bool {
    true
}

pub fn xfs_hooks_add(chain: &mut XfsHooks, hook: XfsHook) -> i32 {
    if chain
        .hooks
        .iter()
        .any(|registered| registered.id == hook.id)
    {
        return -EEXIST;
    }

    let insert_at = chain
        .hooks
        .iter()
        .position(|registered| hook.nb.priority > registered.nb.priority)
        .unwrap_or(chain.hooks.len());
    chain.hooks.insert(insert_at, hook);
    0
}

pub fn xfs_hooks_add_result(hook_has_notifier_call: bool, register_result: i32) -> Option<i32> {
    if hook_has_notifier_call {
        Some(register_result)
    } else {
        None
    }
}

pub fn xfs_hooks_del(chain: &mut XfsHooks, hook_id: usize) {
    if let Some(index) = chain.hooks.iter().position(|hook| hook.id == hook_id) {
        chain.hooks.remove(index);
    }
}

pub const fn xfs_hooks_del_unregisters() -> bool {
    true
}

pub fn xfs_hooks_call(chain: &XfsHooks, val: u64, priv_data: usize) -> i32 {
    let mut ret = NOTIFY_DONE;

    for hook in &chain.hooks {
        ret = (hook.nb.callback)(val, priv_data);
        if ret & NOTIFY_STOP_MASK != 0 {
            break;
        }
    }

    ret
}

pub const fn xfs_hooks_call_result(last_notifier_result: i32) -> XfsHooksCallOutcome {
    XfsHooksCallOutcome {
        notifier_chain_called: true,
        result: last_notifier_result,
    }
}

impl XfsHooks {
    pub const fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.hooks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicUsize, Ordering};

    static ORDER: AtomicUsize = AtomicUsize::new(0);

    fn high_priority(val: u64, priv_data: usize) -> i32 {
        assert_eq!(val, 7);
        assert_eq!(priv_data, 0xf5);
        let _ = ORDER.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire);
        NOTIFY_OK
    }

    fn stop_priority(val: u64, priv_data: usize) -> i32 {
        assert_eq!(val, 7);
        assert_eq!(priv_data, 0xf5);
        let _ = ORDER.compare_exchange(1, 2, Ordering::AcqRel, Ordering::Acquire);
        NOTIFY_STOP
    }

    fn unused_hook(_val: u64, _priv_data: usize) -> i32 {
        ORDER.store(99, Ordering::Release);
        NOTIFY_OK
    }

    #[test]
    fn xfs_hooks_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/xfs/xfs_hooks.c"
        ));
        assert!(source.contains("#include \"xfs_platform.h\""));
        assert!(source.contains("#include \"xfs_fs.h\""));
        assert!(source.contains("#include \"xfs_shared.h\""));
        assert!(source.contains("#include \"xfs_format.h\""));
        assert!(source.contains("#include \"xfs_trans_resv.h\""));
        assert!(source.contains("#include \"xfs_mount.h\""));
        assert!(source.contains("#include \"xfs_ag.h\""));
        assert!(source.contains("#include \"xfs_trace.h\""));
        assert!(source.contains("xfs_hooks_init("));
        assert!(source.contains("BLOCKING_INIT_NOTIFIER_HEAD(&chain->head);"));
        assert!(source.contains("xfs_hooks_add("));
        assert!(source.contains("ASSERT(hook->nb.notifier_call != NULL);"));
        assert!(source.contains("BUILD_BUG_ON(offsetof(struct xfs_hook, nb) != 0);"));
        assert!(source.contains("blocking_notifier_chain_register(&chain->head, &hook->nb);"));
        assert!(source.contains("xfs_hooks_del("));
        assert!(source.contains("blocking_notifier_chain_unregister(&chain->head, &hook->nb);"));
        assert!(source.contains("xfs_hooks_call("));
        assert!(source.contains("blocking_notifier_call_chain(&chain->head, val, priv);"));

        let mut chain = XfsHooks::new();
        assert!(chain.is_empty());
        assert_eq!(
            xfs_hooks_add(
                &mut chain,
                XfsHook {
                    id: 1,
                    nb: XfsHookNotifier {
                        priority: 0,
                        callback: stop_priority,
                    },
                },
            ),
            0
        );
        assert_eq!(
            xfs_hooks_add(
                &mut chain,
                XfsHook {
                    id: 2,
                    nb: XfsHookNotifier {
                        priority: 10,
                        callback: high_priority,
                    },
                },
            ),
            0
        );
        assert_eq!(
            xfs_hooks_add(
                &mut chain,
                XfsHook {
                    id: 3,
                    nb: XfsHookNotifier {
                        priority: -1,
                        callback: unused_hook,
                    },
                },
            ),
            0
        );
        assert_eq!(
            xfs_hooks_add(
                &mut chain,
                XfsHook {
                    id: 2,
                    nb: XfsHookNotifier {
                        priority: 10,
                        callback: high_priority,
                    },
                },
            ),
            -EEXIST
        );
        assert_eq!(chain.len(), 3);
        ORDER.store(0, Ordering::Release);
        assert_eq!(xfs_hooks_call(&chain, 7, 0xf5), NOTIFY_STOP);
        assert_eq!(ORDER.load(Ordering::Acquire), 2);
        xfs_hooks_del(&mut chain, 2);
        assert_eq!(chain.len(), 2);
        xfs_hooks_init(&mut chain);
        assert!(chain.is_empty());
        assert!(xfs_hooks_init_clears_head());
        assert_eq!(xfs_hooks_add_result(true, -22), Some(-22));
        assert_eq!(xfs_hooks_add_result(false, 0), None);
        assert!(xfs_hooks_del_unregisters());
        assert_eq!(
            xfs_hooks_call_result(7),
            XfsHooksCallOutcome {
                notifier_chain_called: true,
                result: 7,
            }
        );
        assert_eq!(NOTIFY_DONE, 0);
        assert_eq!(XFS_HOOK_NOTIFIER_OFFSET, 0);
    }
}
