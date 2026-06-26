//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nft_last.c
//! test-origin: linux:vendor/linux/net/netfilter/nft_last.c
//! nftables stateful last-use expression.

use crate::include::uapi::errno::ENOMEM;

pub const NFTA_LAST_SET: u16 = 1;
pub const NFTA_LAST_MSECS: u16 = 2;
pub const NFTA_LAST_PAD: u16 = 3;
pub const NFTA_LAST_MAX: u16 = NFTA_LAST_PAD;
pub const NFT_EXPR_STATEFUL: u32 = 0x1;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NftLast {
    pub jiffies: u64,
    pub set: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NftLastPriv {
    pub last: Option<NftLast>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftLastAttrs {
    pub set: Option<u32>,
    pub msecs_to_jiffies: Option<Result<u64, i32>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftLastDump {
    pub set: u32,
    pub msecs: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftExprOps {
    pub type_name: &'static str,
    pub size: usize,
    pub eval: &'static str,
    pub init: &'static str,
    pub destroy: &'static str,
    pub clone: &'static str,
    pub dump: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftExprType {
    pub name: &'static str,
    pub maxattr: u16,
    pub flags: u32,
    pub ops: &'static NftExprOps,
}

pub const NFT_LAST_OPS: NftExprOps = NftExprOps {
    type_name: "nft_last_type",
    size: core::mem::size_of::<NftLastPriv>(),
    eval: "nft_last_eval",
    init: "nft_last_init",
    destroy: "nft_last_destroy",
    clone: "nft_last_clone",
    dump: "nft_last_dump",
};

pub const NFT_LAST_TYPE: NftExprType = NftExprType {
    name: "last",
    maxattr: NFTA_LAST_MAX,
    flags: NFT_EXPR_STATEFUL,
    ops: &NFT_LAST_OPS,
};

pub const fn nft_last_init(
    attrs: NftLastAttrs,
    now_jiffies: u64,
    alloc_ok: bool,
) -> Result<NftLastPriv, i32> {
    if !alloc_ok {
        return Err(-ENOMEM);
    }
    let mut last = NftLast { jiffies: 0, set: 0 };
    if let Some(set) = attrs.set {
        last.set = set;
    }
    if last.set != 0 {
        if let Some(converted) = attrs.msecs_to_jiffies {
            match converted {
                Ok(last_jiffies) => {
                    last.jiffies = now_jiffies.wrapping_sub(last_jiffies);
                }
                Err(err) => return Err(err),
            }
        }
    }
    Ok(NftLastPriv { last: Some(last) })
}

pub const fn nft_last_eval(priv_data: &mut NftLastPriv, now_jiffies: u64) {
    if let Some(mut last) = priv_data.last {
        if last.jiffies != now_jiffies {
            last.jiffies = now_jiffies;
        }
        if last.set == 0 {
            last.set = 1;
        }
        priv_data.last = Some(last);
    }
}

pub const fn nft_last_dump(
    priv_data: &mut NftLastPriv,
    now_jiffies: u64,
) -> Result<NftLastDump, i32> {
    if let Some(mut last) = priv_data.last {
        let mut last_jiffies = last.jiffies;
        let mut last_set = last.set;
        if time_before(now_jiffies, last_jiffies) {
            last.set = 0;
            last_set = 0;
            priv_data.last = Some(last);
        }
        let msecs = if last_set != 0 {
            now_jiffies.wrapping_sub(last_jiffies)
        } else {
            last_jiffies = 0;
            last_jiffies
        };
        Ok(NftLastDump {
            set: last_set,
            msecs,
        })
    } else {
        Err(-ENOMEM)
    }
}

pub const fn nft_last_destroy(priv_data: &mut NftLastPriv) {
    priv_data.last = None;
}

pub const fn nft_last_clone(src: &NftLastPriv, alloc_ok: bool) -> Result<NftLastPriv, i32> {
    if !alloc_ok {
        return Err(-ENOMEM);
    }
    Ok(NftLastPriv { last: src.last })
}

pub const fn nft_last_type() -> &'static NftExprType {
    &NFT_LAST_TYPE
}

const fn time_before(a: u64, b: u64) -> bool {
    (a.wrapping_sub(b) as i64 as i128) < 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nft_last_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nft_last.c"
        ));
        assert!(source.contains("struct nft_last"));
        assert!(source.contains("unsigned long\tjiffies;"));
        assert!(source.contains("unsigned int\tset;"));
        assert!(source.contains("struct nft_last_priv"));
        assert!(source.contains("[NFTA_LAST_SET] = { .type = NLA_U32 }"));
        assert!(source.contains("[NFTA_LAST_MSECS] = { .type = NLA_U64 }"));
        assert!(source.contains("last = kzalloc_obj(*last, GFP_KERNEL_ACCOUNT);"));
        assert!(source.contains("if (tb[NFTA_LAST_SET])"));
        assert!(source.contains("last->set = ntohl(nla_get_be32(tb[NFTA_LAST_SET]));"));
        assert!(source.contains("if (last->set && tb[NFTA_LAST_MSECS])"));
        assert!(
            source.contains("err = nf_msecs_to_jiffies64(tb[NFTA_LAST_MSECS], &last_jiffies);")
        );
        assert!(source.contains("last->jiffies = jiffies - (unsigned long)last_jiffies;"));
        assert!(source.contains("if (READ_ONCE(last->jiffies) != jiffies)"));
        assert!(source.contains("WRITE_ONCE(last->jiffies, jiffies);"));
        assert!(source.contains("if (READ_ONCE(last->set) == 0)"));
        assert!(source.contains("WRITE_ONCE(last->set, 1);"));
        assert!(source.contains("if (time_before(jiffies, last_jiffies))"));
        assert!(source.contains("WRITE_ONCE(last->set, 0);"));
        assert!(source.contains("msecs = nf_jiffies64_to_msecs(jiffies - last_jiffies);"));
        assert!(source.contains("nla_put_be32(skb, NFTA_LAST_SET, htonl(last_set))"));
        assert!(source.contains("nla_put_be64(skb, NFTA_LAST_MSECS, msecs, NFTA_LAST_PAD)"));
        assert!(source.contains("kfree(priv->last);"));
        assert!(source.contains("priv_dst->last->set = priv_src->last->set;"));
        assert!(source.contains("priv_dst->last->jiffies = priv_src->last->jiffies;"));
        assert!(source.contains(".name\t\t= \"last\""));
        assert!(source.contains(".flags\t\t= NFT_EXPR_STATEFUL"));
    }

    #[test]
    fn last_init_eval_dump_clone_and_destroy_follow_stateful_flow() {
        let attrs = NftLastAttrs {
            set: Some(1),
            msecs_to_jiffies: Some(Ok(25)),
        };
        let mut priv_data = nft_last_init(attrs, 100, true).unwrap();
        assert_eq!(
            priv_data.last,
            Some(NftLast {
                set: 1,
                jiffies: 75,
            })
        );
        assert_eq!(
            nft_last_dump(&mut priv_data, 125).unwrap(),
            NftLastDump { set: 1, msecs: 50 }
        );
        nft_last_eval(&mut priv_data, 130);
        assert_eq!(
            priv_data.last,
            Some(NftLast {
                set: 1,
                jiffies: 130,
            })
        );
        let mut future = NftLastPriv {
            last: Some(NftLast {
                set: 1,
                jiffies: 200,
            }),
        };
        assert_eq!(
            nft_last_dump(&mut future, 100).unwrap(),
            NftLastDump { set: 0, msecs: 0 }
        );
        assert_eq!(future.last.unwrap().set, 0);
        assert_eq!(nft_last_clone(&priv_data, true).unwrap(), priv_data);
        assert_eq!(nft_last_clone(&priv_data, false), Err(-ENOMEM));
        assert_eq!(
            nft_last_init(
                NftLastAttrs {
                    set: Some(1),
                    msecs_to_jiffies: Some(Err(-5)),
                },
                100,
                true,
            ),
            Err(-5)
        );
        nft_last_destroy(&mut priv_data);
        assert_eq!(priv_data.last, None);
        assert_eq!(nft_last_type(), &NFT_LAST_TYPE);
    }
}
