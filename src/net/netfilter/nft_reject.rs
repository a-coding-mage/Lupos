//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nft_reject.c
//! test-origin: linux:vendor/linux/net/netfilter/nft_reject.c
//! Shared nftables reject expression validation, parsing, dumping, and ICMP mapping.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "Patrick McHardy <kaber@trash.net>";
pub const MODULE_DESCRIPTION: &str = "Netfilter x_tables over nftables module";
pub const NF_INET_PRE_ROUTING: u8 = 0;
pub const NF_INET_LOCAL_IN: u8 = 1;
pub const NF_INET_FORWARD: u8 = 2;
pub const NF_INET_LOCAL_OUT: u8 = 3;
pub const NFTA_REJECT_TYPE: usize = 1;
pub const NFTA_REJECT_ICMP_CODE: usize = 2;
pub const NFTA_REJECT_MAX: usize = 2;
pub const NFT_REJECT_ICMP_UNREACH: u32 = 0;
pub const NFT_REJECT_TCP_RST: u32 = 1;
pub const NFT_REJECT_ICMPX_UNREACH: u32 = 2;
pub const NFT_REJECT_ICMPX_NO_ROUTE: u8 = 0;
pub const NFT_REJECT_ICMPX_PORT_UNREACH: u8 = 1;
pub const NFT_REJECT_ICMPX_HOST_UNREACH: u8 = 2;
pub const NFT_REJECT_ICMPX_ADMIN_PROHIBITED: u8 = 3;
pub const NFT_REJECT_ICMPX_MAX: u8 = 3;
pub const ICMP_NET_UNREACH: u8 = 0;
pub const ICMP_HOST_UNREACH: u8 = 1;
pub const ICMP_PORT_UNREACH: u8 = 3;
pub const ICMP_PKT_FILTERED: u8 = 13;
pub const ICMPV6_NOROUTE: u8 = 0;
pub const ICMPV6_ADM_PROHIBITED: u8 = 1;
pub const ICMPV6_ADDR_UNREACH: u8 = 3;
pub const ICMPV6_PORT_UNREACH: u8 = 4;
pub const VALIDATE_HOOKS: u32 = (1 << NF_INET_LOCAL_IN)
    | (1 << NF_INET_FORWARD)
    | (1 << NF_INET_LOCAL_OUT)
    | (1 << NF_INET_PRE_ROUTING);

