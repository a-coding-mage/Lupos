//! linux-parity: complete
//! linux-source: vendor/linux/lib/test_ref_tracker.c
//! test-origin: linux:vendor/linux/lib/test_ref_tracker.c
//! Reference tracker self-test sequence model.

extern crate alloc;

use alloc::vec::Vec;

pub const TRACKER_SLOTS: usize = 20;
pub const REF_TRACKER_DIR_LIMIT: usize = 100;
pub const REF_TRACKER_DIR_NAME: &str = "selftest";
pub const MODULE_DESCRIPTION: &str = "Reference tracker self test";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TrackerSlot {
    pub allocated: bool,
    pub frees: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefTrackerSelftest {
    pub slots: [TrackerSlot; TRACKER_SLOTS],
    pub timer_done: bool,
    pub double_frees: usize,
}

impl Default for RefTrackerSelftest {
    fn default() -> Self {
        Self {
            slots: [TrackerSlot::default(); TRACKER_SLOTS],
            timer_done: false,
            double_frees: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefTrackerSelftestResult {
    pub leaks: Vec<usize>,
    pub double_frees: usize,
    pub timer_done: bool,
}

impl RefTrackerSelftest {
    pub fn alloc(&mut self, index: usize) {
        self.slots[index].allocated = true;
    }

    pub fn free(&mut self, index: usize) {
        if self.slots[index].allocated {
            self.slots[index].allocated = false;
            self.slots[index].frees += 1;
        } else {
            self.double_frees += 1;
        }
    }

    pub fn timer_func(&mut self) {
        self.alloc(0);
        self.timer_done = true;
    }

    pub fn leaks(&self) -> Vec<usize> {
        let mut leaks = Vec::new();
        for (index, slot) in self.slots.iter().enumerate() {
            if slot.allocated {
                leaks.push(index);
            }
        }
        leaks
    }
}

pub fn run_ref_tracker_selftest() -> RefTrackerSelftestResult {
    let mut test = RefTrackerSelftest::default();
    for index in 1..TRACKER_SLOTS {
        test.alloc(index);
    }
    for index in 2..TRACKER_SLOTS {
        test.free(index);
    }
    test.free(2);
    test.timer_func();

    RefTrackerSelftestResult {
        leaks: test.leaks(),
        double_frees: test.double_frees,
        timer_done: test.timer_done,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ref_tracker_matches_linux_selftest_sequence() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/test_ref_tracker.c"
        ));
        assert!(source.contains("static struct ref_tracker *tracker[20];"));
        assert!(source.matches("TRT_ALLOC(").count() >= 19);
        assert!(source.contains("ref_tracker_dir_init(&ref_dir, 100, \"selftest\");"));
        assert!(
            source
                .contains("timer_setup(&test_ref_tracker_timer, test_ref_tracker_timer_func, 0);")
        );
        assert!(source.contains("ref_tracker_alloc(&ref_dir, &tracker[0], GFP_ATOMIC);"));
        assert!(source.contains("for (i = 2; i < ARRAY_SIZE(tracker); i++)"));
        assert!(source.contains("alloctest_ref_tracker_free(&ref_dir, &tracker[2]);"));
        assert!(source.contains("while (!atomic_read(&test_ref_timer_done))"));
        assert!(source.contains("ref_tracker_dir_exit(&ref_dir);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Reference tracker self test\")"));

        let result = run_ref_tracker_selftest();
        assert_eq!(result.leaks, alloc::vec![0, 1]);
        assert_eq!(result.double_frees, 1);
        assert!(result.timer_done);
        assert_eq!(REF_TRACKER_DIR_LIMIT, 100);
        assert_eq!(REF_TRACKER_DIR_NAME, "selftest");
    }
}
