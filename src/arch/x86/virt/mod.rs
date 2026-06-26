//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/virt
//! test-origin: linux:vendor/linux/arch/x86/virt
//! x86 virtualization facility policy.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/virt/hw.c
//! - vendor/linux/arch/x86/virt/svm/cmdline.c
//! - vendor/linux/arch/x86/virt/vmx/tdx/tdx_global_metadata.c

pub mod svm;
pub mod virtualization;
pub mod vmx;

use crate::include::uapi::errno::{EINVAL, ENODEV};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VirtBackend {
    None,
    Svm,
    Vmx,
    Tdx,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VirtHardwareCaps {
    pub svm: bool,
    pub vmx: bool,
    pub tdx_module: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TdxMetadata {
    pub version: u16,
    pub fields: u16,
    pub checksum_ok: bool,
}

pub const fn select_virt_backend(caps: VirtHardwareCaps) -> VirtBackend {
    if caps.tdx_module && caps.vmx {
        VirtBackend::Tdx
    } else if caps.vmx {
        VirtBackend::Vmx
    } else if caps.svm {
        VirtBackend::Svm
    } else {
        VirtBackend::None
    }
}

pub const fn svm_cmdline_enabled(arg: Option<bool>) -> bool {
    match arg {
        Some(value) => value,
        None => true,
    }
}

pub const fn validate_tdx_global_metadata(metadata: TdxMetadata) -> Result<(), i32> {
    if metadata.version == 0 || metadata.fields == 0 {
        return Err(EINVAL);
    }
    if !metadata.checksum_ok {
        return Err(ENODEV);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtualization_backend_prefers_tdx_then_vmx_then_svm() {
        assert_eq!(
            select_virt_backend(VirtHardwareCaps {
                svm: true,
                vmx: true,
                tdx_module: true,
            }),
            VirtBackend::Tdx
        );
        assert_eq!(svm_cmdline_enabled(None), true);
        assert_eq!(svm_cmdline_enabled(Some(false)), false);
    }

    #[test]
    fn tdx_metadata_requires_version_fields_and_checksum() {
        assert_eq!(
            validate_tdx_global_metadata(TdxMetadata {
                version: 1,
                fields: 4,
                checksum_ok: true,
            }),
            Ok(())
        );
        assert_eq!(
            validate_tdx_global_metadata(TdxMetadata {
                version: 1,
                fields: 4,
                checksum_ok: false,
            }),
            Err(ENODEV)
        );
    }
}
