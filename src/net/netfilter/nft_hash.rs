//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nft_hash.c
//! test-origin: linux:vendor/linux/net/netfilter/nft_hash.c
//! nftables Jenkins and symmetric hash expression.

use crate::include::uapi::errno::{EINVAL, EOPNOTSUPP, EOVERFLOW, ERANGE};

pub const NFTA_HASH_SREG: u16 = 1;
pub const NFTA_HASH_DREG: u16 = 2;
pub const NFTA_HASH_LEN: u16 = 3;
pub const NFTA_HASH_MODULUS: u16 = 4;
pub const NFTA_HASH_SEED: u16 = 5;
pub const NFTA_HASH_OFFSET: u16 = 6;
pub const NFTA_HASH_TYPE: u16 = 7;
pub const NFTA_HASH_SET_NAME: u16 = 8;
pub const NFTA_HASH_SET_ID: u16 = 9;
pub const NFTA_HASH_MAX: u16 = NFTA_HASH_SET_ID;
pub const NFT_HASH_JENKINS: u32 = 0;
pub const NFT_HASH_SYM: u32 = 1;
pub const NFT_REG32_COUNT: usize = 16;
pub const NFT_DATA_VALUE_MAXLEN: usize = NFT_REG32_COUNT * core::mem::size_of::<u32>();
const JHASH_INITVAL: u32 = 0xdead_beef;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftJhash {
    pub sreg: u8,
    pub dreg: u8,
    pub len: u8,
    pub autogen_seed: bool,
    pub modulus: u32,
    pub seed: u32,
    pub offset: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftSymhash {
    pub dreg: u8,
    pub modulus: u32,
    pub offset: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftHashAttrs {
    pub sreg: Option<Result<u8, i32>>,
    pub dreg: Option<Result<u8, i32>>,
    pub len: Option<u32>,
    pub modulus: Option<u32>,
    pub seed: Option<u32>,
    pub offset: Option<u32>,
    pub type_: Option<u32>,
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
pub struct NftHashDump {
    pub sreg: Option<u8>,
    pub dreg: u8,
    pub len: Option<u8>,
    pub modulus: u32,
    pub seed: Option<u32>,
    pub offset: Option<u32>,
    pub type_: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NftHashOpsKind {
    Jenkins,
    Symmetric,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftExprOps {
    pub type_name: &'static str,
    pub size: usize,
    pub eval: &'static str,
    pub init: &'static str,
    pub dump: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftExprType {
    pub name: &'static str,
    pub maxattr: u16,
    pub select_ops: &'static str,
}

pub const NFT_JHASH_OPS: NftExprOps = NftExprOps {
    type_name: "nft_hash_type",
    size: core::mem::size_of::<NftJhash>(),
    eval: "nft_jhash_eval",
    init: "nft_jhash_init",
    dump: "nft_jhash_dump",
};

pub const NFT_SYMHASH_OPS: NftExprOps = NftExprOps {
    type_name: "nft_hash_type",
    size: core::mem::size_of::<NftSymhash>(),
    eval: "nft_symhash_eval",
    init: "nft_symhash_init",
    dump: "nft_symhash_dump",
};

pub const NFT_HASH_TYPE: NftExprType = NftExprType {
    name: "hash",
    maxattr: NFTA_HASH_MAX,
    select_ops: "nft_hash_select_ops",
};

pub fn nft_jhash_eval(expr: &NftJhash, regs: &mut NftRegs) {
    let data = regs_as_bytes(regs);
    let start = expr.sreg as usize * 4;
    let hash = jhash(&data[start..start + expr.len as usize], expr.seed);
    let h = reciprocal_scale(hash, expr.modulus);
    regs.data[expr.dreg as usize] = h.wrapping_add(expr.offset);
}

pub fn nft_symhash_eval(expr: &NftSymhash, regs: &mut NftRegs, symmetric_skb_hash: u32) {
    let h = reciprocal_scale(symmetric_skb_hash, expr.modulus);
    regs.data[expr.dreg as usize] = h.wrapping_add(expr.offset);
}

pub const fn nft_jhash_init(attrs: NftHashAttrs, random_seed: u32) -> Result<NftJhash, i32> {
    if attrs.sreg.is_none()
        || attrs.dreg.is_none()
        || attrs.len.is_none()
        || attrs.modulus.is_none()
    {
        return Err(-EINVAL);
    }
    let offset = match attrs.offset {
        Some(offset) => offset,
        None => 0,
    };
    let len_value = attrs.len.unwrap();
    if len_value > u8::MAX as u32 {
        return Err(-ERANGE);
    }
    if len_value == 0 {
        return Err(-ERANGE);
    }
    let len = len_value as u8;
    let sreg = match attrs.sreg.unwrap() {
        Ok(sreg) => sreg,
        Err(err) => return Err(err),
    };
    let modulus = attrs.modulus.unwrap();
    if modulus < 1 {
        return Err(-ERANGE);
    }
    if offset.wrapping_add(modulus).wrapping_sub(1) < offset {
        return Err(-EOVERFLOW);
    }
    let (seed, autogen_seed) = match attrs.seed {
        Some(seed) => (seed, false),
        None => (random_seed, true),
    };
    let dreg = match attrs.dreg.unwrap() {
        Ok(dreg) => dreg,
        Err(err) => return Err(err),
    };
    Ok(NftJhash {
        sreg,
        dreg,
        len,
        autogen_seed,
        modulus,
        seed,
        offset,
    })
}

pub const fn nft_symhash_init(attrs: NftHashAttrs) -> Result<NftSymhash, i32> {
    if attrs.dreg.is_none() || attrs.modulus.is_none() {
        return Err(-EINVAL);
    }
    let offset = match attrs.offset {
        Some(offset) => offset,
        None => 0,
    };
    let modulus = attrs.modulus.unwrap();
    if modulus < 1 {
        return Err(-ERANGE);
    }
    if offset.wrapping_add(modulus).wrapping_sub(1) < offset {
        return Err(-EOVERFLOW);
    }
    let dreg = match attrs.dreg.unwrap() {
        Ok(dreg) => dreg,
        Err(err) => return Err(err),
    };
    Ok(NftSymhash {
        dreg,
        modulus,
        offset,
    })
}

pub const fn nft_jhash_dump(expr: &NftJhash) -> NftHashDump {
    NftHashDump {
        sreg: Some(expr.sreg),
        dreg: expr.dreg,
        len: Some(expr.len),
        modulus: expr.modulus,
        seed: if expr.autogen_seed {
            None
        } else {
            Some(expr.seed)
        },
        offset: if expr.offset == 0 {
            None
        } else {
            Some(expr.offset)
        },
        type_: NFT_HASH_JENKINS,
    }
}

pub const fn nft_symhash_dump(expr: &NftSymhash) -> NftHashDump {
    NftHashDump {
        sreg: None,
        dreg: expr.dreg,
        len: None,
        modulus: expr.modulus,
        seed: None,
        offset: if expr.offset == 0 {
            None
        } else {
            Some(expr.offset)
        },
        type_: NFT_HASH_SYM,
    }
}

pub const fn nft_hash_select_ops(attrs: NftHashAttrs) -> Result<NftHashOpsKind, i32> {
    match attrs.type_ {
        None => Ok(NftHashOpsKind::Jenkins),
        Some(NFT_HASH_SYM) => Ok(NftHashOpsKind::Symmetric),
        Some(NFT_HASH_JENKINS) => Ok(NftHashOpsKind::Jenkins),
        Some(_) => Err(-EOPNOTSUPP),
    }
}

pub const fn nft_hash_type() -> &'static NftExprType {
    &NFT_HASH_TYPE
}

pub const fn reciprocal_scale(val: u32, ep_ro: u32) -> u32 {
    (((val as u64) * (ep_ro as u64)) >> 32) as u32
}

pub fn jhash(key: &[u8], initval: u32) -> u32 {
    let mut length = key.len();
    let mut offset = 0usize;
    let mut a = JHASH_INITVAL
        .wrapping_add(length as u32)
        .wrapping_add(initval);
    let mut b = a;
    let mut c = a;

    while length > 12 {
        a = a.wrapping_add(read_le_u32(key, offset));
        b = b.wrapping_add(read_le_u32(key, offset + 4));
        c = c.wrapping_add(read_le_u32(key, offset + 8));
        (a, b, c) = jhash_mix(a, b, c);
        offset += 12;
        length -= 12;
    }

    let tail = &key[offset..];
    if length >= 12 {
        c = c.wrapping_add((tail[11] as u32) << 24);
    }
    if length >= 11 {
        c = c.wrapping_add((tail[10] as u32) << 16);
    }
    if length >= 10 {
        c = c.wrapping_add((tail[9] as u32) << 8);
    }
    if length >= 9 {
        c = c.wrapping_add(tail[8] as u32);
    }
    if length >= 8 {
        b = b.wrapping_add((tail[7] as u32) << 24);
    }
    if length >= 7 {
        b = b.wrapping_add((tail[6] as u32) << 16);
    }
    if length >= 6 {
        b = b.wrapping_add((tail[5] as u32) << 8);
    }
    if length >= 5 {
        b = b.wrapping_add(tail[4] as u32);
    }
    if length >= 4 {
        a = a.wrapping_add((tail[3] as u32) << 24);
    }
    if length >= 3 {
        a = a.wrapping_add((tail[2] as u32) << 16);
    }
    if length >= 2 {
        a = a.wrapping_add((tail[1] as u32) << 8);
    }
    if length >= 1 {
        a = a.wrapping_add(tail[0] as u32);
        (_, _, c) = jhash_final(a, b, c);
    }

    c
}

fn read_le_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn jhash_mix(mut a: u32, mut b: u32, mut c: u32) -> (u32, u32, u32) {
    a = a.wrapping_sub(c);
    a ^= c.rotate_left(4);
    c = c.wrapping_add(b);
    b = b.wrapping_sub(a);
    b ^= a.rotate_left(6);
    a = a.wrapping_add(c);
    c = c.wrapping_sub(b);
    c ^= b.rotate_left(8);
    b = b.wrapping_add(a);
    a = a.wrapping_sub(c);
    a ^= c.rotate_left(16);
    c = c.wrapping_add(b);
    b = b.wrapping_sub(a);
    b ^= a.rotate_left(19);
    a = a.wrapping_add(c);
    c = c.wrapping_sub(b);
    c ^= b.rotate_left(4);
    b = b.wrapping_add(a);
    (a, b, c)
}

fn jhash_final(mut a: u32, mut b: u32, mut c: u32) -> (u32, u32, u32) {
    c ^= b;
    c = c.wrapping_sub(b.rotate_left(14));
    a ^= c;
    a = a.wrapping_sub(c.rotate_left(11));
    b ^= a;
    b = b.wrapping_sub(a.rotate_left(25));
    c ^= b;
    c = c.wrapping_sub(b.rotate_left(16));
    a ^= c;
    a = a.wrapping_sub(c.rotate_left(4));
    b ^= a;
    b = b.wrapping_sub(a.rotate_left(14));
    c ^= b;
    c = c.wrapping_sub(b.rotate_left(24));
    (a, b, c)
}

fn regs_as_bytes(regs: &NftRegs) -> [u8; NFT_DATA_VALUE_MAXLEN] {
    let mut out = [0u8; NFT_DATA_VALUE_MAXLEN];
    let mut i = 0usize;
    while i < NFT_REG32_COUNT {
        let bytes = regs.data[i].to_ne_bytes();
        out[i * 4] = bytes[0];
        out[i * 4 + 1] = bytes[1];
        out[i * 4 + 2] = bytes[2];
        out[i * 4 + 3] = bytes[3];
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attrs(type_: Option<u32>) -> NftHashAttrs {
        NftHashAttrs {
            sreg: Some(Ok(1)),
            dreg: Some(Ok(4)),
            len: Some(8),
            modulus: Some(7),
            seed: Some(0x1234),
            offset: Some(3),
            type_,
        }
    }

    #[test]
    fn nft_hash_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nft_hash.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter/nf_tables.h"
        ));
        let jhash_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/jhash.h"
        ));
        assert!(source.contains("struct nft_jhash"));
        assert!(source.contains("bool\t\t\tautogen_seed:1;"));
        assert!(source.contains("reciprocal_scale(jhash(data, priv->len, priv->seed),"));
        assert!(source.contains("regs->data[priv->dreg] = h + priv->offset;"));
        assert!(source.contains("__skb_get_hash_symmetric_net(nft_net(pkt), skb)"));
        assert!(source.contains("[NFTA_HASH_SREG]\t= NLA_POLICY_MAX(NLA_BE32, NFT_REG32_MAX)"));
        assert!(source.contains("if (!tb[NFTA_HASH_SREG] ||"));
        assert!(source.contains("if (len == 0)"));
        assert!(source.contains("if (priv->modulus < 1)"));
        assert!(source.contains("if (priv->offset + priv->modulus - 1 < priv->offset)"));
        assert!(source.contains("get_random_bytes(&priv->seed, sizeof(priv->seed));"));
        assert!(source.contains("if (!priv->autogen_seed &&"));
        assert!(source.contains("if (priv->offset != 0)"));
        assert!(source.contains("case NFT_HASH_SYM:"));
        assert!(source.contains("case NFT_HASH_JENKINS:"));
        assert!(source.contains("return ERR_PTR(-EOPNOTSUPP);"));
        assert!(source.contains(".name\t\t= \"hash\""));
        assert!(source.contains("nft_register_expr(&nft_hash_type);"));
        assert!(header.contains("NFT_HASH_JENKINS"));
        assert!(header.contains("NFTA_HASH_MAX"));
        assert!(jhash_header.contains("static inline u32 jhash(const void *key"));
        assert!(jhash_header.contains("#define JHASH_INITVAL\t\t0xdeadbeef"));
    }

    #[test]
    fn jhash_eval_hashes_register_bytes_and_dump_omits_autogen_seed() {
        let mut regs = NftRegs::default();
        regs.data[1] = 0x0403_0201;
        regs.data[2] = 0x0807_0605;
        let expr = nft_jhash_init(attrs(Some(NFT_HASH_JENKINS)), 0).unwrap();
        nft_jhash_eval(&expr, &mut regs);
        let expected = reciprocal_scale(jhash(&[1, 2, 3, 4, 5, 6, 7, 8], 0x1234), 7) + 3;
        assert_eq!(regs.data[4], expected);
        assert_eq!(nft_jhash_dump(&expr).seed, Some(0x1234));

        let autogen = nft_jhash_init(
            NftHashAttrs {
                seed: None,
                offset: None,
                ..attrs(None)
            },
            0xdead,
        )
        .unwrap();
        assert!(autogen.autogen_seed);
        assert_eq!(nft_jhash_dump(&autogen).seed, None);
        assert_eq!(nft_jhash_dump(&autogen).offset, None);
    }

    #[test]
    fn symhash_uses_packet_hash_input_and_select_ops_follow_type() {
        let expr = nft_symhash_init(attrs(Some(NFT_HASH_SYM))).unwrap();
        let mut regs = NftRegs::default();
        nft_symhash_eval(&expr, &mut regs, u32::MAX);
        assert_eq!(regs.data[4], 9);
        assert_eq!(nft_symhash_dump(&expr).type_, NFT_HASH_SYM);
        assert_eq!(
            nft_hash_select_ops(attrs(None)),
            Ok(NftHashOpsKind::Jenkins)
        );
        assert_eq!(
            nft_hash_select_ops(attrs(Some(NFT_HASH_SYM))),
            Ok(NftHashOpsKind::Symmetric)
        );
        assert_eq!(nft_hash_select_ops(attrs(Some(9))), Err(-EOPNOTSUPP));
    }

    #[test]
    fn hash_init_rejects_missing_zero_and_overflow_values() {
        assert_eq!(
            nft_jhash_init(
                NftHashAttrs {
                    sreg: None,
                    ..attrs(None)
                },
                0,
            ),
            Err(-EINVAL)
        );
        assert_eq!(
            nft_jhash_init(
                NftHashAttrs {
                    len: Some(0),
                    ..attrs(None)
                },
                0,
            ),
            Err(-ERANGE)
        );
        assert_eq!(
            nft_symhash_init(NftHashAttrs {
                modulus: Some(0),
                ..attrs(Some(NFT_HASH_SYM))
            }),
            Err(-ERANGE)
        );
        assert_eq!(
            nft_symhash_init(NftHashAttrs {
                modulus: Some(2),
                offset: Some(u32::MAX),
                ..attrs(Some(NFT_HASH_SYM))
            }),
            Err(-EOVERFLOW)
        );
        assert_eq!(nft_hash_type(), &NFT_HASH_TYPE);
    }
}
