//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv4/netfilter/nft_dup_ipv4.c
//! test-origin: linux:vendor/linux/net/ipv4/netfilter/nft_dup_ipv4.c
//! IPv4 nftables dup expression registration and register parsing.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "Pablo Neira Ayuso <pablo@netfilter.org>";
pub const MODULE_ALIAS: &str = "ip:dup";
pub const MODULE_DESCRIPTION: &str = "IPv4 nftables packet duplication support";
pub const NFPROTO_IPV4: u8 = 2;
pub const NFTA_DUP_SREG_ADDR: usize = 1;
pub const NFTA_DUP_SREG_DEV: usize = 2;
pub const NFTA_DUP_MAX: usize = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftDupIpv4 {
    pub sreg_addr: u8,
    pub sreg_dev: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftExprType {
    pub family: u8,
    pub name: &'static str,
    pub maxattr: usize,
}

pub const NFT_DUP_IPV4_TYPE: NftExprType = NftExprType {
    family: NFPROTO_IPV4,
    name: "dup",
    maxattr: NFTA_DUP_MAX,
};

pub const fn nft_dup_ipv4_eval(priv_: NftDupIpv4, regs: &[u32; 16]) -> (u32, i32) {
    let oif = if priv_.sreg_dev != 0 {
        regs[priv_.sreg_dev as usize] as i32
    } else {
        -1
    };
    (regs[priv_.sreg_addr as usize], oif)
}

pub const fn nft_dup_ipv4_init(
    has_addr: bool,
    addr_reg_ret: Result<u8, i32>,
    dev_reg_ret: Option<Result<u8, i32>>,
) -> Result<NftDupIpv4, i32> {
    if !has_addr {
        return Err(-EINVAL);
    }
    let sreg_addr = match addr_reg_ret {
        Ok(reg) => reg,
        Err(err) => return Err(err),
    };
    let sreg_dev = match dev_reg_ret {
        Some(Ok(reg)) => reg,
        Some(Err(err)) => return Err(err),
        None => 0,
    };
    Ok(NftDupIpv4 {
        sreg_addr,
        sreg_dev,
    })
}

pub const fn nft_dup_ipv4_dump(priv_: NftDupIpv4, dump_addr_ok: bool, dump_dev_ok: bool) -> i32 {
    if !dump_addr_ok || (priv_.sreg_dev != 0 && !dump_dev_ok) {
        -1
    } else {
        0
    }
}

pub const fn nft_dup_ipv4_module_init(register_ret: i32) -> Result<&'static NftExprType, i32> {
    if register_ret < 0 {
        Err(register_ret)
    } else {
        Ok(&NFT_DUP_IPV4_TYPE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nft_dup_ipv4_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv4/netfilter/nft_dup_ipv4.c"
        ));
        assert!(source.contains("struct nft_dup_ipv4"));
        assert!(source.contains("u8\tsreg_addr;"));
        assert!(source.contains("u8\tsreg_dev;"));
        assert!(source.contains("nft_dup_ipv4_eval"));
        assert!(source.contains(".s_addr = (__force __be32)regs->data[priv->sreg_addr]"));
        assert!(source.contains("int oif = priv->sreg_dev ? regs->data[priv->sreg_dev] : -1;"));
        assert!(source.contains("nf_dup_ipv4(nft_net(pkt), pkt->skb, nft_hook(pkt), &gw, oif);"));
        assert!(source.contains("if (tb[NFTA_DUP_SREG_ADDR] == NULL)"));
        assert!(source.contains("nft_parse_register_load(ctx, tb[NFTA_DUP_SREG_ADDR]"));
        assert!(source.contains("nft_parse_register_load(ctx, tb[NFTA_DUP_SREG_DEV]"));
        assert!(source.contains("nft_dump_register(skb, NFTA_DUP_SREG_ADDR"));
        assert!(source.contains("nft_dump_register(skb, NFTA_DUP_SREG_DEV"));
        assert!(source.contains(".family\t\t= NFPROTO_IPV4"));
        assert!(source.contains(".name\t\t= \"dup\""));
        assert!(source.contains(".maxattr\t= NFTA_DUP_MAX"));
        assert!(source.contains("nft_register_expr(&nft_dup_ipv4_type);"));
        assert!(source.contains("MODULE_ALIAS_NFT_AF_EXPR(AF_INET, \"dup\")"));
        assert!(
            source.contains("MODULE_DESCRIPTION(\"IPv4 nftables packet duplication support\")")
        );
    }

    #[test]
    fn dup_ipv4_init_eval_and_dump_follow_register_rules() {
        assert_eq!(nft_dup_ipv4_init(false, Ok(1), None), Err(-EINVAL));
        assert_eq!(nft_dup_ipv4_init(true, Err(-5), None), Err(-5));
        assert_eq!(nft_dup_ipv4_init(true, Ok(1), Some(Err(-6))), Err(-6));
        let priv_ = nft_dup_ipv4_init(true, Ok(1), Some(Ok(2))).unwrap();
        let mut regs = [0u32; 16];
        regs[1] = 0xc000_0201;
        regs[2] = 9;
        assert_eq!(nft_dup_ipv4_eval(priv_, &regs), (0xc000_0201, 9));
        assert_eq!(
            nft_dup_ipv4_eval(
                NftDupIpv4 {
                    sreg_dev: 0,
                    ..priv_
                },
                &regs,
            )
            .1,
            -1
        );
        assert_eq!(nft_dup_ipv4_dump(priv_, true, true), 0);
        assert_eq!(nft_dup_ipv4_dump(priv_, false, true), -1);
        assert_eq!(nft_dup_ipv4_dump(priv_, true, false), -1);
        assert_eq!(nft_dup_ipv4_module_init(-7), Err(-7));
        assert_eq!(nft_dup_ipv4_module_init(0), Ok(&NFT_DUP_IPV4_TYPE));
    }
}
