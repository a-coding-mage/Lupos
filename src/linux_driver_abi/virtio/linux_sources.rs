//! linux-parity: complete
//! linux-source: vendor/linux/drivers/virtio
//! test-origin: linux:vendor/linux/drivers/virtio
//! Linux VirtIO transport (drivers/virtio) source coverage.
//!
//! Linux source inventory for this subsystem. This catalog names the
//! `vendor/linux` sources that produce the module payloads and ABI surfaces
//! Lupos must support. It is not a local driver implementation plan: runtime
//! virtio-pci and function drivers must arrive as Linux-built `.ko` artifacts.
//!
//! Refs:
//! - `vendor/linux/drivers/virtio/{virtio_anchor,virtio_balloon,virtio_debug,virtio_dma_buf,virtio_input,virtio_mem,virtio_mmio,virtio_pci_admin_legacy_io,virtio_pci_common,virtio_pci_legacy,virtio_pci_legacy_dev,virtio_pci_modern_dev,virtio_rtc_arm,virtio_rtc_class,virtio_rtc_driver,virtio_rtc_ptp,virtio_vdpa}.c`

/// Number of Linux `.c` files catalogued for this subsystem.
pub const VIRTIO_SOURCES_COUNT: usize = 17;

/// Catalogued upstream Linux source paths used as driver-module source truth.
pub const VIRTIO_SOURCES: &[&str] = &[
    "vendor/linux/drivers/virtio/virtio_anchor.c",
    "vendor/linux/drivers/virtio/virtio_balloon.c",
    "vendor/linux/drivers/virtio/virtio_debug.c",
    "vendor/linux/drivers/virtio/virtio_dma_buf.c",
    "vendor/linux/drivers/virtio/virtio_input.c",
    "vendor/linux/drivers/virtio/virtio_mem.c",
    "vendor/linux/drivers/virtio/virtio_mmio.c",
    "vendor/linux/drivers/virtio/virtio_pci_admin_legacy_io.c",
    "vendor/linux/drivers/virtio/virtio_pci_common.c",
    "vendor/linux/drivers/virtio/virtio_pci_legacy.c",
    "vendor/linux/drivers/virtio/virtio_pci_legacy_dev.c",
    "vendor/linux/drivers/virtio/virtio_pci_modern_dev.c",
    "vendor/linux/drivers/virtio/virtio_rtc_arm.c",
    "vendor/linux/drivers/virtio/virtio_rtc_class.c",
    "vendor/linux/drivers/virtio/virtio_rtc_driver.c",
    "vendor/linux/drivers/virtio/virtio_rtc_ptp.c",
    "vendor/linux/drivers/virtio/virtio_vdpa.c",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_matches_table() {
        assert_eq!(VIRTIO_SOURCES.len(), VIRTIO_SOURCES_COUNT);
    }

    #[test]
    fn all_paths_have_canonical_prefix() {
        for path in VIRTIO_SOURCES {
            assert!(path.starts_with("vendor/linux/drivers/virtio/"), "{path}");
            assert!(path.ends_with(".c"));
        }
    }
}
