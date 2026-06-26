//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/events/intel/lbr.c
//! test-origin: linux:vendor/linux/arch/x86/events/intel/lbr.c
//! Intel Last Branch Record model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/events/intel/lbr.c

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntelLbrFormat {
    None,
    Lbr32,
    Lbr64,
    EipFlags,
    Info,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntelLbrCapabilities {
    pub depth: u8,
    pub format: IntelLbrFormat,
    pub call_stack: bool,
}

pub const fn lbr_capabilities(model: u8, has_lbr: bool) -> IntelLbrCapabilities {
    if !has_lbr {
        return IntelLbrCapabilities {
            depth: 0,
            format: IntelLbrFormat::None,
            call_stack: false,
        };
    }
    IntelLbrCapabilities {
        depth: if model >= 0x3c { 32 } else { 16 },
        format: if model >= 0x3c {
            IntelLbrFormat::Info
        } else {
            IntelLbrFormat::Lbr64
        },
        call_stack: model >= 0x3c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn haswell_style_models_get_info_lbr() {
        let caps = lbr_capabilities(0x3c, true);
        assert_eq!(caps.depth, 32);
        assert_eq!(caps.format, IntelLbrFormat::Info);
    }
}
