//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv6/netfilter/ip6t_REJECT.c
//! test-origin: linux:vendor/linux/net/ipv6/netfilter/ip6t_REJECT.c
//! IPv6 iptables REJECT target.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_AUTHOR: &str = "Yasuyuki KOZAKAI <yasuyuki.kozakai@toshiba.co.jp>";
pub const MODULE_DESCRIPTION: &str = "Xtables: packet \"rejection\" target for IPv6";
pub const MODULE_LICENSE: &str = "GPL";
pub const NF_DROP: u32 = 0;
pub const NFPROTO_IPV6: u8 = 10;
pub const NF_INET_LOCAL_IN: u8 = 1;
pub const NF_INET_FORWARD: u8 = 2;
pub const NF_INET_LOCAL_OUT: u8 = 3;
pub const IPPROTO_TCP: u8 = 6;
pub const XT_INV_PROTO: u8 = 0x40;
pub const IP6T_F_PROTO: u8 = 0x01;
pub const ICMPV6_NOROUTE: u8 = 0;
pub const ICMPV6_ADM_PROHIBITED: u8 = 1;
pub const ICMPV6_NOT_NEIGHBOUR: u8 = 2;
pub const ICMPV6_ADDR_UNREACH: u8 = 3;
pub const ICMPV6_PORT_UNREACH: u8 = 4;
pub const ICMPV6_POLICY_FAIL: u8 = 5;
pub const ICMPV6_REJECT_ROUTE: u8 = 6;
pub const IP6T_ICMP6_NO_ROUTE: u32 = 0;
pub const IP6T_ICMP6_ADM_PROHIBITED: u32 = 1;
pub const IP6T_ICMP6_NOT_NEIGHBOUR: u32 = 2;
pub const IP6T_ICMP6_ADDR_UNREACH: u32 = 3;
pub const IP6T_ICMP6_PORT_UNREACH: u32 = 4;
pub const IP6T_ICMP6_ECHOREPLY: u32 = 5;
pub const IP6T_TCP_RESET: u32 = 6;
pub const IP6T_ICMP6_POLICY_FAIL: u32 = 7;
pub const IP6T_ICMP6_REJECT_ROUTE: u32 = 8;
pub const REJECT_HOOKS: u32 =
    (1 << NF_INET_LOCAL_IN) | (1 << NF_INET_FORWARD) | (1 << NF_INET_LOCAL_OUT);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ip6tRejectInfo {
    pub with: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ip6tIp6 {
    pub flags: u8,
    pub proto: u8,
    pub invflags: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RejectAction6 {
    SendUnreach6 { code: u8, hook: u8 },
    SendReset6 { hook: u8 },
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RejectVerdict6 {
    pub action: RejectAction6,
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

pub const REJECT_TG6_REG: XtTarget = XtTarget {
    name: "REJECT",
    family: NFPROTO_IPV6,
    targetsize: core::mem::size_of::<Ip6tRejectInfo>(),
    table: "filter",
    hooks: REJECT_HOOKS,
};

pub const fn reject_tg6(reject: Ip6tRejectInfo, hook: u8) -> RejectVerdict6 {
    let action = match reject.with {
        IP6T_ICMP6_NO_ROUTE => RejectAction6::SendUnreach6 {
            code: ICMPV6_NOROUTE,
            hook,
        },
        IP6T_ICMP6_ADM_PROHIBITED => RejectAction6::SendUnreach6 {
            code: ICMPV6_ADM_PROHIBITED,
            hook,
        },
        IP6T_ICMP6_NOT_NEIGHBOUR => RejectAction6::SendUnreach6 {
            code: ICMPV6_NOT_NEIGHBOUR,
            hook,
        },
        IP6T_ICMP6_ADDR_UNREACH => RejectAction6::SendUnreach6 {
            code: ICMPV6_ADDR_UNREACH,
            hook,
        },
        IP6T_ICMP6_PORT_UNREACH => RejectAction6::SendUnreach6 {
            code: ICMPV6_PORT_UNREACH,
            hook,
        },
        IP6T_ICMP6_ECHOREPLY => RejectAction6::None,
        IP6T_TCP_RESET => RejectAction6::SendReset6 { hook },
        IP6T_ICMP6_POLICY_FAIL => RejectAction6::SendUnreach6 {
            code: ICMPV6_POLICY_FAIL,
            hook,
        },
        IP6T_ICMP6_REJECT_ROUTE => RejectAction6::SendUnreach6 {
            code: ICMPV6_REJECT_ROUTE,
            hook,
        },
        _ => RejectAction6::None,
    };
    RejectVerdict6 {
        action,
        verdict: NF_DROP,
    }
}

pub const fn reject_tg6_check(rejinfo: Ip6tRejectInfo, entry: Ip6tIp6) -> Result<(), i32> {
    if rejinfo.with == IP6T_ICMP6_ECHOREPLY {
        return Err(-EINVAL);
    }
    if rejinfo.with == IP6T_TCP_RESET
        && ((entry.flags & IP6T_F_PROTO) == 0
            || entry.proto != IPPROTO_TCP
            || (entry.invflags & XT_INV_PROTO) != 0)
    {
        return Err(-EINVAL);
    }
    Ok(())
}

pub const fn reject_tg6_init(register_target_ret: i32) -> i32 {
    register_target_ret
}

pub const fn reject_tg6_exit() -> &'static XtTarget {
    &REJECT_TG6_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip6t_reject_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv6/netfilter/ip6t_REJECT.c"
        ));
        let reject_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter_ipv6/ip6t_REJECT.h"
        ));
        let icmpv6 = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/icmpv6.h"
        ));
        let inet = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/in.h"
        ));
        let xtables = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter/x_tables.h"
        ));
        let ip6tables = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter_ipv6/ip6_tables.h"
        ));

        assert!(reject_header.contains("enum ip6t_reject_with"));
        assert!(reject_header.contains("IP6T_ICMP6_NO_ROUTE"));
        assert!(reject_header.contains("IP6T_TCP_RESET"));
        assert!(icmpv6.contains("#define ICMPV6_REJECT_ROUTE\t\t6"));
        assert!(inet.contains("IPPROTO_TCP = 6"));
        assert!(xtables.contains("#define XT_INV_PROTO\t\t0x40"));
        assert!(ip6tables.contains("#define IP6T_F_PROTO\t\t0x01"));
        assert!(
            source.contains(
                "MODULE_DESCRIPTION(\"Xtables: packet \\\"rejection\\\" target for IPv6\")"
            )
        );
        assert!(source.contains("reject_tg6(struct sk_buff *skb"));
        assert!(source.contains("const struct ip6t_reject_info *reject = par->targinfo;"));
        assert!(source.contains("struct net *net = xt_net(par);"));
        assert!(source.contains("case IP6T_ICMP6_NO_ROUTE:"));
        assert!(source.contains("nf_send_unreach6(net, skb, ICMPV6_NOROUTE, xt_hooknum(par));"));
        assert!(source.contains("case IP6T_ICMP6_ADM_PROHIBITED:"));
        assert!(source.contains("ICMPV6_ADM_PROHIBITED"));
        assert!(source.contains("case IP6T_ICMP6_NOT_NEIGHBOUR:"));
        assert!(source.contains("ICMPV6_NOT_NEIGHBOUR"));
        assert!(source.contains("case IP6T_ICMP6_ADDR_UNREACH:"));
        assert!(source.contains("ICMPV6_ADDR_UNREACH"));
        assert!(source.contains("case IP6T_ICMP6_PORT_UNREACH:"));
        assert!(source.contains("ICMPV6_PORT_UNREACH"));
        assert!(source.contains("case IP6T_ICMP6_ECHOREPLY:"));
        assert!(source.contains("case IP6T_TCP_RESET:"));
        assert!(source.contains("nf_send_reset6(net, par->state->sk, skb, xt_hooknum(par));"));
        assert!(source.contains("case IP6T_ICMP6_POLICY_FAIL:"));
        assert!(source.contains("ICMPV6_POLICY_FAIL"));
        assert!(source.contains("case IP6T_ICMP6_REJECT_ROUTE:"));
        assert!(source.contains("ICMPV6_REJECT_ROUTE"));
        assert!(source.contains("return NF_DROP;"));
        assert!(source.contains("reject_tg6_check(const struct xt_tgchk_param *par)"));
        assert!(source.contains("if (rejinfo->with == IP6T_ICMP6_ECHOREPLY)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("!(e->ipv6.flags & IP6T_F_PROTO)"));
        assert!(source.contains("e->ipv6.proto != IPPROTO_TCP"));
        assert!(source.contains("(e->ipv6.invflags & XT_INV_PROTO)"));
        assert!(source.contains(".name\t\t= \"REJECT\""));
        assert!(source.contains(".family\t\t= NFPROTO_IPV6"));
        assert!(source.contains(".targetsize\t= sizeof(struct ip6t_reject_info)"));
        assert!(source.contains(".table\t\t= \"filter\""));
        assert!(source.contains("xt_register_target(&reject_tg6_reg);"));
        assert!(source.contains("xt_unregister_target(&reject_tg6_reg);"));
    }

    #[test]
    fn reject6_target_maps_each_linux_action_and_validation_edge() {
        let hook = NF_INET_FORWARD;
        assert_eq!(
            reject_tg6(
                Ip6tRejectInfo {
                    with: IP6T_ICMP6_NO_ROUTE
                },
                hook
            ),
            RejectVerdict6 {
                action: RejectAction6::SendUnreach6 {
                    code: ICMPV6_NOROUTE,
                    hook,
                },
                verdict: NF_DROP,
            }
        );
        assert_eq!(
            reject_tg6(
                Ip6tRejectInfo {
                    with: IP6T_ICMP6_ADM_PROHIBITED
                },
                hook
            )
            .action,
            RejectAction6::SendUnreach6 {
                code: ICMPV6_ADM_PROHIBITED,
                hook,
            }
        );
        assert_eq!(
            reject_tg6(
                Ip6tRejectInfo {
                    with: IP6T_ICMP6_NOT_NEIGHBOUR
                },
                hook
            )
            .action,
            RejectAction6::SendUnreach6 {
                code: ICMPV6_NOT_NEIGHBOUR,
                hook,
            }
        );
        assert_eq!(
            reject_tg6(
                Ip6tRejectInfo {
                    with: IP6T_ICMP6_ADDR_UNREACH
                },
                hook
            )
            .action,
            RejectAction6::SendUnreach6 {
                code: ICMPV6_ADDR_UNREACH,
                hook,
            }
        );
        assert_eq!(
            reject_tg6(
                Ip6tRejectInfo {
                    with: IP6T_ICMP6_PORT_UNREACH
                },
                hook
            )
            .action,
            RejectAction6::SendUnreach6 {
                code: ICMPV6_PORT_UNREACH,
                hook,
            }
        );
        assert_eq!(
            reject_tg6(
                Ip6tRejectInfo {
                    with: IP6T_ICMP6_ECHOREPLY
                },
                hook
            )
            .action,
            RejectAction6::None
        );
        assert_eq!(
            reject_tg6(
                Ip6tRejectInfo {
                    with: IP6T_TCP_RESET
                },
                hook
            )
            .action,
            RejectAction6::SendReset6 { hook }
        );
        assert_eq!(
            reject_tg6(
                Ip6tRejectInfo {
                    with: IP6T_ICMP6_POLICY_FAIL
                },
                hook
            )
            .action,
            RejectAction6::SendUnreach6 {
                code: ICMPV6_POLICY_FAIL,
                hook,
            }
        );
        assert_eq!(
            reject_tg6(
                Ip6tRejectInfo {
                    with: IP6T_ICMP6_REJECT_ROUTE
                },
                hook
            )
            .action,
            RejectAction6::SendUnreach6 {
                code: ICMPV6_REJECT_ROUTE,
                hook,
            }
        );
        assert_eq!(
            reject_tg6_check(
                Ip6tRejectInfo {
                    with: IP6T_ICMP6_ECHOREPLY,
                },
                Ip6tIp6 {
                    flags: IP6T_F_PROTO,
                    proto: IPPROTO_TCP,
                    invflags: 0,
                },
            ),
            Err(-EINVAL)
        );
        assert_eq!(
            reject_tg6_check(
                Ip6tRejectInfo {
                    with: IP6T_TCP_RESET,
                },
                Ip6tIp6 {
                    flags: 0,
                    proto: IPPROTO_TCP,
                    invflags: 0,
                },
            ),
            Err(-EINVAL)
        );
        assert_eq!(
            reject_tg6_check(
                Ip6tRejectInfo {
                    with: IP6T_TCP_RESET,
                },
                Ip6tIp6 {
                    flags: IP6T_F_PROTO,
                    proto: 17,
                    invflags: 0,
                },
            ),
            Err(-EINVAL)
        );
        assert_eq!(
            reject_tg6_check(
                Ip6tRejectInfo {
                    with: IP6T_TCP_RESET,
                },
                Ip6tIp6 {
                    flags: IP6T_F_PROTO,
                    proto: IPPROTO_TCP,
                    invflags: XT_INV_PROTO,
                },
            ),
            Err(-EINVAL)
        );
        assert_eq!(
            reject_tg6_check(
                Ip6tRejectInfo {
                    with: IP6T_TCP_RESET,
                },
                Ip6tIp6 {
                    flags: IP6T_F_PROTO,
                    proto: IPPROTO_TCP,
                    invflags: 0,
                },
            ),
            Ok(())
        );
        assert_eq!(REJECT_TG6_REG.targetsize, 4);
        assert_eq!(REJECT_TG6_REG.hooks, REJECT_HOOKS);
        assert_eq!(reject_tg6_init(-5), -5);
        assert_eq!(reject_tg6_exit(), &REJECT_TG6_REG);
    }
}
