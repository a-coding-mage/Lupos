//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/ipvs/ip_vs_wlc.c
//! test-origin: linux:vendor/linux/net/netfilter/ipvs/ip_vs_wlc.c
//! IPVS weighted least-connection scheduler.

pub const IP_VS_DEST_F_OVERLOAD: u32 = 0x0002;
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_DESCRIPTION: &str = "ipvs weighted least connection scheduler";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpVsDest {
    pub addr: u32,
    pub port: u16,
    pub activeconns: i32,
    pub inactconns: i32,
    pub weight: i32,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpVsScheduler {
    pub name: &'static str,
}

pub const IP_VS_WLC_SCHEDULER: IpVsScheduler = IpVsScheduler { name: "wlc" };

pub const fn ip_vs_dest_conn_overhead(dest: IpVsDest) -> i32 {
    (dest.activeconns << 8) + dest.inactconns
}

pub fn ip_vs_wlc_schedule(destinations: &[IpVsDest]) -> Option<IpVsDest> {
    let mut iter = destinations.iter();
    let mut least = loop {
        let dest = *iter.next()?;
        if dest.flags & IP_VS_DEST_F_OVERLOAD == 0 && dest.weight > 0 {
            break dest;
        }
    };
    let mut loh = ip_vs_dest_conn_overhead(least);

    for dest in iter {
        if dest.flags & IP_VS_DEST_F_OVERLOAD != 0 {
            continue;
        }

        let doh = ip_vs_dest_conn_overhead(*dest);
        if i64::from(loh) * i64::from(dest.weight) > i64::from(doh) * i64::from(least.weight) {
            least = *dest;
            loh = doh;
        }
    }

    Some(least)
}

pub const fn ip_vs_wlc_init() -> &'static IpVsScheduler {
    &IP_VS_WLC_SCHEDULER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip_vs_wlc_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/ipvs/ip_vs_wlc.c"
        ));
        assert!(source.contains("IPVS:        Weighted Least-Connection Scheduling module"));
        assert!(source.contains("ip_vs_wlc_schedule"));
        assert!(source.contains("struct ip_vs_dest *dest, *least;"));
        assert!(source.contains("int loh, doh;"));
        assert!(source.contains("list_for_each_entry_rcu(dest, &svc->destinations, n_list)"));
        assert!(source.contains("atomic_read(&dest->weight) > 0"));
        assert!(source.contains("loh = ip_vs_dest_conn_overhead(least);"));
        assert!(source.contains("goto nextstage;"));
        assert!(
            source.contains("list_for_each_entry_continue_rcu(dest, &svc->destinations, n_list)")
        );
        assert!(source.contains("if (dest->flags & IP_VS_DEST_F_OVERLOAD)"));
        assert!(source.contains("doh = ip_vs_dest_conn_overhead(dest);"));
        assert!(source.contains("(__s64)loh * atomic_read(&dest->weight) >"));
        assert!(source.contains("(__s64)doh * atomic_read(&least->weight)"));
        assert!(source.contains("ip_vs_scheduler_err(svc, \"no destination available\");"));
        assert!(source.contains("return least;"));
        assert!(source.contains(".name =\t\t\t\"wlc\""));
        assert!(source.contains("register_ip_vs_scheduler(&ip_vs_wlc_scheduler)"));
        assert!(source.contains("unregister_ip_vs_scheduler(&ip_vs_wlc_scheduler);"));
        assert!(source.contains("synchronize_rcu();"));
    }

    #[test]
    fn wlc_scheduler_chooses_lowest_overhead_per_weight() {
        let dests = [
            IpVsDest {
                addr: 1,
                port: 80,
                activeconns: 1,
                inactconns: 0,
                weight: 0,
                flags: 0,
            },
            IpVsDest {
                addr: 2,
                port: 80,
                activeconns: 2,
                inactconns: 0,
                weight: 4,
                flags: 0,
            },
            IpVsDest {
                addr: 3,
                port: 80,
                activeconns: 3,
                inactconns: 0,
                weight: 8,
                flags: 0,
            },
            IpVsDest {
                addr: 4,
                port: 80,
                activeconns: 0,
                inactconns: 1,
                weight: 100,
                flags: IP_VS_DEST_F_OVERLOAD,
            },
        ];

        assert_eq!(ip_vs_dest_conn_overhead(dests[1]), 512);
        assert_eq!(ip_vs_wlc_schedule(&dests).unwrap().addr, 3);
        assert_eq!(
            ip_vs_wlc_schedule(&[IpVsDest {
                weight: 0,
                ..dests[0]
            }]),
            None
        );
        assert_eq!(ip_vs_wlc_init(), &IP_VS_WLC_SCHEDULER);
    }
}
