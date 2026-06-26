//! linux-parity: complete
//! linux-source: vendor/linux/net/rds/tcp_stats.c
//! test-origin: linux:vendor/linux/net/rds/tcp_stats.c
//! RDS TCP statistics aggregation.

pub const RDS_TCP_STAT_NAMES: [&str; 5] = [
    "tcp_data_ready_calls",
    "tcp_write_space_calls",
    "tcp_sndbuf_full",
    "tcp_connect_raced",
    "tcp_listen_closed_stale",
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RdsTcpStatistics {
    pub tcp_data_ready_calls: u64,
    pub tcp_write_space_calls: u64,
    pub tcp_sndbuf_full: u64,
    pub tcp_connect_raced: u64,
    pub tcp_listen_closed_stale: u64,
}

pub fn rds_tcp_stats_info_copy(
    per_cpu: &[RdsTcpStatistics],
    avail: usize,
) -> (usize, Option<RdsTcpStatistics>) {
    if avail < RDS_TCP_STAT_NAMES.len() {
        return (RDS_TCP_STAT_NAMES.len(), None);
    }

    let mut stats = RdsTcpStatistics::default();
    for cpu in per_cpu {
        stats.tcp_data_ready_calls = stats
            .tcp_data_ready_calls
            .saturating_add(cpu.tcp_data_ready_calls);
        stats.tcp_write_space_calls = stats
            .tcp_write_space_calls
            .saturating_add(cpu.tcp_write_space_calls);
        stats.tcp_sndbuf_full = stats.tcp_sndbuf_full.saturating_add(cpu.tcp_sndbuf_full);
        stats.tcp_connect_raced = stats
            .tcp_connect_raced
            .saturating_add(cpu.tcp_connect_raced);
        stats.tcp_listen_closed_stale = stats
            .tcp_listen_closed_stale
            .saturating_add(cpu.tcp_listen_closed_stale);
    }

    (RDS_TCP_STAT_NAMES.len(), Some(stats))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rds_tcp_stats_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/rds/tcp_stats.c"
        ));
        assert!(source.contains("DEFINE_PER_CPU(struct rds_tcp_statistics, rds_tcp_stats)"));
        assert!(source.contains("static const char * const rds_tcp_stat_names[]"));
        assert!(source.contains("\"tcp_data_ready_calls\""));
        assert!(source.contains("\"tcp_write_space_calls\""));
        assert!(source.contains("\"tcp_sndbuf_full\""));
        assert!(source.contains("\"tcp_connect_raced\""));
        assert!(source.contains("\"tcp_listen_closed_stale\""));
        assert!(source.contains("if (avail < ARRAY_SIZE(rds_tcp_stat_names))"));
        assert!(source.contains("for_each_online_cpu(cpu)"));
        assert!(source.contains("*(sum++) += *(src++);"));
        assert!(
            source.contains("rds_stats_info_copy(iter, (uint64_t *)&stats, rds_tcp_stat_names")
        );
        assert!(source.contains("return ARRAY_SIZE(rds_tcp_stat_names);"));
    }

    #[test]
    fn rds_tcp_stats_sum_only_when_available() {
        let cpus = [
            RdsTcpStatistics {
                tcp_data_ready_calls: 1,
                tcp_write_space_calls: 2,
                tcp_sndbuf_full: 3,
                tcp_connect_raced: 4,
                tcp_listen_closed_stale: 5,
            },
            RdsTcpStatistics {
                tcp_data_ready_calls: 10,
                tcp_write_space_calls: 20,
                tcp_sndbuf_full: 30,
                tcp_connect_raced: 40,
                tcp_listen_closed_stale: 50,
            },
        ];
        assert_eq!(rds_tcp_stats_info_copy(&cpus, 4), (5, None));
        assert_eq!(
            rds_tcp_stats_info_copy(&cpus, 5),
            (
                5,
                Some(RdsTcpStatistics {
                    tcp_data_ready_calls: 11,
                    tcp_write_space_calls: 22,
                    tcp_sndbuf_full: 33,
                    tcp_connect_raced: 44,
                    tcp_listen_closed_stale: 55,
                })
            )
        );
    }
}
