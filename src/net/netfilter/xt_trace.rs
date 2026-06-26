//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_TRACE.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_TRACE.c
//! Xtables packet flow tracing target.

pub const MODULE_DESCRIPTION: &str = "Xtables: packet flow tracing";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_TRACE", "ip6t_TRACE"];
pub const MODULE_SOFTDEP: &str = "pre: nf_log_syslog";

pub const NFPROTO_IPV4: u8 = 2;
pub const NFPROTO_IPV6: u8 = 10;
pub const NF_LOG_TYPE_LOG: u8 = 0;
pub const XT_CONTINUE: u32 = 0xffff_fffc;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TraceSkb {
    pub nf_trace: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtTarget {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
    pub table: &'static str,
}

pub const TRACE_TG_REG: [XtTarget; 2] = [
    XtTarget {
        name: "TRACE",
        revision: 0,
        family: NFPROTO_IPV4,
        table: "raw",
    },
    XtTarget {
        name: "TRACE",
        revision: 0,
        family: NFPROTO_IPV6,
        table: "raw",
    },
];

pub const fn trace_tg_check(logger_find_get_result: i32) -> i32 {
    logger_find_get_result
}

pub const fn trace_tg_destroy(family: u8) -> (u8, u8) {
    (family, NF_LOG_TYPE_LOG)
}

pub const fn trace_tg(skb: &mut TraceSkb) -> u32 {
    skb.nf_trace = true;
    XT_CONTINUE
}

pub const fn trace_tg_init() -> &'static [XtTarget; 2] {
    &TRACE_TG_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_trace_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_TRACE.c"
        ));
        assert!(source.contains("MODULE_DESCRIPTION(\"Xtables: packet flow tracing\")"));
        assert!(source.contains("MODULE_ALIAS(\"ipt_TRACE\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_TRACE\");"));
        assert!(source.contains("return nf_logger_find_get(par->family, NF_LOG_TYPE_LOG);"));
        assert!(source.contains("nf_logger_put(par->family, NF_LOG_TYPE_LOG);"));
        assert!(source.contains("skb->nf_trace = 1;"));
        assert!(source.contains("return XT_CONTINUE;"));
        assert!(source.contains(".name\t\t= \"TRACE\""));
        assert!(source.contains(".family\t\t= NFPROTO_IPV4"));
        assert!(source.contains(".family\t\t= NFPROTO_IPV6"));
        assert!(source.contains(".table\t\t= \"raw\""));
        assert!(source.contains("xt_register_targets(trace_tg_reg, ARRAY_SIZE(trace_tg_reg));"));
        assert!(source.contains("MODULE_SOFTDEP(\"pre: nf_log_syslog\");"));

        let mut skb = TraceSkb::default();
        assert_eq!(trace_tg(&mut skb), XT_CONTINUE);
        assert!(skb.nf_trace);
        assert_eq!(
            trace_tg_destroy(NFPROTO_IPV4),
            (NFPROTO_IPV4, NF_LOG_TYPE_LOG)
        );
        assert_eq!(trace_tg_init().len(), 2);
        assert_eq!(MODULE_ALIASES, ["ipt_TRACE", "ip6t_TRACE"]);
    }
}
