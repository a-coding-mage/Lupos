//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! x86 APIC MSI message model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/apic/msi.c

use crate::include::uapi::errno::EINVAL;

pub const MSI_ADDR_BASE: u32 = 0xfee0_0000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MsiMessage {
    pub address: u32,
    pub data: u32,
}

pub const fn compose_msi_message(
    dest_apic_id: u8,
    vector: u8,
    level_triggered: bool,
) -> Result<MsiMessage, i32> {
    if vector < 0x10 {
        return Err(EINVAL);
    }
    let address = MSI_ADDR_BASE | ((dest_apic_id as u32) << 12);
    let mut data = vector as u32;
    if level_triggered {
        data |= 1 << 15;
    }
    Ok(MsiMessage { address, data })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msi_message_uses_apic_destination_and_vector() {
        let msg = compose_msi_message(3, 0x41, true).unwrap();
        assert_eq!(msg.address, MSI_ADDR_BASE | (3 << 12));
        assert_eq!(msg.data & 0xff, 0x41);
        assert_ne!(msg.data & (1 << 15), 0);
    }
}
