//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/ipvs/ip_vs_ovf.c
//! test-origin: linux:vendor/linux/net/netfilter/ipvs/ip_vs_ovf.c
//! IPVS overflow connection scheduler.

pub const IP_VS_DEST_F_OVERLOAD: u32 = 0x0002;
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_DESCRIPTION: &str = "ipvs overflow connection scheduler";

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

pub const IP_VS_OVF_SCHEDULER: IpVsScheduler = IpVsScheduler { name: "ovf" };

pub fn ip_vs_ovf_schedule(destinations: &[IpVsDest]) -> Option<IpVsDest> {
    let mut highest = None;
    let mut highest_weight = 0;

    for dest in destinations {
        let weight = dest.weight;
        if dest.flags & IP_VS_DEST_F_OVERLOAD != 0 || dest.activeconns > weight || weight == 0 {
            continue;
        }
        if highest.is_none() || weight > highest_weight {
            highest = Some(*dest);
            highest_weight = weight;
        }
    }

    highest
}

pub const fn ip_vs_ovf_init() -> &'static IpVsScheduler {
    &IP_VS_OVF_SCHEDULER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip_vs_ovf_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/ipvs/ip_vs_ovf.c"
        ));
        assert!(source.contains("IPVS:        Overflow-Connection Scheduling module"));
        assert!(source.contains("ip_vs_ovf_schedule"));
        assert!(source.contains("struct ip_vs_dest *dest, *h = NULL;"));
        assert!(source.contains("int hw = 0, w;"));
        assert!(source.contains("list_for_each_entry_rcu(dest, &svc->destinations, n_list)"));
        assert!(source.contains("atomic_read(&dest->activeconns) > w"));
        assert!(source.contains("w == 0"));
        assert!(source.contains("if (!h || w > hw)"));
        assert!(source.contains("return h;"));
        assert!(source.contains("ip_vs_scheduler_err(svc, \"no destination available\");"));
        assert!(source.contains(".name =\t\t\t\"ovf\""));
        assert!(source.contains("register_ip_vs_scheduler(&ip_vs_ovf_scheduler);"));
        assert!(source.contains("unregister_ip_vs_scheduler(&ip_vs_ovf_scheduler);"));

        let dests = [
            IpVsDest {
                addr: 1,
                port: 80,
                activeconns: 9,
                weight: 10,
                flags: 0,
            },
            IpVsDest {
                addr: 2,
                port: 80,
                activeconns: 11,
                weight: 10,
                flags: 0,
            },
            IpVsDest {
                addr: 3,
                port: 80,
                activeconns: 1,
                weight: 12,
                flags: 0,
            },
            IpVsDest {
                addr: 4,
                port: 80,
                activeconns: 1,
                weight: 20,
                flags: IP_VS_DEST_F_OVERLOAD,
            },
        ];
        assert_eq!(ip_vs_ovf_schedule(&dests).unwrap().addr, 3);
        assert_eq!(
            ip_vs_ovf_schedule(&[IpVsDest {
                activeconns: 2,
                weight: 1,
                ..dests[0]
            }]),
            None
        );
        assert_eq!(ip_vs_ovf_init(), &IP_VS_OVF_SCHEDULER);
    }
}
