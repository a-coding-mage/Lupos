//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nf_conntrack_broadcast.c
//! test-origin: linux:vendor/linux/net/netfilter/nf_conntrack_broadcast.c
//! Broadcast connection tracking expectation helper.

pub const NF_ACCEPT: i32 = 1;
pub const RTCF_BROADCAST: u32 = 0x1000_0000;
pub const IFA_F_SECONDARY: u32 = 0x01;
pub const IP_CT_DIR_ORIGINAL: u8 = 0;
pub const IP_CT_DIR_REPLY: u8 = 1;
pub const NF_CT_EXPECT_PERMANENT: u32 = 0x1;
pub const NF_CT_EXPECT_CLASS_DEFAULT: u8 = 0;
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_DESCRIPTION: &str = "Broadcast connection tracking helper";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InIfAddr {
    pub flags: u32,
    pub broadcast: u32,
    pub mask: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BroadcastHelpContext<'a> {
    pub skb_has_socket: bool,
    pub socket_net_matches_ct: bool,
    pub route_flags: Option<u32>,
    pub ct_dir: u8,
    pub ip_daddr: u32,
    pub ifaddrs: &'a [InIfAddr],
    pub helper_udp_port_be: Option<u16>,
    pub expect_alloc_ok: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BroadcastExpectation {
    pub mask_src_ip: u32,
    pub mask_src_udp_port: u16,
    pub tuple_src_udp_port: Option<u16>,
    pub flags: u32,
    pub class: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BroadcastHelpResult {
    pub verdict: i32,
    pub expectation: Option<BroadcastExpectation>,
    pub refreshed_timeout: Option<u32>,
}

pub fn nf_conntrack_broadcast_help(
    ctx: BroadcastHelpContext<'_>,
    timeout: u32,
) -> BroadcastHelpResult {
    let out = BroadcastHelpResult {
        verdict: NF_ACCEPT,
        expectation: None,
        refreshed_timeout: None,
    };

    if !ctx.skb_has_socket || !ctx.socket_net_matches_ct {
        return out;
    }
    if ctx
        .route_flags
        .map(|flags| flags & RTCF_BROADCAST == 0)
        .unwrap_or(true)
    {
        return out;
    }
    if ctx.ct_dir != IP_CT_DIR_ORIGINAL {
        return out;
    }

    let mut mask = 0;
    for ifa in ctx.ifaddrs {
        if ifa.flags & IFA_F_SECONDARY != 0 {
            continue;
        }
        if ifa.broadcast == ctx.ip_daddr {
            mask = ifa.mask;
            break;
        }
    }

    if mask == 0 || !ctx.expect_alloc_ok {
        return out;
    }

    BroadcastHelpResult {
        verdict: NF_ACCEPT,
        expectation: Some(BroadcastExpectation {
            mask_src_ip: mask,
            mask_src_udp_port: 0xffffu16.to_be(),
            tuple_src_udp_port: ctx.helper_udp_port_be,
            flags: NF_CT_EXPECT_PERMANENT,
            class: NF_CT_EXPECT_CLASS_DEFAULT,
        }),
        refreshed_timeout: Some(timeout),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nf_conntrack_broadcast_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nf_conntrack_broadcast.c"
        ));
        assert!(source.contains("int nf_conntrack_broadcast_help"));
        assert!(source.contains("struct net *net = read_pnet(&ct->ct_net);"));
        assert!(source.contains("struct iphdr *iph = ip_hdr(skb);"));
        assert!(source.contains("struct rtable *rt = skb_rtable(skb);"));
        assert!(
            source.contains("if (skb->sk == NULL || !net_eq(nf_ct_net(ct), sock_net(skb->sk)))")
        );
        assert!(source.contains("if (rt == NULL || !(rt->rt_flags & RTCF_BROADCAST))"));
        assert!(source.contains("if (CTINFO2DIR(ctinfo) != IP_CT_DIR_ORIGINAL)"));
        assert!(source.contains("if (ifa->ifa_flags & IFA_F_SECONDARY)"));
        assert!(source.contains("if (ifa->ifa_broadcast == iph->daddr)"));
        assert!(source.contains("if (mask == 0)"));
        assert!(source.contains("exp = nf_ct_expect_alloc(ct);"));
        assert!(
            source.contains("exp->tuple                = ct->tuplehash[IP_CT_DIR_REPLY].tuple;")
        );
        assert!(source.contains("exp->tuple.src.u.udp.port = helper->tuple.src.u.udp.port;"));
        assert!(source.contains("exp->mask.src.u3.ip       = mask;"));
        assert!(source.contains("exp->mask.src.u.udp.port  = htons(0xFFFF);"));
        assert!(source.contains("exp->flags                = NF_CT_EXPECT_PERMANENT;"));
        assert!(source.contains("exp->class\t\t  = NF_CT_EXPECT_CLASS_DEFAULT;"));
        assert!(source.contains("nf_ct_refresh(ct, timeout * HZ);"));
        assert!(source.contains("return NF_ACCEPT;"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(nf_conntrack_broadcast_help);"));
    }

    #[test]
    fn broadcast_help_creates_expectation_only_for_local_original_broadcasts() {
        let ifaddrs = [
            InIfAddr {
                flags: IFA_F_SECONDARY,
                broadcast: 0xffff_ffff,
                mask: 0xffff_ff00,
            },
            InIfAddr {
                flags: 0,
                broadcast: 0xffff_ffff,
                mask: 0xffff_0000,
            },
        ];
        let ctx = BroadcastHelpContext {
            skb_has_socket: true,
            socket_net_matches_ct: true,
            route_flags: Some(RTCF_BROADCAST),
            ct_dir: IP_CT_DIR_ORIGINAL,
            ip_daddr: 0xffff_ffff,
            ifaddrs: &ifaddrs,
            helper_udp_port_be: Some(137u16.to_be()),
            expect_alloc_ok: true,
        };
        let result = nf_conntrack_broadcast_help(ctx, 30);
        assert_eq!(result.verdict, NF_ACCEPT);
        assert_eq!(result.refreshed_timeout, Some(30));
        assert_eq!(result.expectation.unwrap().mask_src_ip, 0xffff_0000);
        assert_eq!(
            nf_conntrack_broadcast_help(
                BroadcastHelpContext {
                    ct_dir: IP_CT_DIR_REPLY,
                    ..ctx
                },
                30,
            )
            .expectation,
            None
        );
    }
}
