//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! SGI UV (Ultraviolet) non-standard x2APIC topology decoder.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/apic/x2apic_uv_x.c

// UV systems pack hub/pnode/node identifiers into the upper bits of the
// 32-bit APIC ID. Linux exposes helpers like uv_apicid_to_pnode() to
// other subsystems; we mirror just the bit-extraction policy used by the
// `arch/x86/kernel/apic/x2apic_uv_x.c` boot path.

pub const UV_APICID_PNODE_SHIFT: u32 = 14;
pub const UV_APICID_PNODE_MASK: u32 = 0x3ff;
pub const UV_APICID_NASID_SHIFT: u32 = 15;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UvApicLayout {
    pub pnode: u16,
    pub nasid: u16,
    pub socket: u8,
}

pub const fn uv_apic_layout(apicid: u32) -> UvApicLayout {
    let pnode = ((apicid >> UV_APICID_PNODE_SHIFT) & UV_APICID_PNODE_MASK) as u16;
    let nasid = (apicid >> UV_APICID_NASID_SHIFT) as u16;
    let socket = (apicid & 0xff) as u8;
    UvApicLayout {
        pnode,
        nasid,
        socket,
    }
}

pub const fn uv_apicid_to_pnode(apicid: u32) -> u16 {
    ((apicid >> UV_APICID_PNODE_SHIFT) & UV_APICID_PNODE_MASK) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pnode_extracted_from_upper_apicid_bits() {
        let layout = uv_apic_layout(0x0040_4002);
        assert_eq!(layout.pnode, 0x101);
        assert_eq!(layout.socket, 0x02);
    }

    #[test]
    fn helper_matches_layout_pnode() {
        let apicid = 0x0010_8000;
        assert_eq!(uv_apicid_to_pnode(apicid), uv_apic_layout(apicid).pnode);
    }
}
