//! linux-parity: complete
//! linux-source: vendor/linux/net/6lowpan/nhc.c
//! test-origin: linux:vendor/linux/net/6lowpan/nhc.c
//! 6LoWPAN next-header compression lookup and registration contracts.

use core::fmt;

use crate::include::uapi::errno::{EEXIST, EINVAL, ENOENT};

pub const NEXTHDR_MAX: usize = 255;
pub const LOWPAN_NEXTHDR_TABLE_LEN: usize = NEXTHDR_MAX + 1;
pub const IPV6HDR_SIZE: usize = 40;
pub const ENOTSUPP: i32 = 524;

pub type LowpanNhcCompress = for<'a> fn(&mut LowpanSkb<'a>) -> i32;
pub type LowpanNhcUncompress = for<'a> fn(&mut LowpanSkb<'a>, usize) -> i32;

#[derive(Clone, Copy)]
pub struct LowpanNhc {
    pub name: &'static str,
    pub nexthdr: u8,
    pub nexthdrlen: usize,
    pub id: u8,
    pub idmask: u8,
    pub uncompress: Option<LowpanNhcUncompress>,
    pub compress: Option<LowpanNhcCompress>,
}

impl LowpanNhc {
    pub const fn matches_nhcid(self, nhcid: u8) -> bool {
        nhcid & self.idmask == self.id
    }
}

impl fmt::Debug for LowpanNhc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LowpanNhc")
            .field("name", &self.name)
            .field("nexthdr", &self.nexthdr)
            .field("nexthdrlen", &self.nexthdrlen)
            .field("id", &self.id)
            .field("idmask", &self.idmask)
            .field("uncompress", &self.uncompress.is_some())
            .field("compress", &self.compress.is_some())
            .finish()
    }
}

impl PartialEq for LowpanNhc {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.nexthdr == other.nexthdr
            && self.nexthdrlen == other.nexthdrlen
            && self.id == other.id
            && self.idmask == other.idmask
            && self.uncompress.is_some() == other.uncompress.is_some()
            && self.compress.is_some() == other.compress.is_some()
    }
}

