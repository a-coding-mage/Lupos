//! linux-parity: complete
//! linux-source: vendor/linux/fs/netfs/fscache_stats.c
//! test-origin: linux:vendor/linux/fs/netfs/fscache_stats.c
//! FS-Cache statistics counters and proc row ordering.

pub const FSCACHE_STATS_HEADER: &str = "-- FS-Cache statistics --\n";

pub const FSCACHE_COUNTER_SYMBOLS: &[&str] = &[
    "fscache_n_volumes",
    "fscache_n_volumes_collision",
    "fscache_n_volumes_nomem",
    "fscache_n_cookies",
    "fscache_n_cookies_lru",
    "fscache_n_cookies_lru_expired",
    "fscache_n_cookies_lru_removed",
    "fscache_n_cookies_lru_dropped",
    "fscache_n_acquires",
    "fscache_n_acquires_ok",
    "fscache_n_acquires_oom",
    "fscache_n_invalidates",
    "fscache_n_updates",
    "fscache_n_relinquishes",
    "fscache_n_relinquishes_retire",
    "fscache_n_relinquishes_dropped",
    "fscache_n_resizes",
    "fscache_n_resizes_null",
    "fscache_n_read",
    "fscache_n_write",
    "fscache_n_no_write_space",
    "fscache_n_no_create_space",
    "fscache_n_culled",
    "fscache_n_dio_misfit",
];

pub const FSCACHE_STATS_ROWS: &[&str] = &[
    "Cookies: n=%d v=%d vcol=%u voom=%u\\n",
    "Acquire: n=%u ok=%u oom=%u\\n",
    "LRU    : n=%u exp=%u rmv=%u drp=%u at=%ld\\n",
    "Invals : n=%u\\n",
    "Updates: n=%u rsz=%u rsn=%u\\n",
    "Relinqs: n=%u rtr=%u drop=%u\\n",
    "NoSpace: nwr=%u ncr=%u cull=%u\\n",
    "IO     : rd=%u wr=%u mis=%u\\n",
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FscacheStats {
    pub cookies: u32,
    pub volumes: u32,
    pub volumes_collision: u32,
    pub volumes_nomem: u32,
    pub acquires: u32,
    pub acquires_ok: u32,
    pub acquires_oom: u32,
    pub cookies_lru: u32,
    pub cookies_lru_expired: u32,
    pub cookies_lru_removed: u32,
    pub cookies_lru_dropped: u32,
    pub invalidates: u32,
    pub updates: u32,
    pub resizes: u32,
    pub resizes_null: u32,
    pub relinquishes: u32,
    pub relinquishes_retire: u32,
    pub relinquishes_dropped: u32,
    pub no_write_space: u32,
    pub no_create_space: u32,
    pub culled: u32,
    pub read: u32,
    pub write: u32,
    pub dio_misfit: u32,
}

pub const fn fscache_stats_show_row_count() -> usize {
    FSCACHE_STATS_ROWS.len()
}

pub const fn fscache_lru_timer_delta(timer_pending: bool, expires: i64, jiffies: i64) -> i64 {
    if timer_pending { expires - jiffies } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fscache_stats_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/netfs/fscache_stats.c"
        ));
        assert!(source.contains("#define FSCACHE_DEBUG_LEVEL CACHE"));
        assert!(source.contains("#include <linux/proc_fs.h>"));
        assert!(source.contains("#include <linux/seq_file.h>"));
        assert!(source.contains("atomic_t fscache_n_volumes;"));
        assert!(source.contains("atomic_t fscache_n_cookies_lru_dropped;"));
        assert!(source.contains("EXPORT_SYMBOL(fscache_n_updates);"));
        assert!(source.contains("EXPORT_SYMBOL(fscache_n_read);"));
        assert!(source.contains("EXPORT_SYMBOL(fscache_n_dio_misfit);"));
        assert!(source.contains("int fscache_stats_show(struct seq_file *m)"));
        assert!(source.contains("seq_puts(m, \"-- FS-Cache statistics --\\n\");"));
        for row in FSCACHE_STATS_ROWS {
            assert!(source.contains(row));
        }
        assert!(source.contains("timer_pending(&fscache_cookie_lru_timer) ?"));
        assert!(source.contains("fscache_cookie_lru_timer.expires - jiffies : 0"));

        assert_eq!(FSCACHE_COUNTER_SYMBOLS.len(), 24);
        assert_eq!(fscache_stats_show_row_count(), 8);
        assert_eq!(fscache_lru_timer_delta(false, 100, 25), 0);
        assert_eq!(fscache_lru_timer_delta(true, 100, 25), 75);
        assert_eq!(FSCACHE_STATS_HEADER, "-- FS-Cache statistics --\n");
    }
}
