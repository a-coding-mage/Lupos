//! linux-parity: partial
//! linux-source: vendor/linux/drivers/gpu/drm
//! test-origin: linux:vendor/linux/drivers/gpu/drm
//! Linux DRM top-level source inventory (not implementation coverage).
//!
//! Linux source inventory for this subsystem. These references are source
//! truth for ABI glue, module staging, and parity audits; driver
//! implementations remain vendor-built Linux artifacts.
//!
//! Refs:
//! - `vendor/linux/drivers/gpu/{drm/drm_atomic,drm/drm_atomic_helper,drm/drm_atomic_state_helper,drm/drm_atomic_uapi,drm/drm_auth,drm/drm_blend,drm/drm_bridge,drm/drm_bridge_helper,drm/drm_buddy,drm/drm_cache,drm/drm_client,drm/drm_client_event,drm/drm_client_modeset,drm/drm_client_sysrq,drm/drm_color_mgmt,drm/drm_colorop,drm/drm_connector,drm/drm_crtc,drm/drm_crtc_helper,drm/drm_damage_helper,drm/drm_debugfs,drm/drm_debugfs_crc,drm/drm_displayid,drm/drm_draw,drm/drm_drv,drm/drm_dumb_buffers,drm/drm_edid,drm/drm_edid_load,drm/drm_eld,drm/drm_encoder,drm/drm_exec,drm/drm_fb_dma_helper,drm/drm_fb_helper,drm/drm_fbdev_dma,drm/drm_fbdev_shmem,drm/drm_fbdev_ttm,drm/drm_file,drm/drm_flip_work,drm/drm_format_helper,drm/drm_fourcc,drm/drm_framebuffer,drm/drm_gem,drm/drm_gem_atomic_helper,drm/drm_gem_dma_helper,drm/drm_gem_framebuffer_helper,drm/drm_gem_shmem_helper,drm/drm_gem_ttm_helper,drm/drm_gem_vram_helper,drm/drm_gpusvm,drm/drm_gpuvm,drm/drm_ioc32,drm/drm_ioctl,drm/drm_kms_helper_common,drm/drm_lease,drm/drm_managed,drm/drm_mipi_dbi,drm/drm_mipi_dsi,drm/drm_mm,drm/drm_mode_config,drm/drm_mode_object,drm/drm_modes,drm/drm_modeset_helper,drm/drm_modeset_lock,drm/drm_of,drm/drm_pagemap,drm/drm_pagemap_util,drm/drm_panel,drm/drm_panel_backlight_quirks,drm/drm_panel_orientation_quirks,drm/drm_panic,drm/drm_pci,drm/drm_plane,drm/drm_plane_helper,drm/drm_prime,drm/drm_print,drm/drm_privacy_screen,drm/drm_privacy_screen_x86,drm/drm_probe_helper,drm/drm_property,drm/drm_ras,drm/drm_ras_genl_family,drm/drm_ras_nl,drm/drm_rect,drm/drm_self_refresh_helper,drm/drm_simple_kms_helper,drm/drm_suballoc,drm/drm_syncobj,drm/drm_sysfs,drm/drm_trace_points,drm/drm_vblank,drm/drm_vblank_helper,drm/drm_vblank_work,drm/drm_vma_manager,drm/drm_writeback}.c`

/// Number of Linux `.c` files catalogued for this subsystem.
pub const DRM_SOURCES_COUNT: usize = 94;

