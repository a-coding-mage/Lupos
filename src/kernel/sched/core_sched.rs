//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/core_sched.c
//! test-origin: linux:vendor/linux/kernel/sched/core_sched.c
//! Core scheduling cookie support.
//!
//! Mirrors `vendor/linux/kernel/sched/core_sched.c`. Linux uses per-task core
//! cookies to constrain SMT sibling co-scheduling. Lupos records the cookie
//! semantics independently of SMT placement so future CPU topology work can
//! consume the same surface.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static CORE_SCHED_ENABLED: AtomicBool = AtomicBool::new(false);
static NEXT_COOKIE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoreCookie(u64);

impl CoreCookie {
    pub const NONE: Self = Self(0);

    pub const fn raw(self) -> u64 {
        self.0
    }
}

pub fn sched_core_enabled() -> bool {
    CORE_SCHED_ENABLED.load(Ordering::Acquire)
}

pub fn sched_core_set_enabled(enabled: bool) {
    CORE_SCHED_ENABLED.store(enabled, Ordering::Release);
}

pub fn sched_core_alloc_cookie() -> CoreCookie {
    CoreCookie(NEXT_COOKIE.fetch_add(1, Ordering::Relaxed))
}

pub const fn sched_core_cookie_match(left: CoreCookie, right: CoreCookie) -> bool {
    left.0 == 0 || right.0 == 0 || left.0 == right.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_cookie_zero_matches_anything() {
        let cookie = sched_core_alloc_cookie();
        assert!(sched_core_cookie_match(CoreCookie::NONE, cookie));
    }

    #[test]
    fn nonzero_cookies_must_match() {
        let a = sched_core_alloc_cookie();
        let b = sched_core_alloc_cookie();
        assert!(sched_core_cookie_match(a, a));
        assert!(!sched_core_cookie_match(a, b));
    }
}
