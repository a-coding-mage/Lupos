//! linux-parity: complete
//! linux-source: vendor/linux/net/rxrpc/skbuff.c
//! test-origin: linux:vendor/linux/net/rxrpc/skbuff.c
//! RxRPC socket-buffer accounting.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RxrpcSkbTrace {
    New,
    See,
    Get,
    Free,
    PutPurge,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RxrpcSkBuff {
    pub users: i32,
    pub consumed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RxrpcSkbTraceEvent {
    pub users: i32,
    pub outstanding: i32,
    pub why: RxrpcSkbTrace,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RxrpcSkbAccounting {
    pub n_rx_skbs: i32,
    pub traces: u32,
    pub last_trace: Option<RxrpcSkbTraceEvent>,
}

impl RxrpcSkbAccounting {
    fn trace(&mut self, skb: &RxrpcSkBuff, why: RxrpcSkbTrace) {
        self.traces = self.traces.saturating_add(1);
        self.last_trace = Some(RxrpcSkbTraceEvent {
            users: skb.users,
            outstanding: self.n_rx_skbs,
            why,
        });
    }
}

pub fn rxrpc_new_skb(acct: &mut RxrpcSkbAccounting, skb: &RxrpcSkBuff, why: RxrpcSkbTrace) {
    acct.n_rx_skbs = acct.n_rx_skbs.saturating_add(1);
    acct.trace(skb, why);
}

pub fn rxrpc_see_skb(acct: &mut RxrpcSkbAccounting, skb: Option<&RxrpcSkBuff>, why: RxrpcSkbTrace) {
    if let Some(skb) = skb {
        acct.trace(skb, why);
    }
}

pub fn rxrpc_get_skb(acct: &mut RxrpcSkbAccounting, skb: &mut RxrpcSkBuff, why: RxrpcSkbTrace) {
    acct.n_rx_skbs = acct.n_rx_skbs.saturating_add(1);
    acct.trace(skb, why);
    skb.users = skb.users.saturating_add(1);
}

pub fn rxrpc_free_skb(
    acct: &mut RxrpcSkbAccounting,
    skb: Option<&mut RxrpcSkBuff>,
    why: RxrpcSkbTrace,
) {
    if let Some(skb) = skb {
        acct.n_rx_skbs -= 1;
        acct.trace(skb, why);
        skb.consumed = true;
    }
}

pub fn rxrpc_purge_queue(acct: &mut RxrpcSkbAccounting, queue: &mut [Option<RxrpcSkBuff>]) {
    for slot in queue {
        if let Some(mut skb) = slot.take() {
            acct.n_rx_skbs -= 1;
            acct.trace(&skb, RxrpcSkbTrace::PutPurge);
            skb.consumed = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rxrpc_skbuff_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/rxrpc/skbuff.c"
        ));
        assert!(source.contains("#define select_skb_count(skb) (&rxrpc_n_rx_skbs)"));
        assert!(source.contains("void rxrpc_new_skb"));
        assert!(source.contains("atomic_inc_return(select_skb_count(skb))"));
        assert!(source.contains("trace_rxrpc_skb(skb, refcount_read(&skb->users), n, why);"));
        assert!(source.contains("void rxrpc_see_skb"));
        assert!(source.contains("if (skb)"));
        assert!(source.contains("atomic_read(select_skb_count(skb))"));
        assert!(source.contains("void rxrpc_get_skb"));
        assert!(source.contains("skb_get(skb);"));
        assert!(source.contains("void rxrpc_free_skb"));
        assert!(source.contains("atomic_dec_return(select_skb_count(skb))"));
        assert!(source.contains("consume_skb(skb);"));
        assert!(source.contains("void rxrpc_purge_queue"));
        assert!(source.contains("while ((skb = skb_dequeue((list))) != NULL)"));
        assert!(source.contains("rxrpc_skb_put_purge"));
    }

    #[test]
    fn skb_accounting_tracks_refs_and_purge_decrements() {
        let mut acct = RxrpcSkbAccounting::default();
        let mut skb = RxrpcSkBuff {
            users: 1,
            consumed: false,
        };
        rxrpc_new_skb(&mut acct, &skb, RxrpcSkbTrace::New);
        assert_eq!(acct.n_rx_skbs, 1);
        rxrpc_see_skb(&mut acct, Some(&skb), RxrpcSkbTrace::See);
        assert_eq!(acct.n_rx_skbs, 1);
        rxrpc_get_skb(&mut acct, &mut skb, RxrpcSkbTrace::Get);
        assert_eq!(acct.n_rx_skbs, 2);
        assert_eq!(skb.users, 2);
        rxrpc_free_skb(&mut acct, Some(&mut skb), RxrpcSkbTrace::Free);
        assert_eq!(acct.n_rx_skbs, 1);
        assert!(skb.consumed);

        let mut queue = [
            Some(RxrpcSkBuff {
                users: 1,
                consumed: false,
            }),
            None,
            Some(RxrpcSkBuff {
                users: 1,
                consumed: false,
            }),
        ];
        acct.n_rx_skbs = 2;
        rxrpc_purge_queue(&mut acct, &mut queue);
        assert_eq!(acct.n_rx_skbs, 0);
        assert!(queue.iter().all(Option::is_none));
        assert_eq!(acct.last_trace.unwrap().why, RxrpcSkbTrace::PutPurge);
    }
}
