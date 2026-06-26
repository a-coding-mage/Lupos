//! linux-parity: complete
//! linux-source: vendor/linux/net/tipc/eth_media.c
//! test-origin: linux:vendor/linux/net/tipc/eth_media.c
//! Ethernet bearer support for TIPC.

pub const ETH_ALEN: usize = 6;
pub const TIPC_MEDIA_INFO_SIZE: usize = 32;
pub const TIPC_MEDIA_TYPE_OFFSET: usize = 3;
pub const TIPC_MEDIA_ADDR_OFFSET: usize = 4;
pub const TIPC_MEDIA_TYPE_ETH: u8 = 1;
pub const TIPC_DEF_LINK_PRI: u32 = 10;
pub const TIPC_DEF_LINK_TOL: u32 = 1500;
pub const TIPC_DEF_LINK_WIN: u32 = 50;
pub const TIPC_MAX_LINK_WIN: u32 = 8191;

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

pub const ETH_MEDIA_INFO: TipcMedia = TipcMedia {
    priority: TIPC_DEF_LINK_PRI,
    tolerance: TIPC_DEF_LINK_TOL,
    min_win: TIPC_DEF_LINK_WIN,
    max_win: TIPC_MAX_LINK_WIN,
    type_id: TIPC_MEDIA_TYPE_ETH,
    hwaddr_len: ETH_ALEN,
    name: "eth",
};

pub fn tipc_eth_addr2str(addr: &TipcMediaAddr, strbuf: &mut [u8]) -> i32 {
    if strbuf.len() < 18 {
        return 1;
    }
    for i in 0..ETH_ALEN {
        let byte = addr.value[i];
        strbuf[i * 3] = hex(byte >> 4);
        strbuf[i * 3 + 1] = hex(byte & 0x0f);
        if i != ETH_ALEN - 1 {
            strbuf[i * 3 + 2] = b':';
        }
    }
    strbuf[17] = 0;
    0
}

pub fn tipc_eth_addr2msg(msg: &mut [u8; TIPC_MEDIA_INFO_SIZE], addr: &TipcMediaAddr) -> i32 {
    msg.fill(0);
    msg[TIPC_MEDIA_TYPE_OFFSET] = TIPC_MEDIA_TYPE_ETH;
    msg[TIPC_MEDIA_ADDR_OFFSET..TIPC_MEDIA_ADDR_OFFSET + ETH_ALEN]
        .copy_from_slice(&addr.value[..ETH_ALEN]);
    0
}

pub fn tipc_eth_raw2addr(raw: &[u8; ETH_ALEN]) -> TipcMediaAddr {
    let mut addr = TipcMediaAddr {
        value: [0; TIPC_MEDIA_INFO_SIZE],
        media_id: TIPC_MEDIA_TYPE_ETH,
        broadcast: raw.iter().all(|byte| *byte == 0xff),
    };
    addr.value[..ETH_ALEN].copy_from_slice(raw);
    addr
}

pub fn tipc_eth_msg2addr(msg: &[u8; TIPC_MEDIA_INFO_SIZE]) -> TipcMediaAddr {
    let mut raw = [0; ETH_ALEN];
    raw.copy_from_slice(&msg[TIPC_MEDIA_ADDR_OFFSET..TIPC_MEDIA_ADDR_OFFSET + ETH_ALEN]);
    tipc_eth_raw2addr(&raw)
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
    fn tipc_eth_media_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/tipc/eth_media.c"
        ));
        let bearer = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/tipc/bearer.h"
        ));
        assert!(bearer.contains("#define TIPC_MEDIA_INFO_SIZE\t32"));
        assert!(bearer.contains("#define TIPC_MEDIA_TYPE_OFFSET\t3"));
        assert!(bearer.contains("#define TIPC_MEDIA_ADDR_OFFSET\t4"));
        assert!(bearer.contains("#define TIPC_MEDIA_TYPE_ETH\t1"));
        assert!(source.contains("if (bufsz < 18)"));
        assert!(source.contains("sprintf(strbuf, \"%pM\", addr->value);"));
        assert!(source.contains("memset(msg, 0, TIPC_MEDIA_INFO_SIZE);"));
        assert!(source.contains("msg[TIPC_MEDIA_TYPE_OFFSET] = TIPC_MEDIA_TYPE_ETH;"));
        assert!(source.contains("memcpy(msg + TIPC_MEDIA_ADDR_OFFSET, addr->value, ETH_ALEN);"));
        assert!(source.contains("ether_addr_copy(addr->value, msg);"));
        assert!(source.contains("addr->media_id = TIPC_MEDIA_TYPE_ETH;"));
        assert!(source.contains("addr->broadcast = is_broadcast_ether_addr(addr->value);"));
        assert!(source.contains("msg += TIPC_MEDIA_ADDR_OFFSET;"));
        assert!(source.contains(".priority\t= TIPC_DEF_LINK_PRI"));
        assert!(source.contains(".tolerance\t= TIPC_DEF_LINK_TOL"));
        assert!(source.contains(".min_win\t= TIPC_DEF_LINK_WIN"));
        assert!(source.contains(".max_win\t= TIPC_MAX_LINK_WIN"));
        assert!(source.contains(".type_id\t= TIPC_MEDIA_TYPE_ETH"));
        assert!(source.contains(".hwaddr_len\t= ETH_ALEN"));
        assert!(source.contains(".name\t\t= \"eth\""));
    }

    #[test]
    fn eth_media_converts_between_raw_string_and_discovery_message() {
        let raw = [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];
        let addr = tipc_eth_raw2addr(&raw);
        assert_eq!(addr.media_id, TIPC_MEDIA_TYPE_ETH);
        assert!(!addr.broadcast);
        let mut text = [0; 18];
        assert_eq!(tipc_eth_addr2str(&addr, &mut text), 0);
        assert_eq!(&text[..17], b"aa:bb:cc:dd:ee:ff");
        assert_eq!(tipc_eth_addr2str(&addr, &mut [0; 17]), 1);

        let mut msg = [0xff; TIPC_MEDIA_INFO_SIZE];
        assert_eq!(tipc_eth_addr2msg(&mut msg, &addr), 0);
        assert_eq!(msg[TIPC_MEDIA_TYPE_OFFSET], TIPC_MEDIA_TYPE_ETH);
        assert_eq!(
            &msg[TIPC_MEDIA_ADDR_OFFSET..TIPC_MEDIA_ADDR_OFFSET + ETH_ALEN],
            &raw
        );
        assert!(msg[..TIPC_MEDIA_TYPE_OFFSET].iter().all(|byte| *byte == 0));
        assert_eq!(tipc_eth_msg2addr(&msg), addr);
        assert!(tipc_eth_raw2addr(&[0xff; ETH_ALEN]).broadcast);
        assert_eq!(
            ETH_MEDIA_INFO,
            TipcMedia {
                priority: TIPC_DEF_LINK_PRI,
                tolerance: TIPC_DEF_LINK_TOL,
                min_win: TIPC_DEF_LINK_WIN,
                max_win: TIPC_MAX_LINK_WIN,
                type_id: TIPC_MEDIA_TYPE_ETH,
                hwaddr_len: ETH_ALEN,
                name: "eth",
            }
        );
    }
}
