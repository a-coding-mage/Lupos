//! linux-parity: complete
//! linux-source: vendor/linux/net/rxrpc/local_event.c
//! test-origin: linux:vendor/linux/net/rxrpc/local_event.c
//! RxRPC local VERSION packet construction.

pub const RXRPC_VERSION_STRING_SIZE: usize = 65;
pub const RXRPC_VERSION_PREFIX: &[u8] = b"linux-";
pub const RXRPC_VERSION_SUFFIX: &[u8] = b" AF_RXRPC";
pub const RXRPC_UTS_RELEASE_MAX: usize = 49;
pub const RXRPC_PACKET_TYPE_VERSION: u8 = 13;
pub const RXRPC_CLIENT_INITIATED: u8 = 0x01;
pub const RXRPC_LAST_PACKET: u8 = 0x04;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RxrpcHostHeader {
    pub flags: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RxrpcSkbHeader {
    pub epoch: u32,
    pub cid: u32,
    pub call_number: u32,
    pub flags: u8,
    pub service_id: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RxrpcWireHeader {
    pub epoch: u32,
    pub cid: u32,
    pub call_number: u32,
    pub seq: u32,
    pub serial: u32,
    pub packet_type: u8,
    pub flags: u8,
    pub user_status: u8,
    pub security_index: u8,
    pub reserved: u16,
    pub service_id: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VersionReply {
    pub header: RxrpcWireHeader,
    pub version_string: [u8; RXRPC_VERSION_STRING_SIZE],
    pub iov_count: usize,
    pub len: usize,
}

pub fn rxrpc_gen_version_string(uts_release: &str) -> [u8; RXRPC_VERSION_STRING_SIZE] {
    let mut out = [0u8; RXRPC_VERSION_STRING_SIZE];
    let mut pos = 0;
    for byte in RXRPC_VERSION_PREFIX {
        out[pos] = *byte;
        pos += 1;
    }
    for byte in uts_release.bytes().take(RXRPC_UTS_RELEASE_MAX) {
        out[pos] = byte;
        pos += 1;
    }
    for byte in RXRPC_VERSION_SUFFIX {
        out[pos] = *byte;
        pos += 1;
    }
    out
}

pub fn rxrpc_send_version_request(
    hdr: RxrpcHostHeader,
    skb_hdr: RxrpcSkbHeader,
    uts_release: &str,
) -> VersionReply {
    let header = RxrpcWireHeader {
        epoch: skb_hdr.epoch.to_be(),
        cid: skb_hdr.cid.to_be(),
        call_number: skb_hdr.call_number.to_be(),
        seq: 0,
        serial: 0,
        packet_type: RXRPC_PACKET_TYPE_VERSION,
        flags: RXRPC_LAST_PACKET | ((!hdr.flags) & RXRPC_CLIENT_INITIATED),
        user_status: 0,
        security_index: 0,
        reserved: 0,
        service_id: skb_hdr.service_id.to_be(),
    };
    VersionReply {
        header,
        version_string: rxrpc_gen_version_string(uts_release),
        iov_count: 2,
        len: core::mem::size_of::<RxrpcWireHeader>() + RXRPC_VERSION_STRING_SIZE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rxrpc_local_event_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/rxrpc/local_event.c"
        ));
        assert!(source.contains("static char rxrpc_version_string[65];"));
        assert!(source.contains("void rxrpc_gen_version_string(void)"));
        assert!(source.contains("\"linux-%.49s AF_RXRPC\""));
        assert!(source.contains("void rxrpc_send_version_request"));
        assert!(source.contains("rxrpc_extract_addr_from_skb(&srx, skb) < 0"));
        assert!(source.contains("whdr.epoch\t= htonl(sp->hdr.epoch);"));
        assert!(source.contains("whdr.cid\t= htonl(sp->hdr.cid);"));
        assert!(source.contains("whdr.callNumber\t= htonl(sp->hdr.callNumber);"));
        assert!(source.contains("whdr.type\t= RXRPC_PACKET_TYPE_VERSION;"));
        assert!(
            source.contains(
                "whdr.flags\t= RXRPC_LAST_PACKET | (~hdr->flags & RXRPC_CLIENT_INITIATED);"
            )
        );
        assert!(source.contains("whdr.serviceId\t= htons(sp->hdr.serviceId);"));
        assert!(source.contains("iov[0].iov_base\t= &whdr;"));
        assert!(source.contains("iov[1].iov_base\t= (char *)rxrpc_version_string;"));
        assert!(source.contains("kernel_sendmsg(local->socket, &msg, iov, 2, len);"));

        let version =
            rxrpc_gen_version_string("12345678901234567890123456789012345678901234567890-extra");
        assert_eq!(&version[..6], b"linux-");
        assert_eq!(
            &version[6..55],
            b"1234567890123456789012345678901234567890123456789"
        );
        assert_eq!(&version[55..64], b" AF_RXRPC");
        assert_eq!(version[64], 0);

        let reply = rxrpc_send_version_request(
            RxrpcHostHeader {
                flags: RXRPC_CLIENT_INITIATED,
            },
            RxrpcSkbHeader {
                epoch: 1,
                cid: 2,
                call_number: 3,
                flags: 0,
                service_id: 7000,
            },
            "test",
        );
        assert_eq!(reply.header.epoch, 1u32.to_be());
        assert_eq!(reply.header.flags, RXRPC_LAST_PACKET);
        assert_eq!(reply.header.packet_type, RXRPC_PACKET_TYPE_VERSION);
        assert_eq!(reply.header.service_id, 7000u16.to_be());
        assert_eq!(reply.iov_count, 2);
    }
}
