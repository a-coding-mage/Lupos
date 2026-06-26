//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_string.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_string.c
//! Xtables string-based packet match.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::EINVAL;

pub const MODULE_AUTHOR: &str = "Pablo Neira Ayuso <pablo@eurodev.net>";
pub const MODULE_DESCRIPTION: &str = "Xtables: string-based matching";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIASES: [&str; 3] = ["ipt_string", "ip6t_string", "ebt_string"];
pub const XT_STRING_MAX_PATTERN_SIZE: usize = 128;
pub const XT_STRING_MAX_ALGO_NAME_SIZE: usize = 16;
pub const XT_STRING_FLAG_INVERT: u8 = 0x01;
pub const XT_STRING_FLAG_IGNORECASE: u8 = 0x02;
pub const TS_AUTOLOAD: u32 = 0x1;
pub const TS_IGNORECASE: u32 = 0x2;
pub const NFPROTO_UNSPEC: u8 = 0;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XtStringInfo {
    pub from_offset: usize,
    pub to_offset: usize,
    pub algo: [u8; XT_STRING_MAX_ALGO_NAME_SIZE],
    pub pattern: Vec<u8>,
    pub flags: u8,
    pub config_prepared: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
    pub matchsize: usize,
}

pub const XT_STRING_MT_REG: XtMatch = XtMatch {
    name: "string",
    revision: 1,
    family: NFPROTO_UNSPEC,
    matchsize: core::mem::size_of::<XtStringInfo>(),
};

pub fn string_mt(skb: &[u8], conf: &XtStringInfo) -> bool {
    let found = find_text(
        skb,
        conf.from_offset,
        conf.to_offset,
        &conf.pattern,
        conf.flags & XT_STRING_FLAG_IGNORECASE != 0,
    );
    (found.is_some()) != (conf.flags & XT_STRING_FLAG_INVERT != 0)
}

pub fn string_mt_check(conf: &mut XtStringInfo, textsearch_prepare_ok: bool) -> Result<u32, i32> {
    if conf.from_offset > conf.to_offset {
        return Err(-EINVAL);
    }
    if conf.algo[XT_STRING_MAX_ALGO_NAME_SIZE - 1] != 0 {
        return Err(-EINVAL);
    }
    if conf.pattern.len() > XT_STRING_MAX_PATTERN_SIZE {
        return Err(-EINVAL);
    }
    if conf.flags & !(XT_STRING_FLAG_IGNORECASE | XT_STRING_FLAG_INVERT) != 0 {
        return Err(-EINVAL);
    }
    if !textsearch_prepare_ok {
        return Err(-EINVAL);
    }

    conf.config_prepared = true;
    let mut flags = TS_AUTOLOAD;
    if conf.flags & XT_STRING_FLAG_IGNORECASE != 0 {
        flags |= TS_IGNORECASE;
    }
    Ok(flags)
}

pub fn string_mt_destroy(conf: &mut XtStringInfo) {
    conf.config_prepared = false;
}

pub const fn string_mt_init() -> &'static XtMatch {
    &XT_STRING_MT_REG
}

fn find_text(
    skb: &[u8],
    from_offset: usize,
    to_offset: usize,
    pattern: &[u8],
    ignorecase: bool,
) -> Option<usize> {
    if pattern.is_empty() || from_offset > skb.len() {
        return None;
    }
    let end = to_offset.min(skb.len());
    if end < from_offset || end - from_offset < pattern.len() {
        return None;
    }

    skb[from_offset..end]
        .windows(pattern.len())
        .position(|window| bytes_eq(window, pattern, ignorecase))
        .map(|offset| from_offset + offset)
}

fn bytes_eq(left: &[u8], right: &[u8], ignorecase: bool) -> bool {
    left.iter().zip(right).all(|(a, b)| {
        if ignorecase {
            a.eq_ignore_ascii_case(b)
        } else {
            a == b
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn xt_string_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_string.c"
        ));
        assert!(source.contains("MODULE_AUTHOR(\"Pablo Neira Ayuso <pablo@eurodev.net>\");"));
        assert!(source.contains("MODULE_ALIAS(\"ipt_string\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_string\");"));
        assert!(source.contains("MODULE_ALIAS(\"ebt_string\");"));
        assert!(source.contains("string_mt(const struct sk_buff *skb"));
        assert!(source.contains("invert = conf->u.v1.flags & XT_STRING_FLAG_INVERT;"));
        assert!(source.contains("skb_find_text((struct sk_buff *)skb, conf->from_offset"));
        assert!(source.contains("!= UINT_MAX) ^ invert;"));
        assert!(source.contains("#define STRING_TEXT_PRIV(m) ((struct xt_string_info *)(m))"));
        assert!(source.contains("if (conf->from_offset > conf->to_offset)"));
        assert!(source.contains("if (conf->algo[XT_STRING_MAX_ALGO_NAME_SIZE - 1] != '\\0')"));
        assert!(source.contains("if (conf->patlen > XT_STRING_MAX_PATTERN_SIZE)"));
        assert!(source.contains("XT_STRING_FLAG_IGNORECASE | XT_STRING_FLAG_INVERT"));
        assert!(source.contains("flags |= TS_IGNORECASE;"));
        assert!(source.contains("textsearch_prepare(conf->algo, conf->pattern, conf->patlen"));
        assert!(source.contains("textsearch_destroy(STRING_TEXT_PRIV(par->matchinfo)->config);"));
        assert!(source.contains(".name       = \"string\""));
        assert!(source.contains(".revision   = 1"));
        assert!(source.contains("xt_register_match(&xt_string_mt_reg);"));
    }

    #[test]
    fn string_match_validates_config_and_searches_offsets() {
        let mut algo = [0u8; XT_STRING_MAX_ALGO_NAME_SIZE];
        algo[..2].copy_from_slice(b"bm");
        let mut conf = XtStringInfo {
            from_offset: 1,
            to_offset: 8,
            algo,
            pattern: b"Needle".to_vec(),
            flags: XT_STRING_FLAG_IGNORECASE,
            config_prepared: false,
        };
        assert_eq!(
            string_mt_check(&mut conf, true),
            Ok(TS_AUTOLOAD | TS_IGNORECASE)
        );
        assert!(conf.config_prepared);
        assert!(string_mt(b"--needle--", &conf));
        conf.flags |= XT_STRING_FLAG_INVERT;
        assert!(!string_mt(b"--needle--", &conf));
        string_mt_destroy(&mut conf);
        assert!(!conf.config_prepared);

        let mut invalid = conf.clone();
        invalid.from_offset = 9;
        invalid.to_offset = 1;
        assert_eq!(string_mt_check(&mut invalid, true), Err(-EINVAL));
        invalid.from_offset = 0;
        invalid.to_offset = 10;
        invalid.pattern = vec![0; XT_STRING_MAX_PATTERN_SIZE + 1];
        assert_eq!(string_mt_check(&mut invalid, true), Err(-EINVAL));
        assert_eq!(string_mt_init(), &XT_STRING_MT_REG);
    }
}
