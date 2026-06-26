//! linux-parity: complete
//! linux-source: vendor/linux/net/sched/em_u32.c
//! test-origin: linux:vendor/linux/net/sched/em_u32.c
//! U32 traffic-control ematch.

pub const TCF_EM_U32: u16 = 3;
pub const MODULE_DESCRIPTION: &str = "ematch skb classifier using 32 bit chunks of data";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TcU32Key {
    pub val: u32,
    pub mask: u32,
    pub off: usize,
    pub offmask: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TcfPktInfo {
    pub ptr: Option<usize>,
    pub nexthdr: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TcfEmatchOps {
    pub kind: u16,
    pub datalen: usize,
}

pub const EM_U32_OPS: TcfEmatchOps = TcfEmatchOps {
    kind: TCF_EM_U32,
    datalen: core::mem::size_of::<TcU32Key>(),
};

pub fn em_u32_match(
    packet: &[u8],
    network_header: usize,
    key: TcU32Key,
    info: Option<TcfPktInfo>,
) -> bool {
    let mut ptr = network_header;
    if let Some(info) = info {
        if let Some(info_ptr) = info.ptr {
            ptr = info_ptr;
        }
        ptr = ptr.saturating_add((info.nexthdr & key.offmask) as usize);
    }
    ptr = ptr.saturating_add(key.off);

    let Some(word) = packet.get(ptr..ptr.saturating_add(4)) else {
        return false;
    };
    let word = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
    ((word ^ key.val) & key.mask) == 0
}

pub const fn init_em_u32() -> &'static TcfEmatchOps {
    &EM_U32_OPS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn em_u32_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/sched/em_u32.c"
        ));
        assert!(source.contains("static int em_u32_match"));
        assert!(source.contains("struct tc_u32_key *key"));
        assert!(source.contains("skb_network_header(skb)"));
        assert!(source.contains("ptr += (info->nexthdr & key->offmask);"));
        assert!(source.contains("ptr += key->off;"));
        assert!(source.contains("tcf_valid_offset(skb, ptr, sizeof(u32))"));
        assert!(source.contains("return !(((*(__be32 *) ptr)  ^ key->val) & key->mask);"));
        assert!(source.contains(".kind\t  = TCF_EM_U32"));
        assert!(source.contains("MODULE_ALIAS_TCF_EMATCH(TCF_EM_U32);"));

        let packet = [0, 1, 2, 3, 0xde, 0xad, 0xbe, 0xef, 9, 10];
        let key = TcU32Key {
            val: 0xdead_beef,
            mask: 0xffff_ffff,
            off: 4,
            offmask: 0,
        };
        assert!(em_u32_match(&packet, 0, key, None));
        assert!(!em_u32_match(&packet, 0, TcU32Key { val: 0, ..key }, None));
        assert!(em_u32_match(
            &packet,
            0,
            TcU32Key {
                off: 0,
                offmask: 0x0f,
                ..key
            },
            Some(TcfPktInfo {
                ptr: Some(0),
                nexthdr: 4,
            }),
        ));
        assert!(!em_u32_match(&packet[..6], 0, key, None));
        assert_eq!(init_em_u32(), &EM_U32_OPS);
    }
}
