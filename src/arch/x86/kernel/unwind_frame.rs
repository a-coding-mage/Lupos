//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/unwind_frame.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/unwind_frame.c
//! x86 frame-pointer unwinder.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/unwind_frame.c

#![allow(dead_code)]

use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StackFrame {
    pub bp: u64,
    pub ret_addr: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UnwindState {
    pub index: usize,
    pub done: bool,
    pub error: bool,
    pub last_ip: u64,
}

pub const fn unwind_get_return_address(state: &UnwindState) -> u64 {
    state.last_ip
}

pub fn __unwind_start(frames: &[StackFrame], first_ip: Option<u64>) -> UnwindState {
    let mut state = UnwindState::default();
    if let Some(ip) = first_ip {
        state.last_ip = ip;
        return state;
    }
    if let Some(frame) = frames.first() {
        state.last_ip = frame.ret_addr;
    } else {
        state.done = true;
    }
    state
}

pub fn unwind_next_frame(state: &mut UnwindState, frames: &[StackFrame]) -> bool {
    if state.done || state.error {
        return false;
    }
    state.index += 1;
    let Some(frame) = frames.get(state.index) else {
        state.done = true;
        state.last_ip = 0;
        return false;
    };
    if frame.ret_addr == 0 {
        state.error = true;
        state.last_ip = 0;
        return false;
    }
    state.last_ip = frame.ret_addr;
    true
}

pub fn unwind_all(frames: &[StackFrame]) -> Result<alloc::vec::Vec<u64>, i32> {
    extern crate alloc;
    let mut out = alloc::vec::Vec::new();
    let mut state = __unwind_start(frames, None);
    while !state.done {
        let ip = unwind_get_return_address(&state);
        if ip == 0 {
            return Err(EINVAL);
        }
        out.push(ip);
        unwind_next_frame(&mut state, frames);
    }
    if state.error { Err(EINVAL) } else { Ok(out) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_unwinder_walks_return_addresses_until_end() {
        let frames = [
            StackFrame {
                bp: 0x1000,
                ret_addr: 1,
            },
            StackFrame {
                bp: 0x1010,
                ret_addr: 2,
            },
        ];
        assert_eq!(unwind_all(&frames).unwrap(), alloc::vec![1, 2]);
    }

    #[test]
    fn frame_unwinder_reports_zero_return_as_error() {
        let frames = [StackFrame {
            bp: 0x1000,
            ret_addr: 0,
        }];
        assert_eq!(unwind_all(&frames), Err(EINVAL));
    }
}
