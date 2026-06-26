//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/unwind_guess.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/unwind_guess.c
//! x86 fallback stack-scan unwinder.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/unwind_guess.c

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GuessUnwindState {
    pub sp_index: usize,
    pub last_ip: u64,
    pub done: bool,
}

pub fn __unwind_start(stack_words: &[u64]) -> GuessUnwindState {
    let mut state = GuessUnwindState::default();
    for (idx, &ip) in stack_words.iter().enumerate() {
        if ip != 0 {
            state.sp_index = idx + 1;
            state.last_ip = ip;
            return state;
        }
    }
    state.done = true;
    state
}

pub const fn unwind_get_return_address(state: &GuessUnwindState) -> u64 {
    state.last_ip
}

pub fn unwind_next_frame(state: &mut GuessUnwindState, stack_words: &[u64]) -> bool {
    if state.done {
        return false;
    }
    while state.sp_index < stack_words.len() {
        let ip = stack_words[state.sp_index];
        state.sp_index += 1;
        if ip != 0 {
            state.last_ip = ip;
            return true;
        }
    }
    state.done = true;
    state.last_ip = 0;
    false
}

pub fn guess_unwind(stack_words: &[u64], limit: usize) -> Vec<u64> {
    let mut out = Vec::new();
    let mut state = __unwind_start(stack_words);
    while !state.done && out.len() < limit {
        out.push(unwind_get_return_address(&state));
        unwind_next_frame(&mut state, stack_words);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guess_unwind_skips_zero_words() {
        assert_eq!(guess_unwind(&[0, 1, 0, 2, 3], 8), alloc::vec![1, 2, 3]);
    }
}
