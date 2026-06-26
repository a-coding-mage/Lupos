//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
/// Intrusive doubly-linked circular list — a direct port of Linux's
/// `struct list_head` from `include/linux/list.h`.
///
/// This is the backbone data structure for the buddy allocator's free lists.
/// Each `FreeArea` contains a `ListHead` sentinel (head node), and free
/// `Page` structs are linked via their embedded `lru` field.
///
/// ## Safety
///
/// All operations are `unsafe` because they manipulate raw pointers.
/// The caller must ensure:
/// - Pointers are valid and properly aligned
/// - No aliasing violations (single writer or synchronized access)
/// - Nodes are not used-after-free
///
/// Ref: Linux include/linux/list.h
///      Linux include/linux/types.h — struct list_head

/// Intrusive doubly-linked list node.
///
/// A standalone `ListHead` acts as a sentinel (head) for a circular list.
/// When embedded in a struct (e.g., `Page::lru`), it links the struct
/// into a list.
///
/// Invariant: an initialized, non-empty list forms a closed ring:
///   head → A → B → ... → head (via `next`)
///   head ← A ← B ← ... ← head (via `prev`)
///
/// An empty list points to itself: `head.next == head`, `head.prev == head`.
#[repr(C)]
pub struct ListHead {
    pub next: *mut ListHead,
    pub prev: *mut ListHead,
}

// ListHead contains raw pointers which are !Send/!Sync by default.
// The buddy allocator protects all list access with a spinlock, so we
// assert Send+Sync to allow the allocator struct to be stored in statics.
unsafe impl Send for ListHead {}
unsafe impl Sync for ListHead {}

#[allow(unsafe_op_in_unsafe_fn)]
impl ListHead {
    /// Create an uninitialized list head.
    ///
    /// MUST call `init()` before use.  This exists for `const` contexts
    /// where we cannot take `&mut self` (e.g., static array init).
    pub const fn uninit() -> Self {
        ListHead {
            next: core::ptr::null_mut(),
            prev: core::ptr::null_mut(),
        }
    }

    /// Initialize this node as an empty list (points to itself).
    ///
    /// Equivalent to Linux's `INIT_LIST_HEAD()`.
    ///
    /// # Safety
    /// `self` must be a valid, aligned pointer.
    #[inline]
    pub unsafe fn init(this: *mut ListHead) {
        (*this).next = this;
        (*this).prev = this;
    }

    /// Check if the list is empty (head points to itself).
    ///
    /// Equivalent to Linux's `list_empty()`.
    ///
    /// # Safety
    /// `head` must be a valid, initialized list head.
    #[inline]
    pub unsafe fn is_empty(head: *const ListHead) -> bool {
        (*head).next as *const ListHead == head
    }

    /// Insert `new` between `prev` and `next`.
    ///
    /// This is the internal helper — callers should use `list_add` or
    /// `list_add_tail` instead.
    ///
    /// Equivalent to Linux's `__list_add()`.
    ///
    /// # Safety
    /// All three pointers must be valid and initialized.
    #[inline]
    unsafe fn __list_add(new: *mut ListHead, prev: *mut ListHead, next: *mut ListHead) {
        (*next).prev = new;
        (*new).next = next;
        (*new).prev = prev;
        (*prev).next = new;
    }

    /// Add `new` at the front of `head`'s list (after head, before head.next).
    ///
    /// Equivalent to Linux's `list_add()`.
    ///
    /// # Safety
    /// Both `new` and `head` must be valid. `new` must not already be in a list.
    #[inline]
    pub unsafe fn list_add(new: *mut ListHead, head: *mut ListHead) {
        Self::__list_add(new, head, (*head).next);
    }

    /// Add `new` at the back of `head`'s list (before head, after head.prev).
    ///
    /// Equivalent to Linux's `list_add_tail()`.
    ///
    /// # Safety
    /// Both `new` and `head` must be valid. `new` must not already be in a list.
    #[inline]
    pub unsafe fn list_add_tail(new: *mut ListHead, head: *mut ListHead) {
        Self::__list_add(new, head.as_ref().unwrap().prev, head);
    }

    /// Remove `entry` from its list and reinitialize it to point to itself.
    ///
    /// Equivalent to Linux's `list_del_init()`.
    ///
    /// # Safety
    /// `entry` must be a valid node currently linked in a list.
    #[inline]
    pub unsafe fn list_del(entry: *mut ListHead) {
        let prev = (*entry).prev;
        let next = (*entry).next;
        (*prev).next = next;
        (*next).prev = prev;
        // Re-initialize to prevent dangling references.
        Self::init(entry);
    }

    /// Return the first entry in the list, or `None` if empty.
    ///
    /// Equivalent to Linux's `list_first_entry_or_null()`.
    ///
    /// # Safety
    /// `head` must be a valid, initialized list head.
    #[inline]
    pub unsafe fn first_entry(head: *const ListHead) -> Option<*mut ListHead> {
        if Self::is_empty(head) {
            None
        } else {
            Some((*head).next)
        }
    }
}