/// Catalogued upstream Linux source paths used as source truth.
pub const DRM_SOURCES: &[&str] = &[
    "vendor/linux/drivers/gpu/drm/drm_atomic.c",
    "vendor/linux/drivers/gpu/drm/drm_atomic_helper.c",
    "vendor/linux/drivers/gpu/drm/drm_atomic_state_helper.c",
    "vendor/linux/drivers/gpu/drm/drm_atomic_uapi.c",
    "vendor/linux/drivers/gpu/drm/drm_auth.c",
    "vendor/linux/drivers/gpu/drm/drm_blend.c",
    "vendor/linux/drivers/gpu/drm/drm_bridge.c",
    "vendor/linux/drivers/gpu/drm/drm_bridge_helper.c",
    "vendor/linux/drivers/gpu/drm/drm_buddy.c",
    "vendor/linux/drivers/gpu/drm/drm_cache.c",
    "vendor/linux/drivers/gpu/drm/drm_client.c",
    "vendor/linux/drivers/gpu/drm/drm_client_event.c",
    "vendor/linux/drivers/gpu/drm/drm_client_modeset.c",
    "vendor/linux/drivers/gpu/drm/drm_client_sysrq.c",
    "vendor/linux/drivers/gpu/drm/drm_color_mgmt.c",
    "vendor/linux/drivers/gpu/drm/drm_colorop.c",
    "vendor/linux/drivers/gpu/drm/drm_connector.c",
    "vendor/linux/drivers/gpu/drm/drm_crtc.c",
    "vendor/linux/drivers/gpu/drm/drm_crtc_helper.c",
    "vendor/linux/drivers/gpu/drm/drm_damage_helper.c",
    "vendor/linux/drivers/gpu/drm/drm_debugfs.c",
    "vendor/linux/drivers/gpu/drm/drm_debugfs_crc.c",
    "vendor/linux/drivers/gpu/drm/drm_displayid.c",
    "vendor/linux/drivers/gpu/drm/drm_draw.c",
    "vendor/linux/drivers/gpu/drm/drm_drv.c",
    "vendor/linux/drivers/gpu/drm/drm_dumb_buffers.c",
    "vendor/linux/drivers/gpu/drm/drm_edid.c",
    "vendor/linux/drivers/gpu/drm/drm_edid_load.c",
    "vendor/linux/drivers/gpu/drm/drm_eld.c",
    "vendor/linux/drivers/gpu/drm/drm_encoder.c",
    "vendor/linux/drivers/gpu/drm/drm_exec.c",
    "vendor/linux/drivers/gpu/drm/drm_fb_dma_helper.c",
    "vendor/linux/drivers/gpu/drm/drm_fb_helper.c",
    "vendor/linux/drivers/gpu/drm/drm_fbdev_dma.c",
    "vendor/linux/drivers/gpu/drm/drm_fbdev_shmem.c",
    "vendor/linux/drivers/gpu/drm/drm_fbdev_ttm.c",
    "vendor/linux/drivers/gpu/drm/drm_file.c",
    "vendor/linux/drivers/gpu/drm/drm_flip_work.c",
    "vendor/linux/drivers/gpu/drm/drm_format_helper.c",
    "vendor/linux/drivers/gpu/drm/drm_fourcc.c",
    "vendor/linux/drivers/gpu/drm/drm_framebuffer.c",
    "vendor/linux/drivers/gpu/drm/drm_gem.c",
    "vendor/linux/drivers/gpu/drm/drm_gem_atomic_helper.c",
    "vendor/linux/drivers/gpu/drm/drm_gem_dma_helper.c",
    "vendor/linux/drivers/gpu/drm/drm_gem_framebuffer_helper.c",
    "vendor/linux/drivers/gpu/drm/drm_gem_shmem_helper.c",
    "vendor/linux/drivers/gpu/drm/drm_gem_ttm_helper.c",
    "vendor/linux/drivers/gpu/drm/drm_gem_vram_helper.c",
    "vendor/linux/drivers/gpu/drm/drm_gpusvm.c",
    "vendor/linux/drivers/gpu/drm/drm_gpuvm.c",
    "vendor/linux/drivers/gpu/drm/drm_ioc32.c",
    "vendor/linux/drivers/gpu/drm/drm_ioctl.c",
    "vendor/linux/drivers/gpu/drm/drm_kms_helper_common.c",
    "vendor/linux/drivers/gpu/drm/drm_lease.c",
    "vendor/linux/drivers/gpu/drm/drm_managed.c",
    "vendor/linux/drivers/gpu/drm/drm_mipi_dbi.c",
    "vendor/linux/drivers/gpu/drm/drm_mipi_dsi.c",
    "vendor/linux/drivers/gpu/drm/drm_mm.c",
    "vendor/linux/drivers/gpu/drm/drm_mode_config.c",
    "vendor/linux/drivers/gpu/drm/drm_mode_object.c",
    "vendor/linux/drivers/gpu/drm/drm_modes.c",
    "vendor/linux/drivers/gpu/drm/drm_modeset_helper.c",
    "vendor/linux/drivers/gpu/drm/drm_modeset_lock.c",
    "vendor/linux/drivers/gpu/drm/drm_of.c",
    "vendor/linux/drivers/gpu/drm/drm_pagemap.c",
    "vendor/linux/drivers/gpu/drm/drm_pagemap_util.c",
    "vendor/linux/drivers/gpu/drm/drm_panel.c",
    "vendor/linux/drivers/gpu/drm/drm_panel_backlight_quirks.c",
    "vendor/linux/drivers/gpu/drm/drm_panel_orientation_quirks.c",
    "vendor/linux/drivers/gpu/drm/drm_panic.c",
    "vendor/linux/drivers/gpu/drm/drm_pci.c",
    "vendor/linux/drivers/gpu/drm/drm_plane.c",
    "vendor/linux/drivers/gpu/drm/drm_plane_helper.c",
    "vendor/linux/drivers/gpu/drm/drm_prime.c",
    "vendor/linux/drivers/gpu/drm/drm_print.c",
    "vendor/linux/drivers/gpu/drm/drm_privacy_screen.c",
    "vendor/linux/drivers/gpu/drm/drm_privacy_screen_x86.c",
    "vendor/linux/drivers/gpu/drm/drm_probe_helper.c",
    "vendor/linux/drivers/gpu/drm/drm_property.c",
    "vendor/linux/drivers/gpu/drm/drm_ras.c",
    "vendor/linux/drivers/gpu/drm/drm_ras_genl_family.c",
    "vendor/linux/drivers/gpu/drm/drm_ras_nl.c",
    "vendor/linux/drivers/gpu/drm/drm_rect.c",
    "vendor/linux/drivers/gpu/drm/drm_self_refresh_helper.c",
    "vendor/linux/drivers/gpu/drm/drm_simple_kms_helper.c",
    "vendor/linux/drivers/gpu/drm/drm_suballoc.c",
    "vendor/linux/drivers/gpu/drm/drm_syncobj.c",
    "vendor/linux/drivers/gpu/drm/drm_sysfs.c",
    "vendor/linux/drivers/gpu/drm/drm_trace_points.c",
    "vendor/linux/drivers/gpu/drm/drm_vblank.c",
    "vendor/linux/drivers/gpu/drm/drm_vblank_helper.c",
    "vendor/linux/drivers/gpu/drm/drm_vblank_work.c",
    "vendor/linux/drivers/gpu/drm/drm_vma_manager.c",
    "vendor/linux/drivers/gpu/drm/drm_writeback.c",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_matches_table() {
        assert_eq!(DRM_SOURCES.len(), DRM_SOURCES_COUNT);
    }

    #[test]
    fn all_paths_have_canonical_prefix() {
        for path in DRM_SOURCES {
            assert!(path.starts_with("vendor/linux/drivers/gpu/"), "{path}");
            assert!(path.ends_with(".c"));
        }
    }

    #[test]
    fn table_matches_every_top_level_drm_c_file() {
        extern crate std;
        use std::collections::BTreeSet;
        use std::fs;

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let drm_dir = root.join("vendor/linux/drivers/gpu/drm");
        let actual = fs::read_dir(&drm_dir)
            .expect("read vendor DRM directory")
            .map(|entry| entry.expect("read vendor DRM entry").path())
            .filter(|path| {
                path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("c")
            })
            .map(|path| {
                path.strip_prefix(root)
                    .expect("DRM source under repository")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect::<BTreeSet<_>>();
        let catalogued = DRM_SOURCES
            .iter()
            .map(|path| std::string::String::from(*path))
            .collect::<BTreeSet<_>>();

        assert_eq!(catalogued, actual);
    }
}
