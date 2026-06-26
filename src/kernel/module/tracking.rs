//! linux-parity: complete
//! linux-source: vendor/linux/kernel/module/tracking.c
//! test-origin: linux:vendor/linux/kernel/module/tracking.c
//! Tracking for unloaded tainted modules.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;

use crate::include::uapi::errno::ENOMEM;

pub const MODULE_NAME_LEN: usize = 64 - core::mem::size_of::<usize>();
pub const TAINT_FLAGS_COUNT: usize = 20;
pub const MODULE_FLAGS_BUF_SIZE: usize = TAINT_FLAGS_COUNT + 4;

pub const TAINT_TRUE_CHARS: [char; TAINT_FLAGS_COUNT] = [
    'P', 'F', 'S', 'R', 'M', 'B', 'U', 'D', 'A', 'W', 'C', 'I', 'O', 'E', 'L', 'K', 'X', 'T', 'N',
    'J',
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleTaint {
    pub name: String,
    pub taints: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModUnloadTaint {
    pub name: String,
    pub taints: u64,
    pub count: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UnloadedTaintedModules {
    entries: Vec<ModUnloadTaint>,
}

impl UnloadedTaintedModules {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn try_add_tainted_module(&mut self, module: ModuleTaint) {
        let _ = self.try_add_tainted_module_result(module, true);
    }

    pub fn try_add_tainted_module_result(
        &mut self,
        module: ModuleTaint,
        allocation_available: bool,
    ) -> Result<(), i32> {
        if module.taints == 0 {
            return Ok(());
        }
        if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|entry| entry.name == module.name && entry.taints & module.taints != 0)
        {
            existing.count = existing.count.saturating_add(1);
            return Ok(());
        }

        if !allocation_available {
            return Err(-ENOMEM);
        }

        self.entries.push(ModUnloadTaint {
            name: truncate_module_name(&module.name),
            taints: module.taints,
            count: 1,
        });
        Ok(())
    }

    pub fn print_unloaded_tainted_modules(&self) -> Option<String> {
        if self.entries.is_empty() {
            return None;
        }
        let mut out = String::from("Unloaded tainted modules:");
        for entry in &self.entries {
            let _ = write!(
                &mut out,
                " {}({}):{}",
                entry.name,
                module_flags_taint(entry.taints),
                entry.count
            );
        }
        Some(out)
    }

    pub fn debugfs_rows(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|entry| {
                format!(
                    "{} ({}) {}",
                    entry.name,
                    module_flags_taint(entry.taints),
                    entry.count
                )
            })
            .collect()
    }

    pub fn entries(&self) -> &[ModUnloadTaint] {
        &self.entries
    }
}

pub fn module_flags_taint(taints: u64) -> String {
    let mut out = String::new();
    for (idx, flag) in TAINT_TRUE_CHARS.iter().enumerate() {
        if taints & (1u64 << idx) != 0 {
            out.push(*flag);
        }
    }
    out
}

pub fn truncate_module_name(name: &str) -> String {
    let max = MODULE_NAME_LEN.saturating_sub(1);
    if name.len() <= max {
        return String::from(name);
    }
    let mut end = 0;
    for (idx, ch) in name.char_indices() {
        let next = idx + ch.len_utf8();
        if next > max {
            break;
        }
        end = next;
    }
    String::from(&name[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module(name: &str, taints: u64) -> ModuleTaint {
        ModuleTaint {
            name: String::from(name),
            taints,
        }
    }

    #[test]
    fn unloaded_tainted_module_tracking_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/module/tracking.c"
        ));
        let internal = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/module/internal.h"
        ));
        let panic = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/panic.c"
        ));

        assert!(source.contains("static LIST_HEAD(unloaded_tainted_modules);"));
        assert!(source.contains("if (!mod->taints)"));
        assert!(source.contains("!strcmp(mod_taint->name, mod->name) &&"));
        assert!(source.contains("mod_taint->taints & mod->taints"));
        assert!(source.contains("mod_taint->count++;"));
        assert!(source.contains("mod_taint = kmalloc_obj(*mod_taint);"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("strscpy(mod_taint->name, mod->name, MODULE_NAME_LEN);"));
        assert!(source.contains("list_add_rcu(&mod_taint->list, &unloaded_tainted_modules);"));
        assert!(source.contains("printk(KERN_DEFAULT \"Unloaded tainted modules:\");"));
        assert!(source.contains("module_flags_taint(mod_taint->taints, buf);"));
        assert!(source.contains("seq_list_start_rcu(&unloaded_tainted_modules, *pos);"));
        assert!(source.contains("seq_list_next_rcu(p, &unloaded_tainted_modules, pos);"));
        assert!(source.contains("rcu_read_unlock();"));
        assert!(source.contains("seq_printf(m, \"%s (%s) %llu\""));
        assert!(source.contains("debugfs_create_file(\"unloaded_tainted\", 0444"));
        assert!(source.contains("module_init(unloaded_tainted_modules_init);"));
        assert!(internal.contains("struct mod_unload_taint"));
        assert!(panic.contains("TAINT_FLAG(PROPRIETARY_MODULE"));

        let mut tracker = UnloadedTaintedModules::new();
        tracker.try_add_tainted_module(module("clean", 0));
        assert!(tracker.entries().is_empty());

        tracker.try_add_tainted_module(module("netdrv", 1 << 0));
        tracker.try_add_tainted_module(module("netdrv", 1 << 0));
        tracker.try_add_tainted_module(module("netdrv", 1 << 13));

        assert_eq!(tracker.entries().len(), 2);
        assert_eq!(tracker.entries()[0].count, 2);
        assert_eq!(tracker.entries()[0].taints, 1);
        assert_eq!(tracker.entries()[1].taints, 1 << 13);
        assert_eq!(module_flags_taint((1 << 0) | (1 << 13)), "PE");
        assert_eq!(
            tracker.print_unloaded_tainted_modules().unwrap(),
            "Unloaded tainted modules: netdrv(P):2 netdrv(E):1"
        );
        assert_eq!(
            tracker.debugfs_rows(),
            [String::from("netdrv (P) 2"), String::from("netdrv (E) 1")]
        );
    }

    #[test]
    fn unloaded_tainted_module_allocation_failure_matches_linux_return() {
        let mut tracker = UnloadedTaintedModules::new();
        assert_eq!(
            tracker.try_add_tainted_module_result(module("bad", 1), false),
            Err(-ENOMEM)
        );
        assert!(tracker.entries().is_empty());

        tracker
            .try_add_tainted_module_result(module("bad", 1), true)
            .unwrap();
        assert_eq!(
            tracker.try_add_tainted_module_result(module("bad", 1), false),
            Ok(())
        );
        assert_eq!(tracker.entries()[0].count, 2);
    }

    #[test]
    fn module_name_truncation_uses_linux_module_name_capacity() {
        let long = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let truncated = truncate_module_name(long);
        assert_eq!(MODULE_NAME_LEN, 64 - core::mem::size_of::<usize>());
        assert_eq!(truncated.len(), MODULE_NAME_LEN - 1);
        assert!(long.starts_with(&truncated));
    }
}