/// Convert a `ListHead` pointer embedded at `$field` in `$container_type`
/// back to a pointer to the containing struct.
///
/// This is a Rust port of Linux's `container_of()` / `list_entry()` macro.
///
/// # Safety
/// The `$ptr` must point to the `$field` member of a valid `$container_type`.
///
/// # Example
/// ```ignore
/// let page_ptr: *mut Page = container_of!(list_ptr, Page, lru);
/// ```
#[macro_export]
macro_rules! container_of {
    ($ptr:expr, $container:ty, $field:ident) => {{
        let offset = core::mem::offset_of!($container, $field);
        ($ptr as *const u8).sub(offset) as *mut $container
    }};
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
#[allow(unsafe_op_in_unsafe_fn)]
mod tests {
    use super::*;

    #[test]
    fn empty_list_is_empty() {
        unsafe {
            let mut head = ListHead::uninit();
            ListHead::init(&mut head);
            assert!(ListHead::is_empty(&head));
            // Verify the self-referential invariant via raw pointer.
            let head_ptr = &mut head as *mut ListHead;
            assert_eq!(head.next, head_ptr);
            assert_eq!(head.prev, head_ptr);
        }
    }

    #[test]
    fn add_single_entry_makes_list_non_empty() {
        unsafe {
            let mut head = ListHead::uninit();
            let mut a = ListHead::uninit();
            ListHead::init(&mut head);
            ListHead::init(&mut a);
            ListHead::list_add(&mut a, &mut head);
            assert!(!ListHead::is_empty(&head));
            // head → a → head
            assert_eq!(head.next, &mut a as *mut ListHead);
            assert_eq!(head.prev, &mut a as *mut ListHead);
            assert_eq!(a.next, &mut head as *mut ListHead);
            assert_eq!(a.prev, &mut head as *mut ListHead);
        }
    }

    #[test]
    fn add_tail_ordering() {
        unsafe {
            let mut head = ListHead::uninit();
            let mut a = ListHead::uninit();
            let mut b = ListHead::uninit();
            ListHead::init(&mut head);
            ListHead::init(&mut a);
            ListHead::init(&mut b);
            // Add a, then b to tail → order should be: head → a → b → head
            ListHead::list_add_tail(&mut a, &mut head);
            ListHead::list_add_tail(&mut b, &mut head);
            assert_eq!(head.next, &mut a as *mut ListHead);
            assert_eq!(a.next, &mut b as *mut ListHead);
            assert_eq!(b.next, &mut head as *mut ListHead);
        }
    }

    #[test]
    fn list_add_front_ordering() {
        unsafe {
            let mut head = ListHead::uninit();
            let mut a = ListHead::uninit();
            let mut b = ListHead::uninit();
            ListHead::init(&mut head);
            ListHead::init(&mut a);
            ListHead::init(&mut b);
            // Add a, then b to front → order should be: head → b → a → head
            ListHead::list_add(&mut a, &mut head);
            ListHead::list_add(&mut b, &mut head);
            assert_eq!(head.next, &mut b as *mut ListHead);
            assert_eq!(b.next, &mut a as *mut ListHead);
            assert_eq!(a.next, &mut head as *mut ListHead);
        }
    }

    #[test]
    fn del_removes_entry() {
        unsafe {
            let mut head = ListHead::uninit();
            let mut a = ListHead::uninit();
            let mut b = ListHead::uninit();
            ListHead::init(&mut head);
            ListHead::init(&mut a);
            ListHead::init(&mut b);
            ListHead::list_add_tail(&mut a, &mut head);
            ListHead::list_add_tail(&mut b, &mut head);
            // Remove a → head → b → head
            ListHead::list_del(&mut a);
            assert_eq!(head.next, &mut b as *mut ListHead);
            assert_eq!(b.next, &mut head as *mut ListHead);
            // a should be re-initialized (points to itself)
            assert!(ListHead::is_empty(&a));
        }
    }

    #[test]
    fn del_last_entry_makes_list_empty() {
        unsafe {
            let mut head = ListHead::uninit();
            let mut a = ListHead::uninit();
            ListHead::init(&mut head);
            ListHead::init(&mut a);
            ListHead::list_add(&mut a, &mut head);
            ListHead::list_del(&mut a);
            assert!(ListHead::is_empty(&head));
        }
    }

    #[test]
    fn first_entry_returns_first_or_none() {
        unsafe {
            let mut head = ListHead::uninit();
            ListHead::init(&mut head);
            assert!(ListHead::first_entry(&head).is_none());

            let mut a = ListHead::uninit();
            ListHead::init(&mut a);
            ListHead::list_add(&mut a, &mut head);
            assert_eq!(ListHead::first_entry(&head), Some(&mut a as *mut ListHead));
        }
    }

    #[test]
    fn list_iteration_count() {
        unsafe {
            let mut head = ListHead::uninit();
            ListHead::init(&mut head);
            let mut nodes: [ListHead; 5] = core::array::from_fn(|_| ListHead::uninit());
            for node in nodes.iter_mut() {
                ListHead::init(node);
                ListHead::list_add_tail(node, &mut head);
            }

            // Walk the list and count entries.
            let mut count = 0;
            let mut cursor = head.next;
            let head_ptr = &mut head as *mut ListHead;
            while cursor != head_ptr {
                count += 1;
                cursor = (*cursor).next;
            }
            assert_eq!(count, 5);
        }
    }
}
