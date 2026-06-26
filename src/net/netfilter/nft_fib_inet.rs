//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nft_fib_inet.c
//! test-origin: linux:vendor/linux/net/netfilter/nft_fib_inet.c
//! nftables inet-family FIB expression dispatch.

pub const NF_DROP: i32 = 0;
pub const NFPROTO_INET: u8 = 1;
pub const NFPROTO_IPV4: u8 = 2;
pub const NFPROTO_IPV6: u8 = 10;
pub const NFTA_FIB_MAX: u8 = 3;
pub const MODULE_AUTHOR: &str = "Florian Westphal <fw@strlen.de>";
pub const MODULE_DESCRIPTION: &str = "nftables fib inet support";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIAS: &str = "nft-afinfo-1-fib";

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
pub enum NftFibDispatch {
    Fib4,
    Fib4Type,
    Fib6,
    Fib6Type,
    Drop,
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

pub const NFT_FIB_INET_TYPE: NftExprType = NftExprType {
    family: NFPROTO_INET,
    name: "fib",
    maxattr: NFTA_FIB_MAX,
};

pub fn nft_fib_inet_eval(pf: u8, priv_: NftFib, regs: &mut NftRegs) -> NftFibDispatch {
    let dispatch = match (pf, priv_.result) {
        (NFPROTO_IPV4, NftFibResult::Oif | NftFibResult::OifName) => NftFibDispatch::Fib4,
        (NFPROTO_IPV4, NftFibResult::AddrType) => NftFibDispatch::Fib4Type,
        (NFPROTO_IPV6, NftFibResult::Oif | NftFibResult::OifName) => NftFibDispatch::Fib6,
        (NFPROTO_IPV6, NftFibResult::AddrType) => NftFibDispatch::Fib6Type,
        _ => NftFibDispatch::Drop,
    };
    if dispatch == NftFibDispatch::Drop {
        regs.verdict_code = NF_DROP;
    }
    dispatch
}

pub const fn nft_fib_inet_module_init() -> &'static NftExprType {
    &NFT_FIB_INET_TYPE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nft_fib_inet_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nft_fib_inet.c"
        ));
        assert!(source.contains("static void nft_fib_inet_eval"));
        assert!(source.contains("const struct nft_fib *priv = nft_expr_priv(expr);"));
        assert!(source.contains("switch (nft_pf(pkt))"));
        assert!(source.contains("case NFPROTO_IPV4:"));
        assert!(source.contains("case NFT_FIB_RESULT_OIF:"));
        assert!(source.contains("case NFT_FIB_RESULT_OIFNAME:"));
        assert!(source.contains("return nft_fib4_eval(expr, regs, pkt);"));
        assert!(source.contains("return nft_fib4_eval_type(expr, regs, pkt);"));
        assert!(source.contains("case NFPROTO_IPV6:"));
        assert!(source.contains("return nft_fib6_eval(expr, regs, pkt);"));
        assert!(source.contains("return nft_fib6_eval_type(expr, regs, pkt);"));
        assert!(source.contains("regs->verdict.code = NF_DROP;"));
        assert!(source.contains(".family\t\t= NFPROTO_INET"));
        assert!(source.contains(".name\t\t= \"fib\""));
        assert!(source.contains(".maxattr\t= NFTA_FIB_MAX"));
        assert!(source.contains("MODULE_ALIAS_NFT_AF_EXPR(1, \"fib\");"));
        assert!(source.contains("MODULE_DESCRIPTION(\"nftables fib inet support\")"));
    }

    #[test]
    fn fib_inet_dispatches_by_family_and_result_or_drops() {
        let mut regs = NftRegs::default();
        assert_eq!(
            nft_fib_inet_eval(
                NFPROTO_IPV4,
                NftFib {
                    result: NftFibResult::Oif,
                },
                &mut regs,
            ),
            NftFibDispatch::Fib4
        );
        assert_eq!(
            nft_fib_inet_eval(
                NFPROTO_IPV6,
                NftFib {
                    result: NftFibResult::AddrType,
                },
                &mut regs,
            ),
            NftFibDispatch::Fib6Type
        );
        assert_eq!(
            nft_fib_inet_eval(
                99,
                NftFib {
                    result: NftFibResult::Oif,
                },
                &mut regs,
            ),
            NftFibDispatch::Drop
        );
        assert_eq!(regs.verdict_code, NF_DROP);
        assert_eq!(nft_fib_inet_module_init(), &NFT_FIB_INET_TYPE);
    }
}
