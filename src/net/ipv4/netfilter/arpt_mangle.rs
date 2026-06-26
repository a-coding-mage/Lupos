//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv4/netfilter/arpt_mangle.c
//! test-origin: linux:vendor/linux/net/ipv4/netfilter/arpt_mangle.c
//! Arptables ARP payload mangle target.

use crate::include::uapi::errno::EINVAL;

pub const ARPT_DEV_ADDR_LEN_MAX: usize = 16;
pub const ARPT_MANGLE_ADDR_LEN_MAX: usize = 4;
pub const ARPT_MANGLE_SDEV: u8 = 0x01;
pub const ARPT_MANGLE_TDEV: u8 = 0x02;
pub const ARPT_MANGLE_SIP: u8 = 0x04;
pub const ARPT_MANGLE_TIP: u8 = 0x08;
pub const ARPT_MANGLE_MASK: u8 = 0x0f;
pub const NF_DROP: u32 = 0;
pub const NF_ACCEPT: u32 = 1;
pub const XT_CONTINUE: u32 = 0xffff_ffff;
pub const NFPROTO_ARP: u8 = 3;
pub const ARPHRD_IEEE1394: u16 = 24;
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "Bart De Schuymer <bdschuym@pandora.be>";
pub const MODULE_DESCRIPTION: &str = "arptables arp payload mangle target";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArptMangle {
    pub src_devaddr: [u8; ARPT_DEV_ADDR_LEN_MAX],
    pub tgt_devaddr: [u8; ARPT_DEV_ADDR_LEN_MAX],
    pub src_ip: [u8; ARPT_MANGLE_ADDR_LEN_MAX],
    pub tgt_ip: [u8; ARPT_MANGLE_ADDR_LEN_MAX],
    pub flags: u8,
    pub target: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtTarget {
    pub name: &'static str,
    pub family: u8,
    pub targetsize: usize,
}

pub const ARPT_MANGLE_REG: XtTarget = XtTarget {
    name: "mangle",
    family: NFPROTO_ARP,
    targetsize: core::mem::size_of::<ArptMangle>(),
};

pub fn arpt_mangle_target(
    payload: &mut [u8],
    hln: usize,
    pln: usize,
    dev_type: u16,
    writable: bool,
    mangle: &ArptMangle,
) -> u32 {
    if !writable {
        return NF_DROP;
    }

    let mut off = 0usize;
    if mangle.flags & ARPT_MANGLE_SDEV != 0 {
        if hln > ARPT_DEV_ADDR_LEN_MAX || !copy_checked(payload, off, &mangle.src_devaddr[..hln]) {
            return NF_DROP;
        }
    }
    off = off.saturating_add(hln);

    if mangle.flags & ARPT_MANGLE_SIP != 0 {
        if pln > ARPT_MANGLE_ADDR_LEN_MAX || !copy_checked(payload, off, &mangle.src_ip[..pln]) {
            return NF_DROP;
        }
    }
    off = off.saturating_add(pln);

    if mangle.flags & ARPT_MANGLE_TDEV != 0 {
        if dev_type == ARPHRD_IEEE1394
            || hln > ARPT_DEV_ADDR_LEN_MAX
            || !copy_checked(payload, off, &mangle.tgt_devaddr[..hln])
        {
            return NF_DROP;
        }
    }
    off = off.saturating_add(hln);

    if mangle.flags & ARPT_MANGLE_TIP != 0 {
        if dev_type == ARPHRD_IEEE1394
            || pln > ARPT_MANGLE_ADDR_LEN_MAX
            || !copy_checked(payload, off, &mangle.tgt_ip[..pln])
        {
            return NF_DROP;
        }
    }
    mangle.target
}

pub fn checkentry(mangle: &ArptMangle) -> Result<(), i32> {
    if mangle.flags & !ARPT_MANGLE_MASK != 0 || mangle.flags & ARPT_MANGLE_MASK == 0 {
        return Err(-EINVAL);
    }
    if mangle.target != NF_DROP && mangle.target != NF_ACCEPT && mangle.target != XT_CONTINUE {
        return Err(-EINVAL);
    }
    Ok(())
}

pub const fn arpt_mangle_init() -> &'static XtTarget {
    &ARPT_MANGLE_REG
}

