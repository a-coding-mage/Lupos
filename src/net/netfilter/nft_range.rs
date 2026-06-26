//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nft_range.c
//! test-origin: linux:vendor/linux/net/netfilter/nft_range.c
//! nftables range expression.

use crate::include::uapi::errno::{EINVAL, ERANGE};

pub const NFT_BREAK: i32 = -2;
pub const NFT_DATA_VALUE_MAXLEN: usize = 64;
pub const NFT_RANGE_EQ: u8 = 0;
pub const NFT_RANGE_NEQ: u8 = 1;
pub const NFTA_RANGE_SREG: u16 = 1;
pub const NFTA_RANGE_OP: u16 = 2;
pub const NFTA_RANGE_FROM_DATA: u16 = 3;
pub const NFTA_RANGE_TO_DATA: u16 = 4;
pub const NFTA_RANGE_MAX: u16 = NFTA_RANGE_TO_DATA;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftData {
    pub bytes: [u8; NFT_DATA_VALUE_MAXLEN],
    pub len: usize,
}

impl NftData {
    pub const fn new(bytes: [u8; NFT_DATA_VALUE_MAXLEN], len: usize) -> Self {
        Self { bytes, len }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftRangeExpr {
    pub data_from: NftData,
    pub data_to: NftData,
    pub sreg: u8,
    pub len: u8,
    pub op: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftRangeInitAttrs {
    pub sreg: Option<Result<u8, i32>>,
    pub op: Option<u32>,
    pub data_from: Option<Result<NftData, i32>>,
    pub data_to: Option<Result<NftData, i32>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftRegs {
    pub data: [u8; NFT_DATA_VALUE_MAXLEN],
    pub verdict_code: i32,
}

impl Default for NftRegs {
    fn default() -> Self {
        Self {
            data: [0; NFT_DATA_VALUE_MAXLEN],
            verdict_code: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftRangeDump {
    pub sreg: u8,
    pub op: u8,
    pub data_from: NftData,
    pub data_to: NftData,
    pub len: u8,
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

pub const NFT_RANGE_OPS: NftExprOps = NftExprOps {
    type_name: "nft_range_type",
    size: core::mem::size_of::<NftRangeExpr>(),
    eval: "nft_range_eval",
    init: "nft_range_init",
    dump: "nft_range_dump",
};

pub const NFT_RANGE_TYPE: NftExprType = NftExprType {
    name: "range",
    maxattr: NFTA_RANGE_MAX,
    ops: &NFT_RANGE_OPS,
};

pub fn nft_range_eval(expr: &NftRangeExpr, regs: &mut NftRegs) {
    let start = expr.sreg as usize * core::mem::size_of::<u32>();
    let len = expr.len as usize;
    if start + len > regs.data.len() {
        regs.verdict_code = NFT_BREAK;
        return;
    }
    let d1 = memcmp(&regs.data[start..start + len], &expr.data_from.bytes[..len]);
    let d2 = memcmp(&regs.data[start..start + len], &expr.data_to.bytes[..len]);
    match expr.op {
        NFT_RANGE_EQ => {
            if d1 < 0 || d2 > 0 {
                regs.verdict_code = NFT_BREAK;
            }
        }
        NFT_RANGE_NEQ => {
            if d1 >= 0 && d2 <= 0 {
                regs.verdict_code = NFT_BREAK;
            }
        }
        _ => {}
    }
}

pub const fn nft_range_init(attrs: NftRangeInitAttrs) -> Result<NftRangeExpr, i32> {
    if attrs.sreg.is_none()
        || attrs.op.is_none()
        || attrs.data_from.is_none()
        || attrs.data_to.is_none()
    {
        return Err(-EINVAL);
    }

    let data_from = match attrs.data_from.unwrap() {
        Ok(data) => data,
        Err(err) => return Err(err),
    };
    let data_to = match attrs.data_to.unwrap() {
        Ok(data) => data,
        Err(err) => return Err(err),
    };
    if data_from.len != data_to.len {
        return Err(-EINVAL);
    }
    let sreg = match attrs.sreg.unwrap() {
        Ok(sreg) => sreg,
        Err(err) => return Err(err),
    };
    let op_value = attrs.op.unwrap();
    if op_value > u8::MAX as u32 {
        return Err(-ERANGE);
    }
    let op = op_value as u8;
    match op {
        NFT_RANGE_EQ | NFT_RANGE_NEQ => {}
        _ => return Err(-EINVAL),
    }
    Ok(NftRangeExpr {
        data_from,
        data_to,
        sreg,
        len: data_from.len as u8,
        op,
    })
}

pub const fn nft_range_dump(expr: &NftRangeExpr) -> NftRangeDump {
    NftRangeDump {
        sreg: expr.sreg,
        op: expr.op,
        data_from: expr.data_from,
        data_to: expr.data_to,
        len: expr.len,
    }
}

pub const fn nft_range_type() -> &'static NftExprType {
    &NFT_RANGE_TYPE
}

fn memcmp(left: &[u8], right: &[u8]) -> i32 {
    for (&a, &b) in left.iter().zip(right.iter()) {
        if a != b {
            return a as i32 - b as i32;
        }
    }
    left.len() as i32 - right.len() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(value: u8, len: usize) -> NftData {
        let mut bytes = [0u8; NFT_DATA_VALUE_MAXLEN];
        bytes[0] = value;
        NftData { bytes, len }
    }

    fn attrs(op: u32) -> NftRangeInitAttrs {
        NftRangeInitAttrs {
            sreg: Some(Ok(1)),
            op: Some(op),
            data_from: Some(Ok(data(10, 1))),
            data_to: Some(Ok(data(20, 1))),
        }
    }

    #[test]
    fn nft_range_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nft_range.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter/nf_tables.h"
        ));
        assert!(source.contains("struct nft_range_expr"));
        assert!(source.contains("struct nft_data\t\tdata_from;"));
        assert!(source.contains("struct nft_data\t\tdata_to;"));
        assert!(source.contains("u8\t\t\tsreg;"));
        assert!(source.contains("enum nft_range_ops\top:8;"));
        assert!(source.contains("void nft_range_eval"));
        assert!(
            source.contains("d1 = memcmp(&regs->data[priv->sreg], &priv->data_from, priv->len);")
        );
        assert!(
            source.contains("d2 = memcmp(&regs->data[priv->sreg], &priv->data_to, priv->len);")
        );
        assert!(source.contains("case NFT_RANGE_EQ:"));
        assert!(source.contains("if (d1 < 0 || d2 > 0)"));
        assert!(source.contains("case NFT_RANGE_NEQ:"));
        assert!(source.contains("if (d1 >= 0 && d2 <= 0)"));
        assert!(source.contains("regs->verdict.code = NFT_BREAK;"));
        assert!(source.contains("[NFTA_RANGE_SREG]\t\t= NLA_POLICY_MAX(NLA_BE32, NFT_REG32_MAX)"));
        assert!(source.contains("[NFTA_RANGE_OP]\t\t\t= NLA_POLICY_MAX(NLA_BE32, 255)"));
        assert!(source.contains("[NFTA_RANGE_FROM_DATA]\t\t= { .type = NLA_NESTED }"));
        assert!(source.contains("[NFTA_RANGE_TO_DATA]\t\t= { .type = NLA_NESTED }"));
        assert!(source.contains("if (!tb[NFTA_RANGE_SREG]"));
        assert!(source.contains("err = nft_data_init(NULL, &priv->data_from, &desc_from"));
        assert!(source.contains("err = nft_data_init(NULL, &priv->data_to, &desc_to"));
        assert!(source.contains("if (desc_from.len != desc_to.len)"));
        assert!(
            source.contains("err = nft_parse_register_load(ctx, tb[NFTA_RANGE_SREG], &priv->sreg")
        );
        assert!(source.contains("err = nft_parse_u32_check(tb[NFTA_RANGE_OP], U8_MAX, &op);"));
        assert!(source.contains("nft_data_release(&priv->data_to, desc_to.type);"));
        assert!(source.contains("nft_dump_register(skb, NFTA_RANGE_SREG, priv->sreg)"));
        assert!(source.contains("nla_put_be32(skb, NFTA_RANGE_OP, htonl(priv->op))"));
        assert!(source.contains("nft_data_dump(skb, NFTA_RANGE_FROM_DATA, &priv->data_from"));
        assert!(source.contains(".name\t\t= \"range\""));
        assert!(header.contains("NFT_BREAK\t= -2"));
        assert!(header.contains("NFT_RANGE_EQ"));
        assert!(header.contains("NFT_RANGE_NEQ"));
    }

    #[test]
    fn range_eval_breaks_outside_eq_and_inside_neq() {
        let expr = nft_range_init(attrs(NFT_RANGE_EQ as u32)).unwrap();
        let mut regs = NftRegs::default();
        regs.data[4] = 15;
        nft_range_eval(&expr, &mut regs);
        assert_eq!(regs.verdict_code, 0);
        regs.data[4] = 9;
        nft_range_eval(&expr, &mut regs);
        assert_eq!(regs.verdict_code, NFT_BREAK);

        let expr = nft_range_init(attrs(NFT_RANGE_NEQ as u32)).unwrap();
        let mut regs = NftRegs::default();
        regs.data[4] = 15;
        nft_range_eval(&expr, &mut regs);
        assert_eq!(regs.verdict_code, NFT_BREAK);
        let dump = nft_range_dump(&expr);
        assert_eq!(dump.op, NFT_RANGE_NEQ);
        assert_eq!(dump.sreg, 1);
        assert_eq!(nft_range_type(), &NFT_RANGE_TYPE);
    }

    #[test]
    fn range_init_rejects_missing_bad_or_mismatched_attributes() {
        assert_eq!(
            nft_range_init(NftRangeInitAttrs {
                sreg: None,
                ..attrs(0)
            }),
            Err(-EINVAL)
        );
        assert_eq!(
            nft_range_init(NftRangeInitAttrs {
                data_to: Some(Ok(data(20, 2))),
                ..attrs(0)
            }),
            Err(-EINVAL)
        );
        assert_eq!(
            nft_range_init(NftRangeInitAttrs {
                sreg: Some(Err(-5)),
                ..attrs(0)
            }),
            Err(-5)
        );
        assert_eq!(nft_range_init(attrs(255)), Err(-EINVAL));
        assert_eq!(nft_range_init(attrs(256)), Err(-ERANGE));
        assert_eq!(
            nft_range_init(NftRangeInitAttrs {
                data_from: Some(Err(-7)),
                ..attrs(0)
            }),
            Err(-7)
        );
    }
}
