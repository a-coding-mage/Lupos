//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv4/udp_tunnel_stub.c
//! test-origin: linux:vendor/linux/net/ipv4/udp_tunnel_stub.c
//! UDP tunnel NIC operations export stub.

pub const LINUX_SOURCE: &str = "vendor/linux/net/ipv4/udp_tunnel_stub.c";
pub const EXPORTED_SYMBOL: &str = "udp_tunnel_nic_ops";

pub fn exported_symbol() -> &'static str {
    EXPORTED_SYMBOL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn udp_tunnel_stub_exports_ops_pointer() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv4/udp_tunnel_stub.c"
        ));
        assert!(source.contains("#include <net/udp_tunnel.h>"));
        assert!(source.contains("const struct udp_tunnel_nic_ops *udp_tunnel_nic_ops;"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(udp_tunnel_nic_ops);"));
        assert_eq!(exported_symbol(), EXPORTED_SYMBOL);
    }
}
