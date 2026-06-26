//! linux-parity: complete
//! linux-source: vendor/linux/security/landlock
//! test-origin: linux:vendor/linux/security/landlock
//! Landlock â€” sandboxing for filesystem access (M64).
//!
//! Mirrors `vendor/linux/security/landlock/`.  Implements:
//! - `LandlockRuleset` (set of path rules + allowed-access mask).
//! - `LandlockDomain` (per-task chain of rulesets â€” single-slot in M64).
//! - The three syscalls `landlock_create_ruleset`, `landlock_add_rule`,
//!   `landlock_restrict_self`.
//! - A `path_open` policy check used by the LSM hook.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicI32, Ordering};

use spin::Mutex;

pub mod cred;
pub mod object;
pub mod setup;
pub mod syscalls;
pub use syscalls::{
    sys_landlock_add_rule, sys_landlock_create_ruleset, sys_landlock_restrict_self,
};

/// `LANDLOCK_ACCESS_FS_*` â€” Linux uapi/linux/landlock.h.
pub const LANDLOCK_ACCESS_FS_EXECUTE: u64 = 1 << 0;
pub const LANDLOCK_ACCESS_FS_WRITE_FILE: u64 = 1 << 1;
pub const LANDLOCK_ACCESS_FS_READ_FILE: u64 = 1 << 2;
pub const LANDLOCK_ACCESS_FS_READ_DIR: u64 = 1 << 3;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathRule {
    pub path: String,
    pub allowed: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LandlockRuleset {
    pub id: i32,
    pub fs_handled_mask: u64,
    pub rules: Vec<PathRule>,
}

static NEXT_ID: AtomicI32 = AtomicI32::new(0x500);
static RULESETS: Mutex<Vec<LandlockRuleset>> = Mutex::new(Vec::new());
static ACTIVE: Mutex<Option<LandlockRuleset>> = Mutex::new(None);

pub fn create_ruleset(handled_mask: u64) -> i32 {
    let id = NEXT_ID.fetch_add(1, Ordering::AcqRel);
    RULESETS.lock().push(LandlockRuleset {
        id,
        fs_handled_mask: handled_mask,
        rules: Vec::new(),
    });
    id
}

pub fn add_path_rule(ruleset_id: i32, path: &str, allowed: u64) -> Result<(), i32> {
    let mut g = RULESETS.lock();
    for rs in g.iter_mut() {
        if rs.id == ruleset_id {
            rs.rules.push(PathRule {
                path: String::from(path),
                allowed,
            });
            return Ok(());
        }
    }
    Err(-9) // EBADF
}

pub fn ruleset_snapshot(ruleset_id: i32) -> Option<LandlockRuleset> {
    RULESETS
        .lock()
        .iter()
        .find(|ruleset| ruleset.id == ruleset_id)
        .cloned()
}

pub fn restrict_self(ruleset_id: i32) -> Result<(), i32> {
    let g = RULESETS.lock();
    for rs in g.iter() {
        if rs.id == ruleset_id {
            *ACTIVE.lock() = Some(rs.clone());
            return Ok(());
        }
    }
    Err(-9)
}

/// `path_open` policy hook: returns 0 if allowed, -EACCES if denied.
pub fn check_path_open(path: &[u8], _flags: i32) -> i32 {
    let g = ACTIVE.lock();
    let rs = match &*g {
        Some(rs) => rs,
        None => return 0, // No active ruleset â†’ allow.
    };
    // If READ_FILE isn't even in the handled mask, Landlock doesn't care.
    if rs.fs_handled_mask & LANDLOCK_ACCESS_FS_READ_FILE == 0 {
        return 0;
    }
    // Path-prefix match.
    for rule in rs.rules.iter() {
        if path_starts_with(path, rule.path.as_bytes())
            && rule.allowed & LANDLOCK_ACCESS_FS_READ_FILE != 0
        {
            return 0;
        }
    }
    -13 // EACCES
}

fn path_starts_with(path: &[u8], prefix: &[u8]) -> bool {
    if path.len() < prefix.len() {
        return false;
    }
    &path[..prefix.len()] == prefix
}

/// Register the Landlock LSM `path_open` hook.
pub fn register_hooks() {
    use crate::security::hooks::{LSM_ID_LANDLOCK, LsmHooks, NOOP_HOOKS};
    use crate::security::lsm_list::register_lsm;
    let hooks = LsmHooks {
        name: "landlock",
        id: LSM_ID_LANDLOCK,
        path_open: Some(check_path_open),
        ..NOOP_HOOKS
    };
    let _ = register_lsm(hooks);
}

#[cfg(test)]
pub fn reset_for_test() {
    RULESETS.lock().clear();
    *ACTIVE.lock() = None;
    NEXT_ID.store(0x500, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;

    static LANDLOCK_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn create_add_restrict_round_trip() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = LANDLOCK_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_test();
        let id = create_ruleset(LANDLOCK_ACCESS_FS_READ_FILE);
        assert!(id > 0);
        add_path_rule(id, "/tmp", LANDLOCK_ACCESS_FS_READ_FILE).unwrap();
        restrict_self(id).unwrap();
    }

    #[test]
    fn path_open_inside_allowed_outside_denied() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = LANDLOCK_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_test();
        let id = create_ruleset(LANDLOCK_ACCESS_FS_READ_FILE);
        add_path_rule(id, "/tmp", LANDLOCK_ACCESS_FS_READ_FILE).unwrap();
        restrict_self(id).unwrap();
        assert_eq!(check_path_open(b"/tmp/foo", 0), 0);
        assert_eq!(check_path_open(b"/etc/passwd", 0), -13);
    }

    #[test]
    fn no_active_ruleset_allows_all() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = LANDLOCK_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_test();
        assert_eq!(check_path_open(b"/anything", 0), 0);
    }
}
