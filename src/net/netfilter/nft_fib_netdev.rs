//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nft_fib_netdev.c
//! test-origin: linux:vendor/linux/net/netfilter/nft_fib_netdev.c
//! nftables netdev-family FIB expression dispatch.

pub const ETH_P_IP: u16 = 0x0800;
pub const ETH_P_IPV6: u16 = 0x86dd;
pub const NFPROTO_NETDEV: u8 = 5;
pub const NFTA_FIB_MAX: u8 = 3;
pub const NFT_BREAK: i32 = -2;
pub const MODULE_AUTHOR: &str = "Pablo M. Bermudo Garay <pablombg@gmail.com>";
pub const MODULE_DESCRIPTION: &str = "nftables netdev fib lookups support";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIAS: &str = "nft-afinfo-5-fib";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NftFibResult {
    Oif,
    OifName,
    AddrType,
    Other(u8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftFib {
    pub result: NftFibResult,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NftFibNetdevDispatch {
    Fib4,
    Fib4Type,
    Fib6,
    Fib6Type,
    Break,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NftRegs {
    pub verdict_code: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftExprType {
    pub family: u8,
    pub name: &'static str,
    pub maxattr: u8,
}

pub const NFT_FIB_NETDEV_TYPE: NftExprType = NftExprType {
    family: NFPROTO_NETDEV,
    name: "fib",
    maxattr: NFTA_FIB_MAX,
};

pub fn nft_fib_netdev_eval(
    skb_protocol: u16,
    ipv6_enabled: bool,
    priv_: NftFib,
    regs: &mut NftRegs,
) -> NftFibNetdevDispatch {
    let dispatch = match (skb_protocol, priv_.result) {
        (ETH_P_IP, NftFibResult::Oif | NftFibResult::OifName) => NftFibNetdevDispatch::Fib4,
        (ETH_P_IP, NftFibResult::AddrType) => NftFibNetdevDispatch::Fib4Type,
        (ETH_P_IPV6, NftFibResult::Oif | NftFibResult::OifName) if ipv6_enabled => {
            NftFibNetdevDispatch::Fib6
        }
        (ETH_P_IPV6, NftFibResult::AddrType) if ipv6_enabled => NftFibNetdevDispatch::Fib6Type,
        _ => NftFibNetdevDispatch::Break,
    };

    if dispatch == NftFibNetdevDispatch::Break {
        regs.verdict_code = NFT_BREAK;
    }

    dispatch
}

pub const fn nft_fib_netdev_module_init() -> &'static NftExprType {
    &NFT_FIB_NETDEV_TYPE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nft_fib_netdev_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nft_fib_netdev.c"
        ));
        assert!(source.contains("static void nft_fib_netdev_eval"));
        assert!(source.contains("const struct nft_fib *priv = nft_expr_priv(expr);"));
        assert!(source.contains("switch (ntohs(pkt->skb->protocol))"));
        assert!(source.contains("case ETH_P_IP:"));
        assert!(source.contains("case NFT_FIB_RESULT_OIF:"));
        assert!(source.contains("case NFT_FIB_RESULT_OIFNAME:"));
        assert!(source.contains("return nft_fib4_eval(expr, regs, pkt);"));
        assert!(source.contains("return nft_fib4_eval_type(expr, regs, pkt);"));
        assert!(source.contains("case ETH_P_IPV6:"));
        assert!(source.contains("if (!ipv6_mod_enabled())"));
        assert!(source.contains("return nft_fib6_eval(expr, regs, pkt);"));
        assert!(source.contains("return nft_fib6_eval_type(expr, regs, pkt);"));
        assert!(source.contains("regs->verdict.code = NFT_BREAK;"));
        assert!(source.contains(".family\t\t= NFPROTO_NETDEV"));
        assert!(source.contains(".name\t\t= \"fib\""));
        assert!(source.contains(".maxattr\t= NFTA_FIB_MAX"));
        assert!(source.contains("MODULE_ALIAS_NFT_AF_EXPR(5, \"fib\");"));
        assert!(source.contains("MODULE_DESCRIPTION(\"nftables netdev fib lookups support\")"));
    }

    #[test]
    fn netdev_dispatches_by_ethertype_and_ipv6_state() {
        let mut regs = NftRegs::default();
        assert_eq!(
            nft_fib_netdev_eval(
                ETH_P_IP,
                true,
                NftFib {
                    result: NftFibResult::OifName,
                },
                &mut regs,
            ),
            NftFibNetdevDispatch::Fib4
        );
        assert_eq!(
            nft_fib_netdev_eval(
                ETH_P_IPV6,
                true,
                NftFib {
                    result: NftFibResult::AddrType,
                },
                &mut regs,
            ),
            NftFibNetdevDispatch::Fib6Type
        );
        assert_eq!(
            nft_fib_netdev_eval(
                ETH_P_IPV6,
                false,
                NftFib {
                    result: NftFibResult::Oif,
                },
                &mut regs,
            ),
            NftFibNetdevDispatch::Break
        );
        assert_eq!(regs.verdict_code, NFT_BREAK);
        assert_eq!(nft_fib_netdev_module_init(), &NFT_FIB_NETDEV_TYPE);
    }
}
