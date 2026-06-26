//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_connlabel.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_connlabel.c
//! Xtables connection-label match and optional label setter.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "Florian Westphal <fw@strlen.de>";
pub const MODULE_DESCRIPTION: &str = "Xtables: add/match connection tracking labels";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_connlabel", "ip6t_connlabel"];
pub const XT_CONNLABEL_MAXBIT: u16 = 127;
pub const XT_CONNLABEL_OP_INVERT: u16 = 1 << 0;
pub const XT_CONNLABEL_OP_SET: u16 = 1 << 1;
pub const NFPROTO_UNSPEC: u8 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtConnlabelMtinfo {
    pub bit: u16,
    pub options: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConnLabels {
    pub bits: u128,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConnLabelResult {
    pub matched: bool,
    pub labels: Option<ConnLabels>,
    pub event_cached: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub family: u8,
    pub matchsize: usize,
}

pub const CONNLABELS_MT_REG: XtMatch = XtMatch {
    name: "connlabel",
    family: NFPROTO_UNSPEC,
    matchsize: core::mem::size_of::<XtConnlabelMtinfo>(),
};

pub const fn connlabel_mt(info: XtConnlabelMtinfo, labels: Option<ConnLabels>) -> ConnLabelResult {
    let invert = info.options & XT_CONNLABEL_OP_INVERT != 0;
    let Some(mut labels) = labels else {
        return ConnLabelResult {
            matched: invert,
            labels: None,
            event_cached: false,
        };
    };

    let mask = 1u128 << info.bit;
    if labels.bits & mask != 0 {
        return ConnLabelResult {
            matched: !invert,
            labels: Some(labels),
            event_cached: false,
        };
    }

    if info.options & XT_CONNLABEL_OP_SET != 0 {
        labels.bits |= mask;
        return ConnLabelResult {
            matched: !invert,
            labels: Some(labels),
            event_cached: true,
        };
    }

    ConnLabelResult {
        matched: invert,
        labels: Some(labels),
        event_cached: false,
    }
}

pub const fn connlabel_mt_check(
    info: XtConnlabelMtinfo,
    nf_ct_netns_get_ret: i32,
    nf_connlabels_get_ret: i32,
) -> Result<(), i32> {
    let options = XT_CONNLABEL_OP_INVERT | XT_CONNLABEL_OP_SET;
    if info.options & !options != 0 || info.bit > XT_CONNLABEL_MAXBIT {
        return Err(-EINVAL);
    }
    if nf_ct_netns_get_ret < 0 {
        return Err(nf_ct_netns_get_ret);
    }
    if nf_connlabels_get_ret < 0 {
        return Err(nf_connlabels_get_ret);
    }
    Ok(())
}

pub const fn connlabel_mt_destroy() -> (bool, bool) {
    (true, true)
}

pub const fn connlabel_mt_init() -> &'static XtMatch {
    &CONNLABELS_MT_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_connlabel_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_connlabel.c"
        ));
        assert!(source.contains("MODULE_ALIAS(\"ipt_connlabel\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_connlabel\");"));
        assert!(source.contains("connlabel_mt(const struct sk_buff *skb"));
        assert!(source.contains("bool invert = info->options & XT_CONNLABEL_OP_INVERT;"));
        assert!(source.contains("ct = nf_ct_get(skb, &ctinfo);"));
        assert!(source.contains("labels = nf_ct_labels_find(ct);"));
        assert!(source.contains("if (test_bit(info->bit, labels->bits))"));
        assert!(source.contains("if (info->options & XT_CONNLABEL_OP_SET)"));
        assert!(source.contains("nf_conntrack_event_cache(IPCT_LABEL, ct);"));
        assert!(source.contains("nf_ct_netns_get(par->net, par->family);"));
        assert!(source.contains("nf_connlabels_get(par->net, info->bit);"));
        assert!(source.contains("nf_connlabels_put(par->net);"));
        assert!(source.contains(".name           = \"connlabel\""));
        assert!(source.contains("xt_register_match(&connlabels_mt_reg);"));
    }

    #[test]
    fn connlabel_match_sets_missing_labels_when_requested() {
        let info = XtConnlabelMtinfo {
            bit: 3,
            options: XT_CONNLABEL_OP_SET,
        };
        let out = connlabel_mt(info, Some(ConnLabels { bits: 0 }));
        assert!(out.matched);
        assert_eq!(out.labels.unwrap().bits, 1 << 3);
        assert!(out.event_cached);

        let present = connlabel_mt(
            XtConnlabelMtinfo {
                options: XT_CONNLABEL_OP_INVERT,
                ..info
            },
            Some(ConnLabels { bits: 1 << 3 }),
        );
        assert!(!present.matched);
        assert!(!present.event_cached);
        assert!(
            connlabel_mt(
                XtConnlabelMtinfo {
                    options: XT_CONNLABEL_OP_INVERT,
                    ..info
                },
                None,
            )
            .matched
        );
        assert_eq!(connlabel_mt_check(info, 0, 0), Ok(()));
        assert_eq!(
            connlabel_mt_check(
                XtConnlabelMtinfo {
                    options: 0x80,
                    ..info
                },
                0,
                0
            ),
            Err(-EINVAL)
        );
        assert_eq!(connlabel_mt_check(info, -3, 0), Err(-3));
        assert_eq!(connlabel_mt_check(info, 0, -4), Err(-4));
        assert_eq!(connlabel_mt_destroy(), (true, true));
        assert_eq!(connlabel_mt_init(), &CONNLABELS_MT_REG);
    }
}
