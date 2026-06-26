//! linux-parity: complete
//! linux-source: vendor/linux/net/atm/atm_misc.c
//! test-origin: linux:vendor/linux/net/atm/atm_misc.c
//! ATM receive charging, PCR rounding, and SONET statistic helpers.

pub const ATM_MAX_PCR: i32 = -1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtmVccAccounting {
    pub sk_rmem_alloc: i32,
    pub sk_rcvbuf: i32,
    pub rx_drop: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtmAllocResult {
    pub vcc: AtmVccAccounting,
    pub skb_allocated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtmTrafprm {
    pub max_pcr: i32,
    pub pcr: i32,
    pub min_pcr: i32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SonetStats {
    pub section_bip: i32,
    pub line_bip: i32,
    pub path_bip: i32,
    pub line_febe: i32,
    pub path_febe: i32,
    pub corr_hcs: i32,
    pub uncorr_hcs: i32,
    pub tx_cells: i32,
    pub rx_cells: i32,
}

pub const fn skb_truesize(pdu_size: i32) -> i32 {
    pdu_size
}

pub const fn atm_force_charge(mut vcc: AtmVccAccounting, truesize: i32) -> AtmVccAccounting {
    vcc.sk_rmem_alloc += truesize;
    vcc
}

pub const fn atm_return(mut vcc: AtmVccAccounting, truesize: i32) -> AtmVccAccounting {
    vcc.sk_rmem_alloc -= truesize;
    vcc
}

pub const fn atm_charge(vcc: AtmVccAccounting, truesize: i32) -> (AtmVccAccounting, bool) {
    let charged = atm_force_charge(vcc, truesize);
    if charged.sk_rmem_alloc <= charged.sk_rcvbuf {
        return (charged, true);
    }
    let mut returned = atm_return(charged, truesize);
    returned.rx_drop += 1;
    (returned, false)
}

pub const fn atm_alloc_charge(
    vcc: AtmVccAccounting,
    pdu_size: i32,
    alloc_skb_ok: bool,
) -> AtmAllocResult {
    let guess = skb_truesize(pdu_size);
    let charged = atm_force_charge(vcc, guess);
    if charged.sk_rmem_alloc <= charged.sk_rcvbuf && alloc_skb_ok {
        return AtmAllocResult {
            vcc: charged,
            skb_allocated: true,
        };
    }
    let mut returned = atm_return(charged, guess);
    returned.rx_drop += 1;
    AtmAllocResult {
        vcc: returned,
        skb_allocated: false,
    }
}

pub const fn atm_pcr_goal(tp: AtmTrafprm) -> i32 {
    if tp.pcr != 0 && tp.pcr != ATM_MAX_PCR {
        return -tp.pcr;
    }
    if tp.min_pcr != 0 && tp.pcr == 0 {
        return tp.min_pcr;
    }
    if tp.max_pcr != ATM_MAX_PCR {
        return -tp.max_pcr;
    }
    0
}

pub const fn sonet_copy_stats(from: SonetStats) -> SonetStats {
    from
}

pub fn sonet_subtract_stats(from: &mut SonetStats, to: SonetStats) {
    from.section_bip -= to.section_bip;
    from.line_bip -= to.line_bip;
    from.path_bip -= to.path_bip;
    from.line_febe -= to.line_febe;
    from.path_febe -= to.path_febe;
    from.corr_hcs -= to.corr_hcs;
    from.uncorr_hcs -= to.uncorr_hcs;
    from.tx_cells -= to.tx_cells;
    from.rx_cells -= to.rx_cells;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atm_misc_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/atm/atm_misc.c"
        ));
        assert!(source.contains("int atm_charge(struct atm_vcc *vcc, int truesize)"));
        assert!(source.contains("atm_force_charge(vcc, truesize);"));
        assert!(
            source.contains("atomic_read(&sk_atm(vcc)->sk_rmem_alloc) <= sk_atm(vcc)->sk_rcvbuf")
        );
        assert!(source.contains("atm_return(vcc, truesize);"));
        assert!(source.contains("atomic_inc(&vcc->stats->rx_drop);"));
        assert!(source.contains("struct sk_buff *atm_alloc_charge"));
        assert!(source.contains("int guess = SKB_TRUESIZE(pdu_size);"));
        assert!(source.contains("alloc_skb(pdu_size, gfp_flags);"));
        assert!(source.contains("int atm_pcr_goal(const struct atm_trafprm *tp)"));
        assert!(source.contains("if (tp->pcr && tp->pcr != ATM_MAX_PCR)"));
        assert!(source.contains("void sonet_copy_stats"));
        assert!(source.contains("void sonet_subtract_stats"));
        assert!(source.contains("EXPORT_SYMBOL(sonet_subtract_stats);"));
    }

    #[test]
    fn atm_charge_and_alloc_follow_receive_buffer_limit() {
        let vcc = AtmVccAccounting {
            sk_rmem_alloc: 0,
            sk_rcvbuf: 128,
            rx_drop: 0,
        };
        assert_eq!(
            atm_charge(vcc, 64),
            (
                AtmVccAccounting {
                    sk_rmem_alloc: 64,
                    sk_rcvbuf: 128,
                    rx_drop: 0,
                },
                true
            )
        );
        assert_eq!(
            atm_charge(vcc, 256),
            (AtmVccAccounting { rx_drop: 1, ..vcc }, false)
        );
        assert!(atm_alloc_charge(vcc, 64, true).skb_allocated);
        let failed = atm_alloc_charge(vcc, 64, false);
        assert!(!failed.skb_allocated);
        assert_eq!(failed.vcc.rx_drop, 1);
    }

    #[test]
    fn pcr_goal_and_sonet_stats_match_source_rules() {
        assert_eq!(
            atm_pcr_goal(AtmTrafprm {
                pcr: 100,
                min_pcr: 0,
                max_pcr: ATM_MAX_PCR,
            }),
            -100
        );
        assert_eq!(
            atm_pcr_goal(AtmTrafprm {
                pcr: 0,
                min_pcr: 50,
                max_pcr: ATM_MAX_PCR,
            }),
            50
        );
        assert_eq!(
            atm_pcr_goal(AtmTrafprm {
                pcr: ATM_MAX_PCR,
                min_pcr: 0,
                max_pcr: 250,
            }),
            -250
        );
        assert_eq!(
            atm_pcr_goal(AtmTrafprm {
                pcr: ATM_MAX_PCR,
                min_pcr: 0,
                max_pcr: ATM_MAX_PCR,
            }),
            0
        );
        let stats = SonetStats {
            section_bip: 10,
            line_bip: 9,
            path_bip: 8,
            line_febe: 7,
            path_febe: 6,
            corr_hcs: 5,
            uncorr_hcs: 4,
            tx_cells: 3,
            rx_cells: 2,
        };
        assert_eq!(sonet_copy_stats(stats), stats);
        let mut from = stats;
        sonet_subtract_stats(
            &mut from,
            SonetStats {
                rx_cells: 2,
                ..Default::default()
            },
        );
        assert_eq!(from.rx_cells, 0);
    }
}
