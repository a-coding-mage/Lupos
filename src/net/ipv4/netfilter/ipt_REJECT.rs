//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv4/netfilter/ipt_REJECT.c
//! test-origin: linux:vendor/linux/net/ipv4/netfilter/ipt_REJECT.c
//! IPv4 iptables REJECT target.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "Netfilter Core Team <coreteam@netfilter.org>";
pub const MODULE_DESCRIPTION: &str = "Xtables: packet \"rejection\" target for IPv4";
pub const NF_DROP: u32 = 0;
pub const NFPROTO_IPV4: u8 = 2;
pub const NF_INET_LOCAL_IN: u8 = 1;
pub const NF_INET_FORWARD: u8 = 2;
pub const NF_INET_LOCAL_OUT: u8 = 3;
pub const IPPROTO_TCP: u8 = 6;
pub const XT_INV_PROTO: u8 = 0x40;
pub const ICMP_NET_UNREACH: u8 = 0;
pub const ICMP_HOST_UNREACH: u8 = 1;
pub const ICMP_PROT_UNREACH: u8 = 2;
pub const ICMP_PORT_UNREACH: u8 = 3;
pub const ICMP_NET_ANO: u8 = 9;
pub const ICMP_HOST_ANO: u8 = 10;
pub const ICMP_PKT_FILTERED: u8 = 13;
pub const IPT_ICMP_NET_UNREACHABLE: u32 = 0;
pub const IPT_ICMP_HOST_UNREACHABLE: u32 = 1;
pub const IPT_ICMP_PROT_UNREACHABLE: u32 = 2;
pub const IPT_ICMP_PORT_UNREACHABLE: u32 = 3;
pub const IPT_ICMP_ECHOREPLY: u32 = 4;
pub const IPT_ICMP_NET_PROHIBITED: u32 = 5;
pub const IPT_ICMP_HOST_PROHIBITED: u32 = 6;
pub const IPT_TCP_RESET: u32 = 7;
pub const IPT_ICMP_ADMIN_PROHIBITED: u32 = 8;
pub const REJECT_HOOKS: u32 =
    (1 << NF_INET_LOCAL_IN) | (1 << NF_INET_FORWARD) | (1 << NF_INET_LOCAL_OUT);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IptRejectInfo {
    pub with: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IptIp {
    pub proto: u8,
    pub invflags: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RejectAction {
    SendUnreach { code: u8, hook: u8 },
    SendReset { hook: u8 },
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RejectVerdict {
    pub action: RejectAction,
    pub verdict: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtTarget {
    pub name: &'static str,
    pub family: u8,
    pub targetsize: usize,
    pub table: &'static str,
    pub hooks: u32,
}

pub const REJECT_TG_REG: XtTarget = XtTarget {
    name: "REJECT",
    family: NFPROTO_IPV4,
    targetsize: core::mem::size_of::<IptRejectInfo>(),
    table: "filter",
    hooks: REJECT_HOOKS,
};

pub const fn reject_tg(reject: IptRejectInfo, hook: u8) -> RejectVerdict {
    let action = match reject.with {
        IPT_ICMP_NET_UNREACHABLE => RejectAction::SendUnreach {
            code: ICMP_NET_UNREACH,
            hook,
        },
        IPT_ICMP_HOST_UNREACHABLE => RejectAction::SendUnreach {
            code: ICMP_HOST_UNREACH,
            hook,
        },
        IPT_ICMP_PROT_UNREACHABLE => RejectAction::SendUnreach {
            code: ICMP_PROT_UNREACH,
            hook,
        },
        IPT_ICMP_PORT_UNREACHABLE => RejectAction::SendUnreach {
            code: ICMP_PORT_UNREACH,
            hook,
        },
        IPT_ICMP_NET_PROHIBITED => RejectAction::SendUnreach {
            code: ICMP_NET_ANO,
            hook,
        },
        IPT_ICMP_HOST_PROHIBITED => RejectAction::SendUnreach {
            code: ICMP_HOST_ANO,
            hook,
        },
        IPT_ICMP_ADMIN_PROHIBITED => RejectAction::SendUnreach {
            code: ICMP_PKT_FILTERED,
            hook,
        },
        IPT_TCP_RESET => RejectAction::SendReset { hook },
        IPT_ICMP_ECHOREPLY => RejectAction::None,
        _ => RejectAction::None,
    };
    RejectVerdict {
        action,
        verdict: NF_DROP,
    }
}

pub const fn reject_tg_check(rejinfo: IptRejectInfo, entry: IptIp) -> Result<(), i32> {
    if rejinfo.with == IPT_ICMP_ECHOREPLY {
        return Err(-EINVAL);
    }
    if rejinfo.with == IPT_TCP_RESET
        && (entry.proto != IPPROTO_TCP || (entry.invflags & XT_INV_PROTO) != 0)
    {
        return Err(-EINVAL);
    }
    Ok(())
}

pub const fn reject_tg_init(register_target_ret: i32) -> i32 {
    register_target_ret
}

pub const fn reject_tg_exit() -> &'static XtTarget {
    &REJECT_TG_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipt_reject_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv4/netfilter/ipt_REJECT.c"
        ));
        let reject_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter_ipv4/ipt_REJECT.h"
        ));
        let icmp = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/icmp.h"
        ));
        let inet = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/in.h"
        ));
        let xtables = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter/x_tables.h"
        ));

        assert!(reject_header.contains("enum ipt_reject_with"));
        assert!(reject_header.contains("IPT_ICMP_NET_UNREACHABLE"));
        assert!(reject_header.contains("IPT_TCP_RESET"));
        assert!(icmp.contains("#define ICMP_PKT_FILTERED\t13"));
        assert!(inet.contains("IPPROTO_TCP = 6"));
        assert!(xtables.contains("#define XT_INV_PROTO\t\t0x40"));
        assert!(
            source.contains(
                "MODULE_DESCRIPTION(\"Xtables: packet \\\"rejection\\\" target for IPv4\")"
            )
        );
        assert!(source.contains("reject_tg(struct sk_buff *skb"));
        assert!(source.contains("const struct ipt_reject_info *reject = par->targinfo;"));
        assert!(source.contains("int hook = xt_hooknum(par);"));
        assert!(source.contains("case IPT_ICMP_NET_UNREACHABLE:"));
        assert!(source.contains("nf_send_unreach(skb, ICMP_NET_UNREACH, hook);"));
        assert!(source.contains("case IPT_ICMP_HOST_UNREACHABLE:"));
        assert!(source.contains("nf_send_unreach(skb, ICMP_HOST_UNREACH, hook);"));
        assert!(source.contains("case IPT_ICMP_PROT_UNREACHABLE:"));
        assert!(source.contains("nf_send_unreach(skb, ICMP_PROT_UNREACH, hook);"));
        assert!(source.contains("case IPT_ICMP_PORT_UNREACHABLE:"));
        assert!(source.contains("nf_send_unreach(skb, ICMP_PORT_UNREACH, hook);"));
        assert!(source.contains("case IPT_ICMP_NET_PROHIBITED:"));
        assert!(source.contains("nf_send_unreach(skb, ICMP_NET_ANO, hook);"));
        assert!(source.contains("case IPT_ICMP_HOST_PROHIBITED:"));
        assert!(source.contains("nf_send_unreach(skb, ICMP_HOST_ANO, hook);"));
        assert!(source.contains("case IPT_ICMP_ADMIN_PROHIBITED:"));
        assert!(source.contains("nf_send_unreach(skb, ICMP_PKT_FILTERED, hook);"));
        assert!(source.contains("case IPT_TCP_RESET:"));
        assert!(source.contains("nf_send_reset(xt_net(par), par->state->sk, skb, hook);"));
        assert!(source.contains("case IPT_ICMP_ECHOREPLY:"));
        assert!(source.contains("return NF_DROP;"));
        assert!(source.contains("reject_tg_check(const struct xt_tgchk_param *par)"));
        assert!(source.contains("if (rejinfo->with == IPT_ICMP_ECHOREPLY)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("e->ip.proto != IPPROTO_TCP"));
        assert!(source.contains("(e->ip.invflags & XT_INV_PROTO)"));
        assert!(source.contains(".name\t\t= \"REJECT\""));
        assert!(source.contains(".family\t\t= NFPROTO_IPV4"));
        assert!(source.contains(".targetsize\t= sizeof(struct ipt_reject_info)"));
        assert!(source.contains(".table\t\t= \"filter\""));
        assert!(source.contains("xt_register_target(&reject_tg_reg);"));
        assert!(source.contains("xt_unregister_target(&reject_tg_reg);"));
    }

    #[test]
    fn reject_target_maps_each_linux_action_and_validation_edge() {
        let hook = NF_INET_LOCAL_OUT;
        assert_eq!(
            reject_tg(
                IptRejectInfo {
                    with: IPT_ICMP_NET_UNREACHABLE
                },
                hook
            ),
            RejectVerdict {
                action: RejectAction::SendUnreach {
                    code: ICMP_NET_UNREACH,
                    hook,
                },
                verdict: NF_DROP,
            }
        );
        assert_eq!(
            reject_tg(
                IptRejectInfo {
                    with: IPT_ICMP_HOST_UNREACHABLE
                },
                hook
            )
            .action,
            RejectAction::SendUnreach {
                code: ICMP_HOST_UNREACH,
                hook,
            }
        );
        assert_eq!(
            reject_tg(
                IptRejectInfo {
                    with: IPT_ICMP_PROT_UNREACHABLE
                },
                hook
            )
            .action,
            RejectAction::SendUnreach {
                code: ICMP_PROT_UNREACH,
                hook,
            }
        );
        assert_eq!(
            reject_tg(
                IptRejectInfo {
                    with: IPT_ICMP_PORT_UNREACHABLE
                },
                hook
            )
            .action,
            RejectAction::SendUnreach {
                code: ICMP_PORT_UNREACH,
                hook,
            }
        );
        assert_eq!(
            reject_tg(
                IptRejectInfo {
                    with: IPT_ICMP_NET_PROHIBITED
                },
                hook
            )
            .action,
            RejectAction::SendUnreach {
                code: ICMP_NET_ANO,
                hook,
            }
        );
        assert_eq!(
            reject_tg(
                IptRejectInfo {
                    with: IPT_ICMP_HOST_PROHIBITED
                },
                hook
            )
            .action,
            RejectAction::SendUnreach {
                code: ICMP_HOST_ANO,
                hook,
            }
        );
        assert_eq!(
            reject_tg(
                IptRejectInfo {
                    with: IPT_ICMP_ADMIN_PROHIBITED
                },
                hook
            )
            .action,
            RejectAction::SendUnreach {
                code: ICMP_PKT_FILTERED,
                hook,
            }
        );
        assert_eq!(
            reject_tg(
                IptRejectInfo {
                    with: IPT_TCP_RESET
                },
                hook
            )
            .action,
            RejectAction::SendReset { hook }
        );
        assert_eq!(
            reject_tg(
                IptRejectInfo {
                    with: IPT_ICMP_ECHOREPLY
                },
                hook
            )
            .action,
            RejectAction::None
        );
        assert_eq!(
            reject_tg_check(
                IptRejectInfo {
                    with: IPT_ICMP_ECHOREPLY,
                },
                IptIp {
                    proto: IPPROTO_TCP,
                    invflags: 0,
                },
            ),
            Err(-EINVAL)
        );
        assert_eq!(
            reject_tg_check(
                IptRejectInfo {
                    with: IPT_TCP_RESET,
                },
                IptIp {
                    proto: 17,
                    invflags: 0,
                },
            ),
            Err(-EINVAL)
        );
        assert_eq!(
            reject_tg_check(
                IptRejectInfo {
                    with: IPT_TCP_RESET,
                },
                IptIp {
                    proto: IPPROTO_TCP,
                    invflags: XT_INV_PROTO,
                },
            ),
            Err(-EINVAL)
        );
        assert_eq!(
            reject_tg_check(
                IptRejectInfo {
                    with: IPT_TCP_RESET,
                },
                IptIp {
                    proto: IPPROTO_TCP,
                    invflags: 0,
                },
            ),
            Ok(())
        );
        assert_eq!(REJECT_TG_REG.targetsize, 4);
        assert_eq!(REJECT_TG_REG.hooks, REJECT_HOOKS);
        assert_eq!(reject_tg_init(-5), -5);
        assert_eq!(reject_tg_exit(), &REJECT_TG_REG);
    }
}
