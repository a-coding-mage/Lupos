//! linux-parity: complete
//! linux-source: vendor/linux/lib/rbtree.c
//! test-origin: linux:vendor/linux/lib/rbtree.c
//! Linux rbtree core ABI exports used by vendor-built modules.

use core::ptr;

use crate::kernel::module::{export_symbol, find_symbol};

const RB_RED: usize = 0;
const RB_BLACK: usize = 1;
const RB_PARENT_MASK: usize = !3usize;

type AugmentRotate = Option<unsafe extern "C" fn(old: *mut LinuxRbNode, new: *mut LinuxRbNode)>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxRbNode {
    pub __rb_parent_color: usize,
    pub rb_right: *mut LinuxRbNode,
    pub rb_left: *mut LinuxRbNode,
}

#[repr(C)]
pub struct LinuxRbRoot {
    pub rb_node: *mut LinuxRbNode,
}

#[repr(C)]
pub struct LinuxRbNodeLinked {
    pub node: LinuxRbNode,
    pub prev: *mut LinuxRbNodeLinked,
    pub next: *mut LinuxRbNodeLinked,
}

#[repr(C)]
pub struct LinuxRbRootLinked {
    pub rb_root: LinuxRbRoot,
    pub rb_leftmost: *mut LinuxRbNodeLinked,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("__rb_erase_color", linux___rb_erase_color as usize, false);
    export_symbol_once(
        "__rb_insert_augmented",
        linux___rb_insert_augmented as usize,
        false,
    );
    export_symbol_once("rb_erase", linux_rb_erase as usize, false);
    export_symbol_once("rb_erase_linked", linux_rb_erase_linked as usize, true);
    export_symbol_once(
        "rb_first_postorder",
        linux_rb_first_postorder as usize,
        false,
    );
    export_symbol_once("rb_insert_color", linux_rb_insert_color as usize, false);
    export_symbol_once("rb_next", linux_rb_next as usize, false);
    export_symbol_once("rb_next_postorder", linux_rb_next_postorder as usize, false);
    export_symbol_once("rb_prev", linux_rb_prev as usize, false);
    export_symbol_once("rb_replace_node", linux_rb_replace_node as usize, false);
    export_symbol_once(
        "rb_replace_node_rcu",
        linux_rb_replace_node_rcu as usize,
        false,
    );
}

#[inline]
unsafe fn rb_parent(node: *const LinuxRbNode) -> *mut LinuxRbNode {
    unsafe { ((*node).__rb_parent_color & RB_PARENT_MASK) as *mut LinuxRbNode }
}

#[inline]
fn rb_parent_from_color(parent_color: usize) -> *mut LinuxRbNode {
    (parent_color & RB_PARENT_MASK) as *mut LinuxRbNode
}

#[inline]
fn rb_is_red_from_color(parent_color: usize) -> bool {
    (parent_color & RB_BLACK) == RB_RED
}

#[inline]
fn rb_is_black_from_color(parent_color: usize) -> bool {
    (parent_color & RB_BLACK) == RB_BLACK
}

#[inline]
unsafe fn rb_is_red(node: *const LinuxRbNode) -> bool {
    unsafe { rb_is_red_from_color((*node).__rb_parent_color) }
}

#[inline]
unsafe fn rb_is_black(node: *const LinuxRbNode) -> bool {
    unsafe { rb_is_black_from_color((*node).__rb_parent_color) }
}

#[inline]
unsafe fn rb_set_parent(node: *mut LinuxRbNode, parent: *mut LinuxRbNode) {
    let color = unsafe { (*node).__rb_parent_color & RB_BLACK };
    unsafe {
        (*node).__rb_parent_color = color + parent as usize;
    }
}

#[inline]
unsafe fn rb_set_parent_color(node: *mut LinuxRbNode, parent: *mut LinuxRbNode, color: usize) {
    unsafe {
        (*node).__rb_parent_color = parent as usize + color;
    }
}

#[inline]
unsafe fn rb_set_black(node: *mut LinuxRbNode) {
    unsafe {
        (*node).__rb_parent_color += RB_BLACK;
    }
}

#[inline]
unsafe fn rb_red_parent(red: *const LinuxRbNode) -> *mut LinuxRbNode {
    unsafe { (*red).__rb_parent_color as *mut LinuxRbNode }
}

