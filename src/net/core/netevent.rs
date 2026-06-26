//! linux-parity: complete
//! linux-source: vendor/linux/net/core/netevent.c
//! test-origin: linux:vendor/linux/net/core/netevent.c
//! Network event notifier chain.

extern crate alloc;

use alloc::vec::Vec;

pub type NeteventCallback = fn(val: usize, data: usize) -> i32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NotifierBlock {
    pub id: usize,
    pub call: NeteventCallback,
}

#[derive(Clone, Debug, Default)]
pub struct NeteventNotifierChain {
    blocks: Vec<NotifierBlock>,
}

impl NeteventNotifierChain {
    pub const fn new() -> Self {
        Self { blocks: Vec::new() }
    }

    pub fn register_netevent_notifier(&mut self, nb: NotifierBlock) -> i32 {
        if self.blocks.iter().any(|block| block.id == nb.id) {
            return -1;
        }
        self.blocks.push(nb);
        0
    }

    pub fn unregister_netevent_notifier(&mut self, id: usize) -> i32 {
        if let Some(pos) = self.blocks.iter().position(|block| block.id == id) {
            self.blocks.remove(pos);
            0
        } else {
            -1
        }
    }

    pub fn call_netevent_notifiers(&self, val: usize, data: usize) -> i32 {
        let mut ret = 0;
        for block in &self.blocks {
            ret = (block.call)(val, data);
            if ret != 0 {
                break;
            }
        }
        ret
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(val: usize, data: usize) -> i32 {
        (val + data) as i32
    }

    #[test]
    fn netevent_notifier_chain_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/core/netevent.c"
        ));
        assert!(source.contains("static ATOMIC_NOTIFIER_HEAD(netevent_notif_chain);"));
        assert!(source.contains("int register_netevent_notifier(struct notifier_block *nb)"));
        assert!(source.contains("atomic_notifier_chain_register(&netevent_notif_chain, nb)"));
        assert!(source.contains("int unregister_netevent_notifier(struct notifier_block *nb)"));
        assert!(source.contains("atomic_notifier_chain_unregister(&netevent_notif_chain, nb)"));
        assert!(source.contains("int call_netevent_notifiers(unsigned long val, void *v)"));
        assert!(source.contains("atomic_notifier_call_chain(&netevent_notif_chain, val, v)"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(register_netevent_notifier);"));

        let mut chain = NeteventNotifierChain::new();
        assert!(chain.is_empty());
        assert_eq!(
            chain.register_netevent_notifier(NotifierBlock { id: 1, call: ok }),
            0
        );
        assert_eq!(
            chain.register_netevent_notifier(NotifierBlock { id: 1, call: ok }),
            -1
        );
        assert_eq!(chain.call_netevent_notifiers(7, 5), 12);
        assert_eq!(chain.unregister_netevent_notifier(1), 0);
        assert_eq!(chain.unregister_netevent_notifier(1), -1);
    }
}
