//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking/qrwlock.c
//! test-origin: linux:vendor/linux/kernel/locking/qrwlock.c
//! Queued rwlock coverage for M33.
//!
//! Mirrors `vendor/linux/kernel/locking/qrwlock.c`.  Readers hold a positive
//! count; the writer owns the lock with the negative sentinel value.

use core::sync::atomic::{AtomicIsize, Ordering};

use crate::kernel::locking::{
    local_irq_disable, local_irq_restore, local_irq_save, preempt_disable, preempt_enable,
};
use crate::kernel::module::{export_symbol, find_symbol};

pub const QRWLOCK_WRITE_LOCKED: isize = -1;

#[repr(C)]
pub struct QrwLock {
    state: AtomicIsize,
}

impl QrwLock {
    pub const fn new() -> Self {
        Self {
            state: AtomicIsize::new(0),
        }
    }

    pub fn try_read_lock(&self) -> bool {
        loop {
            let state = self.state.load(Ordering::Acquire);
            if state < 0 {
                return false;
            }
            if self
                .state
                .compare_exchange(state, state + 1, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return true;
            }
        }
    }

    pub fn read_unlock(&self) {
        self.state.fetch_sub(1, Ordering::AcqRel);
    }

    pub fn try_write_lock(&self) -> bool {
        self.state
            .compare_exchange(0, QRWLOCK_WRITE_LOCKED, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    pub fn write_unlock(&self) {
        self.state.store(0, Ordering::Release);
    }
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("_raw_read_lock", linux_raw_read_lock as usize, false);
    export_symbol_once(
        "_raw_read_lock_irq",
        linux_raw_read_lock_irq as usize,
        false,
    );
    export_symbol_once(
        "_raw_read_lock_irqsave",
        linux_raw_read_lock_irqsave as usize,
        false,
    );
    export_symbol_once("_raw_read_unlock", linux_raw_read_unlock as usize, false);
    export_symbol_once(
        "_raw_read_unlock_irq",
        linux_raw_read_unlock_irq as usize,
        false,
    );
    export_symbol_once(
        "_raw_read_unlock_irqrestore",
        linux_raw_read_unlock_irqrestore as usize,
        false,
    );
    export_symbol_once("_raw_write_lock", linux_raw_write_lock as usize, false);
    export_symbol_once(
        "_raw_write_lock_irq",
        linux_raw_write_lock_irq as usize,
        false,
    );
    export_symbol_once(
        "_raw_write_lock_irqsave",
        linux_raw_write_lock_irqsave as usize,
        false,
    );
    export_symbol_once("_raw_write_unlock", linux_raw_write_unlock as usize, false);
    export_symbol_once(
        "_raw_write_unlock_irq",
        linux_raw_write_unlock_irq as usize,
        false,
    );
    export_symbol_once(
        "_raw_write_unlock_irqrestore",
        linux_raw_write_unlock_irqrestore as usize,
        false,
    );
}

fn read_lock(lock: &QrwLock) {
    while !lock.try_read_lock() {
        core::hint::spin_loop();
    }
}

fn write_lock(lock: &QrwLock) {
    while !lock.try_write_lock() {
        core::hint::spin_loop();
    }
}

#[unsafe(export_name = "_raw_read_lock")]
pub unsafe extern "C" fn linux_raw_read_lock(lock: *mut QrwLock) {
    if lock.is_null() {
        return;
    }
    preempt_disable();
    read_lock(unsafe { &*lock });
}

#[unsafe(export_name = "_raw_read_lock_irq")]
pub unsafe extern "C" fn linux_raw_read_lock_irq(lock: *mut QrwLock) {
    local_irq_disable();
    if !lock.is_null() {
        read_lock(unsafe { &*lock });
    }
}

#[unsafe(export_name = "_raw_read_lock_irqsave")]
pub unsafe extern "C" fn linux_raw_read_lock_irqsave(lock: *mut QrwLock) -> usize {
    let flags = local_irq_save();
    if !lock.is_null() {
        read_lock(unsafe { &*lock });
    }
    flags as usize
}

#[unsafe(export_name = "_raw_read_unlock")]
pub unsafe extern "C" fn linux_raw_read_unlock(lock: *mut QrwLock) {
    if !lock.is_null() {
        unsafe { &*lock }.read_unlock();
    }
    preempt_enable();
}

#[unsafe(export_name = "_raw_read_unlock_irq")]
pub unsafe extern "C" fn linux_raw_read_unlock_irq(lock: *mut QrwLock) {
    if !lock.is_null() {
        unsafe { &*lock }.read_unlock();
    }
    crate::kernel::locking::local_irq_enable();
}

#[unsafe(export_name = "_raw_read_unlock_irqrestore")]
pub unsafe extern "C" fn linux_raw_read_unlock_irqrestore(lock: *mut QrwLock, flags: usize) {
    if !lock.is_null() {
        unsafe { &*lock }.read_unlock();
    }
    local_irq_restore(flags as u64);
}

#[unsafe(export_name = "_raw_write_lock")]
pub unsafe extern "C" fn linux_raw_write_lock(lock: *mut QrwLock) {
    if lock.is_null() {
        return;
    }
    preempt_disable();
    write_lock(unsafe { &*lock });
}

#[unsafe(export_name = "_raw_write_lock_irq")]
pub unsafe extern "C" fn linux_raw_write_lock_irq(lock: *mut QrwLock) {
    local_irq_disable();
    if !lock.is_null() {
        write_lock(unsafe { &*lock });
    }
}

#[unsafe(export_name = "_raw_write_lock_irqsave")]
pub unsafe extern "C" fn linux_raw_write_lock_irqsave(lock: *mut QrwLock) -> usize {
    let flags = local_irq_save();
    if !lock.is_null() {
        write_lock(unsafe { &*lock });
    }
    flags as usize
}

#[unsafe(export_name = "_raw_write_unlock")]
pub unsafe extern "C" fn linux_raw_write_unlock(lock: *mut QrwLock) {
    if !lock.is_null() {
        unsafe { &*lock }.write_unlock();
    }
    preempt_enable();
}

#[unsafe(export_name = "_raw_write_unlock_irq")]
pub unsafe extern "C" fn linux_raw_write_unlock_irq(lock: *mut QrwLock) {
    if !lock.is_null() {
        unsafe { &*lock }.write_unlock();
    }
    crate::kernel::locking::local_irq_enable();
}

#[unsafe(export_name = "_raw_write_unlock_irqrestore")]
pub unsafe extern "C" fn linux_raw_write_unlock_irqrestore(lock: *mut QrwLock, flags: usize) {
    if !lock.is_null() {
        unsafe { &*lock }.write_unlock();
    }
    local_irq_restore(flags as u64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_side_excludes_writer() {
        let lock = QrwLock::new();
        assert!(lock.try_read_lock());
        assert!(!lock.try_write_lock());
        lock.read_unlock();
        assert!(lock.try_write_lock());
        lock.write_unlock();
        assert!(lock.try_read_lock());
    }
}
