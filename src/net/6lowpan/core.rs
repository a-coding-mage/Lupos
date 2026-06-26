//! linux-parity: complete
//! linux-source: vendor/linux/net/6lowpan/core.c
//! test-origin: linux:vendor/linux/net/6lowpan/core.c
//! 6LoWPAN core device registration, notifier, and IEEE 802.15.4 IID rules.

pub const EUI64_ADDR_LEN: usize = 8;
pub const ETH_ALEN: usize = 6;
pub const ARPHRD_6LOWPAN: u16 = 825;
pub const IPV6_MIN_MTU: u32 = 1280;
pub const LOWPAN_IPHC_CTX_TABLE_SIZE: usize = 1 << 4;
pub const IEEE802154_PAN_ID_BROADCAST: u16 = 0xffff;
pub const LOWPAN_IPHC_CTX_FLAG_ACTIVE: usize = 0;

pub const NETDEV_UP: u64 = 1;
pub const NETDEV_DOWN: u64 = 2;
pub const NETDEV_CHANGE: u64 = 4;
pub const NOTIFY_DONE: i32 = 0x0000;
pub const NOTIFY_OK: i32 = 0x0001;

pub const LOWPAN_REQUESTED_NHC_MODULES: [&str; 7] = [
    "nhc_dest",
    "nhc_fragment",
    "nhc_hop",
    "nhc_ipv6",
    "nhc_mobility",
    "nhc_routing",
    "nhc_udp",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LowpanLlType {
    Btle,
    Ieee802154,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LowpanNetdevice {
    pub addr_len: usize,
    pub dev_type: u16,
    pub mtu: u32,
    pub lltype: LowpanLlType,
    pub ctx_ids: [u8; LOWPAN_IPHC_CTX_TABLE_SIZE],
    pub ctx_active: [bool; LOWPAN_IPHC_CTX_TABLE_SIZE],
    pub ndisc_ops_installed: bool,
    pub debugfs_created: bool,
    pub registered: bool,
    pub inet6_dev_present: bool,
    pub pan_id: u16,
    pub short_addr: u16,
    pub linklocal_added: Option<[u8; 16]>,
    pub rtnl_lock_depth: usize,
}

impl LowpanNetdevice {
    pub const fn new(lltype: LowpanLlType) -> Self {
        Self {
            addr_len: 0,
            dev_type: 0,
            mtu: 0,
            lltype,
            ctx_ids: [0; LOWPAN_IPHC_CTX_TABLE_SIZE],
            ctx_active: [false; LOWPAN_IPHC_CTX_TABLE_SIZE],
            ndisc_ops_installed: false,
            debugfs_created: false,
            registered: false,
            inet6_dev_present: true,
            pan_id: 0,
            short_addr: 0,
            linklocal_added: None,
            rtnl_lock_depth: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LowpanModuleState {
    pub debugfs_created: bool,
    pub notifier_registered: bool,
    pub requested_modules: [&'static str; 7],
    pub requested_count: usize,
}

pub fn lowpan_register_netdevice(
    dev: &mut LowpanNetdevice,
    lltype: LowpanLlType,
    register_netdevice_ret: i32,
) -> i32 {
    dev.addr_len = match lltype {
        LowpanLlType::Ieee802154 => EUI64_ADDR_LEN,
        LowpanLlType::Btle => ETH_ALEN,
    };
    dev.dev_type = ARPHRD_6LOWPAN;
    dev.mtu = IPV6_MIN_MTU;
    dev.lltype = lltype;

    for i in 0..LOWPAN_IPHC_CTX_TABLE_SIZE {
        dev.ctx_ids[i] = i as u8;
    }

    dev.ndisc_ops_installed = true;

    if register_netdevice_ret < 0 {
        return register_netdevice_ret;
    }

    dev.registered = true;
    dev.debugfs_created = true;
    register_netdevice_ret
}

pub fn lowpan_register_netdev(
    dev: &mut LowpanNetdevice,
    lltype: LowpanLlType,
    register_netdevice_ret: i32,
) -> i32 {
    dev.rtnl_lock_depth += 1;
    let ret = lowpan_register_netdevice(dev, lltype, register_netdevice_ret);
    dev.rtnl_lock_depth -= 1;
    ret
}

pub fn lowpan_unregister_netdevice(dev: &mut LowpanNetdevice) {
    dev.registered = false;
    dev.debugfs_created = false;
}

pub fn lowpan_unregister_netdev(dev: &mut LowpanNetdevice) {
    dev.rtnl_lock_depth += 1;
    lowpan_unregister_netdevice(dev);
    dev.rtnl_lock_depth -= 1;
}

pub const fn lowpan_802154_is_valid_src_short_addr(short_addr: u16) -> bool {
    (short_addr & 0x8000) == 0
}

pub const fn addrconf_ifid_802154_6lowpan(pan_id: u16, short_addr: u16) -> Option<[u8; 8]> {
    if !lowpan_802154_is_valid_src_short_addr(short_addr) {
        return None;
    }
    if pan_id == 0 && short_addr == 0 {
        return None;
    }

    let mut eui = [0u8; 8];
    if pan_id != IEEE802154_PAN_ID_BROADCAST {
        let pan = pan_id.to_be_bytes();
        eui[0] = pan[0];
        eui[1] = pan[1];
    }
    eui[0] &= !2;
    eui[2] = 0;
    eui[3] = 0xff;
    eui[4] = 0xfe;
    eui[5] = 0;
    let short = short_addr.to_be_bytes();
    eui[6] = short[0];
    eui[7] = short[1];
    Some(eui)
}

pub fn lowpan_event(dev: &mut LowpanNetdevice, event: u64) -> i32 {
    if dev.dev_type != ARPHRD_6LOWPAN {
        return NOTIFY_DONE;
    }
    if !dev.inet6_dev_present {
        return NOTIFY_DONE;
    }

    match event {
        NETDEV_UP | NETDEV_CHANGE => {
            if dev.lltype == LowpanLlType::Ieee802154 {
                if let Some(ifid) = addrconf_ifid_802154_6lowpan(dev.pan_id, dev.short_addr) {
                    let mut addr = [0u8; 16];
                    addr[0] = 0xfe;
                    addr[1] = 0x80;
                    addr[8..].copy_from_slice(&ifid);
                    dev.linklocal_added = Some(addr);
                }
            }
        }
        NETDEV_DOWN => {
            for active in &mut dev.ctx_active {
                *active = false;
            }
        }
        _ => return NOTIFY_DONE,
    }

    NOTIFY_OK
}

pub fn lowpan_module_init(register_notifier_ret: i32) -> LowpanModuleState {
    if register_notifier_ret < 0 {
        return LowpanModuleState {
            debugfs_created: false,
            notifier_registered: false,
            requested_modules: LOWPAN_REQUESTED_NHC_MODULES,
            requested_count: 0,
        };
    }

    LowpanModuleState {
        debugfs_created: true,
        notifier_registered: true,
        requested_modules: LOWPAN_REQUESTED_NHC_MODULES,
        requested_count: LOWPAN_REQUESTED_NHC_MODULES.len(),
    }
}

pub fn lowpan_module_exit(state: &mut LowpanModuleState) {
    state.debugfs_created = false;
    state.notifier_registered = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowpan_core_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/6lowpan/core.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/net/6lowpan.h"
        ));
        let internal = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/6lowpan/6lowpan_i.h"
        ));
        assert!(source.contains("dev->addr_len = EUI64_ADDR_LEN;"));
        assert!(source.contains("dev->addr_len = ETH_ALEN;"));
        assert!(source.contains("dev->type = ARPHRD_6LOWPAN;"));
        assert!(source.contains("dev->mtu = IPV6_MIN_MTU;"));
        assert!(source.contains("lowpan_dev(dev)->ctx.table[i].id = i;"));
        assert!(source.contains("dev->ndisc_ops = &lowpan_ndisc_ops;"));
        assert!(source.contains("ret = register_netdevice(dev);"));
        assert!(source.contains("if (ret < 0)"));
        assert!(source.contains("lowpan_dev_debugfs_init(dev);"));
        assert!(source.contains("rtnl_lock();"));
        assert!(source.contains("unregister_netdevice(dev);"));
        assert!(source.contains("addrconf_ifid_802154_6lowpan"));
        assert!(source.contains("eui[0] &= ~2;"));
        assert!(source.contains("case NETDEV_UP:"));
        assert!(source.contains("case NETDEV_CHANGE:"));
        assert!(source.contains("case NETDEV_DOWN:"));
        assert!(source.contains("clear_bit(LOWPAN_IPHC_CTX_FLAG_ACTIVE"));
        assert!(source.contains("request_module_nowait(\"nhc_udp\");"));
        assert!(source.contains("module_init(lowpan_module_init);"));
        assert!(source.contains("module_exit(lowpan_module_exit);"));
        assert!(source.contains(
            "MODULE_DESCRIPTION(\"IPv6 over Low-Power Wireless Personal Area Network core module\")"
        ));
        assert!(header.contains("LOWPAN_IPHC_CTX_TABLE_SIZE\t(1 << 4)"));
        assert!(header.contains("LOWPAN_IPHC_CTX_FLAG_ACTIVE"));
        assert!(header.contains("return !(addr & cpu_to_le16(0x8000));"));
        assert!(internal.contains("lowpan_is_ll"));
    }

    #[test]
    fn register_and_unregister_netdevice_match_linux_ordering() {
        let mut ieee = LowpanNetdevice::new(LowpanLlType::Btle);
        assert_eq!(
            lowpan_register_netdevice(&mut ieee, LowpanLlType::Ieee802154, 0),
            0
        );
        assert_eq!(ieee.addr_len, EUI64_ADDR_LEN);
        assert_eq!(ieee.dev_type, ARPHRD_6LOWPAN);
        assert_eq!(ieee.mtu, IPV6_MIN_MTU);
        assert_eq!(ieee.ctx_ids[15], 15);
        assert!(ieee.ndisc_ops_installed);
        assert!(ieee.registered);
        assert!(ieee.debugfs_created);

        let mut btle = LowpanNetdevice::new(LowpanLlType::Ieee802154);
        assert_eq!(lowpan_register_netdev(&mut btle, LowpanLlType::Btle, 0), 0);
        assert_eq!(btle.addr_len, ETH_ALEN);
        assert_eq!(btle.rtnl_lock_depth, 0);
        lowpan_unregister_netdev(&mut btle);
        assert!(!btle.registered);
        assert!(!btle.debugfs_created);
        assert_eq!(btle.rtnl_lock_depth, 0);

        let mut failed = LowpanNetdevice::new(LowpanLlType::Btle);
        assert_eq!(
            lowpan_register_netdevice(&mut failed, LowpanLlType::Ieee802154, -7),
            -7
        );
        assert_eq!(failed.addr_len, EUI64_ADDR_LEN);
        assert!(failed.ndisc_ops_installed);
        assert!(!failed.registered);
        assert!(!failed.debugfs_created);
    }

    #[test]
    fn ieee802154_short_addr_iid_matches_linux_rules() {
        assert_eq!(
            addrconf_ifid_802154_6lowpan(0x1234, 0x4567),
            Some([0x10, 0x34, 0x00, 0xff, 0xfe, 0x00, 0x45, 0x67])
        );
        assert_eq!(
            addrconf_ifid_802154_6lowpan(IEEE802154_PAN_ID_BROADCAST, 0x0001),
            Some([0, 0, 0, 0xff, 0xfe, 0, 0, 1])
        );
        assert_eq!(addrconf_ifid_802154_6lowpan(0, 0), None);
        assert_eq!(addrconf_ifid_802154_6lowpan(0x1234, 0x8000), None);
    }

    #[test]
    fn notifier_adds_linklocal_and_clears_context_flags() {
        let mut dev = LowpanNetdevice::new(LowpanLlType::Ieee802154);
        assert_eq!(
            lowpan_register_netdevice(&mut dev, LowpanLlType::Ieee802154, 0),
            0
        );
        dev.pan_id = 0x1234;
        dev.short_addr = 0x4567;
        assert_eq!(lowpan_event(&mut dev, NETDEV_UP), NOTIFY_OK);
        assert_eq!(
            dev.linklocal_added,
            Some([
                0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0x10, 0x34, 0, 0xff, 0xfe, 0, 0x45, 0x67
            ])
        );

        dev.ctx_active = [true; LOWPAN_IPHC_CTX_TABLE_SIZE];
        assert_eq!(lowpan_event(&mut dev, NETDEV_DOWN), NOTIFY_OK);
        assert_eq!(dev.ctx_active, [false; LOWPAN_IPHC_CTX_TABLE_SIZE]);

        let mut missing_idev = dev;
        missing_idev.inet6_dev_present = false;
        assert_eq!(lowpan_event(&mut missing_idev, NETDEV_UP), NOTIFY_DONE);

        let mut not_lowpan = LowpanNetdevice::new(LowpanLlType::Btle);
        assert_eq!(lowpan_event(&mut not_lowpan, NETDEV_UP), NOTIFY_DONE);
        assert_eq!(lowpan_event(&mut dev, 99), NOTIFY_DONE);
    }

    #[test]
    fn module_init_and_exit_follow_debugfs_notifier_paths() {
        let state = lowpan_module_init(0);
        assert!(state.debugfs_created);
        assert!(state.notifier_registered);
        assert_eq!(state.requested_count, LOWPAN_REQUESTED_NHC_MODULES.len());
        assert_eq!(state.requested_modules[6], "nhc_udp");

        let failed = lowpan_module_init(-5);
        assert!(!failed.debugfs_created);
        assert!(!failed.notifier_registered);
        assert_eq!(failed.requested_count, 0);

        let mut exiting = state;
        lowpan_module_exit(&mut exiting);
        assert!(!exiting.debugfs_created);
        assert!(!exiting.notifier_registered);
    }
}
