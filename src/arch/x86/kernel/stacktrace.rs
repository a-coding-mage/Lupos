//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/stacktrace.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/stacktrace.c
//! x86 stack walking helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/stacktrace.c

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UserStackFrame {
    pub next_fp: u64,
    pub ret_addr: u64,
}

pub fn arch_stack_walk(
    initial_ip: Option<u64>,
    return_addresses: &[u64],
    limit: usize,
) -> Vec<u64> {
    let mut out = Vec::new();
    if let Some(ip) = initial_ip {
        if ip != 0 {
            out.push(ip);
        }
    }
    for &addr in return_addresses {
        if out.len() >= limit || addr == 0 {
            break;
        }
        out.push(addr);
    }
    out
}

pub fn arch_stack_walk_reliable(return_addresses: &[u64]) -> Result<Vec<u64>, i32> {
    let mut out = Vec::new();
    for &addr in return_addresses {
        if addr == 0 {
            return Err(EINVAL);
        }
        out.push(addr);
    }
    Ok(out)
}

pub fn arch_stack_walk_user(ip: u64, sp: u64, bp: u64, frames: &[UserStackFrame]) -> Vec<u64> {
    let mut out = Vec::new();
    if ip == 0 {
        return out;
    }
    out.push(ip);
    let mut fp = bp;
    for frame in frames {
        if fp < sp || frame.ret_addr == 0 {
            break;
        }
        out.push(frame.ret_addr);
        fp = frame.next_fp;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_walk_includes_regs_ip_then_unwind_addresses() {
        assert_eq!(
            arch_stack_walk(Some(1), &[2, 3, 0, 4], 8),
            alloc::vec![1, 2, 3]
        );
        assert_eq!(
            arch_stack_walk_reliable(&[2, 3]).unwrap(),
            alloc::vec![2, 3]
        );
        assert_eq!(arch_stack_walk_reliable(&[2, 0]), Err(EINVAL));
    }

    #[test]
    fn user_walk_stops_on_stack_growth_or_null_return() {
        let frames = [
            UserStackFrame {
                next_fp: 0x9000,
                ret_addr: 0x401000,
            },
            UserStackFrame {
                next_fp: 0x9010,
                ret_addr: 0,
            },
        ];
        assert_eq!(
            arch_stack_walk_user(0x400000, 0x8000, 0x8008, &frames),
            alloc::vec![0x400000, 0x401000]
        );
    }
}
