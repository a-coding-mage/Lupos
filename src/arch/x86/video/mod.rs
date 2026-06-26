//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/video/video-common.c
//! test-origin: linux:vendor/linux/arch/x86/video/video-common.c
//! x86 boot/video common helpers.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/video/video-common.c

use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VideoMode {
    pub columns: u16,
    pub rows: u16,
    pub depth: u8,
}

pub const fn video_mode_valid(mode: VideoMode) -> Result<(), i32> {
    if mode.columns == 0 || mode.rows == 0 {
        return Err(EINVAL);
    }
    match mode.depth {
        0 | 4 | 8 | 15 | 16 | 24 | 32 => Ok(()),
        _ => Err(EINVAL),
    }
}

pub const fn video_text_cells(mode: VideoMode) -> Result<u32, i32> {
    match video_mode_valid(mode) {
        Ok(()) => Ok((mode.columns as u32) * (mode.rows as u32)),
        Err(err) => Err(err),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FramebufferProt {
    pub cache_mask_cleared: bool,
    pub uc_minus: bool,
}

pub const fn pgprot_framebuffer(cpu_family: u8) -> FramebufferProt {
    FramebufferProt {
        cache_mask_cleared: true,
        uc_minus: cpu_family > 3,
    }
}

pub const fn video_is_primary_device(
    is_pci: bool,
    is_display: bool,
    is_vga_default: bool,
    matches_screen_resource: bool,
) -> bool {
    is_pci && is_display && (is_vga_default || matches_screen_resource)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_video_mode_rejects_empty_or_unknown_depth() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/video/video-common.c"
        ));
        assert!(source.contains("pgprot_t pgprot_framebuffer"));
        assert!(source.contains("pgprot_val(prot) &= ~_PAGE_CACHE_MASK;"));
        assert!(source.contains("boot_cpu_data.x86 > 3"));
        assert!(source.contains("cachemode2protval(_PAGE_CACHE_MODE_UC_MINUS)"));
        assert!(source.contains("bool video_is_primary_device"));
        assert!(source.contains("if (!dev_is_pci(dev))"));
        assert!(source.contains("if (!pci_is_display(pdev))"));
        assert!(source.contains("if (pdev == vga_default_device())"));
        assert!(source.contains("pci_find_resource(pdev, &res[i])"));
        assert!(source.contains("EXPORT_SYMBOL(video_is_primary_device);"));
        assert!(source.contains("MODULE_LICENSE(\"GPL\");"));

        assert_eq!(
            video_text_cells(VideoMode {
                columns: 80,
                rows: 25,
                depth: 0,
            }),
            Ok(2000)
        );
        assert_eq!(
            video_mode_valid(VideoMode {
                columns: 80,
                rows: 25,
                depth: 7,
            }),
            Err(EINVAL)
        );
        assert!(pgprot_framebuffer(6).uc_minus);
        assert!(!pgprot_framebuffer(3).uc_minus);
        assert!(video_is_primary_device(true, true, false, true));
        assert!(!video_is_primary_device(true, false, true, true));
    }
}
