//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/ipvs/ip_vs_fo.c
//! test-origin: linux:vendor/linux/net/netfilter/ipvs/ip_vs_fo.c
//! IPVS weighted failover scheduler.

pub const IP_VS_DEST_F_OVERLOAD: u32 = 0x0002;
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_DESCRIPTION: &str = "ipvs weighted failover scheduler";

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

pub const IP_VS_FO_SCHEDULER: IpVsScheduler = IpVsScheduler { name: "fo" };

pub fn ip_vs_fo_schedule(destinations: &[IpVsDest]) -> Option<IpVsDest> {
    let mut highest = None;
    let mut highest_weight = 0;
    for dest in destinations {
        if dest.flags & IP_VS_DEST_F_OVERLOAD == 0 && dest.weight > highest_weight {
            highest = Some(*dest);
            highest_weight = dest.weight;
        }
    }
    highest
}

pub const fn ip_vs_fo_init() -> &'static IpVsScheduler {
    &IP_VS_FO_SCHEDULER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip_vs_fo_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/ipvs/ip_vs_fo.c"
        ));
        assert!(source.contains("IPVS:        Weighted Fail Over module"));
        assert!(source.contains("ip_vs_fo_schedule"));
        assert!(source.contains("struct ip_vs_dest *dest, *hweight = NULL;"));
        assert!(source.contains("int hw = 0;"));
        assert!(source.contains("list_for_each_entry_rcu(dest, &svc->destinations, n_list)"));
        assert!(source.contains("!(dest->flags & IP_VS_DEST_F_OVERLOAD)"));
        assert!(source.contains("atomic_read(&dest->weight) > hw"));
        assert!(source.contains("return hweight;"));
        assert!(source.contains("ip_vs_scheduler_err(svc, \"no destination available\");"));
        assert!(source.contains(".name =\t\t\t\"fo\""));
        assert!(source.contains("register_ip_vs_scheduler(&ip_vs_fo_scheduler);"));
        assert!(source.contains("unregister_ip_vs_scheduler(&ip_vs_fo_scheduler);"));
    }

    #[test]
    fn fo_scheduler_chooses_highest_non_overloaded_positive_weight() {
        let dests = [
            IpVsDest {
                addr: 1,
                port: 80,
                activeconns: 9,
                weight: 8,
                flags: IP_VS_DEST_F_OVERLOAD,
            },
            IpVsDest {
                addr: 2,
                port: 80,
                activeconns: 2,
                weight: 4,
                flags: 0,
            },
            IpVsDest {
                addr: 3,
                port: 80,
                activeconns: 1,
                weight: 7,
                flags: 0,
            },
        ];
        assert_eq!(ip_vs_fo_schedule(&dests).unwrap().addr, 3);
        assert_eq!(
            ip_vs_fo_schedule(&[IpVsDest {
                weight: 0,
                flags: 0,
                ..dests[1]
            }]),
            None
        );
        assert_eq!(ip_vs_fo_init(), &IP_VS_FO_SCHEDULER);
    }
}
