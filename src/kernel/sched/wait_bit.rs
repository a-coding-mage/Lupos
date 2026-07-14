//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/wait_bit.c
//! test-origin: linux:vendor/linux/kernel/sched/wait_bit.c
//! Wait-on-bit helpers.
//!
//! Mirrors `vendor/linux/kernel/sched/wait_bit.c`.

use core::ffi::c_void;
use core::mem::{offset_of, size_of};
use core::sync::atomic::{AtomicU8, AtomicU64, Ordering};

use super::wait::WaitQueueHead;
use crate::include::uapi::errno::EAGAIN;
use crate::kernel::module::{export_symbol, find_symbol};

const WAIT_TABLE_BITS: usize = 8;
const WAIT_TABLE_SIZE: usize = 1 << WAIT_TABLE_BITS;
const BITS_PER_LONG: usize = usize::BITS as usize;
const TASK_NORMAL: u32 = 0x0003;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("bit_waitqueue", linux_bit_waitqueue as usize, false);
    export_symbol_once("wake_bit_function", linux_wake_bit_function as usize, false);
    export_symbol_once("__wait_on_bit", linux___wait_on_bit as usize, false);
    export_symbol_once(
        "out_of_line_wait_on_bit",
        linux_out_of_line_wait_on_bit as usize,
        false,
    );
    export_symbol_once(
        "out_of_line_wait_on_bit_timeout",
        linux_out_of_line_wait_on_bit_timeout as usize,
        true,
    );
    export_symbol_once(
        "__wait_on_bit_lock",
        linux___wait_on_bit_lock as usize,
        false,
    );
    export_symbol_once(
        "out_of_line_wait_on_bit_lock",
        linux_out_of_line_wait_on_bit_lock as usize,
        false,
    );
    export_symbol_once("__wake_up_bit", linux___wake_up_bit as usize, false);
    export_symbol_once("wake_up_bit", linux_wake_up_bit as usize, false);
    export_symbol_once("__var_waitqueue", linux___var_waitqueue as usize, false);
    export_symbol_once(
        "init_wait_var_entry",
        linux_init_wait_var_entry as usize,
        false,
    );
    export_symbol_once("wake_up_var", linux_wake_up_var as usize, false);
    export_symbol_once("bit_wait", linux_bit_wait as usize, false);
    export_symbol_once("bit_wait_io", linux_bit_wait_io as usize, false);
    export_symbol_once("bit_wait_timeout", linux_bit_wait_timeout as usize, false);
}

#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxListHead {
    next: *mut LinuxListHead,
    prev: *mut LinuxListHead,
}

impl LinuxListHead {
    const fn empty() -> Self {
        Self {
            next: core::ptr::null_mut(),
            prev: core::ptr::null_mut(),
        }
    }
}

type LinuxWaitQueueFunc =
    unsafe extern "C" fn(*mut LinuxWaitQueueEntry, u32, i32, *mut c_void) -> i32;
type LinuxWaitBitAction = unsafe extern "C" fn(*mut LinuxWaitBitKey, i32) -> i32;

#[repr(C)]
struct LinuxWaitQueueEntry {
    flags: u32,
    private: *mut c_void,
    func: Option<LinuxWaitQueueFunc>,
    entry: LinuxListHead,
}

#[repr(C)]
struct LinuxWaitBitKey {
    flags: *mut c_void,
    bit_nr: i32,
    timeout: usize,
}

#[repr(C)]
struct LinuxWaitBitQueueEntry {
    key: LinuxWaitBitKey,
    wq_entry: LinuxWaitQueueEntry,
}

static WAIT_TABLE_STATE: AtomicU8 = AtomicU8::new(0);
static mut BIT_WAIT_TABLE: [LinuxListHead; WAIT_TABLE_SIZE] =
    [LinuxListHead::empty(); WAIT_TABLE_SIZE];

