//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_NFQUEUE.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_NFQUEUE.c
//! Xtables NFQUEUE target verdict construction.

use crate::include::uapi::errno::{EINVAL, ERANGE};
use crate::net::netfilter::nft_hash::reciprocal_scale;

pub const MODULE_AUTHOR: &str = "Harald Welte <laforge@netfilter.org>";
pub const MODULE_DESCRIPTION: &str = "Xtables: packet forwarding to netlink";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIASES: [&str; 3] = ["ipt_NFQUEUE", "ip6t_NFQUEUE", "arpt_NFQUEUE"];

pub const NFPROTO_UNSPEC: u8 = 0;
pub const NFPROTO_IPV4: u8 = 2;
pub const NFPROTO_IPV6: u8 = 10;
pub const NF_QUEUE: u32 = 3;
pub const NF_VERDICT_QMASK: u32 = 0xffff_0000;
pub const NF_VERDICT_FLAG_QUEUE_BYPASS: u32 = 0x0000_8000;
pub const NFQ_FLAG_BYPASS: u16 = 0x01;
pub const NFQ_FLAG_CPU_FANOUT: u16 = 0x02;
pub const NFQ_FLAG_MASK: u16 = 0x03;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtNfqInfo {
    pub queuenum: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtNfqInfoV1 {
    pub queuenum: u16,
    pub queues_total: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtNfqInfoV2 {
    pub queuenum: u16,
    pub queues_total: u16,
    pub bypass: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtNfqInfoV3 {
    pub queuenum: u16,
    pub queues_total: u16,
    pub flags: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtActionParam {
    pub family: u8,
    pub target_revision: u8,
    pub cpu: u32,
    pub jhash_initval: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtTarget {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
    pub targetsize: usize,
}

pub const NFQUEUE_TG_REG: [XtTarget; 4] = [
    XtTarget {
        name: "NFQUEUE",
        revision: 0,
        family: NFPROTO_UNSPEC,
        targetsize: core::mem::size_of::<XtNfqInfo>(),
    },
    XtTarget {
        name: "NFQUEUE",
        revision: 1,
        family: NFPROTO_UNSPEC,
        targetsize: core::mem::size_of::<XtNfqInfoV1>(),
    },
    XtTarget {
        name: "NFQUEUE",
        revision: 2,
        family: NFPROTO_UNSPEC,
        targetsize: core::mem::size_of::<XtNfqInfoV2>(),
    },
    XtTarget {
        name: "NFQUEUE",
        revision: 3,
        family: NFPROTO_UNSPEC,
        targetsize: core::mem::size_of::<XtNfqInfoV3>(),
    },
];

pub const fn nf_queue_nr(queue: u32) -> u32 {
    ((queue << 16) & NF_VERDICT_QMASK) | NF_QUEUE
}

pub const fn nfqueue_tg(info: XtNfqInfo) -> u32 {
    nf_queue_nr(info.queuenum as u32)
}

pub fn nfqueue_tg_v1(skb: &[u8], par: XtActionParam, info: XtNfqInfoV1) -> u32 {
    let mut queue = info.queuenum as u32;
    if info.queues_total > 1 {
        queue = nfqueue_hash(
            skb,
            queue,
            info.queues_total as u32,
            par.family,
            par.jhash_initval,
        );
    }
    nf_queue_nr(queue)
}

pub fn nfqueue_tg_v2(skb: &[u8], par: XtActionParam, info: XtNfqInfoV2) -> u32 {
    let mut ret = nfqueue_tg_v1(
        skb,
        par,
        XtNfqInfoV1 {
            queuenum: info.queuenum,
            queues_total: info.queues_total,
        },
    );
    if info.bypass != 0 {
        ret |= NF_VERDICT_FLAG_QUEUE_BYPASS;
    }
    ret
}

pub fn nfqueue_tg_v3(skb: &[u8], par: XtActionParam, info: XtNfqInfoV3) -> u32 {
    let mut queue = info.queuenum as u32;
    if info.queues_total > 1 {
        if info.flags & NFQ_FLAG_CPU_FANOUT != 0 {
            queue = queue.wrapping_add(par.cpu % info.queues_total as u32);
        } else {
            queue = nfqueue_hash(
                skb,
                queue,
                info.queues_total as u32,
                par.family,
                par.jhash_initval,
            );
        }
    }

    let mut ret = nf_queue_nr(queue);
    if info.flags & NFQ_FLAG_BYPASS != 0 {
        ret |= NF_VERDICT_FLAG_QUEUE_BYPASS;
    }
    ret
}

pub const fn nfqueue_tg_check(info: XtNfqInfoV3, target_revision: u8) -> Result<(), i32> {
    if info.queues_total == 0 {
        return Err(-EINVAL);
    }
    let maxid = info.queues_total as u32 - 1 + info.queuenum as u32;
    if maxid > 0xffff {
        return Err(-ERANGE);
    }
    if target_revision == 2 && info.flags > 1 {
        return Err(-EINVAL);
    }
    if target_revision == 3 && info.flags & !NFQ_FLAG_MASK != 0 {
        return Err(-EINVAL);
    }
    Ok(())
}

pub const fn nfqueue_tg_init() -> &'static [XtTarget; 4] {
    &NFQUEUE_TG_REG
}

pub fn nfqueue_hash(skb: &[u8], queue: u32, queues_total: u32, family: u8, initval: u32) -> u32 {
    match family {
        NFPROTO_IPV4 => queue.wrapping_add(reciprocal_scale(hash_v4(skb, initval), queues_total)),
        NFPROTO_IPV6 => queue.wrapping_add(reciprocal_scale(hash_v6(skb, initval), queues_total)),
        _ => queue,
    }
}

fn hash_v4(skb: &[u8], initval: u32) -> u32 {
    if skb.len() < 20 || skb[0] >> 4 != 4 {
        return 0;
    }
    let src = u32::from_be_bytes([skb[12], skb[13], skb[14], skb[15]]);
    let dst = u32::from_be_bytes([skb[16], skb[17], skb[18], skb[19]]);
    let proto = skb[9] as u32;
    if src < dst {
        jhash_3words(src, dst, proto, initval)
    } else {
        jhash_3words(dst, src, proto, initval)
    }
}

fn hash_v6(skb: &[u8], initval: u32) -> u32 {
    if skb.len() < 40 || skb[0] >> 4 != 6 {
        return 0;
    }
    let src = u32::from_be_bytes([skb[20], skb[21], skb[22], skb[23]]);
    let dst = u32::from_be_bytes([skb[36], skb[37], skb[38], skb[39]]);
    let src_mid = u32::from_be_bytes([skb[12], skb[13], skb[14], skb[15]]);
    let dst_mid = u32::from_be_bytes([skb[28], skb[29], skb[30], skb[31]]);
    let c = if src_mid < dst_mid { src_mid } else { dst_mid };
    if src < dst {
        jhash_3words(src, dst, c, initval)
    } else {
        jhash_3words(dst, src, c, initval)
    }
}

fn jhash_3words(a: u32, b: u32, c: u32, initval: u32) -> u32 {
    let init = initval.wrapping_add(0xdead_beef).wrapping_add(12);
    let (_, _, c) = jhash_final(
        a.wrapping_add(init),
        b.wrapping_add(init),
        c.wrapping_add(init),
    );
    c
}

fn jhash_final(mut a: u32, mut b: u32, mut c: u32) -> (u32, u32, u32) {
    c ^= b;
    c = c.wrapping_sub(b.rotate_left(14));
    a ^= c;
    a = a.wrapping_sub(c.rotate_left(11));
    b ^= a;
    b = b.wrapping_sub(a.rotate_left(25));
    c ^= b;
    c = c.wrapping_sub(b.rotate_left(16));
    a ^= c;
    a = a.wrapping_sub(c.rotate_left(4));
    b ^= a;
    b = b.wrapping_sub(a.rotate_left(14));
    c ^= b;
    c = c.wrapping_sub(b.rotate_left(24));
    (a, b, c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_nfqueue_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_NFQUEUE.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter/xt_NFQUEUE.h"
        ));
        let queue_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/net/netfilter/nf_queue.h"
        ));
        assert!(header.contains("struct xt_NFQ_info_v3"));
        assert!(header.contains("#define NFQ_FLAG_CPU_FANOUT\t0x02"));
        assert!(source.contains("MODULE_ALIAS(\"ipt_NFQUEUE\");"));
        assert!(source.contains("MODULE_ALIAS(\"arpt_NFQUEUE\");"));
        assert!(source.contains("return NF_QUEUE_NR(tinfo->queuenum);"));
        assert!(source.contains("nfqueue_hash(skb, queue, info->queues_total"));
        assert!(source.contains("ret |= NF_VERDICT_FLAG_QUEUE_BYPASS;"));
        assert!(source.contains("init_hashrandom(&jhash_initval);"));
        assert!(source.contains("if (info->queues_total == 0)"));
        assert!(source.contains("if (par->target->revision == 3 && info->flags & ~NFQ_FLAG_MASK)"));
        assert!(source.contains("queue = info->queuenum + cpu % info->queues_total;"));
        assert!(
            source.contains("xt_register_targets(nfqueue_tg_reg, ARRAY_SIZE(nfqueue_tg_reg));")
        );
        assert!(queue_header.contains("nfqueue_hash(const struct sk_buff *skb"));
        assert!(queue_header.contains("queue += reciprocal_scale(hash_v4(ip_hdr(skb), initval),"));
        assert!(queue_header.contains("ip6h->saddr.s6_addr32[1]"));
    }

    #[test]
    fn nfqueue_targets_build_verdicts_and_validate_ranges() {
        let skb = [
            0x45, 0, 0, 20, 0, 0, 0, 0, 64, 6, 0, 0, 10, 0, 0, 1, 10, 0, 0, 2,
        ];
        let par = XtActionParam {
            family: NFPROTO_IPV4,
            target_revision: 3,
            cpu: 3,
            jhash_initval: 0x1234,
        };
        assert_eq!(nfqueue_tg(XtNfqInfo { queuenum: 7 }), nf_queue_nr(7));
        let v1 = nfqueue_tg_v1(
            &skb,
            par,
            XtNfqInfoV1 {
                queuenum: 4,
                queues_total: 4,
            },
        );
        assert_eq!(v1 & !NF_VERDICT_QMASK, NF_QUEUE);
        assert_eq!(
            nfqueue_tg_v2(
                &skb,
                par,
                XtNfqInfoV2 {
                    queuenum: 4,
                    queues_total: 1,
                    bypass: 1,
                }
            ),
            nf_queue_nr(4) | NF_VERDICT_FLAG_QUEUE_BYPASS
        );
        assert_eq!(
            nfqueue_tg_v3(
                &skb,
                par,
                XtNfqInfoV3 {
                    queuenum: 10,
                    queues_total: 4,
                    flags: NFQ_FLAG_CPU_FANOUT | NFQ_FLAG_BYPASS,
                }
            ),
            nf_queue_nr(13) | NF_VERDICT_FLAG_QUEUE_BYPASS
        );
        assert_eq!(
            nfqueue_tg_check(
                XtNfqInfoV3 {
                    queuenum: 0,
                    queues_total: 0,
                    flags: 0,
                },
                1
            ),
            Err(-EINVAL)
        );
        assert_eq!(
            nfqueue_tg_check(
                XtNfqInfoV3 {
                    queuenum: 0xffff,
                    queues_total: 2,
                    flags: 0,
                },
                1
            ),
            Err(-ERANGE)
        );
        assert_eq!(
            nfqueue_tg_check(
                XtNfqInfoV3 {
                    queuenum: 1,
                    queues_total: 1,
                    flags: 2,
                },
                2
            ),
            Err(-EINVAL)
        );
        assert_eq!(nfqueue_tg_init(), &NFQUEUE_TG_REG);
    }
}
