//! linux-parity: complete
//! linux-source: vendor/linux/net/bridge/netfilter/ebt_nflog.c
//! test-origin: linux:vendor/linux/net/bridge/netfilter/ebt_nflog.c
//! Ebtables NFLOG target.

use crate::include::uapi::errno::EINVAL;

pub const EBT_CONTINUE: i32 = -3;
pub const EBT_NFLOG_MASK: u16 = 0;
pub const EBT_NFLOG_PREFIX_SIZE: usize = 64;
pub const EBT_NFLOG_DEFAULT_GROUP: u16 = 1;
pub const EBT_NFLOG_DEFAULT_THRESHOLD: u16 = 1;
pub const NF_LOG_TYPE_ULOG: u8 = 1;
pub const NFPROTO_BRIDGE: u8 = 7;
pub const MODULE_AUTHOR: &str = "Peter Warasin <peter@endian.com>";
pub const MODULE_DESCRIPTION: &str = "ebtables NFLOG netfilter logging module";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EbtNflogInfo {
    pub len: u32,
    pub group: u16,
    pub threshold: u16,
    pub flags: u16,
    pub prefix: [u8; EBT_NFLOG_PREFIX_SIZE],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NfLogInfo {
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

pub const EBT_NFLOG_TG_REG: XtTarget = XtTarget {
    name: "nflog",
    revision: 0,
    family: NFPROTO_BRIDGE,
    targetsize: core::mem::size_of::<EbtNflogInfo>(),
};

pub fn ebt_nflog_tg(info: &EbtNflogInfo) -> (i32, NfLogInfo) {
    (
        EBT_CONTINUE,
        NfLogInfo {
            log_type: NF_LOG_TYPE_ULOG,
            copy_len: info.len,
            group: info.group,
            qthreshold: info.threshold,
            flags: 0,
        },
    )
}

pub fn ebt_nflog_tg_check(info: &mut EbtNflogInfo) -> Result<(), i32> {
    if info.flags & !EBT_NFLOG_MASK != 0 {
        return Err(-EINVAL);
    }
    info.prefix[EBT_NFLOG_PREFIX_SIZE - 1] = 0;
    Ok(())
}

pub const fn ebt_nflog_init() -> &'static XtTarget {
    &EBT_NFLOG_TG_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ebt_nflog_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/bridge/netfilter/ebt_nflog.c"
        ));
        assert!(source.contains("ebt_nflog_tg(struct sk_buff *skb"));
        assert!(source.contains("li.type = NF_LOG_TYPE_ULOG;"));
        assert!(source.contains("li.u.ulog.copy_len = info->len;"));
        assert!(source.contains("li.u.ulog.group = info->group;"));
        assert!(source.contains("li.u.ulog.qthreshold = info->threshold;"));
        assert!(source.contains("li.u.ulog.flags = 0;"));
        assert!(source.contains("nf_log_packet(net, PF_BRIDGE"));
        assert!(source.contains("return EBT_CONTINUE;"));
        assert!(source.contains("if (info->flags & ~EBT_NFLOG_MASK)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("info->prefix[EBT_NFLOG_PREFIX_SIZE - 1] = '\\0';"));
        assert!(source.contains(".name       = \"nflog\""));
        assert!(source.contains(".family     = NFPROTO_BRIDGE"));
        assert!(source.contains("MODULE_AUTHOR(\"Peter Warasin"));
        assert!(source.contains("MODULE_DESCRIPTION(\"ebtables NFLOG netfilter logging module\")"));
    }

    #[test]
    fn nflog_check_masks_flags_and_target_builds_loginfo() {
        let mut info = EbtNflogInfo {
            len: 128,
            group: 7,
            threshold: 3,
            flags: 0,
            prefix: [b'x'; EBT_NFLOG_PREFIX_SIZE],
        };
        assert_eq!(ebt_nflog_tg_check(&mut info), Ok(()));
        assert_eq!(info.prefix[EBT_NFLOG_PREFIX_SIZE - 1], 0);
        let (verdict, li) = ebt_nflog_tg(&info);
        assert_eq!(verdict, EBT_CONTINUE);
        assert_eq!(
            li,
            NfLogInfo {
                log_type: NF_LOG_TYPE_ULOG,
                copy_len: 128,
                group: 7,
                qthreshold: 3,
                flags: 0,
            }
        );
        info.flags = 1;
        assert_eq!(ebt_nflog_tg_check(&mut info), Err(-EINVAL));
        assert_eq!(ebt_nflog_init(), &EBT_NFLOG_TG_REG);
    }
}
