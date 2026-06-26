//! linux-parity: complete
//! linux-source: vendor/linux/kernel/ksyms_common.c
//! test-origin: linux:vendor/linux/kernel/ksyms_common.c
//! Kallsyms address visibility policy shared with non-KALLSYMS builds.

use crate::kernel::capability::CAP_SYSLOG;

pub const KPTR_RESTRICT_DISABLED: u8 = 0;
pub const KPTR_RESTRICT_CAP_SYSLOG: u8 = 1;

pub const fn kallsyms_for_perf(perf_events_enabled: bool, perf_event_paranoid: i32) -> bool {
    perf_events_enabled && perf_event_paranoid <= 1
}

pub const fn kallsyms_show_value(
    kptr_restrict: u8,
    perf_events_enabled: bool,
    perf_event_paranoid: i32,
    capable_cap: Option<u32>,
) -> bool {
    match kptr_restrict {
        KPTR_RESTRICT_DISABLED => {
            if kallsyms_for_perf(perf_events_enabled, perf_event_paranoid) {
                true
            } else {
                has_cap_syslog(capable_cap)
            }
        }
        KPTR_RESTRICT_CAP_SYSLOG => has_cap_syslog(capable_cap),
        _ => false,
    }
}

const fn has_cap_syslog(capable_cap: Option<u32>) -> bool {
    match capable_cap {
        Some(cap) => cap == CAP_SYSLOG,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kallsyms_show_value_matches_linux_fallthrough_policy() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/ksyms_common.c"
        ));
        assert!(source.contains("static inline int kallsyms_for_perf(void)"));
        assert!(source.contains("sysctl_perf_event_paranoid <= 1"));
        assert!(source.contains("switch (kptr_restrict)"));
        assert!(source.contains("security_capable(cred, &init_user_ns, CAP_SYSLOG"));
        assert!(source.contains("fallthrough;"));

        assert!(kallsyms_show_value(0, true, 1, None));
        assert!(kallsyms_show_value(0, false, 3, Some(CAP_SYSLOG)));
        assert!(!kallsyms_show_value(0, false, 3, None));
        assert!(kallsyms_show_value(1, false, 3, Some(CAP_SYSLOG)));
        assert!(!kallsyms_show_value(2, true, 0, Some(CAP_SYSLOG)));
    }
}
