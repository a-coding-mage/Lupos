//! linux-parity: complete
//! linux-source: vendor/linux/net/tipc/ib_media.c
//! test-origin: linux:vendor/linux/net/tipc/ib_media.c
//! InfiniBand bearer media address conversion for TIPC.

use super::eth_media::{TIPC_DEF_LINK_PRI, TIPC_DEF_LINK_TOL, TIPC_DEF_LINK_WIN};

pub const INFINIBAND_ALEN: usize = 20;
pub const TIPC_MEDIA_INFO_SIZE: usize = 32;
pub const TIPC_MEDIA_TYPE_IB: u8 = 2;
pub const TIPC_MAX_IB_LINK_WIN: u32 = 500;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TipcMediaAddr {
    pub value: [u8; TIPC_MEDIA_INFO_SIZE],
    pub media_id: u8,
    pub broadcast: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TipcMedia {
    pub priority: u32,
    pub tolerance: u32,
    pub min_win: u32,
    pub max_win: u32,
    pub type_id: u8,
    pub hwaddr_len: usize,
    pub name: &'static str,
}

pub const IB_MEDIA_INFO: TipcMedia = TipcMedia {
    priority: TIPC_DEF_LINK_PRI,
    tolerance: TIPC_DEF_LINK_TOL,
    min_win: TIPC_DEF_LINK_WIN,
    max_win: TIPC_MAX_IB_LINK_WIN,
    type_id: TIPC_MEDIA_TYPE_IB,
    hwaddr_len: INFINIBAND_ALEN,
    name: "ib",
};

pub fn tipc_ib_addr2str(addr: &TipcMediaAddr, str_buf: &mut [u8]) -> i32 {
    if str_buf.len() < 60 {
        return 1;
    }
    for i in 0..INFINIBAND_ALEN {
        let byte = addr.value[i];
        str_buf[i * 3] = hex(byte >> 4);
        str_buf[i * 3 + 1] = hex(byte & 0x0f);
        if i != INFINIBAND_ALEN - 1 {
            str_buf[i * 3 + 2] = b':';
        }
    }
    str_buf[59] = 0;
    0
}

pub fn tipc_ib_addr2msg(msg: &mut [u8; TIPC_MEDIA_INFO_SIZE], addr: &TipcMediaAddr) -> i32 {
    msg.fill(0);
    msg[..INFINIBAND_ALEN].copy_from_slice(&addr.value[..INFINIBAND_ALEN]);
    0
}

pub fn tipc_ib_raw2addr(
    bcast_addr: &[u8; INFINIBAND_ALEN],
    raw: &[u8; INFINIBAND_ALEN],
) -> TipcMediaAddr {
    let mut addr = TipcMediaAddr {
        value: [0; TIPC_MEDIA_INFO_SIZE],
        media_id: TIPC_MEDIA_TYPE_IB,
        broadcast: raw == bcast_addr,
    };
    addr.value[..INFINIBAND_ALEN].copy_from_slice(raw);
    addr
}

pub fn tipc_ib_msg2addr(
    bcast_addr: &[u8; INFINIBAND_ALEN],
    msg: &[u8; TIPC_MEDIA_INFO_SIZE],
) -> TipcMediaAddr {
    let mut raw = [0; INFINIBAND_ALEN];
    raw.copy_from_slice(&msg[..INFINIBAND_ALEN]);
    tipc_ib_raw2addr(bcast_addr, &raw)
}

const fn hex(nibble: u8) -> u8 {
    match nibble {
        0..=9 => b'0' + nibble,
        _ => b'a' + (nibble - 10),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tipc_ib_media_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/tipc/ib_media.c"
        ));
        assert!(source.contains("#define TIPC_MAX_IB_LINK_WIN 500"));
        assert!(source.contains("if (str_size < 60)"));
        assert!(source.contains("sprintf(str_buf, \"%20phC\", a->value);"));
        assert!(source.contains("memset(msg, 0, TIPC_MEDIA_INFO_SIZE);"));
        assert!(source.contains("memcpy(msg, addr->value, INFINIBAND_ALEN);"));
        assert!(source.contains("memset(addr, 0, sizeof(*addr));"));
        assert!(source.contains("memcpy(addr->value, msg, INFINIBAND_ALEN);"));
        assert!(source.contains("addr->media_id = TIPC_MEDIA_TYPE_IB;"));
        assert!(source.contains("addr->broadcast = !memcmp(msg, b->bcast_addr.value"));
        assert!(source.contains("return tipc_ib_raw2addr(b, addr, msg);"));
        assert!(source.contains(".max_win\t= TIPC_MAX_IB_LINK_WIN"));
        assert!(source.contains(".type_id\t= TIPC_MEDIA_TYPE_IB"));
        assert!(source.contains(".hwaddr_len\t= INFINIBAND_ALEN"));
        assert!(source.contains(".name\t\t= \"ib\""));
    }

    #[test]
    fn ib_media_converts_between_raw_string_and_discovery_message() {
        let bcast = [0xff; INFINIBAND_ALEN];
        let raw = [0x12; INFINIBAND_ALEN];
        let addr = tipc_ib_raw2addr(&bcast, &raw);
        assert_eq!(addr.media_id, TIPC_MEDIA_TYPE_IB);
        assert!(!addr.broadcast);
        assert!(tipc_ib_raw2addr(&bcast, &bcast).broadcast);

        let mut text = [0; 60];
        assert_eq!(tipc_ib_addr2str(&addr, &mut text), 0);
        assert_eq!(&text[..5], b"12:12");
        assert_eq!(tipc_ib_addr2str(&addr, &mut [0; 59]), 1);

        let mut msg = [0xff; TIPC_MEDIA_INFO_SIZE];
        assert_eq!(tipc_ib_addr2msg(&mut msg, &addr), 0);
        assert_eq!(&msg[..INFINIBAND_ALEN], &raw);
        assert!(msg[INFINIBAND_ALEN..].iter().all(|byte| *byte == 0));
        assert_eq!(tipc_ib_msg2addr(&bcast, &msg), addr);
        assert_eq!(IB_MEDIA_INFO.name, "ib");
    }
}