pub const ICMP_CODE_V4: [u8; 4] = [
    ICMP_NET_UNREACH,
    ICMP_PORT_UNREACH,
    ICMP_HOST_UNREACH,
    ICMP_PKT_FILTERED,
];
pub const ICMP_CODE_V6: [u8; 4] = [
    ICMPV6_NOROUTE,
    ICMPV6_PORT_UNREACH,
    ICMPV6_ADDR_UNREACH,
    ICMPV6_ADM_PROHIBITED,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftReject {
    pub reject_type: u32,
    pub icmp_code: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftRejectDump {
    pub reject_type: u32,
    pub icmp_code: Option<u8>,
}

pub const fn nft_reject_validate(chain_validate_ret: i32) -> Result<u32, i32> {
    if chain_validate_ret < 0 {
        Err(chain_validate_ret)
    } else {
        Ok(VALIDATE_HOOKS)
    }
}

pub const fn nft_reject_init(
    reject_type: Option<u32>,
    icmp_code: Option<u8>,
) -> Result<NftReject, i32> {
    let reject_type = match reject_type {
        Some(reject_type) => reject_type,
        None => return Err(-EINVAL),
    };

    match reject_type {
        NFT_REJECT_ICMP_UNREACH | NFT_REJECT_ICMPX_UNREACH => {
            let icmp_code = match icmp_code {
                Some(icmp_code) => icmp_code,
                None => return Err(-EINVAL),
            };
            if reject_type == NFT_REJECT_ICMPX_UNREACH && icmp_code > NFT_REJECT_ICMPX_MAX {
                return Err(-EINVAL);
            }
            Ok(NftReject {
                reject_type,
                icmp_code,
            })
        }
        NFT_REJECT_TCP_RST => Ok(NftReject {
            reject_type,
            icmp_code: 0,
        }),
        _ => Err(-EINVAL),
    }
}

pub const fn nft_reject_dump(
    priv_: NftReject,
    put_type_ok: bool,
    put_icmp_code_ok: bool,
) -> Result<NftRejectDump, i32> {
    if !put_type_ok {
        return Err(-1);
    }
    match priv_.reject_type {
        NFT_REJECT_ICMP_UNREACH | NFT_REJECT_ICMPX_UNREACH => {
            if !put_icmp_code_ok {
                return Err(-1);
            }
            Ok(NftRejectDump {
                reject_type: priv_.reject_type,
                icmp_code: Some(priv_.icmp_code),
            })
        }
        _ => Ok(NftRejectDump {
            reject_type: priv_.reject_type,
            icmp_code: None,
        }),
    }
}

pub const fn nft_reject_icmp_code(code: u8) -> u8 {
    if code > NFT_REJECT_ICMPX_MAX {
        ICMP_NET_UNREACH
    } else {
        ICMP_CODE_V4[code as usize]
    }
}

pub const fn nft_reject_icmpv6_code(code: u8) -> u8 {
    if code > NFT_REJECT_ICMPX_MAX {
        ICMPV6_NOROUTE
    } else {
        ICMP_CODE_V6[code as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nft_reject_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nft_reject.c"
        ));
        assert!(source.contains("const struct nla_policy nft_reject_policy"));
        assert!(source.contains("[NFTA_REJECT_TYPE]\t\t= NLA_POLICY_MAX(NLA_BE32, 255)"));
        assert!(source.contains("[NFTA_REJECT_ICMP_CODE]\t\t= { .type = NLA_U8 }"));
        assert!(source.contains("return nft_chain_validate_hooks(ctx->chain"));
        assert!(source.contains("(1 << NF_INET_LOCAL_IN) |"));
        assert!(source.contains("(1 << NF_INET_PRE_ROUTING));"));
        assert!(source.contains("if (tb[NFTA_REJECT_TYPE] == NULL)"));
        assert!(source.contains("priv->type = ntohl(nla_get_be32(tb[NFTA_REJECT_TYPE]));"));
        assert!(source.contains("case NFT_REJECT_ICMP_UNREACH:"));
        assert!(source.contains("case NFT_REJECT_ICMPX_UNREACH:"));
        assert!(source.contains("if (tb[NFTA_REJECT_ICMP_CODE] == NULL)"));
        assert!(source.contains("icmp_code = nla_get_u8(tb[NFTA_REJECT_ICMP_CODE]);"));
        assert!(source.contains("icmp_code > NFT_REJECT_ICMPX_MAX"));
        assert!(source.contains("case NFT_REJECT_TCP_RST:"));
        assert!(source.contains("nla_put_be32(skb, NFTA_REJECT_TYPE, htonl(priv->type))"));
        assert!(source.contains("nla_put_u8(skb, NFTA_REJECT_ICMP_CODE, priv->icmp_code)"));
        assert!(source.contains("[NFT_REJECT_ICMPX_NO_ROUTE]\t\t= ICMP_NET_UNREACH"));
        assert!(source.contains("[NFT_REJECT_ICMPX_ADMIN_PROHIBITED]\t= ICMP_PKT_FILTERED"));
        assert!(source.contains("[NFT_REJECT_ICMPX_PORT_UNREACH]\t\t= ICMPV6_PORT_UNREACH"));
        assert!(source.contains("return ICMP_NET_UNREACH;"));
        assert!(source.contains("return ICMPV6_NOROUTE;"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Netfilter x_tables over nftables module\")"));
    }

    #[test]
    fn reject_validate_init_dump_and_icmp_maps_follow_linux() {
        assert_eq!(nft_reject_validate(0), Ok(VALIDATE_HOOKS));
        assert_eq!(nft_reject_validate(-4), Err(-4));
        assert_eq!(nft_reject_init(None, Some(1)), Err(-EINVAL));
        assert_eq!(
            nft_reject_init(Some(NFT_REJECT_ICMP_UNREACH), None),
            Err(-EINVAL)
        );
        assert_eq!(
            nft_reject_init(
                Some(NFT_REJECT_ICMPX_UNREACH),
                Some(NFT_REJECT_ICMPX_MAX + 1)
            ),
            Err(-EINVAL)
        );
        assert_eq!(
            nft_reject_init(Some(NFT_REJECT_TCP_RST), None),
            Ok(NftReject {
                reject_type: NFT_REJECT_TCP_RST,
                icmp_code: 0,
            })
        );
        let reject = nft_reject_init(
            Some(NFT_REJECT_ICMPX_UNREACH),
            Some(NFT_REJECT_ICMPX_ADMIN_PROHIBITED),
        )
        .unwrap();
        assert_eq!(
            nft_reject_dump(reject, true, true),
            Ok(NftRejectDump {
                reject_type: NFT_REJECT_ICMPX_UNREACH,
                icmp_code: Some(NFT_REJECT_ICMPX_ADMIN_PROHIBITED),
            })
        );
        assert_eq!(nft_reject_dump(reject, false, true), Err(-1));
        assert_eq!(nft_reject_dump(reject, true, false), Err(-1));
        assert_eq!(
            nft_reject_dump(
                NftReject {
                    reject_type: NFT_REJECT_TCP_RST,
                    icmp_code: 99,
                },
                true,
                false,
            ),
            Ok(NftRejectDump {
                reject_type: NFT_REJECT_TCP_RST,
                icmp_code: None,
            })
        );

        assert_eq!(
            nft_reject_icmp_code(NFT_REJECT_ICMPX_NO_ROUTE),
            ICMP_NET_UNREACH
        );
        assert_eq!(
            nft_reject_icmp_code(NFT_REJECT_ICMPX_PORT_UNREACH),
            ICMP_PORT_UNREACH
        );
        assert_eq!(
            nft_reject_icmp_code(NFT_REJECT_ICMPX_HOST_UNREACH),
            ICMP_HOST_UNREACH
        );
        assert_eq!(
            nft_reject_icmp_code(NFT_REJECT_ICMPX_ADMIN_PROHIBITED),
            ICMP_PKT_FILTERED
        );
        assert_eq!(nft_reject_icmp_code(9), ICMP_NET_UNREACH);
        assert_eq!(
            nft_reject_icmpv6_code(NFT_REJECT_ICMPX_NO_ROUTE),
            ICMPV6_NOROUTE
        );
        assert_eq!(
            nft_reject_icmpv6_code(NFT_REJECT_ICMPX_PORT_UNREACH),
            ICMPV6_PORT_UNREACH
        );
        assert_eq!(
            nft_reject_icmpv6_code(NFT_REJECT_ICMPX_HOST_UNREACH),
            ICMPV6_ADDR_UNREACH
        );
        assert_eq!(
            nft_reject_icmpv6_code(NFT_REJECT_ICMPX_ADMIN_PROHIBITED),
            ICMPV6_ADM_PROHIBITED
        );
        assert_eq!(nft_reject_icmpv6_code(9), ICMPV6_NOROUTE);
    }
}
