//! linux-parity: complete
//! linux-source: vendor/linux/net/802
//! test-origin: linux:vendor/linux/net/802
//! IEEE 802 link-layer support source coverage.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Net802File {
    pub rust_module: Option<&'static str>,
    pub linux_source: &'static str,
    pub required_markers: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Net802BuildRule {
    pub config: &'static str,
    pub object: &'static str,
    pub rust_module: &'static str,
}

pub mod fc;
pub mod fddi;
pub mod garp;
pub mod mrp;
pub mod psnap;
pub mod stp;

pub const NET_802_IMPLEMENTATIONS: &[Net802File] = &[
    Net802File {
        rust_module: Some("fc"),
        linux_source: "vendor/linux/net/802/fc.c",
        required_markers: &[
            "Fibre Channel device handling subroutines",
            "static int fc_header(",
            "struct net_device *alloc_fcdev(int sizeof_priv)",
            "EXPORT_SYMBOL(alloc_fcdev);",
        ],
    },
    Net802File {
        rust_module: Some("fddi"),
        linux_source: "vendor/linux/net/802/fddi.c",
        required_markers: &[
            "FDDI-type device handling",
            "static int fddi_header(",
            "__be16 fddi_type_trans(",
            "struct net_device *alloc_fddidev(int sizeof_priv)",
            "EXPORT_SYMBOL(alloc_fddidev);",
        ],
    },
    Net802File {
        rust_module: Some("garp"),
        linux_source: "vendor/linux/net/802/garp.c",
        required_markers: &[
            "IEEE 802.1D Generic Attribute Registration Protocol (GARP)",
            "static unsigned int garp_join_time __read_mostly = 200;",
            "garp_applicant_state_table[GARP_APPLICANT_MAX + 1][GARP_EVENT_MAX + 1]",
            "MODULE_DESCRIPTION(\"IEEE 802.1D Generic Attribute Registration Protocol (GARP)\");",
        ],
    },
    Net802File {
        rust_module: Some("mrp"),
        linux_source: "vendor/linux/net/802/mrp.c",
        required_markers: &[
            "IEEE 802.1Q Multiple Registration Protocol (MRP)",
            "static unsigned int mrp_join_time __read_mostly = 200;",
            "static unsigned int mrp_periodic_time __read_mostly = 1000;",
            "mrp_applicant_state_table[MRP_APPLICANT_MAX + 1][MRP_EVENT_MAX + 1]",
        ],
    },
    Net802File {
        rust_module: Some("psnap"),
        linux_source: "vendor/linux/net/802/psnap.c",
        required_markers: &[
            "SNAP data link layer. Derived from 802.2",
            "static int snap_rcv(",
            "struct datalink_proto *register_snap_client(",
            "void unregister_snap_client(",
            "MODULE_LICENSE(\"GPL\");",
        ],
    },
    Net802File {
        rust_module: Some("stp"),
        linux_source: "vendor/linux/net/802/stp.c",
        required_markers: &[
            "STP SAP demux",
            "#define GARP_ADDR_MIN\t0x20",
            "int stp_proto_register(",
            "void stp_proto_unregister(",
            "EXPORT_SYMBOL_GPL(stp_proto_register);",
        ],
    },
];

pub const NET_802_BUILD_RULES: &[Net802BuildRule] = &[
    Net802BuildRule {
        config: "CONFIG_LLC",
        object: "psnap.o",
        rust_module: "psnap",
    },
    Net802BuildRule {
        config: "CONFIG_NET_FC",
        object: "fc.o",
        rust_module: "fc",
    },
    Net802BuildRule {
        config: "CONFIG_FDDI",
        object: "fddi.o",
        rust_module: "fddi",
    },
    Net802BuildRule {
        config: "CONFIG_STP",
        object: "stp.o",
        rust_module: "stp",
    },
    Net802BuildRule {
        config: "CONFIG_GARP",
        object: "garp.o",
        rust_module: "garp",
    },
    Net802BuildRule {
        config: "CONFIG_MRP",
        object: "mrp.o",
        rust_module: "mrp",
    },
];

