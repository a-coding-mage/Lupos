//! linux-parity: partial
//! linux-source: vendor/linux/init
//! test-origin: linux:vendor/linux/init
//! Early boot option parsing and init handoff planning.
//!
//! Mirrors the boot-visible pieces of `vendor/linux/init/main.c`:
//! command-line setup, `init=` / `rdinit=`, unknown option forwarding to
//! init, and the ordered init fallback list. Bootconfig, setup_command_line's
//! in-place splice, and obsolete_checksetup side effects are intentionally
//! deferred.

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::include::uapi::errno::ENOENT;

const DEFAULT_RAMDISK_INIT: &str = "/init";
const DEFAULT_INIT_FALLBACKS: [&str; 4] = ["/sbin/init", "/etc/init", "/bin/init", "/bin/sh"];
pub const CONFIG_DEFAULT_INIT: &str = "";
pub const MAX_INIT_ARGS: usize = 32;
pub const MAX_INIT_ENVS: usize = 32;
const CONSOLE_LOGLEVEL_QUIET: i32 = 4;
const CONSOLE_LOGLEVEL_DEBUG: i32 = 10;
const SYSCTL_ALIASES: [(&str, &str); 4] = [
    (
        "hardlockup_all_cpu_backtrace",
        "kernel.hardlockup_all_cpu_backtrace",
    ),
    ("hung_task_panic", "kernel.hung_task_panic"),
    ("numa_zonelist_order", "vm.numa_zonelist_order"),
    (
        "softlockup_all_cpu_backtrace",
        "kernel.softlockup_all_cpu_backtrace",
    ),
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitrdRange {
    pub start: u64,
    pub size: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootOptions {
    pub raw_command_line: String,
    pub execute_command: Option<String>,
    pub ramdisk_execute_command: Option<String>,
    pub ramdisk_execute_command_set: bool,
    pub root: Option<String>,
    pub rootfstype: Option<String>,
    pub rootflags: Option<String>,
    pub root_readonly: bool,
    pub noinitrd: bool,
    pub initrd: Option<InitrdRange>,
    pub initrdmem: Option<InitrdRange>,
    pub ramdisk_start: u32,
    pub preset_lpj: Option<u64>,
    pub hostname: Option<String>,
    pub initcall_blacklist: Vec<String>,
    pub sysctl_args: Vec<String>,
    pub console_loglevel: Option<i32>,
    pub argv_init: Vec<String>,
    pub envp_init: Vec<String>,
    panic_later: Option<&'static str>,
    panic_param: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitPlan {
    pub candidates: Vec<InitCandidate>,
    pub argv: Vec<String>,
    pub envp: Vec<String>,
    pub rdinit_warn: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitCandidate {
    pub path: String,
    pub kind: InitCandidateKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InitCandidateKind {
    Ramdisk,
    Explicit,
    ConfigDefault,
    Fallback,
}

impl Default for BootOptions {
    fn default() -> Self {
        Self {
            raw_command_line: String::new(),
            execute_command: None,
            ramdisk_execute_command: Some(DEFAULT_RAMDISK_INIT.to_string()),
            ramdisk_execute_command_set: false,
            root: None,
            rootfstype: None,
            rootflags: None,
            root_readonly: true,
            noinitrd: false,
            initrd: None,
            initrdmem: None,
            ramdisk_start: 0,
            preset_lpj: None,
            hostname: None,
            initcall_blacklist: Vec::new(),
            sysctl_args: Vec::new(),
            console_loglevel: None,
            argv_init: vec!["init".to_string()],
            envp_init: vec!["HOME=/".to_string(), "TERM=linux".to_string()],
            panic_later: None,
            panic_param: None,
        }
    }
}

impl BootOptions {
    pub fn parse(cmdline: &str) -> Self {
        let mut options = Self {
            raw_command_line: cmdline.to_string(),
            ..Self::default()
        };
        let mut pass_to_init = false;

        for token in cmdline.split_whitespace() {
            if token == "--" {
                pass_to_init = true;
                continue;
            }
            if pass_to_init {
                options.push_init_arg(token);
                continue;
            }

            if let Some(value) = token.strip_prefix("init=") {
                options.execute_command = Some(value.to_string());
                options.argv_init.truncate(1);
            } else if let Some(value) = token.strip_prefix("rdinit=") {
                options.ramdisk_execute_command = Some(value.to_string());
                options.ramdisk_execute_command_set = true;
                options.argv_init.truncate(1);
            } else if let Some(value) = token.strip_prefix("root=") {
                options.root = Some(value.to_string());
            } else if let Some(value) = token.strip_prefix("rootfstype=") {
                options.rootfstype = Some(value.to_string());
            } else if let Some(value) = token.strip_prefix("rootflags=") {
                options.rootflags = Some(value.to_string());
            } else if token == "ro" {
                options.root_readonly = true;
            } else if token == "rw" {
                options.root_readonly = false;
            } else if token == "noinitrd" {
                options.noinitrd = true;
            } else if let Some(value) = token.strip_prefix("initrd=") {
                options.initrd = parse_initrd_range(value);
            } else if let Some(value) = token.strip_prefix("initrdmem=") {
                options.initrdmem = parse_initrd_range(value);
            } else if let Some(value) = token.strip_prefix("ramdisk_start=") {
                if let Some(parsed) = parse_u64_auto(value) {
                    options.ramdisk_start = parsed as u32;
                }
            } else if let Some(value) = token.strip_prefix("lpj=") {
                options.preset_lpj = parse_u64_auto(value);
            } else if let Some(value) = token.strip_prefix("hostname=") {
                options.hostname = Some(value.to_string());
            } else if let Some(value) = token.strip_prefix("initcall_blacklist=") {
                options.initcall_blacklist = value
                    .split(',')
                    .filter(|name| !name.is_empty())
                    .map(ToString::to_string)
                    .collect();
            } else if token == "quiet" {
                options.console_loglevel = Some(CONSOLE_LOGLEVEL_QUIET);
            } else if token == "debug" {
                options.console_loglevel = Some(CONSOLE_LOGLEVEL_DEBUG);
            } else if let Some(value) = token.strip_prefix("loglevel=") {
                if let Ok(level) = value.parse::<i32>() {
                    options.console_loglevel = Some(level);
                }
            } else {
                options.forward_unknown_bootoption(token);
            }
        }

        options
    }

    pub fn init_plan<F>(&self, mut exists: F) -> InitPlan
    where
        F: FnMut(&str) -> bool,
    {
        let mut candidates = Vec::new();
        let mut rdinit_warn = None;

        if let Some(rdinit) = self.ramdisk_execute_command.as_deref() {
            if exists(rdinit) {
                candidates.push(InitCandidate {
                    path: rdinit.to_string(),
                    kind: InitCandidateKind::Ramdisk,
                });
            } else if self.ramdisk_execute_command_set {
                rdinit_warn = Some(format!(
                    "check access for rdinit={} failed: {}, ignoring",
                    rdinit, -ENOENT
                ));
            }
        }

        if let Some(init) = self.execute_command.as_deref() {
            candidates.push(InitCandidate {
                path: init.to_string(),
                kind: InitCandidateKind::Explicit,
            });
        }
        if !CONFIG_DEFAULT_INIT.is_empty() {
            candidates.push(InitCandidate {
                path: CONFIG_DEFAULT_INIT.to_string(),
                kind: InitCandidateKind::ConfigDefault,
            });
        }
        for fallback in DEFAULT_INIT_FALLBACKS {
            candidates.push(InitCandidate {
                path: fallback.to_string(),
                kind: InitCandidateKind::Fallback,
            });
        }

        InitPlan {
            candidates,
            argv: self.argv_init.clone(),
            envp: self.envp_init.clone(),
            rdinit_warn,
        }
    }

    pub fn needs_prepare_namespace<F>(&self, mut exists: F) -> bool
    where
        F: FnMut(&str) -> bool,
    {
        self.ramdisk_execute_command
            .as_deref()
            .is_some_and(|rdinit| !exists(rdinit))
    }

    pub fn boot_var_overflow(&self) -> Option<(&'static str, &str)> {
        Some((self.panic_later?, self.panic_param.as_deref().unwrap_or("")))
    }

    pub fn console_on_rootfs_ready<F>(&self, exists: F) -> bool
    where
        F: FnOnce(&str) -> bool,
    {
        let _ = self;
        exists("/dev/console")
    }

    fn push_init_arg(&mut self, arg: &str) {
        if self.panic_later.is_some() {
            return;
        }
        if self.argv_init.len() > MAX_INIT_ARGS {
            self.panic_later = Some("init");
            self.panic_param = Some(arg.to_string());
            return;
        }
        self.argv_init.push(arg.to_string());
    }

    fn forward_unknown_bootoption(&mut self, token: &str) {
        if token.starts_with("BOOT_IMAGE=") || token == "kexec" {
            return;
        }

        if let Some(sysctl) = sysctl_alias_arg(token) {
            self.sysctl_args.push(sysctl);
            return;
        }

        if token.contains('.') {
            return;
        }

        if token.contains('=') {
            let key_len = token.find('=').unwrap_or(token.len());
            if let Some(pos) = self.envp_init.iter().position(|entry| {
                entry.as_bytes().get(..key_len + 1) == token.as_bytes().get(..key_len + 1)
            }) {
                self.envp_init[pos] = token.to_string();
            } else {
                if self.panic_later.is_some() {
                    return;
                }
                if self.envp_init.len() > MAX_INIT_ENVS {
                    self.panic_later = Some("env");
                    self.panic_param = Some(token.to_string());
                    return;
                }
                self.envp_init.push(token.to_string());
            }
        } else {
            self.push_init_arg(token);
        }
    }
}

fn sysctl_alias_arg(token: &str) -> Option<String> {
    let (param, value) = token.split_once('=').unwrap_or((token, ""));
    let alias = SYSCTL_ALIASES
        .iter()
        .find_map(|(kernel_param, sysctl_param)| {
            (*kernel_param == param).then_some(*sysctl_param)
        })?;
    if value.is_empty() {
        Some(alias.to_string())
    } else {
        let mut arg = String::with_capacity(alias.len() + 1 + value.len());
        arg.push_str(alias);
        arg.push('=');
        arg.push_str(value);
        Some(arg)
    }
}

fn parse_initrd_range(value: &str) -> Option<InitrdRange> {
    let (start, size) = value.split_once(',')?;
    Some(InitrdRange {
        start: parse_memparse(start)?,
        size: parse_memparse(size)?,
    })
}

fn parse_memparse(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (digits, multiplier) = match trimmed.as_bytes().last().copied() {
        Some(b'K') | Some(b'k') => (&trimmed[..trimmed.len() - 1], 1024u64),
        Some(b'M') | Some(b'm') => (&trimmed[..trimmed.len() - 1], 1024u64 * 1024),
        Some(b'G') | Some(b'g') => (&trimmed[..trimmed.len() - 1], 1024u64 * 1024 * 1024),
        _ => (trimmed, 1),
    };

    parse_u64_auto(digits)?.checked_mul(multiplier)
}

fn parse_u64_auto(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16).ok()
    } else if let Some(octal) = trimmed.strip_prefix('0') {
        if octal.is_empty() {
            Some(0)
        } else {
            u64::from_str_radix(octal, 8).ok()
        }
    } else {
        trimmed.parse::<u64>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_linux_init_and_rdinit_options() {
        let options = BootOptions::parse(
            "BOOT_IMAGE=/boot/lupos rdinit=/linuxrc init=/bin/sh root=/dev/vda rootfstype=ext4 rootflags=noatime rw quiet -- single debug",
        );
        assert_eq!(options.ramdisk_execute_command.as_deref(), Some("/linuxrc"));
        assert!(options.ramdisk_execute_command_set);
        assert_eq!(options.execute_command.as_deref(), Some("/bin/sh"));
        assert_eq!(options.root.as_deref(), Some("/dev/vda"));
        assert_eq!(options.rootfstype.as_deref(), Some("ext4"));
        assert_eq!(options.rootflags.as_deref(), Some("noatime"));
        assert!(!options.root_readonly);
        assert_eq!(options.argv_init, vec!["init", "single", "debug"]);
        assert!(!options.argv_init.iter().any(|arg| arg == "quiet"));
        assert_eq!(options.console_loglevel, Some(CONSOLE_LOGLEVEL_QUIET));
        assert!(options.initcall_blacklist.is_empty());
        assert!(!options.envp_init.iter().any(|env| env.starts_with("root=")));
    }

    #[test]
    fn init_plan_prefers_existing_rdinit_then_fallbacks() {
        let options = BootOptions::parse("rdinit=/init");
        let plan = options.init_plan(|path| path == "/init");
        assert_eq!(plan.candidates[0].path, "/init");
        assert_eq!(plan.candidates[0].kind, InitCandidateKind::Ramdisk);
        assert!(
            plan.candidates
                .iter()
                .any(|candidate| candidate.path == "/sbin/init"
                    && candidate.kind == InitCandidateKind::Fallback)
        );

        let missing = options.init_plan(|_| false);
        assert_eq!(missing.candidates[0].path, "/sbin/init");
        assert_eq!(
            missing.rdinit_warn.as_deref(),
            Some("check access for rdinit=/init failed: -2, ignoring")
        );
        assert!(options.needs_prepare_namespace(|_| false));
    }

    #[test]
    fn init_plan_records_explicit_and_config_order() {
        let options = BootOptions::parse("init=/bin/custom");
        let plan = options.init_plan(|_| false);
        assert_eq!(plan.candidates[0].path, "/bin/custom");
        assert_eq!(plan.candidates[0].kind, InitCandidateKind::Explicit);
        assert_eq!(plan.candidates[1].path, "/sbin/init");
        assert_eq!(plan.candidates[1].kind, InitCandidateKind::Fallback);
    }

    #[test]
    fn parses_initrd_ranges_and_deprecated_ramdisk_start() {
        let options =
            BootOptions::parse("noinitrd initrd=0x100000,8M initrdmem=2M,4096 ramdisk_start=010");
        assert!(options.noinitrd);
        assert_eq!(
            options.initrd,
            Some(InitrdRange {
                start: 0x100000,
                size: 8 * 1024 * 1024
            })
        );
        assert_eq!(
            options.initrdmem,
            Some(InitrdRange {
                start: 2 * 1024 * 1024,
                size: 4096
            })
        );
        assert_eq!(options.ramdisk_start, 8);
    }

    #[test]
    fn unknown_options_follow_linux_init_forwarding_shape() {
        let options = BootOptions::parse(
            "TERM=vt100 TERMX=1 foo root=/dev/vda rw bar.baz=1 kexec BOOT_IMAGE=/x hung_task_panic=1",
        );
        assert!(options.argv_init.iter().any(|arg| arg == "foo"));
        assert!(options.envp_init.iter().any(|env| env == "TERM=vt100"));
        assert!(options.envp_init.iter().any(|env| env == "TERMX=1"));
        assert!(!options.argv_init.iter().any(|arg| arg == "kexec"));
        assert!(!options.argv_init.iter().any(|arg| arg == "rw"));
        assert!(!options.envp_init.iter().any(|env| env.starts_with("root=")));
        assert!(!options.envp_init.iter().any(|env| env == "bar.baz=1"));
        assert_eq!(options.sysctl_args, vec!["kernel.hung_task_panic=1"]);
    }

    #[test]
    fn loglevel_options_follow_linux_early_params() {
        assert_eq!(BootOptions::parse("debug").console_loglevel, Some(10));
        assert_eq!(BootOptions::parse("quiet").console_loglevel, Some(4));
        assert_eq!(BootOptions::parse("loglevel=6").console_loglevel, Some(6));
    }

    #[test]
    fn parses_initcall_blacklist_like_linux_setup() {
        let options = BootOptions::parse("initcall_blacklist=taskstats_init,zswap_init");
        assert_eq!(
            options.initcall_blacklist,
            vec!["taskstats_init", "zswap_init"]
        );
    }

    #[test]
    fn boot_var_overflow_records_linux_panic_later() {
        let mut cmdline = String::new();
        for i in 0..=MAX_INIT_ARGS + 1 {
            if i != 0 {
                cmdline.push(' ');
            }
            cmdline.push_str("arg");
        }
        let options = BootOptions::parse(&cmdline);
        assert_eq!(options.boot_var_overflow(), Some(("init", "arg")));
    }
}
