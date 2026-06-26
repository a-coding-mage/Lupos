//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_devgroup.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_devgroup.c
//! Xtables device-group match.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_AUTHOR: &str = "Patrick McHardy <kaber@trash.net>";
pub const MODULE_DESCRIPTION: &str = "Xtables: Device group match";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_devgroup", "ip6t_devgroup"];
pub const NFPROTO_UNSPEC: u8 = 0;
pub const XT_DEVGROUP_MATCH_SRC: u8 = 0x1;
pub const XT_DEVGROUP_INVERT_SRC: u8 = 0x2;
pub const XT_DEVGROUP_MATCH_DST: u8 = 0x4;
pub const XT_DEVGROUP_INVERT_DST: u8 = 0x8;
pub const XT_DEVGROUP_VALID_FLAGS: u8 =
    XT_DEVGROUP_MATCH_SRC | XT_DEVGROUP_INVERT_SRC | XT_DEVGROUP_MATCH_DST | XT_DEVGROUP_INVERT_DST;
pub const NF_INET_PRE_ROUTING: u8 = 0;
pub const NF_INET_LOCAL_IN: u8 = 1;
pub const NF_INET_FORWARD: u8 = 2;
pub const NF_INET_LOCAL_OUT: u8 = 3;
pub const NF_INET_POST_ROUTING: u8 = 4;
pub const DEVGROUP_SRC_HOOKS: u32 =
    (1 << NF_INET_PRE_ROUTING) | (1 << NF_INET_LOCAL_IN) | (1 << NF_INET_FORWARD);
pub const DEVGROUP_DST_HOOKS: u32 =
    (1 << NF_INET_FORWARD) | (1 << NF_INET_LOCAL_OUT) | (1 << NF_INET_POST_ROUTING);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtDevgroupInfo {
    pub flags: u8,
    pub src_group: u32,
    pub src_mask: u32,
    pub dst_group: u32,
    pub dst_mask: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub family: u8,
    pub matchsize: usize,
}

pub const DEVGROUP_MT_REG: XtMatch = XtMatch {
    name: "devgroup",
    family: NFPROTO_UNSPEC,
    matchsize: core::mem::size_of::<XtDevgroupInfo>(),
};

pub const fn devgroup_mt(info: XtDevgroupInfo, in_group: u32, out_group: u32) -> bool {
    if info.flags & XT_DEVGROUP_MATCH_SRC != 0 {
        let mismatch = ((info.src_group ^ in_group) & info.src_mask) != 0;
        let inverted = info.flags & XT_DEVGROUP_INVERT_SRC != 0;
        if mismatch != inverted {
            return false;
        }
    }

    if info.flags & XT_DEVGROUP_MATCH_DST != 0 {
        let mismatch = ((info.dst_group ^ out_group) & info.dst_mask) != 0;
        let inverted = info.flags & XT_DEVGROUP_INVERT_DST != 0;
        if mismatch != inverted {
            return false;
        }
    }

    true
}

pub const fn devgroup_mt_check_hooks(info: XtDevgroupInfo, hook_mask: u32) -> Result<(), i32> {
    if info.flags & XT_DEVGROUP_MATCH_SRC != 0 && hook_mask & !DEVGROUP_SRC_HOOKS != 0 {
        return Err(-EINVAL);
    }
    if info.flags & XT_DEVGROUP_MATCH_DST != 0 && hook_mask & !DEVGROUP_DST_HOOKS != 0 {
        return Err(-EINVAL);
    }
    Ok(())
}

pub const fn devgroup_mt_checkentry(info: XtDevgroupInfo) -> Result<(), i32> {
    if info.flags & !XT_DEVGROUP_VALID_FLAGS != 0 {
        Err(-EINVAL)
    } else {
        Ok(())
    }
}

pub const fn devgroup_mt_init() -> &'static XtMatch {
    &DEVGROUP_MT_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_devgroup_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_devgroup.c"
        ));
        assert!(source.contains("MODULE_DESCRIPTION(\"Xtables: Device group match\")"));
        assert!(source.contains("MODULE_ALIAS(\"ipt_devgroup\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_devgroup\");"));
        assert!(source.contains("if (info->flags & XT_DEVGROUP_MATCH_SRC"));
        assert!(source.contains("((info->src_group ^ xt_in(par)->group) & info->src_mask"));
        assert!(source.contains("XT_DEVGROUP_INVERT_SRC"));
        assert!(source.contains("if (info->flags & XT_DEVGROUP_MATCH_DST"));
        assert!(source.contains("((info->dst_group ^ xt_out(par)->group) & info->dst_mask"));
        assert!(source.contains("XT_DEVGROUP_INVERT_DST"));
        assert!(source.contains("devgroup_mt_check_hooks"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains(".name\t\t= \"devgroup\""));
        assert!(source.contains(".family\t\t= NFPROTO_UNSPEC"));
        assert!(source.contains("xt_register_match(&devgroup_mt_reg);"));

        let info = XtDevgroupInfo {
            flags: XT_DEVGROUP_MATCH_SRC | XT_DEVGROUP_MATCH_DST,
            src_group: 0x12,
            src_mask: 0xff,
            dst_group: 0x34,
            dst_mask: 0xff,
        };
        assert!(devgroup_mt(info, 0x12, 0x34));
        assert!(!devgroup_mt(info, 0x13, 0x34));
        assert!(devgroup_mt(
            XtDevgroupInfo {
                flags: XT_DEVGROUP_MATCH_SRC | XT_DEVGROUP_INVERT_SRC,
                ..info
            },
            0x13,
            0x34
        ));
        assert_eq!(
            devgroup_mt_check_hooks(info, DEVGROUP_SRC_HOOKS & DEVGROUP_DST_HOOKS),
            Ok(())
        );
        assert_eq!(
            devgroup_mt_check_hooks(info, 1 << NF_INET_LOCAL_OUT),
            Err(-EINVAL)
        );
        assert_eq!(
            devgroup_mt_checkentry(XtDevgroupInfo {
                flags: 0x80,
                ..info
            }),
            Err(-EINVAL)
        );
        assert_eq!(devgroup_mt_init(), &DEVGROUP_MT_REG);
    }
}
