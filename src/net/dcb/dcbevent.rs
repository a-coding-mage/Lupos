//! linux-parity: complete
//! linux-source: vendor/linux/net/dcb/dcbevent.c
//! test-origin: linux:vendor/linux/net/dcb/dcbevent.c
//! DCB atomic notifier chain.

extern crate alloc;

use alloc::vec::Vec;
use spin::Mutex;

use crate::include::uapi::errno::{EEXIST, ENOENT};

pub const NOTIFY_DONE: i32 = 0x0000;
pub const NOTIFY_OK: i32 = 0x0001;
pub const NOTIFY_STOP_MASK: i32 = 0x8000;
pub const NOTIFY_STOP: i32 = NOTIFY_OK | NOTIFY_STOP_MASK;
pub type DcbEventCallback = fn(val: u64, data: usize) -> i32;

#[derive(Clone, Copy)]
pub struct DcbEventNotifier {
    pub id: usize,
    pub priority: i32,
    pub callback: DcbEventCallback,
}

static DCB_EVENT_NOTIFIERS: Mutex<Vec<DcbEventNotifier>> = Mutex::new(Vec::new());

pub fn register_dcbevent_notifier(nb: DcbEventNotifier) -> i32 {
    let mut chain = DCB_EVENT_NOTIFIERS.lock();
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

pub fn unregister_dcbevent_notifier(id: usize) -> i32 {
    let mut chain = DCB_EVENT_NOTIFIERS.lock();
    match chain.iter().position(|registered| registered.id == id) {
        Some(index) => {
            chain.remove(index);
            0
        }
        None => -ENOENT,
    }
}

pub fn call_dcbevent_notifiers(val: u64, data: usize) -> i32 {
    let notifiers = DCB_EVENT_NOTIFIERS.lock().clone();
    let mut ret = NOTIFY_DONE;

    for notifier in notifiers {
        ret = (notifier.callback)(val, data);
        if ret & NOTIFY_STOP_MASK != 0 {
            break;
        }
    }

    ret
}

#[cfg(test)]
fn reset_dcbevent_notifiers() {
    DCB_EVENT_NOTIFIERS.lock().clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicUsize, Ordering};

    static TEST_LOCK: Mutex<()> = Mutex::new(());
    static ORDER: AtomicUsize = AtomicUsize::new(0);

    fn high_priority(val: u64, data: usize) -> i32 {
        assert_eq!(val, 7);
        assert_eq!(data, 0xdcb);
        let _ = ORDER.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire);
        NOTIFY_OK
    }

    fn low_priority(val: u64, data: usize) -> i32 {
        assert_eq!(val, 7);
        assert_eq!(data, 0xdcb);
        let _ = ORDER.compare_exchange(1, 12, Ordering::AcqRel, Ordering::Acquire);
        NOTIFY_STOP
    }

    fn unused_notifier(_val: u64, _data: usize) -> i32 {
        ORDER.store(99, Ordering::Release);
        NOTIFY_OK
    }

    #[test]
    fn dcbevent_notifier_chain_matches_linux_source() {
        let _guard = TEST_LOCK.lock();
        reset_dcbevent_notifiers();
        ORDER.store(0, Ordering::Release);
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dcb/dcbevent.c"
        ));
        assert!(source.contains("static ATOMIC_NOTIFIER_HEAD(dcbevent_notif_chain);"));
        assert!(source.contains("register_dcbevent_notifier(struct notifier_block *nb)"));
        assert!(source.contains("atomic_notifier_chain_register(&dcbevent_notif_chain, nb)"));
        assert!(source.contains("unregister_dcbevent_notifier(struct notifier_block *nb)"));
        assert!(source.contains("atomic_notifier_chain_unregister(&dcbevent_notif_chain, nb)"));
        assert!(source.contains("call_dcbevent_notifiers(unsigned long val, void *v)"));
        assert!(source.contains("atomic_notifier_call_chain(&dcbevent_notif_chain, val, v)"));

        assert_eq!(
            register_dcbevent_notifier(DcbEventNotifier {
                id: 1,
                priority: 0,
                callback: low_priority,
            }),
            0
        );
        assert_eq!(
            register_dcbevent_notifier(DcbEventNotifier {
                id: 2,
                priority: 10,
                callback: high_priority,
            }),
            0
        );
        assert_eq!(
            register_dcbevent_notifier(DcbEventNotifier {
                id: 3,
                priority: -1,
                callback: unused_notifier,
            }),
            0
        );

        assert_eq!(call_dcbevent_notifiers(7, 0xdcb), NOTIFY_STOP);
        assert_eq!(ORDER.load(Ordering::Acquire), 12);
        reset_dcbevent_notifiers();
    }

    #[test]
    fn dcbevent_register_unregister_error_paths_match_notifier_core() {
        let _guard = TEST_LOCK.lock();
        reset_dcbevent_notifiers();
        let notifier = DcbEventNotifier {
            id: 1,
            priority: 0,
            callback: unused_notifier,
        };

        assert_eq!(register_dcbevent_notifier(notifier), 0);
        assert_eq!(register_dcbevent_notifier(notifier), -EEXIST);
        assert_eq!(unregister_dcbevent_notifier(1), 0);
        assert_eq!(unregister_dcbevent_notifier(1), -ENOENT);
        assert_eq!(call_dcbevent_notifiers(1, 2), NOTIFY_DONE);
        reset_dcbevent_notifiers();
    }
}