#[inline]
unsafe fn write_child(slot: *mut *mut LinuxRbNode, value: *mut LinuxRbNode) {
    unsafe { ptr::write_volatile(slot, value) };
}

#[inline]
unsafe fn rb_change_child(
    old: *mut LinuxRbNode,
    new: *mut LinuxRbNode,
    parent: *mut LinuxRbNode,
    root: *mut LinuxRbRoot,
) {
    unsafe {
        if !parent.is_null() {
            if (*parent).rb_left == old {
                write_child(ptr::addr_of_mut!((*parent).rb_left), new);
            } else {
                write_child(ptr::addr_of_mut!((*parent).rb_right), new);
            }
        } else {
            write_child(ptr::addr_of_mut!((*root).rb_node), new);
        }
    }
}

#[inline]
unsafe fn rb_rotate_set_parents(
    old: *mut LinuxRbNode,
    new: *mut LinuxRbNode,
    root: *mut LinuxRbRoot,
    color: usize,
) {
    let parent = unsafe { rb_parent(old) };
    unsafe {
        (*new).__rb_parent_color = (*old).__rb_parent_color;
        rb_set_parent_color(old, new, color);
        rb_change_child(old, new, parent, root);
    }
}

#[inline]
unsafe fn call_rotate(augment_rotate: AugmentRotate, old: *mut LinuxRbNode, new: *mut LinuxRbNode) {
    if let Some(rotate) = augment_rotate {
        unsafe { rotate(old, new) };
    }
}

unsafe fn rb_insert(
    mut node: *mut LinuxRbNode,
    root: *mut LinuxRbRoot,
    augment_rotate: AugmentRotate,
) {
    let mut parent = unsafe { rb_red_parent(node) };

    loop {
        if parent.is_null() {
            unsafe { rb_set_parent_color(node, ptr::null_mut(), RB_BLACK) };
            break;
        }

        if unsafe { rb_is_black(parent) } {
            break;
        }

        let gparent = unsafe { rb_red_parent(parent) };
        let mut tmp = unsafe { (*gparent).rb_right };
        if parent != tmp {
            if !tmp.is_null() && unsafe { rb_is_red(tmp) } {
                unsafe {
                    rb_set_parent_color(tmp, gparent, RB_BLACK);
                    rb_set_parent_color(parent, gparent, RB_BLACK);
                }
                node = gparent;
                parent = unsafe { rb_parent(node) };
                unsafe { rb_set_parent_color(node, parent, RB_RED) };
                continue;
            }

            tmp = unsafe { (*parent).rb_right };
            if node == tmp {
                tmp = unsafe { (*node).rb_left };
                unsafe {
                    write_child(ptr::addr_of_mut!((*parent).rb_right), tmp);
                    write_child(ptr::addr_of_mut!((*node).rb_left), parent);
                    if !tmp.is_null() {
                        rb_set_parent_color(tmp, parent, RB_BLACK);
                    }
                    rb_set_parent_color(parent, node, RB_RED);
                    call_rotate(augment_rotate, parent, node);
                }
                parent = node;
                tmp = unsafe { (*node).rb_right };
            }

            unsafe {
                write_child(ptr::addr_of_mut!((*gparent).rb_left), tmp);
                write_child(ptr::addr_of_mut!((*parent).rb_right), gparent);
                if !tmp.is_null() {
                    rb_set_parent_color(tmp, gparent, RB_BLACK);
                }
                rb_rotate_set_parents(gparent, parent, root, RB_RED);
                call_rotate(augment_rotate, gparent, parent);
            }
            break;
        } else {
            tmp = unsafe { (*gparent).rb_left };
            if !tmp.is_null() && unsafe { rb_is_red(tmp) } {
                unsafe {
                    rb_set_parent_color(tmp, gparent, RB_BLACK);
                    rb_set_parent_color(parent, gparent, RB_BLACK);
                }
                node = gparent;
                parent = unsafe { rb_parent(node) };
                unsafe { rb_set_parent_color(node, parent, RB_RED) };
                continue;
            }

            tmp = unsafe { (*parent).rb_left };
            if node == tmp {
                tmp = unsafe { (*node).rb_right };
                unsafe {
                    write_child(ptr::addr_of_mut!((*parent).rb_left), tmp);
                    write_child(ptr::addr_of_mut!((*node).rb_right), parent);
                    if !tmp.is_null() {
                        rb_set_parent_color(tmp, parent, RB_BLACK);
                    }
                    rb_set_parent_color(parent, node, RB_RED);
                    call_rotate(augment_rotate, parent, node);
                }
                parent = node;
                tmp = unsafe { (*node).rb_left };
            }

            unsafe {
                write_child(ptr::addr_of_mut!((*gparent).rb_right), tmp);
                write_child(ptr::addr_of_mut!((*parent).rb_left), gparent);
                if !tmp.is_null() {
                    rb_set_parent_color(tmp, gparent, RB_BLACK);
                }
                rb_rotate_set_parents(gparent, parent, root, RB_RED);
                call_rotate(augment_rotate, gparent, parent);
            }
            break;
        }
    }
}

