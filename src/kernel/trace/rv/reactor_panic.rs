//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/reactor_panic.c
//! test-origin: linux:vendor/linux/kernel/trace/rv/reactor_panic.c
//! `panic` reactor — fires `panic()` when an RV monitor reports a violation.
//!
//! Ref: vendor/linux/kernel/trace/rv/reactor_panic.c

use core::sync::atomic::{AtomicU64, Ordering};

pub static PANIC_INVOCATIONS: AtomicU64 = AtomicU64::new(0);

pub const REACTOR_NAME: &str = "panic";
pub const REACTOR_DESCRIPTION: &str = "panic the system if an exception is found.";
pub const REACTION_SYMBOL: &str = "rv_panic_reaction";
pub const REGISTER_SYMBOL: &str = "register_react_panic";
pub const UNREGISTER_SYMBOL: &str = "unregister_react_panic";
pub const MODULE_INIT_HOOK: &str = "module_init(register_react_panic)";
pub const MODULE_EXIT_HOOK: &str = "module_exit(unregister_react_panic)";
pub const MODULE_AUTHOR: &str = "Daniel Bristot de Oliveira";
pub const MODULE_DESCRIPTION: &str = "panic rv reactor: panic if an exception is found.";

pub type RvReaction = fn(&str, &[&str]) -> !;

#[derive(Clone, Copy, Debug)]
pub struct RvReactor {
    pub name: &'static str,
    pub description: &'static str,
    pub react: RvReaction,
}

pub static RV_PANIC: RvReactor = RvReactor {
    name: REACTOR_NAME,
    description: REACTOR_DESCRIPTION,
    react: rv_panic_reaction,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct RvReactorRegistry {
    pub registered: Option<&'static RvReactor>,
}

/// Rust analogue of Linux `vpanic(msg, args)`: this reaction does not return.
pub fn rv_panic_reaction(msg: &str, args: &[&str]) -> ! {
    PANIC_INVOCATIONS.fetch_add(1, Ordering::AcqRel);

    if args.is_empty() {
        panic!("{}", msg);
    }

    panic!("{} {:?}", msg, args);
}

pub fn react(monitor: &str) -> ! {
    rv_panic_reaction(monitor, &[])
}

pub fn register_react_panic(registry: &mut RvReactorRegistry) -> i32 {
    registry.registered = Some(&RV_PANIC);
    0
}

pub fn unregister_react_panic(registry: &mut RvReactorRegistry) {
    if let Some(registered) = registry.registered {
        if core::ptr::eq(registered, &RV_PANIC) {
            registry.registered = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    extern crate std;
    use std::panic::catch_unwind;

    #[test]
    fn panic_reaction_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/reactor_panic.c"
        ));
        assert!(source.contains("#include <linux/ftrace.h>"));
        assert!(source.contains("#include <linux/tracepoint.h>"));
        assert!(source.contains("#include <linux/kernel.h>"));
        assert!(source.contains("#include <linux/module.h>"));
        assert!(source.contains("#include <linux/init.h>"));
        assert!(source.contains("#include <linux/rv.h>"));
        assert!(source.contains("__printf(1, 0) static void rv_panic_reaction"));
        assert!(source.contains("vpanic(msg, args);"));
        assert!(source.contains("static struct rv_reactor rv_panic = {"));
        assert!(source.contains(".name = \"panic\""));
        assert!(source.contains(".description = \"panic the system if an exception is found.\""));
        assert!(source.contains(".react = rv_panic_reaction"));
        assert!(source.contains("rv_register_reactor(&rv_panic);"));
        assert!(source.contains("return 0;"));
        assert!(source.contains("rv_unregister_reactor(&rv_panic);"));
        assert!(source.contains("module_init(register_react_panic);"));
        assert!(source.contains("module_exit(unregister_react_panic);"));
        assert!(source.contains("MODULE_AUTHOR(\"Daniel Bristot de Oliveira\")"));
        assert!(
            source.contains(
                "MODULE_DESCRIPTION(\"panic rv reactor: panic if an exception is found.\")"
            )
        );

        assert_eq!(RV_PANIC.name, "panic");
        assert_eq!(RV_PANIC.description, REACTOR_DESCRIPTION);
        assert_eq!(RV_PANIC.react as usize, rv_panic_reaction as usize);
        assert_eq!(MODULE_INIT_HOOK, "module_init(register_react_panic)");
        assert_eq!(MODULE_EXIT_HOOK, "module_exit(unregister_react_panic)");
        assert_eq!(MODULE_AUTHOR, "Daniel Bristot de Oliveira");
    }

    #[test]
    fn react_panics_like_vpanic() {
        let args = ["cpu0"];
        let reaction = catch_unwind(|| (RV_PANIC.react)("exception in %s", &args));
        assert!(reaction.is_err());

        let reaction = catch_unwind(|| react("test"));
        assert!(reaction.is_err());
    }

    #[test]
    fn register_and_unregister_panic_reactor() {
        let mut registry = RvReactorRegistry::default();
        assert_eq!(register_react_panic(&mut registry), 0);
        assert!(matches!(
            registry.registered,
            Some(registered) if core::ptr::eq(registered, &RV_PANIC)
        ));
        unregister_react_panic(&mut registry);
        assert!(registry.registered.is_none());
    }
}
