//! linux-parity: partial
//! linux-source: vendor/linux/init/main.c
//! test-origin: linux:vendor/linux/init/main.c
//! Linux initcall level sequencing.
//!
//! Linux collects initcall entries into linker ranges and runs them from
//! `kernel_init_freeable()` via `do_pre_smp_initcalls()` and `do_initcalls()`.
//! Lupos does not yet have linker-collected initcall sections, so this module
//! provides the same level model with explicit tables for translated one-shot
//! hooks that are currently wired by hand.

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum InitcallLevel {
    Pure = 0,
    Core = 1,
    Postcore = 2,
    Arch = 3,
    Subsys = 4,
    Fs = 5,
    Device = 6,
    Late = 7,
}

impl InitcallLevel {
    pub const fn name(self) -> &'static str {
        INITCALL_LEVEL_NAMES[self as usize]
    }
}

/// Keep in sync with `initcall_level_names[]` in Linux `init/main.c`.
pub const INITCALL_LEVEL_NAMES: [&str; 8] = [
    "pure", "core", "postcore", "arch", "subsys", "fs", "device", "late",
];

pub type InitcallFn = fn() -> i32;

#[derive(Copy, Clone)]
pub struct Initcall {
    pub name: &'static str,
    pub func: InitcallFn,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct InitcallReport {
    pub level: InitcallLevel,
    pub ran: usize,
    pub first_error: Option<i32>,
}

fn taskstats_init_late() -> i32 {
    crate::kernel::taskstats::init();
    0
}

fn zswap_init_late() -> i32 {
    crate::mm::zswap::init();
    0
}

/// Explicit late-initcall table for translated hooks that Linux marks with
/// `late_initcall(...)` but Lupos previously called directly from
/// `kernel_main`.
pub const LUPOS_LATE_INITCALLS: &[Initcall] = &[
    Initcall {
        name: "taskstats_init",
        func: taskstats_init_late,
    },
    Initcall {
        name: "zswap_init",
        func: zswap_init_late,
    },
];

pub fn do_initcall_level(level: InitcallLevel, initcalls: &[Initcall]) -> InitcallReport {
    let mut first_error = None;

    for initcall in initcalls {
        let ret = (initcall.func)();
        if ret != 0 && first_error.is_none() {
            first_error = Some(ret);
        }
    }

    InitcallReport {
        level,
        ran: initcalls.len(),
        first_error,
    }
}

pub fn do_late_initcalls() -> InitcallReport {
    do_initcall_level(InitcallLevel::Late, LUPOS_LATE_INITCALLS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    extern crate std;

    fn ok() -> i32 {
        0
    }

    fn fail() -> i32 {
        -22
    }

    #[test]
    fn initcall_level_names_match_linux_main() {
        let source = std::fs::read_to_string("vendor/linux/init/main.c")
            .expect("read vendor Linux init/main.c");
        assert!(source.contains("static const char *initcall_level_names[]"));
        for name in INITCALL_LEVEL_NAMES {
            assert!(source.contains(&format!("\"{name}\"")));
        }
    }

    #[test]
    fn initcall_level_runner_preserves_count_and_first_error() {
        let calls = [
            Initcall {
                name: "ok0",
                func: ok,
            },
            Initcall {
                name: "fail",
                func: fail,
            },
            Initcall {
                name: "ok1",
                func: ok,
            },
        ];

        let report = do_initcall_level(InitcallLevel::Late, &calls);

        assert_eq!(
            report,
            InitcallReport {
                level: InitcallLevel::Late,
                ran: 3,
                first_error: Some(-22),
            }
        );
    }

    #[test]
    fn translated_lupos_hooks_are_late_initcalls() {
        let names: [&str; 2] = [LUPOS_LATE_INITCALLS[0].name, LUPOS_LATE_INITCALLS[1].name];

        assert_eq!(InitcallLevel::Late.name(), "late");
        assert_eq!(names, ["taskstats_init", "zswap_init"]);
    }
}
