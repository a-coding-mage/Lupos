//! linux-parity: complete
//! linux-source: vendor/linux/mm/tests/lazy_mmu_mode_kunit.c
//! test-origin: linux:vendor/linux/mm/tests/lazy_mmu_mode_kunit.c
//! Lazy MMU mode nesting and pause semantics.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LazyMmuMode {
    depth: usize,
    paused: usize,
}

impl LazyMmuMode {
    pub const fn new() -> Self {
        Self {
            depth: 0,
            paused: 0,
        }
    }

    pub const fn is_active(&self) -> bool {
        self.depth != 0 && self.paused == 0
    }

    pub fn enable(&mut self) {
        if self.paused == 0 {
            self.depth += 1;
        }
    }

    pub fn disable(&mut self) {
        if self.paused == 0 && self.depth != 0 {
            self.depth -= 1;
        }
    }

    pub fn pause(&mut self) {
        self.paused += 1;
    }

    pub fn resume(&mut self) {
        if self.paused != 0 {
            self.paused -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lazy_mmu_mode_kunit_sequence_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/tests/lazy_mmu_mode_kunit.c"
        ));
        assert!(source.contains("expect_not_active(test);"));
        assert!(source.contains("lazy_mmu_mode_enable();"));
        assert!(source.contains("lazy_mmu_mode_disable();"));
        assert!(source.contains("lazy_mmu_mode_pause();"));
        assert!(source.contains("lazy_mmu_mode_resume();"));
        assert!(source.contains(".name = \"lazy_mmu_mode\""));
        assert!(source.contains("MODULE_DESCRIPTION(\"Tests for the lazy MMU mode\")"));

        let mut mode = LazyMmuMode::new();
        assert!(!mode.is_active());
        mode.enable();
        assert!(mode.is_active());
        mode.enable();
        assert!(mode.is_active());
        mode.disable();
        assert!(mode.is_active());
        mode.pause();
        assert!(!mode.is_active());
        mode.enable();
        mode.disable();
        mode.pause();
        mode.resume();
        assert!(!mode.is_active());
        mode.resume();
        assert!(mode.is_active());
        mode.disable();
        assert!(!mode.is_active());
    }
}
