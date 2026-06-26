//! linux-parity: complete
//! linux-source: vendor/linux/net/rxrpc/misc.c
//! test-origin: linux:vendor/linux/net/rxrpc/misc.c
//! RxRPC tunable defaults.

pub const RXRPC_WIRE_HEADER_SIZE: usize = 28;
pub const RXRPC_JUMBO_HEADER_SIZE: usize = 4;
pub const RXRPC_JUMBO_DATALEN: usize = 1412;
pub const RXRPC_JUMBO_SUBPKTLEN: usize = RXRPC_JUMBO_DATALEN + RXRPC_JUMBO_HEADER_SIZE;

pub const RXRPC_MAX_BACKLOG: u32 = 10;
pub const RXRPC_SOFT_ACK_DELAY_MS: u64 = 1000;
pub const RXRPC_IDLE_ACK_DELAY_MS: u64 = 500;
pub const RXRPC_RX_WINDOW_SIZE: u32 = 255;
pub const RXRPC_RX_JUMBO_MAX: u32 = 46;
pub const RXRPC_RX_MTU: usize = rxrpc_jumbo(RXRPC_RX_JUMBO_MAX as usize);

pub const fn rxrpc_jumbo(subpackets: usize) -> usize {
    RXRPC_WIRE_HEADER_SIZE + RXRPC_JUMBO_DATALEN + (subpackets - 1) * RXRPC_JUMBO_SUBPKTLEN
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RxrpcTunableDefaults {
    pub max_backlog: u32,
    pub soft_ack_delay_ms: u64,
    pub idle_ack_delay_ms: u64,
    pub rx_window_size: u32,
    pub rx_mtu: usize,
    pub rx_jumbo_max: u32,
}

pub const DEFAULTS: RxrpcTunableDefaults = RxrpcTunableDefaults {
    max_backlog: RXRPC_MAX_BACKLOG,
    soft_ack_delay_ms: RXRPC_SOFT_ACK_DELAY_MS,
    idle_ack_delay_ms: RXRPC_IDLE_ACK_DELAY_MS,
    rx_window_size: RXRPC_RX_WINDOW_SIZE,
    rx_mtu: RXRPC_RX_MTU,
    rx_jumbo_max: RXRPC_RX_JUMBO_MAX,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rxrpc_misc_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/rxrpc/misc.c"
        ));
        assert!(source.contains("unsigned int rxrpc_max_backlog __read_mostly = 10;"));
        assert!(source.contains("unsigned long rxrpc_soft_ack_delay = 1000;"));
        assert!(source.contains("unsigned long rxrpc_idle_ack_delay = 500;"));
        assert!(source.contains("unsigned int rxrpc_rx_window_size = 255;"));
        assert!(source.contains("unsigned int rxrpc_rx_mtu = RXRPC_JUMBO(46);"));
        assert!(source.contains("unsigned int rxrpc_rx_jumbo_max = 46;"));

        assert_eq!(rxrpc_jumbo(1), 1440);
        assert_eq!(RXRPC_RX_MTU, 65160);
        assert_eq!(
            DEFAULTS,
            RxrpcTunableDefaults {
                max_backlog: 10,
                soft_ack_delay_ms: 1000,
                idle_ack_delay_ms: 500,
                rx_window_size: 255,
                rx_mtu: 65160,
                rx_jumbo_max: 46,
            }
        );
    }
}
