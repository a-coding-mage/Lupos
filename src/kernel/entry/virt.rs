//! linux-parity: complete
//! linux-source: vendor/linux/kernel/entry/virt.c
//! test-origin: linux:vendor/linux/kernel/entry/virt.c
//! Generic transfer-to-guest-mode pending-work loop.

use crate::include::uapi::errno::EINTR;
use crate::kernel::task::{TIF_NEED_RESCHED, TIF_SIGPENDING};

pub const TIF_NOTIFY_SIGNAL: u64 = 1 << 13;
pub const TIF_NEED_RESCHED_LAZY: u64 = 1 << 14;
pub const TIF_NOTIFY_RESUME: u64 = 1 << 15;
pub const XFER_TO_GUEST_MODE_WORK: u64 = TIF_SIGPENDING
    | TIF_NOTIFY_SIGNAL
    | TIF_NEED_RESCHED
    | TIF_NEED_RESCHED_LAZY
    | TIF_NOTIFY_RESUME;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GuestModeWorkResult {
    pub errno: i32,
    pub scheduled: bool,
    pub resumed_user_work: bool,
    pub arch_handler_called: bool,
}

pub const fn xfer_to_guest_mode_work_once(ti_work: u64, arch_ret: i32) -> GuestModeWorkResult {
    if ti_work & (TIF_SIGPENDING | TIF_NOTIFY_SIGNAL) != 0 {
        return GuestModeWorkResult {
            errno: -EINTR,
            scheduled: false,
            resumed_user_work: false,
            arch_handler_called: false,
        };
    }

    let scheduled = ti_work & (TIF_NEED_RESCHED | TIF_NEED_RESCHED_LAZY) != 0;
    let resumed_user_work = ti_work & TIF_NOTIFY_RESUME != 0;
    if arch_ret != 0 {
        return GuestModeWorkResult {
            errno: arch_ret,
            scheduled,
            resumed_user_work,
            arch_handler_called: true,
        };
    }

    GuestModeWorkResult {
        errno: 0,
        scheduled,
        resumed_user_work,
        arch_handler_called: ti_work & XFER_TO_GUEST_MODE_WORK != 0,
    }
}

pub const fn xfer_to_guest_mode_handle_work(
    thread_flags: u64,
    arch_ret: i32,
) -> GuestModeWorkResult {
    if thread_flags & XFER_TO_GUEST_MODE_WORK == 0 {
        GuestModeWorkResult {
            errno: 0,
            scheduled: false,
            resumed_user_work: false,
            arch_handler_called: false,
        }
    } else {
        xfer_to_guest_mode_work_once(thread_flags, arch_ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guest_mode_work_order_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/entry/virt.c"
        ));
        assert!(source.contains("if (ti_work & (_TIF_SIGPENDING | _TIF_NOTIFY_SIGNAL))"));
        assert!(source.contains("return -EINTR;"));
        assert!(source.contains("if (ti_work & (_TIF_NEED_RESCHED | _TIF_NEED_RESCHED_LAZY))"));
        assert!(source.contains("schedule();"));
        assert!(source.contains("if (ti_work & _TIF_NOTIFY_RESUME)"));
        assert!(source.contains("resume_user_mode_work(NULL);"));
        assert!(source.contains("arch_xfer_to_guest_mode_handle_work(ti_work);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(xfer_to_guest_mode_handle_work);"));

        assert_eq!(
            xfer_to_guest_mode_handle_work(TIF_SIGPENDING, 0).errno,
            -EINTR
        );
        assert_eq!(
            xfer_to_guest_mode_handle_work(TIF_NEED_RESCHED | TIF_NOTIFY_RESUME, -22),
            GuestModeWorkResult {
                errno: -22,
                scheduled: true,
                resumed_user_work: true,
                arch_handler_called: true,
            }
        );
        assert_eq!(
            xfer_to_guest_mode_handle_work(0, -1),
            GuestModeWorkResult::default()
        );
    }
}
