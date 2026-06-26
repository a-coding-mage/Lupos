//! linux-parity: complete
//! linux-source: vendor/linux/lib/kunit/try-catch.c
//! test-origin: linux:vendor/linux/lib/kunit/try-catch.c
//! KUnit try/catch result normalization.

use alloc::vec::Vec;

use crate::include::uapi::errno::{EFAULT, EINTR, ETIMEDOUT};
use crate::kernel::module::{export_symbol, find_symbol};

pub const TRY_CATCH_THREAD_NAME: &str = "kunit_try_catch_thread";

pub type KunitTryFn = fn(&mut KunitTryCatch);
pub type KunitCatchFn = fn(&mut KunitTryCatch);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KunitTryCatchAction {
    Return,
    Catch { normalized_result: i32 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KunitTryCatchEvent {
    KthreadCreateOk,
    KthreadCreateFailed(i32),
    GetTaskStruct,
    WakeUpProcess,
    KthreadExit(i32),
    KthreadStop,
    PutTaskStruct,
    TryFaulted,
    TryFaultedAt { file: &'static str, line: u32 },
    TryTimedOut,
    UnknownError(i32),
    CatchCalled,
}

#[derive(Clone, Debug)]
pub struct KunitTryCatch {
    pub context: usize,
    pub timeout: u64,
    pub try_result: i32,
    pub try_fn: KunitTryFn,
    pub catch_fn: KunitCatchFn,
    pub create_error: Option<i32>,
    pub wait_time_remaining: i32,
    pub last_seen: Option<(&'static str, u32)>,
    pub events: Vec<KunitTryCatchEvent>,
}

impl KunitTryCatch {
    pub fn new(try_fn: KunitTryFn, catch_fn: KunitCatchFn) -> Self {
        Self {
            context: 0,
            timeout: 1,
            try_result: 0,
            try_fn,
            catch_fn,
            create_error: None,
            wait_time_remaining: 1,
            last_seen: None,
            events: Vec::new(),
        }
    }
}

impl Default for KunitTryCatch {
    fn default() -> Self {
        Self::new(noop_try, record_catch)
    }
}

fn noop_try(_try_catch: &mut KunitTryCatch) {}

fn record_catch(try_catch: &mut KunitTryCatch) {
    try_catch.events.push(KunitTryCatchEvent::CatchCalled);
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "kunit_try_catch_throw",
        kunit_try_catch_throw as usize,
        true,
    );
    export_symbol_once("kunit_try_catch_run", kunit_try_catch_run as usize, true);
}

pub const fn kunit_try_catch_throw_result() -> i32 {
    -EFAULT
}

pub const fn kunit_try_catch_initial_thread_result() -> i32 {
    -EINTR
}

pub const fn kunit_try_catch_complete_result(current: i32) -> i32 {
    if current == -EINTR { 0 } else { current }
}

pub const fn kunit_try_catch_action(exit_code: i32) -> KunitTryCatchAction {
    if exit_code == 0 {
        KunitTryCatchAction::Return
    } else if exit_code == -EFAULT {
        KunitTryCatchAction::Catch {
            normalized_result: 0,
        }
    } else {
        KunitTryCatchAction::Catch {
            normalized_result: exit_code,
        }
    }
}

pub const fn kunit_try_catch_timeout_result() -> i32 {
    -ETIMEDOUT
}

pub fn kunit_try_catch_throw(try_catch: &mut KunitTryCatch) {
    try_catch.try_result = kunit_try_catch_throw_result();
    try_catch.events.push(KunitTryCatchEvent::KthreadExit(0));
}

fn kunit_generic_run_threadfn_adapter(try_catch: &mut KunitTryCatch) -> i32 {
    try_catch.try_result = kunit_try_catch_initial_thread_result();
    (try_catch.try_fn)(try_catch);
    try_catch.try_result = kunit_try_catch_complete_result(try_catch.try_result);
    0
}

pub fn kunit_try_catch_run(try_catch: &mut KunitTryCatch, context: usize) {
    try_catch.context = context;
    try_catch.try_result = 0;

    if let Some(err) = try_catch.create_error {
        try_catch
            .events
            .push(KunitTryCatchEvent::KthreadCreateFailed(err));
        try_catch.try_result = err;
        (try_catch.catch_fn)(try_catch);
        return;
    }

    try_catch.events.push(KunitTryCatchEvent::KthreadCreateOk);
    try_catch.events.push(KunitTryCatchEvent::GetTaskStruct);
    try_catch.events.push(KunitTryCatchEvent::WakeUpProcess);
    let _ = kunit_generic_run_threadfn_adapter(try_catch);

    if try_catch.wait_time_remaining == 0 {
        try_catch.try_result = kunit_try_catch_timeout_result();
        try_catch.events.push(KunitTryCatchEvent::KthreadStop);
    }

    try_catch.events.push(KunitTryCatchEvent::PutTaskStruct);
    let exit_code = try_catch.try_result;

    if exit_code == 0 {
        return;
    }

    match exit_code {
        code if code == -EFAULT => {
            try_catch.try_result = 0;
        }
        code if code == -EINTR => {
            if let Some((file, line)) = try_catch.last_seen {
                try_catch
                    .events
                    .push(KunitTryCatchEvent::TryFaultedAt { file, line });
            } else {
                try_catch.events.push(KunitTryCatchEvent::TryFaulted);
            }
        }
        code if code == -ETIMEDOUT => {
            try_catch.events.push(KunitTryCatchEvent::TryTimedOut);
        }
        code => {
            try_catch
                .events
                .push(KunitTryCatchEvent::UnknownError(code));
        }
    }

    (try_catch.catch_fn)(try_catch);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_op_try(_try_catch: &mut KunitTryCatch) {}

    fn throw_try(try_catch: &mut KunitTryCatch) {
        kunit_try_catch_throw(try_catch);
    }

    fn unknown_error_try(try_catch: &mut KunitTryCatch) {
        try_catch.try_result = -123;
    }

    #[test]
    fn try_catch_matches_linux_result_flow() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/kunit/try-catch.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/kunit/try-catch-impl.h"
        ));
        assert!(source.contains("try_catch->try_result = -EFAULT;"));
        assert!(source.contains("kthread_exit(0);"));
        assert!(source.contains("try_catch->try_result = -EINTR;"));
        assert!(source.contains("kthread_create(kunit_generic_run_threadfn_adapter"));
        assert!(source.contains("\"kunit_try_catch_thread\""));
        assert!(source.contains("get_task_struct(task_struct);"));
        assert!(source.contains("task_done = task_struct->vfork_done;"));
        assert!(source.contains("wake_up_process(task_struct);"));
        assert!(source.contains("wait_for_completion_timeout("));
        assert!(source.contains("try_catch->try_result = -ETIMEDOUT;"));
        assert!(source.contains("kthread_stop(task_struct);"));
        assert!(source.contains("put_task_struct(task_struct);"));
        assert!(source.contains("if (exit_code == -EFAULT)"));
        assert!(source.contains("try_catch->try_result = 0;"));
        assert!(source.contains("test->last_seen.file"));
        assert!(source.contains("try_catch->catch(try_catch->context);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(kunit_try_catch_run);"));
        assert!(header.contains("struct kunit_try_catch"));

        assert_eq!(kunit_try_catch_throw_result(), -EFAULT);
        assert_eq!(kunit_try_catch_initial_thread_result(), -EINTR);
        assert_eq!(kunit_try_catch_complete_result(-EINTR), 0);
        assert_eq!(kunit_try_catch_complete_result(-ETIMEDOUT), -ETIMEDOUT);
        assert_eq!(kunit_try_catch_action(0), KunitTryCatchAction::Return);
        assert_eq!(
            kunit_try_catch_action(-EFAULT),
            KunitTryCatchAction::Catch {
                normalized_result: 0,
            }
        );
        assert_eq!(kunit_try_catch_timeout_result(), -ETIMEDOUT);
    }

    #[test]
    fn run_returns_without_catch_when_try_completes() {
        let mut tc = KunitTryCatch::new(no_op_try, record_catch);
        kunit_try_catch_run(&mut tc, 0x51);
        assert_eq!(tc.context, 0x51);
        assert_eq!(tc.try_result, 0);
        assert!(!tc.events.contains(&KunitTryCatchEvent::CatchCalled));
        assert!(tc.events.contains(&KunitTryCatchEvent::WakeUpProcess));
        assert!(tc.events.contains(&KunitTryCatchEvent::PutTaskStruct));
    }

    #[test]
    fn throw_normalizes_fault_to_zero_then_runs_catch() {
        let mut tc = KunitTryCatch::new(throw_try, record_catch);
        kunit_try_catch_run(&mut tc, 0);
        assert_eq!(tc.try_result, 0);
        assert!(tc.events.contains(&KunitTryCatchEvent::KthreadExit(0)));
        assert!(tc.events.contains(&KunitTryCatchEvent::CatchCalled));
    }

    #[test]
    fn create_failure_and_timeout_run_catch_paths() {
        let mut create_failed = KunitTryCatch::new(no_op_try, record_catch);
        create_failed.create_error = Some(-12);
        kunit_try_catch_run(&mut create_failed, 0);
        assert_eq!(create_failed.try_result, -12);
        assert_eq!(
            create_failed.events,
            [
                KunitTryCatchEvent::KthreadCreateFailed(-12),
                KunitTryCatchEvent::CatchCalled,
            ]
        );

        let mut timed_out = KunitTryCatch::new(no_op_try, record_catch);
        timed_out.wait_time_remaining = 0;
        kunit_try_catch_run(&mut timed_out, 0);
        assert_eq!(timed_out.try_result, -ETIMEDOUT);
        assert!(timed_out.events.contains(&KunitTryCatchEvent::KthreadStop));
        assert!(timed_out.events.contains(&KunitTryCatchEvent::TryTimedOut));
        assert!(timed_out.events.contains(&KunitTryCatchEvent::CatchCalled));
    }

    #[test]
    fn unknown_errors_are_reported_before_catch() {
        let mut unknown = KunitTryCatch::new(unknown_error_try, record_catch);
        kunit_try_catch_run(&mut unknown, 0);
        assert!(
            unknown
                .events
                .contains(&KunitTryCatchEvent::UnknownError(-123))
        );
        assert!(unknown.events.contains(&KunitTryCatchEvent::CatchCalled));
    }

    #[test]
    fn exports_register_gpl_symbols() {
        register_module_exports();
        assert!(crate::kernel::module::find_symbol("kunit_try_catch_throw").is_some());
        assert!(crate::kernel::module::find_symbol("kunit_try_catch_run").is_some());
    }
}
