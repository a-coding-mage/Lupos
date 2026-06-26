//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nf_flow_table_procfs.c
//! test-origin: linux:vendor/linux/net/netfilter/nf_flow_table_procfs.c
//! `/proc/net/stat/nf_flowtable` sequence rendering.

use crate::include::uapi::errno::ENOMEM;

pub const NF_FLOWTABLE_PROC_NAME: &str = "nf_flowtable";
pub const NF_FLOWTABLE_PROC_MODE: u16 = 0o444;
pub const NF_FLOWTABLE_HEADER: &str = "wq_add   wq_del   wq_stats\n";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NfFlowTableStat {
    pub count_wq_add: i32,
    pub count_wq_del: i32,
    pub count_wq_stats: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowTableSeqItem<'a> {
    Header,
    Cpu {
        cpu: usize,
        stat: &'a NfFlowTableStat,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowTableSeqLine {
    Header(&'static str),
    Stat {
        count_wq_add: i32,
        count_wq_del: i32,
        count_wq_stats: i32,
    },
}

pub fn nf_flow_table_cpu_seq_start<'a>(
    pos: &mut usize,
    cpu_possible: &[bool],
    stats: &'a [NfFlowTableStat],
) -> Option<FlowTableSeqItem<'a>> {
    if *pos == 0 {
        return Some(FlowTableSeqItem::Header);
    }

    for cpu in (*pos - 1)..cpu_possible.len() {
        if !cpu_possible[cpu] {
            continue;
        }
        *pos = cpu + 1;
        return stats
            .get(cpu)
            .map(|stat| FlowTableSeqItem::Cpu { cpu, stat });
    }

    None
}

pub fn nf_flow_table_cpu_seq_next<'a>(
    pos: &mut usize,
    cpu_possible: &[bool],
    stats: &'a [NfFlowTableStat],
) -> Option<FlowTableSeqItem<'a>> {
    for cpu in *pos..cpu_possible.len() {
        if !cpu_possible[cpu] {
            continue;
        }
        *pos = cpu + 1;
        return stats
            .get(cpu)
            .map(|stat| FlowTableSeqItem::Cpu { cpu, stat });
    }
    *pos = (*pos).saturating_add(1);
    None
}

pub const fn nf_flow_table_cpu_seq_show(item: FlowTableSeqItem<'_>) -> FlowTableSeqLine {
    match item {
        FlowTableSeqItem::Header => FlowTableSeqLine::Header(NF_FLOWTABLE_HEADER),
        FlowTableSeqItem::Cpu { stat, .. } => FlowTableSeqLine::Stat {
            count_wq_add: stat.count_wq_add,
            count_wq_del: stat.count_wq_del,
            count_wq_stats: stat.count_wq_stats,
        },
    }
}

pub const fn nf_flow_table_init_proc(proc_created: bool) -> Result<(), i32> {
    if proc_created { Ok(()) } else { Err(-ENOMEM) }
}

pub const fn nf_flow_table_fini_proc() -> &'static str {
    NF_FLOWTABLE_PROC_NAME
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nf_flow_table_procfs_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nf_flow_table_procfs.c"
        ));
        assert!(source.contains("nf_flow_table_cpu_seq_start"));
        assert!(source.contains("if (*pos == 0)"));
        assert!(source.contains("return SEQ_START_TOKEN;"));
        assert!(source.contains("for (cpu = *pos - 1; cpu < nr_cpu_ids; ++cpu)"));
        assert!(source.contains("if (!cpu_possible(cpu))"));
        assert!(source.contains("return per_cpu_ptr(net->ft.stat, cpu);"));
        assert!(source.contains("seq_puts(seq, \"wq_add   wq_del   wq_stats\\n\");"));
        assert!(source.contains("seq_printf(seq, \"%8d %8d %8d\\n\""));
        assert!(source.contains("proc_create_net(\"nf_flowtable\", 0444"));
        assert!(source.contains("return pde ? 0 : -ENOMEM;"));
        assert!(source.contains("remove_proc_entry(\"nf_flowtable\", net->proc_net_stat);"));

        let stats = [
            NfFlowTableStat::default(),
            NfFlowTableStat {
                count_wq_add: 1,
                count_wq_del: 2,
                count_wq_stats: 3,
            },
            NfFlowTableStat {
                count_wq_add: 4,
                count_wq_del: 5,
                count_wq_stats: 6,
            },
        ];
        let possible = [false, true, true];
        let mut pos = 0;
        assert_eq!(
            nf_flow_table_cpu_seq_show(
                nf_flow_table_cpu_seq_start(&mut pos, &possible, &stats).unwrap()
            ),
            FlowTableSeqLine::Header(NF_FLOWTABLE_HEADER)
        );
        pos = 1;
        assert_eq!(
            nf_flow_table_cpu_seq_start(&mut pos, &possible, &stats),
            Some(FlowTableSeqItem::Cpu {
                cpu: 1,
                stat: &stats[1]
            })
        );
        assert_eq!(
            nf_flow_table_cpu_seq_show(
                nf_flow_table_cpu_seq_next(&mut pos, &possible, &stats).unwrap()
            ),
            FlowTableSeqLine::Stat {
                count_wq_add: 4,
                count_wq_del: 5,
                count_wq_stats: 6
            }
        );
        assert_eq!(nf_flow_table_init_proc(false), Err(-ENOMEM));
        assert_eq!(nf_flow_table_fini_proc(), "nf_flowtable");
    }
}
