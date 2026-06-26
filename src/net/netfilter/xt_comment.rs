//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_comment.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_comment.c
//! Xtables no-op comment match.

pub const MODULE_AUTHOR: &str = "Brad Fisher <brad@info-link.net>";
pub const MODULE_DESCRIPTION: &str = "Xtables: No-op match which can be tagged with a comment";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_comment", "ip6t_comment"];
pub const XT_MAX_COMMENT_LEN: usize = 256;
pub const NFPROTO_UNSPEC: u8 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtCommentInfo {
    pub comment: [u8; XT_MAX_COMMENT_LEN],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
    pub matchsize: usize,
}

pub const COMMENT_MT_REG: XtMatch = XtMatch {
    name: "comment",
    revision: 0,
    family: NFPROTO_UNSPEC,
    matchsize: core::mem::size_of::<XtCommentInfo>(),
};

pub const fn comment_mt() -> bool {
    true
}

pub const fn comment_mt_init() -> &'static XtMatch {
    &COMMENT_MT_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_comment_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_comment.c"
        ));
        assert!(source.contains("MODULE_AUTHOR(\"Brad Fisher <brad@info-link.net>\");"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Xtables: No-op match"));
        assert!(source.contains("MODULE_ALIAS(\"ipt_comment\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_comment\");"));
        assert!(source.contains("static bool"));
        assert!(source.contains("comment_mt(const struct sk_buff *skb"));
        assert!(source.contains("return true;"));
        assert!(source.contains(".name      = \"comment\""));
        assert!(source.contains(".revision  = 0"));
        assert!(source.contains(".family    = NFPROTO_UNSPEC"));
        assert!(source.contains(".matchsize = sizeof(struct xt_comment_info)"));
        assert!(source.contains("xt_register_match(&comment_mt_reg);"));
        assert!(source.contains("xt_unregister_match(&comment_mt_reg);"));

        assert!(comment_mt());
        assert_eq!(COMMENT_MT_REG.name, "comment");
        assert_eq!(COMMENT_MT_REG.revision, 0);
        assert_eq!(COMMENT_MT_REG.family, NFPROTO_UNSPEC);
        assert_eq!(COMMENT_MT_REG.matchsize, XT_MAX_COMMENT_LEN);
        assert_eq!(comment_mt_init(), &COMMENT_MT_REG);
        assert_eq!(MODULE_ALIASES, ["ipt_comment", "ip6t_comment"]);
    }
}
