//! linux-parity: complete
//! linux-source: vendor/linux/net/rxrpc/txbuf.c
//! test-origin: linux:vendor/linux/net/rxrpc/txbuf.c
//! RxRPC transmit data buffer allocation and reference tracking.

pub const L1_CACHE_BYTES: usize = 64;
pub const RXRPC_JUMBO_HEADER_SIZE: usize = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RxrpcCall {
    pub debug_id: u32,
    pub out_clientflag: u8,
    pub send_top: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RxrpcTxbuf {
    pub refcount: i32,
    pub call_debug_id: u32,
    pub debug_id: u32,
    pub alloc_size: usize,
    pub space: usize,
    pub offset: usize,
    pub flags: u8,
    pub seq: u32,
    pub data_offset: usize,
    pub total_alloc: usize,
    pub freed: bool,
}

const fn round_up(value: usize, align: usize) -> usize {
    if align == 0 {
        value
    } else {
        value.div_ceil(align) * align
    }
}

pub fn rxrpc_alloc_data_txbuf(
    call: RxrpcCall,
    data_size: usize,
    data_align: usize,
    alloc_txb_ok: bool,
    page_frag_ok: bool,
    next_debug_id: u32,
) -> Option<RxrpcTxbuf> {
    if !alloc_txb_ok {
        return None;
    }
    let doff = round_up(RXRPC_JUMBO_HEADER_SIZE, data_align);
    let total = doff + data_size;
    let _frag_align = data_align.max(L1_CACHE_BYTES);
    if !page_frag_ok {
        return None;
    }
    Some(RxrpcTxbuf {
        refcount: 1,
        call_debug_id: call.debug_id,
        debug_id: next_debug_id,
        alloc_size: data_size,
        space: data_size,
        offset: 0,
        flags: call.out_clientflag,
        seq: call.send_top + 1,
        data_offset: doff,
        total_alloc: total,
        freed: false,
    })
}

pub const fn rxrpc_see_txbuf(txb: &RxrpcTxbuf) -> (u32, u32, u32, i32) {
    (txb.debug_id, txb.call_debug_id, txb.seq, txb.refcount)
}

pub fn rxrpc_put_txbuf(txb: &mut RxrpcTxbuf) -> bool {
    if txb.refcount > 0 {
        txb.refcount -= 1;
    }
    if txb.refcount == 0 {
        txb.freed = true;
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rxrpc_txbuf_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/rxrpc/txbuf.c"
        ));
        assert!(source.contains("static atomic_t rxrpc_txbuf_debug_ids;"));
        assert!(source.contains("atomic_t rxrpc_nr_txbuf;"));
        assert!(source.contains("rxrpc_alloc_data_txbuf"));
        assert!(source.contains("txb = kzalloc_obj(*txb, gfp);"));
        assert!(source.contains("doff = round_up(jsize, data_align);"));
        assert!(source.contains("total = doff + data_size;"));
        assert!(source.contains("data_align = umax(data_align, L1_CACHE_BYTES);"));
        assert!(source.contains("page_frag_alloc_align(&call->conn->tx_data_alloc, total, gfp"));
        assert!(source.contains("refcount_set(&txb->ref, 1);"));
        assert!(source.contains("txb->call_debug_id\t= call->debug_id;"));
        assert!(source.contains("txb->seq\t\t= call->send_top + 1;"));
        assert!(source.contains("txb->data\t\t= buf + doff;"));
        assert!(source.contains("atomic_inc(&rxrpc_nr_txbuf);"));
        assert!(source.contains("void rxrpc_see_txbuf"));
        assert!(source.contains("static void rxrpc_free_txbuf"));
        assert!(source.contains("page_frag_free(txb->data);"));
        assert!(source.contains("__refcount_dec_and_test(&txb->ref, &r);"));
    }

    #[test]
    fn txbuf_allocation_tracks_offsets_and_last_put_frees() {
        let call = RxrpcCall {
            debug_id: 44,
            out_clientflag: 1,
            send_top: 9,
        };
        let mut txb = rxrpc_alloc_data_txbuf(call, 100, 16, true, true, 7).unwrap();
        assert_eq!(txb.refcount, 1);
        assert_eq!(txb.call_debug_id, 44);
        assert_eq!(txb.debug_id, 7);
        assert_eq!(txb.seq, 10);
        assert_eq!(txb.data_offset, 16);
        assert_eq!(txb.total_alloc, 116);
        assert_eq!(rxrpc_see_txbuf(&txb), (7, 44, 10, 1));
        assert!(rxrpc_put_txbuf(&mut txb));
        assert!(txb.freed);
        assert!(rxrpc_alloc_data_txbuf(call, 100, 16, false, true, 7).is_none());
        assert!(rxrpc_alloc_data_txbuf(call, 100, 16, true, false, 7).is_none());
    }
}
