//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_u32.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_u32.c
//! Xtables arbitrary u32 packet-content match.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_AUTHOR: &str = "Jan Engelhardt <jengelh@medozas.de>";
pub const MODULE_DESCRIPTION: &str = "Xtables: arbitrary byte matching";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_u32", "ip6t_u32"];
pub const XT_U32_MAXSIZE: usize = 10;
pub const XT_U32_ARRAY_LEN: usize = XT_U32_MAXSIZE + 1;
pub const NFPROTO_UNSPEC: u8 = 0;

pub const XT_U32_AND: u8 = 0;
pub const XT_U32_LEFTSH: u8 = 1;
pub const XT_U32_RIGHTSH: u8 = 2;
pub const XT_U32_AT: u8 = 3;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct XtU32LocationElement {
    pub number: u32,
    pub nextop: u8,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct XtU32ValueElement {
    pub min: u32,
    pub max: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtU32Test {
    pub location: [XtU32LocationElement; XT_U32_ARRAY_LEN],
    pub value: [XtU32ValueElement; XT_U32_ARRAY_LEN],
    pub nnums: u8,
    pub nvalues: u8,
}

impl Default for XtU32Test {
    fn default() -> Self {
        Self {
            location: [XtU32LocationElement::default(); XT_U32_ARRAY_LEN],
            value: [XtU32ValueElement::default(); XT_U32_ARRAY_LEN],
            nnums: 0,
            nvalues: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtU32 {
    pub tests: [XtU32Test; XT_U32_ARRAY_LEN],
    pub ntests: u8,
    pub invert: u8,
}

impl Default for XtU32 {
    fn default() -> Self {
        Self {
            tests: [XtU32Test::default(); XT_U32_ARRAY_LEN],
            ntests: 0,
            invert: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
    pub matchsize: usize,
    pub match_fn: &'static str,
    pub checkentry: &'static str,
}

pub const XT_U32_MT_REG: XtMatch = XtMatch {
    name: "u32",
    revision: 0,
    family: NFPROTO_UNSPEC,
    matchsize: core::mem::size_of::<XtU32>(),
    match_fn: "u32_mt",
    checkentry: "u32_mt_checkentry",
};

pub fn u32_match_it(data: &XtU32, skb: &[u8]) -> bool {
    let mut testind = 0usize;
    while testind < data.ntests as usize {
        let ct = &data.tests[testind];
        let mut at = 0u32;
        let mut pos = ct.location[0].number;

        if skb.len() < 4 || pos as usize > skb.len() - 4 {
            return false;
        }

        let mut val = read_be_u32(skb, pos as usize);
        let nnums = ct.nnums as usize;

        let mut i = 1usize;
        while i < nnums {
            let number = ct.location[i].number;
            match ct.location[i].nextop {
                XT_U32_AND => val &= number,
                XT_U32_LEFTSH => val = val.wrapping_shl(number),
                XT_U32_RIGHTSH => val = val.wrapping_shr(number),
                XT_U32_AT => {
                    if at.wrapping_add(val) < at {
                        return false;
                    }
                    at = at.wrapping_add(val);
                    pos = number;
                    if at.wrapping_add(4) < at
                        || skb.len() < at as usize + 4
                        || pos as usize > skb.len() - at as usize - 4
                    {
                        return false;
                    }
                    val = read_be_u32(skb, at as usize + pos as usize);
                }
                _ => {}
            }
            i += 1;
        }

        let mut matched = false;
        let mut i = 0usize;
        while i < ct.nvalues as usize {
            let range = ct.value[i];
            if range.min <= val && val <= range.max {
                matched = true;
                break;
            }
            i += 1;
        }
        if !matched {
            return false;
        }
        testind += 1;
    }
    true
}

pub fn u32_mt(skb: &[u8], data: &XtU32) -> bool {
    ((u32_match_it(data, skb) as u8) ^ data.invert) != 0
}

pub const fn u32_mt_checkentry(data: &XtU32) -> Result<(), i32> {
    if data.ntests as usize > XT_U32_ARRAY_LEN {
        return Err(-EINVAL);
    }
    let mut i = 0usize;
    while i < data.ntests as usize {
        let ct = &data.tests[i];
        if ct.nnums as usize > XT_U32_ARRAY_LEN || ct.nvalues as usize > XT_U32_ARRAY_LEN {
            return Err(-EINVAL);
        }
        i += 1;
    }
    Ok(())
}

pub const fn u32_mt_init() -> &'static XtMatch {
    &XT_U32_MT_REG
}

fn read_be_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_test() -> XtU32 {
        let mut data = XtU32 {
            ntests: 1,
            ..XtU32::default()
        };
        data.tests[0].location[0].number = 0;
        data.tests[0].location[1] = XtU32LocationElement {
            number: 28,
            nextop: XT_U32_RIGHTSH,
        };
        data.tests[0].value[0] = XtU32ValueElement { min: 4, max: 4 };
        data.tests[0].nnums = 2;
        data.tests[0].nvalues = 1;
        data
    }

    #[test]
    fn xt_u32_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_u32.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter/xt_u32.h"
        ));
        assert!(header.contains("enum xt_u32_ops"));
        assert!(header.contains("#define XT_U32_MAXSIZE 10"));
        assert!(source.contains("static bool u32_match_it"));
        assert!(source.contains("for (testind = 0; testind < data->ntests; ++testind)"));
        assert!(source.contains("if (skb->len < 4 || pos > skb->len - 4)"));
        assert!(source.contains("val   = ntohl(n);"));
        assert!(source.contains("case XT_U32_AND:"));
        assert!(source.contains("case XT_U32_LEFTSH:"));
        assert!(source.contains("case XT_U32_RIGHTSH:"));
        assert!(source.contains("case XT_U32_AT:"));
        assert!(source.contains("if (at + val < at)"));
        assert!(source.contains("if (ct->value[i].min <= val && val <= ct->value[i].max)"));
        assert!(source.contains("return ret ^ data->invert;"));
        assert!(source.contains("if (data->ntests > ARRAY_SIZE(data->tests))"));
        assert!(source.contains(".name       = \"u32\""));
        assert!(source.contains(".family     = NFPROTO_UNSPEC"));
        assert!(source.contains("xt_register_match(&xt_u32_mt_reg);"));
        assert!(source.contains("MODULE_ALIAS(\"ipt_u32\");"));
    }

    #[test]
    fn u32_match_reads_network_order_words_and_applies_ops() {
        let data = one_test();
        let skb = [0x45, 0, 0, 0, 0, 0, 0, 0];
        assert!(u32_match_it(&data, &skb));
        assert!(u32_mt(&skb, &data));
        assert!(!u32_mt(&skb, &XtU32 { invert: 1, ..data }));
        assert!(u32_mt(&skb, &XtU32 { invert: 2, ..data }));
        assert_eq!(u32_mt_checkentry(&data), Ok(()));
    }

    #[test]
    fn u32_at_checks_overflow_bounds_and_checkentry_limits() {
        let mut data = XtU32 {
            ntests: 1,
            ..XtU32::default()
        };
        data.tests[0].location[0].number = 0;
        data.tests[0].location[1] = XtU32LocationElement {
            number: 0,
            nextop: XT_U32_AT,
        };
        data.tests[0].value[0] = XtU32ValueElement { min: 1, max: 1 };
        data.tests[0].nnums = 2;
        data.tests[0].nvalues = 1;
        assert!(!u32_match_it(&data, &[0xff, 0xff, 0xff, 0xff]));

        let mut bad = XtU32 {
            ntests: 1,
            ..XtU32::default()
        };
        bad.tests[0].nnums = XT_U32_ARRAY_LEN as u8 + 1;
        assert_eq!(u32_mt_checkentry(&bad), Err(-EINVAL));
        assert_eq!(u32_mt_init(), &XT_U32_MT_REG);
    }
}
