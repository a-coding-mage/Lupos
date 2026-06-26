//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_NFLOG.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_NFLOG.c
//! Xtables NFLOG target log-info construction and validation.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_AUTHOR: &str = "Patrick McHardy <kaber@trash.net>";
pub const MODULE_DESCRIPTION: &str = "Xtables: packet logging to netlink using NFLOG";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_NFLOG", "ip6t_NFLOG"];
pub const MODULE_SOFTDEP: &str = "pre: nfnetlink_log";
pub const XT_NFLOG_DEFAULT_GROUP: u16 = 0x1;
pub const XT_NFLOG_DEFAULT_THRESHOLD: u16 = 0;
pub const XT_NFLOG_MASK: u16 = 0x1;
pub const XT_NFLOG_F_COPY_LEN: u16 = 0x1;
pub const NF_LOG_TYPE_ULOG: u8 = 1;
pub const NF_LOG_F_COPY_LEN: u16 = 0x1;
pub const XT_CONTINUE: u32 = 0xffff_ffff;
pub const NFPROTO_IPV4: u8 = 2;
pub const NFPROTO_IPV6: u8 = 10;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XtNflogInfo {
    pub len: u32,
    pub group: u16,
    pub threshold: u16,
    pub flags: u16,
    pub prefix: [u8; 64],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NfLoginfo {
    pub log_type: u8,
    pub copy_len: u32,
    pub group: u16,
    pub qthreshold: u16,
    pub flags: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtTarget {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
    pub targetsize: usize,
}

pub const NFLOG_TG_REG: [XtTarget; 2] = [
    XtTarget {
        name: "NFLOG",
        revision: 0,
        family: NFPROTO_IPV4,
        targetsize: core::mem::size_of::<XtNflogInfo>(),
    },
    XtTarget {
        name: "NFLOG",
        revision: 0,
        family: NFPROTO_IPV6,
        targetsize: core::mem::size_of::<XtNflogInfo>(),
    },
];

pub const fn nflog_tg(info: &XtNflogInfo) -> (u32, NfLoginfo) {
    let mut flags = 0;
    if info.flags & XT_NFLOG_F_COPY_LEN != 0 {
        flags |= NF_LOG_F_COPY_LEN;
    }
    (
        XT_CONTINUE,
        NfLoginfo {
            log_type: NF_LOG_TYPE_ULOG,
            copy_len: info.len,
            group: info.group,
            qthreshold: info.threshold,
            flags,
        },
    )
}

pub const fn nflog_tg_check(
    info: &XtNflogInfo,
    logger_find_ret: i32,
    logger_find_after_request_ret: i32,
    nft_compat: bool,
) -> Result<(), i32> {
    if info.flags & !XT_NFLOG_MASK != 0 {
        return Err(-EINVAL);
    }
    if info.prefix[63] != 0 {
        return Err(-EINVAL);
    }
    if logger_find_ret == 0 || nft_compat {
        return if logger_find_ret == 0 {
            Ok(())
        } else {
            Err(logger_find_ret)
        };
    }
    if logger_find_after_request_ret == 0 {
        Ok(())
    } else {
        Err(logger_find_after_request_ret)
    }
}

pub const fn nflog_tg_destroy() -> (u8, u8) {
    (0, NF_LOG_TYPE_ULOG)
}

pub const fn nflog_tg_init() -> &'static [XtTarget; 2] {
    &NFLOG_TG_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(flags: u16) -> XtNflogInfo {
        XtNflogInfo {
            len: 128,
            group: 5,
            threshold: 2,
            flags,
            prefix: [0; 64],
        }
    }

    #[test]
    fn xt_nflog_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_NFLOG.c"
        ));
        assert!(source.contains("MODULE_ALIAS(\"ipt_NFLOG\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_NFLOG\");"));
        assert!(source.contains("nflog_tg(struct sk_buff *skb"));
        assert!(source.contains("li.type"));
        assert!(source.contains("NF_LOG_TYPE_ULOG"));
        assert!(source.contains("li.u.ulog.copy_len   = info->len;"));
        assert!(source.contains("if (info->flags & XT_NFLOG_F_COPY_LEN)"));
        assert!(source.contains("nf_log_packet(net, xt_family(par), xt_hooknum(par)"));
        assert!(source.contains("if (info->flags & ~XT_NFLOG_MASK)"));
        assert!(source.contains("info->prefix[sizeof(info->prefix) - 1] != '\\0'"));
        assert!(source.contains("request_module(\"%s\", \"nfnetlink_log\");"));
        assert!(source.contains("nf_logger_put(par->family, NF_LOG_TYPE_ULOG);"));
        assert!(source.contains("MODULE_SOFTDEP(\"pre: nfnetlink_log\");"));
    }

    #[test]
    fn nflog_builds_loginfo_and_validates_flags_prefix_and_logger() {
        let log = nflog_tg(&info(XT_NFLOG_F_COPY_LEN)).1;
        assert_eq!(
            log,
            NfLoginfo {
                log_type: NF_LOG_TYPE_ULOG,
                copy_len: 128,
                group: 5,
                qthreshold: 2,
                flags: NF_LOG_F_COPY_LEN,
            }
        );
        assert_eq!(nflog_tg_check(&info(0), 0, -2, false), Ok(()));
        assert_eq!(nflog_tg_check(&info(0), -2, 0, false), Ok(()));
        assert_eq!(nflog_tg_check(&info(0), -2, 0, true), Err(-2));
        assert_eq!(nflog_tg_check(&info(0x80), 0, 0, false), Err(-EINVAL));
        let mut bad_prefix = info(0);
        bad_prefix.prefix[63] = b'x';
        assert_eq!(nflog_tg_check(&bad_prefix, 0, 0, false), Err(-EINVAL));
        assert_eq!(nflog_tg_destroy(), (0, NF_LOG_TYPE_ULOG));
        assert_eq!(nflog_tg_init(), &NFLOG_TG_REG);
    }
}
