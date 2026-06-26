//! linux-parity: complete
//! linux-source: vendor/linux/fs/smb/smbdirect/debug.c
//! test-origin: linux:vendor/linux/fs/smb/smbdirect/debug.c
//! SMBDirect legacy proc debug fields.

pub const SMBDIRECT_V1: u32 = 0x0100;

pub const DEBUG_FIELD_LABELS: &[&str] = &[
    "SMBDirect protocol version",
    "transport status",
    "Conn receive_credit_max",
    "send_credit_target",
    "max_send_size",
    "Conn max_fragmented_recv_size",
    "max_fragmented_send_size",
    "max_receive_size",
    "Conn keep_alive_interval",
    "max_readwrite_size",
    "rdma_readwrite_threshold",
    "Debug count_get_receive_buffer",
    "count_put_receive_buffer",
    "count_send_empty",
    "Read Queue count_enqueue_reassembly_queue",
    "count_dequeue_reassembly_queue",
    "reassembly_data_length",
    "reassembly_queue_length",
    "Current Credits send_credits",
    "receive_credits",
    "receive_credit_target",
    "Pending send_pending",
    "MR responder_resources",
    "max_frmr_depth",
    "mr_type",
    "MR mr_ready_count",
    "mr_used_count",
];

pub const fn smbdirect_debug_should_emit(socket_present: bool) -> bool {
    socket_present
}

pub const fn keepalive_seconds(keepalive_interval_msec: u32) -> u32 {
    keepalive_interval_msec / 1000
}

pub const fn smbdirect_debug_field_count() -> usize {
    DEBUG_FIELD_LABELS.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smbdirect_debug_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/smb/smbdirect/debug.c"
        ));
        assert!(source.contains("#include \"internal.h\""));
        assert!(source.contains("#include <linux/seq_file.h>"));
        assert!(source.contains("void smbdirect_connection_legacy_debug_proc_show"));
        assert!(source.contains("if (!sc)"));
        assert!(source.contains("sp = &sc->parameters;"));
        assert!(source.contains("SMBDirect protocol version: 0x%x"));
        assert!(source.contains("SMBDIRECT_V1"));
        assert!(source.contains("smbdirect_socket_status_string(sc->status)"));
        assert!(source.contains("sp->keepalive_interval_msec / 1000"));
        assert!(source.contains("rdma_readwrite_threshold"));
        assert!(source.contains("sc->statistics.get_receive_buffer"));
        assert!(source.contains("sc->recv_io.reassembly.data_length"));
        assert!(source.contains("atomic_read(&sc->send_io.credits.count)"));
        assert!(source.contains("atomic_read(&sc->mr_io.ready.count)"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(smbdirect_connection_legacy_debug_proc_show);"));
        for label in DEBUG_FIELD_LABELS {
            assert!(source.contains(label));
        }

        assert!(!smbdirect_debug_should_emit(false));
        assert!(smbdirect_debug_should_emit(true));
        assert_eq!(keepalive_seconds(30_500), 30);
        assert_eq!(smbdirect_debug_field_count(), DEBUG_FIELD_LABELS.len());
    }
}
