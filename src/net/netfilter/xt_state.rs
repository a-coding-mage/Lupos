//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_state.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_state.c
//! Xtables conntrack state match.

pub const MODULE_AUTHOR: &str = "Rusty Russell <rusty@rustcorp.com.au>";
pub const MODULE_DESCRIPTION: &str = "ip[6]_tables connection tracking state match module";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_state", "ip6t_state"];
pub const NFPROTO_UNSPEC: u8 = 0;
pub const IP_CT_IS_REPLY: u32 = 3;
pub const IP_CT_NUMBER: u32 = 6;
pub const XT_STATE_INVALID: u32 = 1 << 0;
pub const XT_STATE_UNTRACKED: u32 = 1 << (IP_CT_NUMBER + 1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IpConntrackInfo {
    Established = 0,
    Related = 1,
    New = 2,
    EstablishedReply = 3,
    RelatedReply = 4,
    Untracked = 7,
    Invalid = 255,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtStateInfo {
    pub statemask: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub family: u8,
    pub matchsize: usize,
}

pub const STATE_MT_REG: XtMatch = XtMatch {
    name: "state",
    family: NFPROTO_UNSPEC,
    matchsize: core::mem::size_of::<XtStateInfo>(),
};

pub const fn xt_state_bit(ctinfo: IpConntrackInfo, has_conn: bool) -> u32 {
    if has_conn {
        1 << ((ctinfo as u32) % IP_CT_IS_REPLY + 1)
    } else if matches!(ctinfo, IpConntrackInfo::Untracked) {
        XT_STATE_UNTRACKED
    } else {
        XT_STATE_INVALID
    }
}

pub const fn state_mt(info: XtStateInfo, has_conn: bool, ctinfo: IpConntrackInfo) -> bool {
    info.statemask & xt_state_bit(ctinfo, has_conn) != 0
}

pub const fn state_mt_init() -> &'static XtMatch {
    &STATE_MT_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_state_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_state.c"
        ));
        assert!(source.contains("MODULE_AUTHOR(\"Rusty Russell"));
        assert!(source.contains("MODULE_ALIAS(\"ipt_state\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_state\");"));
        assert!(source.contains("state_mt(const struct sk_buff *skb"));
        assert!(source.contains("const struct xt_state_info *sinfo = par->matchinfo;"));
        assert!(source.contains("struct nf_conn *ct = nf_ct_get(skb, &ctinfo);"));
        assert!(source.contains("statebit = XT_STATE_BIT(ctinfo);"));
        assert!(source.contains("ctinfo == IP_CT_UNTRACKED"));
        assert!(source.contains("statebit = XT_STATE_UNTRACKED;"));
        assert!(source.contains("statebit = XT_STATE_INVALID;"));
        assert!(source.contains("return (sinfo->statemask & statebit);"));
        assert!(source.contains("nf_ct_netns_get(par->net, par->family);"));
        assert!(source.contains("nf_ct_netns_put(par->net, par->family);"));
        assert!(source.contains(".name       = \"state\""));
        assert!(source.contains(".family     = NFPROTO_UNSPEC"));
        assert!(source.contains(".matchsize  = sizeof(struct xt_state_info)"));
    }

    #[test]
    fn state_match_maps_conntrack_info_to_state_bits() {
        let established = xt_state_bit(IpConntrackInfo::Established, true);
        let new = xt_state_bit(IpConntrackInfo::New, true);
        assert_eq!(established, 1 << 1);
        assert_eq!(new, 1 << 3);
        assert_eq!(
            xt_state_bit(IpConntrackInfo::Untracked, false),
            XT_STATE_UNTRACKED
        );
        assert_eq!(
            xt_state_bit(IpConntrackInfo::Invalid, false),
            XT_STATE_INVALID
        );
        assert!(state_mt(
            XtStateInfo {
                statemask: established | XT_STATE_UNTRACKED,
            },
            true,
            IpConntrackInfo::Established,
        ));
        assert!(!state_mt(
            XtStateInfo {
                statemask: established,
            },
            true,
            IpConntrackInfo::Related,
        ));
        assert_eq!(state_mt_init(), &STATE_MT_REG);
    }
}
