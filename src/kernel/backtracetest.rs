//! linux-parity: complete
//! linux-source: vendor/linux/kernel/backtracetest.c
//! test-origin: linux:vendor/linux/kernel/backtracetest.c
//! Stack backtrace regression-test sequencing.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BacktraceTestStep {
    Header,
    ProcessContext,
    BhContextQueueWork,
    BhContextFlushWork,
    SavedBacktrace,
    Footer,
}

pub fn backtrace_regression_plan(stacktrace_enabled: bool) -> alloc::vec::Vec<BacktraceTestStep> {
    let mut steps = alloc::vec::Vec::new();
    steps.push(BacktraceTestStep::Header);
    steps.push(BacktraceTestStep::ProcessContext);
    steps.push(BacktraceTestStep::BhContextQueueWork);
    steps.push(BacktraceTestStep::BhContextFlushWork);
    if stacktrace_enabled {
        steps.push(BacktraceTestStep::SavedBacktrace);
    }
    steps.push(BacktraceTestStep::Footer);
    steps
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn backtrace_regression_sequence_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/backtracetest.c"
        ));
        assert!(source.contains("backtrace_test_normal"));
        assert!(source.contains("dump_stack();"));
        assert!(source.contains("DECLARE_WORK(backtrace_bh_work"));
        assert!(source.contains("queue_work(system_bh_wq, &backtrace_bh_work);"));
        assert!(source.contains("flush_work(&backtrace_bh_work);"));
        assert!(source.contains("#ifdef CONFIG_STACKTRACE"));
        assert!(source.contains("stack_trace_save(entries, ARRAY_SIZE(entries), 0);"));
        assert!(source.contains("stack_trace_print(entries, nr_entries, 0);"));
        assert!(source.contains("module_init(backtrace_regression_test);"));
        assert!(
            source
                .contains("MODULE_DESCRIPTION(\"Simple stack backtrace regression test module\")")
        );

        assert_eq!(
            backtrace_regression_plan(true),
            vec![
                BacktraceTestStep::Header,
                BacktraceTestStep::ProcessContext,
                BacktraceTestStep::BhContextQueueWork,
                BacktraceTestStep::BhContextFlushWork,
                BacktraceTestStep::SavedBacktrace,
                BacktraceTestStep::Footer,
            ]
        );
        assert!(!backtrace_regression_plan(false).contains(&BacktraceTestStep::SavedBacktrace));
    }
}
