//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv6/xfrm6_state.c
//! test-origin: linux:vendor/linux/net/ipv6/xfrm6_state.c
//! IPv6 XFRM state afinfo registration.

use crate::net::socket::AF_INET6;

pub const IPPROTO_IPV6: u8 = 41;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Xfrm6StateAfInfo {
    pub family: u16,
    pub proto: u8,
    pub output: &'static str,
    pub transport_finish: &'static str,
    pub local_error: &'static str,
}

pub const XFRM6_STATE_AFINFO: Xfrm6StateAfInfo = Xfrm6StateAfInfo {
    family: AF_INET6,
    proto: IPPROTO_IPV6,
    output: "xfrm6_output",
    transport_finish: "xfrm6_transport_finish",
    local_error: "xfrm6_local_error",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xfrm6_state_afinfo_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv6/xfrm6_state.c"
        ));
        assert!(source.contains("#include <net/xfrm.h>"));
        assert!(source.contains("AF_INET6"));
        assert!(source.contains("IPPROTO_IPV6"));
        assert!(source.contains("xfrm_state_register_afinfo(&xfrm6_state_afinfo);"));
        assert!(source.contains("xfrm_state_unregister_afinfo(&xfrm6_state_afinfo);"));
        assert_eq!(XFRM6_STATE_AFINFO.family, AF_INET6);
        assert_eq!(XFRM6_STATE_AFINFO.proto, IPPROTO_IPV6);
        assert_eq!(XFRM6_STATE_AFINFO.output, "xfrm6_output");
    }
}