unsafe fn rb_erase_color(
    mut parent: *mut LinuxRbNode,
    root: *mut LinuxRbRoot,
    augment_rotate: AugmentRotate,
) {
    if parent.is_null() {
        return;
    }

    let mut node = ptr::null_mut();
    loop {
        let mut sibling = unsafe { (*parent).rb_right };
        if node != sibling {
            if unsafe { rb_is_red(sibling) } {
                let tmp1 = unsafe { (*sibling).rb_left };
                unsafe {
                    write_child(ptr::addr_of_mut!((*parent).rb_right), tmp1);
                    write_child(ptr::addr_of_mut!((*sibling).rb_left), parent);
                    rb_set_parent_color(tmp1, parent, RB_BLACK);
                    rb_rotate_set_parents(parent, sibling, root, RB_RED);
                    call_rotate(augment_rotate, parent, sibling);
                }
                sibling = tmp1;
            }

            let mut tmp1 = unsafe { (*sibling).rb_right };
            if tmp1.is_null() || unsafe { rb_is_black(tmp1) } {
                let tmp2 = unsafe { (*sibling).rb_left };
                if tmp2.is_null() || unsafe { rb_is_black(tmp2) } {
                    unsafe { rb_set_parent_color(sibling, parent, RB_RED) };
                    if unsafe { rb_is_red(parent) } {
                        unsafe { rb_set_black(parent) };
                    } else {
                        node = parent;
                        parent = unsafe { rb_parent(node) };
                        if !parent.is_null() {
                            continue;
                        }
                    }
                    break;
                }

                tmp1 = unsafe { (*tmp2).rb_right };
                unsafe {
                    write_child(ptr::addr_of_mut!((*sibling).rb_left), tmp1);
                    write_child(ptr::addr_of_mut!((*tmp2).rb_right), sibling);
                    write_child(ptr::addr_of_mut!((*parent).rb_right), tmp2);
                    if !tmp1.is_null() {
                        rb_set_parent_color(tmp1, sibling, RB_BLACK);
                    }
                    call_rotate(augment_rotate, sibling, tmp2);
                }
                tmp1 = sibling;
                sibling = tmp2;
            }

            let tmp2 = unsafe { (*sibling).rb_left };
            unsafe {
                write_child(ptr::addr_of_mut!((*parent).rb_right), tmp2);
                write_child(ptr::addr_of_mut!((*sibling).rb_left), parent);
                rb_set_parent_color(tmp1, sibling, RB_BLACK);
                if !tmp2.is_null() {
                    rb_set_parent(tmp2, parent);
                }
                rb_rotate_set_parents(parent, sibling, root, RB_BLACK);
                call_rotate(augment_rotate, parent, sibling);
            }
            break;
        } else {
            sibling = unsafe { (*parent).rb_left };
            if unsafe { rb_is_red(sibling) } {
                let tmp1 = unsafe { (*sibling).rb_right };
                unsafe {
                    write_child(ptr::addr_of_mut!((*parent).rb_left), tmp1);
                    write_child(ptr::addr_of_mut!((*sibling).rb_right), parent);
                    rb_set_parent_color(tmp1, parent, RB_BLACK);
                    rb_rotate_set_parents(parent, sibling, root, RB_RED);
                    call_rotate(augment_rotate, parent, sibling);
                }
                sibling = tmp1;
            }

            let mut tmp1 = unsafe { (*sibling).rb_left };
            if tmp1.is_null() || unsafe { rb_is_black(tmp1) } {
                let tmp2 = unsafe { (*sibling).rb_right };
                if tmp2.is_null() || unsafe { rb_is_black(tmp2) } {
                    unsafe { rb_set_parent_color(sibling, parent, RB_RED) };
                    if unsafe { rb_is_red(parent) } {
                        unsafe { rb_set_black(parent) };
                    } else {
                        node = parent;
                        parent = unsafe { rb_parent(node) };
                        if !parent.is_null() {
                            continue;
                        }
                    }
                    break;
                }

                tmp1 = unsafe { (*tmp2).rb_left };
                unsafe {
                    write_child(ptr::addr_of_mut!((*sibling).rb_right), tmp1);
                    write_child(ptr::addr_of_mut!((*tmp2).rb_left), sibling);
                    write_child(ptr::addr_of_mut!((*parent).rb_left), tmp2);
                    if !tmp1.is_null() {
                        rb_set_parent_color(tmp1, sibling, RB_BLACK);
                    }
                    call_rotate(augment_rotate, sibling, tmp2);
                }
                tmp1 = sibling;
                sibling = tmp2;
            }

            let tmp2 = unsafe { (*sibling).rb_right };
            unsafe {
                write_child(ptr::addr_of_mut!((*parent).rb_left), tmp2);
                write_child(ptr::addr_of_mut!((*sibling).rb_right), parent);
                rb_set_parent_color(tmp1, sibling, RB_BLACK);
                if !tmp2.is_null() {
                    rb_set_parent(tmp2, parent);
                }
                rb_rotate_set_parents(parent, sibling, root, RB_BLACK);
                call_rotate(augment_rotate, parent, sibling);
            }
            break;
        }
    }
}

