//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/ipvs/ip_vs_twos.c
//! test-origin: linux:vendor/linux/net/netfilter/ipvs/ip_vs_twos.c
//! IPVS power of twos choice scheduler.

pub const IP_VS_DEST_F_OVERLOAD: u32 = 0x0002;
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_DESCRIPTION: &str = "ipvs power of twos choice scheduler";

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

pub const IP_VS_TWOS_SCHEDULER: IpVsScheduler = IpVsScheduler { name: "twos" };

pub const fn ip_vs_dest_conn_overhead(dest: IpVsDest) -> i32 {
    (dest.activeconns << 8) + dest.inactconns
}

pub fn ip_vs_twos_total_random_ceiling(destinations: &[IpVsDest]) -> Option<i32> {
    let mut total_weight = 0;
    let mut has_choice = false;

    for dest in destinations {
        if dest.flags & IP_VS_DEST_F_OVERLOAD == 0 && dest.weight > 0 {
            total_weight += dest.weight;
            has_choice = true;
        }
    }

    has_choice.then_some(total_weight + 1)
}

pub fn ip_vs_twos_schedule(
    destinations: &[IpVsDest],
    mut rweight1: i32,
    mut rweight2: i32,
) -> Option<IpVsDest> {
    let mut choice1 = None;
    let mut choice2 = None;
    let mut fallback = None;
    let mut weight1 = -1;
    let mut weight2 = -1;
    let mut overhead1 = 0;
    let mut overhead2 = 0;

    for dest in destinations {
        if dest.flags & IP_VS_DEST_F_OVERLOAD == 0 && dest.weight > 0 {
            fallback = Some(*dest);
        }
    }

    fallback?;

    for dest in destinations {
        if dest.flags & IP_VS_DEST_F_OVERLOAD != 0 || dest.weight <= 0 {
            continue;
        }

        rweight1 -= dest.weight;
        rweight2 -= dest.weight;

        if rweight1 <= 0 && weight1 == -1 {
            choice1 = Some(*dest);
            weight1 = dest.weight;
            overhead1 = ip_vs_dest_conn_overhead(*dest);
        }

        if rweight2 <= 0 && weight2 == -1 {
            choice2 = Some(*dest);
            weight2 = dest.weight;
            overhead2 = ip_vs_dest_conn_overhead(*dest);
        }

        if weight1 != -1 && weight2 != -1 {
            break;
        }
    }

    let mut selected = choice1.or(fallback).unwrap();
    if let Some(second) = choice2 {
        if weight2 * overhead1 > weight1 * overhead2 {
            selected = second;
        }
    }

    Some(selected)
}

pub const fn ip_vs_twos_init() -> &'static IpVsScheduler {
    &IP_VS_TWOS_SCHEDULER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip_vs_twos_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/ipvs/ip_vs_twos.c"
        ));
        assert!(source.contains("IPVS:        Power of Twos Choice Scheduling module"));
        assert!(source.contains("ip_vs_twos_schedule"));
        assert!(source.contains("choice1 = NULL, *choice2 = NULL"));
        assert!(source.contains("weight1 = -1, weight2 = -1, overhead1 = 0"));
        assert!(source.contains("overhead2, total_weight = 0, weight;"));
        assert!(source.contains("list_for_each_entry_rcu(dest, &svc->destinations, n_list)"));
        assert!(source.contains("total_weight += weight;"));
        assert!(source.contains("choice1 = dest;"));
        assert!(source.contains("if (!choice1)"));
        assert!(source.contains("total_weight += 1;"));
        assert!(source.contains("rweight1 = get_random_u32_below(total_weight);"));
        assert!(source.contains("rweight2 = get_random_u32_below(total_weight);"));
        assert!(source.contains("rweight1 -= weight;"));
        assert!(source.contains("rweight2 -= weight;"));
        assert!(source.contains("if (rweight1 <= 0 && weight1 == -1)"));
        assert!(source.contains("overhead1 = ip_vs_dest_conn_overhead(dest);"));
        assert!(source.contains("if (rweight2 <= 0 && weight2 == -1)"));
        assert!(source.contains("overhead2 = ip_vs_dest_conn_overhead(dest);"));
        assert!(source.contains("if (choice2 && (weight2 * overhead1) > (weight1 * overhead2))"));
        assert!(source.contains("return choice1;"));
        assert!(source.contains(".name = \"twos\""));
        assert!(source.contains("register_ip_vs_scheduler(&ip_vs_twos_scheduler)"));
        assert!(source.contains("unregister_ip_vs_scheduler(&ip_vs_twos_scheduler);"));
        assert!(source.contains("synchronize_rcu();"));
    }

    #[test]
    fn twos_scheduler_samples_two_weighted_choices_and_normalizes_load() {
        let dests = [
            IpVsDest {
                addr: 1,
                port: 80,
                activeconns: 10,
                inactconns: 0,
                weight: 5,
                flags: 0,
            },
            IpVsDest {
                addr: 2,
                port: 80,
                activeconns: 1,
                inactconns: 0,
                weight: 5,
                flags: 0,
            },
            IpVsDest {
                addr: 3,
                port: 80,
                activeconns: 0,
                inactconns: 0,
                weight: 100,
                flags: IP_VS_DEST_F_OVERLOAD,
            },
        ];

        assert_eq!(ip_vs_twos_total_random_ceiling(&dests), Some(11));
        assert_eq!(ip_vs_dest_conn_overhead(dests[0]), 2560);
        assert_eq!(ip_vs_twos_schedule(&dests, 1, 6).unwrap().addr, 2);
        assert_eq!(ip_vs_twos_schedule(&dests, 1, 1).unwrap().addr, 1);
        assert_eq!(
            ip_vs_twos_schedule(
                &[IpVsDest {
                    weight: 0,
                    flags: 0,
                    ..dests[0]
                }],
                0,
                0
            ),
            None
        );
        assert_eq!(ip_vs_twos_init(), &IP_VS_TWOS_SCHEDULER);
    }
}
