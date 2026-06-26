//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_LOG.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_LOG.c
//! Xtables LOG target log-info construction and validation.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHORS: [&str; 2] = [
    "Netfilter Core Team <coreteam@netfilter.org>",
    "Jan Rekorajski <baggins@pld.org.pl>",
];
pub const MODULE_DESCRIPTION: &str = "Xtables: IPv4/IPv6 packet logging";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_LOG", "ip6t_LOG"];
pub const MODULE_SOFTDEP: &str = "pre: nf_log_syslog";

pub const NFPROTO_IPV4: u8 = 2;
pub const NFPROTO_IPV6: u8 = 10;
pub const NF_LOG_TYPE_LOG: u8 = 0;
pub const XT_CONTINUE: u32 = 0xffff_ffff;
pub const XT_LOG_PREFIX_LEN: usize = 30;
pub const XT_LOG_TCPSEQ: u8 = 0x01;
pub const XT_LOG_TCPOPT: u8 = 0x02;
pub const XT_LOG_IPOPT: u8 = 0x04;
pub const XT_LOG_UID: u8 = 0x08;
pub const XT_LOG_NFLOG: u8 = 0x10;
pub const XT_LOG_MACDECODE: u8 = 0x20;
pub const XT_LOG_MASK: u8 = 0x2f;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtLogInfo {
    pub level: u8,
    pub logflags: u8,
    pub prefix: [u8; XT_LOG_PREFIX_LEN],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NfLoginfo {
    pub log_type: u8,
    pub level: u8,
    pub logflags: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LogPacketCall {
    pub family: u8,
    pub hooknum: u8,
    pub indev: Option<u32>,
    pub outdev: Option<u32>,
    pub info: NfLoginfo,
    pub prefix: [u8; XT_LOG_PREFIX_LEN],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtTarget {
    pub name: &'static str,
    pub family: u8,
    pub targetsize: usize,
    pub checkentry: &'static str,
    pub destroy: &'static str,
    pub target: &'static str,
}

pub const LOG_TG_REGS: [XtTarget; 2] = [
    XtTarget {
        name: "LOG",
        family: NFPROTO_IPV4,
        targetsize: core::mem::size_of::<XtLogInfo>(),
        checkentry: "log_tg_check",
        destroy: "log_tg_destroy",
        target: "log_tg",
    },
    XtTarget {
        name: "LOG",
        family: NFPROTO_IPV6,
        targetsize: core::mem::size_of::<XtLogInfo>(),
        checkentry: "log_tg_check",
        destroy: "log_tg_destroy",
        target: "log_tg",
    },
];

pub const fn log_tg(
    info: &XtLogInfo,
    family: u8,
    hooknum: u8,
    indev: Option<u32>,
    outdev: Option<u32>,
) -> (u32, LogPacketCall) {
    let loginfo = NfLoginfo {
        log_type: NF_LOG_TYPE_LOG,
        level: info.level,
        logflags: info.logflags,
    };
    (
        XT_CONTINUE,
        LogPacketCall {
            family,
            hooknum,
            indev,
            outdev,
            info: loginfo,
            prefix: info.prefix,
        },
    )
}

pub const fn log_tg_check(
    info: &XtLogInfo,
    family: u8,
    logger_find_ret: i32,
    logger_find_after_request_ret: i32,
    nft_compat: bool,
) -> Result<(), i32> {
    if family != NFPROTO_IPV4 && family != NFPROTO_IPV6 {
        return Err(-EINVAL);
    }
    if info.level >= 8 {
        return Err(-EINVAL);
    }
    if info.prefix[XT_LOG_PREFIX_LEN - 1] != 0 {
        return Err(-EINVAL);
    }
    if logger_find_ret != 0 && !nft_compat {
        if logger_find_after_request_ret == 0 {
            return Ok(());
        }
        return Err(logger_find_after_request_ret);
    }
    if logger_find_ret == 0 {
        Ok(())
    } else {
        Err(logger_find_ret)
    }
}

pub const fn log_tg_destroy(family: u8) -> (u8, u8) {
    (family, NF_LOG_TYPE_LOG)
}

pub const fn log_tg_init() -> &'static [XtTarget; 2] {
    &LOG_TG_REGS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(level: u8) -> XtLogInfo {
        XtLogInfo {
            level,
            logflags: XT_LOG_TCPSEQ | XT_LOG_IPOPT,
            prefix: [0; XT_LOG_PREFIX_LEN],
        }
    }

    #[test]
    fn xt_log_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_LOG.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter/xt_LOG.h"
        ));
        assert!(source.contains("MODULE_ALIAS(\"ipt_LOG\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_LOG\");"));
        assert!(source.contains("MODULE_SOFTDEP(\"pre: nf_log_syslog\");"));
        assert!(source.contains("log_tg(struct sk_buff *skb"));
        assert!(source.contains("li.type = NF_LOG_TYPE_LOG;"));
        assert!(source.contains("li.u.log.level = loginfo->level;"));
        assert!(source.contains("li.u.log.logflags = loginfo->logflags;"));
        assert!(source.contains("nf_log_packet(net, xt_family(par), xt_hooknum(par)"));
        assert!(source.contains("return XT_CONTINUE;"));
        assert!(source.contains("if (par->family != NFPROTO_IPV4 && par->family != NFPROTO_IPV6)"));
        assert!(source.contains("if (loginfo->level >= 8)"));
        assert!(source.contains("loginfo->prefix[sizeof(loginfo->prefix)-1] != '\\0'"));
        assert!(source.contains("ret = nf_logger_find_get(par->family, NF_LOG_TYPE_LOG);"));
        assert!(source.contains("if (ret != 0 && !par->nft_compat)"));
        assert!(source.contains("request_module(\"%s\", \"nf_log_syslog\");"));
        assert!(source.contains("nf_logger_put(par->family, NF_LOG_TYPE_LOG);"));
        assert!(source.contains(".name\t\t= \"LOG\""));
        assert!(source.contains(".family\t\t= NFPROTO_IPV4"));
        assert!(source.contains(".family\t\t= NFPROTO_IPV6"));
        assert!(source.contains(".targetsize\t= sizeof(struct xt_log_info)"));
        assert!(source.contains("xt_register_targets(log_tg_regs, ARRAY_SIZE(log_tg_regs));"));
        assert!(header.contains("#define XT_LOG_MASK\t\t0x2f"));
        assert!(header.contains("char prefix[30];"));
    }

    #[test]
    fn log_target_builds_loginfo_and_validates_logger_state() {
        let log_info = info(4);
        let (verdict, call) = log_tg(&log_info, NFPROTO_IPV4, 1, Some(10), None);
        assert_eq!(verdict, XT_CONTINUE);
        assert_eq!(
            call.info,
            NfLoginfo {
                log_type: NF_LOG_TYPE_LOG,
                level: 4,
                logflags: XT_LOG_TCPSEQ | XT_LOG_IPOPT,
            }
        );
        assert_eq!(log_tg_check(&log_info, NFPROTO_IPV4, 0, -2, false), Ok(()));
        assert_eq!(log_tg_check(&log_info, NFPROTO_IPV6, -2, 0, false), Ok(()));
        assert_eq!(log_tg_check(&log_info, NFPROTO_IPV4, -2, 0, true), Err(-2));
        assert_eq!(log_tg_check(&log_info, 0, 0, 0, false), Err(-EINVAL));
        assert_eq!(
            log_tg_check(&info(8), NFPROTO_IPV4, 0, 0, false),
            Err(-EINVAL)
        );
        let mut bad_prefix = log_info;
        bad_prefix.prefix[XT_LOG_PREFIX_LEN - 1] = b'x';
        assert_eq!(
            log_tg_check(&bad_prefix, NFPROTO_IPV4, 0, 0, false),
            Err(-EINVAL)
        );
        assert_eq!(
            log_tg_destroy(NFPROTO_IPV6),
            (NFPROTO_IPV6, NF_LOG_TYPE_LOG)
        );
        assert_eq!(log_tg_init(), &LOG_TG_REGS);
    }
}
