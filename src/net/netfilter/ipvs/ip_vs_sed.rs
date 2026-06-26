//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/ipvs/ip_vs_sed.c
//! test-origin: linux:vendor/linux/net/netfilter/ipvs/ip_vs_sed.c
//! IPVS shortest expected delay scheduler.

pub const IP_VS_DEST_F_OVERLOAD: u32 = 0x0002;
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_DESCRIPTION: &str = "ipvs shortest expected delay scheduler";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpVsDest {
    pub addr: u32,
    pub port: u16,
    pub activeconns: i32,
    pub weight: i32,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpVsScheduler {
    pub name: &'static str,
}

pub const IP_VS_SED_SCHEDULER: IpVsScheduler = IpVsScheduler { name: "sed" };

pub const fn ip_vs_sed_dest_overhead(dest: IpVsDest) -> i32 {
    dest.activeconns + 1
}

pub fn ip_vs_sed_schedule(destinations: &[IpVsDest]) -> Option<IpVsDest> {
    let mut iter = destinations.iter();
    let mut least = loop {
        let dest = *iter.next()?;
        if dest.flags & IP_VS_DEST_F_OVERLOAD == 0 && dest.weight > 0 {
            break dest;
        }
    };
    let mut loh = ip_vs_sed_dest_overhead(least);

    for dest in iter {
        if dest.flags & IP_VS_DEST_F_OVERLOAD != 0 {
            continue;
        }

        let doh = ip_vs_sed_dest_overhead(*dest);
        if i64::from(loh) * i64::from(dest.weight) > i64::from(doh) * i64::from(least.weight) {
            least = *dest;
            loh = doh;
        }
    }

    Some(least)
}

pub const fn ip_vs_sed_init() -> &'static IpVsScheduler {
    &IP_VS_SED_SCHEDULER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip_vs_sed_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/ipvs/ip_vs_sed.c"
        ));
        assert!(source.contains("IPVS:        Shortest Expected Delay scheduling module"));
        assert!(source.contains("ip_vs_sed_dest_overhead"));
        assert!(source.contains("return atomic_read(&dest->activeconns) + 1;"));
        assert!(source.contains("ip_vs_sed_schedule"));
        assert!(source.contains("struct ip_vs_dest *dest, *least;"));
        assert!(source.contains("int loh, doh;"));
        assert!(source.contains("list_for_each_entry_rcu(dest, &svc->destinations, n_list)"));
        assert!(source.contains("atomic_read(&dest->weight) > 0"));
        assert!(source.contains("loh = ip_vs_sed_dest_overhead(least);"));
        assert!(source.contains("goto nextstage;"));
        assert!(
            source.contains("list_for_each_entry_continue_rcu(dest, &svc->destinations, n_list)")
        );
        assert!(source.contains("if (dest->flags & IP_VS_DEST_F_OVERLOAD)"));
        assert!(source.contains("doh = ip_vs_sed_dest_overhead(dest);"));
        assert!(source.contains("(__s64)loh * atomic_read(&dest->weight) >"));
        assert!(source.contains("(__s64)doh * atomic_read(&least->weight)"));
        assert!(source.contains("ip_vs_scheduler_err(svc, \"no destination available\");"));
        assert!(source.contains("return least;"));
        assert!(source.contains(".name =\t\t\t\"sed\""));
        assert!(source.contains("register_ip_vs_scheduler(&ip_vs_sed_scheduler)"));
        assert!(source.contains("unregister_ip_vs_scheduler(&ip_vs_sed_scheduler);"));
        assert!(source.contains("synchronize_rcu();"));
    }

    #[test]
    fn sed_scheduler_chooses_shortest_expected_delay() {
        let dests = [
            IpVsDest {
                addr: 1,
                port: 80,
                activeconns: 7,
                weight: 0,
                flags: 0,
            },
            IpVsDest {
                addr: 2,
                port: 80,
                activeconns: 3,
                weight: 2,
                flags: 0,
            },
            IpVsDest {
                addr: 3,
                port: 80,
                activeconns: 7,
                weight: 8,
                flags: 0,
            },
            IpVsDest {
                addr: 4,
                port: 80,
                activeconns: 0,
                weight: 100,
                flags: IP_VS_DEST_F_OVERLOAD,
            },
        ];

        assert_eq!(ip_vs_sed_dest_overhead(dests[1]), 4);
        assert_eq!(ip_vs_sed_schedule(&dests).unwrap().addr, 3);
        assert_eq!(
            ip_vs_sed_schedule(&[IpVsDest {
                weight: 0,
                ..dests[0]
            }]),
            None
        );
        assert_eq!(ip_vs_sed_init(), &IP_VS_SED_SCHEDULER);
    }
}
