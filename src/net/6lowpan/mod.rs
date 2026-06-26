//! linux-parity: partial
//! linux-source: vendor/linux/net/6lowpan
//! test-origin: linux:vendor/linux/net/6lowpan
//! 6LoWPAN next-header compression source contracts.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LowpanNhcSource {
    pub linux_source: &'static str,
    pub symbol: &'static str,
    pub description: &'static str,
    pub module_description: &'static str,
    pub id: u8,
    pub mask: u8,
    pub id_literal: &'static str,
    pub mask_literal: &'static str,
}

impl LowpanNhcSource {
    pub const fn matches_first_byte(self, first: u8) -> bool {
        first & self.mask == self.id
    }
}

pub mod core;
pub mod debugfs;
pub mod iphc;
pub mod ndisc;
pub mod nhc;
pub mod nhc_dest;
pub mod nhc_fragment;
pub mod nhc_ghc_ext_dest;
pub mod nhc_ghc_ext_frag;
pub mod nhc_ghc_ext_hop;
pub mod nhc_ghc_ext_route;
pub mod nhc_ghc_icmpv6;
pub mod nhc_ghc_udp;
pub mod nhc_hop;
pub mod nhc_ipv6;
pub mod nhc_mobility;
pub mod nhc_routing;
pub mod nhc_udp;

#[cfg(test)]
pub(crate) fn assert_lowpan_nhc_source(source: &str, contract: LowpanNhcSource) {
    assert!(
        source.contains("#include \"nhc.h\""),
        "{} missing nhc.h include",
        contract.linux_source
    );
    assert!(
        source.contains(contract.symbol),
        "{} missing {}",
        contract.linux_source,
        contract.symbol
    );
    assert!(
        source.contains(contract.description),
        "{} missing {}",
        contract.linux_source,
        contract.description
    );
    assert!(
        source.contains(contract.module_description),
        "{} missing {}",
        contract.linux_source,
        contract.module_description
    );
    assert!(
        source.contains(contract.id_literal),
        "{} missing {}",
        contract.linux_source,
        contract.id_literal
    );
    assert!(
        source.contains(contract.mask_literal),
        "{} missing {}",
        contract.linux_source,
        contract.mask_literal
    );
    assert!(contract.matches_first_byte(contract.id));
}
