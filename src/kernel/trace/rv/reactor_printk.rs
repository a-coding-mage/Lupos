//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/reactor_printk.c
//! test-origin: linux:vendor/linux/kernel/trace/rv/reactor_printk.c
//! `printk` reactor — prints a `pr_err()` line when a monitor violates.
//!
//! Ref: vendor/linux/kernel/trace/rv/reactor_printk.c

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use spin::Mutex;

static MESSAGES: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub const REACTOR_NAME: &str = "printk";
pub const REACTOR_DESCRIPTION: &str = "prints the exception msg to the kernel message log.";
pub const REACTION_SYMBOL: &str = "rv_printk_reaction";
pub const REGISTER_SYMBOL: &str = "register_react_printk";
pub const UNREGISTER_SYMBOL: &str = "unregister_react_printk";
pub const MODULE_INIT_HOOK: &str = "module_init(register_react_printk)";
pub const MODULE_EXIT_HOOK: &str = "module_exit(unregister_react_printk)";
pub const MODULE_AUTHOR: &str = "Daniel Bristot de Oliveira";
pub const MODULE_DESCRIPTION: &str = "printk rv reactor: printk if an exception is hit.";

pub type RvReaction = fn(&str, &[&str]);

#[derive(Clone, Copy, Debug)]
pub struct RvReactor {
    pub name: &'static str,
    pub description: &'static str,
    pub react: RvReaction,
}

pub static RV_PRINTK: RvReactor = RvReactor {
    name: REACTOR_NAME,
    description: REACTOR_DESCRIPTION,
    react: rv_printk_reaction,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct RvReactorRegistry {
    pub registered: Option<&'static RvReactor>,
}

pub fn rv_printk_reaction(msg: &str, args: &[&str]) {
    vprintk_deferred(msg, args);
}

pub fn vprintk_deferred(msg: &str, args: &[&str]) {
    let mut rendered = String::from(msg);
    if !args.is_empty() {
        rendered.push(' ');
        rendered.push_str(&alloc::format!("{args:?}"));
    }
    MESSAGES.lock().push(rendered);
}

pub fn react(monitor: &str) {
    rv_printk_reaction("rv: %s violated", &[monitor]);
}

pub fn register_react_printk(registry: &mut RvReactorRegistry) -> i32 {
    registry.registered = Some(&RV_PRINTK);
    0
}

pub fn unregister_react_printk(registry: &mut RvReactorRegistry) {
    if let Some(registered) = registry.registered {
        if core::ptr::eq(registered, &RV_PRINTK) {
            registry.registered = None;
        }
    }
}

pub fn drain() -> Vec<String> {
    core::mem::take(&mut *MESSAGES.lock())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn printk_reactor_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/reactor_printk.c"
        ));
        assert!(source.contains("#include <linux/ftrace.h>"));
        assert!(source.contains("#include <linux/tracepoint.h>"));
        assert!(source.contains("#include <linux/kernel.h>"));
        assert!(source.contains("#include <linux/module.h>"));
        assert!(source.contains("#include <linux/init.h>"));
        assert!(source.contains("#include <linux/rv.h>"));
        assert!(source.contains("__printf(1, 0) static void rv_printk_reaction"));
        assert!(source.contains("vprintk_deferred(msg, args);"));
        assert!(source.contains("static struct rv_reactor rv_printk = {"));
        assert!(source.contains(".name = \"printk\""));
        assert!(
            source
                .contains(".description = \"prints the exception msg to the kernel message log.\"")
        );
        assert!(source.contains(".react = rv_printk_reaction"));
        assert!(source.contains("rv_register_reactor(&rv_printk);"));
        assert!(source.contains("return 0;"));
        assert!(source.contains("rv_unregister_reactor(&rv_printk);"));
        assert!(source.contains("module_init(register_react_printk);"));
        assert!(source.contains("module_exit(unregister_react_printk);"));
        assert!(source.contains("MODULE_AUTHOR(\"Daniel Bristot de Oliveira\")"));
        assert!(
            source.contains(
                "MODULE_DESCRIPTION(\"printk rv reactor: printk if an exception is hit.\")"
            )
        );

        assert_eq!(RV_PRINTK.name, "printk");
        assert_eq!(RV_PRINTK.description, REACTOR_DESCRIPTION);
        assert_eq!(RV_PRINTK.react as usize, rv_printk_reaction as usize);
        assert_eq!(MODULE_INIT_HOOK, "module_init(register_react_printk)");
        assert_eq!(MODULE_EXIT_HOOK, "module_exit(unregister_react_printk)");
        assert_eq!(MODULE_AUTHOR, "Daniel Bristot de Oliveira");
        assert_eq!(
            MODULE_DESCRIPTION,
            "printk rv reactor: printk if an exception is hit."
        );
    }

    #[test]
    fn react_records_message() {
        react("test_mon");
        let m = drain();
        assert!(m.iter().any(|s| s.contains("rv: %s violated")));
        assert!(m.iter().any(|s| s.contains("test_mon")));
    }

    #[test]
    fn register_and_unregister_printk_reactor() {
        let mut registry = RvReactorRegistry::default();
        assert_eq!(register_react_printk(&mut registry), 0);
        assert!(matches!(
            registry.registered,
            Some(registered) if core::ptr::eq(registered, &RV_PRINTK)
        ));
        unregister_react_printk(&mut registry);
        assert!(registry.registered.is_none());
    }
}
