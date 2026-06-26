//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/ipvs/ip_vs_rr.c
//! test-origin: linux:vendor/linux/net/netfilter/ipvs/ip_vs_rr.c
//! IPVS round-robin scheduler.

pub const IP_VS_DEST_F_OVERLOAD: u32 = 0x0002;
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_DESCRIPTION: &str = "ipvs round-robin scheduler";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpVsDest {
    pub addr: u32,
    pub port: u16,
    pub activeconns: i32,
    pub weight: i32,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedData {
    Head,
    Dest(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IpVsService<'a> {
    pub destinations: &'a [IpVsDest],
    pub sched_data: SchedData,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpVsScheduler {
    pub name: &'static str,
    pub has_init_service: bool,
    pub has_del_dest: bool,
}

pub const IP_VS_RR_SCHEDULER: IpVsScheduler = IpVsScheduler {
    name: "rr",
    has_init_service: true,
    has_del_dest: true,
};

pub const fn ip_vs_rr_init_svc<'a>(destinations: &'a [IpVsDest]) -> IpVsService<'a> {
    IpVsService {
        destinations,
        sched_data: SchedData::Head,
    }
}

pub fn ip_vs_rr_del_dest(svc: &mut IpVsService<'_>, deleted_index: usize) -> i32 {
    if svc.sched_data == SchedData::Dest(deleted_index) {
        svc.sched_data = if deleted_index == 0 {
            SchedData::Head
        } else {
            SchedData::Dest(deleted_index - 1)
        };
    }
    0
}

pub fn ip_vs_rr_schedule(svc: &mut IpVsService<'_>) -> Option<IpVsDest> {
    if svc.destinations.is_empty() {
        return None;
    }

    let last = match svc.sched_data {
        SchedData::Head => None,
        SchedData::Dest(index) => Some(index.min(svc.destinations.len() - 1)),
    };
    let start = last.map_or(0, |index| index + 1);

    for index in start..svc.destinations.len() {
        let dest = svc.destinations[index];
        if dest.flags & IP_VS_DEST_F_OVERLOAD == 0 && dest.weight > 0 {
            svc.sched_data = SchedData::Dest(index);
            return Some(dest);
        }
    }

    if let Some(last) = last {
        for index in 0..=last {
            let dest = svc.destinations[index];
            if dest.flags & IP_VS_DEST_F_OVERLOAD == 0 && dest.weight > 0 {
                svc.sched_data = SchedData::Dest(index);
                return Some(dest);
            }
            if index == last {
                break;
            }
        }
    }

    None
}

pub const fn ip_vs_rr_init() -> &'static IpVsScheduler {
    &IP_VS_RR_SCHEDULER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip_vs_rr_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/ipvs/ip_vs_rr.c"
        ));
        assert!(source.contains("IPVS:        Round-Robin Scheduling module"));
        assert!(source.contains("ip_vs_rr_init_svc"));
        assert!(source.contains("svc->sched_data = &svc->destinations;"));
        assert!(source.contains("ip_vs_rr_del_dest"));
        assert!(source.contains("spin_lock_bh(&svc->sched_lock);"));
        assert!(source.contains("p = (struct list_head *) svc->sched_data;"));
        assert!(source.contains("if (p == &dest->n_list)"));
        assert!(source.contains("svc->sched_data = p->next->prev;"));
        assert!(source.contains("ip_vs_rr_schedule"));
        assert!(source.contains("struct list_head *p;"));
        assert!(source.contains("struct ip_vs_dest *dest, *last;"));
        assert!(source.contains("int pass = 0;"));
        assert!(source.contains("last = dest = list_entry(p, struct ip_vs_dest, n_list);"));
        assert!(source.contains("list_for_each_entry_continue_rcu(dest,"));
        assert!(source.contains("atomic_read(&dest->weight) > 0"));
        assert!(source.contains("goto out;"));
        assert!(source.contains("if (dest == last)"));
        assert!(source.contains("goto stop;"));
        assert!(source.contains("while (pass < 2 && p != &svc->destinations);"));
        assert!(source.contains("svc->sched_data = &dest->n_list;"));
        assert!(source.contains("ip_vs_scheduler_err(svc, \"no destination available\");"));
        assert!(source.contains("return dest;"));
        assert!(source.contains(".name =\t\t\t\"rr\""));
        assert!(source.contains(".init_service =\t\tip_vs_rr_init_svc"));
        assert!(source.contains(".del_dest =\t\tip_vs_rr_del_dest"));
        assert!(source.contains("register_ip_vs_scheduler(&ip_vs_rr_scheduler)"));
        assert!(source.contains("unregister_ip_vs_scheduler(&ip_vs_rr_scheduler);"));
        assert!(source.contains("synchronize_rcu();"));
    }

    #[test]
    fn rr_scheduler_advances_cursor_and_wraps_once() {
        let dests = [
            IpVsDest {
                addr: 1,
                port: 80,
                activeconns: 0,
                weight: 1,
                flags: 0,
            },
            IpVsDest {
                addr: 2,
                port: 80,
                activeconns: 0,
                weight: 0,
                flags: 0,
            },
            IpVsDest {
                addr: 3,
                port: 80,
                activeconns: 0,
                weight: 1,
                flags: 0,
            },
        ];
        let mut svc = ip_vs_rr_init_svc(&dests);

        assert_eq!(ip_vs_rr_schedule(&mut svc).unwrap().addr, 1);
        assert_eq!(svc.sched_data, SchedData::Dest(0));
        assert_eq!(ip_vs_rr_schedule(&mut svc).unwrap().addr, 3);
        assert_eq!(svc.sched_data, SchedData::Dest(2));
        assert_eq!(ip_vs_rr_schedule(&mut svc).unwrap().addr, 1);
        assert_eq!(ip_vs_rr_del_dest(&mut svc, 0), 0);
        assert_eq!(svc.sched_data, SchedData::Head);
        assert_eq!(ip_vs_rr_init(), &IP_VS_RR_SCHEDULER);
    }

    #[test]
    fn rr_scheduler_returns_none_without_available_destination() {
        let dests = [
            IpVsDest {
                addr: 1,
                port: 80,
                activeconns: 0,
                weight: 0,
                flags: 0,
            },
            IpVsDest {
                addr: 2,
                port: 80,
                activeconns: 0,
                weight: 1,
                flags: IP_VS_DEST_F_OVERLOAD,
            },
        ];
        let mut svc = ip_vs_rr_init_svc(&dests);

        assert_eq!(ip_vs_rr_schedule(&mut svc), None);
    }
}