fn ensure_wait_table_init() {
    if WAIT_TABLE_STATE.load(Ordering::Acquire) == 2 {
        return;
    }

    if WAIT_TABLE_STATE
        .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        unsafe {
            let base = core::ptr::addr_of_mut!(BIT_WAIT_TABLE).cast::<LinuxListHead>();
            for index in 0..WAIT_TABLE_SIZE {
                let head = base.add(index);
                (*head).next = head;
                (*head).prev = head;
            }
        }
        WAIT_TABLE_STATE.store(2, Ordering::Release);
        return;
    }

    while WAIT_TABLE_STATE.load(Ordering::Acquire) != 2 {
        core::hint::spin_loop();
    }
}

fn wait_table_head(index: usize) -> *mut LinuxListHead {
    ensure_wait_table_init();
    unsafe {
        core::ptr::addr_of_mut!(BIT_WAIT_TABLE)
            .cast::<LinuxListHead>()
            .add(index)
    }
}

fn hash_ptr(ptr: *const c_void) -> usize {
    let value = ptr as usize;
    (value ^ (value >> 8) ^ (value >> 16)) & (WAIT_TABLE_SIZE - 1)
}

fn hash_bit(word: *const c_void, bit: i32) -> usize {
    let shift = if BITS_PER_LONG == 32 { 5 } else { 6 };
    let value = ((word as usize) << shift) ^ bit as usize;
    (value ^ (value >> 8) ^ (value >> 16)) & (WAIT_TABLE_SIZE - 1)
}

fn test_bit_raw(word: *const c_void, bit: i32) -> bool {
    if word.is_null() || bit < 0 {
        return false;
    }

    let bit = bit as usize;
    let index = bit / BITS_PER_LONG;
    let mask = 1usize << (bit % BITS_PER_LONG);
    let value = unsafe { core::ptr::read_volatile(word.cast::<usize>().add(index)) };
    value & mask != 0
}

fn set_bit_raw(word: *mut c_void, bit: i32) {
    if word.is_null() || bit < 0 {
        return;
    }

    let bit = bit as usize;
    let index = bit / BITS_PER_LONG;
    let mask = 1usize << (bit % BITS_PER_LONG);
    unsafe {
        let ptr = word.cast::<usize>().add(index);
        let value = core::ptr::read_volatile(ptr);
        core::ptr::write_volatile(ptr, value | mask);
    }
}

unsafe fn init_list_head(head: *mut LinuxListHead) {
    if head.is_null() {
        return;
    }
    unsafe {
        (*head).next = head;
        (*head).prev = head;
    }
}

unsafe fn list_del_init(node: *mut LinuxListHead) {
    if node.is_null() {
        return;
    }

    unsafe {
        let next = (*node).next;
        let prev = (*node).prev;
        if !next.is_null() && !prev.is_null() {
            (*next).prev = prev;
            (*prev).next = next;
        }
        init_list_head(node);
    }
}

unsafe fn entry_from_list(node: *mut LinuxListHead) -> *mut LinuxWaitQueueEntry {
    unsafe {
        node.cast::<u8>()
            .sub(offset_of!(LinuxWaitQueueEntry, entry))
            .cast::<LinuxWaitQueueEntry>()
    }
}

unsafe fn bit_entry_from_wait(wait: *mut LinuxWaitQueueEntry) -> *mut LinuxWaitBitQueueEntry {
    unsafe {
        wait.cast::<u8>()
            .sub(offset_of!(LinuxWaitBitQueueEntry, wq_entry))
            .cast::<LinuxWaitBitQueueEntry>()
    }
}

