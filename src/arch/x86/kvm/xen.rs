//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/xen.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/xen.c
//! Xen paravirtual interface exposure in KVM.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/xen.c

use crate::include::uapi::errno::ENODEV;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XenKvmCaps {
    pub xen_hvm_config: bool,
    pub shared_info_page: bool,
    pub event_channel: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XenKvmCall {
    SetSharedInfo,
    EventChannelOp,
}

pub const fn xen_kvm_call_allowed(caps: XenKvmCaps, call: XenKvmCall) -> Result<(), i32> {
    if !caps.xen_hvm_config {
        return Err(ENODEV);
    }
    match call {
        XenKvmCall::SetSharedInfo if caps.shared_info_page => Ok(()),
        XenKvmCall::EventChannelOp if caps.event_channel => Ok(()),
        _ => Err(ENODEV),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xen_calls_are_hidden_without_hvm_config() {
        let caps = XenKvmCaps {
            xen_hvm_config: false,
            shared_info_page: true,
            event_channel: true,
        };
        assert_eq!(
            xen_kvm_call_allowed(caps, XenKvmCall::SetSharedInfo),
            Err(ENODEV)
        );
    }
}
