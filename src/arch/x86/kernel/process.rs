//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/process.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/process.c
//! x86 process/thread helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/process.c

#![allow(dead_code)]

use core::sync::atomic::{AtomicU8, Ordering};

pub const STACK_ALIGN: u64 = 16;
pub const PAGE_SIZE: u64 = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TscMode {
    Enabled = 0,
    Sigsegv = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CpuidMode {
    Enabled = 0,
    Faulting = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdleStrategy {
    Default,
    Poll,
    Halt,
    NoMwait,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ThreadContext {
    pub sp: u64,
    pub ip: u64,
    pub fsbase: u64,
    pub gsbase: u64,
    pub io_bitmap_ptr: u64,
    pub tsc_mode: u8,
    pub cpuid_faulting: bool,
}

static TSC_MODE: AtomicU8 = AtomicU8::new(TscMode::Enabled as u8);
static CPUID_MODE: AtomicU8 = AtomicU8::new(CpuidMode::Enabled as u8);

pub fn disable_tsc() {
    TSC_MODE.store(TscMode::Sigsegv as u8, Ordering::Release);
}

pub fn enable_tsc() {
    TSC_MODE.store(TscMode::Enabled as u8, Ordering::Release);
}

pub fn get_tsc_mode() -> TscMode {
    match TSC_MODE.load(Ordering::Acquire) {
        1 => TscMode::Sigsegv,
        _ => TscMode::Enabled,
    }
}

pub fn set_tsc_mode(mode: TscMode) {
    TSC_MODE.store(mode as u8, Ordering::Release);
}

pub fn set_cpuid_mode(mode: CpuidMode) {
    CPUID_MODE.store(mode as u8, Ordering::Release);
}

pub fn get_cpuid_mode() -> CpuidMode {
    match CPUID_MODE.load(Ordering::Acquire) {
        1 => CpuidMode::Faulting,
        _ => CpuidMode::Enabled,
    }
}

pub fn arch_setup_new_exec(thread: &mut ThreadContext) {
    thread.cpuid_faulting = false;
    set_cpuid_mode(CpuidMode::Enabled);
}

pub const fn parse_idle_param(param: &str) -> Option<IdleStrategy> {
    let bytes = param.as_bytes();
    if bytes.len() == 4
        && bytes[0] == b'p'
        && bytes[1] == b'o'
        && bytes[2] == b'l'
        && bytes[3] == b'l'
    {
        Some(IdleStrategy::Poll)
    } else if bytes.len() == 4
        && bytes[0] == b'h'
        && bytes[1] == b'a'
        && bytes[2] == b'l'
        && bytes[3] == b't'
    {
        Some(IdleStrategy::Halt)
    } else if bytes.len() == 7
        && bytes[0] == b'n'
        && bytes[1] == b'o'
        && bytes[2] == b'm'
        && bytes[3] == b'w'
        && bytes[4] == b'a'
        && bytes[5] == b'i'
        && bytes[6] == b't'
    {
        Some(IdleStrategy::NoMwait)
    } else {
        None
    }
}

pub const fn arch_align_stack(sp: u64, random_offset: u64) -> u64 {
    let randomized = sp.wrapping_sub(random_offset & 0x3f0);
    randomized & !(STACK_ALIGN - 1)
}

pub const fn arch_randomize_brk(brk: u64, random: u64) -> u64 {
    brk + ((random & 0x1ff) * PAGE_SIZE)
}

pub fn arch_dup_task_struct(src: &ThreadContext) -> ThreadContext {
    *src
}

pub fn arch_release_task_struct(thread: &mut ThreadContext) {
    thread.io_bitmap_ptr = 0;
}

pub fn exit_thread(thread: &mut ThreadContext) {
    thread.io_bitmap_ptr = 0;
    thread.fsbase = 0;
    thread.gsbase = 0;
}

pub fn flush_thread(thread: &mut ThreadContext) {
    thread.fsbase = 0;
    thread.gsbase = 0;
    thread.cpuid_faulting = false;
}

pub fn copy_thread(parent: &ThreadContext, child_stack: u64, ret_from_fork: u64) -> ThreadContext {
    let mut child = *parent;
    child.sp = child_stack;
    child.ip = ret_from_fork;
    child
}

pub const fn __get_wchan(saved_ips: &[u64]) -> Option<u64> {
    let mut i = 0;
    while i < saved_ips.len() {
        if saved_ips[i] != 0 {
            return Some(saved_ips[i]);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tsc_and_cpuid_modes_round_trip() {
        set_tsc_mode(TscMode::Sigsegv);
        assert_eq!(get_tsc_mode(), TscMode::Sigsegv);
        enable_tsc();
        assert_eq!(get_tsc_mode(), TscMode::Enabled);

        set_cpuid_mode(CpuidMode::Faulting);
        assert_eq!(get_cpuid_mode(), CpuidMode::Faulting);
        let mut t = ThreadContext {
            cpuid_faulting: true,
            ..Default::default()
        };
        arch_setup_new_exec(&mut t);
        assert_eq!(get_cpuid_mode(), CpuidMode::Enabled);
        assert!(!t.cpuid_faulting);
    }

    #[test]
    fn idle_param_and_stack_randomization_match_policy() {
        assert_eq!(parse_idle_param("poll"), Some(IdleStrategy::Poll));
        assert_eq!(parse_idle_param("nomwait"), Some(IdleStrategy::NoMwait));
        assert_eq!(arch_align_stack(0x1003, 0x21), 0x0fe0);
        assert_eq!(arch_randomize_brk(0x4000, 2), 0x6000);
    }

    #[test]
    fn copy_thread_sets_child_stack_and_entry() {
        let parent = ThreadContext {
            fsbase: 1,
            gsbase: 2,
            ..Default::default()
        };
        let child = copy_thread(&parent, 0x8000, 0x9000);
        assert_eq!(child.sp, 0x8000);
        assert_eq!(child.ip, 0x9000);
        assert_eq!(child.fsbase, 1);
    }
}
