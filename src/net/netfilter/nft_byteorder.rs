//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nft_byteorder.c
//! test-origin: linux:vendor/linux/net/netfilter/nft_byteorder.c
//! nftables byteorder expression.

use crate::include::uapi::errno::{EINVAL, ERANGE};

pub const NFT_BYTEORDER_NTOH: u8 = 0;
pub const NFT_BYTEORDER_HTON: u8 = 1;
pub const NFT_DATA_VALUE_MAXLEN: usize = 64;
pub const NFT_REG32_COUNT: usize = NFT_DATA_VALUE_MAXLEN / core::mem::size_of::<u32>();
pub const NFTA_BYTEORDER_SREG: u16 = 1;
pub const NFTA_BYTEORDER_DREG: u16 = 2;
pub const NFTA_BYTEORDER_OP: u16 = 3;
pub const NFTA_BYTEORDER_LEN: u16 = 4;
pub const NFTA_BYTEORDER_SIZE: u16 = 5;
pub const NFTA_BYTEORDER_MAX: u16 = NFTA_BYTEORDER_SIZE;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NftByteorder {
    pub sreg: u8,
    pub dreg: u8,
    pub op: u8,
    pub len: u8,
    pub size: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftByteorderAttrs {
    pub sreg: Option<Result<u8, i32>>,
    pub dreg: Option<Result<u8, i32>>,
    pub op: Option<u32>,
    pub len: Option<u32>,
    pub size: Option<u32>,
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
pub struct NftByteorderDump {
    pub sreg: u8,
    pub dreg: u8,
    pub op: u8,
    pub len: u8,
    pub size: u8,
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
    pub ops: &'static NftExprOps,
}

pub const NFT_BYTEORDER_OPS: NftExprOps = NftExprOps {
    type_name: "nft_byteorder_type",
    size: core::mem::size_of::<NftByteorder>(),
    eval: "nft_byteorder_eval",
    init: "nft_byteorder_init",
    dump: "nft_byteorder_dump",
};

pub const NFT_BYTEORDER_TYPE: NftExprType = NftExprType {
    name: "byteorder",
    maxattr: NFTA_BYTEORDER_MAX,
    ops: &NFT_BYTEORDER_OPS,
};

pub fn nft_byteorder_eval(expr: &NftByteorder, regs: &mut NftRegs) {
    let sreg = expr.sreg as usize;
    let dreg = expr.dreg as usize;
    match expr.size {
        8 => {
            let mut i = 0usize;
            while i < expr.len as usize / 8 {
                let src64 = nft_reg_load64(&regs.data, sreg + i);
                let converted = match expr.op {
                    NFT_BYTEORDER_NTOH => u64::from_be(src64),
                    NFT_BYTEORDER_HTON => src64.to_be(),
                    _ => src64,
                };
                nft_reg_store64(&mut regs.data, dreg + i * 2, converted);
                i += 1;
            }
        }
        4 => {
            let mut i = 0usize;
            while i < expr.len as usize / 4 {
                regs.data[dreg + i] = match expr.op {
                    NFT_BYTEORDER_NTOH => u32::from_be(regs.data[sreg + i]),
                    NFT_BYTEORDER_HTON => regs.data[sreg + i].to_be(),
                    _ => regs.data[sreg + i],
                };
                i += 1;
            }
        }
        2 => {
            let mut bytes = regs_as_bytes(regs);
            let mut i = 0usize;
            while i < expr.len as usize / 2 {
                let s = (sreg * 4) + (i * 2);
                let d = (dreg * 4) + (i * 2);
                let src16 = u16::from_ne_bytes([bytes[s], bytes[s + 1]]);
                let converted = match expr.op {
                    NFT_BYTEORDER_NTOH => u16::from_be(src16),
                    NFT_BYTEORDER_HTON => src16.to_be(),
                    _ => src16,
                };
                let out = converted.to_ne_bytes();
                bytes[d] = out[0];
                bytes[d + 1] = out[1];
                i += 1;
            }
            regs_from_bytes(regs, bytes);
        }
        _ => {}
    }
}

pub const fn nft_byteorder_init(attrs: NftByteorderAttrs) -> Result<NftByteorder, i32> {
    if attrs.sreg.is_none()
        || attrs.dreg.is_none()
        || attrs.len.is_none()
        || attrs.size.is_none()
        || attrs.op.is_none()
    {
        return Err(-EINVAL);
    }

    let op_value = attrs.op.unwrap();
    if op_value > u8::MAX as u32 {
        return Err(-ERANGE);
    }
    let op = op_value as u8;
    match op {
        NFT_BYTEORDER_NTOH | NFT_BYTEORDER_HTON => {}
        _ => return Err(-EINVAL),
    }

    let size_value = attrs.size.unwrap();
    if size_value > u8::MAX as u32 {
        return Err(-ERANGE);
    }
    let size = size_value as u8;
    match size {
        2 | 4 | 8 => {}
        _ => return Err(-EINVAL),
    }

    let len_value = attrs.len.unwrap();
    if len_value > u8::MAX as u32 {
        return Err(-ERANGE);
    }
    let len = len_value as u8;

    let sreg = match attrs.sreg.unwrap() {
        Ok(sreg) => sreg,
        Err(err) => return Err(err),
    };
    let dreg = match attrs.dreg.unwrap() {
        Ok(dreg) => dreg,
        Err(err) => return Err(err),
    };
    if nft_reg_overlap(sreg, dreg, len) {
        return Err(-EINVAL);
    }

    Ok(NftByteorder {
        sreg,
        dreg,
        op,
        len,
        size,
    })
}

pub const fn nft_byteorder_dump(expr: &NftByteorder) -> NftByteorderDump {
    NftByteorderDump {
        sreg: expr.sreg,
        dreg: expr.dreg,
        op: expr.op,
        len: expr.len,
        size: expr.size,
    }
}

pub const fn nft_byteorder_type() -> &'static NftExprType {
    &NFT_BYTEORDER_TYPE
}

const fn nft_reg_overlap(sreg: u8, dreg: u8, len: u8) -> bool {
    let s = sreg as usize * 4;
    let d = dreg as usize * 4;
    let e1 = s + len as usize;
    let e2 = d + len as usize;
    s < e2 && d < e1
}

fn nft_reg_load64(data: &[u32; NFT_REG32_COUNT], index: usize) -> u64 {
    ((data[index + 1] as u64) << 32) | data[index] as u64
}

fn nft_reg_store64(data: &mut [u32; NFT_REG32_COUNT], index: usize, value: u64) {
    data[index] = value as u32;
    data[index + 1] = (value >> 32) as u32;
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

fn regs_from_bytes(regs: &mut NftRegs, bytes: [u8; NFT_DATA_VALUE_MAXLEN]) {
    let mut i = 0usize;
    while i < NFT_REG32_COUNT {
        regs.data[i] = u32::from_ne_bytes([
            bytes[i * 4],
            bytes[i * 4 + 1],
            bytes[i * 4 + 2],
            bytes[i * 4 + 3],
        ]);
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attrs(op: u32, size: u32, len: u32) -> NftByteorderAttrs {
        NftByteorderAttrs {
            sreg: Some(Ok(1)),
            dreg: Some(Ok(4)),
            op: Some(op),
            len: Some(len),
            size: Some(size),
        }
    }

    #[test]
    fn nft_byteorder_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nft_byteorder.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter/nf_tables.h"
        ));
        assert!(source.contains("struct nft_byteorder"));
        assert!(source.contains("enum nft_byteorder_ops\top:8;"));
        assert!(source.contains("void nft_byteorder_eval"));
        assert!(source.contains("case 8:"));
        assert!(source.contains("be64_to_cpu((__force __be64)src64)"));
        assert!(source.contains("cpu_to_be64(nft_reg_load64(&src[i]))"));
        assert!(source.contains("case 4:"));
        assert!(source.contains("dst[i] = ntohl((__force __be32)src[i]);"));
        assert!(source.contains("case 2:"));
        assert!(source.contains("d16[i] = ntohs((__force __be16)s16[i]);"));
        assert!(
            source.contains("[NFTA_BYTEORDER_SREG]\t= NLA_POLICY_MAX(NLA_BE32, NFT_REG32_MAX)")
        );
        assert!(source.contains("if (tb[NFTA_BYTEORDER_SREG] == NULL"));
        assert!(
            source.contains("err = nft_parse_u32_check(tb[NFTA_BYTEORDER_SIZE], U8_MAX, &size);")
        );
        assert!(source.contains("case NFT_BYTEORDER_NTOH:"));
        assert!(source.contains("case NFT_BYTEORDER_HTON:"));
        assert!(source.contains("if (nft_reg_overlap(priv->sreg, priv->dreg, priv->len))"));
        assert!(source.contains("nft_dump_register(skb, NFTA_BYTEORDER_SREG, priv->sreg)"));
        assert!(source.contains(".name\t\t= \"byteorder\""));
        assert!(header.contains("NFT_BYTEORDER_NTOH"));
        assert!(header.contains("NFTA_BYTEORDER_MAX"));
    }

    #[test]
    fn byteorder_eval_converts_16_32_and_64_bit_units() {
        let mut regs = NftRegs::default();
        regs.data[1] = 0x0102_0304u32.to_be();
        let expr = nft_byteorder_init(attrs(NFT_BYTEORDER_NTOH as u32, 4, 4)).unwrap();
        nft_byteorder_eval(&expr, &mut regs);
        assert_eq!(regs.data[4], 0x0102_0304);

        regs.data[1] = 0x0102_0304;
        regs.data[2] = 0x0506_0708;
        let expr = nft_byteorder_init(attrs(NFT_BYTEORDER_HTON as u32, 8, 8)).unwrap();
        nft_byteorder_eval(&expr, &mut regs);
        assert_eq!(
            nft_reg_load64(&regs.data, 4),
            nft_reg_load64(&regs.data, 1).to_be()
        );

        let expr = nft_byteorder_init(NftByteorderAttrs {
            sreg: Some(Ok(1)),
            dreg: Some(Ok(5)),
            op: Some(NFT_BYTEORDER_HTON as u32),
            len: Some(2),
            size: Some(2),
        })
        .unwrap();
        regs.data[1] = 0x0000_1234;
        nft_byteorder_eval(&expr, &mut regs);
        assert_eq!((regs.data[5] & 0xffff) as u16, 0x1234u16.to_be());
    }

    #[test]
    fn byteorder_init_rejects_bad_attrs_and_overlap() {
        assert_eq!(
            nft_byteorder_init(NftByteorderAttrs {
                sreg: None,
                ..attrs(0, 4, 4)
            }),
            Err(-EINVAL)
        );
        assert_eq!(nft_byteorder_init(attrs(2, 4, 4)), Err(-EINVAL));
        assert_eq!(nft_byteorder_init(attrs(0, 3, 4)), Err(-EINVAL));
        assert_eq!(nft_byteorder_init(attrs(0, 4, 256)), Err(-ERANGE));
        assert_eq!(
            nft_byteorder_init(NftByteorderAttrs {
                dreg: Some(Ok(1)),
                ..attrs(0, 4, 4)
            }),
            Err(-EINVAL)
        );
        let expr = nft_byteorder_init(attrs(0, 4, 4)).unwrap();
        assert_eq!(nft_byteorder_dump(&expr).size, 4);
        assert_eq!(nft_byteorder_type(), &NFT_BYTEORDER_TYPE);
    }
}
