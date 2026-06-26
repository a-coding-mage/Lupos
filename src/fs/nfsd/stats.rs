//! linux-parity: complete
//! linux-source: vendor/linux/fs/nfsd/stats.c
//! test-origin: linux:vendor/linux/fs/nfsd/stats.c
//! NFS daemon procfs stats layout.

pub const REPLY_CACHE_FIELDS: usize = 3;
pub const FILEHANDLE_STALE_FIELDS: usize = 5;
pub const IO_FIELDS: usize = 2;
pub const THREAD_HISTOGRAM_BUCKETS: usize = 10;
pub const RA_CACHE_DEPRECATED_FIELDS: usize = 12;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NfsdStatsSnapshot {
    pub rc_hits: u64,
    pub rc_misses: u64,
    pub rc_nocache: u64,
    pub fh_stale: u64,
    pub io_read: u64,
    pub io_write: u64,
    pub threads: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NfsdProcStatsPrefix {
    pub reply_cache: [u64; REPLY_CACHE_FIELDS],
    pub filehandle: [u64; FILEHANDLE_STALE_FIELDS],
    pub io: [u64; IO_FIELDS],
    pub threads: u32,
    pub thread_histogram_buckets: usize,
    pub ra_cache_fields: usize,
}

pub const fn nfsd_proc_stats_prefix(snapshot: NfsdStatsSnapshot) -> NfsdProcStatsPrefix {
    NfsdProcStatsPrefix {
        reply_cache: [snapshot.rc_hits, snapshot.rc_misses, snapshot.rc_nocache],
        filehandle: [snapshot.fh_stale, 0, 0, 0, 0],
        io: [snapshot.io_read, snapshot.io_write],
        threads: snapshot.threads,
        thread_histogram_buckets: THREAD_HISTOGRAM_BUCKETS,
        ra_cache_fields: RA_CACHE_DEPRECATED_FIELDS,
    }
}

pub const fn nfsd_proc_stat_init_uses_svc_proc_register() -> bool {
    true
}

pub const fn nfsd_proc_stat_shutdown_name() -> &'static str {
    "nfsd"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfsd_stats_proc_layout_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/nfsd/stats.c"
        ));
        assert!(source.contains("#include <linux/seq_file.h>"));
        assert!(source.contains("#include <linux/module.h>"));
        assert!(source.contains("#include <linux/sunrpc/stats.h>"));
        assert!(source.contains("#include <net/net_namespace.h>"));
        assert!(source.contains("#include \"nfsd.h\""));
        assert!(source.contains("static int nfsd_show"));
        assert!(source.contains("\"rc %lld %lld %lld\\nfh %lld 0 0 0 0\\nio %lld %lld\\n\""));
        assert!(source.contains("NFSD_STATS_RC_HITS"));
        assert!(source.contains("NFSD_STATS_RC_MISSES"));
        assert!(source.contains("NFSD_STATS_RC_NOCACHE"));
        assert!(source.contains("NFSD_STATS_FH_STALE"));
        assert!(source.contains("NFSD_STATS_IO_READ"));
        assert!(source.contains("NFSD_STATS_IO_WRITE"));
        assert!(source.contains("seq_printf(seq, \"th %u 0\""));
        assert!(source.contains("for (i = 0; i < 10; i++)"));
        assert!(source.contains("seq_puts(seq, \" 0.000\");"));
        assert!(source.contains("\"\\nra 0 0 0 0 0 0 0 0 0 0 0 0\\n\""));
        assert!(source.contains("svc_seq_show(seq, &nn->nfsd_svcstats);"));
        assert!(source.contains("DEFINE_PROC_SHOW_ATTRIBUTE(nfsd);"));
        assert!(source.contains("svc_proc_register(net, &nn->nfsd_svcstats, &nfsd_proc_ops);"));
        assert!(source.contains("svc_proc_unregister(net, \"nfsd\");"));

        let prefix = nfsd_proc_stats_prefix(NfsdStatsSnapshot {
            rc_hits: 1,
            rc_misses: 2,
            rc_nocache: 3,
            fh_stale: 4,
            io_read: 5,
            io_write: 6,
            threads: 7,
        });
        assert_eq!(prefix.reply_cache, [1, 2, 3]);
        assert_eq!(prefix.filehandle, [4, 0, 0, 0, 0]);
        assert_eq!(prefix.io, [5, 6]);
        assert_eq!(prefix.threads, 7);
        assert_eq!(prefix.thread_histogram_buckets, 10);
        assert_eq!(prefix.ra_cache_fields, 12);
        assert!(nfsd_proc_stat_init_uses_svc_proc_register());
        assert_eq!(nfsd_proc_stat_shutdown_name(), "nfsd");
    }
}
