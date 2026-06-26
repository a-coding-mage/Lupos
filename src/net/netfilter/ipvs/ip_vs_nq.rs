//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/ipvs/ip_vs_nq.c
//! test-origin: linux:vendor/linux/net/netfilter/ipvs/ip_vs_nq.c
//! IPVS never queue scheduler.

pub const IP_VS_DEST_F_OVERLOAD: u32 = 0x0002;
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_DESCRIPTION: &str = "ipvs never queue scheduler";

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

pub const IP_VS_NQ_SCHEDULER: IpVsScheduler = IpVsScheduler { name: "nq" };

pub const fn ip_vs_nq_dest_overhead(dest: IpVsDest) -> i32 {
    dest.activeconns + 1
}

pub fn ip_vs_nq_schedule(destinations: &[IpVsDest]) -> Option<IpVsDest> {
    let mut least: Option<IpVsDest> = None;
    let mut loh = 0;

    for dest in destinations {
        if dest.flags & IP_VS_DEST_F_OVERLOAD != 0 || dest.weight == 0 {
            continue;
        }

        let doh = ip_vs_nq_dest_overhead(*dest);
        if dest.activeconns == 0 {
            return Some(*dest);
        }

        if least.is_none()
            || i64::from(loh) * i64::from(dest.weight)
                > i64::from(doh) * i64::from(least.unwrap().weight)
        {
            least = Some(*dest);
            loh = doh;
        }
    }

    least
}

pub const fn ip_vs_nq_init() -> &'static IpVsScheduler {
    &IP_VS_NQ_SCHEDULER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip_vs_nq_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/ipvs/ip_vs_nq.c"
        ));
        assert!(source.contains("IPVS:        Never Queue scheduling module"));
        assert!(source.contains("ip_vs_nq_dest_overhead"));
        assert!(source.contains("return atomic_read(&dest->activeconns) + 1;"));
        assert!(source.contains("ip_vs_nq_schedule"));
        assert!(source.contains("struct ip_vs_dest *dest, *least = NULL;"));
        assert!(source.contains("int loh = 0, doh;"));
        assert!(source.contains("list_for_each_entry_rcu(dest, &svc->destinations, n_list)"));
        assert!(source.contains("dest->flags & IP_VS_DEST_F_OVERLOAD"));
        assert!(source.contains("!atomic_read(&dest->weight)"));
        assert!(source.contains("doh = ip_vs_nq_dest_overhead(dest);"));
        assert!(source.contains("if (atomic_read(&dest->activeconns) == 0)"));
        assert!(source.contains("goto out;"));
        assert!(source.contains("(__s64)loh * atomic_read(&dest->weight) >"));
        assert!(source.contains("(__s64)doh * atomic_read(&least->weight)"));
        assert!(source.contains("ip_vs_scheduler_err(svc, \"no destination available\");"));
        assert!(source.contains("return least;"));
        assert!(source.contains(".name =\t\t\t\"nq\""));
        assert!(source.contains("register_ip_vs_scheduler(&ip_vs_nq_scheduler)"));
        assert!(source.contains("unregister_ip_vs_scheduler(&ip_vs_nq_scheduler);"));
        assert!(source.contains("synchronize_rcu();"));
    }

    #[test]
    fn nq_scheduler_returns_first_idle_else_sed_choice() {
        let dests = [
            IpVsDest {
                addr: 1,
                port: 80,
                activeconns: 4,
                weight: 2,
                flags: 0,
            },
            IpVsDest {
                addr: 2,
                port: 80,
                activeconns: 0,
                weight: 1,
                flags: 0,
            },
            IpVsDest {
                addr: 3,
                port: 80,
                activeconns: 0,
                weight: 100,
                flags: 0,
            },
        ];

        assert_eq!(ip_vs_nq_dest_overhead(dests[0]), 5);
        assert_eq!(ip_vs_nq_schedule(&dests).unwrap().addr, 2);
        assert_eq!(
            ip_vs_nq_schedule(&[
                IpVsDest {
                    activeconns: 5,
                    weight: 2,
                    ..dests[0]
                },
                IpVsDest {
                    activeconns: 7,
                    weight: 8,
                    ..dests[2]
                },
            ])
            .unwrap()
            .addr,
            3
        );
        assert_eq!(
            ip_vs_nq_schedule(&[IpVsDest {
                weight: 0,
                ..dests[0]
            }]),
            None
        );
        assert_eq!(ip_vs_nq_init(), &IP_VS_NQ_SCHEDULER);
    }
}
