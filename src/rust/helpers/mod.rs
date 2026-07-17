//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers
//! test-origin: linux:vendor/linux/rust/helpers
//! Linux Rust helper C shim source contracts.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustHelperSource {
    pub linux_source: &'static str,
    pub include_line: &'static str,
    pub helper_symbol: &'static str,
    pub forwards_to: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustHelperModule {
    pub rust_module: &'static str,
    pub linux_source: &'static str,
}

pub const RUST_HELPER_MODULES: &[RustHelperModule] = &[
    RustHelperModule {
        rust_module: "atomic",
        linux_source: "vendor/linux/rust/helpers/atomic.c",
    },
    RustHelperModule {
        rust_module: "atomic_ext",
        linux_source: "vendor/linux/rust/helpers/atomic_ext.c",
    },
    RustHelperModule {
        rust_module: "auxiliary",
        linux_source: "vendor/linux/rust/helpers/auxiliary.c",
    },
    RustHelperModule {
        rust_module: "barrier",
        linux_source: "vendor/linux/rust/helpers/barrier.c",
    },
    RustHelperModule {
        rust_module: "binder",
        linux_source: "vendor/linux/rust/helpers/binder.c",
    },
    RustHelperModule {
        rust_module: "bitmap",
        linux_source: "vendor/linux/rust/helpers/bitmap.c",
    },
    RustHelperModule {
        rust_module: "bitops",
        linux_source: "vendor/linux/rust/helpers/bitops.c",
    },
    RustHelperModule {
        rust_module: "blk",
        linux_source: "vendor/linux/rust/helpers/blk.c",
    },
    RustHelperModule {
        rust_module: "bug",
        linux_source: "vendor/linux/rust/helpers/bug.c",
    },
    RustHelperModule {
        rust_module: "build_assert",
        linux_source: "vendor/linux/rust/helpers/build_assert.c",
    },
    RustHelperModule {
        rust_module: "build_bug",
        linux_source: "vendor/linux/rust/helpers/build_bug.c",
    },
    RustHelperModule {
        rust_module: "clk",
        linux_source: "vendor/linux/rust/helpers/clk.c",
    },
    RustHelperModule {
        rust_module: "completion",
        linux_source: "vendor/linux/rust/helpers/completion.c",
    },
    RustHelperModule {
        rust_module: "cpu",
        linux_source: "vendor/linux/rust/helpers/cpu.c",
    },
    RustHelperModule {
        rust_module: "cpufreq",
        linux_source: "vendor/linux/rust/helpers/cpufreq.c",
    },
    RustHelperModule {
        rust_module: "cpumask",
        linux_source: "vendor/linux/rust/helpers/cpumask.c",
    },
    RustHelperModule {
        rust_module: "cred",
        linux_source: "vendor/linux/rust/helpers/cred.c",
    },
    RustHelperModule {
        rust_module: "device",
        linux_source: "vendor/linux/rust/helpers/device.c",
    },
    RustHelperModule {
        rust_module: "dma",
        linux_source: "vendor/linux/rust/helpers/dma.c",
    },
    RustHelperModule {
        rust_module: "dma_resv",
        linux_source: "vendor/linux/rust/helpers/dma-resv.c",
    },
    RustHelperModule {
        rust_module: "drm",
        linux_source: "vendor/linux/rust/helpers/drm.c",
    },
    RustHelperModule {
        rust_module: "err",
        linux_source: "vendor/linux/rust/helpers/err.c",
    },
    RustHelperModule {
        rust_module: "fs",
        linux_source: "vendor/linux/rust/helpers/fs.c",
    },
    RustHelperModule {
        rust_module: "gpu",
        linux_source: "vendor/linux/rust/helpers/gpu.c",
    },
    RustHelperModule {
        rust_module: "helpers",
        linux_source: "vendor/linux/rust/helpers/helpers.c",
    },
    RustHelperModule {
        rust_module: "io",
        linux_source: "vendor/linux/rust/helpers/io.c",
    },
    RustHelperModule {
        rust_module: "irq",
        linux_source: "vendor/linux/rust/helpers/irq.c",
    },
    RustHelperModule {
        rust_module: "jump_label",
        linux_source: "vendor/linux/rust/helpers/jump_label.c",
    },
    RustHelperModule {
        rust_module: "kunit",
        linux_source: "vendor/linux/rust/helpers/kunit.c",
    },
    RustHelperModule {
        rust_module: "list",
        linux_source: "vendor/linux/rust/helpers/list.c",
    },
    RustHelperModule {
        rust_module: "maple_tree",
        linux_source: "vendor/linux/rust/helpers/maple_tree.c",
    },
    RustHelperModule {
        rust_module: "mm",
        linux_source: "vendor/linux/rust/helpers/mm.c",
    },
    RustHelperModule {
        rust_module: "mutex",
        linux_source: "vendor/linux/rust/helpers/mutex.c",
    },
    RustHelperModule {
        rust_module: "of",
        linux_source: "vendor/linux/rust/helpers/of.c",
    },
    RustHelperModule {
        rust_module: "page",
        linux_source: "vendor/linux/rust/helpers/page.c",
    },
    RustHelperModule {
        rust_module: "pci",
        linux_source: "vendor/linux/rust/helpers/pci.c",
    },
    RustHelperModule {
        rust_module: "pid_namespace",
        linux_source: "vendor/linux/rust/helpers/pid_namespace.c",
    },
    RustHelperModule {
        rust_module: "platform",
        linux_source: "vendor/linux/rust/helpers/platform.c",
    },
    RustHelperModule {
        rust_module: "poll",
        linux_source: "vendor/linux/rust/helpers/poll.c",
    },
    RustHelperModule {
        rust_module: "processor",
        linux_source: "vendor/linux/rust/helpers/processor.c",
    },
    RustHelperModule {
        rust_module: "property",
        linux_source: "vendor/linux/rust/helpers/property.c",
    },
    RustHelperModule {
        rust_module: "pwm",
        linux_source: "vendor/linux/rust/helpers/pwm.c",
    },
    RustHelperModule {
        rust_module: "rbtree",
        linux_source: "vendor/linux/rust/helpers/rbtree.c",
    },
    RustHelperModule {
        rust_module: "rcu",
        linux_source: "vendor/linux/rust/helpers/rcu.c",
    },
    RustHelperModule {
        rust_module: "refcount",
        linux_source: "vendor/linux/rust/helpers/refcount.c",
    },
    RustHelperModule {
        rust_module: "regulator",
        linux_source: "vendor/linux/rust/helpers/regulator.c",
    },
    RustHelperModule {
        rust_module: "scatterlist",
        linux_source: "vendor/linux/rust/helpers/scatterlist.c",
    },
    RustHelperModule {
        rust_module: "security",
        linux_source: "vendor/linux/rust/helpers/security.c",
    },
    RustHelperModule {
        rust_module: "signal",
        linux_source: "vendor/linux/rust/helpers/signal.c",
    },
    RustHelperModule {
        rust_module: "slab",
        linux_source: "vendor/linux/rust/helpers/slab.c",
    },
    RustHelperModule {
        rust_module: "spinlock",
        linux_source: "vendor/linux/rust/helpers/spinlock.c",
    },
    RustHelperModule {
        rust_module: "sync",
        linux_source: "vendor/linux/rust/helpers/sync.c",
    },
    RustHelperModule {
        rust_module: "task",
        linux_source: "vendor/linux/rust/helpers/task.c",
    },
    RustHelperModule {
        rust_module: "time",
        linux_source: "vendor/linux/rust/helpers/time.c",
    },
    RustHelperModule {
        rust_module: "uaccess",
        linux_source: "vendor/linux/rust/helpers/uaccess.c",
    },
    RustHelperModule {
        rust_module: "usb",
        linux_source: "vendor/linux/rust/helpers/usb.c",
    },
    RustHelperModule {
        rust_module: "vmalloc",
        linux_source: "vendor/linux/rust/helpers/vmalloc.c",
    },
    RustHelperModule {
        rust_module: "wait",
        linux_source: "vendor/linux/rust/helpers/wait.c",
    },
    RustHelperModule {
        rust_module: "workqueue",
        linux_source: "vendor/linux/rust/helpers/workqueue.c",
    },
    RustHelperModule {
        rust_module: "xarray",
        linux_source: "vendor/linux/rust/helpers/xarray.c",
    },
];

