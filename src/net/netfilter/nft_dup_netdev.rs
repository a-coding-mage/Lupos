//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nft_dup_netdev.c
//! test-origin: linux:vendor/linux/net/netfilter/nft_dup_netdev.c
//! Netdev-family nftables dup expression registration and offload plumbing.

use crate::include::uapi::errno::EINVAL;
use crate::net::netfilter::nf_dup_netdev::{FlowActionEntry, nft_fwd_dup_netdev_offload};

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "Pablo Neira Ayuso <pablo@netfilter.org>";
pub const MODULE_ALIAS: &str = "5:dup";
pub const MODULE_DESCRIPTION: &str = "nftables netdev packet duplication support";
pub const NFPROTO_NETDEV: u8 = 5;
pub const NFTA_DUP_SREG_DEV: usize = 2;
pub const NFTA_DUP_MAX: usize = 2;
pub const FLOW_ACTION_MIRRED: u32 = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftDupNetdev {
    pub sreg_dev: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftExprType {
    pub family: u8,
    pub name: &'static str,
    pub maxattr: usize,
}

pub const NFT_DUP_NETDEV_TYPE: NftExprType = NftExprType {
    family: NFPROTO_NETDEV,
    name: "dup",
    maxattr: NFTA_DUP_MAX,
};

pub const fn nft_dup_netdev_eval(priv_: NftDupNetdev, regs: &[u32; 16]) -> i32 {
    regs[priv_.sreg_dev as usize] as i32
}

pub const fn nft_dup_netdev_init(
    has_dev: bool,
    dev_reg_ret: Result<u8, i32>,
) -> Result<NftDupNetdev, i32> {
    if !has_dev {
        return Err(-EINVAL);
    }
    let sreg_dev = match dev_reg_ret {
        Ok(reg) => reg,
        Err(err) => return Err(err),
    };
    Ok(NftDupNetdev { sreg_dev })
}

pub const fn nft_dup_netdev_dump(dump_dev_ok: bool) -> i32 {
    if dump_dev_ok { 0 } else { -1 }
}

pub const fn nft_dup_netdev_offload(
    priv_: NftDupNetdev,
    regs: &[u32; 16],
    dev_exists: bool,
    entry_available: bool,
) -> Result<FlowActionEntry, i32> {
    let oif = regs[priv_.sreg_dev as usize] as i32;
    nft_fwd_dup_netdev_offload(dev_exists, entry_available, FLOW_ACTION_MIRRED, oif)
}

pub const fn nft_dup_netdev_offload_action() -> bool {
    true
}

pub const fn nft_dup_netdev_module_init(register_ret: i32) -> Result<&'static NftExprType, i32> {
    if register_ret < 0 {
        Err(register_ret)
    } else {
        Ok(&NFT_DUP_NETDEV_TYPE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::include::uapi::errno::{E2BIG, EOPNOTSUPP};

    #[test]
    fn nft_dup_netdev_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nft_dup_netdev.c"
        ));
        assert!(source.contains("struct nft_dup_netdev"));
        assert!(source.contains("u8\tsreg_dev;"));
        assert!(source.contains("static void nft_dup_netdev_eval"));
        assert!(source.contains("int oif = regs->data[priv->sreg_dev];"));
        assert!(source.contains("nf_dup_netdev_egress(pkt, oif);"));
        assert!(source.contains("if (tb[NFTA_DUP_SREG_DEV] == NULL)"));
        assert!(source.contains("nft_parse_register_load(ctx, tb[NFTA_DUP_SREG_DEV]"));
        assert!(source.contains("nft_dump_register(skb, NFTA_DUP_SREG_DEV, priv->sreg_dev)"));
        assert!(source.contains("FLOW_ACTION_MIRRED"));
        assert!(source.contains("return true;"));
        assert!(source.contains(".family\t\t= NFPROTO_NETDEV"));
        assert!(source.contains(".name\t\t= \"dup\""));
        assert!(source.contains(".maxattr\t= NFTA_DUP_MAX"));
        assert!(source.contains("nft_register_expr(&nft_dup_netdev_type);"));
        assert!(source.contains("MODULE_ALIAS_NFT_AF_EXPR(5, \"dup\")"));
        assert!(
            source.contains("MODULE_DESCRIPTION(\"nftables netdev packet duplication support\")")
        );
    }

    #[test]
    fn dup_netdev_init_eval_dump_and_offload_follow_source_paths() {
        assert_eq!(nft_dup_netdev_init(false, Ok(1)), Err(-EINVAL));
        assert_eq!(nft_dup_netdev_init(true, Err(-5)), Err(-5));
        let priv_ = nft_dup_netdev_init(true, Ok(3)).unwrap();
        let mut regs = [0u32; 16];
        regs[3] = 11;

        assert_eq!(nft_dup_netdev_eval(priv_, &regs), 11);
        assert_eq!(nft_dup_netdev_dump(true), 0);
        assert_eq!(nft_dup_netdev_dump(false), -1);
        assert_eq!(
            nft_dup_netdev_offload(priv_, &regs, true, true),
            Ok(FlowActionEntry {
                id: FLOW_ACTION_MIRRED,
                dev_index: 11,
            })
        );
        assert_eq!(
            nft_dup_netdev_offload(priv_, &regs, false, true),
            Err(-EOPNOTSUPP)
        );
        assert_eq!(
            nft_dup_netdev_offload(priv_, &regs, true, false),
            Err(-E2BIG)
        );
        assert!(nft_dup_netdev_offload_action());
        assert_eq!(nft_dup_netdev_module_init(-7), Err(-7));
        assert_eq!(nft_dup_netdev_module_init(0), Ok(&NFT_DUP_NETDEV_TYPE));
    }
}
