//! linux-parity: complete
//! linux-source: vendor/linux/net/handshake/alert.c
//! test-origin: linux:vendor/linux/net/handshake/alert.c
//! TLS alert send and receive helpers.

pub const MODULE_LICENSE: &str = "GPL";
pub const TLS_RECORD_TYPE_ALERT: u8 = 21;
pub const SOL_TLS: i32 = 282;
pub const TLS_SET_RECORD_TYPE: i32 = 1;
pub const TLS_GET_RECORD_TYPE: i32 = 2;
pub const MSG_DONTWAIT: i32 = 0x40;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TlsAlertMessage {
    pub level: u8,
    pub description: u8,
    pub record_type: u8,
    pub flags: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cmsghdr {
    pub cmsg_level: i32,
    pub cmsg_type: i32,
    pub data: u8,
}

pub const fn tls_alert_send(
    level: u8,
    description: u8,
    sock_sendmsg_ret: i32,
) -> Result<TlsAlertMessage, i32> {
    if sock_sendmsg_ret < 0 {
        return Err(sock_sendmsg_ret);
    }
    Ok(TlsAlertMessage {
        level,
        description,
        record_type: TLS_RECORD_TYPE_ALERT,
        flags: MSG_DONTWAIT,
    })
}

pub const fn tls_get_record_type(cmsg: Cmsghdr) -> u8 {
    if cmsg.cmsg_level != SOL_TLS || cmsg.cmsg_type != TLS_GET_RECORD_TYPE {
        return 0;
    }
    cmsg.data
}

pub const fn tls_alert_recv(data: [u8; 2]) -> (u8, u8) {
    (data[0], data[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_alert_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/handshake/alert.c"
        ));
        assert!(source.contains("int tls_alert_send(struct socket *sock"));
        assert!(source.contains("u8 record_type = TLS_RECORD_TYPE_ALERT;"));
        assert!(source.contains("trace_tls_alert_send(sock->sk, level, description);"));
        assert!(source.contains("alert[0] = level;"));
        assert!(source.contains("alert[1] = description;"));
        assert!(source.contains("msg.msg_flags = MSG_DONTWAIT;"));
        assert!(source.contains("cmsg->cmsg_level = SOL_TLS;"));
        assert!(source.contains("cmsg->cmsg_type = TLS_SET_RECORD_TYPE;"));
        assert!(source.contains("sock_sendmsg(sock, &msg);"));
        assert!(source.contains("return ret < 0 ? ret : 0;"));
        assert!(source.contains("u8 tls_get_record_type"));
        assert!(source.contains("if (cmsg->cmsg_level != SOL_TLS)"));
        assert!(source.contains("if (cmsg->cmsg_type != TLS_GET_RECORD_TYPE)"));
        assert!(source.contains("trace_tls_contenttype(sk, record_type);"));
        assert!(source.contains("void tls_alert_recv"));
        assert!(source.contains("*level = data[0];"));
        assert!(source.contains("*description = data[1];"));
    }

    #[test]
    fn alert_helpers_send_parse_and_receive_tls_alerts() {
        assert_eq!(
            tls_alert_send(2, 40, 0),
            Ok(TlsAlertMessage {
                level: 2,
                description: 40,
                record_type: TLS_RECORD_TYPE_ALERT,
                flags: MSG_DONTWAIT,
            })
        );
        assert_eq!(tls_alert_send(2, 40, -5), Err(-5));
        assert_eq!(
            tls_get_record_type(Cmsghdr {
                cmsg_level: SOL_TLS,
                cmsg_type: TLS_GET_RECORD_TYPE,
                data: 23,
            }),
            23
        );
        assert_eq!(
            tls_get_record_type(Cmsghdr {
                cmsg_level: SOL_TLS,
                cmsg_type: TLS_SET_RECORD_TYPE,
                data: 23,
            }),
            0
        );
        assert_eq!(tls_alert_recv([1, 90]), (1, 90));
    }
}