unsafe fn rb_erase_augmented(node: *mut LinuxRbNode, root: *mut LinuxRbRoot) -> *mut LinuxRbNode {
    let child = unsafe { (*node).rb_right };
    let mut tmp = unsafe { (*node).rb_left };
    let parent;
    let rebalance;
    let pc;

    if tmp.is_null() {
        pc = unsafe { (*node).__rb_parent_color };
        parent = rb_parent_from_color(pc);
        unsafe { rb_change_child(node, child, parent, root) };
        if !child.is_null() {
            unsafe {
                (*child).__rb_parent_color = pc;
            }
            rebalance = ptr::null_mut();
        } else if rb_is_black_from_color(pc) {
            rebalance = parent;
        } else {
            rebalance = ptr::null_mut();
        }
    } else if child.is_null() {
        pc = unsafe { (*node).__rb_parent_color };
        unsafe {
            (*tmp).__rb_parent_color = pc;
        }
        parent = rb_parent_from_color(pc);
        unsafe { rb_change_child(node, tmp, parent, root) };
        rebalance = ptr::null_mut();
    } else {
        let mut successor = child;
        let child2;

        tmp = unsafe { (*child).rb_left };
        if tmp.is_null() {
            parent = successor;
            child2 = unsafe { (*successor).rb_right };
        } else {
            let mut parent_walk;
            loop {
                parent_walk = successor;
                successor = tmp;
                tmp = unsafe { (*tmp).rb_left };
                if tmp.is_null() {
                    break;
                }
            }
            parent = parent_walk;
            child2 = unsafe { (*successor).rb_right };
            unsafe {
                write_child(ptr::addr_of_mut!((*parent).rb_left), child2);
                write_child(ptr::addr_of_mut!((*successor).rb_right), child);
                rb_set_parent(child, successor);
            }
        }

        tmp = unsafe { (*node).rb_left };
        unsafe {
            write_child(ptr::addr_of_mut!((*successor).rb_left), tmp);
            rb_set_parent(tmp, successor);
        }

        pc = unsafe { (*node).__rb_parent_color };
        tmp = rb_parent_from_color(pc);
        unsafe { rb_change_child(node, successor, tmp, root) };

        if !child2.is_null() {
            unsafe { rb_set_parent_color(child2, parent, RB_BLACK) };
            rebalance = ptr::null_mut();
        } else if unsafe { rb_is_black(successor) } {
            rebalance = parent;
        } else {
            rebalance = ptr::null_mut();
        }
        unsafe {
            (*successor).__rb_parent_color = pc;
        }
    }

    rebalance
}

unsafe fn rb_left_deepest_node(mut node: *const LinuxRbNode) -> *mut LinuxRbNode {
    loop {
        if unsafe { !(*node).rb_left.is_null() } {
            node = unsafe { (*node).rb_left };
        } else if unsafe { !(*node).rb_right.is_null() } {
            node = unsafe { (*node).rb_right };
        } else {
            return node as *mut LinuxRbNode;
        }
    }
}

