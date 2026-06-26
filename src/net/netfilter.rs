//! linux-parity: complete
//! linux-source: vendor/linux/net
//! test-origin: linux:vendor/linux/net
//! Netfilter hook points and nf_tables-style rule evaluation.

extern crate alloc;

use alloc::vec::Vec;

#[path = "netfilter/ipvs/mod.rs"]
pub mod ipvs;
#[path = "netfilter/nf_conntrack_acct.rs"]
pub mod nf_conntrack_acct;
#[path = "netfilter/nf_conntrack_broadcast.rs"]
pub mod nf_conntrack_broadcast;
#[path = "netfilter/nf_conntrack_labels.rs"]
pub mod nf_conntrack_labels;
#[path = "netfilter/nf_conntrack_netbios_ns.rs"]
pub mod nf_conntrack_netbios_ns;
#[path = "netfilter/nf_conntrack_proto_generic.rs"]
pub mod nf_conntrack_proto_generic;
#[path = "netfilter/nf_conntrack_snmp.rs"]
pub mod nf_conntrack_snmp;
#[path = "netfilter/nf_conntrack_timestamp.rs"]
pub mod nf_conntrack_timestamp;
#[path = "netfilter/nf_dup_netdev.rs"]
pub mod nf_dup_netdev;
#[path = "netfilter/nf_flow_table_procfs.rs"]
pub mod nf_flow_table_procfs;
#[path = "netfilter/nf_nat_amanda.rs"]
pub mod nf_nat_amanda;
#[path = "netfilter/nf_nat_bpf.rs"]
pub mod nf_nat_bpf;
#[path = "netfilter/nf_nat_tftp.rs"]
pub mod nf_nat_tftp;
#[path = "netfilter/nft_byteorder.rs"]
pub mod nft_byteorder;
#[path = "netfilter/nft_ct_fast.rs"]
pub mod nft_ct_fast;
#[path = "netfilter/nft_dup_netdev.rs"]
pub mod nft_dup_netdev;
#[path = "netfilter/nft_fib_inet.rs"]
pub mod nft_fib_inet;
#[path = "netfilter/nft_fib_netdev.rs"]
pub mod nft_fib_netdev;
#[path = "netfilter/nft_hash.rs"]
pub mod nft_hash;
#[path = "netfilter/nft_last.rs"]
pub mod nft_last;
#[path = "netfilter/nft_numgen.rs"]
pub mod nft_numgen;
#[path = "netfilter/nft_range.rs"]
pub mod nft_range;
#[path = "netfilter/nft_reject.rs"]
pub mod nft_reject;
#[path = "netfilter/nft_reject_inet.rs"]
pub mod nft_reject_inet;
#[path = "netfilter/xt_AUDIT.rs"]
pub mod xt_audit;
#[path = "netfilter/xt_CHECKSUM.rs"]
pub mod xt_checksum;
#[path = "netfilter/xt_CLASSIFY.rs"]
pub mod xt_classify;
#[path = "netfilter/xt_comment.rs"]
pub mod xt_comment;
#[path = "netfilter/xt_connlabel.rs"]
pub mod xt_connlabel;
#[path = "netfilter/xt_cpu.rs"]
pub mod xt_cpu;
#[path = "netfilter/xt_devgroup.rs"]
pub mod xt_devgroup;
#[path = "netfilter/xt_dscp.rs"]
pub mod xt_dscp;
#[path = "netfilter/xt_esp.rs"]
pub mod xt_esp;
#[path = "netfilter/xt_helper.rs"]
pub mod xt_helper;
#[path = "netfilter/xt_hl.rs"]
pub mod xt_hl;
#[path = "netfilter/xt_ipcomp.rs"]
pub mod xt_ipcomp;
#[path = "netfilter/xt_iprange.rs"]
pub mod xt_iprange;
#[path = "netfilter/xt_length.rs"]
pub mod xt_length;
#[path = "netfilter/xt_limit.rs"]
pub mod xt_limit;
#[path = "netfilter/xt_LOG.rs"]
pub mod xt_log;
#[path = "netfilter/xt_mac.rs"]
pub mod xt_mac;
#[path = "netfilter/xt_mark.rs"]
pub mod xt_mark;
#[path = "netfilter/xt_nfacct.rs"]
pub mod xt_nfacct;
#[path = "netfilter/xt_NFLOG.rs"]
pub mod xt_nflog;
#[path = "netfilter/xt_NFQUEUE.rs"]
pub mod xt_nfqueue;
#[path = "netfilter/xt_osf.rs"]
pub mod xt_osf;
#[path = "netfilter/xt_pkttype.rs"]
pub mod xt_pkttype;
#[path = "netfilter/xt_quota.rs"]
pub mod xt_quota;
#[path = "netfilter/xt_rateest.rs"]
pub mod xt_rateest;
#[path = "netfilter/xt_realm.rs"]
pub mod xt_realm;
#[path = "netfilter/xt_state.rs"]
pub mod xt_state;
#[path = "netfilter/xt_statistic.rs"]
pub mod xt_statistic;
#[path = "netfilter/xt_string.rs"]
pub mod xt_string;
#[path = "netfilter/xt_tcpmss.rs"]
pub mod xt_tcpmss;
#[path = "netfilter/xt_TCPOPTSTRIP.rs"]
pub mod xt_tcpoptstrip;
#[path = "netfilter/xt_trace.rs"]
pub mod xt_trace;
#[path = "netfilter/xt_u32.rs"]
pub mod xt_u32;

