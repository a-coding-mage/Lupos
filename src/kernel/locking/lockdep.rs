//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking/lockdep.c
//! test-origin: linux:vendor/linux/kernel/locking/lockdep.c
//! Lock dependency validator coverage for M33.
//!
//! Mirrors `vendor/linux/kernel/locking/lockdep.c`.  This is a bounded
//! lock-class graph that records observed acquire order and rejects new edges
//! that would introduce an ABBA cycle.

use spin::Mutex;

use crate::include::uapi::errno::{EDEADLK, EINVAL};

pub const MAX_LOCKDEP_KEYS: usize = 32;
pub const MAX_LOCK_DEPTH: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct LockClassKey {
    pub id: u16,
}

impl LockClassKey {
    pub const fn new(id: u16) -> Self {
        Self { id }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct LockdepMap {
    pub key: LockClassKey,
    pub name: &'static str,
}

impl LockdepMap {
    pub const fn new(name: &'static str, id: u16) -> Self {
        Self {
            key: LockClassKey::new(id),
            name,
        }
    }
}

struct LockdepState {
    held: [u16; MAX_LOCK_DEPTH],
    depth: usize,
    edges: [u32; MAX_LOCKDEP_KEYS],
}

impl LockdepState {
    const fn new() -> Self {
        Self {
            held: [0; MAX_LOCK_DEPTH],
            depth: 0,
            edges: [0; MAX_LOCKDEP_KEYS],
        }
    }
}

static LOCKDEP: Mutex<LockdepState> = Mutex::new(LockdepState::new());

pub fn lockdep_init_map(map: &mut LockdepMap, name: &'static str, key: LockClassKey) {
    *map = LockdepMap { key, name };
}

pub fn lockdep_acquire(map: &LockdepMap) -> Result<(), i32> {
    let class = map.key.id as usize;
    if class >= MAX_LOCKDEP_KEYS {
        return Err(EINVAL);
    }

    let mut state = LOCKDEP.lock();
    if state.depth >= MAX_LOCK_DEPTH {
        return Err(EINVAL);
    }

    let depth = state.depth;
    for idx in 0..depth {
        let held = state.held[idx] as usize;
        if reaches(&state, class, held) {
            return Err(EDEADLK);
        }
        state.edges[held] |= 1u32 << class;
    }

    state.held[depth] = class as u16;
    state.depth += 1;
    Ok(())
}

pub fn lockdep_release(map: &LockdepMap) -> Result<(), i32> {
    let class = map.key.id;
    let mut state = LOCKDEP.lock();
    if let Some(pos) = state.held[..state.depth]
        .iter()
        .rposition(|held| *held == class)
    {
        for idx in pos..state.depth - 1 {
            state.held[idx] = state.held[idx + 1];
        }
        state.depth -= 1;
        Ok(())
    } else {
        Err(EINVAL)
    }
}

pub fn held_lock_count() -> usize {
    LOCKDEP.lock().depth
}

#[doc(hidden)]
pub fn reset_for_tests() {
    *LOCKDEP.lock() = LockdepState::new();
}

fn reaches(state: &LockdepState, from: usize, to: usize) -> bool {
    let mut seen = 0u32;
    let mut stack = 1u32 << from;
    while stack != 0 {
        let node = stack.trailing_zeros() as usize;
        stack &= !(1u32 << node);
        if node == to {
            return true;
        }
        if seen & (1u32 << node) != 0 {
            continue;
        }
        seen |= 1u32 << node;
        stack |= state.edges[node] & !seen;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_and_release_tracks_depth() {
        reset_for_tests();
        let a = LockdepMap::new("a", 1);
        lockdep_acquire(&a).unwrap();
        assert_eq!(held_lock_count(), 1);
        lockdep_release(&a).unwrap();
        assert_eq!(held_lock_count(), 0);
    }

    #[test]
    fn rejects_abba_cycle() {
        reset_for_tests();
        let a = LockdepMap::new("a", 1);
        let b = LockdepMap::new("b", 2);
        lockdep_acquire(&a).unwrap();
        lockdep_acquire(&b).unwrap();
        lockdep_release(&b).unwrap();
        lockdep_release(&a).unwrap();

        lockdep_acquire(&b).unwrap();
        assert_eq!(lockdep_acquire(&a), Err(EDEADLK));
    }
}