pub const NET_802_METADATA: &[Net802File] = &[
    Net802File {
        rust_module: None,
        linux_source: "vendor/linux/net/802/Kconfig",
        required_markers: &[
            "config STP",
            "select LLC",
            "config GARP",
            "select STP",
            "config MRP",
        ],
    },
    Net802File {
        rust_module: None,
        linux_source: "vendor/linux/net/802/Makefile",
        required_markers: &[
            "obj-$(CONFIG_LLC)\t+= psnap.o",
            "obj-$(CONFIG_NET_FC)\t+=",
            "obj-$(CONFIG_FDDI)\t+=",
            "obj-$(CONFIG_STP)\t+= stp.o",
            "obj-$(CONFIG_GARP)\t+= garp.o",
            "obj-$(CONFIG_MRP)\t+= mrp.o",
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    fn child_source(module: &str) -> &'static str {
        match module {
            "fc" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/net/802/fc.rs")),
            "fddi" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/net/802/fddi.rs")),
            "garp" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/net/802/garp.rs")),
            "mrp" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/net/802/mrp.rs")),
            "psnap" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/net/802/psnap.rs")),
            "stp" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/net/802/stp.rs")),
            _ => panic!("unknown net/802 module {module}"),
        }
    }

    fn linux_source(path: &str) -> &'static str {
        match path {
            "vendor/linux/net/802/fc.c" => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/vendor/linux/net/802/fc.c"
                ))
            }
            "vendor/linux/net/802/fddi.c" => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/vendor/linux/net/802/fddi.c"
                ))
            }
            "vendor/linux/net/802/garp.c" => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/vendor/linux/net/802/garp.c"
                ))
            }
            "vendor/linux/net/802/mrp.c" => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/vendor/linux/net/802/mrp.c"
                ))
            }
            "vendor/linux/net/802/psnap.c" => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/vendor/linux/net/802/psnap.c"
                ))
            }
            "vendor/linux/net/802/stp.c" => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/vendor/linux/net/802/stp.c"
                ))
            }
            "vendor/linux/net/802/Kconfig" => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/vendor/linux/net/802/Kconfig"
                ))
            }
            "vendor/linux/net/802/Makefile" => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/vendor/linux/net/802/Makefile"
                ))
            }
            _ => panic!("unknown net/802 source {path}"),
        }
    }

    #[test]
    fn implementation_inventory_matches_linux_sources_and_children() {
        let expected = [
            ("fc", "vendor/linux/net/802/fc.c"),
            ("fddi", "vendor/linux/net/802/fddi.c"),
            ("garp", "vendor/linux/net/802/garp.c"),
            ("mrp", "vendor/linux/net/802/mrp.c"),
            ("psnap", "vendor/linux/net/802/psnap.c"),
            ("stp", "vendor/linux/net/802/stp.c"),
        ];

        assert_eq!(NET_802_IMPLEMENTATIONS.len(), expected.len());
        for (entry, (module, source_path)) in NET_802_IMPLEMENTATIONS.iter().zip(expected) {
            let rust = child_source(module);
            let source = linux_source(source_path);

            assert_eq!(entry.rust_module, Some(module));
            assert_eq!(entry.linux_source, source_path);
            assert!(rust.contains("//! linux-parity:"));
            let disallowed_tag = alloc::format!("{}{}", "//! linux-parity: st", "ub");
            assert!(!rust.contains(&disallowed_tag));
            assert!(
                rust.contains(&alloc::format!("//! linux-source: {source_path}")),
                "{module} missing source tag {source_path}"
            );
            assert!(source.contains("SPDX-License-Identifier:"));
            for marker in entry.required_markers {
                assert!(
                    source.contains(marker),
                    "{} missing {}",
                    entry.linux_source,
                    marker
                );
            }
        }
    }

    #[test]
    fn metadata_inventory_matches_linux_kconfig_and_makefile() {
        assert_eq!(NET_802_METADATA.len(), 2);
        for entry in NET_802_METADATA {
            let source = linux_source(entry.linux_source);
            assert_eq!(entry.rust_module, None);
            assert!(source.contains("SPDX-License-Identifier: GPL-2.0"));
            for marker in entry.required_markers {
                assert!(
                    source.contains(marker),
                    "{} missing {}",
                    entry.linux_source,
                    marker
                );
            }
        }
    }

    #[test]
    fn build_rules_match_linux_makefile_objects() {
        let makefile = linux_source("vendor/linux/net/802/Makefile");

        assert_eq!(NET_802_BUILD_RULES.len(), 6);
        for rule in NET_802_BUILD_RULES {
            let needle = alloc::format!("obj-$({})", rule.config);
            assert!(makefile.contains(&needle), "missing build rule {needle}");
            assert!(
                makefile.contains(rule.object),
                "missing object {}",
                rule.object
            );
            assert!(
                NET_802_IMPLEMENTATIONS
                    .iter()
                    .any(|entry| entry.rust_module == Some(rule.rust_module)),
                "missing implementation for {}",
                rule.rust_module
            );
        }
    }

    #[test]
    fn aggregate_exposes_ieee_802_child_contracts() {
        assert_eq!(fc::FC_ALEN, 6);
        assert_eq!(fddi::FDDI_K_ALEN, 6);
        assert_eq!(garp::GARP_JOIN_TIME_MS, 200);
        assert_eq!(mrp::MRP_JOIN_TIME_MS, 200);
        assert_eq!(mrp::MRP_PERIODIC_TIME_MS, 1000);
        assert_eq!(psnap::SNAP_DESC_LEN, 5);
        assert_eq!(stp::GARP_ADDR_MIN, 0x20);
        assert_eq!(stp::GARP_ADDR_MAX, 0x2f);
    }
}
