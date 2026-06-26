//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_tcpmss.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_tcpmss.c
//! Xtables TCP MSS option match.

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "Marc Boucher <marc@mbsi.ca>";
pub const MODULE_DESCRIPTION: &str = "Xtables: TCP MSS match";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_tcpmss", "ip6t_tcpmss"];

pub const NFPROTO_IPV4: u8 = 2;
pub const NFPROTO_IPV6: u8 = 10;
pub const IPPROTO_TCP: u8 = 6;
pub const TCPOPT_MSS: u8 = 2;
pub const TCPOLEN_MSS: usize = 4;
pub const TCP_HEADER_LEN: usize = 20;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtTcpmssMatchInfo {
    pub mss_min: u16,
    pub mss_max: u16,
    pub invert: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct XtActionParam {
    pub thoff: usize,
    pub fragoff: bool,
    pub hotdrop: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub family: u8,
    pub matchsize: usize,
    pub proto: u8,
}

pub const TCPMSS_MT_REG: [XtMatch; 2] = [
    XtMatch {
        name: "tcpmss",
        family: NFPROTO_IPV4,
        matchsize: core::mem::size_of::<XtTcpmssMatchInfo>(),
        proto: IPPROTO_TCP,
    },
    XtMatch {
        name: "tcpmss",
        family: NFPROTO_IPV6,
        matchsize: core::mem::size_of::<XtTcpmssMatchInfo>(),
        proto: IPPROTO_TCP,
    },
];

pub fn tcpmss_mt(skb: &[u8], par: &mut XtActionParam, info: XtTcpmssMatchInfo) -> bool {
    if par.fragoff {
        return false;
    }

    let Some(th) = skb.get(par.thoff..par.thoff + TCP_HEADER_LEN) else {
        par.hotdrop = true;
        return false;
    };
    let tcp_len = ((th[12] >> 4) as usize) * 4;
    if tcp_len < TCP_HEADER_LEN {
        par.hotdrop = true;
        return false;
    }

    let optlen = tcp_len - TCP_HEADER_LEN;
    if optlen == 0 {
        return info.invert;
    }
    let Some(op) = skb.get(par.thoff + TCP_HEADER_LEN..par.thoff + TCP_HEADER_LEN + optlen) else {
        par.hotdrop = true;
        return false;
    };

    let mut i = 0usize;
    while i < optlen {
        if op[i] == TCPOPT_MSS && optlen - i >= TCPOLEN_MSS && op[i + 1] as usize == TCPOLEN_MSS {
            let mssval = u16::from_be_bytes([op[i + 2], op[i + 3]]);
            return (mssval >= info.mss_min && mssval <= info.mss_max) != info.invert;
        }
        if op[i] < 2 || i == optlen - 1 {
            i += 1;
        } else {
            let step = op[i + 1] as usize;
            i += if step == 0 { 1 } else { step };
        }
    }
    info.invert
}

pub const fn tcpmss_mt_init() -> &'static [XtMatch; 2] {
    &TCPMSS_MT_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tcp_header_with_options(options: &[u8]) -> [u8; 60] {
        let mut tcp = [0u8; 60];
        let len = TCP_HEADER_LEN + options.len();
        tcp[12] = ((len / 4) as u8) << 4;
        tcp[TCP_HEADER_LEN..TCP_HEADER_LEN + options.len()].copy_from_slice(options);
        tcp
    }

    #[test]
    fn xt_tcpmss_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_tcpmss.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter/xt_tcpmss.h"
        ));
        assert!(header.contains("struct xt_tcpmss_match_info"));
        assert!(source.contains("MODULE_ALIAS(\"ipt_tcpmss\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_tcpmss\");"));
        assert!(source.contains("tcpmss_mt(const struct sk_buff *skb"));
        assert!(source.contains("if (par->fragoff)"));
        assert!(source.contains("skb_header_pointer(skb, par->thoff, sizeof(_tcph), &_tcph)"));
        assert!(source.contains("if (th->doff*4 < sizeof(*th))"));
        assert!(source.contains("if (op[i] == TCPOPT_MSS"));
        assert!(source.contains("mssval = (op[i+2] << 8) | op[i+3];"));
        assert!(source.contains("return info->invert;"));
        assert!(source.contains("par->hotdrop = true;"));
        assert!(source.contains(".name\t\t= \"tcpmss\""));
        assert!(source.contains(".family\t\t= NFPROTO_IPV4"));
        assert!(source.contains(".family\t\t= NFPROTO_IPV6"));
        assert!(source.contains(".proto\t\t= IPPROTO_TCP"));
        assert!(source.contains("xt_register_matches(tcpmss_mt_reg, ARRAY_SIZE(tcpmss_mt_reg));"));
    }

    #[test]
    fn tcpmss_scans_options_and_hotdrops_truncation() {
        let info = XtTcpmssMatchInfo {
            mss_min: 1200,
            mss_max: 1460,
            invert: false,
        };
        let tcp = tcp_header_with_options(&[1, TCPOPT_MSS, 4, 0x05, 0xb4, 0, 0, 0]);
        let mut par = XtActionParam::default();
        assert!(tcpmss_mt(&tcp, &mut par, info));
        assert!(!par.hotdrop);

        let mut par = XtActionParam::default();
        assert!(!tcpmss_mt(&tcp[..22], &mut par, info));
        assert!(par.hotdrop);

        let mut par = XtActionParam {
            fragoff: true,
            ..XtActionParam::default()
        };
        assert!(!tcpmss_mt(&tcp, &mut par, info));
        assert!(!par.hotdrop);

        let no_options = tcp_header_with_options(&[]);
        assert!(!tcpmss_mt(&no_options, &mut XtActionParam::default(), info));
        assert!(tcpmss_mt(
            &no_options,
            &mut XtActionParam::default(),
            XtTcpmssMatchInfo {
                invert: true,
                ..info
            }
        ));
        assert_eq!(tcpmss_mt_init(), &TCPMSS_MT_REG);
    }
}