fn copy_checked(dst: &mut [u8], off: usize, src: &[u8]) -> bool {
    let Some(end) = off.checked_add(src.len()) else {
        return false;
    };
    let Some(out) = dst.get_mut(off..end) else {
        return false;
    };
    out.copy_from_slice(src);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mangle(flags: u8, target: u32) -> ArptMangle {
        let mut out = ArptMangle {
            src_devaddr: [0; ARPT_DEV_ADDR_LEN_MAX],
            tgt_devaddr: [0; ARPT_DEV_ADDR_LEN_MAX],
            src_ip: [192, 0, 2, 1],
            tgt_ip: [192, 0, 2, 99],
            flags,
            target,
        };
        out.src_devaddr[..6].copy_from_slice(&[1, 2, 3, 4, 5, 6]);
        out.tgt_devaddr[..6].copy_from_slice(&[6, 5, 4, 3, 2, 1]);
        out
    }

    #[test]
    fn arpt_mangle_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv4/netfilter/arpt_mangle.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter_arp/arpt_mangle.h"
        ));
        assert!(header.contains("#define ARPT_MANGLE_SDEV 0x01"));
        assert!(header.contains("#define ARPT_MANGLE_TDEV 0x02"));
        assert!(header.contains("#define ARPT_MANGLE_SIP 0x04"));
        assert!(header.contains("#define ARPT_MANGLE_TIP 0x08"));
        assert!(header.contains("#define ARPT_MANGLE_MASK 0x0f"));
        assert!(source.contains("MODULE_DESCRIPTION(\"arptables arp payload mangle target\")"));
        assert!(source.contains("target(struct sk_buff *skb"));
        assert!(source.contains("if (skb_ensure_writable(skb, skb->len))"));
        assert!(source.contains("return NF_DROP;"));
        assert!(source.contains("pln = arp->ar_pln;"));
        assert!(source.contains("hln = arp->ar_hln;"));
        assert!(source.contains("if (mangle->flags & ARPT_MANGLE_SDEV)"));
        assert!(source.contains("memcpy(arpptr, mangle->src_devaddr, hln);"));
        assert!(source.contains("if (mangle->flags & ARPT_MANGLE_SIP)"));
        assert!(source.contains("memcpy(arpptr, &mangle->u_s.src_ip, pln);"));
        assert!(source.contains("if (unlikely(IS_ENABLED(CONFIG_FIREWIRE_NET)"));
        assert!(source.contains("skb->dev->type == ARPHRD_IEEE1394"));
        assert!(source.contains("memcpy(arpptr, mangle->tgt_devaddr, hln);"));
        assert!(source.contains("memcpy(arpptr, &mangle->u_t.tgt_ip, pln);"));
        assert!(source.contains("return mangle->target;"));
        assert!(source.contains("if (mangle->flags & ~ARPT_MANGLE_MASK ||"));
        assert!(source.contains("mangle->target != NF_DROP && mangle->target != NF_ACCEPT"));
        assert!(source.contains("mangle->target != XT_CONTINUE"));
        assert!(source.contains(".name\t\t= \"mangle\""));
        assert!(source.contains(".family\t\t= NFPROTO_ARP"));
    }

    #[test]
    fn arp_payload_mangle_rewrites_selected_fields_and_checks_target() {
        let mangle = mangle(
            ARPT_MANGLE_SDEV | ARPT_MANGLE_SIP | ARPT_MANGLE_TDEV | ARPT_MANGLE_TIP,
            XT_CONTINUE,
        );
        assert_eq!(checkentry(&mangle), Ok(()));
        let mut payload = [0; 20];
        assert_eq!(
            arpt_mangle_target(&mut payload, 6, 4, 1, true, &mangle),
            XT_CONTINUE
        );
        assert_eq!(&payload[0..6], &[1, 2, 3, 4, 5, 6]);
        assert_eq!(&payload[6..10], &[192, 0, 2, 1]);
        assert_eq!(&payload[10..16], &[6, 5, 4, 3, 2, 1]);
        assert_eq!(&payload[16..20], &[192, 0, 2, 99]);

        assert_eq!(
            arpt_mangle_target(&mut payload, 6, 4, ARPHRD_IEEE1394, true, &mangle),
            NF_DROP
        );
        assert_eq!(
            arpt_mangle_target(&mut payload[..2], 6, 4, 1, true, &mangle),
            NF_DROP
        );
        assert_eq!(checkentry(&ArptMangle { flags: 0, ..mangle }), Err(-EINVAL));
        assert_eq!(
            checkentry(&ArptMangle {
                target: 99,
                ..mangle
            }),
            Err(-EINVAL)
        );
        assert_eq!(arpt_mangle_init(), &ARPT_MANGLE_REG);
    }
}
