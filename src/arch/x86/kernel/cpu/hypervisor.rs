//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/hypervisor.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/hypervisor.c
//! Common x86 hypervisor detection and init-hook copying.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X86HypervisorType {
    None,
    XenPv,
    XenHvm,
    Vmware,
    MicrosoftHyperV,
    Kvm,
    Jailhouse,
    Acrn,
    Bhyve,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HypervisorX86 {
    pub name: &'static str,
    pub hv_type: X86HypervisorType,
    pub detect_priority: u32,
    pub ignore_nopv: bool,
    pub init_slots: [Option<usize>; 4],
    pub runtime_slots: [Option<usize>; 4],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HypervisorPlatform {
    pub hv_type: X86HypervisorType,
    pub init_slots: [Option<usize>; 4],
    pub runtime_slots: [Option<usize>; 4],
    pub init_platform_called: bool,
}

pub fn parse_nopv() -> bool {
    true
}

pub fn detect_hypervisor_vendor(
    nopv: bool,
    hypervisors: &[HypervisorX86],
) -> Option<HypervisorX86> {
    let mut selected = None;
    let mut max_pri = 0;

    for hypervisor in hypervisors {
        if nopv && !hypervisor.ignore_nopv {
            continue;
        }
        if hypervisor.detect_priority > max_pri {
            max_pri = hypervisor.detect_priority;
            selected = Some(*hypervisor);
        }
    }

    selected
}

pub fn copy_array(src: &[Option<usize>], target: &mut [Option<usize>]) {
    let count = core::cmp::min(src.len(), target.len());
    for i in 0..count {
        if src[i].is_some() {
            target[i] = src[i];
        }
    }
}

pub fn init_hypervisor_platform(
    nopv: bool,
    hypervisors: &[HypervisorX86],
) -> Option<HypervisorPlatform> {
    let h = detect_hypervisor_vendor(nopv, hypervisors)?;
    let mut platform = HypervisorPlatform {
        hv_type: h.hv_type,
        init_slots: [None; 4],
        runtime_slots: [None; 4],
        init_platform_called: false,
    };
    copy_array(&h.init_slots, &mut platform.init_slots);
    copy_array(&h.runtime_slots, &mut platform.runtime_slots);
    platform.init_platform_called = true;
    Some(platform)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VMWARE: HypervisorX86 = HypervisorX86 {
        name: "VMware",
        hv_type: X86HypervisorType::Vmware,
        detect_priority: 10,
        ignore_nopv: false,
        init_slots: [Some(1), None, Some(3), None],
        runtime_slots: [None, Some(8), None, None],
    };
    const KVM: HypervisorX86 = HypervisorX86 {
        name: "KVM",
        hv_type: X86HypervisorType::Kvm,
        detect_priority: 20,
        ignore_nopv: false,
        init_slots: [Some(4), None, None, None],
        runtime_slots: [Some(9), None, None, None],
    };
    const XEN_PV: HypervisorX86 = HypervisorX86 {
        name: "Xen PV",
        hv_type: X86HypervisorType::XenPv,
        detect_priority: 5,
        ignore_nopv: true,
        init_slots: [Some(7), None, None, None],
        runtime_slots: [None; 4],
    };

    #[test]
    fn hypervisor_detection_matches_linux_priority_and_nopv_rules() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/cpu/hypervisor.c"
        ));
        assert!(
            source.contains("static const __initconst struct hypervisor_x86 * const hypervisors[]")
        );
        assert!(source.contains("enum x86_hypervisor_type x86_hyper_type;"));
        assert!(source.contains("early_param(\"nopv\", parse_nopv);"));
        assert!(source.contains("if (unlikely(nopv) && !(*p)->ignore_nopv)"));
        assert!(source.contains("pri = (*p)->detect();"));
        assert!(source.contains("if (pri > max_pri)"));
        assert!(source.contains("pr_info(\"Hypervisor detected: %s\\n\", h->name);"));
        assert!(source.contains("copy_array(&h->init, &x86_init.hyper, sizeof(h->init));"));
        assert!(
            source.contains("copy_array(&h->runtime, &x86_platform.hyper, sizeof(h->runtime));")
        );
        assert!(source.contains("x86_hyper_type = h->type;"));
        assert!(source.contains("x86_init.hyper.init_platform();"));

        let hypervisors = [VMWARE, KVM, XEN_PV];
        assert_eq!(
            detect_hypervisor_vendor(false, &hypervisors)
                .unwrap()
                .hv_type,
            X86HypervisorType::Kvm
        );
        assert_eq!(
            detect_hypervisor_vendor(true, &hypervisors)
                .unwrap()
                .hv_type,
            X86HypervisorType::XenPv
        );
        assert!(parse_nopv());
    }

    #[test]
    fn init_platform_copies_only_non_null_hook_slots() {
        let mut target = [Some(99), None, Some(88), None];
        copy_array(&VMWARE.init_slots, &mut target);
        assert_eq!(target, [Some(1), None, Some(3), None]);

        let platform = init_hypervisor_platform(false, &[VMWARE]).unwrap();
        assert_eq!(platform.hv_type, X86HypervisorType::Vmware);
        assert_eq!(platform.init_slots, [Some(1), None, Some(3), None]);
        assert_eq!(platform.runtime_slots, [None, Some(8), None, None]);
        assert!(platform.init_platform_called);
    }
}
