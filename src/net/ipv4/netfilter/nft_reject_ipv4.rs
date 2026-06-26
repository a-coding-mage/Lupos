//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv4/netfilter/nft_reject_ipv4.c
//! test-origin: linux:vendor/linux/net/ipv4/netfilter/nft_reject_ipv4.c
//! IPv4 nftables reject expression.

pub const NF_DROP: i32 = 0;
pub const NFPROTO_IPV4: u8 = 2;
pub const NFTA_REJECT_MAX: u8 = 2;
pub const MODULE_AUTHOR: &str = "Patrick McHardy <kaber@trash.net>";
pub const MODULE_DESCRIPTION: &str = "IPv4 packet rejection for nftables";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIAS: &str = "nft-afinfo-2-reject";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NftRejectType {
    IcmpUnreach,
    TcpRst,
    Other(u8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftReject {
    pub reject_type: NftRejectType,
    pub icmp_code: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NftRejectAction {
    SendUnreach { code: u8 },
    SendReset,
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftRejectEval {
    pub action: NftRejectAction,
    pub verdict_code: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftExprType {
    pub family: u8,
    pub name: &'static str,
    pub maxattr: u8,
}

pub const NFT_REJECT_IPV4_TYPE: NftExprType = NftExprType {
    family: NFPROTO_IPV4,
    name: "reject",
    maxattr: NFTA_REJECT_MAX,
};

pub const fn nft_reject_ipv4_eval(priv_: NftReject) -> NftRejectEval {
    let action = match priv_.reject_type {
        NftRejectType::IcmpUnreach => NftRejectAction::SendUnreach {
            code: priv_.icmp_code,
        },
        NftRejectType::TcpRst => NftRejectAction::SendReset,
        NftRejectType::Other(_) => NftRejectAction::None,
    };
    NftRejectEval {
        action,
        verdict_code: NF_DROP,
    }
}

pub const fn nft_reject_ipv4_module_init() -> &'static NftExprType {
    &NFT_REJECT_IPV4_TYPE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nft_reject_ipv4_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv4/netfilter/nft_reject_ipv4.c"
        ));
        assert!(source.contains("static void nft_reject_ipv4_eval"));
        assert!(source.contains("struct nft_reject *priv = nft_expr_priv(expr);"));
        assert!(source.contains("case NFT_REJECT_ICMP_UNREACH:"));
        assert!(source.contains("nf_send_unreach(pkt->skb, priv->icmp_code, nft_hook(pkt));"));
        assert!(source.contains("case NFT_REJECT_TCP_RST:"));
        assert!(source.contains("nf_send_reset(nft_net(pkt), nft_sk(pkt), pkt->skb"));
        assert!(source.contains("regs->verdict.code = NF_DROP;"));
        assert!(source.contains(".family\t\t= NFPROTO_IPV4"));
        assert!(source.contains(".name\t\t= \"reject\""));
        assert!(source.contains(".policy\t\t= nft_reject_policy"));
        assert!(source.contains(".maxattr\t= NFTA_REJECT_MAX"));
        assert!(source.contains("nft_register_expr(&nft_reject_ipv4_type);"));
        assert!(source.contains("MODULE_ALIAS_NFT_AF_EXPR(AF_INET, \"reject\");"));
        assert!(source.contains("MODULE_DESCRIPTION(\"IPv4 packet rejection for nftables\")"));
    }

    #[test]
    fn reject_ipv4_eval_sends_family_specific_action_and_drops() {
        assert_eq!(
            nft_reject_ipv4_eval(NftReject {
                reject_type: NftRejectType::IcmpUnreach,
                icmp_code: 3,
            }),
            NftRejectEval {
                action: NftRejectAction::SendUnreach { code: 3 },
                verdict_code: NF_DROP,
            }
        );
        assert_eq!(
            nft_reject_ipv4_eval(NftReject {
                reject_type: NftRejectType::TcpRst,
                icmp_code: 0,
            })
            .action,
            NftRejectAction::SendReset
        );
        assert_eq!(nft_reject_ipv4_module_init(), &NFT_REJECT_IPV4_TYPE);
    }
}
