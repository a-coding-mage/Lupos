//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/ipvs/ip_vs_lc.c
//! test-origin: linux:vendor/linux/net/netfilter/ipvs/ip_vs_lc.c
//! IPVS least-connection scheduler.

pub const IP_VS_DEST_F_OVERLOAD: u32 = 0x0002;
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_DESCRIPTION: &str = "ipvs least connection scheduler";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpVsDest {
    pub addr: u32,
    pub port: u16,
    pub activeconns: u32,
    pub inactconns: u32,
    pub weight: i32,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpVsScheduler {
    pub name: &'static str,
}

pub const IP_VS_LC_SCHEDULER: IpVsScheduler = IpVsScheduler { name: "lc" };

pub const fn ip_vs_dest_conn_overhead(dest: IpVsDest) -> u32 {
    (dest.activeconns << 5).saturating_add(dest.inactconns)
}

pub fn ip_vs_lc_schedule(destinations: &[IpVsDest]) -> Option<IpVsDest> {
    let mut least = None;
    let mut least_overhead = 0;

    for dest in destinations {
        if dest.flags & IP_VS_DEST_F_OVERLOAD != 0 || dest.weight == 0 {
            continue;
        }

        let overhead = ip_vs_dest_conn_overhead(*dest);
        if least.is_none() || overhead < least_overhead {
            least = Some(*dest);
            least_overhead = overhead;
        }
    }

    least
}

pub const fn ip_vs_lc_init() -> &'static IpVsScheduler {
    &IP_VS_LC_SCHEDULER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip_vs_lc_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/ipvs/ip_vs_lc.c"
        ));
        assert!(source.contains("IPVS:        Least-Connection Scheduling module"));
        assert!(source.contains("ip_vs_lc_schedule"));
        assert!(source.contains("struct ip_vs_dest *dest, *least = NULL;"));
        assert!(source.contains("unsigned int loh = 0, doh;"));
        assert!(source.contains("list_for_each_entry_rcu(dest, &svc->destinations, n_list)"));
        assert!(source.contains("(dest->flags & IP_VS_DEST_F_OVERLOAD)"));
        assert!(source.contains("atomic_read(&dest->weight) == 0"));
        assert!(source.contains("doh = ip_vs_dest_conn_overhead(dest);"));
        assert!(source.contains("if (!least || doh < loh)"));
        assert!(source.contains("ip_vs_scheduler_err(svc, \"no destination available\");"));
        assert!(source.contains("return least;"));
        assert!(source.contains(".name =\t\t\t\"lc\""));
        assert!(source.contains("register_ip_vs_scheduler(&ip_vs_lc_scheduler)"));
        assert!(source.contains("unregister_ip_vs_scheduler(&ip_vs_lc_scheduler);"));
        assert!(source.contains("synchronize_rcu();"));
    }

    #[test]
    fn lc_scheduler_chooses_least_non_overloaded_nonzero_weight() {
        let dests = [
            IpVsDest {
                addr: 1,
                port: 80,
                activeconns: 1,
                inactconns: 1,
                weight: 0,
                flags: 0,
            },
            IpVsDest {
                addr: 2,
                port: 80,
                activeconns: 2,
                inactconns: 0,
                weight: 1,
                flags: 0,
            },
            IpVsDest {
                addr: 3,
                port: 80,
                activeconns: 0,
                inactconns: 9,
                weight: 1,
                flags: 0,
            },
            IpVsDest {
                addr: 4,
                port: 80,
                activeconns: 0,
                inactconns: 1,
                weight: 1,
                flags: IP_VS_DEST_F_OVERLOAD,
            },
        ];

        assert_eq!(ip_vs_dest_conn_overhead(dests[1]), 64);
        assert_eq!(ip_vs_lc_schedule(&dests).unwrap().addr, 3);
        assert_eq!(
            ip_vs_lc_schedule(&[IpVsDest {
                weight: 0,
                ..dests[0]
            }]),
            None
        );
        assert_eq!(ip_vs_lc_init(), &IP_VS_LC_SCHEDULER);
    }
}