unsafe fn rb_clear_linked_node(node: *mut LinuxRbNodeLinked) {
    unsafe {
        (*node).node.__rb_parent_color = ptr::addr_of!((*node).node) as usize;
        (*node).prev = ptr::null_mut();
        (*node).next = ptr::null_mut();
    }
}

/// `__rb_erase_color` - `vendor/linux/lib/rbtree.c:410`.
pub unsafe extern "C" fn linux___rb_erase_color(
    parent: *mut LinuxRbNode,
    root: *mut LinuxRbRoot,
    augment_rotate: AugmentRotate,
) {
    unsafe { rb_erase_color(parent, root, augment_rotate) };
}

/// `__rb_insert_augmented` - `vendor/linux/lib/rbtree.c:473`.
pub unsafe extern "C" fn linux___rb_insert_augmented(
    node: *mut LinuxRbNode,
    root: *mut LinuxRbRoot,
    augment_rotate: AugmentRotate,
) {
    if node.is_null() || root.is_null() {
        return;
    }
    unsafe { rb_insert(node, root, augment_rotate) };
}

/// `rb_insert_color` - `vendor/linux/lib/rbtree.c:434`.
pub unsafe extern "C" fn linux_rb_insert_color(node: *mut LinuxRbNode, root: *mut LinuxRbRoot) {
    if node.is_null() || root.is_null() {
        return;
    }
    unsafe { rb_insert(node, root, None) };
}

/// `rb_erase` - `vendor/linux/lib/rbtree.c:440`.
pub unsafe extern "C" fn linux_rb_erase(node: *mut LinuxRbNode, root: *mut LinuxRbRoot) {
    if node.is_null() || root.is_null() {
        return;
    }
    let rebalance = unsafe { rb_erase_augmented(node, root) };
    if !rebalance.is_null() {
        unsafe { rb_erase_color(rebalance, root, None) };
    }
}

/// `rb_erase_linked` - `vendor/linux/lib/rbtree.c:449`.
pub unsafe extern "C" fn linux_rb_erase_linked(
    node: *mut LinuxRbNodeLinked,
    root: *mut LinuxRbRootLinked,
) -> bool {
    if node.is_null() || root.is_null() {
        return false;
    }

    unsafe {
        if !(*node).prev.is_null() {
            (*(*node).prev).next = (*node).next;
        } else {
            (*root).rb_leftmost = (*node).next;
        }

        if !(*node).next.is_null() {
            (*(*node).next).prev = (*node).prev;
        }

        linux_rb_erase(
            ptr::addr_of_mut!((*node).node),
            ptr::addr_of_mut!((*root).rb_root),
        );
        rb_clear_linked_node(node);
        !(*root).rb_leftmost.is_null()
    }
}

/// `rb_next` - `vendor/linux/lib/rbtree.c:480`.
pub unsafe extern "C" fn linux_rb_next(mut node: *const LinuxRbNode) -> *mut LinuxRbNode {
    if node.is_null() || unsafe { (*node).__rb_parent_color == node as usize } {
        return ptr::null_mut();
    }

    if unsafe { !(*node).rb_right.is_null() } {
        node = unsafe { (*node).rb_right };
        while unsafe { !(*node).rb_left.is_null() } {
            node = unsafe { (*node).rb_left };
        }
        return node as *mut LinuxRbNode;
    }

    let mut parent = unsafe { rb_parent(node) };
    while !parent.is_null() && node == unsafe { (*parent).rb_right } {
        node = parent;
        parent = unsafe { rb_parent(node) };
    }

    parent
}

/// `rb_prev` - `vendor/linux/lib/rbtree.c:512`.
pub unsafe extern "C" fn linux_rb_prev(mut node: *const LinuxRbNode) -> *mut LinuxRbNode {
    if node.is_null() || unsafe { (*node).__rb_parent_color == node as usize } {
        return ptr::null_mut();
    }

    if unsafe { !(*node).rb_left.is_null() } {
        node = unsafe { (*node).rb_left };
        while unsafe { !(*node).rb_right.is_null() } {
            node = unsafe { (*node).rb_right };
        }
        return node as *mut LinuxRbNode;
    }

    let mut parent = unsafe { rb_parent(node) };
    while !parent.is_null() && node == unsafe { (*parent).rb_left } {
        node = parent;
        parent = unsafe { rb_parent(node) };
    }

    parent
}

