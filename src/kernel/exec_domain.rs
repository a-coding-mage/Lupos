//! linux-parity: complete
//! linux-source: vendor/linux/kernel/exec_domain.c
//! test-origin: linux:vendor/linux/kernel/exec_domain.c
//! Linux execution-domain/personality syscall behavior.

use core::sync::atomic::{AtomicU32, Ordering};

pub const PERSONALITY_QUERY: u32 = 0xffff_ffff;
pub const EXEC_DOMAINS_PROC_LINE: &str = "0-0\tLinux           \t[kernel]\n";

static CURRENT_PERSONALITY: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PersonalityState {
    pub current: u32,
}

impl PersonalityState {
    pub const fn new(current: u32) -> Self {
        Self { current }
    }

    pub fn personality(&mut self, personality: u32) -> u32 {
        let old = self.current;
        if personality != PERSONALITY_QUERY {
            self.current = personality;
        }
        old
    }
}

pub const fn execdomains_proc_show() -> &'static str {
    EXEC_DOMAINS_PROC_LINE
}

pub fn proc_execdomains_init() -> i32 {
    0
}

pub fn sys_personality(personality: u32) -> i64 {
    let old = CURRENT_PERSONALITY.load(Ordering::Acquire);
    if personality != PERSONALITY_QUERY {
        CURRENT_PERSONALITY.store(personality, Ordering::Release);
    }
    old as i64
}

#[cfg(test)]
pub fn reset_personality_for_test() {
    CURRENT_PERSONALITY.store(0, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn personality_syscall_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/exec_domain.c"
        ));
        assert!(source.contains("seq_puts(m, \"0-0\\tLinux"));
        assert!(source.contains("SYSCALL_DEFINE1(personality"));
        assert!(source.contains("unsigned int old = current->personality;"));
        assert!(source.contains("if (personality != 0xffffffff)"));
        assert!(source.contains("set_personality(personality);"));
        assert!(source.contains("return old;"));

        assert_eq!(execdomains_proc_show(), EXEC_DOMAINS_PROC_LINE);
        assert_eq!(proc_execdomains_init(), 0);
        let mut state = PersonalityState::new(7);
        assert_eq!(state.personality(PERSONALITY_QUERY), 7);
        assert_eq!(state.current, 7);
        assert_eq!(state.personality(3), 7);
        assert_eq!(state.current, 3);

        reset_personality_for_test();
        assert_eq!(sys_personality(PERSONALITY_QUERY), 0);
        assert_eq!(sys_personality(0x08), 0);
        assert_eq!(sys_personality(PERSONALITY_QUERY), 0x08);
        reset_personality_for_test();
    }
}