impl Eq for LowpanNhc {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ipv6Header {
    pub nexthdr: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LowpanSkb<'a> {
    pub data: &'a [u8],
    pub network_header: usize,
    pub transport_header: usize,
    pub pulled: usize,
    pub transport_header_reset: bool,
    pub callback_invocations: usize,
    pub last_uncompress_needed: Option<usize>,
}

impl<'a> LowpanSkb<'a> {
    pub const fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            network_header: 0,
            transport_header: 0,
            pulled: 0,
            transport_header_reset: false,
            callback_invocations: 0,
            last_uncompress_needed: None,
        }
    }

    pub const fn pskb_may_pull(&self, len: usize) -> bool {
        self.data.len() >= len
    }

    pub const fn data_id(&self) -> Option<u8> {
        if self.pskb_may_pull(1) {
            Some(self.data[0])
        } else {
            None
        }
    }

    pub fn skb_set_transport_header(&mut self, offset: usize) {
        self.transport_header = self.network_header + offset;
    }

    pub fn skb_reset_transport_header(&mut self) {
        self.transport_header = self.pulled;
        self.transport_header_reset = true;
    }

    pub fn skb_pull(&mut self, len: usize) {
        self.pulled += len;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LowpanNhcTable {
    lowpan_nexthdr_nhcs: [Option<LowpanNhc>; LOWPAN_NEXTHDR_TABLE_LEN],
}

impl LowpanNhcTable {
    pub const fn new() -> Self {
        Self {
            lowpan_nexthdr_nhcs: [None; LOWPAN_NEXTHDR_TABLE_LEN],
        }
    }

    pub fn slot(&self, nexthdr: u8) -> Option<LowpanNhc> {
        self.lowpan_nexthdr_nhcs[nexthdr as usize]
    }
}

impl Default for LowpanNhcTable {
    fn default() -> Self {
        Self::new()
    }
}

pub fn lowpan_nhc_by_nhcid(table: &LowpanNhcTable, skb: &LowpanSkb<'_>) -> Option<LowpanNhc> {
    let id = skb.data_id()?;

    for nhc in table.lowpan_nexthdr_nhcs.iter().flatten() {
        if nhc.matches_nhcid(id) {
            return Some(*nhc);
        }
    }

    None
}

pub fn lowpan_nhc_check_compression(table: &LowpanNhcTable, hdr: &Ipv6Header) -> i32 {
    match table.lowpan_nexthdr_nhcs[hdr.nexthdr as usize] {
        Some(nhc) if nhc.compress.is_some() => 0,
        _ => -ENOENT,
    }
}

pub fn lowpan_nhc_do_compression(
    table: &LowpanNhcTable,
    skb: &mut LowpanSkb<'_>,
    hdr: &Ipv6Header,
) -> i32 {
    let nhc = match table.lowpan_nexthdr_nhcs[hdr.nexthdr as usize] {
        Some(nhc) if nhc.compress.is_some() => nhc,
        _ => return -EINVAL,
    };

    if skb.transport_header == skb.network_header {
        skb.skb_set_transport_header(IPV6HDR_SIZE);
    }

    let ret = (nhc.compress.expect("checked above"))(skb);
    if ret < 0 {
        return ret;
    }

    skb.skb_pull(nhc.nexthdrlen);
    ret
}

pub fn lowpan_nhc_do_uncompression(
    table: &LowpanNhcTable,
    skb: &mut LowpanSkb<'_>,
    hdr: &mut Ipv6Header,
) -> i32 {
    let nhc = match lowpan_nhc_by_nhcid(table, skb) {
        Some(nhc) => nhc,
        None => return -ENOENT,
    };

    match nhc.uncompress {
        Some(uncompress) => {
            let ret = uncompress(skb, IPV6HDR_SIZE + nhc.nexthdrlen);
            if ret < 0 {
                return ret;
            }
        }
        None => return -ENOTSUPP,
    }

    hdr.nexthdr = nhc.nexthdr;
    skb.skb_reset_transport_header();
    0
}

pub fn lowpan_nhc_add(table: &mut LowpanNhcTable, nhc: LowpanNhc) -> i32 {
    let slot = &mut table.lowpan_nexthdr_nhcs[nhc.nexthdr as usize];
    if slot.is_some() {
        return -EEXIST;
    }

    *slot = Some(nhc);
    0
}

pub fn lowpan_nhc_del(table: &mut LowpanNhcTable, nhc: LowpanNhc) {
    table.lowpan_nexthdr_nhcs[nhc.nexthdr as usize] = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    const UDP_NEXTHDR: u8 = 17;
    const IPV6_NEXTHDR: u8 = 41;

    fn compress_ok(skb: &mut LowpanSkb<'_>) -> i32 {
        skb.callback_invocations += 1;
        0
    }

    fn compress_error(skb: &mut LowpanSkb<'_>) -> i32 {
        skb.callback_invocations += 1;
        -EINVAL
    }

    fn uncompress_ok(skb: &mut LowpanSkb<'_>, needed: usize) -> i32 {
        skb.callback_invocations += 1;
        skb.last_uncompress_needed = Some(needed);
        0
    }

    fn uncompress_error(skb: &mut LowpanSkb<'_>, needed: usize) -> i32 {
        skb.callback_invocations += 1;
        skb.last_uncompress_needed = Some(needed);
        -EINVAL
    }

    fn udp_nhc() -> LowpanNhc {
        LowpanNhc {
            name: "RFC6282 UDP",
            nexthdr: UDP_NEXTHDR,
            nexthdrlen: 8,
            id: 0xf0,
            idmask: 0xf8,
            uncompress: Some(uncompress_ok),
            compress: Some(compress_ok),
        }
    }

    fn ipv6_nhc() -> LowpanNhc {
        LowpanNhc {
            name: "RFC6282 IPv6",
            nexthdr: IPV6_NEXTHDR,
            nexthdrlen: 40,
            id: 0xee,
            idmask: 0xfe,
            uncompress: Some(uncompress_ok),
            compress: None,
        }
    }

    #[test]
    fn lowpan_nhc_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/6lowpan/nhc.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/6lowpan/nhc.h"
        ));
        assert!(source.contains("lowpan_nexthdr_nhcs[NEXTHDR_MAX + 1]"));
        assert!(source.contains("DEFINE_SPINLOCK(lowpan_nhc_lock)"));
        assert!(source.contains("lowpan_nhc_by_nhcid"));
        assert!(source.contains("if (!pskb_may_pull(skb, 1))"));
        assert!(source.contains("if ((id & nhc->idmask) == nhc->id)"));
        assert!(source.contains("lowpan_nhc_check_compression"));
        assert!(source.contains("if (!(nhc && nhc->compress))"));
        assert!(source.contains("lowpan_nhc_do_compression"));
        assert!(source.contains("skb_set_transport_header(skb, sizeof(struct ipv6hdr))"));
        assert!(source.contains("skb_pull(skb, nhc->nexthdrlen)"));
        assert!(source.contains("lowpan_nhc_do_uncompression"));
        assert!(source.contains("return -ENOTSUPP;"));
        assert!(source.contains("hdr->nexthdr = nhc->nexthdr;"));
        assert!(source.contains("skb_reset_transport_header(skb)"));
        assert!(source.contains("lowpan_nhc_add"));
        assert!(source.contains("ret = -EEXIST;"));
        assert!(source.contains("EXPORT_SYMBOL(lowpan_nhc_add);"));
        assert!(source.contains("EXPORT_SYMBOL(lowpan_nhc_del);"));
        assert!(source.contains("synchronize_net();"));
        assert!(header.contains("struct lowpan_nhc"));
        assert!(header.contains("int\t\t(*uncompress)(struct sk_buff *skb, size_t needed);"));
        assert!(header.contains("int\t\t(*compress)(struct sk_buff *skb, u8 **hc_ptr);"));
    }

    #[test]
    fn registration_and_lookup_follow_linux_table_rules() {
        let udp = udp_nhc();
        let ipv6 = ipv6_nhc();
        let mut table = LowpanNhcTable::new();

        assert_eq!(lowpan_nhc_add(&mut table, udp), 0);
        assert_eq!(lowpan_nhc_add(&mut table, udp), -EEXIST);
        assert_eq!(lowpan_nhc_add(&mut table, ipv6), 0);
        assert_eq!(table.slot(UDP_NEXTHDR), Some(udp));

        let skb = LowpanSkb::new(&[0xf3]);
        assert_eq!(lowpan_nhc_by_nhcid(&table, &skb), Some(udp));
        assert_eq!(
            lowpan_nhc_by_nhcid(&table, &LowpanSkb::new(&[0xee])),
            Some(ipv6)
        );
        assert_eq!(lowpan_nhc_by_nhcid(&table, &LowpanSkb::new(&[0xaa])), None);
        assert_eq!(lowpan_nhc_by_nhcid(&table, &LowpanSkb::new(&[])), None);

        lowpan_nhc_del(&mut table, udp);
        assert_eq!(table.slot(UDP_NEXTHDR), None);
    }

    #[test]
    fn compression_paths_match_linux_return_contracts() {
        let udp = udp_nhc();
        let ipv6 = ipv6_nhc();
        let mut table = LowpanNhcTable::new();
        assert_eq!(lowpan_nhc_add(&mut table, udp), 0);
        assert_eq!(lowpan_nhc_add(&mut table, ipv6), 0);

        assert_eq!(
            lowpan_nhc_check_compression(
                &table,
                &Ipv6Header {
                    nexthdr: UDP_NEXTHDR
                }
            ),
            0
        );
        assert_eq!(
            lowpan_nhc_check_compression(
                &table,
                &Ipv6Header {
                    nexthdr: IPV6_NEXTHDR
                }
            ),
            -ENOENT
        );
        assert_eq!(
            lowpan_nhc_check_compression(&table, &Ipv6Header { nexthdr: 59 }),
            -ENOENT
        );

        let mut skb = LowpanSkb::new(&[0xf0, 0]);
        assert_eq!(
            lowpan_nhc_do_compression(
                &table,
                &mut skb,
                &Ipv6Header {
                    nexthdr: UDP_NEXTHDR
                }
            ),
            0
        );
        assert_eq!(skb.transport_header, IPV6HDR_SIZE);
        assert_eq!(skb.pulled, udp.nexthdrlen);
        assert_eq!(skb.callback_invocations, 1);

        let mut missing = LowpanSkb::new(&[0]);
        assert_eq!(
            lowpan_nhc_do_compression(&table, &mut missing, &Ipv6Header { nexthdr: 59 }),
            -EINVAL
        );
        assert_eq!(
            lowpan_nhc_do_compression(
                &table,
                &mut missing,
                &Ipv6Header {
                    nexthdr: IPV6_NEXTHDR
                }
            ),
            -EINVAL
        );

        let mut failing_table = LowpanNhcTable::new();
        let failing = LowpanNhc {
            compress: Some(compress_error),
            ..udp
        };
        assert_eq!(lowpan_nhc_add(&mut failing_table, failing), 0);
        let mut failing_skb = LowpanSkb::new(&[0xf0, 0]);
        assert_eq!(
            lowpan_nhc_do_compression(
                &failing_table,
                &mut failing_skb,
                &Ipv6Header {
                    nexthdr: UDP_NEXTHDR
                }
            ),
            -EINVAL
        );
        assert_eq!(failing_skb.pulled, 0);
        assert_eq!(failing_skb.callback_invocations, 1);
    }

    #[test]
    fn uncompression_paths_match_linux_return_contracts() {
        let udp = udp_nhc();
        let mut table = LowpanNhcTable::new();
        assert_eq!(lowpan_nhc_add(&mut table, udp), 0);

        let mut hdr = Ipv6Header { nexthdr: 0 };
        let mut skb = LowpanSkb::new(&[0xf3, 0]);
        assert_eq!(lowpan_nhc_do_uncompression(&table, &mut skb, &mut hdr), 0);
        assert_eq!(hdr.nexthdr, UDP_NEXTHDR);
        assert_eq!(skb.callback_invocations, 1);
        assert_eq!(
            skb.last_uncompress_needed,
            Some(IPV6HDR_SIZE + udp.nexthdrlen)
        );
        assert!(skb.transport_header_reset);

        let mut unknown = LowpanSkb::new(&[0xaa]);
        assert_eq!(
            lowpan_nhc_do_uncompression(&table, &mut unknown, &mut Ipv6Header { nexthdr: 0 }),
            -ENOENT
        );
        let mut empty = LowpanSkb::new(&[]);
        assert_eq!(
            lowpan_nhc_do_uncompression(&table, &mut empty, &mut Ipv6Header { nexthdr: 0 }),
            -ENOENT
        );

        let mut unsupported_table = LowpanNhcTable::new();
        assert_eq!(
            lowpan_nhc_add(
                &mut unsupported_table,
                LowpanNhc {
                    uncompress: None,
                    ..udp
                }
            ),
            0
        );
        let mut unsupported = LowpanSkb::new(&[0xf0]);
        assert_eq!(
            lowpan_nhc_do_uncompression(
                &unsupported_table,
                &mut unsupported,
                &mut Ipv6Header { nexthdr: 0 }
            ),
            -ENOTSUPP
        );

        let mut failing_table = LowpanNhcTable::new();
        assert_eq!(
            lowpan_nhc_add(
                &mut failing_table,
                LowpanNhc {
                    uncompress: Some(uncompress_error),
                    ..udp
                }
            ),
            0
        );
        let mut failing = LowpanSkb::new(&[0xf0]);
        assert_eq!(
            lowpan_nhc_do_uncompression(
                &failing_table,
                &mut failing,
                &mut Ipv6Header { nexthdr: 0 }
            ),
            -EINVAL
        );
        assert_eq!(failing.callback_invocations, 1);
        assert_eq!(
            failing.last_uncompress_needed,
            Some(IPV6HDR_SIZE + udp.nexthdrlen)
        );
    }
}
