//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/hyperv/irqdomain.c
//! test-origin: linux:vendor/linux/arch/x86/hyperv/irqdomain.c
//! Hyper-V IRQ domain routing model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/hyperv/irqdomain.c

use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HvIrqRoute {
    pub vcpu: u32,
    pub vector: u8,
    pub sint: u8,
}

impl HvIrqRoute {
    pub const fn new(vcpu: u32, vector: u8, sint: u8) -> Result<Self, i32> {
        if vector < 0x10 || sint >= 16 {
            Err(EINVAL)
        } else {
            Ok(Self { vcpu, vector, sint })
        }
    }
}

pub const fn route_matches_cpu(route: HvIrqRoute, vcpu: u32) -> bool {
    route.vcpu == vcpu
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn irq_routes_validate_vector_and_sint() {
        assert_eq!(HvIrqRoute::new(0, 0x0f, 0), Err(EINVAL));
        assert_eq!(HvIrqRoute::new(0, 0x20, 16), Err(EINVAL));
        assert!(route_matches_cpu(HvIrqRoute::new(3, 0x30, 1).unwrap(), 3));
    }
}
