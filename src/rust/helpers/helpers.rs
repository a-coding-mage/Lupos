//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/helpers.c
//! test-origin: linux:vendor/linux/rust/helpers/helpers.c
//! Aggregate source manifest for inline Rust helper C shims.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustHelperAggregate {
    pub linux_source: &'static str,
    pub include_line: &'static str,
    pub helper_define: &'static str,
}

pub const SOURCE: RustHelperAggregate = RustHelperAggregate {
    linux_source: "vendor/linux/rust/helpers/helpers.c",
    include_line: "#include <linux/compiler_types.h>",
    helper_define: "#define __rust_helper __always_inline",
};

pub const INCLUDED_HELPER_FILES: &[&str] = &[
    "atomic.c",
    "atomic_ext.c",
    "auxiliary.c",
    "barrier.c",
    "binder.c",
    "bitmap.c",
    "bitops.c",
    "blk.c",
    "bug.c",
    "build_assert.c",
    "build_bug.c",
    "clk.c",
    "completion.c",
    "cpu.c",
    "cpufreq.c",
    "cpumask.c",
    "cred.c",
    "device.c",
    "dma.c",
    "dma-resv.c",
    "drm.c",
    "err.c",
    "irq.c",
    "fs.c",
    "gpu.c",
    "io.c",
    "jump_label.c",
    "kunit.c",
    "list.c",
    "maple_tree.c",
    "mm.c",
    "mutex.c",
    "of.c",
    "page.c",
    "pci.c",
    "pid_namespace.c",
    "platform.c",
    "poll.c",
    "processor.c",
    "property.c",
    "pwm.c",
    "rbtree.c",
    "rcu.c",
    "refcount.c",
    "regulator.c",
    "scatterlist.c",
    "security.c",
    "signal.c",
    "slab.c",
    "spinlock.c",
    "sync.c",
    "task.c",
    "time.c",
    "uaccess.c",
    "usb.c",
    "vmalloc.c",
    "wait.c",
    "workqueue.c",
    "xarray.c",
];

pub fn source() -> RustHelperAggregate {
    SOURCE
}

pub fn included_helper_files() -> &'static [&'static str] {
    INCLUDED_HELPER_FILES
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    fn aggregate_manifest_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/helpers.c"
        ));

        assert_eq!(
            source.lines().next(),
            Some("// SPDX-License-Identifier: GPL-2.0")
        );
        assert!(source.contains(SOURCE.include_line));
        assert!(source.contains(SOURCE.helper_define));
        assert!(source.contains("Sorted alphabetically."));

        for file in INCLUDED_HELPER_FILES {
            let include = format!("#include \"{}\"", file);
            assert!(
                source.contains(&include),
                "vendor/linux/rust/helpers/helpers.c missing {}",
                include
            );
        }
    }
}
