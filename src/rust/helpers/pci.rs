//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/pci.c
//! test-origin: linux:vendor/linux/rust/helpers/pci.c
//! Rust helper shims for PCI devices.

use super::RustHelperSource;

pub const LINUX_SOURCE: &str = "vendor/linux/rust/helpers/pci.c";
pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/pci.h>",
        helper_symbol: "rust_helper_pci_dev_id",
        forwards_to: "PCI_DEVID(dev->bus->number, dev->devfn)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/pci.h>",
        helper_symbol: "rust_helper_pci_resource_start",
        forwards_to: "pci_resource_start(pdev, bar)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/pci.h>",
        helper_symbol: "rust_helper_pci_resource_len",
        forwards_to: "pci_resource_len(pdev, bar)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/pci.h>",
        helper_symbol: "rust_helper_dev_is_pci",
        forwards_to: "dev_is_pci(dev)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/pci.h>",
        helper_symbol: "rust_helper_pci_alloc_irq_vectors",
        forwards_to: "pci_alloc_irq_vectors(dev, min_vecs, max_vecs, flags)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/pci.h>",
        helper_symbol: "rust_helper_pci_free_irq_vectors",
        forwards_to: "pci_free_irq_vectors(dev)",
    },
    RustHelperSource {
        linux_source: LINUX_SOURCE,
        include_line: "#include <linux/pci.h>",
        helper_symbol: "rust_helper_pci_irq_vector",
        forwards_to: "pci_irq_vector(pdev, nvec)",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_pci_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/pci.c"
        ));
        assert!(source.contains("#ifndef CONFIG_PCI_MSI"));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