pub mod atomic;
pub mod atomic_ext;
pub mod auxiliary;
pub mod barrier;
pub mod binder;
pub mod bitmap;
pub mod bitops;
pub mod blk;
pub mod bug;
pub mod build_assert;
pub mod build_bug;
pub mod clk;
pub mod completion;
pub mod cpu;
pub mod cpufreq;
pub mod cpumask;
pub mod cred;
pub mod device;
pub mod dma;
pub mod dma_resv;
pub mod drm;
pub mod err;
pub mod fs;
pub mod gpu;
pub mod helpers;
pub mod io;
pub mod irq;
pub mod jump_label;
pub mod kunit;
pub mod list;
pub mod maple_tree;
pub mod mm;
pub mod mutex;
pub mod of;
pub mod page;
pub mod pci;
pub mod pid_namespace;
pub mod platform;
pub mod poll;
pub mod processor;
pub mod property;
pub mod pwm;
pub mod rbtree;
pub mod rcu;
pub mod refcount;
pub mod regulator;
pub mod scatterlist;
pub mod security;
pub mod signal;
pub mod slab;
pub mod spinlock;
pub mod sync;
pub mod task;
pub mod time;
pub mod uaccess;
pub mod usb;
pub mod vmalloc;
pub mod wait;
pub mod workqueue;
pub mod xarray;

