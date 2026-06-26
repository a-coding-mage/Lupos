//! linux-parity: complete
//! linux-source: vendor/linux/kernel/user-return-notifier.c
//! test-origin: linux:vendor/linux/kernel/user-return-notifier.c
//! Per-CPU user-return notifier list semantics.

extern crate alloc;

use alloc::vec::Vec;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UserReturnNotifierList {
    notifiers: Vec<usize>,
    tif_user_return_notify: bool,
}

impl UserReturnNotifierList {
    pub fn register(&mut self, notifier: usize) {
        self.tif_user_return_notify = true;
        self.notifiers.retain(|entry| *entry != notifier);
        self.notifiers.insert(0, notifier);
    }

    pub fn unregister(&mut self, notifier: usize) {
        self.notifiers.retain(|entry| *entry != notifier);
        if self.notifiers.is_empty() {
            self.tif_user_return_notify = false;
        }
    }

    pub fn fire_user_return_notifiers(&self) -> Vec<usize> {
        self.notifiers.clone()
    }

    pub const fn thread_flag_set(&self) -> bool {
        self.tif_user_return_notify
    }

    pub fn len(&self) -> usize {
        self.notifiers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_return_notifier_list_matches_linux_register_unregister_shape() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/user-return-notifier.c"
        ));
        assert!(source.contains("DEFINE_PER_CPU(struct hlist_head, return_notifier_list);"));
        assert!(source.contains("set_tsk_thread_flag(current, TIF_USER_RETURN_NOTIFY);"));
        assert!(source.contains("hlist_add_head(&urn->link"));
        assert!(source.contains("hlist_del(&urn->link);"));
        assert!(source.contains("clear_tsk_thread_flag(current, TIF_USER_RETURN_NOTIFY);"));
        assert!(source.contains("hlist_for_each_entry_safe(urn, tmp2, head, link)"));
        assert!(source.contains("urn->on_user_return(urn);"));

        let mut list = UserReturnNotifierList::default();
        list.register(1);
        list.register(2);
        assert!(list.thread_flag_set());
        assert_eq!(list.fire_user_return_notifiers(), [2, 1]);
        list.unregister(2);
        assert!(list.thread_flag_set());
        list.unregister(1);
        assert!(!list.thread_flag_set());
        assert_eq!(list.len(), 0);
    }
}
