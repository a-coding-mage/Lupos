//! linux-parity: complete
//! linux-source: vendor/linux/lib/timerqueue.c
//! test-origin: linux:vendor/linux/lib/timerqueue.c
//! Timerqueue ordering helpers.

extern crate alloc;

use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimerqueueNode {
    pub id: usize,
    pub expires: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TimerqueueHead {
    nodes: Vec<TimerqueueNode>,
}

pub type TimerqueueLinkedHead = TimerqueueHead;

impl TimerqueueHead {
    pub const fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn nodes(&self) -> &[TimerqueueNode] {
        &self.nodes
    }
}

pub const fn timerqueue_less(a: TimerqueueNode, b: TimerqueueNode) -> bool {
    a.expires < b.expires
}

pub fn timerqueue_add(head: &mut TimerqueueHead, node: TimerqueueNode) -> bool {
    let pos = head
        .nodes
        .iter()
        .position(|existing| timerqueue_less(node, *existing))
        .unwrap_or(head.nodes.len());
    head.nodes.insert(pos, node);
    pos == 0
}

pub fn timerqueue_del(head: &mut TimerqueueHead, node_id: usize) -> bool {
    if let Some(pos) = head.nodes.iter().position(|node| node.id == node_id) {
        head.nodes.remove(pos);
    }
    !head.nodes.is_empty()
}

pub fn timerqueue_iterate_next(
    head: &TimerqueueHead,
    node_id: Option<usize>,
) -> Option<TimerqueueNode> {
    let node_id = node_id?;
    let pos = head.nodes.iter().position(|node| node.id == node_id)?;
    head.nodes.get(pos + 1).copied()
}

pub fn timerqueue_linked_add(head: &mut TimerqueueLinkedHead, node: TimerqueueNode) -> bool {
    timerqueue_add(head, node)
}

pub fn timerqueue_add_returns_first(
    current_first: Option<TimerqueueNode>,
    node: TimerqueueNode,
) -> bool {
    match current_first {
        Some(first) => timerqueue_less(node, first),
        None => true,
    }
}

pub const fn timerqueue_del_returns_nonempty(remaining_nodes: usize) -> bool {
    remaining_nodes != 0
}

pub fn timerqueue_linked_add_returns_first(
    current_first: Option<TimerqueueNode>,
    node: TimerqueueNode,
) -> bool {
    timerqueue_add_returns_first(current_first, node)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timerqueue_matches_linux_rbtree_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/timerqueue.c"
        ));
        assert!(source.contains("#define __node_2_tq(_n)"));
        assert!(source.contains("return __node_2_tq(a)->expires < __node_2_tq(b)->expires;"));
        assert!(source.contains("WARN_ON_ONCE(!RB_EMPTY_NODE(&node->node));"));
        assert!(
            source
                .contains("return rb_add_cached(&node->node, &head->rb_root, __timerqueue_less);")
        );
        assert!(source.contains("rb_erase_cached(&node->node, &head->rb_root);"));
        assert!(source.contains("RB_CLEAR_NODE(&node->node);"));
        assert!(source.contains("return !RB_EMPTY_ROOT(&head->rb_root.rb_root);"));
        assert!(source.contains("if (!node)"));
        assert!(source.contains("next = rb_next(&node->node);"));
        assert!(source.contains("return container_of(next, struct timerqueue_node, node);"));
        assert!(source.contains("#define __node_2_tq_linked(_n)"));
        assert!(
            source.contains("return rb_add_linked(&node->node, &head->rb_root, __tq_linked_less);")
        );
        assert!(source.contains("EXPORT_SYMBOL_GPL(timerqueue_linked_add);"));

        let first = TimerqueueNode { id: 1, expires: 10 };
        let same = TimerqueueNode { id: 2, expires: 10 };
        let earlier = TimerqueueNode { id: 3, expires: 5 };
        let later = TimerqueueNode { id: 4, expires: 20 };

        assert!(timerqueue_less(earlier, first));
        assert!(!timerqueue_less(first, same));
        assert!(timerqueue_add_returns_first(Some(first), earlier));
        assert!(!timerqueue_add_returns_first(Some(earlier), first));
        assert!(timerqueue_del_returns_nonempty(1));
        assert!(!timerqueue_del_returns_nonempty(0));

        let mut head = TimerqueueHead::new();
        assert!(timerqueue_add(&mut head, first));
        assert!(!timerqueue_add(&mut head, same));
        assert!(timerqueue_add(&mut head, earlier));
        assert!(!timerqueue_add(&mut head, later));
        assert_eq!(head.nodes(), &[earlier, first, same, later]);
        assert_eq!(timerqueue_iterate_next(&head, None), None);
        assert_eq!(timerqueue_iterate_next(&head, Some(first.id)), Some(same));
        assert_eq!(timerqueue_iterate_next(&head, Some(later.id)), None);

        assert!(timerqueue_del(&mut head, earlier.id));
        assert_eq!(head.nodes(), &[first, same, later]);
        assert!(timerqueue_del(&mut head, first.id));
        assert!(timerqueue_del(&mut head, same.id));
        assert!(!timerqueue_del(&mut head, later.id));

        let mut linked = TimerqueueLinkedHead::new();
        assert!(timerqueue_linked_add(&mut linked, later));
        assert!(timerqueue_linked_add_returns_first(Some(later), earlier));
        assert!(timerqueue_linked_add(&mut linked, earlier));
        assert_eq!(linked.nodes(), &[earlier, later]);
    }
}
