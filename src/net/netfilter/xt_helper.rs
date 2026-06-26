//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_helper.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_helper.c
//! Xtables related connection helper match.

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "Martin Josefsson <gandalf@netfilter.org>";
pub const MODULE_DESCRIPTION: &str = "Xtables: Related connection matching";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_helper", "ip6t_helper"];
pub const NFPROTO_UNSPEC: u8 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtHelperInfo<'a> {
    pub name: &'a str,
    pub invert: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConntrackHelper<'a> {
    pub name: &'a str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Conntrack<'a> {
    pub has_master: bool,
    pub master_helper: Option<ConntrackHelper<'a>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
}

pub const HELPER_MT_REG: XtMatch = XtMatch {
    name: "helper",
    revision: 0,
    family: NFPROTO_UNSPEC,
};

pub fn helper_mt(info: XtHelperInfo<'_>, ct: Option<Conntrack<'_>>) -> bool {
    let mut ret = info.invert;
    let Some(ct) = ct else {
        return ret;
    };
    if !ct.has_master {
        return ret;
    }
    let Some(helper) = ct.master_helper else {
        return ret;
    };

    if info.name.is_empty() {
        ret = !ret;
    } else {
        ret ^= info.name.as_bytes().starts_with(helper.name.as_bytes());
    }
    ret
}

pub fn helper_mt_check(name: &mut [u8], netns_get_ret: i32) -> Result<(), i32> {
    if netns_get_ret < 0 {
        return Err(netns_get_ret);
    }
    if let Some(last) = name.last_mut() {
        *last = 0;
    }
    Ok(())
}

pub const fn helper_mt_destroy(netns_put_called: bool) -> bool {
    netns_put_called
}

pub const fn helper_mt_init() -> &'static XtMatch {
    &HELPER_MT_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_helper_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_helper.c"
        ));
        assert!(source.contains("MODULE_AUTHOR(\"Martin Josefsson"));
        assert!(source.contains("MODULE_ALIAS(\"ipt_helper\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_helper\");"));
        assert!(source.contains("helper_mt(const struct sk_buff *skb"));
        assert!(source.contains("bool ret = info->invert;"));
        assert!(source.contains("ct = nf_ct_get(skb, &ctinfo);"));
        assert!(source.contains("if (!ct || !ct->master)"));
        assert!(source.contains("master_help = nfct_help(ct->master);"));
        assert!(source.contains("helper = rcu_dereference(master_help->helper);"));
        assert!(source.contains("if (info->name[0] == '\\0')"));
        assert!(source.contains("ret ^= !strncmp(helper->name, info->name"));
        assert!(source.contains("ret = nf_ct_netns_get(par->net, par->family);"));
        assert!(source.contains("info->name[sizeof(info->name) - 1] = '\\0';"));
        assert!(source.contains("nf_ct_netns_put(par->net, par->family);"));
        assert!(source.contains(".name       = \"helper\""));
        assert!(source.contains("xt_register_match(&helper_mt_reg);"));
    }

    #[test]
    fn helper_match_defaults_to_invert_until_related_helper_exists() {
        let info = XtHelperInfo {
            name: "ftp",
            invert: false,
        };
        assert!(!helper_mt(info, None));
        assert!(helper_mt(
            XtHelperInfo {
                name: "",
                invert: false,
            },
            Some(Conntrack {
                has_master: true,
                master_helper: Some(ConntrackHelper { name: "ftp" }),
            })
        ));
        assert!(helper_mt(
            info,
            Some(Conntrack {
                has_master: true,
                master_helper: Some(ConntrackHelper { name: "ftp" }),
            })
        ));
        assert!(helper_mt(
            XtHelperInfo {
                name: "sip",
                invert: true,
            },
            Some(Conntrack {
                has_master: true,
                master_helper: Some(ConntrackHelper { name: "ftp" }),
            })
        ));
        let mut name = [b'a'; 4];
        assert_eq!(helper_mt_check(&mut name, 0), Ok(()));
        assert_eq!(name[3], 0);
        assert_eq!(helper_mt_init(), &HELPER_MT_REG);
    }
}
