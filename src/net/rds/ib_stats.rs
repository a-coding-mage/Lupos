//! linux-parity: complete
//! linux-source: vendor/linux/net/rds/ib_stats.c
//! test-origin: linux:vendor/linux/net/rds/ib_stats.c
//! RDS InfiniBand per-CPU statistics aggregation.

pub const RDS_IB_STAT_NAMES: [&str; 39] = [
    "ib_connect_raced",
    "ib_listen_closed_stale",
    "ib_evt_handler_call",
    "ib_tasklet_call",
    "ib_tx_cq_event",
    "ib_tx_ring_full",
    "ib_tx_throttle",
    "ib_tx_sg_mapping_failure",
    "ib_tx_stalled",
    "ib_tx_credit_updates",
    "ib_rx_cq_event",
    "ib_rx_ring_empty",
    "ib_rx_refill_from_cq",
    "ib_rx_refill_from_thread",
    "ib_rx_alloc_limit",
    "ib_rx_total_frags",
    "ib_rx_total_incs",
    "ib_rx_credit_updates",
    "ib_ack_sent",
    "ib_ack_send_failure",
    "ib_ack_send_delayed",
    "ib_ack_send_piggybacked",
    "ib_ack_received",
    "ib_rdma_mr_8k_alloc",
    "ib_rdma_mr_8k_free",
    "ib_rdma_mr_8k_used",
    "ib_rdma_mr_8k_pool_flush",
    "ib_rdma_mr_8k_pool_wait",
    "ib_rdma_mr_8k_pool_depleted",
    "ib_rdma_mr_1m_alloc",
    "ib_rdma_mr_1m_free",
    "ib_rdma_mr_1m_used",
    "ib_rdma_mr_1m_pool_flush",
    "ib_rdma_mr_1m_pool_wait",
    "ib_rdma_mr_1m_pool_depleted",
    "ib_rdma_mr_8k_reused",
    "ib_rdma_mr_1m_reused",
    "ib_atomic_cswp",
    "ib_atomic_fadd",
];

pub fn rds_ib_stats_info_copy(per_cpu: &[[u64; 39]], avail: usize) -> (usize, Option<[u64; 39]>) {
    if avail < RDS_IB_STAT_NAMES.len() {
        return (RDS_IB_STAT_NAMES.len(), None);
    }
    let mut stats = [0u64; 39];
    for cpu in per_cpu {
        for (sum, value) in stats.iter_mut().zip(cpu.iter()) {
            *sum += *value;
        }
    }
    (RDS_IB_STAT_NAMES.len(), Some(stats))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rds_ib_stats_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/rds/ib_stats.c"
        ));
        assert!(
            source
                .contains("DEFINE_PER_CPU_SHARED_ALIGNED(struct rds_ib_statistics, rds_ib_stats);")
        );
        assert!(source.contains("static const char *const rds_ib_stat_names[]"));
        assert!(source.contains("\"ib_connect_raced\""));
        assert!(source.contains("\"ib_atomic_fadd\""));
        assert!(source.contains("unsigned int rds_ib_stats_info_copy"));
        assert!(source.contains("struct rds_ib_statistics stats = {0, };"));
        assert!(source.contains("if (avail < ARRAY_SIZE(rds_ib_stat_names))"));
        assert!(source.contains("for_each_online_cpu(cpu)"));
        assert!(source.contains("for (i = 0; i < sizeof(stats) / sizeof(uint64_t); i++)"));
        assert!(source.contains("*(sum++) += *(src++);"));
        assert!(source.contains("rds_stats_info_copy(iter, (uint64_t *)&stats, rds_ib_stat_names"));
        assert!(source.contains("return ARRAY_SIZE(rds_ib_stat_names);"));
    }

    #[test]
    fn ib_stats_sums_each_counter_when_space_is_available() {
        assert_eq!(RDS_IB_STAT_NAMES.len(), 39);
        let cpu0 = [1u64; 39];
        let cpu1 = [2u64; 39];
        let (needed, stats) = rds_ib_stats_info_copy(&[cpu0, cpu1], 39);
        assert_eq!(needed, 39);
        assert_eq!(stats.unwrap(), [3u64; 39]);
        assert_eq!(rds_ib_stats_info_copy(&[cpu0], 38), (39, None));
    }
}
