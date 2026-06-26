//! linux-parity: partial
//! linux-source: vendor/linux/net
//! test-origin: linux:vendor/linux/net
//! Linux networking source coverage and unsupported-operation policy.
//!
//! Coverage/policy index: classifies the 549 Linux net source files
//! (drivers/net, net/core, ipv4/ipv6, netfilter, ipset, ipvs, unix, …) as
//! Implemented/Unsupported and supplies the unsupported-op errno policy.
//! Remaining work vs Linux for `complete`: this index reflects partial
//! subsystem coverage — entries marked `Unsupported` are Linux net features
//! Lupos does not yet implement.
//!
//! Keep these Linux references source-shaped, but route behavior through the
//! existing networking modules instead of adding roadmap-named placeholders.
//!
//! Refs:
//! - `vendor/linux/drivers/net/{virtio_net}.c`
//! - `vendor/linux/net/{socket}.c`
//! - `vendor/linux/net/core/{bpf_sk_storage,datagram,dev,dev_addr_lists,dev_api,dev_ioctl,devmem,drop_monitor,dst,dst_cache,failover,fib_notifier,fib_rules,filter,flow_dissector,flow_offload,gen_estimator,gen_stats,gro,gro_cells,gso,hotdata,hwbm,ieee8021q_helpers,link_watch,lock_debug,lwt_bpf,lwtunnel,neighbour,net_namespace,netclassid_cgroup,netdev_config,netdev_queues,netdev_rx_queue,netdev-genl,netdev-genl-gen,netevent,netpoll,netprio_cgroup,net-procfs,net-sysfs,net-traces,of_net,page_pool,page_pool_user,pktgen,ptp_classifier,rtnetlink,scm,secure_seq,selftests,skb_fault_injection,skbuff,skmsg,sock,sock_diag,sock_map,sock_reuseport,stream,sysctl_net_core,timestamping,tso,utils,xdp}.c`
//! - `vendor/linux/net/mpls/{mpls_gso}.c`
//! - `vendor/linux/net/ipv4/{af_inet,ah4,arp,bpf_tcp_ca,cipso_ipv4,datagram,devinet,esp4,esp4_offload,fib_frontend,fib_notifier,fib_rules,fib_semantics,fib_trie,fou_bpf,fou_core,fou_nl,gre_demux,gre_offload,icmp,igmp,inet_connection_sock,inet_diag,inet_fragment,inet_hashtables,inet_timewait_sock,inetpeer,ip_forward,ip_fragment,ip_gre,ip_input,ip_options,ip_output,ip_sockglue,ip_tunnel,ip_tunnel_core,ip_vti,ipcomp,ipconfig,ipip,ipmr,ipmr_base,metrics,netfilter,netlink,nexthop,ping,proc,protocol,raw,raw_diag,route,syncookies,sysctl_net_ipv4,tcp,tcp_ao,tcp_bbr,tcp_bic,tcp_bpf,tcp_cdg,tcp_cong,tcp_cubic,tcp_dctcp,tcp_diag,tcp_fastopen,tcp_highspeed,tcp_htcp,tcp_hybla,tcp_illinois,tcp_input,tcp_ipv4,tcp_lp,tcp_metrics,tcp_minisocks,tcp_nv,tcp_offload,tcp_output,tcp_plb,tcp_recovery,tcp_scalable,tcp_sigpool,tcp_timer,tcp_ulp,tcp_vegas,tcp_veno,tcp_westwood,tcp_yeah,tunnel4,udp,udp_bpf,udp_diag,udp_offload,udp_tunnel_core,udp_tunnel_nic,udp_tunnel_stub,xfrm4_input,xfrm4_output,xfrm4_policy,xfrm4_protocol,xfrm4_state,xfrm4_tunnel}.c`
//! - `vendor/linux/net/ipv4/netfilter/{arp_tables,arpt_mangle,arptable_filter,ip_tables,ipt_ah,ipt_ECN,ipt_REJECT,ipt_rpfilter,ipt_SYNPROXY,iptable_filter,iptable_mangle,iptable_nat,iptable_raw,iptable_security,nf_defrag_ipv4,nf_dup_ipv4,nf_nat_h323,nf_nat_pptp,nf_nat_snmp_basic_main,nf_reject_ipv4,nf_socket_ipv4,nf_tproxy_ipv4,nft_dup_ipv4,nft_fib_ipv4,nft_reject_ipv4}.c`
//! - `vendor/linux/net/ipv6/{addrconf,addrconf_core,addrlabel,af_inet6,ah6,anycast,calipso,datagram,esp6,esp6_offload,exthdrs,exthdrs_core,exthdrs_offload,fib6_notifier,fib6_rules,fou6,icmp,inet6_connection_sock,inet6_hashtables,ioam6,ioam6_iptunnel,ip6_checksum,ip6_fib,ip6_flowlabel,ip6_gre,ip6_icmp,ip6_input,ip6_offload,ip6_output,ip6_tunnel,ip6_udp_tunnel,ip6_vti,ip6mr,ipcomp6,ipv6_sockglue,mcast,mcast_snoop,mip6,ndisc,netfilter,output_core,ping,proc,protocol,raw,reassembly,route,rpl,rpl_iptunnel,seg6,seg6_hmac,seg6_iptunnel,seg6_local,sit,syncookies,sysctl_net_ipv6,tcp_ao,tcp_ipv6,tcpv6_offload,tunnel6,udp,udp_offload,xfrm6_input,xfrm6_output,xfrm6_policy,xfrm6_protocol,xfrm6_state,xfrm6_tunnel}.c`
//! - `vendor/linux/net/ipv6/ila/{ila_common,ila_lwt,ila_main,ila_xlat}.c`
//! - `vendor/linux/net/ipv6/netfilter/{ip6_tables,ip6t_ah,ip6t_eui64,ip6t_frag,ip6t_hbh,ip6t_ipv6header,ip6t_mh,ip6t_NPT,ip6t_REJECT,ip6t_rpfilter,ip6t_rt,ip6t_srh,ip6t_SYNPROXY,ip6table_filter,ip6table_mangle,ip6table_nat,ip6table_raw,ip6table_security,nf_conntrack_reasm,nf_defrag_ipv6_hooks,nf_dup_ipv6,nf_reject_ipv6,nf_socket_ipv6,nf_tproxy_ipv6,nft_dup_ipv6,nft_fib_ipv6,nft_reject_ipv6}.c`
//! - `vendor/linux/net/netfilter/{core,nf_bpf_link,nf_conncount,nf_conntrack_acct,nf_conntrack_amanda,nf_conntrack_bpf,nf_conntrack_broadcast,nf_conntrack_core,nf_conntrack_ecache,nf_conntrack_expect,nf_conntrack_extend,nf_conntrack_ftp,nf_conntrack_h323_asn1,nf_conntrack_h323_main,nf_conntrack_h323_types,nf_conntrack_helper,nf_conntrack_irc,nf_conntrack_labels,nf_conntrack_netbios_ns,nf_conntrack_netlink,nf_conntrack_ovs,nf_conntrack_pptp,nf_conntrack_proto,nf_conntrack_proto_generic,nf_conntrack_proto_gre,nf_conntrack_proto_icmp,nf_conntrack_proto_icmpv6,nf_conntrack_proto_sctp,nf_conntrack_proto_tcp,nf_conntrack_proto_udp,nf_conntrack_sane,nf_conntrack_seqadj,nf_conntrack_sip,nf_conntrack_snmp,nf_conntrack_standalone,nf_conntrack_tftp,nf_conntrack_timeout,nf_conntrack_timestamp,nf_dup_netdev,nf_flow_table_bpf,nf_flow_table_core,nf_flow_table_inet,nf_flow_table_ip,nf_flow_table_offload,nf_flow_table_path,nf_flow_table_procfs,nf_flow_table_xdp,nf_hooks_lwtunnel,nf_log,nf_log_syslog,nf_nat_amanda,nf_nat_bpf,nf_nat_core,nf_nat_ftp,nf_nat_helper,nf_nat_irc,nf_nat_masquerade,nf_nat_ovs,nf_nat_proto,nf_nat_redirect,nf_nat_sip,nf_nat_tftp,nf_queue,nf_sockopt,nf_synproxy_core,nf_tables_api,nf_tables_core,nf_tables_offload,nf_tables_trace,nfnetlink,nfnetlink_acct,nfnetlink_cthelper,nfnetlink_cttimeout,nfnetlink_hook,nfnetlink_log,nfnetlink_osf,nfnetlink_queue,nft_bitwise,nft_byteorder,nft_chain_filter,nft_chain_nat,nft_chain_route,nft_cmp,nft_compat,nft_connlimit,nft_counter,nft_ct,nft_ct_fast,nft_dup_netdev,nft_dynset,nft_exthdr,nft_fib,nft_fib_inet,nft_fib_netdev,nft_flow_offload,nft_fwd_netdev,nft_hash,nft_immediate,nft_inner,nft_last,nft_limit,nft_log,nft_lookup,nft_masq,nft_meta,nft_nat,nft_numgen,nft_objref,nft_osf,nft_payload,nft_queue,nft_quota,nft_range,nft_redir,nft_reject,nft_reject_inet,nft_reject_netdev,nft_rt,nft_set_bitmap,nft_set_hash,nft_set_pipapo,nft_set_pipapo_avx2,nft_set_rbtree,nft_socket,nft_synproxy,nft_tproxy,nft_tunnel,nft_xfrm,utils,x_tables,xt_addrtype,xt_AUDIT,xt_bpf,xt_cgroup,xt_CHECKSUM,xt_CLASSIFY,xt_cluster,xt_comment,xt_connbytes,xt_connlabel,xt_connlimit,xt_connmark,xt_CONNSECMARK,xt_conntrack,xt_cpu,xt_CT,xt_dccp,xt_devgroup,xt_DSCP,xt_dscp,xt_ecn,xt_esp,xt_hashlimit,xt_helper,xt_HL,xt_hl,xt_HMARK,xt_IDLETIMER,xt_ipcomp,xt_iprange,xt_ipvs,xt_l2tp,xt_LED,xt_length,xt_limit,xt_LOG,xt_mac,xt_mark,xt_MASQUERADE,xt_multiport,xt_nat,xt_NETMAP,xt_nfacct,xt_NFLOG,xt_NFQUEUE,xt_osf,xt_owner,xt_physdev,xt_pkttype,xt_policy,xt_quota,xt_RATEEST,xt_rateest,xt_realm,xt_recent,xt_REDIRECT,xt_sctp,xt_SECMARK,xt_set,xt_socket,xt_state,xt_statistic,xt_string,xt_TCPMSS,xt_tcpmss,xt_TCPOPTSTRIP,xt_tcpudp,xt_TEE,xt_time,xt_TPROXY,xt_TRACE,xt_u32}.c`
//! - `vendor/linux/net/netfilter/ipset/{ip_set_bitmap_ip,ip_set_bitmap_ipmac,ip_set_bitmap_port,ip_set_core,ip_set_getport,ip_set_hash_ip,ip_set_hash_ipmac,ip_set_hash_ipmark,ip_set_hash_ipport,ip_set_hash_ipportip,ip_set_hash_ipportnet,ip_set_hash_mac,ip_set_hash_net,ip_set_hash_netiface,ip_set_hash_netnet,ip_set_hash_netport,ip_set_hash_netportnet,ip_set_list_set,pfxlen}.c`
//! - `vendor/linux/net/netfilter/ipvs/{ip_vs_app,ip_vs_conn,ip_vs_core,ip_vs_ctl,ip_vs_dh,ip_vs_est,ip_vs_fo,ip_vs_ftp,ip_vs_lblc,ip_vs_lblcr,ip_vs_lc,ip_vs_mh,ip_vs_nfct,ip_vs_nq,ip_vs_ovf,ip_vs_pe,ip_vs_pe_sip,ip_vs_proto,ip_vs_proto_ah_esp,ip_vs_proto_sctp,ip_vs_proto_tcp,ip_vs_proto_udp,ip_vs_rr,ip_vs_sched,ip_vs_sed,ip_vs_sh,ip_vs_sync,ip_vs_twos,ip_vs_wlc,ip_vs_wrr,ip_vs_xmit}.c`
//! - `vendor/linux/net/unix/{af_unix,diag,garbage,sysctl_net_unix,unix_bpf}.c`

