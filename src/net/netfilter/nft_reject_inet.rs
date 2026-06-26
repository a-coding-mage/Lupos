//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nft_reject_inet.c
//! test-origin: linux:vendor/linux/net/netfilter/nft_reject_inet.c
//! Inet-family nftables reject expression dispatch.

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "Patrick McHardy <kaber@trash.net>";
pub const MODULE_ALIAS: &str = "1:reject";
pub const MODULE_DESCRIPTION: &str = "Netfilter nftables reject inet support";
pub const NFPROTO_INET: u8 = 1;
pub const NFPROTO_IPV4: u8 = 2;
pub const NFPROTO_IPV6: u8 = 10;
pub const NF_DROP: i32 = 0;
pub const NF_INET_PRE_ROUTING: u8 = 0;
pub const NF_INET_LOCAL_IN: u8 = 1;
pub const NF_INET_FORWARD: u8 = 2;
pub const NF_INET_LOCAL_OUT: u8 = 3;
pub const NF_INET_INGRESS: u8 = 5;
pub const NFT_REJECT_ICMP_UNREACH: u8 = 0;
pub const NFT_REJECT_TCP_RST: u8 = 1;
pub const NFT_REJECT_ICMPX_UNREACH: u8 = 2;
pub const VALIDATE_HOOKS: u32 = (1 << NF_INET_LOCAL_IN)
    | (1 << NF_INET_FORWARD)
    | (1 << NF_INET_LOCAL_OUT)
    | (1 << NF_INET_PRE_ROUTING)
    | (1 << NF_INET_INGRESS);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftReject {
    pub reject_type: u8,
    pub icmp_code: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RejectAction {
    Ipv4Unreach(u8),
    Ipv4Reset,
    Ipv4Icmpx(u8),
    Ipv6Unreach(u8),
    Ipv6Reset,
    Ipv6Icmpx(u8),
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RejectEvalResult {
    pub action: RejectAction,
    pub verdict: i32,
}

pub const fn nft_reject_icmp_code(code: u8) -> u8 {
    code
}

pub const fn nft_reject_icmpv6_code(code: u8) -> u8 {
    code
}

pub const fn nft_reject_inet_eval(pf: u8, priv_: NftReject) -> RejectEvalResult {
    let action = match (pf, priv_.reject_type) {
        (NFPROTO_IPV4, NFT_REJECT_ICMP_UNREACH) => RejectAction::Ipv4Unreach(priv_.icmp_code),
        (NFPROTO_IPV4, NFT_REJECT_TCP_RST) => RejectAction::Ipv4Reset,
        (NFPROTO_IPV4, NFT_REJECT_ICMPX_UNREACH) => {
            RejectAction::Ipv4Icmpx(nft_reject_icmp_code(priv_.icmp_code))
        }
        (NFPROTO_IPV6, NFT_REJECT_ICMP_UNREACH) => RejectAction::Ipv6Unreach(priv_.icmp_code),
        (NFPROTO_IPV6, NFT_REJECT_TCP_RST) => RejectAction::Ipv6Reset,
        (NFPROTO_IPV6, NFT_REJECT_ICMPX_UNREACH) => {
            RejectAction::Ipv6Icmpx(nft_reject_icmpv6_code(priv_.icmp_code))
        }
        _ => RejectAction::None,
    };
    RejectEvalResult {
        action,
        verdict: NF_DROP,
    }
}

pub const fn nft_reject_inet_validate(chain_validate_ret: i32) -> Result<u32, i32> {
    if chain_validate_ret < 0 {
        Err(chain_validate_ret)
    } else {
        Ok(VALIDATE_HOOKS)
    }
}

pub const fn nft_reject_inet_module_init(register_ret: i32) -> Result<(), i32> {
    if register_ret < 0 {
        Err(register_ret)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nft_reject_inet_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nft_reject_inet.c"
        ));
        assert!(source.contains("nft_reject_inet_eval"));
        assert!(source.contains("switch (nft_pf(pkt))"));
        assert!(source.contains("case NFPROTO_IPV4:"));
        assert!(source.contains("case NFT_REJECT_ICMP_UNREACH:"));
        assert!(source.contains("nf_send_unreach(pkt->skb, priv->icmp_code"));
        assert!(source.contains("nf_send_reset(nft_net(pkt), nft_sk(pkt)"));
        assert!(source.contains("nft_reject_icmp_code(priv->icmp_code)"));
        assert!(source.contains("case NFPROTO_IPV6:"));
        assert!(source.contains("nf_send_unreach6(nft_net(pkt), pkt->skb"));
        assert!(source.contains("nf_send_reset6(nft_net(pkt), nft_sk(pkt)"));
        assert!(source.contains("nft_reject_icmpv6_code(priv->icmp_code)"));
        assert!(source.contains("regs->verdict.code = NF_DROP;"));
        assert!(source.contains("nft_chain_validate_hooks(ctx->chain"));
        assert!(source.contains("(1 << NF_INET_INGRESS)"));
        assert!(source.contains(".family\t\t= NFPROTO_INET"));
        assert!(source.contains(".name\t\t= \"reject\""));
        assert!(source.contains("MODULE_ALIAS_NFT_AF_EXPR(1, \"reject\")"));
    }

    #[test]
    fn reject_inet_dispatches_by_protocol_and_reject_type() {
        assert_eq!(
            nft_reject_inet_eval(
                NFPROTO_IPV4,
                NftReject {
                    reject_type: NFT_REJECT_TCP_RST,
                    icmp_code: 3,
                }
            ),
            RejectEvalResult {
                action: RejectAction::Ipv4Reset,
                verdict: NF_DROP,
            }
        );
        assert_eq!(
            nft_reject_inet_eval(
                NFPROTO_IPV6,
                NftReject {
                    reject_type: NFT_REJECT_ICMPX_UNREACH,
                    icmp_code: 9,
                }
            )
            .action,
            RejectAction::Ipv6Icmpx(9)
        );
        assert_eq!(nft_reject_inet_validate(0), Ok(VALIDATE_HOOKS));
        assert_eq!(nft_reject_inet_validate(-4), Err(-4));
        assert_eq!(nft_reject_inet_module_init(0), Ok(()));
    }
}