/// `rb_replace_node` - `vendor/linux/lib/rbtree.c:541`.
pub unsafe extern "C" fn linux_rb_replace_node(
    victim: *mut LinuxRbNode,
    new: *mut LinuxRbNode,
    root: *mut LinuxRbRoot,
) {
    if victim.is_null() || new.is_null() || root.is_null() {
        return;
    }

    let parent = unsafe { rb_parent(victim) };
    unsafe {
        *new = *victim;
        if !(*victim).rb_left.is_null() {
            rb_set_parent((*victim).rb_left, new);
        }
        if !(*victim).rb_right.is_null() {
            rb_set_parent((*victim).rb_right, new);
        }
        rb_change_child(victim, new, parent, root);
    }
}

/// `rb_replace_node_rcu` - `vendor/linux/lib/rbtree.c:558`.
pub unsafe extern "C" fn linux_rb_replace_node_rcu(
    victim: *mut LinuxRbNode,
    new: *mut LinuxRbNode,
    root: *mut LinuxRbRoot,
) {
    unsafe { linux_rb_replace_node(victim, new, root) };
}

/// `rb_next_postorder` - `vendor/linux/lib/rbtree.c:592`.
pub unsafe extern "C" fn linux_rb_next_postorder(node: *const LinuxRbNode) -> *mut LinuxRbNode {
    if node.is_null() {
        return ptr::null_mut();
    }

    let parent = unsafe { rb_parent(node) };
    if !parent.is_null()
        && node == unsafe { (*parent).rb_left }
        && unsafe { !(*parent).rb_right.is_null() }
    {
        unsafe { rb_left_deepest_node((*parent).rb_right) }
    } else {
        parent
    }
}