#[cfg(test)]
pub(crate) fn assert_helper_source(source: &str, contract: RustHelperSource) {
    let mut lines = source.lines();
    assert_eq!(
        lines.next(),
        Some("// SPDX-License-Identifier: GPL-2.0"),
        "{}",
        contract.linux_source
    );
    assert!(
        source.contains(contract.include_line),
        "{} missing {}",
        contract.linux_source,
        contract.include_line
    );
    assert!(
        source.contains("__rust_helper"),
        "{} missing __rust_helper marker",
        contract.linux_source
    );
    assert!(
        source.contains(contract.helper_symbol),
        "{} missing {}",
        contract.linux_source,
        contract.helper_symbol
    );
    assert!(
        source.contains(contract.forwards_to),
        "{} missing {}",
        contract.linux_source,
        contract.forwards_to
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_helper_inventory {
        ($(($module:literal, $source:literal, $marker:literal)),+ $(,)?) => {
            #[test]
            fn helper_inventory_matches_complete_children_and_vendor_sources() {
                let mut idx = 0usize;
                $(
                    let rust = include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/src/rust/helpers/",
                        $module,
                        ".rs"
                    ));
                    let linux = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $source));
                    let declared = RUST_HELPER_MODULES[idx];

                    assert_eq!(declared.rust_module, $module);
                    assert_eq!(declared.linux_source, $source);
                    let parity = match $module {
                        "drm" | "gpu" => "stub",
                        _ => "complete",
                    };
                    assert!(
                        rust.contains(&alloc::format!("//! linux-parity: {parity}")),
                        "{}",
                        $module
                    );
                    assert!(
                        rust.contains(concat!("//! linux-source: ", $source)),
                        "{} missing source tag {}",
                        $module,
                        $source
                    );
                    assert!(linux.starts_with("// SPDX-License-Identifier: GPL-2.0"), "{}", $source);
                    assert!(linux.contains($marker), "{} missing {}", $source, $marker);

                    idx += 1;
                )+
                assert_eq!(idx, RUST_HELPER_MODULES.len());
            }
        };
    }

    assert_helper_inventory!(
        (
            "atomic",
            "vendor/linux/rust/helpers/atomic.c",
            "__rust_helper"
        ),
        (
            "atomic_ext",
            "vendor/linux/rust/helpers/atomic_ext.c",
            "__rust_helper"
        ),
        (
            "auxiliary",
            "vendor/linux/rust/helpers/auxiliary.c",
            "__rust_helper"
        ),
        (
            "barrier",
            "vendor/linux/rust/helpers/barrier.c",
            "__rust_helper"
        ),
        (
            "binder",
            "vendor/linux/rust/helpers/binder.c",
            "__rust_helper"
        ),
        (
            "bitmap",
            "vendor/linux/rust/helpers/bitmap.c",
            "__rust_helper"
        ),
        (
            "bitops",
            "vendor/linux/rust/helpers/bitops.c",
            "__rust_helper"
        ),
        ("blk", "vendor/linux/rust/helpers/blk.c", "__rust_helper"),
        ("bug", "vendor/linux/rust/helpers/bug.c", "__rust_helper"),
        (
            "build_assert",
            "vendor/linux/rust/helpers/build_assert.c",
            "static_assert("
        ),
        (
            "build_bug",
            "vendor/linux/rust/helpers/build_bug.c",
            "__rust_helper"
        ),
        ("clk", "vendor/linux/rust/helpers/clk.c", "__rust_helper"),
        (
            "completion",
            "vendor/linux/rust/helpers/completion.c",
            "__rust_helper"
        ),
        ("cpu", "vendor/linux/rust/helpers/cpu.c", "__rust_helper"),
        (
            "cpufreq",
            "vendor/linux/rust/helpers/cpufreq.c",
            "__rust_helper"
        ),
        (
            "cpumask",
            "vendor/linux/rust/helpers/cpumask.c",
            "__rust_helper"
        ),
        ("cred", "vendor/linux/rust/helpers/cred.c", "__rust_helper"),
        (
            "device",
            "vendor/linux/rust/helpers/device.c",
            "__rust_helper"
        ),
        ("dma", "vendor/linux/rust/helpers/dma.c", "__rust_helper"),
        (
            "dma_resv",
            "vendor/linux/rust/helpers/dma-resv.c",
            "__rust_helper"
        ),
        ("drm", "vendor/linux/rust/helpers/drm.c", "__rust_helper"),
        ("err", "vendor/linux/rust/helpers/err.c", "__rust_helper"),
        ("fs", "vendor/linux/rust/helpers/fs.c", "__rust_helper"),
        ("gpu", "vendor/linux/rust/helpers/gpu.c", "__rust_helper"),
        (
            "helpers",
            "vendor/linux/rust/helpers/helpers.c",
            "__rust_helper"
        ),
        ("io", "vendor/linux/rust/helpers/io.c", "__rust_helper"),
        ("irq", "vendor/linux/rust/helpers/irq.c", "__rust_helper"),
        (
            "jump_label",
            "vendor/linux/rust/helpers/jump_label.c",
            "__rust_helper"
        ),
        (
            "kunit",
            "vendor/linux/rust/helpers/kunit.c",
            "__rust_helper"
        ),
        ("list", "vendor/linux/rust/helpers/list.c", "__rust_helper"),
        (
            "maple_tree",
            "vendor/linux/rust/helpers/maple_tree.c",
            "__rust_helper"
        ),
        ("mm", "vendor/linux/rust/helpers/mm.c", "__rust_helper"),
        (
            "mutex",
            "vendor/linux/rust/helpers/mutex.c",
            "__rust_helper"
        ),
        ("of", "vendor/linux/rust/helpers/of.c", "__rust_helper"),
        ("page", "vendor/linux/rust/helpers/page.c", "__rust_helper"),
        ("pci", "vendor/linux/rust/helpers/pci.c", "__rust_helper"),
        (
            "pid_namespace",
            "vendor/linux/rust/helpers/pid_namespace.c",
            "__rust_helper"
        ),
        (
            "platform",
            "vendor/linux/rust/helpers/platform.c",
            "__rust_helper"
        ),
        ("poll", "vendor/linux/rust/helpers/poll.c", "__rust_helper"),
        (
            "processor",
            "vendor/linux/rust/helpers/processor.c",
            "__rust_helper"
        ),
        (
            "property",
            "vendor/linux/rust/helpers/property.c",
            "__rust_helper"
        ),
        ("pwm", "vendor/linux/rust/helpers/pwm.c", "__rust_helper"),
        (
            "rbtree",
            "vendor/linux/rust/helpers/rbtree.c",
            "__rust_helper"
        ),
        ("rcu", "vendor/linux/rust/helpers/rcu.c", "__rust_helper"),
        (
            "refcount",
            "vendor/linux/rust/helpers/refcount.c",
            "__rust_helper"
        ),
        (
            "regulator",
            "vendor/linux/rust/helpers/regulator.c",
            "__rust_helper"
        ),
        (
            "scatterlist",
            "vendor/linux/rust/helpers/scatterlist.c",
            "__rust_helper"
        ),
        (
            "security",
            "vendor/linux/rust/helpers/security.c",
            "__rust_helper"
        ),
        (
            "signal",
            "vendor/linux/rust/helpers/signal.c",
            "__rust_helper"
        ),
        ("slab", "vendor/linux/rust/helpers/slab.c", "__rust_helper"),
        (
            "spinlock",
            "vendor/linux/rust/helpers/spinlock.c",
            "__rust_helper"
        ),
        ("sync", "vendor/linux/rust/helpers/sync.c", "__rust_helper"),
        ("task", "vendor/linux/rust/helpers/task.c", "__rust_helper"),
        ("time", "vendor/linux/rust/helpers/time.c", "__rust_helper"),
        (
            "uaccess",
            "vendor/linux/rust/helpers/uaccess.c",
            "__rust_helper"
        ),
        ("usb", "vendor/linux/rust/helpers/usb.c", "__rust_helper"),
        (
            "vmalloc",
            "vendor/linux/rust/helpers/vmalloc.c",
            "__rust_helper"
        ),
        ("wait", "vendor/linux/rust/helpers/wait.c", "__rust_helper"),
        (
            "workqueue",
            "vendor/linux/rust/helpers/workqueue.c",
            "__rust_helper"
        ),
        (
            "xarray",
            "vendor/linux/rust/helpers/xarray.c",
            "__rust_helper"
        ),
    );
}
