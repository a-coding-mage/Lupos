//! linux-parity: complete
//! linux-source: vendor/linux/kernel/bpf/preload
//! test-origin: linux:vendor/linux/kernel/bpf/preload
//! BPF preload support.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BpfPreloadFile {
    pub rust_module: Option<&'static str>,
    pub linux_source: &'static str,
    pub required_markers: &'static [&'static str],
}

pub mod bpf_preload_kern;
pub mod iterators;

pub const BPF_PRELOAD_IMPLEMENTATIONS: &[BpfPreloadFile] = &[
    BpfPreloadFile {
        rust_module: Some("bpf_preload_kern"),
        linux_source: "vendor/linux/kernel/bpf/preload/bpf_preload_kern.c",
        required_markers: &[
            "#include \"bpf_preload.h\"",
            "iterators/iterators.lskel-little-endian.h",
            "static struct bpf_link *maps_link, *progs_link;",
            "static struct iterators_bpf *skel;",
            "strscpy(obj[0].link_name, \"maps.debug\"",
            "strscpy(obj[1].link_name, \"progs.debug\"",
            "bpf_link_get_from_fd(skel->links.dump_bpf_map_fd);",
            "bpf_link_get_from_fd(skel->links.dump_bpf_prog_fd);",
            "late_initcall(load);",
            "module_exit(fini);",
            "MODULE_DESCRIPTION(\"Embedded BPF programs for introspection in bpffs\")",
        ],
    },
    BpfPreloadFile {
        rust_module: Some("iterators"),
        linux_source: "vendor/linux/kernel/bpf/preload/iterators",
        required_markers: &["SEC(\"iter/bpf_map\")", "SEC(\"iter/bpf_prog\")"],
    },
];

pub const BPF_PRELOAD_METADATA: &[BpfPreloadFile] = &[
    BpfPreloadFile {
        rust_module: None,
        linux_source: "vendor/linux/kernel/bpf/preload/bpf_preload.h",
        required_markers: &[
            "#ifndef _BPF_PRELOAD_H",
            "struct bpf_preload_info",
            "char link_name[16];",
            "struct bpf_preload_ops",
            "extern struct bpf_preload_ops *bpf_preload_ops;",
            "#define BPF_PRELOAD_LINKS 2",
        ],
    },
    BpfPreloadFile {
        rust_module: None,
        linux_source: "vendor/linux/kernel/bpf/preload/Kconfig",
        required_markers: &[
            "menuconfig BPF_PRELOAD",
            "depends on BPF",
            "depends on BPF_SYSCALL",
            "depends on !COMPILE_TEST",
            "config BPF_PRELOAD_UMD",
            "default m",
        ],
    },
    BpfPreloadFile {
        rust_module: None,
        linux_source: "vendor/linux/kernel/bpf/preload/Makefile",
        required_markers: &[
            "LIBBPF_INCLUDE = $(srctree)/tools/lib",
            "obj-$(CONFIG_BPF_PRELOAD_UMD) += bpf_preload.o",
            "CFLAGS_bpf_preload_kern.o += -I$(LIBBPF_INCLUDE)",
            "bpf_preload-objs += bpf_preload_kern.o",
        ],
    },
    BpfPreloadFile {
        rust_module: None,
        linux_source: "vendor/linux/kernel/bpf/preload/.gitignore",
        required_markers: &["/libbpf", "/bpf_preload_umd"],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implementation_inventory_matches_linux_sources_and_children() {
        let kern_rs = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/kernel/bpf/preload/bpf_preload_kern.rs"
        ));
        let iterators_rs = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/kernel/bpf/preload/iterators/mod.rs"
        ));
        let kern_c = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/bpf/preload/bpf_preload_kern.c"
        ));
        let iterators_bpf = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/bpf/preload/iterators/iterators.bpf.c"
        ));

        assert_eq!(BPF_PRELOAD_IMPLEMENTATIONS.len(), 2);
        assert_eq!(
            BPF_PRELOAD_IMPLEMENTATIONS[0].rust_module,
            Some("bpf_preload_kern")
        );
        assert_eq!(
            BPF_PRELOAD_IMPLEMENTATIONS[0].linux_source,
            "vendor/linux/kernel/bpf/preload/bpf_preload_kern.c"
        );
        assert_eq!(
            BPF_PRELOAD_IMPLEMENTATIONS[1].rust_module,
            Some("iterators")
        );
        assert_eq!(
            BPF_PRELOAD_IMPLEMENTATIONS[1].linux_source,
            "vendor/linux/kernel/bpf/preload/iterators"
        );
        assert!(kern_rs.contains("//! linux-parity: complete"));
        assert!(iterators_rs.contains("//! linux-parity: complete"));
        assert!(kern_c.contains("SPDX-License-Identifier: GPL-2.0"));
        assert!(iterators_bpf.contains("SPDX-License-Identifier: GPL-2.0"));
        for marker in BPF_PRELOAD_IMPLEMENTATIONS[0].required_markers {
            assert!(kern_c.contains(marker), "missing {}", marker);
        }
        for marker in BPF_PRELOAD_IMPLEMENTATIONS[1].required_markers {
            assert!(iterators_bpf.contains(marker), "missing {}", marker);
        }
    }

    #[test]
    fn metadata_inventory_matches_linux_build_and_header_files() {
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/bpf/preload/bpf_preload.h"
        ));
        let kconfig = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/bpf/preload/Kconfig"
        ));
        let makefile = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/bpf/preload/Makefile"
        ));
        let gitignore = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/bpf/preload/.gitignore"
        ));
        let sources = [header, kconfig, makefile, gitignore];

        assert_eq!(BPF_PRELOAD_METADATA.len(), sources.len());
        for (entry, source) in BPF_PRELOAD_METADATA.iter().zip(sources) {
            assert_eq!(entry.rust_module, None);
            for marker in entry.required_markers {
                assert!(
                    source.contains(marker),
                    "{} missing {}",
                    entry.linux_source,
                    marker
                );
            }
        }
        assert!(header.contains("SPDX-License-Identifier: GPL-2.0"));
        assert!(kconfig.contains("SPDX-License-Identifier: GPL-2.0-only"));
        assert!(makefile.contains("SPDX-License-Identifier: GPL-2.0"));
    }

    #[test]
    fn aggregate_exposes_preload_contracts() {
        assert_eq!(bpf_preload_kern::BPF_PRELOAD_LINKS, 2);
        assert_eq!(bpf_preload_kern::MAPS_LINK_NAME, "maps.debug");
        assert_eq!(bpf_preload_kern::PROGS_LINK_NAME, "progs.debug");
        assert_eq!(bpf_preload_kern::MODULE_LICENSE, "GPL");
        assert_eq!(iterators::iterators_bpf::LICENSE, "GPL");
    }
}
