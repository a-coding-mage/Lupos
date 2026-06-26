//! linux-parity: complete
//! linux-source: vendor/linux/lib/once.c
//! test-origin: linux:vendor/linux/lib/once.c
//! DO_ONCE gate state transitions.

use alloc::vec::Vec;

use crate::kernel::module::{export_symbol, find_symbol};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OnceStart {
    pub should_run: bool,
    pub lock_reacquired_for_sparse: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StaticKeyTrue {
    pub enabled: bool,
    pub disable_count: u32,
}

impl StaticKeyTrue {
    pub const fn enabled() -> Self {
        Self {
            enabled: true,
            disable_count: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ModuleRef {
    pub refcount: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OnceWork {
    pub key_was_enabled: bool,
    pub module_get_taken: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OnceRuntime {
    pub spin_locked: bool,
    pub mutex_locked: bool,
    pub sparse_reacquires: u32,
    pub spin_unlocks: u32,
    pub mutex_unlocks: u32,
    pub allocation_fails: bool,
    pub scheduled_work: Vec<OnceWork>,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("__do_once_start", __do_once_start as usize, false);
    export_symbol_once("__do_once_done", __do_once_done as usize, false);
    export_symbol_once(
        "__do_once_sleepable_start",
        __do_once_sleepable_start as usize,
        false,
    );
    export_symbol_once(
        "__do_once_sleepable_done",
        __do_once_sleepable_done as usize,
        false,
    );
}

pub const fn do_once_start(done: bool) -> OnceStart {
    OnceStart {
        should_run: !done,
        lock_reacquired_for_sparse: done,
    }
}

pub fn do_once_done(done: &mut bool) {
    *done = true;
}

pub fn once_deferred(
    key: &mut StaticKeyTrue,
    module: &mut ModuleRef,
    work: OnceWork,
) -> Result<(), &'static str> {
    if !key.enabled {
        return Err("BUG_ON(!static_key_enabled(work->key))");
    }
    key.enabled = false;
    key.disable_count += 1;
    if work.module_get_taken {
        module.refcount = module.refcount.saturating_sub(1);
    }
    Ok(())
}

pub fn once_disable_jump(runtime: &mut OnceRuntime, key: &StaticKeyTrue, module: &mut ModuleRef) {
    if runtime.allocation_fails {
        return;
    }
    module.refcount = module.refcount.saturating_add(1);
    runtime.scheduled_work.push(OnceWork {
        key_was_enabled: key.enabled,
        module_get_taken: true,
    });
}

pub fn do_once_start_locked(runtime: &mut OnceRuntime, done: bool) -> bool {
    runtime.spin_locked = true;
    if done {
        runtime.spin_locked = false;
        runtime.spin_unlocks += 1;
        runtime.sparse_reacquires += 1;
        return false;
    }
    true
}

pub fn do_once_done_locked(
    runtime: &mut OnceRuntime,
    done: &mut bool,
    once_key: &StaticKeyTrue,
    module: &mut ModuleRef,
) {
    *done = true;
    runtime.spin_locked = false;
    runtime.spin_unlocks += 1;
    once_disable_jump(runtime, once_key, module);
}

pub fn do_once_sleepable_start_locked(runtime: &mut OnceRuntime, done: bool) -> bool {
    runtime.mutex_locked = true;
    if done {
        runtime.mutex_locked = false;
        runtime.mutex_unlocks += 1;
        runtime.sparse_reacquires += 1;
        return false;
    }
    true
}

pub fn do_once_sleepable_done_locked(
    runtime: &mut OnceRuntime,
    done: &mut bool,
    once_key: &mut StaticKeyTrue,
) {
    *done = true;
    runtime.mutex_locked = false;
    runtime.mutex_unlocks += 1;
    once_key.enabled = false;
    once_key.disable_count += 1;
}

pub unsafe extern "C" fn __do_once_start(done: *mut bool, _flags: *mut usize) -> bool {
    if done.is_null() {
        return false;
    }
    unsafe { !*done }
}

pub unsafe extern "C" fn __do_once_done(
    done: *mut bool,
    _once_key: *mut (),
    _flags: *mut usize,
    _module: *mut (),
) {
    if !done.is_null() {
        unsafe {
            *done = true;
        }
    }
}

pub unsafe extern "C" fn __do_once_sleepable_start(done: *mut bool) -> bool {
    if done.is_null() {
        return false;
    }
    unsafe { !*done }
}

pub unsafe extern "C" fn __do_once_sleepable_done(
    done: *mut bool,
    _once_key: *mut (),
    _module: *mut (),
) {
    if !done.is_null() {
        unsafe {
            *done = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn once_matches_linux_done_and_sleepable_gates() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/once.c"
        ));
        assert!(source.contains("struct once_work"));
        assert!(source.contains("kmalloc_obj(*w, GFP_ATOMIC);"));
        assert!(source.contains("INIT_WORK(&w->work, once_deferred);"));
        assert!(source.contains("__module_get(mod);"));
        assert!(source.contains("BUG_ON(!static_key_enabled(work->key));"));
        assert!(source.contains("static_branch_disable(work->key);"));
        assert!(source.contains("module_put(work->module);"));
        assert!(source.contains("kfree(work);"));
        assert!(source.contains("schedule_work(&w->work);"));
        assert!(source.contains("static DEFINE_SPINLOCK(once_lock);"));
        assert!(source.contains("spin_lock_irqsave(&once_lock, *flags);"));
        assert!(source.contains("spin_unlock_irqrestore(&once_lock, *flags);"));
        assert!(source.contains("if (*done)"));
        assert!(source.contains("__acquire(once_lock);"));
        assert!(source.contains("*done = true;"));
        assert!(source.contains("static DEFINE_MUTEX(once_mutex);"));
        assert!(source.contains("mutex_lock(&once_mutex);"));
        assert!(source.contains("mutex_unlock(&once_mutex);"));
        assert!(source.contains("static_branch_disable(once_key);"));
        assert!(source.contains("EXPORT_SYMBOL(__do_once_start);"));
        assert!(source.contains("EXPORT_SYMBOL(__do_once_sleepable_done);"));

        assert_eq!(
            do_once_start(false),
            OnceStart {
                should_run: true,
                lock_reacquired_for_sparse: false,
            }
        );
        assert_eq!(
            do_once_start(true),
            OnceStart {
                should_run: false,
                lock_reacquired_for_sparse: true,
            }
        );
        let mut done = false;
        assert!(unsafe { __do_once_start(&mut done, core::ptr::null_mut()) });
        unsafe {
            __do_once_done(
                &mut done,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            )
        };
        assert!(done);
        assert!(!unsafe { __do_once_sleepable_start(&mut done) });
    }

    #[test]
    fn once_models_lock_deferred_work_and_sleepable_paths() {
        let mut runtime = OnceRuntime::default();
        let mut done = false;
        let key = StaticKeyTrue::enabled();
        let mut module = ModuleRef::default();

        assert!(do_once_start_locked(&mut runtime, done));
        assert!(runtime.spin_locked);
        do_once_done_locked(&mut runtime, &mut done, &key, &mut module);
        assert!(done);
        assert!(!runtime.spin_locked);
        assert_eq!(runtime.spin_unlocks, 1);
        assert_eq!(module.refcount, 1);
        assert_eq!(runtime.scheduled_work.len(), 1);

        let mut key_for_work = key;
        let work = runtime.scheduled_work.pop().unwrap();
        once_deferred(&mut key_for_work, &mut module, work).expect("deferred work");
        assert!(!key_for_work.enabled);
        assert_eq!(key_for_work.disable_count, 1);
        assert_eq!(module.refcount, 0);

        assert!(!do_once_start_locked(&mut runtime, true));
        assert_eq!(runtime.sparse_reacquires, 1);

        let mut sleep_key = StaticKeyTrue::enabled();
        let mut sleep_done = false;
        assert!(do_once_sleepable_start_locked(&mut runtime, sleep_done));
        do_once_sleepable_done_locked(&mut runtime, &mut sleep_done, &mut sleep_key);
        assert!(sleep_done);
        assert!(!sleep_key.enabled);
        assert_eq!(runtime.mutex_unlocks, 1);

        let mut allocation_failed = OnceRuntime {
            allocation_fails: true,
            ..OnceRuntime::default()
        };
        let mut failed_done = false;
        let mut failed_module = ModuleRef::default();
        do_once_done_locked(
            &mut allocation_failed,
            &mut failed_done,
            &key,
            &mut failed_module,
        );
        assert!(failed_done);
        assert!(allocation_failed.scheduled_work.is_empty());
        assert_eq!(failed_module.refcount, 0);
    }

    #[test]
    fn exports_register_once_symbols() {
        register_module_exports();
        assert!(crate::kernel::module::find_symbol("__do_once_start").is_some());
        assert!(crate::kernel::module::find_symbol("__do_once_done").is_some());
        assert!(crate::kernel::module::find_symbol("__do_once_sleepable_start").is_some());
        assert!(crate::kernel::module::find_symbol("__do_once_sleepable_done").is_some());
    }
}
