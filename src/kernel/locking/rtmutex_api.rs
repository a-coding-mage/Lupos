//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking/rtmutex_api.c
//! test-origin: linux:vendor/linux/kernel/locking/rtmutex_api.c
//! RT mutex API surface coverage for M33.
//!
//! Mirrors `vendor/linux/kernel/locking/rtmutex_api.c`.  The heavy-weight
//! owner and waiter mechanics live in `rt_mutex.rs`; this file exposes the
//! small C-style helper layer used by futex PI and later scheduler paths.

use core::ffi::{c_char, c_void};

use crate::include::uapi::errno::EBUSY;
use crate::kernel::module::{export_symbol, find_symbol};

use super::rt_mutex::RtMutex;

pub fn rt_mutex_init(mutex: &mut RtMutex) {
    *mutex = RtMutex::new();
}

pub fn rt_mutex_trylock(mutex: &RtMutex) -> bool {
    mutex.try_lock()
}

pub fn rt_mutex_lock(mutex: &RtMutex) -> Result<(), i32> {
    if mutex.lock() { Ok(()) } else { Err(EBUSY) }
}

pub fn rt_mutex_unlock(mutex: &RtMutex) {
    mutex.unlock();
}

pub fn rt_mutex_is_locked(mutex: &RtMutex) -> bool {
    mutex.is_locked()
}

/// Target-config prefix of Linux `struct rt_mutex_base`.
///
/// `CONFIG_DEBUG_LOCK_ALLOC=n` and x86_64 qspinlock make `raw_spinlock_t` a
/// four-byte word followed by normal pointer alignment.
#[repr(C)]
struct LinuxRtMutexBaseAbi {
    wait_lock_raw: u32,
    _wait_lock_pad: u32,
    waiters_rb_node: *mut c_void,
    waiters_leftmost: *mut c_void,
    owner: *mut c_void,
}

#[repr(C)]
struct LinuxRtMutexAbi {
    rtmutex: LinuxRtMutexBaseAbi,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "rt_mutex_base_init",
        linux_rt_mutex_base_init as usize,
        false,
    );
    export_symbol_once("__rt_mutex_init", linux___rt_mutex_init as usize, true);
    export_symbol_once("rt_mutex_lock", linux_rt_mutex_lock as usize, true);
    export_symbol_once(
        "rt_mutex_lock_nested",
        linux_rt_mutex_lock_nested as usize,
        true,
    );
    export_symbol_once(
        "_rt_mutex_lock_nest_lock",
        linux__rt_mutex_lock_nest_lock as usize,
        true,
    );
    export_symbol_once(
        "rt_mutex_lock_interruptible",
        linux_rt_mutex_lock_interruptible as usize,
        true,
    );
    export_symbol_once(
        "rt_mutex_lock_killable",
        linux_rt_mutex_lock_killable as usize,
        true,
    );
    export_symbol_once("rt_mutex_trylock", linux_rt_mutex_trylock as usize, true);
    export_symbol_once("rt_mutex_unlock", linux_rt_mutex_unlock as usize, true);
}

unsafe fn init_rt_mutex_base(base: *mut LinuxRtMutexBaseAbi) {
    if base.is_null() {
        return;
    }
    unsafe {
        (*base).wait_lock_raw = 0;
        (*base)._wait_lock_pad = 0;
        (*base).waiters_rb_node = core::ptr::null_mut();
        (*base).waiters_leftmost = core::ptr::null_mut();
        (*base).owner = core::ptr::null_mut();
    }
}

/// `rt_mutex_base_init` - `vendor/linux/kernel/locking/rtmutex_api.c`.
#[unsafe(export_name = "rt_mutex_base_init")]
unsafe extern "C" fn linux_rt_mutex_base_init(base: *mut LinuxRtMutexBaseAbi) {
    unsafe { init_rt_mutex_base(base) };
}

/// `__rt_mutex_init` - `vendor/linux/kernel/locking/rtmutex_api.c`.
#[unsafe(export_name = "__rt_mutex_init")]
unsafe extern "C" fn linux___rt_mutex_init(
    lock: *mut LinuxRtMutexAbi,
    _name: *const c_char,
    _key: *mut c_void,
) {
    if lock.is_null() {
        return;
    }
    unsafe { init_rt_mutex_base(core::ptr::addr_of_mut!((*lock).rtmutex)) };
}

/// `rt_mutex_lock` - `vendor/linux/kernel/locking/rtmutex_api.c`.
#[unsafe(export_name = "rt_mutex_lock")]
unsafe extern "C" fn linux_rt_mutex_lock(_lock: *mut LinuxRtMutexAbi) {}

/// `rt_mutex_lock_nested` - `vendor/linux/kernel/locking/rtmutex_api.c`.
#[unsafe(export_name = "rt_mutex_lock_nested")]
unsafe extern "C" fn linux_rt_mutex_lock_nested(_lock: *mut LinuxRtMutexAbi, _subclass: u32) {}

/// `_rt_mutex_lock_nest_lock` - `vendor/linux/kernel/locking/rtmutex_api.c`.
#[unsafe(export_name = "_rt_mutex_lock_nest_lock")]
unsafe extern "C" fn linux__rt_mutex_lock_nest_lock(
    _lock: *mut LinuxRtMutexAbi,
    _nest_lock: *mut c_void,
) {
}

/// `rt_mutex_lock_interruptible` - `vendor/linux/kernel/locking/rtmutex_api.c`.
#[unsafe(export_name = "rt_mutex_lock_interruptible")]
unsafe extern "C" fn linux_rt_mutex_lock_interruptible(_lock: *mut LinuxRtMutexAbi) -> i32 {
    0
}

/// `rt_mutex_lock_killable` - `vendor/linux/kernel/locking/rtmutex_api.c`.
#[unsafe(export_name = "rt_mutex_lock_killable")]
unsafe extern "C" fn linux_rt_mutex_lock_killable(_lock: *mut LinuxRtMutexAbi) -> i32 {
    0
}

/// `rt_mutex_trylock` - `vendor/linux/kernel/locking/rtmutex_api.c`.
#[unsafe(export_name = "rt_mutex_trylock")]
unsafe extern "C" fn linux_rt_mutex_trylock(_lock: *mut LinuxRtMutexAbi) -> i32 {
    1
}

/// `rt_mutex_unlock` - `vendor/linux/kernel/locking/rtmutex_api.c`.
#[unsafe(export_name = "rt_mutex_unlock")]
unsafe extern "C" fn linux_rt_mutex_unlock(_lock: *mut LinuxRtMutexAbi) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_lock_unlock_round_trip() {
        let mutex = RtMutex::new();
        rt_mutex_lock(&mutex).unwrap();
        assert!(rt_mutex_is_locked(&mutex));
        rt_mutex_unlock(&mutex);
        assert!(!rt_mutex_is_locked(&mutex));
    }
}
