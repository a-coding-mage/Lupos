//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_AUDIT.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_AUDIT.c
//! Xtables AUDIT target packet audit record shape.

use crate::include::uapi::errno::ERANGE;

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "Thomas Graf <tgraf@redhat.com>";
pub const MODULE_DESCRIPTION: &str = "Xtables: creates audit records for dropped/accepted packets";
pub const MODULE_ALIASES: [&str; 4] = ["ipt_AUDIT", "ip6t_AUDIT", "ebt_AUDIT", "arpt_AUDIT"];
pub const AUDIT_OFF: u8 = 0;
pub const AUDIT_NETFILTER_PKT: i32 = 1_310;
pub const XT_AUDIT_TYPE_ACCEPT: u8 = 0;
pub const XT_AUDIT_TYPE_DROP: u8 = 1;
pub const XT_AUDIT_TYPE_REJECT: u8 = 2;
pub const XT_AUDIT_TYPE_MAX: u8 = XT_AUDIT_TYPE_REJECT;
pub const XT_CONTINUE: u32 = 0xffff_ffff;
pub const EBT_CONTINUE: i32 = -3;
pub const NFPROTO_UNSPEC: u8 = 0;
pub const NFPROTO_BRIDGE: u8 = 7;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SkbAudit {
    pub mark: u32,
    pub family: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuditRecord {
    pub msg_type: i32,
    pub mark: u32,
    pub family: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtAuditInfo {
    pub audit_type: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtTarget {
    pub name: &'static str,
    pub family: u8,
    pub targetsize: usize,
}

pub const AUDIT_TG_REG: [XtTarget; 2] = [
    XtTarget {
        name: "AUDIT",
        family: NFPROTO_UNSPEC,
        targetsize: core::mem::size_of::<XtAuditInfo>(),
    },
    XtTarget {
        name: "AUDIT",
        family: NFPROTO_BRIDGE,
        targetsize: core::mem::size_of::<XtAuditInfo>(),
    },
];

pub const fn audit_tg(
    skb: SkbAudit,
    audit_enabled: u8,
    audit_log_start_ok: bool,
) -> (u32, Option<AuditRecord>) {
    if audit_enabled == AUDIT_OFF || !audit_log_start_ok {
        return (XT_CONTINUE, None);
    }
    (
        XT_CONTINUE,
        Some(AuditRecord {
            msg_type: AUDIT_NETFILTER_PKT,
            mark: skb.mark,
            family: skb.family,
        }),
    )
}

pub const fn audit_tg_ebt(skb: SkbAudit, audit_enabled: u8, audit_log_start_ok: bool) -> i32 {
    let _ = audit_tg(skb, audit_enabled, audit_log_start_ok);
    EBT_CONTINUE
}

pub const fn audit_tg_check(info: XtAuditInfo) -> Result<(), i32> {
    if info.audit_type > XT_AUDIT_TYPE_MAX {
        return Err(-ERANGE);
    }
    Ok(())
}

pub const fn audit_tg_init() -> &'static [XtTarget; 2] {
    &AUDIT_TG_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_audit_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_AUDIT.c"
        ));
        assert!(source.contains("MODULE_ALIAS(\"ipt_AUDIT\");"));
        assert!(source.contains("audit_tg(struct sk_buff *skb"));
        assert!(source.contains("if (audit_enabled == AUDIT_OFF)"));
        assert!(source.contains("audit_log_start(NULL, GFP_ATOMIC, AUDIT_NETFILTER_PKT);"));
        assert!(source.contains("audit_log_format(ab, \"mark=%#x\", skb->mark);"));
        assert!(source.contains("audit_log_nf_skb(ab, skb, xt_family(par));"));
        assert!(source.contains("return XT_CONTINUE;"));
        assert!(source.contains("audit_tg_ebt(struct sk_buff *skb"));
        assert!(source.contains("return EBT_CONTINUE;"));
        assert!(source.contains("if (info->type > XT_AUDIT_TYPE_MAX)"));
        assert!(source.contains(".family\t\t= NFPROTO_BRIDGE"));
        assert!(source.contains("xt_register_targets(audit_tg_reg, ARRAY_SIZE(audit_tg_reg));"));
    }

    #[test]
    fn audit_target_logs_when_audit_is_available() {
        let skb = SkbAudit {
            mark: 0x1234,
            family: 2,
        };
        assert_eq!(audit_tg(skb, AUDIT_OFF, true), (XT_CONTINUE, None));
        assert_eq!(audit_tg(skb, 1, false), (XT_CONTINUE, None));
        assert_eq!(
            audit_tg(skb, 1, true),
            (
                XT_CONTINUE,
                Some(AuditRecord {
                    msg_type: AUDIT_NETFILTER_PKT,
                    mark: 0x1234,
                    family: 2,
                })
            )
        );
        assert_eq!(audit_tg_ebt(skb, 1, true), EBT_CONTINUE);
        assert_eq!(
            audit_tg_check(XtAuditInfo {
                audit_type: XT_AUDIT_TYPE_MAX + 1,
            }),
            Err(-ERANGE)
        );
        assert_eq!(audit_tg_init(), &AUDIT_TG_REG);
    }
}
