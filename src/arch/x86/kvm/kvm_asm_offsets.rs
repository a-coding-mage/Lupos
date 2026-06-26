//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kvm/kvm-asm-offsets.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/kvm-asm-offsets.c
//! KVM x86 assembly-offset symbol generator.

use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmAsmOffsetConfig {
    pub kvm_amd: bool,
    pub kvm_intel: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KvmAsmOffset {
    SvmVcpuArchRegs,
    SvmCurrentVmcb,
    SvmSpecCtrl,
    SvmVmcb01,
    KvmVmcbPa,
    SdSaveAreaPa,
    VmxSpecCtrl,
}

pub fn kvm_asm_offsets(config: KvmAsmOffsetConfig) -> Vec<KvmAsmOffset> {
    let mut out = Vec::new();

    if config.kvm_amd {
        out.extend_from_slice(&[
            KvmAsmOffset::SvmVcpuArchRegs,
            KvmAsmOffset::SvmCurrentVmcb,
            KvmAsmOffset::SvmSpecCtrl,
            KvmAsmOffset::SvmVmcb01,
            KvmAsmOffset::KvmVmcbPa,
            KvmAsmOffset::SdSaveAreaPa,
        ]);
    }

    if config.kvm_intel {
        out.push(KvmAsmOffset::VmxSpecCtrl);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn kvm_offsets_are_gated_by_amd_and_intel_config() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kvm/kvm-asm-offsets.c"
        ));
        assert!(source.contains("if (IS_ENABLED(CONFIG_KVM_AMD))"));
        assert!(source.contains("OFFSET(SVM_current_vmcb, vcpu_svm, current_vmcb);"));
        assert!(source.contains("if (IS_ENABLED(CONFIG_KVM_INTEL))"));
        assert!(source.contains("OFFSET(VMX_spec_ctrl, vcpu_vmx, spec_ctrl);"));

        assert_eq!(
            kvm_asm_offsets(KvmAsmOffsetConfig {
                kvm_amd: true,
                kvm_intel: true,
            }),
            vec![
                KvmAsmOffset::SvmVcpuArchRegs,
                KvmAsmOffset::SvmCurrentVmcb,
                KvmAsmOffset::SvmSpecCtrl,
                KvmAsmOffset::SvmVmcb01,
                KvmAsmOffset::KvmVmcbPa,
                KvmAsmOffset::SdSaveAreaPa,
                KvmAsmOffset::VmxSpecCtrl,
            ]
        );
        assert_eq!(
            kvm_asm_offsets(KvmAsmOffsetConfig {
                kvm_amd: false,
                kvm_intel: true,
            }),
            vec![KvmAsmOffset::VmxSpecCtrl]
        );
    }
}
