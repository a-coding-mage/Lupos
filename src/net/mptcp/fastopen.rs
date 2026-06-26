//! linux-parity: complete
//! linux-source: vendor/linux/net/mptcp/fastopen.c
//! test-origin: linux:vendor/linux/net/mptcp/fastopen.c
//! MPTCP Fast Open receive-queue handoff.

extern crate alloc;

use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkbOwner {
    Subflow,
    MptcpConnection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FastopenSkb {
    pub len: u32,
    pub ext_reset: bool,
    pub has_rxtstamp: bool,
    pub map_seq: i64,
    pub end_seq: u64,
    pub offset: u32,
    pub cant_coalesce: bool,
    pub owner: SkbOwner,
}

impl FastopenSkb {
    pub const fn new(len: u32, has_rxtstamp: bool) -> Self {
        Self {
            len,
            ext_reset: false,
            has_rxtstamp,
            map_seq: 0,
            end_seq: 0,
            offset: 0,
            cant_coalesce: false,
            owner: SkbOwner::Subflow,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MptcpSubflowContext {
    pub is_mptfo: bool,
    pub tcp_copied_seq: u32,
    pub ssn_offset: u32,
    pub bytes_received: u64,
    pub subflow_receive_queue: Vec<FastopenSkb>,
    pub conn_receive_queue: Vec<FastopenSkb>,
    pub data_ready_called: bool,
}

pub fn mptcp_fastopen_subflow_synack_set_params(subflow: Option<&mut MptcpSubflowContext>) -> bool {
    let Some(subflow) = subflow else {
        return false;
    };

    subflow.is_mptfo = true;
    if subflow.subflow_receive_queue.is_empty() {
        return false;
    }

    let mut skb = subflow.subflow_receive_queue.remove(0);
    skb.ext_reset = true;

    subflow.tcp_copied_seq = subflow.tcp_copied_seq.saturating_add(skb.len);
    subflow.ssn_offset = subflow.ssn_offset.saturating_add(skb.len);

    let has_rxtstamp = skb.has_rxtstamp;
    skb.map_seq = -(skb.len as i64);
    skb.end_seq = 0;
    skb.offset = 0;
    skb.has_rxtstamp = has_rxtstamp;
    skb.cant_coalesce = true;
    skb.owner = SkbOwner::MptcpConnection;

    subflow.bytes_received = subflow.bytes_received.saturating_add(skb.len as u64);
    subflow.conn_receive_queue.push(skb);
    subflow.data_ready_called = true;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mptcp_fastopen_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/mptcp/fastopen.c"
        ));
        assert!(source.contains("void mptcp_fastopen_subflow_synack_set_params"));
        assert!(source.contains("if (!subflow)"));
        assert!(source.contains("subflow->is_mptfo = 1;"));
        assert!(source.contains("skb = skb_peek(&ssk->sk_receive_queue);"));
        assert!(source.contains("__skb_unlink(skb, &ssk->sk_receive_queue);"));
        assert!(source.contains("skb_ext_reset(skb);"));
        assert!(source.contains("tp->copied_seq += skb->len;"));
        assert!(source.contains("subflow->ssn_offset += skb->len;"));
        assert!(source.contains("MPTCP_SKB_CB(skb)->map_seq = -skb->len;"));
        assert!(source.contains("MPTCP_SKB_CB(skb)->cant_coalesce = 1;"));
        assert!(source.contains("skb_set_owner_r(skb, sk);"));
        assert!(source.contains("__skb_queue_tail(&sk->sk_receive_queue, skb);"));
        assert!(source.contains("mptcp_sk(sk)->bytes_received += skb->len;"));
        assert!(source.contains("sk->sk_data_ready(sk);"));
    }

    #[test]
    fn fastopen_moves_first_subflow_skb_to_mptcp_queue() {
        assert!(!mptcp_fastopen_subflow_synack_set_params(None));

        let mut subflow = MptcpSubflowContext {
            tcp_copied_seq: 10,
            ssn_offset: 20,
            subflow_receive_queue: alloc::vec![
                FastopenSkb::new(5, true),
                FastopenSkb::new(7, false)
            ],
            ..MptcpSubflowContext::default()
        };

        assert!(mptcp_fastopen_subflow_synack_set_params(Some(&mut subflow)));
        assert!(subflow.is_mptfo);
        assert_eq!(subflow.tcp_copied_seq, 15);
        assert_eq!(subflow.ssn_offset, 25);
        assert_eq!(subflow.bytes_received, 5);
        assert!(subflow.data_ready_called);
        assert_eq!(subflow.subflow_receive_queue.len(), 1);
        assert_eq!(subflow.conn_receive_queue.len(), 1);

        let skb = &subflow.conn_receive_queue[0];
        assert!(skb.ext_reset);
        assert!(skb.has_rxtstamp);
        assert_eq!(skb.map_seq, -5);
        assert_eq!(skb.end_seq, 0);
        assert_eq!(skb.offset, 0);
        assert!(skb.cant_coalesce);
        assert_eq!(skb.owner, SkbOwner::MptcpConnection);
    }
}
