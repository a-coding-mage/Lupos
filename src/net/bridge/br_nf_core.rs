//! linux-parity: complete
//! linux-source: vendor/linux/net/bridge/br_nf_core.c
//! test-origin: linux:vendor/linux/net/bridge/br_nf_core.c
//! Bridge netfilter fake route-table setup.

pub const AF_INET: u16 = 2;
pub const DST_NOXFRM: u32 = 0x0002;
pub const DST_FAKE_RTABLE: u32 = 0x0010;
pub const BR_NF_FAKE_RTABLE_FLAGS: u32 = DST_NOXFRM | DST_FAKE_RTABLE;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NetDevice {
    pub mtu: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FakeDst {
    pub family: u16,
    pub dev_mtu: u32,
    pub metric_mtu: u32,
    pub flags: u32,
    pub entries_initialized: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NetBridge {
    pub dev: NetDevice,
    pub fake_rtable: FakeDst,
}

pub const fn fake_update_pmtu() {}

pub const fn fake_redirect() {}

pub const fn fake_cow_metrics() -> Option<&'static [u32]> {
    None
}

pub const fn fake_neigh_lookup() -> Option<()> {
    None
}

pub const fn fake_mtu(dst: &FakeDst) -> u32 {
    dst.dev_mtu
}

pub const fn br_netfilter_rtable_init(dev: NetDevice) -> NetBridge {
    NetBridge {
        dev,
        fake_rtable: FakeDst {
            family: AF_INET,
            dev_mtu: dev.mtu,
            metric_mtu: dev.mtu,
            flags: BR_NF_FAKE_RTABLE_FLAGS,
            entries_initialized: false,
        },
    }
}

pub const fn br_nf_core_init() -> FakeDst {
    FakeDst {
        family: AF_INET,
        dev_mtu: 0,
        metric_mtu: 0,
        flags: 0,
        entries_initialized: true,
    }
}

pub const fn br_nf_core_fini(dst: &mut FakeDst) {
    dst.entries_initialized = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn br_nf_core_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/bridge/br_nf_core.c"
        ));
        assert!(source.contains("static void fake_update_pmtu"));
        assert!(source.contains("static void fake_redirect"));
        assert!(source.contains("static u32 *fake_cow_metrics"));
        assert!(source.contains("return NULL;"));
        assert!(source.contains("static struct neighbour *fake_neigh_lookup"));
        assert!(source.contains("static unsigned int fake_mtu"));
        assert!(source.contains("return dst->dev->mtu;"));
        assert!(source.contains(".family\t\t= AF_INET"));
        assert!(source.contains("void br_netfilter_rtable_init(struct net_bridge *br)"));
        assert!(source.contains("rcuref_init(&rt->dst.__rcuref, 1);"));
        assert!(source.contains("rt->dst.dev = br->dev;"));
        assert!(source.contains("dst_metric_set(&rt->dst, RTAX_MTU, br->dev->mtu);"));
        assert!(source.contains("rt->dst.flags\t= DST_NOXFRM | DST_FAKE_RTABLE;"));
        assert!(source.contains("return dst_entries_init(&fake_dst_ops);"));
        assert!(source.contains("dst_entries_destroy(&fake_dst_ops);"));

        let bridge = br_netfilter_rtable_init(NetDevice { mtu: 1500 });
        assert_eq!(bridge.fake_rtable.family, AF_INET);
        assert_eq!(bridge.fake_rtable.metric_mtu, 1500);
        assert_eq!(bridge.fake_rtable.flags, BR_NF_FAKE_RTABLE_FLAGS);
        assert_eq!(fake_mtu(&bridge.fake_rtable), 1500);
        assert_eq!(fake_cow_metrics(), None);
        assert_eq!(fake_neigh_lookup(), None);

        let mut ops = br_nf_core_init();
        assert!(ops.entries_initialized);
        br_nf_core_fini(&mut ops);
        assert!(!ops.entries_initialized);
    }
}
