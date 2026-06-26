//! linux-parity: complete
//! linux-source: vendor/linux/security/tomoyo/load_policy.c
//! test-origin: linux:vendor/linux/security/tomoyo/load_policy.c
//! TOMOYO policy-loader activation flow.

extern crate alloc;

pub mod environ;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

pub const DEFAULT_LOADER_CONFIG: &str = "/sbin/tomoyo-init";
pub const DEFAULT_TRIGGER_CONFIG: &str = "/sbin/init";
pub const LOADER_SETUP_PREFIX: &str = "TOMOYO_loader=";
pub const TRIGGER_SETUP_PREFIX: &str = "TOMOYO_trigger=";
pub const POLICY_LOADER_ENV: &[&str] = &["HOME=/", "PATH=/sbin:/bin:/usr/sbin:/usr/bin"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TomoyoLoadDecision {
    AlreadyLoaded,
    TriggerMismatch,
    LoaderMissing,
    CallLoader,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UsermodeHelperCall {
    pub path: String,
    pub argv: [Option<String>; 2],
    pub envp: [Option<String>; 3],
    pub wait_proc: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TomoyoLoadPolicy {
    loader: Option<String>,
    trigger: Option<String>,
    pub policy_loaded: bool,
    done: bool,
    loader_exists: bool,
    pub logs: Vec<String>,
    pub helper_calls: Vec<UsermodeHelperCall>,
    pub profile_checked: bool,
}

impl TomoyoLoadPolicy {
    pub fn new(loader_exists: bool) -> Self {
        Self {
            loader: None,
            trigger: None,
            policy_loaded: false,
            done: false,
            loader_exists,
            logs: Vec::new(),
            helper_calls: Vec::new(),
            profile_checked: false,
        }
    }

    pub fn tomoyo_loader_setup(&mut self, value: &str) -> i32 {
        self.loader = Some(value.to_string());
        1
    }

    pub fn tomoyo_trigger_setup(&mut self, value: &str) -> i32 {
        self.trigger = Some(value.to_string());
        1
    }

    pub fn loader(&mut self) -> &str {
        self.loader
            .get_or_insert_with(|| DEFAULT_LOADER_CONFIG.to_string())
            .as_str()
    }

    pub fn trigger(&mut self) -> &str {
        self.trigger
            .get_or_insert_with(|| DEFAULT_TRIGGER_CONFIG.to_string())
            .as_str()
    }

    pub fn tomoyo_policy_loader_exists(&mut self) -> bool {
        let loader = self.loader().to_string();
        if !self.loader_exists {
            self.logs.push(format!(
                "Not activating Mandatory Access Control as {loader} does not exist."
            ));
            return false;
        }
        true
    }

    pub fn tomoyo_load_policy(&mut self, filename: &str) -> TomoyoLoadDecision {
        if self.policy_loaded || self.done {
            return TomoyoLoadDecision::AlreadyLoaded;
        }

        let trigger = self.trigger().to_string();
        if filename != trigger {
            return TomoyoLoadDecision::TriggerMismatch;
        }

        if !self.tomoyo_policy_loader_exists() {
            return TomoyoLoadDecision::LoaderMissing;
        }

        self.done = true;
        let loader = self.loader().to_string();
        self.logs
            .push(format!("Calling {loader} to load policy. Please wait."));
        self.helper_calls.push(UsermodeHelperCall {
            path: loader.clone(),
            argv: [Some(loader), None],
            envp: [
                Some(POLICY_LOADER_ENV[0].to_string()),
                Some(POLICY_LOADER_ENV[1].to_string()),
                None,
            ],
            wait_proc: true,
        });
        self.profile_checked = true;
        TomoyoLoadDecision::CallLoader
    }

    pub const fn done(&self) -> bool {
        self.done
    }
}

pub fn tomoyo_load_policy_decision(
    filename: &str,
    trigger: &str,
    policy_loaded: bool,
    already_done: bool,
    loader_exists: bool,
) -> TomoyoLoadDecision {
    if policy_loaded || already_done {
        TomoyoLoadDecision::AlreadyLoaded
    } else if filename != trigger {
        TomoyoLoadDecision::TriggerMismatch
    } else if !loader_exists {
        TomoyoLoadDecision::LoaderMissing
    } else {
        TomoyoLoadDecision::CallLoader
    }
}

pub fn setup_value<'a>(arg: &'a str, prefix: &str) -> Option<&'a str> {
    arg.strip_prefix(prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tomoyo_policy_loader_flow_matches_linux_source() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/tomoyo/load_policy.c"
        ));
        assert!(source.contains("static const char *tomoyo_loader;"));
        assert!(source.contains("tomoyo_loader_setup"));
        assert!(source.contains("__setup(\"TOMOYO_loader=\", tomoyo_loader_setup);"));
        assert!(source.contains("tomoyo_policy_loader_exists"));
        assert!(source.contains("CONFIG_SECURITY_TOMOYO_POLICY_LOADER"));
        assert!(source.contains("kern_path(tomoyo_loader, LOOKUP_FOLLOW, &path)"));
        assert!(source.contains("static const char *tomoyo_trigger;"));
        assert!(source.contains("__setup(\"TOMOYO_trigger=\", tomoyo_trigger_setup);"));
        assert!(source.contains("CONFIG_SECURITY_TOMOYO_ACTIVATION_TRIGGER"));
        assert!(source.contains("if (tomoyo_policy_loaded || done)"));
        assert!(source.contains("if (strcmp(filename, tomoyo_trigger))"));
        assert!(source.contains("call_usermodehelper(argv[0], argv, envp, UMH_WAIT_PROC);"));
        assert!(source.contains("tomoyo_check_profile();"));

        let mut state = TomoyoLoadPolicy::new(true);
        assert_eq!(state.tomoyo_loader_setup("/custom/loader"), 1);
        assert_eq!(state.tomoyo_trigger_setup("/custom/init"), 1);
        assert_eq!(
            state.tomoyo_load_policy("/bin/sh"),
            TomoyoLoadDecision::TriggerMismatch
        );
        assert_eq!(
            state.tomoyo_load_policy("/custom/init"),
            TomoyoLoadDecision::CallLoader
        );
        assert!(state.done());
        assert!(state.profile_checked);
        assert_eq!(
            state.helper_calls,
            [UsermodeHelperCall {
                path: "/custom/loader".to_string(),
                argv: [Some("/custom/loader".to_string()), None],
                envp: [
                    Some("HOME=/".to_string()),
                    Some("PATH=/sbin:/bin:/usr/sbin:/usr/bin".to_string()),
                    None,
                ],
                wait_proc: true,
            }]
        );
        assert_eq!(
            state.tomoyo_load_policy("/custom/init"),
            TomoyoLoadDecision::AlreadyLoaded
        );

        let mut missing_loader = TomoyoLoadPolicy::new(false);
        assert_eq!(
            missing_loader.tomoyo_load_policy(DEFAULT_TRIGGER_CONFIG),
            TomoyoLoadDecision::LoaderMissing
        );
        assert!(missing_loader.logs[0].contains(DEFAULT_LOADER_CONFIG));

        assert_eq!(
            setup_value("TOMOYO_loader=/sbin/tomoyo-init", LOADER_SETUP_PREFIX),
            Some("/sbin/tomoyo-init")
        );
        assert_eq!(
            tomoyo_load_policy_decision("/sbin/init", "/sbin/init", false, false, true),
            TomoyoLoadDecision::CallLoader
        );
        assert_eq!(
            tomoyo_load_policy_decision("/bin/sh", "/sbin/init", false, false, true),
            TomoyoLoadDecision::TriggerMismatch
        );
        assert_eq!(
            tomoyo_load_policy_decision("/sbin/init", "/sbin/init", true, false, true),
            TomoyoLoadDecision::AlreadyLoaded
        );
        assert_eq!(
            tomoyo_load_policy_decision("/sbin/init", "/sbin/init", false, false, false),
            TomoyoLoadDecision::LoaderMissing
        );
        assert_eq!(
            POLICY_LOADER_ENV,
            ["HOME=/", "PATH=/sbin:/bin:/usr/sbin:/usr/bin"]
        );
    }
}
