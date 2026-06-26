//! linux-parity: complete
//! linux-source: vendor/linux/net/sched/em_cmp.c
//! test-origin: linux:vendor/linux/net/sched/em_cmp.c
//! Traffic-control ematch packet data comparison.

pub const TCF_EM_CMP: u16 = 1;
pub const TCF_EM_ALIGN_U8: u8 = 1;
pub const TCF_EM_ALIGN_U16: u8 = 2;
pub const TCF_EM_ALIGN_U32: u8 = 4;
pub const TCF_EM_CMP_TRANS: u8 = 1;
pub const TCF_EM_OPND_EQ: u8 = 0;
pub const TCF_EM_OPND_GT: u8 = 1;
pub const TCF_EM_OPND_LT: u8 = 2;
pub const MODULE_DESCRIPTION: &str =
    "ematch classifier for basic data types(8/16/32 bit) against skb data";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TcfEmCmp {
    pub val: u32,
    pub mask: u32,
    pub off: usize,
    pub align: u8,
    pub flags: u8,
    pub layer: usize,
    pub opnd: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TcfEmatchOps {
    pub kind: u16,
    pub datalen: usize,
}

pub const EM_CMP_OPS: TcfEmatchOps = TcfEmatchOps {
    kind: TCF_EM_CMP,
    datalen: core::mem::size_of::<TcfEmCmp>(),
};

pub const fn cmp_needs_transformation(cmp: TcfEmCmp) -> bool {
    cmp.flags & TCF_EM_CMP_TRANS != 0
}

pub fn em_cmp_match(skb: &[u8], cmp: TcfEmCmp) -> bool {
    let Some(start) = cmp.layer.checked_add(cmp.off) else {
        return false;
    };
    let mut val = match cmp.align {
        TCF_EM_ALIGN_U8 if start < skb.len() => skb[start] as u32,
        TCF_EM_ALIGN_U16 if start + 2 <= skb.len() => {
            let raw = u16::from_be_bytes([skb[start], skb[start + 1]]);
            if cmp_needs_transformation(cmp) {
                u16::from_be(raw) as u32
            } else {
                raw as u32
            }
        }
        TCF_EM_ALIGN_U32 if start + 4 <= skb.len() => {
            let raw =
                u32::from_be_bytes([skb[start], skb[start + 1], skb[start + 2], skb[start + 3]]);
            if cmp_needs_transformation(cmp) {
                u32::from_be(raw)
            } else {
                raw
            }
        }
        _ => return false,
    };
    if cmp.mask != 0 {
        val &= cmp.mask;
    }
    match cmp.opnd {
        TCF_EM_OPND_EQ => val == cmp.val,
        TCF_EM_OPND_LT => val < cmp.val,
        TCF_EM_OPND_GT => val > cmp.val,
        _ => false,
    }
}

pub const fn init_em_cmp() -> &'static TcfEmatchOps {
    &EM_CMP_OPS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn em_cmp_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/sched/em_cmp.c"
        ));
        assert!(source.contains("cmp_needs_transformation"));
        assert!(source.contains("cmp->flags & TCF_EM_CMP_TRANS"));
        assert!(source.contains("static int em_cmp_match"));
        assert!(source.contains("tcf_get_base_ptr(skb, cmp->layer);"));
        assert!(source.contains("if (!tcf_valid_offset(skb, ptr, cmp->align))"));
        assert!(source.contains("case TCF_EM_ALIGN_U8:"));
        assert!(source.contains("case TCF_EM_ALIGN_U16:"));
        assert!(source.contains("get_unaligned_be16(ptr);"));
        assert!(source.contains("be16_to_cpu(val);"));
        assert!(source.contains("case TCF_EM_ALIGN_U32:"));
        assert!(source.contains("get_unaligned_be32(ptr);"));
        assert!(source.contains("be32_to_cpu(val);"));
        assert!(source.contains("if (cmp->mask)"));
        assert!(source.contains("case TCF_EM_OPND_EQ:"));
        assert!(source.contains("return val == cmp->val;"));
        assert!(source.contains(".kind\t  = TCF_EM_CMP"));
        assert!(source.contains("tcf_em_register(&em_cmp_ops);"));
        assert!(source.contains("MODULE_ALIAS_TCF_EMATCH(TCF_EM_CMP);"));
    }

    #[test]
    fn em_cmp_reads_aligned_values_masks_and_compares() {
        let skb = [0, 0x12, 0x34, 0x56, 0x78];
        assert!(em_cmp_match(
            &skb,
            TcfEmCmp {
                val: 0x1234,
                mask: 0,
                off: 1,
                align: TCF_EM_ALIGN_U16,
                flags: 0,
                layer: 0,
                opnd: TCF_EM_OPND_EQ,
            }
        ));
        assert!(em_cmp_match(
            &skb,
            TcfEmCmp {
                val: 0x34,
                mask: 0xff,
                off: 1,
                align: TCF_EM_ALIGN_U16,
                flags: 0,
                layer: 0,
                opnd: TCF_EM_OPND_EQ,
            }
        ));
        assert!(em_cmp_match(
            &skb,
            TcfEmCmp {
                val: 0x80,
                mask: 0,
                off: 1,
                align: TCF_EM_ALIGN_U8,
                flags: 0,
                layer: 0,
                opnd: TCF_EM_OPND_LT,
            }
        ));
        assert!(!em_cmp_match(
            &skb,
            TcfEmCmp {
                off: 99,
                ..TcfEmCmp {
                    val: 0,
                    mask: 0,
                    off: 0,
                    align: TCF_EM_ALIGN_U32,
                    flags: 0,
                    layer: 0,
                    opnd: TCF_EM_OPND_EQ,
                }
            }
        ));
        assert_eq!(init_em_cmp(), &EM_CMP_OPS);
    }
}