use crate::include::uapi::errno::{ENOENT, EOPNOTSUPP};

pub const NETWORKING_SOURCE_COUNT: usize = 549;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetSubsystem {
    Driver,
    Socket,
    Core,
    Mpls,
    Ipv4,
    Ipv4Netfilter,
    Ipv6,
    Ipv6Ila,
    Ipv6Netfilter,
    Netfilter,
    Ipset,
    Ipvs,
    Unix,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupportStatus {
    Implemented,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxNetSource {
    pub path: &'static str,
    pub subsystem: NetSubsystem,
    pub status: SupportStatus,
    pub unsupported_errno: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxNetSourceGroup {
    pub dir: &'static str,
    pub stems: &'static str,
    pub subsystem: NetSubsystem,
}

pub const SOURCE_GROUPS: &[LinuxNetSourceGroup] = &[
    LinuxNetSourceGroup {
        dir: "vendor/linux/drivers/net",
        stems: "virtio_net",
        subsystem: NetSubsystem::Driver,
    },
    LinuxNetSourceGroup {
        dir: "vendor/linux/net",
        stems: "socket",
        subsystem: NetSubsystem::Socket,
    },
    LinuxNetSourceGroup {
        dir: "vendor/linux/net/core",
        stems: "bpf_sk_storage,datagram,dev,dev_addr_lists,dev_api,dev_ioctl,devmem,drop_monitor,dst,dst_cache,failover,fib_notifier,fib_rules,filter,flow_dissector,flow_offload,gen_estimator,gen_stats,gro,gro_cells,gso,hotdata,hwbm,ieee8021q_helpers,link_watch,lock_debug,lwt_bpf,lwtunnel,neighbour,net_namespace,netclassid_cgroup,netdev_config,netdev_queues,netdev_rx_queue,netdev-genl,netdev-genl-gen,netevent,netpoll,netprio_cgroup,net-procfs,net-sysfs,net-traces,of_net,page_pool,page_pool_user,pktgen,ptp_classifier,rtnetlink,scm,secure_seq,selftests,skb_fault_injection,skbuff,skmsg,sock,sock_diag,sock_map,sock_reuseport,stream,sysctl_net_core,timestamping,tso,utils,xdp",
        subsystem: NetSubsystem::Core,
    },
    LinuxNetSourceGroup {
        dir: "vendor/linux/net/mpls",
        stems: "mpls_gso",
        subsystem: NetSubsystem::Mpls,
    },
    LinuxNetSourceGroup {
        dir: "vendor/linux/net/ipv4",
        stems: "af_inet,ah4,arp,bpf_tcp_ca,cipso_ipv4,datagram,devinet,esp4,esp4_offload,fib_frontend,fib_notifier,fib_rules,fib_semantics,fib_trie,fou_bpf,fou_core,fou_nl,gre_demux,gre_offload,icmp,igmp,inet_connection_sock,inet_diag,inet_fragment,inet_hashtables,inet_timewait_sock,inetpeer,ip_forward,ip_fragment,ip_gre,ip_input,ip_options,ip_output,ip_sockglue,ip_tunnel,ip_tunnel_core,ip_vti,ipcomp,ipconfig,ipip,ipmr,ipmr_base,metrics,netfilter,netlink,nexthop,ping,proc,protocol,raw,raw_diag,route,syncookies,sysctl_net_ipv4,tcp,tcp_ao,tcp_bbr,tcp_bic,tcp_bpf,tcp_cdg,tcp_cong,tcp_cubic,tcp_dctcp,tcp_diag,tcp_fastopen,tcp_highspeed,tcp_htcp,tcp_hybla,tcp_illinois,tcp_input,tcp_ipv4,tcp_lp,tcp_metrics,tcp_minisocks,tcp_nv,tcp_offload,tcp_output,tcp_plb,tcp_recovery,tcp_scalable,tcp_sigpool,tcp_timer,tcp_ulp,tcp_vegas,tcp_veno,tcp_westwood,tcp_yeah,tunnel4,udp,udp_bpf,udp_diag,udp_offload,udp_tunnel_core,udp_tunnel_nic,udp_tunnel_stub,xfrm4_input,xfrm4_output,xfrm4_policy,xfrm4_protocol,xfrm4_state,xfrm4_tunnel",
        subsystem: NetSubsystem::Ipv4,
    },
    LinuxNetSourceGroup {
        dir: "vendor/linux/net/ipv4/netfilter",
        stems: "arp_tables,arpt_mangle,arptable_filter,ip_tables,ipt_ah,ipt_ECN,ipt_REJECT,ipt_rpfilter,ipt_SYNPROXY,iptable_filter,iptable_mangle,iptable_nat,iptable_raw,iptable_security,nf_defrag_ipv4,nf_dup_ipv4,nf_nat_h323,nf_nat_pptp,nf_nat_snmp_basic_main,nf_reject_ipv4,nf_socket_ipv4,nf_tproxy_ipv4,nft_dup_ipv4,nft_fib_ipv4,nft_reject_ipv4",
        subsystem: NetSubsystem::Ipv4Netfilter,
    },
    LinuxNetSourceGroup {
        dir: "vendor/linux/net/ipv6",
        stems: "addrconf,addrconf_core,addrlabel,af_inet6,ah6,anycast,calipso,datagram,esp6,esp6_offload,exthdrs,exthdrs_core,exthdrs_offload,fib6_notifier,fib6_rules,fou6,icmp,inet6_connection_sock,inet6_hashtables,ioam6,ioam6_iptunnel,ip6_checksum,ip6_fib,ip6_flowlabel,ip6_gre,ip6_icmp,ip6_input,ip6_offload,ip6_output,ip6_tunnel,ip6_udp_tunnel,ip6_vti,ip6mr,ipcomp6,ipv6_sockglue,mcast,mcast_snoop,mip6,ndisc,netfilter,output_core,ping,proc,protocol,raw,reassembly,route,rpl,rpl_iptunnel,seg6,seg6_hmac,seg6_iptunnel,seg6_local,sit,syncookies,sysctl_net_ipv6,tcp_ao,tcp_ipv6,tcpv6_offload,tunnel6,udp,udp_offload,xfrm6_input,xfrm6_output,xfrm6_policy,xfrm6_protocol,xfrm6_state,xfrm6_tunnel",
        subsystem: NetSubsystem::Ipv6,
    },
    LinuxNetSourceGroup {
        dir: "vendor/linux/net/ipv6/ila",
        stems: "ila_common,ila_lwt,ila_main,ila_xlat",
        subsystem: NetSubsystem::Ipv6Ila,
    },
    LinuxNetSourceGroup {
        dir: "vendor/linux/net/ipv6/netfilter",
        stems: "ip6_tables,ip6t_ah,ip6t_eui64,ip6t_frag,ip6t_hbh,ip6t_ipv6header,ip6t_mh,ip6t_NPT,ip6t_REJECT,ip6t_rpfilter,ip6t_rt,ip6t_srh,ip6t_SYNPROXY,ip6table_filter,ip6table_mangle,ip6table_nat,ip6table_raw,ip6table_security,nf_conntrack_reasm,nf_defrag_ipv6_hooks,nf_dup_ipv6,nf_reject_ipv6,nf_socket_ipv6,nf_tproxy_ipv6,nft_dup_ipv6,nft_fib_ipv6,nft_reject_ipv6",
        subsystem: NetSubsystem::Ipv6Netfilter,
    },
    LinuxNetSourceGroup {
        dir: "vendor/linux/net/netfilter",
        stems: "core,nf_bpf_link,nf_conncount,nf_conntrack_acct,nf_conntrack_amanda,nf_conntrack_bpf,nf_conntrack_broadcast,nf_conntrack_core,nf_conntrack_ecache,nf_conntrack_expect,nf_conntrack_extend,nf_conntrack_ftp,nf_conntrack_h323_asn1,nf_conntrack_h323_main,nf_conntrack_h323_types,nf_conntrack_helper,nf_conntrack_irc,nf_conntrack_labels,nf_conntrack_netbios_ns,nf_conntrack_netlink,nf_conntrack_ovs,nf_conntrack_pptp,nf_conntrack_proto,nf_conntrack_proto_generic,nf_conntrack_proto_gre,nf_conntrack_proto_icmp,nf_conntrack_proto_icmpv6,nf_conntrack_proto_sctp,nf_conntrack_proto_tcp,nf_conntrack_proto_udp,nf_conntrack_sane,nf_conntrack_seqadj,nf_conntrack_sip,nf_conntrack_snmp,nf_conntrack_standalone,nf_conntrack_tftp,nf_conntrack_timeout,nf_conntrack_timestamp,nf_dup_netdev,nf_flow_table_bpf,nf_flow_table_core,nf_flow_table_inet,nf_flow_table_ip,nf_flow_table_offload,nf_flow_table_path,nf_flow_table_procfs,nf_flow_table_xdp,nf_hooks_lwtunnel,nf_log,nf_log_syslog,nf_nat_amanda,nf_nat_bpf,nf_nat_core,nf_nat_ftp,nf_nat_helper,nf_nat_irc,nf_nat_masquerade,nf_nat_ovs,nf_nat_proto,nf_nat_redirect,nf_nat_sip,nf_nat_tftp,nf_queue,nf_sockopt,nf_synproxy_core,nf_tables_api,nf_tables_core,nf_tables_offload,nf_tables_trace,nfnetlink,nfnetlink_acct,nfnetlink_cthelper,nfnetlink_cttimeout,nfnetlink_hook,nfnetlink_log,nfnetlink_osf,nfnetlink_queue,nft_bitwise,nft_byteorder,nft_chain_filter,nft_chain_nat,nft_chain_route,nft_cmp,nft_compat,nft_connlimit,nft_counter,nft_ct,nft_ct_fast,nft_dup_netdev,nft_dynset,nft_exthdr,nft_fib,nft_fib_inet,nft_fib_netdev,nft_flow_offload,nft_fwd_netdev,nft_hash,nft_immediate,nft_inner,nft_last,nft_limit,nft_log,nft_lookup,nft_masq,nft_meta,nft_nat,nft_numgen,nft_objref,nft_osf,nft_payload,nft_queue,nft_quota,nft_range,nft_redir,nft_reject,nft_reject_inet,nft_reject_netdev,nft_rt,nft_set_bitmap,nft_set_hash,nft_set_pipapo,nft_set_pipapo_avx2,nft_set_rbtree,nft_socket,nft_synproxy,nft_tproxy,nft_tunnel,nft_xfrm,utils,x_tables,xt_addrtype,xt_AUDIT,xt_bpf,xt_cgroup,xt_CHECKSUM,xt_CLASSIFY,xt_cluster,xt_comment,xt_connbytes,xt_connlabel,xt_connlimit,xt_connmark,xt_CONNSECMARK,xt_conntrack,xt_cpu,xt_CT,xt_dccp,xt_devgroup,xt_DSCP,xt_dscp,xt_ecn,xt_esp,xt_hashlimit,xt_helper,xt_HL,xt_hl,xt_HMARK,xt_IDLETIMER,xt_ipcomp,xt_iprange,xt_ipvs,xt_l2tp,xt_LED,xt_length,xt_limit,xt_LOG,xt_mac,xt_mark,xt_MASQUERADE,xt_multiport,xt_nat,xt_NETMAP,xt_nfacct,xt_NFLOG,xt_NFQUEUE,xt_osf,xt_owner,xt_physdev,xt_pkttype,xt_policy,xt_quota,xt_RATEEST,xt_rateest,xt_realm,xt_recent,xt_REDIRECT,xt_sctp,xt_SECMARK,xt_set,xt_socket,xt_state,xt_statistic,xt_string,xt_TCPMSS,xt_tcpmss,xt_TCPOPTSTRIP,xt_tcpudp,xt_TEE,xt_time,xt_TPROXY,xt_TRACE,xt_u32",
        subsystem: NetSubsystem::Netfilter,
    },
    LinuxNetSourceGroup {
        dir: "vendor/linux/net/netfilter/ipset",
        stems: "ip_set_bitmap_ip,ip_set_bitmap_ipmac,ip_set_bitmap_port,ip_set_core,ip_set_getport,ip_set_hash_ip,ip_set_hash_ipmac,ip_set_hash_ipmark,ip_set_hash_ipport,ip_set_hash_ipportip,ip_set_hash_ipportnet,ip_set_hash_mac,ip_set_hash_net,ip_set_hash_netiface,ip_set_hash_netnet,ip_set_hash_netport,ip_set_hash_netportnet,ip_set_list_set,pfxlen",
        subsystem: NetSubsystem::Ipset,
    },
    LinuxNetSourceGroup {
        dir: "vendor/linux/net/netfilter/ipvs",
        stems: "ip_vs_app,ip_vs_conn,ip_vs_core,ip_vs_ctl,ip_vs_dh,ip_vs_est,ip_vs_fo,ip_vs_ftp,ip_vs_lblc,ip_vs_lblcr,ip_vs_lc,ip_vs_mh,ip_vs_nfct,ip_vs_nq,ip_vs_ovf,ip_vs_pe,ip_vs_pe_sip,ip_vs_proto,ip_vs_proto_ah_esp,ip_vs_proto_sctp,ip_vs_proto_tcp,ip_vs_proto_udp,ip_vs_rr,ip_vs_sched,ip_vs_sed,ip_vs_sh,ip_vs_sync,ip_vs_twos,ip_vs_wlc,ip_vs_wrr,ip_vs_xmit",
        subsystem: NetSubsystem::Ipvs,
    },
    LinuxNetSourceGroup {
        dir: "vendor/linux/net/unix",
        stems: "af_unix,diag,garbage,sysctl_net_unix,unix_bpf",
        subsystem: NetSubsystem::Unix,
    },
];

const IMPLEMENTED_SOURCES: &[&str] = &[
    "vendor/linux/drivers/net/virtio_net.c",
    "vendor/linux/net/socket.c",
    "vendor/linux/net/core/dev.c",
    "vendor/linux/net/core/neighbour.c",
    "vendor/linux/net/core/rtnetlink.c",
    "vendor/linux/net/core/skbuff.c",
    "vendor/linux/net/mpls/mpls_gso.c",
    "vendor/linux/net/ipv4/af_inet.c",
    "vendor/linux/net/ipv4/icmp.c",
    "vendor/linux/net/ipv4/route.c",
    "vendor/linux/net/ipv4/tcp.c",
    "vendor/linux/net/ipv4/tcp_cong.c",
    "vendor/linux/net/ipv4/tcp_cubic.c",
    "vendor/linux/net/ipv4/udp.c",
    "vendor/linux/net/ipv6/af_inet6.c",
    "vendor/linux/net/ipv6/icmp.c",
    "vendor/linux/net/ipv6/ioam6.c",
    "vendor/linux/net/ipv6/ioam6_iptunnel.c",
    "vendor/linux/net/ipv6/mip6.c",
    "vendor/linux/net/ipv6/ndisc.c",
    "vendor/linux/net/ipv6/route.c",
    "vendor/linux/net/ipv6/udp.c",
    "vendor/linux/net/netfilter/core.c",
    "vendor/linux/net/netfilter/nf_tables_api.c",
    "vendor/linux/net/netfilter/nf_tables_core.c",
    "vendor/linux/net/unix/af_unix.c",
];

pub fn source_count() -> usize {
    SOURCE_GROUPS
        .iter()
        .map(|group| csv_count(group.stems))
        .sum()
}

pub fn contains_linux_source(path: &str) -> bool {
    source_group(path).is_some()
}

pub fn source_policy(path: &'static str) -> LinuxNetSource {
    let subsystem = source_group(path)
        .map(|group| group.subsystem)
        .unwrap_or(NetSubsystem::Core);
    let status = if is_implemented(path) {
        SupportStatus::Implemented
    } else {
        SupportStatus::Unsupported
    };
    LinuxNetSource {
        path,
        subsystem,
        status,
        unsupported_errno: if status == SupportStatus::Unsupported {
            Some(unsupported_errno(path))
        } else {
            None
        },
    }
}

pub fn unsupported_errno(path: &str) -> i32 {
    if contains_linux_source(path) {
        EOPNOTSUPP
    } else {
        ENOENT
    }
}

pub fn all_sources_have_policy() -> Result<(), i32> {
    if source_count() != NETWORKING_SOURCE_COUNT {
        return Err(ENOENT);
    }
    for group in SOURCE_GROUPS {
        if group.dir.is_empty() || group.stems.is_empty() {
            return Err(ENOENT);
        }
    }
    Ok(())
}

fn source_group(path: &str) -> Option<&'static LinuxNetSourceGroup> {
    let (dir, file) = path.rsplit_once('/')?;
    let stem = file.strip_suffix(".c")?;
    SOURCE_GROUPS
        .iter()
        .find(|group| group.dir == dir && csv_contains(group.stems, stem))
}

fn is_implemented(path: &str) -> bool {
    IMPLEMENTED_SOURCES.iter().any(|source| *source == path)
}

fn csv_count(csv: &str) -> usize {
    if csv.is_empty() {
        return 0;
    }
    csv.split(',').count()
}

fn csv_contains(csv: &str, needle: &str) -> bool {
    csv.split(',').any(|item| item == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::include::uapi::errno::{ENOENT, EOPNOTSUPP};

    #[test]
    fn linux_network_source_inventory_is_complete() {
        assert_eq!(source_count(), NETWORKING_SOURCE_COUNT);
        assert!(contains_linux_source("vendor/linux/net/core/skbuff.c"));
        assert!(contains_linux_source("vendor/linux/net/mpls/mpls_gso.c"));
        assert!(contains_linux_source("vendor/linux/net/ipv4/tcp_cubic.c"));
        assert!(contains_linux_source(
            "vendor/linux/net/netfilter/ipvs/ip_vs_rr.c"
        ));
        assert!(contains_linux_source("vendor/linux/net/unix/af_unix.c"));
        assert_eq!(all_sources_have_policy(), Ok(()));
    }

    #[test]
    fn linux_network_source_policy_reports_real_support_state() {
        let supported = source_policy("vendor/linux/net/core/skbuff.c");
        assert_eq!(supported.status, SupportStatus::Implemented);
        assert_eq!(supported.unsupported_errno, None);

        let unsupported = source_policy("vendor/linux/net/netfilter/nft_nat.c");
        assert_eq!(unsupported.status, SupportStatus::Unsupported);
        assert_eq!(unsupported.unsupported_errno, Some(EOPNOTSUPP));
        let mpls_gso = source_policy("vendor/linux/net/mpls/mpls_gso.c");
        assert_eq!(mpls_gso.subsystem, NetSubsystem::Mpls);
        assert_eq!(mpls_gso.status, SupportStatus::Implemented);
        assert_eq!(mpls_gso.unsupported_errno, None);
        assert_eq!(
            source_policy("vendor/linux/net/ipv6/ioam6.c").status,
            SupportStatus::Implemented
        );
        assert_eq!(
            source_policy("vendor/linux/net/ipv6/ioam6_iptunnel.c").status,
            SupportStatus::Implemented
        );
        assert_eq!(
            source_policy("vendor/linux/net/ipv6/mip6.c").status,
            SupportStatus::Implemented
        );
        assert_eq!(unsupported_errno("vendor/linux/net/missing.c"), ENOENT);
    }
}
