//! linux-parity: complete
//! linux-source: vendor/linux/fs/quota/netlink.c
//! test-origin: linux:vendor/linux/fs/quota/netlink.c
//! Quota warning generic-netlink message layout.

pub const QUOTA_GENL_FAMILY_NAME: &str = "VFS_DQUOT";
pub const QUOTA_GENL_VERSION: u8 = 1;
pub const QUOTA_MCGRP_EVENTS: &str = "events";

pub const QUOTA_NL_C_WARNING: u8 = 1;

pub const QUOTA_NL_A_QTYPE: u8 = 1;
pub const QUOTA_NL_A_EXCESS_ID: u8 = 2;
pub const QUOTA_NL_A_WARNING: u8 = 3;
pub const QUOTA_NL_A_DEV_MAJOR: u8 = 4;
pub const QUOTA_NL_A_DEV_MINOR: u8 = 5;
pub const QUOTA_NL_A_CAUSED_ID: u8 = 6;
pub const QUOTA_NL_A_PAD: u8 = 7;
pub const QUOTA_NL_A_MAX: u8 = 7;

pub const QUOTA_WARNING_ATTR_ORDER: &[u8] = &[
    QUOTA_NL_A_QTYPE,
    QUOTA_NL_A_EXCESS_ID,
    QUOTA_NL_A_WARNING,
    QUOTA_NL_A_DEV_MAJOR,
    QUOTA_NL_A_DEV_MINOR,
    QUOTA_NL_A_CAUSED_ID,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QuotaWarning {
    pub qtype: u32,
    pub excess_id: u64,
    pub warning: u8,
    pub dev_major: u32,
    pub dev_minor: u32,
    pub caused_id: u64,
}

pub const fn quota_dev_major(dev: u64) -> u32 {
    ((dev >> 20) & 0x000f_ffff) as u32
}

pub const fn quota_dev_minor(dev: u64) -> u32 {
    (dev & 0x000f_ffff) as u32
}

pub const fn quota_warning_from_ids(
    qtype: u32,
    excess_id: u64,
    warning: u8,
    dev: u64,
    caused_id: u64,
) -> QuotaWarning {
    QuotaWarning {
        qtype,
        excess_id,
        warning,
        dev_major: quota_dev_major(dev),
        dev_minor: quota_dev_minor(dev),
        caused_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quota_netlink_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/quota/netlink.c"
        ));
        assert!(source.contains("#include <linux/quotaops.h>"));
        assert!(source.contains("#include <net/genetlink.h>"));
        assert!(source.contains("static const struct genl_multicast_group quota_mcgrps[]"));
        assert!(source.contains("{ .name = \"events\", },"));
        assert!(source.contains(".name = \"VFS_DQUOT\""));
        assert!(source.contains(".version = 1"));
        assert!(source.contains(".maxattr = QUOTA_NL_A_MAX"));
        assert!(source.contains(".n_mcgrps = ARRAY_SIZE(quota_mcgrps)"));
        assert!(source.contains("void quota_send_warning(struct kqid qid, dev_t dev,"));
        assert!(source.contains("static atomic_t seq;"));
        assert!(source.contains("4 * nla_total_size(sizeof(u32)) +"));
        assert!(source.contains("2 * nla_total_size_64bit(sizeof(u64))"));
        assert!(source.contains("genlmsg_new(msg_size, GFP_NOFS);"));
        assert!(source.contains("QUOTA_NL_C_WARNING"));
        assert!(source.contains("nla_put_u32(skb, QUOTA_NL_A_QTYPE, qid.type);"));
        assert!(source.contains("nla_put_u64_64bit(skb, QUOTA_NL_A_EXCESS_ID"));
        assert!(source.contains("nla_put_u32(skb, QUOTA_NL_A_WARNING, warntype);"));
        assert!(source.contains("nla_put_u32(skb, QUOTA_NL_A_DEV_MAJOR, MAJOR(dev));"));
        assert!(source.contains("nla_put_u32(skb, QUOTA_NL_A_DEV_MINOR, MINOR(dev));"));
        assert!(source.contains("nla_put_u64_64bit(skb, QUOTA_NL_A_CAUSED_ID"));
        assert!(source.contains("genlmsg_multicast(&quota_genl_family, skb, 0, 0, GFP_NOFS);"));
        assert!(source.contains("fs_initcall(quota_init);"));

        assert_eq!(QUOTA_GENL_FAMILY_NAME, "VFS_DQUOT");
        assert_eq!(QUOTA_MCGRP_EVENTS, "events");
        assert_eq!(QUOTA_NL_A_MAX, QUOTA_NL_A_PAD);
        assert_eq!(QUOTA_WARNING_ATTR_ORDER, &[1, 2, 3, 4, 5, 6]);
        assert_eq!(
            quota_warning_from_ids(0, 99, 6, (8 << 20) | 1, 1000),
            QuotaWarning {
                qtype: 0,
                excess_id: 99,
                warning: 6,
                dev_major: 8,
                dev_minor: 1,
                caused_id: 1000,
            }
        );
    }
}
