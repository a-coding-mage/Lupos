//! linux-parity: complete
//! linux-source: vendor/linux/net/9p/mod.c
//! test-origin: linux:vendor/linux/net/9p/mod.c
//! 9P transport helpers.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

pub mod trans_common;

pub const P9_DEBUG_ERROR: u32 = 1 << 0;
pub const P9_DEBUG_9P: u32 = 1 << 2;
pub const P9_DEBUG_VFS: u32 = 1 << 3;
pub const P9_DEBUG_CONV: u32 = 1 << 4;
pub const P9_DEBUG_MUX: u32 = 1 << 5;
pub const P9_DEBUG_TRANS: u32 = 1 << 6;
pub const P9_DEBUG_SLABS: u32 = 1 << 7;
pub const P9_DEBUG_FCALL: u32 = 1 << 8;
pub const P9_DEBUG_FID: u32 = 1 << 9;
pub const P9_DEBUG_PKT: u32 = 1 << 10;
pub const P9_DEBUG_FSC: u32 = 1 << 11;
pub const P9_DEBUG_VPKT: u32 = 1 << 12;
pub const P9_DEBUG_CACHE: u32 = 1 << 13;
pub const P9_DEBUG_MMAP: u32 = 1 << 14;

pub const V9FS_DEFAULT_TRANSPORTS: [&str; 6] = ["virtio", "tcp", "fd", "unix", "xen", "rdma"];
pub const MODULE_DESCRIPTION: &str = "Plan 9 Resource Sharing Support (9P2000)";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHORS: [&str; 3] = [
    "Latchesar Ionkov <lucho@ionkov.net>",
    "Eric Van Hensbergen <ericvh@gmail.com>",
    "Ron Minnich <rminnich@lanl.gov>",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct P9DebugEvent {
    pub level: u32,
    pub line_prefix: String,
}

pub fn p9_debug_event(
    p9_debug_level: u32,
    level: u32,
    func: &str,
    pid: i32,
) -> Option<P9DebugEvent> {
    if (p9_debug_level & level) != level {
        return None;
    }
    let line_prefix = if level == P9_DEBUG_9P {
        format!("({pid:08})")
    } else {
        format!("-- {func} ({pid})")
    };
    Some(P9DebugEvent { level, line_prefix })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct P9TransModule {
    pub name: &'static str,
    pub is_default: bool,
    pub owner_available: bool,
    pub module_gets: usize,
}

impl P9TransModule {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            is_default: false,
            owner_available: true,
            module_gets: 0,
        }
    }

    pub const fn default(name: &'static str) -> Self {
        Self {
            name,
            is_default: true,
            owner_available: true,
            module_gets: 0,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct V9fsTransportRegistry {
    transports: Vec<P9TransModule>,
    pub requested_modules: Vec<String>,
    pub lock_depth: usize,
}

impl V9fsTransportRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn transports(&self) -> &[P9TransModule] {
        &self.transports
    }
}

pub fn v9fs_register_trans(registry: &mut V9fsTransportRegistry, module: P9TransModule) {
    registry.lock_depth += 1;
    registry.transports.push(module);
    registry.lock_depth -= 1;
}

pub fn v9fs_unregister_trans(registry: &mut V9fsTransportRegistry, name: &str) {
    registry.lock_depth += 1;
    if let Some(index) = registry
        .transports
        .iter()
        .position(|module| module.name.as_bytes() == name.as_bytes())
    {
        registry.transports.remove(index);
    }
    registry.lock_depth -= 1;
}

fn p9_try_module_get(module: &mut P9TransModule) -> bool {
    if !module.owner_available {
        return false;
    }
    module.module_gets += 1;
    true
}

fn p9_get_trans_by_name_locked(
    registry: &mut V9fsTransportRegistry,
    name: &str,
) -> Option<P9TransModule> {
    registry.lock_depth += 1;
    let mut found = None;
    for module in &mut registry.transports {
        if module.name.as_bytes() == name.as_bytes() && p9_try_module_get(module) {
            found = Some(module.clone());
            break;
        }
    }
    registry.lock_depth -= 1;
    found
}

pub fn v9fs_get_trans_by_name(
    registry: &mut V9fsTransportRegistry,
    name: &str,
    config_modules: bool,
) -> Option<P9TransModule> {
    let mut found = p9_get_trans_by_name_locked(registry, name);
    if found.is_none() && config_modules {
        registry.requested_modules.push(format!("9p-{name}"));
        found = p9_get_trans_by_name_locked(registry, name);
    }
    found
}

pub fn v9fs_get_default_trans(
    registry: &mut V9fsTransportRegistry,
    config_modules: bool,
) -> Option<P9TransModule> {
    registry.lock_depth += 1;
    let mut found = None;
    for module in &mut registry.transports {
        if module.is_default && p9_try_module_get(module) {
            found = Some(module.clone());
            break;
        }
    }
    if found.is_none() {
        for module in &mut registry.transports {
            if p9_try_module_get(module) {
                found = Some(module.clone());
                break;
            }
        }
    }
    registry.lock_depth -= 1;

    for name in V9FS_DEFAULT_TRANSPORTS {
        if found.is_some() {
            break;
        }
        found = v9fs_get_trans_by_name(registry, name, config_modules);
    }
    found
}

pub fn v9fs_put_trans(module: Option<&mut P9TransModule>) {
    if let Some(module) = module {
        module.module_gets = module.module_gets.saturating_sub(1);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct P9InitReport {
    pub ret: i32,
    pub p9_client_init_called: bool,
    pub p9_error_init_called: bool,
    pub info_log: Option<&'static str>,
}

pub fn init_p9(p9_client_init_ret: i32) -> P9InitReport {
    if p9_client_init_ret != 0 {
        return P9InitReport {
            ret: p9_client_init_ret,
            p9_client_init_called: true,
            p9_error_init_called: false,
            info_log: None,
        };
    }
    P9InitReport {
        ret: 0,
        p9_client_init_called: true,
        p9_error_init_called: true,
        info_log: Some("Installing 9P2000 support"),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct P9ExitReport {
    pub p9_client_exit_called: bool,
    pub info_log: &'static str,
}

pub fn exit_p9() -> P9ExitReport {
    P9ExitReport {
        p9_client_exit_called: true,
        info_log: "Unloading 9P2000 support",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn debug_gating_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/9p/mod.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/net/9p/9p.h"
        ));
        assert!(source.contains("unsigned int p9_debug_level;"));
        assert!(source.contains("module_param_named(debug, p9_debug_level, uint, 0);"));
        assert!(source.contains("if ((p9_debug_level & level) != level)"));
        assert!(source.contains("if (level == P9_DEBUG_9P)"));
        assert!(source.contains("pr_notice(\"(%8.8d) %pV\""));
        assert!(source.contains("pr_notice(\"-- %s (%d): %pV\""));
        assert!(header.contains("P9_DEBUG_9P =\t\t(1<<2)"));
        assert!(header.contains("P9_DEBUG_MMAP"));
        assert!(header.contains("(1<<14)"));

        assert_eq!(P9_DEBUG_9P, 1 << 2);
        assert_eq!(P9_DEBUG_MMAP, 1 << 14);
        assert_eq!(p9_debug_event(0, P9_DEBUG_9P, "walk", 7), None);
        assert_eq!(
            p9_debug_event(P9_DEBUG_9P, P9_DEBUG_9P, "walk", 7).unwrap(),
            P9DebugEvent {
                level: P9_DEBUG_9P,
                line_prefix: String::from("(00000007)"),
            }
        );
        assert_eq!(
            p9_debug_event(P9_DEBUG_TRANS, P9_DEBUG_TRANS, "create", 9).unwrap(),
            P9DebugEvent {
                level: P9_DEBUG_TRANS,
                line_prefix: String::from("-- create (9)"),
            }
        );
    }

    #[test]
    fn transport_registration_lookup_and_put_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/9p/mod.c"
        ));
        assert!(source.contains("static DEFINE_SPINLOCK(v9fs_trans_lock);"));
        assert!(source.contains("static LIST_HEAD(v9fs_trans_list);"));
        assert!(source.contains("list_add_tail(&m->list, &v9fs_trans_list);"));
        assert!(source.contains("list_del_init(&m->list);"));
        assert!(source.contains("strcmp(t->name, s) == 0"));
        assert!(source.contains("try_module_get(t->owner)"));
        assert!(source.contains("request_module(\"9p-%s\", s);"));
        assert!(source.contains("module_put(m->owner);"));

        let mut registry = V9fsTransportRegistry::new();
        v9fs_register_trans(&mut registry, P9TransModule::new("tcp"));
        v9fs_register_trans(
            &mut registry,
            P9TransModule {
                owner_available: false,
                ..P9TransModule::new("fd")
            },
        );

        let tcp = v9fs_get_trans_by_name(&mut registry, "tcp", false).expect("tcp");
        assert_eq!(tcp.name, "tcp");
        assert_eq!(registry.transports()[0].module_gets, 1);
        assert_eq!(v9fs_get_trans_by_name(&mut registry, "fd", true), None);
        assert_eq!(registry.requested_modules, vec![String::from("9p-fd")]);
        assert_eq!(registry.lock_depth, 0);

        let mut held = registry.transports()[0].clone();
        v9fs_put_trans(Some(&mut held));
        assert_eq!(held.module_gets, 0);

        v9fs_unregister_trans(&mut registry, "tcp");
        assert!(
            registry
                .transports()
                .iter()
                .all(|module| module.name != "tcp")
        );
    }

    #[test]
    fn default_transport_selection_matches_linux_ordering() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/9p/mod.c"
        ));
        assert!(source.contains("static const char * const v9fs_default_transports[]"));
        assert!(source.contains("\"virtio\", \"tcp\", \"fd\", \"unix\", \"xen\", \"rdma\""));
        assert!(source.contains("if (t->def && try_module_get(t->owner))"));
        assert!(source.contains("if (!found)"));
        assert!(
            source.contains("for (i = 0; !found && i < ARRAY_SIZE(v9fs_default_transports); i++)")
        );

        assert_eq!(
            V9FS_DEFAULT_TRANSPORTS,
            ["virtio", "tcp", "fd", "unix", "xen", "rdma"]
        );

        let mut registry = V9fsTransportRegistry::new();
        v9fs_register_trans(&mut registry, P9TransModule::new("tcp"));
        v9fs_register_trans(&mut registry, P9TransModule::default("virtio"));
        assert_eq!(
            v9fs_get_default_trans(&mut registry, false)
                .expect("default")
                .name,
            "virtio"
        );
        assert_eq!(registry.transports()[1].module_gets, 1);

        let mut fallback = V9fsTransportRegistry::new();
        v9fs_register_trans(
            &mut fallback,
            P9TransModule {
                owner_available: false,
                ..P9TransModule::default("virtio")
            },
        );
        v9fs_register_trans(&mut fallback, P9TransModule::new("unix"));
        assert_eq!(
            v9fs_get_default_trans(&mut fallback, false)
                .expect("fallback")
                .name,
            "unix"
        );

        let mut empty = V9fsTransportRegistry::new();
        assert_eq!(v9fs_get_default_trans(&mut empty, true), None);
        assert_eq!(
            empty.requested_modules,
            vec![
                String::from("9p-virtio"),
                String::from("9p-tcp"),
                String::from("9p-fd"),
                String::from("9p-unix"),
                String::from("9p-xen"),
                String::from("9p-rdma"),
            ]
        );
    }

    #[test]
    fn init_exit_and_module_metadata_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/9p/mod.c"
        ));
        assert!(source.contains("ret = p9_client_init();"));
        assert!(source.contains("if (ret)"));
        assert!(source.contains("return ret;"));
        assert!(source.contains("p9_error_init();"));
        assert!(source.contains("pr_info(\"Installing 9P2000 support\\n\");"));
        assert!(source.contains("pr_info(\"Unloading 9P2000 support\\n\");"));
        assert!(source.contains("p9_client_exit();"));
        assert!(source.contains("module_init(init_p9)"));
        assert!(source.contains("module_exit(exit_p9)"));
        assert!(source.contains("MODULE_LICENSE(\"GPL\");"));
        assert!(
            source.contains("MODULE_DESCRIPTION(\"Plan 9 Resource Sharing Support (9P2000)\");")
        );

        assert_eq!(
            init_p9(-5),
            P9InitReport {
                ret: -5,
                p9_client_init_called: true,
                p9_error_init_called: false,
                info_log: None,
            }
        );
        assert_eq!(
            init_p9(0),
            P9InitReport {
                ret: 0,
                p9_client_init_called: true,
                p9_error_init_called: true,
                info_log: Some("Installing 9P2000 support"),
            }
        );
        assert_eq!(
            exit_p9(),
            P9ExitReport {
                p9_client_exit_called: true,
                info_log: "Unloading 9P2000 support",
            }
        );
        assert_eq!(MODULE_LICENSE, "GPL");
        assert_eq!(
            MODULE_DESCRIPTION,
            "Plan 9 Resource Sharing Support (9P2000)"
        );
        assert_eq!(MODULE_AUTHORS.len(), 3);
    }
}
