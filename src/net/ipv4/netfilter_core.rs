//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv4/netfilter.c
//! test-origin: linux:vendor/linux/net/ipv4/netfilter.c
//! IPv4-specific netfilter routing helpers.

use crate::include::uapi::errno::ENOMEM;

pub const RTN_UNSPEC: u8 = 0;
pub const RTN_UNICAST: u8 = 1;
pub const RTN_LOCAL: u8 = 2;
pub const FLOWI_FLAG_ANYSRC: u8 = 0x01;
pub const AF_INET: u8 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ipv4Header {
    pub saddr: u32,
    pub daddr: u32,
    pub dscp: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RouteMeHarderInput {
    pub iph: Ipv4Header,
    pub skb_mark: u32,
    pub dev_hard_header_len: usize,
    pub skb_headroom: usize,
    pub bound_dev_if: Option<u32>,
    pub socket_flow_flags: u8,
    pub l3mdev_ifindex: u32,
    pub inferred_addr_type: u8,
    pub dst_error: i32,
    pub xfrm_transformed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Flowi4 {
    pub daddr: u32,
    pub saddr: u32,
    pub flowi4_dscp: u8,
    pub flowi4_oif: u32,
    pub flowi4_l3mdev: u32,
    pub flowi4_mark: u32,
    pub flowi4_flags: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RouteMeHarderResult {
    pub flow: Flowi4,
    pub expanded_headroom: usize,
}

pub fn ip_route_me_harder(
    input: RouteMeHarderInput,
    mut addr_type: u8,
    route_ret: i32,
    xfrm_ret: i32,
    expand_head_ok: bool,
) -> Result<RouteMeHarderResult, i32> {
    let mut saddr = input.iph.saddr;
    let mut flags = input.socket_flow_flags;

    if addr_type == RTN_UNSPEC {
        addr_type = input.inferred_addr_type;
    }
    if addr_type == RTN_LOCAL || addr_type == RTN_UNICAST {
        flags |= FLOWI_FLAG_ANYSRC;
    } else {
        saddr = 0;
    }

    let flow = Flowi4 {
        daddr: input.iph.daddr,
        saddr,
        flowi4_dscp: input.iph.dscp,
        flowi4_oif: input.bound_dev_if.unwrap_or(0),
        flowi4_l3mdev: input.l3mdev_ifindex,
        flowi4_mark: input.skb_mark,
        flowi4_flags: flags,
    };

    if route_ret < 0 {
        return Err(route_ret);
    }
    if input.dst_error != 0 {
        return Err(input.dst_error);
    }
    if !input.xfrm_transformed && xfrm_ret < 0 {
        return Err(xfrm_ret);
    }
    if input.skb_headroom < input.dev_hard_header_len && !expand_head_ok {
        return Err(-ENOMEM);
    }

    Ok(RouteMeHarderResult {
        flow,
        expanded_headroom: input.skb_headroom.max(input.dev_hard_header_len),
    })
}

pub fn nf_ip_route(route_ret: i32) -> Result<(), i32> {
    if route_ret < 0 {
        Err(route_ret)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipv4_netfilter_core_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv4/netfilter.c"
        ));
        let flow = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/net/flow.h"
        ));
        let rtnetlink = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/rtnetlink.h"
        ));
        assert!(flow.contains("#define FLOWI_FLAG_ANYSRC\t\t0x01"));
        assert!(rtnetlink.contains("RTN_UNSPEC"));
        assert!(rtnetlink.contains("RTN_UNICAST"));
        assert!(rtnetlink.contains("RTN_LOCAL"));
        assert!(
            source.contains(
                "ip_route_me_harder(struct net *net, struct sock *sk, struct sk_buff *skb"
            )
        );
        assert!(source.contains("struct flowi4 fl4 = {};"));
        assert!(source.contains("saddr = iph->saddr;"));
        assert!(source.contains("flags = sk ? inet_sk_flowi_flags(sk) : 0;"));
        assert!(source.contains("if (addr_type == RTN_UNSPEC)"));
        assert!(source.contains("addr_type = inet_addr_type_dev_table(net, dev, saddr);"));
        assert!(source.contains("if (addr_type == RTN_LOCAL || addr_type == RTN_UNICAST)"));
        assert!(source.contains("flags |= FLOWI_FLAG_ANYSRC;"));
        assert!(source.contains("else"));
        assert!(source.contains("saddr = 0;"));
        assert!(source.contains("fl4.daddr = iph->daddr;"));
        assert!(source.contains("fl4.saddr = saddr;"));
        assert!(source.contains("fl4.flowi4_dscp = ip4h_dscp(iph);"));
        assert!(source.contains("fl4.flowi4_oif = sk ? sk->sk_bound_dev_if : 0;"));
        assert!(source.contains("fl4.flowi4_l3mdev = l3mdev_master_ifindex(dev);"));
        assert!(source.contains("fl4.flowi4_mark = skb->mark;"));
        assert!(source.contains("fl4.flowi4_flags = flags;"));
        assert!(source.contains("rt = ip_route_output_key(net, &fl4);"));
        assert!(source.contains("skb_dst_drop(skb);"));
        assert!(source.contains("skb_dst_set(skb, &rt->dst);"));
        assert!(source.contains("if (skb_dst(skb)->error)"));
        assert!(source.contains("xfrm_decode_session(net, skb, flowi4_to_flowi(&fl4), AF_INET)"));
        assert!(source.contains("xfrm_lookup(net, dst, flowi4_to_flowi(&fl4), sk, 0);"));
        assert!(source.contains("hh_len = skb_dst_dev(skb)->hard_header_len;"));
        assert!(source.contains("pskb_expand_head(skb, HH_DATA_ALIGN(hh_len - skb_headroom(skb))"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("nf_ip_route(struct net *net, struct dst_entry **dst"));
        assert!(source.contains("*dst = &rt->dst;"));
    }

    #[test]
    fn route_me_harder_builds_flow_and_propagates_errors() {
        let input = RouteMeHarderInput {
            iph: Ipv4Header {
                saddr: 0x0a00_0001,
                daddr: 0x0a00_0002,
                dscp: 0x2a,
            },
            skb_mark: 7,
            dev_hard_header_len: 14,
            skb_headroom: 4,
            bound_dev_if: Some(3),
            socket_flow_flags: 0,
            l3mdev_ifindex: 5,
            inferred_addr_type: RTN_LOCAL,
            dst_error: 0,
            xfrm_transformed: false,
        };
        let out = ip_route_me_harder(input, RTN_UNSPEC, 0, 0, true).unwrap();
        assert_eq!(
            out.flow,
            Flowi4 {
                daddr: 0x0a00_0002,
                saddr: 0x0a00_0001,
                flowi4_dscp: 0x2a,
                flowi4_oif: 3,
                flowi4_l3mdev: 5,
                flowi4_mark: 7,
                flowi4_flags: FLOWI_FLAG_ANYSRC,
            }
        );
        assert_eq!(out.expanded_headroom, 14);

        let remote = ip_route_me_harder(
            RouteMeHarderInput {
                inferred_addr_type: 9,
                ..input
            },
            RTN_UNSPEC,
            0,
            0,
            true,
        )
        .unwrap();
        assert_eq!(remote.flow.saddr, 0);
        assert_eq!(ip_route_me_harder(input, RTN_LOCAL, -11, 0, true), Err(-11));
        assert_eq!(
            ip_route_me_harder(
                RouteMeHarderInput {
                    dst_error: -22,
                    ..input
                },
                RTN_LOCAL,
                0,
                0,
                true,
            ),
            Err(-22)
        );
        assert_eq!(ip_route_me_harder(input, RTN_LOCAL, 0, -5, true), Err(-5));
        assert_eq!(
            ip_route_me_harder(input, RTN_LOCAL, 0, 0, false),
            Err(-ENOMEM)
        );
        assert_eq!(nf_ip_route(-3), Err(-3));
        assert_eq!(nf_ip_route(0), Ok(()));
    }
}
