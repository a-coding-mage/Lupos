//! linux-parity: complete
//! linux-source: vendor/linux/drivers/base
//! test-origin: linux:vendor/linux/drivers/base
//! Linux driver core (drivers/base) source coverage.
//!
//! Linux source inventory for this subsystem. These references are source
//! truth for ABI glue, module staging, and parity audits; driver
//! implementations remain vendor-built Linux artifacts.
//!
//! Refs:
//! - `vendor/linux/drivers/base/{arch_numa,arch_topology,attribute_container,auxiliary,auxiliary_sysfs,cacheinfo,component,container,cpu,devcoredump,devres,devtmpfs,faux,firmware,firmware_loader/builtin/main,firmware_loader/fallback,firmware_loader/fallback_platform,firmware_loader/fallback_table,firmware_loader/main,firmware_loader/sysfs,firmware_loader/sysfs_upload,hypervisor,init,isa,map,memory,module,node,physical_location,pinctrl,platform-msi,power/clock_ops,power/common,power/generic_ops,power/main,power/qos,power/runtime,power/sysfs,power/trace,power/wakeirq,power/wakeup,power/wakeup_stats,property,regmap/regcache,regmap/regcache-flat,regmap/regcache-maple,regmap/regcache-rbtree,regmap/regmap,regmap/regmap-ac97,regmap/regmap-debugfs,regmap/regmap-fsi,regmap/regmap-i2c,regmap/regmap-i3c,regmap/regmap-irq,regmap/regmap-kunit,regmap/regmap-mdio,regmap/regmap-mmio,regmap/regmap-ram,regmap/regmap-raw-ram,regmap/regmap-sccb,regmap/regmap-sdw,regmap/regmap-sdw-mbq,regmap/regmap-slimbus,regmap/regmap-spi,regmap/regmap-spi-avmm,regmap/regmap-spmi,regmap/regmap-w1,soc,swnode,syscore,test/test_async_driver_probe,topology,trace,transport_class}.c`

/// Number of Linux `.c` files catalogued for this subsystem.
pub const DRIVER_BASE_SOURCES_COUNT: usize = 74;

/// Catalogued upstream Linux source paths used as source truth.
pub const DRIVER_BASE_SOURCES: &[&str] = &[
    "vendor/linux/drivers/base/arch_numa.c",
    "vendor/linux/drivers/base/arch_topology.c",
    "vendor/linux/drivers/base/attribute_container.c",
    "vendor/linux/drivers/base/auxiliary.c",
    "vendor/linux/drivers/base/auxiliary_sysfs.c",
    "vendor/linux/drivers/base/cacheinfo.c",
    "vendor/linux/drivers/base/component.c",
    "vendor/linux/drivers/base/container.c",
    "vendor/linux/drivers/base/cpu.c",
    "vendor/linux/drivers/base/devcoredump.c",
    "vendor/linux/drivers/base/devres.c",
    "vendor/linux/drivers/base/devtmpfs.c",
    "vendor/linux/drivers/base/faux.c",
    "vendor/linux/drivers/base/firmware.c",
    "vendor/linux/drivers/base/firmware_loader/builtin/main.c",
    "vendor/linux/drivers/base/firmware_loader/fallback.c",
    "vendor/linux/drivers/base/firmware_loader/fallback_platform.c",
    "vendor/linux/drivers/base/firmware_loader/fallback_table.c",
    "vendor/linux/drivers/base/firmware_loader/main.c",
    "vendor/linux/drivers/base/firmware_loader/sysfs.c",
    "vendor/linux/drivers/base/firmware_loader/sysfs_upload.c",
    "vendor/linux/drivers/base/hypervisor.c",
    "vendor/linux/drivers/base/init.c",
    "vendor/linux/drivers/base/isa.c",
    "vendor/linux/drivers/base/map.c",
    "vendor/linux/drivers/base/memory.c",
    "vendor/linux/drivers/base/module.c",
    "vendor/linux/drivers/base/node.c",
    "vendor/linux/drivers/base/physical_location.c",
    "vendor/linux/drivers/base/pinctrl.c",
    "vendor/linux/drivers/base/platform-msi.c",
    "vendor/linux/drivers/base/power/clock_ops.c",
    "vendor/linux/drivers/base/power/common.c",
    "vendor/linux/drivers/base/power/generic_ops.c",
    "vendor/linux/drivers/base/power/main.c",
    "vendor/linux/drivers/base/power/qos.c",
    "vendor/linux/drivers/base/power/runtime.c",
    "vendor/linux/drivers/base/power/sysfs.c",
    "vendor/linux/drivers/base/power/trace.c",
    "vendor/linux/drivers/base/power/wakeirq.c",
    "vendor/linux/drivers/base/power/wakeup.c",
    "vendor/linux/drivers/base/power/wakeup_stats.c",
    "vendor/linux/drivers/base/property.c",
    "vendor/linux/drivers/base/regmap/regcache.c",
    "vendor/linux/drivers/base/regmap/regcache-flat.c",
    "vendor/linux/drivers/base/regmap/regcache-maple.c",
    "vendor/linux/drivers/base/regmap/regcache-rbtree.c",
    "vendor/linux/drivers/base/regmap/regmap.c",
    "vendor/linux/drivers/base/regmap/regmap-ac97.c",
    "vendor/linux/drivers/base/regmap/regmap-debugfs.c",
    "vendor/linux/drivers/base/regmap/regmap-fsi.c",
    "vendor/linux/drivers/base/regmap/regmap-i2c.c",
    "vendor/linux/drivers/base/regmap/regmap-i3c.c",
    "vendor/linux/drivers/base/regmap/regmap-irq.c",
    "vendor/linux/drivers/base/regmap/regmap-kunit.c",
    "vendor/linux/drivers/base/regmap/regmap-mdio.c",
    "vendor/linux/drivers/base/regmap/regmap-mmio.c",
    "vendor/linux/drivers/base/regmap/regmap-ram.c",
    "vendor/linux/drivers/base/regmap/regmap-raw-ram.c",
    "vendor/linux/drivers/base/regmap/regmap-sccb.c",
    "vendor/linux/drivers/base/regmap/regmap-sdw.c",
    "vendor/linux/drivers/base/regmap/regmap-sdw-mbq.c",
    "vendor/linux/drivers/base/regmap/regmap-slimbus.c",
    "vendor/linux/drivers/base/regmap/regmap-spi.c",
    "vendor/linux/drivers/base/regmap/regmap-spi-avmm.c",
    "vendor/linux/drivers/base/regmap/regmap-spmi.c",
    "vendor/linux/drivers/base/regmap/regmap-w1.c",
    "vendor/linux/drivers/base/soc.c",
    "vendor/linux/drivers/base/swnode.c",
    "vendor/linux/drivers/base/syscore.c",
    "vendor/linux/drivers/base/test/test_async_driver_probe.c",
    "vendor/linux/drivers/base/topology.c",
    "vendor/linux/drivers/base/trace.c",
    "vendor/linux/drivers/base/transport_class.c",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_matches_table() {
        assert_eq!(DRIVER_BASE_SOURCES.len(), DRIVER_BASE_SOURCES_COUNT);
    }

    #[test]
    fn all_paths_have_canonical_prefix() {
        for path in DRIVER_BASE_SOURCES {
            assert!(path.starts_with("vendor/linux/drivers/base/"), "{path}");
            assert!(path.ends_with(".c"));
        }
    }
}
