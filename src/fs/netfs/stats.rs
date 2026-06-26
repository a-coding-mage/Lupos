//! linux-parity: complete
//! linux-source: vendor/linux/fs/netfs/stats.c
//! test-origin: linux:vendor/linux/fs/netfs/stats.c
//! Netfs statistics counters and proc row ordering.

pub const NETFS_COUNTER_SYMBOLS: &[&str] = &[
    "netfs_n_rh_dio_read",
    "netfs_n_rh_readahead",
    "netfs_n_rh_read_folio",
    "netfs_n_rh_read_single",
    "netfs_n_rh_rreq",
    "netfs_n_rh_sreq",
    "netfs_n_rh_download",
    "netfs_n_rh_download_done",
    "netfs_n_rh_download_failed",
    "netfs_n_rh_download_instead",
    "netfs_n_rh_read",
    "netfs_n_rh_read_done",
    "netfs_n_rh_read_failed",
    "netfs_n_rh_zero",
    "netfs_n_rh_short_read",
    "netfs_n_rh_write",
    "netfs_n_rh_write_begin",
    "netfs_n_rh_write_done",
    "netfs_n_rh_write_failed",
    "netfs_n_rh_write_zskip",
    "netfs_n_wh_buffered_write",
    "netfs_n_wh_writethrough",
    "netfs_n_wh_dio_write",
    "netfs_n_wh_writepages",
    "netfs_n_wh_copy_to_cache",
    "netfs_n_wh_wstream_conflict",
    "netfs_n_wh_upload",
    "netfs_n_wh_upload_done",
    "netfs_n_wh_upload_failed",
    "netfs_n_wh_write",
    "netfs_n_wh_write_done",
    "netfs_n_wh_write_failed",
    "netfs_n_wh_retry_write_req",
    "netfs_n_wh_retry_write_subreq",
    "netfs_n_wb_lock_skip",
    "netfs_n_wb_lock_wait",
    "netfs_n_folioq",
];

pub const NETFS_STATS_ROWS: &[&str] = &[
    "Reads  : DR=%u RA=%u RF=%u RS=%u WB=%u WBZ=%u\\n",
    "Writes : BW=%u WT=%u DW=%u WP=%u 2C=%u\\n",
    "ZeroOps: ZR=%u sh=%u sk=%u\\n",
    "DownOps: DL=%u ds=%u df=%u di=%u\\n",
    "CaRdOps: RD=%u rs=%u rf=%u\\n",
    "UpldOps: UL=%u us=%u uf=%u\\n",
    "CaWrOps: WR=%u ws=%u wf=%u\\n",
    "Retries: rq=%u rs=%u wq=%u ws=%u\\n",
    "Objs   : rr=%u sr=%u foq=%u wsc=%u\\n",
    "WbLock : skip=%u wait=%u\\n",
];

pub const fn netfs_stats_show_row_count() -> usize {
    NETFS_STATS_ROWS.len()
}

pub const fn netfs_stats_show_calls_fscache_stats() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn netfs_stats_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/netfs/stats.c"
        ));
        assert!(source.contains("#include <linux/export.h>"));
        assert!(source.contains("#include <linux/seq_file.h>"));
        assert!(source.contains("atomic_t netfs_n_rh_dio_read;"));
        assert!(source.contains("atomic_t netfs_n_wh_retry_write_subreq;"));
        assert!(source.contains("atomic_t netfs_n_wb_lock_wait;"));
        assert!(source.contains("int netfs_stats_show(struct seq_file *m, void *v)"));
        for row in NETFS_STATS_ROWS {
            assert!(source.contains(row));
        }
        assert!(source.contains("return fscache_stats_show(m);"));
        assert!(source.contains("EXPORT_SYMBOL(netfs_stats_show);"));

        assert_eq!(NETFS_COUNTER_SYMBOLS.len(), 37);
        assert_eq!(netfs_stats_show_row_count(), 10);
        assert!(netfs_stats_show_calls_fscache_stats());
    }
}
