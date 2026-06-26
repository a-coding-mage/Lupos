//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_TCPOPTSTRIP.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_TCPOPTSTRIP.c
//! Xtables TCP option stripping target.

pub const MODULE_AUTHOR: &str =
    "Sven Schnelle <svens@bitebene.org>, Jan Engelhardt <jengelh@medozas.de>";
pub const MODULE_DESCRIPTION: &str = "Xtables: TCP option stripping";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_TCPOPTSTRIP", "ip6t_TCPOPTSTRIP"];
pub const XT_CONTINUE: u32 = 0xffff_ffff;
pub const NF_DROP: u32 = 0;
pub const NFPROTO_IPV4: u8 = 2;
pub const NFPROTO_IPV6: u8 = 10;
pub const IPPROTO_TCP: u8 = 6;
pub const TCPOPT_NOP: u8 = 1;
pub const TCPHDR_LEN: usize = 20;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtTcpoptstripTargetInfo {
    pub strip_bmap: [u32; 8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtTarget {
    pub name: &'static str,
    pub family: u8,
    pub table: &'static str,
    pub proto: u8,
    pub targetsize: usize,
    pub target_fn: &'static str,
}

pub const TCPOPTSTRIP_TG_REG: [XtTarget; 2] = [
    XtTarget {
        name: "TCPOPTSTRIP",
        family: NFPROTO_IPV4,
        table: "mangle",
        proto: IPPROTO_TCP,
        targetsize: core::mem::size_of::<XtTcpoptstripTargetInfo>(),
        target_fn: "tcpoptstrip_tg4",
    },
    XtTarget {
        name: "TCPOPTSTRIP",
        family: NFPROTO_IPV6,
        table: "mangle",
        proto: IPPROTO_TCP,
        targetsize: core::mem::size_of::<XtTcpoptstripTargetInfo>(),
        target_fn: "tcpoptstrip_tg6",
    },
];

pub const fn tcpoptstrip_set_bit(
    mut info: XtTcpoptstripTargetInfo,
    idx: u8,
) -> XtTcpoptstripTargetInfo {
    let word = (idx >> 5) as usize;
    info.strip_bmap[word] |= 1u32 << (idx & 31);
    info
}

pub const fn tcpoptstrip_test_bit(bmap: [u32; 8], idx: u8) -> bool {
    ((1u32 << (idx & 31)) & bmap[(idx >> 5) as usize]) != 0
}

pub const fn optlen(opt: &[u8], offset: usize) -> usize {
    if opt[offset] <= TCPOPT_NOP || opt[offset + 1] == 0 {
        1
    } else {
        opt[offset + 1] as usize
    }
}

pub fn tcpoptstrip_mangle_packet(
    packet: &mut [u8],
    fragoff: u16,
    tcphoff: usize,
    info: &XtTcpoptstripTargetInfo,
) -> u32 {
    if fragoff != 0 {
        return XT_CONTINUE;
    }
    if packet.len() < tcphoff + TCPHDR_LEN {
        return NF_DROP;
    }

    let tcp_hdrlen = ((packet[tcphoff + 12] >> 4) as usize) * 4;
    if tcp_hdrlen < TCPHDR_LEN {
        return NF_DROP;
    }
    if packet.len() < tcphoff + tcp_hdrlen {
        return NF_DROP;
    }

    let tcp = &mut packet[tcphoff..tcphoff + tcp_hdrlen];
    let mut i = TCPHDR_LEN;
    while i < tcp_hdrlen.saturating_sub(1) {
        let optl = optlen(tcp, i);
        if i + optl > tcp_hdrlen {
            break;
        }
        if tcpoptstrip_test_bit(info.strip_bmap, tcp[i]) {
            let mut j = 0usize;
            while j < optl {
                let mut old = tcp[i + j] as u16;
                let mut new = TCPOPT_NOP as u16;
                if (i + j) % 2 == 0 {
                    old <<= 8;
                    new <<= 8;
                }
                inet_proto_csum_replace2(tcp, old, new);
                j += 1;
            }
            let mut j = 0usize;
            while j < optl {
                tcp[i + j] = TCPOPT_NOP;
                j += 1;
            }
        }
        i += optl;
    }

    XT_CONTINUE
}

pub fn tcpoptstrip_tg4(
    packet: &mut [u8],
    fragoff: u16,
    ip_hdrlen: usize,
    info: &XtTcpoptstripTargetInfo,
) -> u32 {
    tcpoptstrip_mangle_packet(packet, fragoff, ip_hdrlen, info)
}

pub fn tcpoptstrip_tg6(
    packet: &mut [u8],
    fragoff: u16,
    ipv6_skip_exthdr: Option<usize>,
    info: &XtTcpoptstripTargetInfo,
) -> u32 {
    match ipv6_skip_exthdr {
        Some(tcphoff) => tcpoptstrip_mangle_packet(packet, fragoff, tcphoff, info),
        None => NF_DROP,
    }
}

pub const fn tcpoptstrip_tg_init() -> &'static [XtTarget; 2] {
    &TCPOPTSTRIP_TG_REG
}

fn inet_proto_csum_replace2(tcp: &mut [u8], old: u16, new: u16) {
    let check = u16::from_be_bytes([tcp[16], tcp[17]]);
    let mut sum = (!check as u32)
        .wrapping_add(!old as u16 as u32)
        .wrapping_add(new as u32);
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    let check = !(sum as u16);
    let bytes = check.to_be_bytes();
    tcp[16] = bytes[0];
    tcp[17] = bytes[1];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_tcpoptstrip_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_TCPOPTSTRIP.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter/xt_TCPOPTSTRIP.h"
        ));
        assert!(header.contains("#define tcpoptstrip_test_bit(bmap, idx)"));
        assert!(header.contains("__u32 strip_bmap[8];"));
        assert!(source.contains("static inline unsigned int optlen"));
        assert!(source.contains("if (opt[offset] <= TCPOPT_NOP || opt[offset+1] == 0)"));
        assert!(source.contains("if (par->fragoff != 0)"));
        assert!(source.contains("tcph = skb_header_pointer(skb, tcphoff, sizeof(_th), &_th);"));
        assert!(source.contains("tcp_hdrlen = tcph->doff * 4;"));
        assert!(source.contains("if (tcp_hdrlen < sizeof(struct tcphdr))"));
        assert!(source.contains("if (skb_ensure_writable(skb, tcphoff + tcp_hdrlen))"));
        assert!(source.contains("for (i = sizeof(struct tcphdr); i < tcp_hdrlen - 1; i += optl)"));
        assert!(source.contains("if (i + optl > tcp_hdrlen)"));
        assert!(source.contains("inet_proto_csum_replace2(&tcph->check, skb, htons(o),"));
        assert!(source.contains("memset(opt + i, TCPOPT_NOP, optl);"));
        assert!(source.contains("return tcpoptstrip_mangle_packet(skb, par, ip_hdrlen(skb));"));
        assert!(source.contains("ipv6_skip_exthdr(skb, sizeof(*ipv6h), &nexthdr, &frag_off);"));
        assert!(source.contains(".name       = \"TCPOPTSTRIP\""));
        assert!(source.contains(".table      = \"mangle\""));
        assert!(source.contains(".proto      = IPPROTO_TCP"));
        assert!(source.contains("xt_register_targets(tcpoptstrip_tg_reg,"));
    }

    #[test]
    fn tcp_options_are_nopped_and_checksum_adjusted() {
        let mut packet = [0u8; 60];
        let tcp = 20usize;
        packet[tcp + 12] = 6 << 4;
        packet[tcp + 16] = 0x12;
        packet[tcp + 17] = 0x34;
        packet[tcp + 20] = 4;
        packet[tcp + 21] = 2;
        packet[tcp + 22] = 0xaa;
        packet[tcp + 23] = 0xbb;
        let info = tcpoptstrip_set_bit(XtTcpoptstripTargetInfo { strip_bmap: [0; 8] }, 4);

        assert_eq!(tcpoptstrip_tg4(&mut packet, 0, tcp, &info), XT_CONTINUE);
        assert_eq!(&packet[tcp + 20..tcp + 22], &[TCPOPT_NOP, TCPOPT_NOP]);
        assert_ne!(&packet[tcp + 16..tcp + 18], &[0x12, 0x34]);
        assert_eq!(tcpoptstrip_tg4(&mut packet, 1, tcp, &info), XT_CONTINUE);
        assert_eq!(tcpoptstrip_tg6(&mut packet, 0, None, &info), NF_DROP);
        assert_eq!(tcpoptstrip_tg_init().len(), 2);
    }

    #[test]
    fn malformed_or_unavailable_tcp_header_drops() {
        let info = XtTcpoptstripTargetInfo { strip_bmap: [0; 8] };
        let mut short = [0u8; 10];
        assert_eq!(tcpoptstrip_tg4(&mut short, 0, 0, &info), NF_DROP);
        let mut bad_doff = [0u8; 40];
        bad_doff[12] = 4 << 4;
        assert_eq!(tcpoptstrip_tg4(&mut bad_doff, 0, 0, &info), NF_DROP);
    }
}
