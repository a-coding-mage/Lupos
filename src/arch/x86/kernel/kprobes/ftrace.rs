//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/kprobes/ftrace.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/kprobes/ftrace.c
//! x86 kprobe-on-ftrace glue.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/kprobes/ftrace.c

#![allow(dead_code)]

use crate::arch::x86::kernel::ftrace::MCOUNT_INSN_SIZE;
use crate::arch::x86::kernel::kprobes::core::INT3_INSN_SIZE;

pub const KPROBE_HIT_ACTIVE: u32 = 0x0000_0001;
pub const KPROBE_HIT_SSDONE: u32 = 0x0000_0008;
pub const NOKPROBE_SYMBOL: &str = "kprobe_ftrace_handler";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FtraceArchProbe {
    pub insn_slot_present: bool,
    pub boostable: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FtraceKprobeInput {
    pub ip: u64,
    pub parent_ip: u64,
    pub recursion_bit: i32,
    pub kprobe_ftrace_disabled: bool,
    pub kprobe_running: bool,
}

impl Default for FtraceKprobeInput {
    fn default() -> Self {
        Self {
            ip: 0,
            parent_ip: 0,
            recursion_bit: 0,
            kprobe_ftrace_disabled: false,
            kprobe_running: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FtracePtRegs {
    pub ip: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FtraceKprobe {
    pub disabled: bool,
    pub has_pre_handler: bool,
    pub pre_handler_result: i32,
    pub pre_handler_ip: Option<u64>,
    pub has_post_handler: bool,
    pub nmissed_count: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FtraceKprobeState {
    pub hits: u64,
    pub pre_hits: u64,
    pub post_hits: u64,
    pub last_ip: u64,
    pub current_kprobe_set: bool,
    pub kprobe_status: u32,
    pub recursion_unlocks: u64,
    pub nmissed_count: u64,
    pub returned_disabled: bool,
    pub returned_recursion: bool,
    pub returned_missing_probe: bool,
    pub returned_disabled_probe: bool,
}

pub const fn emulate_mcount_nop(ip: u64) -> u64 {
    ip + MCOUNT_INSN_SIZE as u64
}

pub fn kprobe_ftrace_handler(
    input: FtraceKprobeInput,
    regs: &mut FtracePtRegs,
    probe: &mut Option<FtraceKprobe>,
    state: &mut FtraceKprobeState,
) {
    if input.kprobe_ftrace_disabled {
        state.returned_disabled = true;
        return;
    }

    if input.recursion_bit < 0 {
        state.returned_recursion = true;
        return;
    }

    let Some(probe) = probe.as_mut() else {
        state.returned_missing_probe = true;
        state.recursion_unlocks += 1;
        return;
    };
    if probe.disabled {
        state.returned_disabled_probe = true;
        state.recursion_unlocks += 1;
        return;
    }

    if input.kprobe_running {
        probe.nmissed_count += 1;
        state.nmissed_count += 1;
        state.recursion_unlocks += 1;
        return;
    }

    let orig_ip = regs.ip;
    regs.ip = input.ip + INT3_INSN_SIZE as u64;
    state.current_kprobe_set = true;
    state.kprobe_status = KPROBE_HIT_ACTIVE;
    state.hits += 1;
    state.last_ip = orig_ip;

    let pre_handler_stopped = if probe.has_pre_handler {
        state.pre_hits += 1;
        if let Some(new_ip) = probe.pre_handler_ip {
            regs.ip = new_ip;
        }
        probe.pre_handler_result != 0
    } else {
        false
    };

    if !pre_handler_stopped {
        if probe.has_post_handler {
            regs.ip = input.ip + MCOUNT_INSN_SIZE as u64;
            state.kprobe_status = KPROBE_HIT_SSDONE;
            state.post_hits += 1;
        }
        regs.ip = orig_ip;
    }

    state.current_kprobe_set = false;
    state.recursion_unlocks += 1;
}

pub fn arch_prepare_kprobe_ftrace(probe: &mut FtraceArchProbe) -> i32 {
    probe.insn_slot_present = false;
    probe.boostable = false;
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(ip: u64) -> FtraceKprobeInput {
        FtraceKprobeInput {
            ip,
            parent_ip: ip.wrapping_sub(5),
            ..FtraceKprobeInput::default()
        }
    }

    #[test]
    fn ftrace_kprobe_glue_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/kprobes/ftrace.c"
        ));
        let kprobes = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/kprobes.h"
        ));
        let text_patching = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/asm/text-patching.h"
        ));
        assert!(source.contains("if (unlikely(kprobe_ftrace_disabled))"));
        assert!(source.contains("ftrace_test_recursion_trylock(ip, parent_ip);"));
        assert!(source.contains("p = get_kprobe((kprobe_opcode_t *)ip);"));
        assert!(source.contains("unlikely(!p) || kprobe_disabled(p)"));
        assert!(source.contains("if (kprobe_running())"));
        assert!(source.contains("kprobes_inc_nmissed_count(p);"));
        assert!(source.contains("instruction_pointer_set(regs, ip + INT3_INSN_SIZE);"));
        assert!(source.contains("kcb->kprobe_status = KPROBE_HIT_ACTIVE;"));
        assert!(source.contains("instruction_pointer_set(regs, ip + MCOUNT_INSN_SIZE);"));
        assert!(source.contains("kcb->kprobe_status = KPROBE_HIT_SSDONE;"));
        assert!(source.contains("ftrace_test_recursion_unlock(bit);"));
        assert!(source.contains("NOKPROBE_SYMBOL(kprobe_ftrace_handler);"));
        assert!(source.contains("p->ainsn.insn = NULL;"));
        assert!(source.contains("p->ainsn.boostable = false;"));
        assert!(kprobes.contains("#define KPROBE_HIT_ACTIVE\t0x00000001"));
        assert!(kprobes.contains("#define KPROBE_HIT_SSDONE\t0x00000008"));
        assert!(text_patching.contains("#define INT3_INSN_SIZE\t\t1"));

        assert_eq!(KPROBE_HIT_ACTIVE, 1);
        assert_eq!(KPROBE_HIT_SSDONE, 8);
        assert_eq!(emulate_mcount_nop(0x1000), 0x1005);
        assert_eq!(NOKPROBE_SYMBOL, "kprobe_ftrace_handler");
    }

    #[test]
    fn ftrace_handler_models_early_exit_and_unlock_paths() {
        let mut regs = FtracePtRegs { ip: 0x2000 };
        let mut state = FtraceKprobeState::default();
        let mut probe = Some(FtraceKprobe::default());
        kprobe_ftrace_handler(
            FtraceKprobeInput {
                kprobe_ftrace_disabled: true,
                ..input(0x1000)
            },
            &mut regs,
            &mut probe,
            &mut state,
        );
        assert!(state.returned_disabled);
        assert_eq!(state.recursion_unlocks, 0);

        let mut state = FtraceKprobeState::default();
        kprobe_ftrace_handler(
            FtraceKprobeInput {
                recursion_bit: -1,
                ..input(0x1000)
            },
            &mut regs,
            &mut probe,
            &mut state,
        );
        assert!(state.returned_recursion);
        assert_eq!(state.recursion_unlocks, 0);

        let mut state = FtraceKprobeState::default();
        let mut missing = None;
        kprobe_ftrace_handler(input(0x1000), &mut regs, &mut missing, &mut state);
        assert!(state.returned_missing_probe);
        assert_eq!(state.recursion_unlocks, 1);

        let mut state = FtraceKprobeState::default();
        let mut disabled = Some(FtraceKprobe {
            disabled: true,
            ..FtraceKprobe::default()
        });
        kprobe_ftrace_handler(input(0x1000), &mut regs, &mut disabled, &mut state);
        assert!(state.returned_disabled_probe);
        assert_eq!(state.recursion_unlocks, 1);
    }

    #[test]
    fn ftrace_handler_models_running_pre_post_and_ip_recovery() {
        let mut regs = FtracePtRegs { ip: 0x2000 };
        let mut state = FtraceKprobeState::default();
        let mut running_probe = Some(FtraceKprobe::default());
        kprobe_ftrace_handler(
            FtraceKprobeInput {
                kprobe_running: true,
                ..input(0x1000)
            },
            &mut regs,
            &mut running_probe,
            &mut state,
        );
        assert_eq!(running_probe.unwrap().nmissed_count, 1);
        assert_eq!(state.nmissed_count, 1);
        assert_eq!(state.recursion_unlocks, 1);

        let mut state = FtraceKprobeState::default();
        let mut probe = Some(FtraceKprobe {
            has_pre_handler: true,
            has_post_handler: true,
            ..FtraceKprobe::default()
        });
        kprobe_ftrace_handler(input(0x1000), &mut regs, &mut probe, &mut state);
        assert_eq!(regs.ip, 0x2000);
        assert!(!state.current_kprobe_set);
        assert_eq!(state.kprobe_status, KPROBE_HIT_SSDONE);
        assert_eq!(state.hits, 1);
        assert_eq!(state.pre_hits, 1);
        assert_eq!(state.post_hits, 1);
        assert_eq!(state.recursion_unlocks, 1);
    }

    #[test]
    fn ftrace_pre_handler_nonzero_skips_post_and_recovery() {
        let mut regs = FtracePtRegs { ip: 0x2000 };
        let mut state = FtraceKprobeState::default();
        let mut probe = Some(FtraceKprobe {
            has_pre_handler: true,
            pre_handler_result: 1,
            pre_handler_ip: Some(0x3000),
            has_post_handler: true,
            ..FtraceKprobe::default()
        });
        kprobe_ftrace_handler(input(0x1000), &mut regs, &mut probe, &mut state);
        assert_eq!(regs.ip, 0x3000);
        assert_eq!(state.kprobe_status, KPROBE_HIT_ACTIVE);
        assert_eq!(state.pre_hits, 1);
        assert_eq!(state.post_hits, 0);
        assert!(!state.current_kprobe_set);
    }

    #[test]
    fn ftrace_prepare_disables_slot_and_boosting() {
        let mut probe = FtraceArchProbe {
            insn_slot_present: true,
            boostable: true,
        };
        assert_eq!(arch_prepare_kprobe_ftrace(&mut probe), 0);
        assert_eq!(
            probe,
            FtraceArchProbe {
                insn_slot_present: false,
                boostable: false,
            }
        );
    }
}