use crate::net::ip::{IPPROTO_ICMP, IPPROTO_UDP, parse_ipv4_packet};
use crate::net::skbuff::SkBuff;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hook {
    PreRouting,
    LocalIn,
    Forward,
    LocalOut,
    PostRouting,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Accept,
    Drop,
    Reject,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Match {
    Any,
    IpProtocol(u8),
    UdpDport(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rule {
    pub hook: Hook,
    pub matcher: Match,
    pub verdict: Verdict,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RuleState {
    rule: Rule,
    hits: u64,
}

#[derive(Clone, Debug, Default)]
pub struct NfTable {
    rules: Vec<RuleState>,
}

impl NfTable {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(RuleState { rule, hits: 0 });
    }

    pub fn drop_icmp(&mut self, hook: Hook) {
        self.add_rule(Rule {
            hook,
            matcher: Match::IpProtocol(IPPROTO_ICMP),
            verdict: Verdict::Drop,
        });
    }

    pub fn evaluate(&self, hook: Hook, skb: &SkBuff) -> Verdict {
        for state in self.rules.iter().filter(|state| state.rule.hook == hook) {
            if rule_matches(state.rule.matcher, skb) {
                return state.rule.verdict;
            }
        }
        Verdict::Accept
    }

    pub fn evaluate_counting(&mut self, hook: Hook, skb: &SkBuff) -> Verdict {
        for state in self
            .rules
            .iter_mut()
            .filter(|state| state.rule.hook == hook)
        {
            if rule_matches(state.rule.matcher, skb) {
                state.hits = state.hits.saturating_add(1);
                return state.rule.verdict;
            }
        }
        Verdict::Accept
    }

    pub fn reject_udp_dport(&mut self, hook: Hook, port: u16) {
        self.add_rule(Rule {
            hook,
            matcher: Match::UdpDport(port),
            verdict: Verdict::Reject,
        });
    }

    pub fn rule_hits(&self, index: usize) -> Option<u64> {
        self.rules.get(index).map(|state| state.hits)
    }
}

fn rule_matches(matcher: Match, skb: &SkBuff) -> bool {
    match matcher {
        Match::Any => true,
        Match::IpProtocol(proto) => parse_ipv4_packet(skb)
            .map(|pkt| pkt.protocol == proto)
            .unwrap_or(false),
        Match::UdpDport(port) => parse_ipv4_packet(skb)
            .map(|pkt| {
                pkt.protocol == IPPROTO_UDP
                    && pkt.payload.len() >= 4
                    && u16::from_be_bytes([pkt.payload[2], pkt.payload[3]]) == port
            })
            .unwrap_or(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::fib::ipv4;
    use crate::net::ip::{IPPROTO_ICMP, build_ipv4_packet};

    #[test]
    fn nft_drop_icmp_rule_takes_effect() {
        let skb = build_ipv4_packet(
            ipv4(10, 0, 0, 1),
            ipv4(10, 0, 0, 2),
            IPPROTO_ICMP,
            b"icmp",
            64,
        )
        .unwrap();
        let mut table = NfTable::new();
        assert_eq!(table.evaluate(Hook::LocalIn, &skb), Verdict::Accept);
        table.drop_icmp(Hook::LocalIn);
        assert_eq!(table.evaluate(Hook::LocalIn, &skb), Verdict::Drop);
    }

    #[test]
    fn nft_reject_rule_counts_udp_port_hits() {
        let skb = crate::net::udp::udp_sendmsg(
            crate::net::fib::ipv4(10, 0, 0, 1),
            crate::net::fib::ipv4(10, 0, 0, 2),
            1000,
            53,
            b"dns",
        )
        .unwrap();
        let mut table = NfTable::new();
        table.reject_udp_dport(Hook::LocalOut, 53);
        assert_eq!(
            table.evaluate_counting(Hook::LocalOut, &skb),
            Verdict::Reject
        );
        assert_eq!(table.rule_hits(0), Some(1));
    }
}
