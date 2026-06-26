//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/kprobes
//! x86 kprobe front-end.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/kprobes/core.c
//! - vendor/linux/arch/x86/kernel/kprobes/ftrace.c
//! - vendor/linux/arch/x86/kernel/kprobes/opt.c

pub mod core;
pub mod ftrace;
pub mod opt;