/// `rb_first_postorder` - `vendor/linux/lib/rbtree.c:611`.
pub unsafe extern "C" fn linux_rb_first_postorder(root: *const LinuxRbRoot) -> *mut LinuxRbNode {
    if root.is_null() || unsafe { (*root).rb_node.is_null() } {
        return ptr::null_mut();
    }

    unsafe { rb_left_deepest_node((*root).rb_node) }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::vec::Vec;
    use core::mem::{align_of, size_of};

    use super::*;

    #[repr(C)]
    struct TestEntry {
        rb: LinuxRbNode,
        key: i32,
    }

    impl TestEntry {
        const fn new(key: i32) -> Self {
            Self {
                rb: LinuxRbNode {
                    __rb_parent_color: 0,
                    rb_right: ptr::null_mut(),
                    rb_left: ptr::null_mut(),
                },
                key,
            }
        }
    }

    unsafe fn entry_from_rb(node: *const LinuxRbNode) -> *const TestEntry {
        node.cast::<TestEntry>()
    }

    unsafe fn rb_first(root: *const LinuxRbRoot) -> *mut LinuxRbNode {
        let mut node = unsafe { (*root).rb_node };
        if node.is_null() {
            return ptr::null_mut();
        }
        while unsafe { !(*node).rb_left.is_null() } {
            node = unsafe { (*node).rb_left };
        }
        node
    }

    unsafe fn rb_last(root: *const LinuxRbRoot) -> *mut LinuxRbNode {
        let mut node = unsafe { (*root).rb_node };
        if node.is_null() {
            return ptr::null_mut();
        }
        while unsafe { !(*node).rb_right.is_null() } {
            node = unsafe { (*node).rb_right };
        }
        node
    }

    unsafe fn rb_link_node(
        node: *mut LinuxRbNode,
        parent: *mut LinuxRbNode,
        link: *mut *mut LinuxRbNode,
    ) {
        unsafe {
            (*node).__rb_parent_color = parent as usize;
            (*node).rb_left = ptr::null_mut();
            (*node).rb_right = ptr::null_mut();
            *link = node;
        }
    }

    unsafe fn insert(root: *mut LinuxRbRoot, entry: *mut TestEntry) {
        let mut link = unsafe { ptr::addr_of_mut!((*root).rb_node) };
        let mut parent = ptr::null_mut();
        unsafe {
            while !(*link).is_null() {
                parent = *link;
                if (*entry).key < (*entry_from_rb(parent)).key {
                    link = ptr::addr_of_mut!((*parent).rb_left);
                } else {
                    link = ptr::addr_of_mut!((*parent).rb_right);
                }
            }
            rb_link_node(ptr::addr_of_mut!((*entry).rb), parent, link);
            linux_rb_insert_color(ptr::addr_of_mut!((*entry).rb), root);
        }
    }

    unsafe fn inorder_keys(root: *const LinuxRbRoot) -> Vec<i32> {
        let mut keys = Vec::new();
        let mut node = unsafe { rb_first(root) };
        while !node.is_null() {
            keys.push(unsafe { (*entry_from_rb(node)).key });
            node = unsafe { linux_rb_next(node) };
        }
        keys
    }

    #[test]
    fn layout_matches_vendor_rbtree_types() {
        assert_eq!(size_of::<LinuxRbNode>(), 24);
        assert_eq!(align_of::<LinuxRbNode>(), 8);
        assert_eq!(size_of::<LinuxRbRoot>(), 8);
        assert_eq!(size_of::<LinuxRbNodeLinked>(), 40);
        assert_eq!(size_of::<LinuxRbRootLinked>(), 16);

        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/rbtree.c"
        ));
        assert!(source.contains("EXPORT_SYMBOL(__rb_erase_color);"));
        assert!(source.contains("EXPORT_SYMBOL(__rb_insert_augmented);"));
        assert!(source.contains("EXPORT_SYMBOL(rb_erase);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(rb_erase_linked);"));
        assert!(source.contains("EXPORT_SYMBOL(rb_first_postorder);"));
        assert!(source.contains("EXPORT_SYMBOL(rb_insert_color);"));
        assert!(source.contains("EXPORT_SYMBOL(rb_next);"));
        assert!(source.contains("EXPORT_SYMBOL(rb_next_postorder);"));
        assert!(source.contains("EXPORT_SYMBOL(rb_prev);"));
        assert!(source.contains("EXPORT_SYMBOL(rb_replace_node);"));
        assert!(source.contains("EXPORT_SYMBOL(rb_replace_node_rcu);"));
    }

    #[test]
    fn insert_iterate_and_erase_match_sorted_order_contract() {
        let mut root = LinuxRbRoot {
            rb_node: ptr::null_mut(),
        };
        let mut entries = [
            TestEntry::new(5),
            TestEntry::new(2),
            TestEntry::new(8),
            TestEntry::new(1),
            TestEntry::new(3),
            TestEntry::new(7),
            TestEntry::new(9),
        ];

        for entry in entries.iter_mut() {
            unsafe { insert(&mut root, entry) };
        }

        assert_eq!(
            unsafe { inorder_keys(&root) },
            Vec::from([1, 2, 3, 5, 7, 8, 9])
        );

        let mut reverse = Vec::new();
        let mut node = unsafe { rb_last(&root) };
        while !node.is_null() {
            reverse.push(unsafe { (*entry_from_rb(node)).key });
            node = unsafe { linux_rb_prev(node) };
        }
        assert_eq!(reverse, Vec::from([9, 8, 7, 5, 3, 2, 1]));

        unsafe {
            linux_rb_erase(ptr::addr_of_mut!(entries[0].rb), &mut root);
            linux_rb_erase(ptr::addr_of_mut!(entries[1].rb), &mut root);
        }
        assert_eq!(unsafe { inorder_keys(&root) }, Vec::from([1, 3, 7, 8, 9]));
    }

    #[test]
    fn postorder_walk_reaches_every_node() {
        let mut root = LinuxRbRoot {
            rb_node: ptr::null_mut(),
        };
        let mut entries = [
            TestEntry::new(4),
            TestEntry::new(2),
            TestEntry::new(6),
            TestEntry::new(1),
            TestEntry::new(3),
        ];

        for entry in entries.iter_mut() {
            unsafe { insert(&mut root, entry) };
        }

        let mut seen = Vec::new();
        let mut node = unsafe { linux_rb_first_postorder(&root) };
        while !node.is_null() {
            seen.push(unsafe { (*entry_from_rb(node)).key });
            node = unsafe { linux_rb_next_postorder(node) };
        }
        seen.sort_unstable();
        assert_eq!(seen, Vec::from([1, 2, 3, 4, 6]));
    }

    #[test]
    fn aggregate_exports_include_vendor_rbtree_symbols() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("__rb_erase_color"),
            Some(linux___rb_erase_color as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("rb_next_postorder"),
            Some(linux_rb_next_postorder as usize)
        );
    }
}
