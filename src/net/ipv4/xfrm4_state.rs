//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv4/xfrm4_state.c
//! test-origin: linux:vendor/linux/net/ipv4/xfrm4_state.c
//! IPv4 XFRM state afinfo registration.

pub const AF_INET: u16 = 2;
pub const IPPROTO_IPIP: u8 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XfrmStateAfInfo {
    pub family: u16,
    pub proto: u8,
    pub output: &'static str,
    pub transport_finish: &'static str,
    pub local_error: &'static str,
}

pub const XFRM4_STATE_AFINFO: XfrmStateAfInfo = XfrmStateAfInfo {
    family: AF_INET,
    proto: IPPROTO_IPIP,
    output: "xfrm4_output",
    transport_finish: "xfrm4_transport_finish",
    local_error: "xfrm4_local_error",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xfrm4_state_afinfo_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv4/xfrm4_state.c"
        ));
        assert!(source.contains("#include <net/xfrm.h>"));
        assert!(source.contains(".family"));
        assert!(source.contains("AF_INET"));
        assert!(source.contains("IPPROTO_IPIP"));
        assert!(source.contains("xfrm_state_register_afinfo(&xfrm4_state_afinfo);"));
        assert_eq!(XFRM4_STATE_AFINFO.family, AF_INET);
        assert_eq!(XFRM4_STATE_AFINFO.proto, IPPROTO_IPIP);
        assert_eq!(XFRM4_STATE_AFINFO.output, "xfrm4_output");
    }
}
