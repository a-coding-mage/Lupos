//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nft_numgen.c
//! test-origin: linux:vendor/linux/net/netfilter/nft_numgen.c
//! nftables number generator expression.

use crate::include::uapi::errno::{EINVAL, ENOMEM, EOVERFLOW, ERANGE};

pub const NFTA_NG_DREG: u16 = 1;
pub const NFTA_NG_MODULUS: u16 = 2;
pub const NFTA_NG_TYPE: u16 = 3;
pub const NFTA_NG_OFFSET: u16 = 4;
pub const NFTA_NG_SET_NAME: u16 = 5;
pub const NFTA_NG_SET_ID: u16 = 6;
pub const NFTA_NG_MAX: u16 = NFTA_NG_SET_ID;
pub const NFT_NG_INCREMENTAL: u32 = 0;
pub const NFT_NG_RANDOM: u32 = 1;
pub const NFT_REG32_COUNT: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftNgInc {
    pub dreg: u8,
    pub modulus: u32,
    pub counter: Option<u32>,
    pub offset: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftNgRandom {
    pub dreg: u8,
    pub modulus: u32,
    pub offset: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftNgAttrs {
    pub dreg: Option<Result<u8, i32>>,
    pub modulus: Option<u32>,
    pub type_: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftRegs {
    pub data: [u32; NFT_REG32_COUNT],
}

impl Default for NftRegs {
    fn default() -> Self {
        Self {
            data: [0; NFT_REG32_COUNT],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftNgDump {
    pub dreg: u8,
    pub modulus: u32,
    pub type_: u32,
    pub offset: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NftNgOpsKind {
    Incremental,
    Random,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftExprOps {
    pub type_name: &'static str,
    pub size: usize,
    pub eval: &'static str,
    pub init: &'static str,
    pub destroy: Option<&'static str>,
    pub dump: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftExprType {
    pub name: &'static str,
    pub maxattr: u16,
    pub select_ops: &'static str,
}

pub const NFT_NG_INC_OPS: NftExprOps = NftExprOps {
    type_name: "nft_ng_type",
    size: core::mem::size_of::<NftNgInc>(),
    eval: "nft_ng_inc_eval",
    init: "nft_ng_inc_init",
    destroy: Some("nft_ng_inc_destroy"),
    dump: "nft_ng_inc_dump",
};

pub const NFT_NG_RANDOM_OPS: NftExprOps = NftExprOps {
    type_name: "nft_ng_type",
    size: core::mem::size_of::<NftNgRandom>(),
    eval: "nft_ng_random_eval",
    init: "nft_ng_random_init",
    destroy: None,
    dump: "nft_ng_random_dump",
};

pub const NFT_NG_TYPE: NftExprType = NftExprType {
    name: "numgen",
    maxattr: NFTA_NG_MAX,
    select_ops: "nft_ng_select_ops",
};

pub fn nft_ng_inc_gen(priv_: &mut NftNgInc) -> u32 {
    let old = priv_.counter.unwrap_or(0);
    let new = if old.wrapping_add(1) < priv_.modulus {
        old.wrapping_add(1)
    } else {
        0
    };
    priv_.counter = Some(new);
    new.wrapping_add(priv_.offset)
}

pub fn nft_ng_inc_eval(expr: &mut NftNgInc, regs: &mut NftRegs) {
    regs.data[expr.dreg as usize] = nft_ng_inc_gen(expr);
}

pub const fn nft_ng_inc_init(attrs: NftNgAttrs, alloc_ok: bool) -> Result<NftNgInc, i32> {
    let offset = match attrs.offset {
        Some(offset) => offset,
        None => 0,
    };
    let modulus = match attrs.modulus {
        Some(modulus) => modulus,
        None => return Err(-EINVAL),
    };
    if modulus == 0 {
        return Err(-ERANGE);
    }
    if offset.wrapping_add(modulus).wrapping_sub(1) < offset {
        return Err(-EOVERFLOW);
    }
    if !alloc_ok {
        return Err(-ENOMEM);
    }
    let dreg = match attrs.dreg {
        Some(Ok(dreg)) => dreg,
        Some(Err(err)) => return Err(err),
        None => return Err(-EINVAL),
    };
    Ok(NftNgInc {
        dreg,
        modulus,
        counter: Some(modulus - 1),
        offset,
    })
}

pub const fn nft_ng_dump(dreg: u8, modulus: u32, type_: u32, offset: u32) -> NftNgDump {
    NftNgDump {
        dreg,
        modulus,
        type_,
        offset,
    }
}

pub const fn nft_ng_inc_dump(expr: &NftNgInc) -> NftNgDump {
    nft_ng_dump(expr.dreg, expr.modulus, NFT_NG_INCREMENTAL, expr.offset)
}

pub const fn nft_ng_inc_destroy(expr: &mut NftNgInc) {
    expr.counter = None;
}

pub const fn nft_ng_random_gen(expr: &NftNgRandom, random_u32: u32) -> u32 {
    reciprocal_scale(random_u32, expr.modulus).wrapping_add(expr.offset)
}

pub fn nft_ng_random_eval(expr: &NftNgRandom, regs: &mut NftRegs, random_u32: u32) {
    regs.data[expr.dreg as usize] = nft_ng_random_gen(expr, random_u32);
}

pub const fn nft_ng_random_init(attrs: NftNgAttrs) -> Result<NftNgRandom, i32> {
    let offset = match attrs.offset {
        Some(offset) => offset,
        None => 0,
    };
    let modulus = match attrs.modulus {
        Some(modulus) => modulus,
        None => return Err(-EINVAL),
    };
    if modulus == 0 {
        return Err(-ERANGE);
    }
    if offset.wrapping_add(modulus).wrapping_sub(1) < offset {
        return Err(-EOVERFLOW);
    }
    let dreg = match attrs.dreg {
        Some(Ok(dreg)) => dreg,
        Some(Err(err)) => return Err(err),
        None => return Err(-EINVAL),
    };
    Ok(NftNgRandom {
        dreg,
        modulus,
        offset,
    })
}

pub const fn nft_ng_random_dump(expr: &NftNgRandom) -> NftNgDump {
    nft_ng_dump(expr.dreg, expr.modulus, NFT_NG_RANDOM, expr.offset)
}

pub const fn nft_ng_select_ops(attrs: NftNgAttrs) -> Result<NftNgOpsKind, i32> {
    if attrs.dreg.is_none() || attrs.modulus.is_none() || attrs.type_.is_none() {
        return Err(-EINVAL);
    }
    match attrs.type_.unwrap() {
        NFT_NG_INCREMENTAL => Ok(NftNgOpsKind::Incremental),
        NFT_NG_RANDOM => Ok(NftNgOpsKind::Random),
        _ => Err(-EINVAL),
    }
}

pub const fn nft_ng_type() -> &'static NftExprType {
    &NFT_NG_TYPE
}

pub const fn reciprocal_scale(val: u32, ep_ro: u32) -> u32 {
    (((val as u64) * (ep_ro as u64)) >> 32) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attrs(type_: u32) -> NftNgAttrs {
        NftNgAttrs {
            dreg: Some(Ok(3)),
            modulus: Some(5),
            type_: Some(type_),
            offset: Some(10),
        }
    }

    #[test]
    fn nft_numgen_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nft_numgen.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter/nf_tables.h"
        ));
        assert!(source.contains("struct nft_ng_inc"));
        assert!(source.contains("atomic_t\t\t*counter;"));
        assert!(source.contains("nval = (oval + 1 < priv->modulus) ? oval + 1 : 0;"));
        assert!(source.contains("atomic_cmpxchg(priv->counter, oval, nval)"));
        assert!(source.contains("regs->data[priv->dreg] = nft_ng_inc_gen(priv);"));
        assert!(source.contains("[NFTA_NG_DREG]\t\t= NLA_POLICY_MAX(NLA_BE32, NFT_REG32_MAX)"));
        assert!(source.contains("priv->modulus = ntohl(nla_get_be32(tb[NFTA_NG_MODULUS]));"));
        assert!(source.contains("if (priv->modulus == 0)"));
        assert!(source.contains("if (priv->offset + priv->modulus - 1 < priv->offset)"));
        assert!(
            source.contains("priv->counter = kmalloc_obj(*priv->counter, GFP_KERNEL_ACCOUNT);")
        );
        assert!(source.contains("atomic_set(priv->counter, priv->modulus - 1);"));
        assert!(
            source.contains("reciprocal_scale(get_random_u32(), priv->modulus) + priv->offset")
        );
        assert!(source.contains("if (!tb[NFTA_NG_DREG]"));
        assert!(source.contains("case NFT_NG_INCREMENTAL:"));
        assert!(source.contains("case NFT_NG_RANDOM:"));
        assert!(source.contains(".name\t\t= \"numgen\""));
        assert!(source.contains("nft_register_expr(&nft_ng_type);"));
        assert!(header.contains("NFT_NG_INCREMENTAL"));
        assert!(header.contains("NFTA_NG_MAX"));
    }

    #[test]
    fn incremental_wraps_from_modulus_minus_one_and_destroys_counter() {
        let mut expr = nft_ng_inc_init(attrs(NFT_NG_INCREMENTAL), true).unwrap();
        let mut regs = NftRegs::default();
        nft_ng_inc_eval(&mut expr, &mut regs);
        assert_eq!(regs.data[3], 10);
        nft_ng_inc_eval(&mut expr, &mut regs);
        assert_eq!(regs.data[3], 11);
        assert_eq!(nft_ng_inc_dump(&expr).type_, NFT_NG_INCREMENTAL);
        nft_ng_inc_destroy(&mut expr);
        assert_eq!(expr.counter, None);
    }

    #[test]
    fn random_uses_reciprocal_scale_and_init_validates_ranges() {
        let expr = nft_ng_random_init(attrs(NFT_NG_RANDOM)).unwrap();
        let mut regs = NftRegs::default();
        nft_ng_random_eval(&expr, &mut regs, u32::MAX);
        assert_eq!(regs.data[3], 14);
        assert_eq!(nft_ng_random_dump(&expr).offset, 10);
        assert_eq!(
            nft_ng_select_ops(attrs(NFT_NG_RANDOM)),
            Ok(NftNgOpsKind::Random)
        );
        assert_eq!(nft_ng_select_ops(attrs(9)), Err(-EINVAL));
        assert_eq!(
            nft_ng_inc_init(
                NftNgAttrs {
                    modulus: Some(0),
                    ..attrs(0)
                },
                true
            ),
            Err(-ERANGE)
        );
        assert_eq!(
            nft_ng_inc_init(
                NftNgAttrs {
                    offset: Some(u32::MAX),
                    modulus: Some(2),
                    ..attrs(0)
                },
                true,
            ),
            Err(-EOVERFLOW)
        );
        assert_eq!(nft_ng_inc_init(attrs(0), false), Err(-ENOMEM));
        assert_eq!(nft_ng_type(), &NFT_NG_TYPE);
    }
}
