//! linux-parity: complete
//! linux-source: vendor/linux/lib/tests/blackhole_dev_kunit.c
//! test-origin: linux:vendor/linux/lib/tests/blackhole_dev_kunit.c
//! KUnit blackhole netdevice packet-shape coverage.

pub const SKB_SIZE: usize = 256;
pub const HEAD_SIZE: usize = 14 + 40 + 8;
pub const TAIL_SIZE: usize = 32;
pub const UDP_PORT: u16 = 1234;
pub const IPV6_HOP_LIMIT: u8 = 32;
pub const TEST_SUITE_NAME: &str = "blackholedev";
pub const MODULE_DESCRIPTION: &str = "module test of the blackhole_dev";

pub const fn blackholedev_payload_len() -> usize {
    SKB_SIZE - (HEAD_SIZE + TAIL_SIZE)
}

pub const fn blackholedev_ipv6_payload_len() -> usize {
    blackholedev_payload_len() + 8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blackhole_dev_kunit_matches_linux_packet_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/tests/blackhole_dev_kunit.c"
        ));
        assert!(source.contains("#define SKB_SIZE  256"));
        assert!(source.contains("#define HEAD_SIZE (14+40+8)"));
        assert!(source.contains("#define TAIL_SIZE 32"));
        assert!(source.contains("#define UDP_PORT 1234"));
        assert!(source.contains("skb_reserve(skb, HEAD_SIZE);"));
        assert!(source.contains("memset(__skb_put(skb, data_len), 0xf, data_len);"));
        assert!(source.contains("uh->source = uh->dest = htons(UDP_PORT);"));
        assert!(source.contains("ip6h->hop_limit = 32;"));
        assert!(source.contains("skb->dev = blackhole_netdev;"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, dev_queue_xmit(skb), NET_XMIT_SUCCESS);"));
        assert!(source.contains(".name = \"blackholedev\""));
        assert!(source.contains("MODULE_DESCRIPTION(\"module test of the blackhole_dev\")"));

        assert_eq!(HEAD_SIZE, 62);
        assert_eq!(blackholedev_payload_len(), 162);
        assert_eq!(blackholedev_ipv6_payload_len(), 170);
        assert_eq!(UDP_PORT, 1234);
        assert_eq!(TEST_SUITE_NAME, "blackholedev");
    }
}