unsafe fn wake_queue_with_key(head: *mut LinuxListHead, key: *mut LinuxWaitBitKey) {
    if head.is_null() {
        return;
    }

    unsafe {
        if (*head).next.is_null() || (*head).prev.is_null() {
            init_list_head(head);
            return;
        }

        let mut node = (*head).next;
        let mut visited = 0usize;
        while !node.is_null() && node != head && visited < 4096 {
            let next = (*node).next;
            let wait = entry_from_list(node);
            if let Some(func) = (*wait).func {
                let _ = func(wait, TASK_NORMAL, 0, key.cast());
            }
            node = next;
            visited += 1;
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WaitBitKey {
    pub word: usize,
    pub bit: u32,
}

impl WaitBitKey {
    pub const fn new(word: usize, bit: u32) -> Self {
        Self { word, bit }
    }

    pub const fn hash(self) -> u64 {
        ((self.word as u64) >> 3) ^ (self.bit as u64)
    }
}

pub fn test_bit(word: &AtomicU64, bit: u32) -> bool {
    word.load(Ordering::Acquire) & (1u64 << (bit & 63)) != 0
}

pub fn clear_bit_unlock(word: &AtomicU64, bit: u32) {
    word.fetch_and(!(1u64 << (bit & 63)), Ordering::Release);
}

pub fn wait_on_bit(word: &AtomicU64, bit: u32, _queue: &WaitQueueHead) -> bool {
    !test_bit(word, bit)
}

pub fn wake_up_bit(queue: &WaitQueueHead) -> usize {
    queue.wake_up_all()
}

/// `bit_waitqueue` - `vendor/linux/kernel/sched/wait_bit.c:15`.
unsafe extern "C" fn linux_bit_waitqueue(word: *mut c_void, bit: i32) -> *mut c_void {
    wait_table_head(hash_bit(word.cast_const(), bit)).cast()
}

/// `wake_bit_function` - `vendor/linux/kernel/sched/wait_bit.c:23`.
unsafe extern "C" fn linux_wake_bit_function(
    wait: *mut LinuxWaitQueueEntry,
    _mode: u32,
    _sync: i32,
    arg: *mut c_void,
) -> i32 {
    if wait.is_null() || arg.is_null() {
        return 0;
    }

    let key = arg.cast::<LinuxWaitBitKey>();
    let wait_bit = unsafe { bit_entry_from_wait(wait) };
    unsafe {
        if (*wait_bit).key.flags != (*key).flags
            || (*wait_bit).key.bit_nr != (*key).bit_nr
            || test_bit_raw((*key).flags.cast_const(), (*key).bit_nr)
        {
            return 0;
        }
        list_del_init(core::ptr::addr_of_mut!((*wait).entry));
    }
    1
}

unsafe fn wait_on_bit_once(
    entry: *mut LinuxWaitBitQueueEntry,
    action: Option<LinuxWaitBitAction>,
    mode: u32,
) -> i32 {
    if entry.is_null() {
        return 0;
    }

    unsafe {
        if !test_bit_raw((*entry).key.flags.cast_const(), (*entry).key.bit_nr) {
            return 0;
        }
        if let Some(action) = action {
            return action(core::ptr::addr_of_mut!((*entry).key), mode as i32);
        }
    }
    0
}

/// `__wait_on_bit` - `vendor/linux/kernel/sched/wait_bit.c:39`.
unsafe extern "C" fn linux___wait_on_bit(
    _head: *mut c_void,
    entry: *mut LinuxWaitBitQueueEntry,
    action: Option<LinuxWaitBitAction>,
    mode: u32,
) -> i32 {
    unsafe { wait_on_bit_once(entry, action, mode) }
}

/// `out_of_line_wait_on_bit` - `vendor/linux/kernel/sched/wait_bit.c:55`.
unsafe extern "C" fn linux_out_of_line_wait_on_bit(
    word: *mut c_void,
    bit: i32,
    action: Option<LinuxWaitBitAction>,
    mode: u32,
) -> i32 {
    let mut entry = LinuxWaitBitQueueEntry {
        key: LinuxWaitBitKey {
            flags: word,
            bit_nr: bit,
            timeout: 0,
        },
        wq_entry: LinuxWaitQueueEntry {
            flags: 0,
            private: unsafe { crate::kernel::sched::get_current().cast() },
            func: Some(linux_wake_bit_function),
            entry: LinuxListHead::empty(),
        },
    };
    unsafe {
        init_list_head(core::ptr::addr_of_mut!(entry.wq_entry.entry));
        wait_on_bit_once(&mut entry, action, mode)
    }
}

/// `out_of_line_wait_on_bit_timeout` - `vendor/linux/kernel/sched/wait_bit.c:65`.
unsafe extern "C" fn linux_out_of_line_wait_on_bit_timeout(
    word: *mut c_void,
    bit: i32,
    action: Option<LinuxWaitBitAction>,
    mode: u32,
    timeout: usize,
) -> i32 {
    let mut entry = LinuxWaitBitQueueEntry {
        key: LinuxWaitBitKey {
            flags: word,
            bit_nr: bit,
            timeout: crate::kernel::time::jiffies::jiffies().saturating_add(timeout as u64)
                as usize,
        },
        wq_entry: LinuxWaitQueueEntry {
            flags: 0,
            private: unsafe { crate::kernel::sched::get_current().cast() },
            func: Some(linux_wake_bit_function),
            entry: LinuxListHead::empty(),
        },
    };
    unsafe {
        init_list_head(core::ptr::addr_of_mut!(entry.wq_entry.entry));
        wait_on_bit_once(&mut entry, action, mode)
    }
}

/// `__wait_on_bit_lock` - `vendor/linux/kernel/sched/wait_bit.c:76`.
unsafe extern "C" fn linux___wait_on_bit_lock(
    _head: *mut c_void,
    entry: *mut LinuxWaitBitQueueEntry,
    action: Option<LinuxWaitBitAction>,
    mode: u32,
) -> i32 {
    if entry.is_null() {
        return 0;
    }

    unsafe {
        let ret = wait_on_bit_once(entry, action, mode);
        if ret == 0 && !test_bit_raw((*entry).key.flags.cast_const(), (*entry).key.bit_nr) {
            set_bit_raw((*entry).key.flags, (*entry).key.bit_nr);
        }
        ret
    }
}

/// `out_of_line_wait_on_bit_lock` - `vendor/linux/kernel/sched/wait_bit.c:103`.
unsafe extern "C" fn linux_out_of_line_wait_on_bit_lock(
    word: *mut c_void,
    bit: i32,
    action: Option<LinuxWaitBitAction>,
    mode: u32,
) -> i32 {
    let mut entry = LinuxWaitBitQueueEntry {
        key: LinuxWaitBitKey {
            flags: word,
            bit_nr: bit,
            timeout: 0,
        },
        wq_entry: LinuxWaitQueueEntry {
            flags: 0,
            private: unsafe { crate::kernel::sched::get_current().cast() },
            func: Some(linux_wake_bit_function),
            entry: LinuxListHead::empty(),
        },
    };
    unsafe {
        init_list_head(core::ptr::addr_of_mut!(entry.wq_entry.entry));
        linux___wait_on_bit_lock(core::ptr::null_mut(), &mut entry, action, mode)
    }
}

/// `__wake_up_bit` - `vendor/linux/kernel/sched/wait_bit.c:112`.
unsafe extern "C" fn linux___wake_up_bit(head: *mut c_void, word: *mut c_void, bit: i32) {
    let mut key = LinuxWaitBitKey {
        flags: word,
        bit_nr: bit,
        timeout: 0,
    };
    unsafe {
        wake_queue_with_key(head.cast(), core::ptr::addr_of_mut!(key));
    }
}

/// `wake_up_bit` - `vendor/linux/kernel/sched/wait_bit.c:147`.
unsafe extern "C" fn linux_wake_up_bit(word: *mut c_void, bit: i32) {
    let head = unsafe { linux_bit_waitqueue(word, bit) };
    unsafe {
        linux___wake_up_bit(head, word, bit);
    }
}

/// `__var_waitqueue` - `vendor/linux/kernel/sched/wait_bit.c:154`.
unsafe extern "C" fn linux___var_waitqueue(var: *mut c_void) -> *mut c_void {
    wait_table_head(hash_ptr(var.cast_const())).cast()
}

unsafe extern "C" fn linux_var_wake_function(
    wait: *mut LinuxWaitQueueEntry,
    _mode: u32,
    _sync: i32,
    arg: *mut c_void,
) -> i32 {
    if wait.is_null() || arg.is_null() {
        return 0;
    }

    let key = arg.cast::<LinuxWaitBitKey>();
    let wait_bit = unsafe { bit_entry_from_wait(wait) };
    unsafe {
        if (*wait_bit).key.flags != (*key).flags || (*wait_bit).key.bit_nr != (*key).bit_nr {
            return 0;
        }
        list_del_init(core::ptr::addr_of_mut!((*wait).entry));
    }
    1
}

/// `init_wait_var_entry` - `vendor/linux/kernel/sched/wait_bit.c:174`.
unsafe extern "C" fn linux_init_wait_var_entry(
    entry: *mut LinuxWaitBitQueueEntry,
    var: *mut c_void,
    flags: i32,
) {
    if entry.is_null() {
        return;
    }

    unsafe {
        core::ptr::write_bytes(entry, 0, size_of::<LinuxWaitBitQueueEntry>());
        (*entry).key.flags = var;
        (*entry).key.bit_nr = -1;
        (*entry).wq_entry.flags = flags as u32;
        (*entry).wq_entry.private = crate::kernel::sched::get_current().cast();
        (*entry).wq_entry.func = Some(linux_var_wake_function);
        init_list_head(core::ptr::addr_of_mut!((*entry).wq_entry.entry));
    }
}

/// `wake_up_var` - `vendor/linux/kernel/sched/wait_bit.c:232`.
unsafe extern "C" fn linux_wake_up_var(var: *mut c_void) {
    let head = unsafe { linux___var_waitqueue(var) };
    unsafe {
        linux___wake_up_bit(head, var, -1);
    }
}

/// `bit_wait` - `vendor/linux/kernel/sched/wait_bit.c:238`.
unsafe extern "C" fn linux_bit_wait(_key: *mut LinuxWaitBitKey, _mode: i32) -> i32 {
    #[cfg(not(test))]
    unsafe {
        crate::kernel::sched::schedule_with_irqs_enabled();
    }
    0
}

/// `bit_wait_io` - `vendor/linux/kernel/sched/wait_bit.c:249`.
unsafe extern "C" fn linux_bit_wait_io(key: *mut LinuxWaitBitKey, mode: i32) -> i32 {
    unsafe { linux_bit_wait(key, mode) }
}

/// `bit_wait_timeout` - `vendor/linux/kernel/sched/wait_bit.c:260`.
unsafe extern "C" fn linux_bit_wait_timeout(key: *mut LinuxWaitBitKey, mode: i32) -> i32 {
    if !key.is_null() && crate::kernel::time::jiffies::jiffies() >= unsafe { (*key).timeout } as u64
    {
        return -EAGAIN;
    }
    unsafe { linux_bit_wait(key, mode) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_bit_hash_includes_word_and_bit() {
        let a = WaitBitKey::new(0x1000, 1);
        let b = WaitBitKey::new(0x1000, 2);
        assert_ne!(a.hash(), b.hash());
    }

    #[test]
    fn wait_on_bit_reports_clear_state() {
        let word = AtomicU64::new(1 << 3);
        let q = WaitQueueHead::new();
        assert!(!wait_on_bit(&word, 3, &q));
        clear_bit_unlock(&word, 3);
        assert!(wait_on_bit(&word, 3, &q));
    }
}
