//! linux-parity: complete
//! linux-source: vendor/linux/net/rxrpc/insecure.c
//! test-origin: linux:vendor/linux/net/rxrpc/insecure.c
//! RxRPC null security operations.

use crate::include::uapi::errno::{EINVAL, EPROTO};

pub const RXRPC_JUMBO_DATALEN: usize = 1412;
pub const RXRPC_SECURITY_NONE: u8 = 0;
pub const RX_PROTOCOL_ERROR: i32 = -5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RxrpcTxbuf {
    pub len: usize,
    pub pkt_len: usize,
    pub jumboable: bool,
    pub nr_subpackets: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RxrpcAbort {
    pub abort_code: i32,
    pub error: i32,
    pub why: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RxrpcSecurity {
    pub name: &'static str,
    pub security_index: u8,
}

pub const RXRPC_NO_SECURITY: RxrpcSecurity = RxrpcSecurity {
    name: "none",
    security_index: RXRPC_SECURITY_NONE,
};

pub const fn none_init_connection_security() -> i32 {
    0
}

pub const fn none_alloc_txbuf(remain: usize) -> RxrpcTxbuf {
    RxrpcTxbuf {
        len: if remain < RXRPC_JUMBO_DATALEN {
            remain
        } else {
            RXRPC_JUMBO_DATALEN
        },
        pkt_len: 0,
        jumboable: false,
        nr_subpackets: 1,
    }
}

pub const fn none_secure_packet(mut txb: RxrpcTxbuf) -> Result<RxrpcTxbuf, i32> {
    txb.pkt_len = txb.len;
    if txb.len == RXRPC_JUMBO_DATALEN {
        txb.jumboable = true;
    }
    Ok(txb)
}

pub const fn none_verify_packet() -> i32 {
    0
}

pub const fn none_validate_challenge() -> (bool, RxrpcAbort) {
    (
        true,
        RxrpcAbort {
            abort_code: RX_PROTOCOL_ERROR,
            error: -EPROTO,
            why: "rxrpc_eproto_rxnull_challenge",
        },
    )
}

pub const fn none_sendmsg_respond_to_challenge() -> Result<(), i32> {
    Err(-EINVAL)
}

pub const fn none_verify_response() -> RxrpcAbort {
    RxrpcAbort {
        abort_code: RX_PROTOCOL_ERROR,
        error: -EPROTO,
        why: "rxrpc_eproto_rxnull_response",
    }
}

pub const fn none_init() -> i32 {
    0
}

pub const fn none_exit() {}

pub const fn rxrpc_no_security() -> &'static RxrpcSecurity {
    &RXRPC_NO_SECURITY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rxrpc_insecure_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/rxrpc/insecure.c"
        ));
        assert!(source.contains("Null security operations."));
        assert!(source.contains("none_init_connection_security"));
        assert!(source.contains("return 0;"));
        assert!(source.contains("none_alloc_txbuf"));
        assert!(
            source.contains(
                "rxrpc_alloc_data_txbuf(call, umin(remain, RXRPC_JUMBO_DATALEN), 1, gfp);"
            )
        );
        assert!(source.contains("none_secure_packet"));
        assert!(source.contains("txb->pkt_len = txb->len;"));
        assert!(source.contains("if (txb->len == RXRPC_JUMBO_DATALEN)"));
        assert!(source.contains("txb->jumboable = true;"));
        assert!(source.contains("none_verify_packet"));
        assert!(source.contains("none_validate_challenge"));
        assert!(source.contains("rxrpc_eproto_rxnull_challenge"));
        assert!(source.contains("none_sendmsg_respond_to_challenge"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("none_verify_response"));
        assert!(source.contains("rxrpc_eproto_rxnull_response"));
        assert!(source.contains("const struct rxrpc_security rxrpc_no_security"));
        assert!(source.contains(".name\t\t\t\t= \"none\""));
        assert!(source.contains(".security_index\t\t\t= RXRPC_SECURITY_NONE"));
        assert!(source.contains(".alloc_txbuf\t\t\t= none_alloc_txbuf"));
        assert!(source.contains(".validate_challenge\t\t= none_validate_challenge"));
    }

    #[test]
    fn null_security_allocates_plain_jumboable_txbufs_and_aborts_challenges() {
        assert_eq!(none_init_connection_security(), 0);
        assert_eq!(none_verify_packet(), 0);
        let small = none_secure_packet(none_alloc_txbuf(10)).unwrap();
        assert_eq!(small.len, 10);
        assert_eq!(small.pkt_len, 10);
        assert!(!small.jumboable);
        let jumbo = none_secure_packet(none_alloc_txbuf(usize::MAX)).unwrap();
        assert_eq!(jumbo.len, RXRPC_JUMBO_DATALEN);
        assert!(jumbo.jumboable);
        let (valid, abort) = none_validate_challenge();
        assert!(valid);
        assert_eq!(abort.error, -EPROTO);
        assert_eq!(none_sendmsg_respond_to_challenge(), Err(-EINVAL));
        assert_eq!(none_verify_response().why, "rxrpc_eproto_rxnull_response");
        assert_eq!(rxrpc_no_security(), &RXRPC_NO_SECURITY);
    }
}
