//! linux-parity: complete
//! linux-source: vendor/linux/security/lsm_notifier.c
//! test-origin: linux:vendor/linux/security/lsm_notifier.c
//! LSM blocking notifier chain facade.

extern crate alloc;

use alloc::vec::Vec;
use spin::Mutex;

use crate::include::uapi::errno::{EEXIST, ENOENT};

pub const LINUX_NOTIFIER_INCLUDE: &str = "#include <linux/notifier.h>";
pub const LINUX_SECURITY_INCLUDE: &str = "#include <linux/security.h>";
pub const LSM_NOTIFIER_CHAIN: &str = "blocking_lsm_notifier_chain";
pub const LSM_POLICY_CHANGE: usize = 0;
pub const NOTIFY_DONE: i32 = 0x0000;
pub const NOTIFY_OK: i32 = 0x0001;
pub const NOTIFY_STOP_MASK: i32 = 0x8000;
pub const NOTIFY_STOP: i32 = NOTIFY_OK | NOTIFY_STOP_MASK;

pub type LsmNotifierCallback = fn(event: usize, data: usize) -> i32;

#[derive(Clone, Copy)]
pub struct LsmNotifierBlock {
    pub id: usize,
    pub priority: i32,
    pub callback: LsmNotifierCallback,
}

static LSM_NOTIFIERS: Mutex<Vec<LsmNotifierBlock>> = Mutex::new(Vec::new());

pub fn register_blocking_lsm_notifier(nb: LsmNotifierBlock) -> i32 {
    let mut chain = LSM_NOTIFIERS.lock();
    if chain.iter().any(|registered| registered.id == nb.id) {
        return -EEXIST;
    }

    let insert_at = chain
        .iter()
        .position(|registered| nb.priority > registered.priority)
        .unwrap_or(chain.len());
    chain.insert(insert_at, nb);
    0
}

pub fn unregister_blocking_lsm_notifier(id: usize) -> i32 {
    let mut chain = LSM_NOTIFIERS.lock();
    match chain.iter().position(|registered| registered.id == id) {
        Some(index) => {
            chain.remove(index);
            0
        }
        None => -ENOENT,
    }
}

pub fn call_blocking_lsm_notifier(event: usize, data: usize) -> i32 {
    let notifiers = LSM_NOTIFIERS.lock().clone();
    let mut ret = NOTIFY_DONE;

    for notifier in notifiers {
        ret = (notifier.callback)(event, data);
        if ret & NOTIFY_STOP_MASK != 0 {
            break;
        }
    }

    ret
}

pub fn registered_lsm_notifiers() -> usize {
    LSM_NOTIFIERS.lock().len()
}

#[cfg(test)]
fn reset_blocking_lsm_notifiers() {
    LSM_NOTIFIERS.lock().clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicUsize, Ordering};
    use spin::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());
    static ORDER: AtomicUsize = AtomicUsize::new(0);

    fn high_priority(event: usize, data: usize) -> i32 {
        assert_eq!(event, LSM_POLICY_CHANGE);
        assert_eq!(data, 0x51);
        let _ = ORDER.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire);
        NOTIFY_OK
    }

    fn stop_priority(event: usize, data: usize) -> i32 {
        assert_eq!(event, LSM_POLICY_CHANGE);
        assert_eq!(data, 0x51);
        let _ = ORDER.compare_exchange(1, 2, Ordering::AcqRel, Ordering::Acquire);
        NOTIFY_STOP
    }

    fn unused_notifier(_event: usize, _data: usize) -> i32 {
        ORDER.store(99, Ordering::Release);
        NOTIFY_OK
    }

    #[test]
    fn lsm_notifier_source_matches_linux() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        reset_blocking_lsm_notifiers();
        ORDER.store(0, Ordering::Release);
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/lsm_notifier.c"
        ));
        assert!(source.contains(LINUX_NOTIFIER_INCLUDE));
        assert!(source.contains(LINUX_SECURITY_INCLUDE));
        assert!(source.contains("static BLOCKING_NOTIFIER_HEAD(blocking_lsm_notifier_chain);"));
        assert!(source.contains("call_blocking_lsm_notifier(enum lsm_event event, void *data)"));
        assert!(source.contains("blocking_notifier_call_chain(&blocking_lsm_notifier_chain"));
        assert!(source.contains("register_blocking_lsm_notifier(struct notifier_block *nb)"));
        assert!(source.contains("blocking_notifier_chain_register(&blocking_lsm_notifier_chain"));
        assert!(source.contains("unregister_blocking_lsm_notifier(struct notifier_block *nb)"));
        assert!(source.contains("blocking_notifier_chain_unregister(&blocking_lsm_notifier_chain"));
        assert!(source.contains("EXPORT_SYMBOL(call_blocking_lsm_notifier);"));
        assert!(source.contains("EXPORT_SYMBOL(register_blocking_lsm_notifier);"));
        assert!(source.contains("EXPORT_SYMBOL(unregister_blocking_lsm_notifier);"));

        assert_eq!(
            register_blocking_lsm_notifier(LsmNotifierBlock {
                id: 1,
                priority: 0,
                callback: stop_priority,
            }),
            0
        );
        assert_eq!(
            register_blocking_lsm_notifier(LsmNotifierBlock {
                id: 2,
                priority: 10,
                callback: high_priority,
            }),
            0
        );
        assert_eq!(
            register_blocking_lsm_notifier(LsmNotifierBlock {
                id: 3,
                priority: -1,
                callback: unused_notifier,
            }),
            0
        );
        assert_eq!(
            register_blocking_lsm_notifier(LsmNotifierBlock {
                id: 2,
                priority: 10,
                callback: high_priority,
            }),
            -EEXIST
        );
        assert_eq!(registered_lsm_notifiers(), 3);
        assert_eq!(
            call_blocking_lsm_notifier(LSM_POLICY_CHANGE, 0x51),
            NOTIFY_STOP
        );
        assert_eq!(ORDER.load(Ordering::Acquire), 2);
        assert_eq!(unregister_blocking_lsm_notifier(2), 0);
        assert_eq!(unregister_blocking_lsm_notifier(2), -ENOENT);
        reset_blocking_lsm_notifiers();
    }
}
