//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv6/ip6_icmp.c
//! test-origin: linux:vendor/linux/net/ipv6/ip6_icmp.c
//! ICMPv6 netdevice send helper with NAT source restoration.

pub const IPS_NAT_MASK: u32 = 0x30;
pub const IP_CT_IS_REPLY: u8 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NfConn {
    pub status: u32,
    pub tuple_src: [[u8; 16]; 2],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Icmpv6NdoSend {
    pub icmp_type: u8,
    pub code: u8,
    pub info: u32,
    pub cloned_skb: bool,
    pub sent_source: [u8; 16],
    pub restored_source: [u8; 16],
}

pub const fn ctinfo_to_dir(ctinfo: u8) -> usize {
    if ctinfo >= IP_CT_IS_REPLY { 1 } else { 0 }
}

pub fn icmpv6_ndo_send(
    skb_source: &mut [u8; 16],
    skb_shared: bool,
    ct: Option<NfConn>,
    ctinfo: u8,
    icmp_type: u8,
    code: u8,
    info: u32,
) -> Icmpv6NdoSend {
    let orig_ip = *skb_source;
    let Some(ct) = ct else {
        return Icmpv6NdoSend {
            icmp_type,
            code,
            info,
            cloned_skb: false,
            sent_source: orig_ip,
            restored_source: *skb_source,
        };
    };

    if ct.status & IPS_NAT_MASK == 0 {
        return Icmpv6NdoSend {
            icmp_type,
            code,
            info,
            cloned_skb: false,
            sent_source: orig_ip,
            restored_source: *skb_source,
        };
    }

    let dir = ctinfo_to_dir(ctinfo);
    *skb_source = ct.tuple_src[dir];
    let sent_source = *skb_source;
    *skb_source = orig_ip;

    Icmpv6NdoSend {
        icmp_type,
        code,
        info,
        cloned_skb: skb_shared,
        sent_source,
        restored_source: *skb_source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icmpv6_ndo_send_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv6/ip6_icmp.c"
        ));
        assert!(source.contains("void icmpv6_ndo_send"));
        assert!(source.contains("struct inet6_skb_parm parm = { 0 };"));
        assert!(source.contains("ct = nf_ct_get(skb_in, &ctinfo);"));
        assert!(source.contains("if (!ct || !(READ_ONCE(ct->status) & IPS_NAT_MASK))"));
        assert!(source.contains("icmp6_send(skb_in, type, code, info, NULL, &parm);"));
        assert!(source.contains("if (skb_shared(skb_in))"));
        assert!(source.contains("skb_in = cloned_skb = skb_clone(skb_in, GFP_ATOMIC);"));
        assert!(source.contains("orig_ip = ipv6_hdr(skb_in)->saddr;"));
        assert!(source.contains("dir = CTINFO2DIR(ctinfo);"));
        assert!(source.contains("ipv6_hdr(skb_in)->saddr = ct->tuplehash[dir].tuple.src.u3.in6;"));
        assert!(source.contains("ipv6_hdr(skb_in)->saddr = orig_ip;"));
        assert!(source.contains("consume_skb(cloned_skb);"));
        assert!(source.contains("EXPORT_SYMBOL(icmpv6_ndo_send);"));

        let original = [1; 16];
        let translated = [2; 16];
        let reply_translated = [3; 16];
        let ct = NfConn {
            status: IPS_NAT_MASK,
            tuple_src: [translated, reply_translated],
        };
        let mut source_addr = original;
        let record = icmpv6_ndo_send(&mut source_addr, true, Some(ct), 0, 1, 4, 1280);
        assert_eq!(record.sent_source, translated);
        assert_eq!(record.restored_source, original);
        assert_eq!(source_addr, original);
        assert!(record.cloned_skb);

        let mut source_addr = original;
        let record = icmpv6_ndo_send(&mut source_addr, false, Some(ct), IP_CT_IS_REPLY, 1, 4, 0);
        assert_eq!(record.sent_source, reply_translated);
        assert_eq!(record.restored_source, original);

        let mut source_addr = original;
        let record = icmpv6_ndo_send(
            &mut source_addr,
            true,
            Some(NfConn {
                status: 0,
                tuple_src: [translated, reply_translated],
            }),
            0,
            1,
            0,
            0,
        );
        assert_eq!(record.sent_source, original);
        assert!(!record.cloned_skb);
    }
}
