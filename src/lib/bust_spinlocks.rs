//! linux-parity: complete
//! linux-source: vendor/linux/lib/bust_spinlocks.c
//! test-origin: linux:vendor/linux/lib/bust_spinlocks.c
//! Minimal spinlock busting state transitions for oops paths.

use core::sync::atomic::{AtomicI32, Ordering};

static OOPS_IN_PROGRESS: AtomicI32 = AtomicI32::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BustSpinlocksResult {
    pub oops_in_progress: i32,
    pub wake_klogd: bool,
}

pub const fn bust_spinlocks_transition(yes: bool, current: i32) -> BustSpinlocksResult {
    if yes {
        BustSpinlocksResult {
            oops_in_progress: current + 1,
            wake_klogd: false,
        }
    } else {
        let next = current - 1;
        BustSpinlocksResult {
            oops_in_progress: next,
            wake_klogd: next == 0,
        }
    }
}

pub fn bust_spinlocks(yes: i32) -> BustSpinlocksResult {
    if yes != 0 {
        let next = OOPS_IN_PROGRESS.fetch_add(1, Ordering::AcqRel) + 1;
        BustSpinlocksResult {
            oops_in_progress: next,
            wake_klogd: false,
        }
    } else {
        let next = OOPS_IN_PROGRESS.fetch_sub(1, Ordering::AcqRel) - 1;
        BustSpinlocksResult {
            oops_in_progress: next,
            wake_klogd: next == 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bust_spinlocks_source_and_transitions_match_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/bust_spinlocks.c"
        ));
        assert!(source.contains("++oops_in_progress;"));
        assert!(source.contains("console_unblank();"));
        assert!(source.contains("if (--oops_in_progress == 0)"));
        assert!(source.contains("wake_up_klogd();"));
        assert_eq!(
            bust_spinlocks_transition(true, 0),
            BustSpinlocksResult {
                oops_in_progress: 1,
                wake_klogd: false
            }
        );
        assert_eq!(
            bust_spinlocks_transition(false, 1),
            BustSpinlocksResult {
                oops_in_progress: 0,
                wake_klogd: true
            }
        );
    }
}
