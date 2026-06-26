//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv4/xfrm4_output.c
//! test-origin: linux:vendor/linux/net/ipv4/xfrm4_output.c
//! IPv4 XFRM output routing decisions.

pub const NFPROTO_IPV4: u8 = 2;
pub const NF_INET_POST_ROUTING: u8 = 4;
pub const IPSKB_REROUTED: u32 = 1 << 4;
pub const EMSGSIZE: i32 = 90;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Xfrm4OutputAction {
    DstOutput,
    XfrmOutput,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Xfrm4OutputTrace {
    pub nfproto: u8,
    pub hook: u8,
    pub ran_post_routing_hook: bool,
    pub action: Xfrm4OutputAction,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Xfrm4Packet {
    pub flags: u32,
    pub has_xfrm: bool,
    pub encapsulation: bool,
    pub daddr: u32,
    pub inner_daddr: u32,
    pub dport: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Xfrm4LocalError {
    pub errno: i32,
    pub daddr: u32,
    pub dport: u16,
    pub mtu: u32,
}

pub fn xfrm4_output(packet: &mut Xfrm4Packet) -> Xfrm4OutputTrace {
    let ran_post_routing_hook = packet.flags & IPSKB_REROUTED == 0;
    let action = if packet.has_xfrm {
        Xfrm4OutputAction::XfrmOutput
    } else {
        packet.flags |= IPSKB_REROUTED;
        Xfrm4OutputAction::DstOutput
    };

    Xfrm4OutputTrace {
        nfproto: NFPROTO_IPV4,
        hook: NF_INET_POST_ROUTING,
        ran_post_routing_hook,
        action,
        flags: packet.flags,
    }
}

pub fn xfrm4_local_error(packet: &Xfrm4Packet, mtu: u32) -> Xfrm4LocalError {
    Xfrm4LocalError {
        errno: EMSGSIZE,
        daddr: if packet.encapsulation {
            packet.inner_daddr
        } else {
            packet.daddr
        },
        dport: packet.dport,
        mtu,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xfrm4_output_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv4/xfrm4_output.c"
        ));
        assert!(source.contains("static int __xfrm4_output"));
        assert!(source.contains("struct xfrm_state *x = skb_dst(skb)->xfrm;"));
        assert!(source.contains("if (!x) {"));
        assert!(source.contains("IPCB(skb)->flags |= IPSKB_REROUTED;"));
        assert!(source.contains("return dst_output(net, sk, skb);"));
        assert!(source.contains("return xfrm_output(sk, skb);"));
        assert!(source.contains("NF_HOOK_COND(NFPROTO_IPV4, NF_INET_POST_ROUTING"));
        assert!(source.contains("!(IPCB(skb)->flags & IPSKB_REROUTED)"));
        assert!(source.contains("hdr = skb->encapsulation ? inner_ip_hdr(skb) : ip_hdr(skb);"));
        assert!(source.contains("ip_local_error(skb->sk, EMSGSIZE, hdr->daddr"));

        let mut packet = Xfrm4Packet {
            flags: 0,
            has_xfrm: false,
            encapsulation: false,
            daddr: 0x0a00_0002,
            inner_daddr: 0x0a00_0003,
            dport: 4500,
        };
        let trace = xfrm4_output(&mut packet);
        assert!(trace.ran_post_routing_hook);
        assert_eq!(trace.action, Xfrm4OutputAction::DstOutput);
        assert_eq!(trace.flags & IPSKB_REROUTED, IPSKB_REROUTED);

        packet.has_xfrm = true;
        let trace = xfrm4_output(&mut packet);
        assert!(!trace.ran_post_routing_hook);
        assert_eq!(trace.action, Xfrm4OutputAction::XfrmOutput);

        packet.encapsulation = true;
        assert_eq!(
            xfrm4_local_error(&packet, 1400),
            Xfrm4LocalError {
                errno: EMSGSIZE,
                daddr: 0x0a00_0003,
                dport: 4500,
                mtu: 1400,
            }
        );
    }
}
