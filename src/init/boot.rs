//! linux-parity: complete
//! linux-source: vendor/linux/init
//! test-origin: linux:vendor/linux/init
//! Early boot option parsing and init handoff planning.
//!
//! Mirrors the boot-visible pieces of `vendor/linux/init/main.c`:
//! command-line setup, `init=` / `rdinit=`, unknown option forwarding to
//! init, and the ordered init fallback list.

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

const DEFAULT_RAMDISK_INIT: &str = "/init";
const DEFAULT_INIT_FALLBACKS: [&str; 4] = ["/sbin/init", "/etc/init", "/bin/init", "/bin/sh"];

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
    pub argv_init: Vec<String>,
    pub envp_init: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitPlan {
    pub candidates: Vec<String>,
    pub argv: Vec<String>,
    pub envp: Vec<String>,
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
            argv_init: vec!["init".to_string()],
            envp_init: vec!["HOME=/".to_string(), "TERM=linux".to_string()],
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

        if let Some(rdinit) = self.ramdisk_execute_command.as_deref() {
            if exists(rdinit) {
                candidates.push(rdinit.to_string());
            }
        }

        if let Some(init) = self.execute_command.as_deref() {
            candidates.push(init.to_string());
        } else {
            for fallback in DEFAULT_INIT_FALLBACKS {
                candidates.push(fallback.to_string());
            }
        }

        InitPlan {
            candidates,
            argv: self.argv_init.clone(),
            envp: self.envp_init.clone(),
        }
    }

    pub fn console_on_rootfs_ready<F>(&self, exists: F) -> bool
    where
        F: FnOnce(&str) -> bool,
    {
        let _ = self;
        exists("/dev/console")
    }

    fn push_init_arg(&mut self, arg: &str) {
        self.argv_init.push(arg.to_string());
    }

    fn forward_unknown_bootoption(&mut self, token: &str) {
        if token.starts_with("BOOT_IMAGE=")
            || token == "kexec"
            || token == "quiet"
            || token.contains('.')
        {
            return;
        }

        if token.contains('=') {
            let key_len = token.find('=').unwrap_or(token.len());
            if let Some(pos) = self.envp_init.iter().position(|entry| {
                entry.as_bytes().get(..key_len + 1) == token.as_bytes().get(..key_len + 1)
            }) {
                self.envp_init[pos] = token.to_string();
            } else {
                self.envp_init.push(token.to_string());
            }
        } else {
            self.push_init_arg(token);
        }
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
        assert!(!options.envp_init.iter().any(|env| env.starts_with("root=")));
    }

    #[test]
    fn init_plan_prefers_existing_rdinit_then_fallbacks() {
        let options = BootOptions::parse("rdinit=/init");
        let plan = options.init_plan(|path| path == "/init");
        assert_eq!(plan.candidates[0], "/init");
        assert!(plan.candidates.iter().any(|path| path == "/sbin/init"));

        let missing = options.init_plan(|_| false);
        assert_eq!(missing.candidates[0], "/sbin/init");
    }

    #[test]
    fn parses_initrd_ranges_and_deprecated_ramdisk_start() {
        let options =
            BootOptions::parse("noinitrd initrd=0x100000,8M initrdmem=2M,4096 ramdisk_start=4");
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
        assert_eq!(options.ramdisk_start, 4);
    }

    #[test]
    fn unknown_options_follow_linux_init_forwarding_shape() {
        let options =
            BootOptions::parse("TERM=vt100 foo root=/dev/vda rw bar.baz=1 kexec BOOT_IMAGE=/x");
        assert!(options.argv_init.iter().any(|arg| arg == "foo"));
        assert!(options.envp_init.iter().any(|env| env == "TERM=vt100"));
        assert!(!options.argv_init.iter().any(|arg| arg == "kexec"));
        assert!(!options.argv_init.iter().any(|arg| arg == "rw"));
        assert!(!options.envp_init.iter().any(|env| env.starts_with("root=")));
        assert!(!options.envp_init.iter().any(|env| env == "bar.baz=1"));
    }
}
