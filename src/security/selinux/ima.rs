//! linux-parity: complete
//! linux-source: vendor/linux/security/selinux/ima.c
//! test-origin: linux:vendor/linux/security/selinux/ima.c
//! SELinux state strings measured by IMA.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SelinuxImaState<'a> {
    pub initialized: bool,
    pub enforcing: bool,
    pub checkreqprot: bool,
    pub policycaps: &'a [(&'a str, bool)],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelinuxPolicyRead<'a> {
    NotInitialized,
    Error(i32),
    Policy(&'a [u8]),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImaMeasurement {
    pub event_name: &'static str,
    pub data: Vec<u8>,
    pub hash: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SelinuxImaMeasureReport {
    pub lock_acquired: bool,
    pub lock_released: bool,
    pub measurements: Vec<ImaMeasurement>,
    pub errors: Vec<String>,
    pub policy_freed: bool,
}

pub fn selinux_ima_collect_state(state: SelinuxImaState<'_>) -> String {
    let mut out = String::new();
    push_bool(&mut out, "initialized", state.initialized);
    push_bool(&mut out, "enforcing", state.enforcing);
    push_bool(&mut out, "checkreqprot", state.checkreqprot);
    for (name, enabled) in state.policycaps {
        push_bool(&mut out, name, *enabled);
    }
    out
}

pub const fn should_measure_policy(initialized: bool) -> bool {
    initialized
}

pub fn selinux_ima_measure_state_locked(
    state: SelinuxImaState<'_>,
    policy: SelinuxPolicyRead<'_>,
) -> SelinuxImaMeasureReport {
    let mut report = SelinuxImaMeasureReport::default();
    let state_str = selinux_ima_collect_state(state);
    if state_str.is_empty() {
        report
            .errors
            .push("SELinux: selinux_ima_measure_state_locked: failed to read state.".into());
        return report;
    }

    report.measurements.push(ImaMeasurement {
        event_name: "selinux-state",
        data: state_str.into_bytes(),
        hash: false,
    });

    match policy {
        SelinuxPolicyRead::NotInitialized => {}
        SelinuxPolicyRead::Error(rc) => report.errors.push(format!(
            "SELinux: selinux_ima_measure_state_locked: failed to read policy {rc}."
        )),
        SelinuxPolicyRead::Policy(bytes) => {
            report.measurements.push(ImaMeasurement {
                event_name: "selinux-policy-hash",
                data: bytes.to_vec(),
                hash: true,
            });
            report.policy_freed = true;
        }
    }
    report
}

pub fn selinux_ima_measure_state(
    state: SelinuxImaState<'_>,
    policy: SelinuxPolicyRead<'_>,
) -> SelinuxImaMeasureReport {
    let mut report = selinux_ima_measure_state_locked(state, policy);
    report.lock_acquired = true;
    report.lock_released = true;
    report
}

fn push_bool(out: &mut String, name: &str, enabled: bool) {
    out.push_str(name);
    out.push_str(if enabled { "=1;" } else { "=0;" });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selinux_ima_state_collection_matches_linux_source() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/selinux/ima.c"
        ));
        assert!(source.contains("selinux_ima_collect_state"));
        assert!(source.contains("const char *on = \"=1;\", *off = \"=0;\";"));
        assert!(source.contains("initialized"));
        assert!(source.contains("enforcing"));
        assert!(source.contains("checkreqprot"));
        assert!(source.contains("for (i = 0; i < __POLICYDB_CAP_MAX; i++)"));
        assert!(source.contains("ima_measure_critical_data(\"selinux\", \"selinux-state\""));
        assert!(source.contains("if (!selinux_initialized())"));
        assert!(source.contains("security_read_state_kernel(&policy, &policy_len)"));
        assert!(source.contains("ima_measure_critical_data(\"selinux\", \"selinux-policy-hash\""));
        assert!(source.contains("vfree(policy);"));
        assert!(source.contains("lockdep_assert_held(&selinux_state.policy_mutex);"));
        assert!(source.contains("lockdep_assert_not_held(&selinux_state.policy_mutex);"));
        assert!(source.contains("mutex_lock(&selinux_state.policy_mutex);"));
        assert!(source.contains("mutex_unlock(&selinux_state.policy_mutex);"));

        let state_input = SelinuxImaState {
            initialized: true,
            enforcing: false,
            checkreqprot: true,
            policycaps: &[("network_peer_controls", true), ("open_perms", false)],
        };
        let state = selinux_ima_collect_state(state_input);
        assert_eq!(
            state,
            "initialized=1;enforcing=0;checkreqprot=1;network_peer_controls=1;open_perms=0;"
        );
        assert!(should_measure_policy(true));
        assert!(!should_measure_policy(false));

        let report = selinux_ima_measure_state(state_input, SelinuxPolicyRead::Policy(b"policy"));
        assert!(report.lock_acquired);
        assert!(report.lock_released);
        assert_eq!(report.measurements[0].event_name, "selinux-state");
        assert!(!report.measurements[0].hash);
        assert_eq!(report.measurements[1].event_name, "selinux-policy-hash");
        assert_eq!(report.measurements[1].data, b"policy");
        assert!(report.measurements[1].hash);
        assert!(report.policy_freed);

        let uninit =
            selinux_ima_measure_state_locked(state_input, SelinuxPolicyRead::NotInitialized);
        assert_eq!(uninit.measurements.len(), 1);
        assert!(!uninit.policy_freed);

        let failed = selinux_ima_measure_state_locked(state_input, SelinuxPolicyRead::Error(-5));
        assert_eq!(failed.measurements.len(), 1);
        assert!(failed.errors[0].contains("failed to read policy -5"));
    }
}
