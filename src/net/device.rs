//! linux-parity: complete
//! linux-source: vendor/linux/net
//! test-origin: linux:vendor/linux/net
//! `struct net_device` and a minimal rtnl-protected registry.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::{EBUSY, EINVAL, ENODEV};
use crate::kernel::locking::qspinlock::QSpinLock;
use crate::net::skbuff::SkBuff;

pub const IFF_UP: u32 = 0x1;
pub const IFF_BROADCAST: u32 = 0x2;
pub const IFF_LOOPBACK: u32 = 0x8;
pub const IFF_RUNNING: u32 = 0x40;
pub const IFF_MULTICAST: u32 = 0x1000;
pub const IFF_LOWER_UP: u32 = 0x1_0000;
pub const IFF_DORMANT: u32 = 0x2_0000;
pub const ETH_MIN_MTU: u32 = 68;
pub const ETH_MAX_MTU: u32 = 65535;
pub const LOOPBACK_MTU: u32 = 64 * 1024;
pub const IF_OPER_UNKNOWN: u8 = 0;
pub const IF_OPER_DOWN: u8 = 2;
pub const IF_OPER_TESTING: u8 = 4;
pub const IF_OPER_DORMANT: u8 = 5;
pub const IF_OPER_UP: u8 = 6;
pub const IF_LINK_MODE_DEFAULT: u8 = 0;
pub const IF_LINK_MODE_DORMANT: u8 = 1;
pub const IF_LINK_MODE_TESTING: u8 = 2;

pub type NetDeviceRef = Arc<NetDevice>;

pub struct NetDeviceOps {
    pub name: &'static str,
    pub open: fn(&NetDeviceRef) -> Result<(), i32>,
    pub stop: fn(&NetDeviceRef) -> Result<(), i32>,
    pub start_xmit: fn(&NetDeviceRef, SkBuff) -> Result<(), i32>,
}

pub struct NetDevice {
    pub ifindex: u32,
    pub name: String,
    pub mtu: u32,
    pub flags: AtomicU32,
    pub dev_addr: [u8; 6],
    pub ops: &'static NetDeviceOps,
    pub carrier: AtomicBool,
    pub operstate: AtomicU8,
    pub link_mode: AtomicU8,
    pub tx_packets: AtomicU64,
    pub rx_packets: AtomicU64,
    /// Authoritative configured Linux `struct net_device` for a vendor-built
    /// C driver, when this registry entry represents one.
    pub linux_dev: Option<usize>,
}

impl NetDevice {
    pub fn is_up(&self) -> bool {
        self.flags.load(Ordering::Acquire) & IFF_UP != 0
    }

    pub fn carrier_ok(&self) -> bool {
        self.carrier.load(Ordering::Acquire)
    }

    pub fn operstate(&self) -> u8 {
        self.operstate.load(Ordering::Acquire)
    }

    pub fn link_mode(&self) -> u8 {
        self.link_mode.load(Ordering::Acquire)
    }

    pub fn userspace_operstate(&self) -> u8 {
        if self.flags.load(Ordering::Acquire) & IFF_UP == 0 {
            IF_OPER_DOWN
        } else {
            self.operstate()
        }
    }

    pub fn userspace_flags(&self) -> u32 {
        let internal = self.flags.load(Ordering::Acquire);
        let mut flags = internal & !(IFF_RUNNING | IFF_LOWER_UP | IFF_DORMANT);
        if internal & IFF_UP != 0 {
            let operstate = self.operstate();
            if operstate == IF_OPER_UP || operstate == IF_OPER_UNKNOWN {
                flags |= IFF_RUNNING;
            }
            if self.carrier_ok() {
                flags |= IFF_LOWER_UP;
            }
            if operstate == IF_OPER_DORMANT {
                flags |= IFF_DORMANT;
            }
        }
        flags
    }

    pub fn refresh_operstate(&self) {
        let state = if self.flags.load(Ordering::Acquire) & IFF_LOOPBACK != 0 {
            IF_OPER_UNKNOWN
        } else if !self.carrier_ok() {
            IF_OPER_DOWN
        } else {
            match self.link_mode() {
                IF_LINK_MODE_DORMANT => IF_OPER_DORMANT,
                IF_LINK_MODE_TESTING => IF_OPER_TESTING,
                _ => IF_OPER_UP,
            }
        };
        self.operstate.store(state, Ordering::Release);
    }

    pub fn set_link_mode(&self, value: u8) -> bool {
        self.link_mode.swap(value, Ordering::AcqRel) != value
    }

    pub fn set_operstate_from_user(&self, transition: u8) -> bool {
        let current = self.operstate();
        let next = match transition {
            IF_OPER_UP
                if matches!(current, IF_OPER_DORMANT | IF_OPER_TESTING | IF_OPER_UNKNOWN) =>
            {
                IF_OPER_UP
            }
            IF_OPER_TESTING if current == IF_OPER_UP || current == IF_OPER_UNKNOWN => {
                IF_OPER_TESTING
            }
            IF_OPER_DORMANT if current == IF_OPER_UP || current == IF_OPER_UNKNOWN => {
                IF_OPER_DORMANT
            }
            _ => current,
        };
        if next == current {
            return false;
        }
        self.operstate.store(next, Ordering::Release);
        true
    }

    pub fn stats(&self) -> NetDeviceStats {
        NetDeviceStats {
            tx_packets: self.tx_packets.load(Ordering::Acquire),
            rx_packets: self.rx_packets.load(Ordering::Acquire),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NetDeviceStats {
    pub tx_packets: u64,
    pub rx_packets: u64,
}

static NEXT_IFINDEX: AtomicU32 = AtomicU32::new(1);
static NETDEV_INIT_DONE: AtomicBool = AtomicBool::new(false);

lazy_static! {
    static ref NETDEV_BY_NAME: Mutex<BTreeMap<String, NetDeviceRef>> = Mutex::new(BTreeMap::new());
    /// Device registries for non-init network namespaces.  Physical/vendor
    /// devices stay exclusively in `NETDEV_BY_NAME`; each child namespace is
    /// lazily populated with its own down loopback device.
    static ref NETDEV_BY_NAMESPACE: Mutex<BTreeMap<usize, BTreeMap<String, NetDeviceRef>>> =
        Mutex::new(BTreeMap::new());
}

static RTNL: QSpinLock = QSpinLock::new();

pub fn rtnl_lock<T>(f: impl FnOnce() -> T) -> T {
    RTNL.lock();
    let result = f();
    RTNL.unlock();
    result
}

pub fn linux_rtnl_lock() {
    RTNL.lock();
}

pub fn linux_rtnl_unlock() {
    RTNL.unlock();
}

pub fn linux_rtnl_is_locked() -> bool {
    RTNL.is_locked()
}

pub fn init() {
    let already_initialized = NETDEV_INIT_DONE.swap(true, Ordering::AcqRel);
    if already_initialized && lookup_netdevice("lo").is_some() {
        return;
    }

    // Linux creates one loopback net_device per net namespace during
    // drivers/net/loopback.c::loopback_net_init().  systemd uses the rtnetlink
    // dump of this device as its baseline network inventory.
    if lookup_netdevice("lo").is_none()
        && let Ok(dev) = register_loopback_netdevice(0)
    {
        dev.flags.store(
            IFF_LOOPBACK | IFF_UP | IFF_RUNNING | IFF_LOWER_UP,
            Ordering::Release,
        );
        dev.carrier.store(true, Ordering::Release);
        dev.refresh_operstate();
    }
}

fn register_loopback_netdevice(namespace_key: usize) -> Result<NetDeviceRef, i32> {
    rtnl_lock(|| {
        let dev = Arc::new(NetDevice {
            ifindex: NEXT_IFINDEX.fetch_add(1, Ordering::AcqRel),
            name: String::from("lo"),
            mtu: LOOPBACK_MTU,
            flags: AtomicU32::new(IFF_LOOPBACK),
            dev_addr: [0; 6],
            ops: &LOOPBACK_NETDEV_OPS,
            carrier: AtomicBool::new(false),
            operstate: AtomicU8::new(IF_OPER_UNKNOWN),
            link_mode: AtomicU8::new(IF_LINK_MODE_DEFAULT),
            tx_packets: AtomicU64::new(0),
            rx_packets: AtomicU64::new(0),
            linux_dev: None,
        });
        if namespace_key == 0 {
            let mut registry = NETDEV_BY_NAME.lock();
            if registry.contains_key("lo") {
                return Err(EBUSY);
            }
            registry.insert(String::from("lo"), dev.clone());
            crate::fs::sysfs::net::register_netdevice(&dev);
            crate::net::uevent::announce_netdevice(
                crate::net::uevent::UeventAction::Add,
                "lo",
                dev.ifindex,
            );
        } else {
            let mut namespaces = NETDEV_BY_NAMESPACE.lock();
            let registry = namespaces.entry(namespace_key).or_default();
            if registry.contains_key("lo") {
                return Err(EBUSY);
            }
            registry.insert(String::from("lo"), dev.clone());
        }
        Ok(dev)
    })
}

fn ensure_current_namespace_loopback(namespace_key: usize) {
    if namespace_key == 0 {
        return;
    }
    let present = NETDEV_BY_NAMESPACE
        .lock()
        .get(&namespace_key)
        .is_some_and(|registry| registry.contains_key("lo"));
    if !present {
        let _ = register_loopback_netdevice(namespace_key);
    }
}

pub fn unregister_net_namespace(namespace_key: usize) {
    if namespace_key != 0 {
        NETDEV_BY_NAMESPACE.lock().remove(&namespace_key);
    }
}

pub fn register_netdevice(
    name: &str,
    mtu: u32,
    dev_addr: [u8; 6],
    ops: &'static NetDeviceOps,
) -> Result<NetDeviceRef, i32> {
    validate_mtu(mtu)?;
    rtnl_lock(|| {
        let mut registry = NETDEV_BY_NAME.lock();
        if registry.contains_key(name) {
            return Err(EBUSY);
        }

        let dev = Arc::new(NetDevice {
            ifindex: NEXT_IFINDEX.fetch_add(1, Ordering::AcqRel),
            name: String::from(name),
            mtu,
            flags: AtomicU32::new(IFF_BROADCAST | IFF_MULTICAST),
            dev_addr,
            ops,
            carrier: AtomicBool::new(false),
            operstate: AtomicU8::new(IF_OPER_DOWN),
            link_mode: AtomicU8::new(IF_LINK_MODE_DEFAULT),
            tx_packets: AtomicU64::new(0),
            rx_packets: AtomicU64::new(0),
            linux_dev: None,
        });
        registry.insert(String::from(name), dev.clone());
        crate::fs::sysfs::net::register_netdevice(&dev);
        crate::net::uevent::announce_netdevice(
            crate::net::uevent::UeventAction::Add,
            name,
            dev.ifindex,
        );
        Ok(dev)
    })
}

pub fn register_linux_netdevice_locked(
    name: &str,
    mtu: u32,
    dev_addr: [u8; 6],
    linux_dev: *mut u8,
) -> Result<NetDeviceRef, i32> {
    validate_mtu(mtu)?;
    if linux_dev.is_null() {
        return Err(EINVAL);
    }
    let mut registry = NETDEV_BY_NAME.lock();
    if registry.contains_key(name) {
        return Err(EBUSY);
    }
    let dev = Arc::new(NetDevice {
        ifindex: NEXT_IFINDEX.fetch_add(1, Ordering::AcqRel),
        name: String::from(name),
        mtu,
        flags: AtomicU32::new(IFF_BROADCAST | IFF_MULTICAST),
        dev_addr,
        ops: &LINUX_NETDEV_OPS,
        carrier: AtomicBool::new(false),
        operstate: AtomicU8::new(IF_OPER_DOWN),
        link_mode: AtomicU8::new(IF_LINK_MODE_DEFAULT),
        tx_packets: AtomicU64::new(0),
        rx_packets: AtomicU64::new(0),
        linux_dev: Some(linux_dev as usize),
    });
    registry.insert(String::from(name), dev.clone());
    crate::fs::sysfs::net::register_netdevice(&dev);
    crate::net::uevent::announce_netdevice(
        crate::net::uevent::UeventAction::Add,
        name,
        dev.ifindex,
    );
    crate::net::socket::broadcast_rtnl_newlink(&dev);
    Ok(dev)
}

pub fn lookup_linux_netdevice(linux_dev: *const u8) -> Option<NetDeviceRef> {
    let address = linux_dev as usize;
    NETDEV_BY_NAME
        .lock()
        .values()
        .find(|dev| dev.linux_dev == Some(address))
        .cloned()
}

pub fn unregister_linux_netdevice_locked(linux_dev: *const u8) -> Result<(), i32> {
    let address = linux_dev as usize;
    let name = NETDEV_BY_NAME
        .lock()
        .iter()
        .find(|(_, dev)| dev.linux_dev == Some(address))
        .map(|(name, _)| name.clone());
    match name {
        Some(name) => {
            if let Some(dev) = NETDEV_BY_NAME.lock().get(&name).cloned() {
                crate::net::uevent::announce_netdevice(
                    crate::net::uevent::UeventAction::Remove,
                    &dev.name,
                    dev.ifindex,
                );
                crate::net::socket::drop_rtnl_ifaddrs_for_device(dev.ifindex);
                crate::net::socket::drop_rtnl_routes_for_device(dev.ifindex);
            }
            NETDEV_BY_NAME.lock().remove(&name);
            crate::fs::sysfs::net::unregister_netdevice(&name);
            Ok(())
        }
        None => Err(ENODEV),
    }
}

pub fn validate_mtu(mtu: u32) -> Result<(), i32> {
    if (ETH_MIN_MTU..=ETH_MAX_MTU).contains(&mtu) {
        Ok(())
    } else {
        Err(EINVAL)
    }
}

pub fn unregister_netdevice(name: &str) -> Result<(), i32> {
    rtnl_lock(|| {
        let mut registry = NETDEV_BY_NAME.lock();
        if let Some(dev) = registry.remove(name) {
            crate::net::uevent::announce_netdevice(
                crate::net::uevent::UeventAction::Remove,
                &dev.name,
                dev.ifindex,
            );
            crate::net::socket::drop_rtnl_ifaddrs_for_device(dev.ifindex);
            crate::net::socket::drop_rtnl_routes_for_device(dev.ifindex);
            crate::fs::sysfs::net::unregister_netdevice(name);
            Ok(())
        } else {
            Err(ENODEV)
        }
    })
}

pub fn lookup_netdevice(name: &str) -> Option<NetDeviceRef> {
    let namespace_key = crate::net::core::net_namespace::current_net_namespace_key();
    if namespace_key == 0 {
        NETDEV_BY_NAME.lock().get(name).cloned()
    } else {
        ensure_current_namespace_loopback(namespace_key);
        NETDEV_BY_NAMESPACE
            .lock()
            .get(&namespace_key)
            .and_then(|registry| registry.get(name).cloned())
    }
}

pub fn list_netdevices() -> alloc::vec::Vec<NetDeviceRef> {
    let namespace_key = crate::net::core::net_namespace::current_net_namespace_key();
    if namespace_key == 0 {
        NETDEV_BY_NAME.lock().values().cloned().collect()
    } else {
        ensure_current_namespace_loopback(namespace_key);
        NETDEV_BY_NAMESPACE
            .lock()
            .get(&namespace_key)
            .map(|registry| registry.values().cloned().collect())
            .unwrap_or_default()
    }
}

pub fn set_device_up(dev: &NetDeviceRef) -> Result<(), i32> {
    (dev.ops.open)(dev)?;
    dev.flags.fetch_or(IFF_UP | IFF_RUNNING, Ordering::AcqRel);
    if let Some(raw) = dev.linux_dev.map(|ptr| ptr as *mut u8) {
        unsafe {
            let flags = raw.add(176).cast::<u32>();
            flags.write_unaligned(flags.read_unaligned() | IFF_UP);
            let state = &*raw.add(168).cast::<AtomicU64>();
            state.fetch_or(1, Ordering::AcqRel);
        }
        if !dev.carrier_ok() {
            dev.flags.fetch_and(!IFF_RUNNING, Ordering::AcqRel);
        }
    } else {
        dev.carrier.store(true, Ordering::Release);
        dev.flags.fetch_or(IFF_LOWER_UP, Ordering::AcqRel);
    }
    dev.refresh_operstate();
    crate::net::socket::broadcast_rtnl_newlink(dev);
    Ok(())
}

pub fn set_device_down(dev: &NetDeviceRef) -> Result<(), i32> {
    (dev.ops.stop)(dev)?;
    dev.flags
        .fetch_and(!(IFF_UP | IFF_RUNNING | IFF_LOWER_UP), Ordering::AcqRel);
    dev.carrier.store(false, Ordering::Release);
    if let Some(raw) = dev.linux_dev.map(|ptr| ptr as *mut u8) {
        unsafe {
            let flags = raw.add(176).cast::<u32>();
            flags.write_unaligned(flags.read_unaligned() & !IFF_UP);
            let state = &*raw.add(168).cast::<AtomicU64>();
            state.fetch_and(!1, Ordering::AcqRel);
        }
    }
    dev.operstate.store(IF_OPER_DOWN, Ordering::Release);
    crate::net::socket::broadcast_rtnl_newlink(dev);
    Ok(())
}

pub fn set_carrier(dev: &NetDeviceRef, up: bool) {
    dev.carrier.store(up, Ordering::Release);
    if up {
        dev.flags
            .fetch_or(IFF_RUNNING | IFF_LOWER_UP, Ordering::AcqRel);
    } else {
        dev.flags
            .fetch_and(!(IFF_RUNNING | IFF_LOWER_UP), Ordering::AcqRel);
    }
    dev.refresh_operstate();
    #[cfg(not(test))]
    if crate::kernel::debug_trace::netlink_enabled() {
        crate::linux_driver_abi::tty::serial_println!(
            "trace-netlink-link action=carrier ifindex={} up={} flags=0x{:x}",
            dev.ifindex,
            u8::from(up),
            dev.flags.load(Ordering::Acquire)
        );
    }
    crate::net::socket::broadcast_rtnl_newlink(dev);
}

pub fn record_rx(dev: &NetDeviceRef) {
    dev.rx_packets.fetch_add(1, Ordering::AcqRel);
}

fn dummy_open(_dev: &NetDeviceRef) -> Result<(), i32> {
    Ok(())
}

fn dummy_stop(_dev: &NetDeviceRef) -> Result<(), i32> {
    Ok(())
}

fn dummy_start_xmit(dev: &NetDeviceRef, _skb: SkBuff) -> Result<(), i32> {
    dev.tx_packets.fetch_add(1, Ordering::AcqRel);
    Ok(())
}

unsafe fn linux_netdev_op(dev: &NetDeviceRef, offset: usize) -> Option<usize> {
    let raw = dev.linux_dev? as *mut u8;
    let ops = unsafe { raw.add(8).cast::<*const u8>().read_unaligned() };
    if ops.is_null() {
        return None;
    }
    let function = unsafe { ops.add(offset).cast::<usize>().read_unaligned() };
    (function != 0).then_some(function)
}

fn linux_open(dev: &NetDeviceRef) -> Result<(), i32> {
    let Some(raw) = dev.linux_dev.map(|ptr| ptr as *mut u8) else {
        return Err(ENODEV);
    };
    let Some(function) = (unsafe { linux_netdev_op(dev, 16) }) else {
        return Ok(());
    };
    let open: unsafe extern "C" fn(*mut u8) -> i32 = unsafe { core::mem::transmute(function) };
    let result = unsafe { open(raw) };
    if result == 0 { Ok(()) } else { Err(-result) }
}

fn linux_stop(dev: &NetDeviceRef) -> Result<(), i32> {
    let Some(raw) = dev.linux_dev.map(|ptr| ptr as *mut u8) else {
        return Err(ENODEV);
    };
    let Some(function) = (unsafe { linux_netdev_op(dev, 24) }) else {
        return Ok(());
    };
    let stop: unsafe extern "C" fn(*mut u8) -> i32 = unsafe { core::mem::transmute(function) };
    let result = unsafe { stop(raw) };
    if result == 0 { Ok(()) } else { Err(-result) }
}

fn linux_start_xmit(_dev: &NetDeviceRef, _skb: SkBuff) -> Result<(), i32> {
    // The raw skb bridge is installed with the packet path; fail closed until
    // a configured C `struct sk_buff` can be passed to ndo_start_xmit.
    Err(crate::include::uapi::errno::EOPNOTSUPP)
}

pub fn transmit(dev: &NetDeviceRef, skb: SkBuff) -> Result<(), i32> {
    (dev.ops.start_xmit)(dev, skb)
}

pub static DUMMY_NETDEV_OPS: NetDeviceOps = NetDeviceOps {
    name: "dummy",
    open: dummy_open,
    stop: dummy_stop,
    start_xmit: dummy_start_xmit,
};

pub static LOOPBACK_NETDEV_OPS: NetDeviceOps = NetDeviceOps {
    name: "loopback",
    open: dummy_open,
    stop: dummy_stop,
    start_xmit: dummy_start_xmit,
};

pub static LINUX_NETDEV_OPS: NetDeviceOps = NetDeviceOps {
    name: "linux-module",
    open: linux_open,
    stop: linux_stop,
    start_xmit: linux_start_xmit,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_lookup_and_toggle_device() {
        let name = "netdev-test0";
        let _ = unregister_netdevice(name);
        let dev = register_netdevice(name, 1500, [2, 0, 0, 0, 0, 1], &DUMMY_NETDEV_OPS).unwrap();
        assert_eq!(lookup_netdevice(name).unwrap().ifindex, dev.ifindex);
        set_device_up(&dev).unwrap();
        assert!(dev.is_up());
        assert!(dev.carrier_ok());
        set_carrier(&dev, false);
        assert!(!dev.carrier_ok());
        set_device_down(&dev).unwrap();
        assert!(!dev.is_up());
        unregister_netdevice(name).unwrap();
    }

    #[test]
    fn init_registers_linux_loopback_device() {
        init();
        let lo = lookup_netdevice("lo").expect("loopback registered");
        assert_eq!(lo.name, "lo");
        assert_eq!(lo.mtu, LOOPBACK_MTU);
        assert_ne!(lo.flags.load(Ordering::Acquire) & IFF_LOOPBACK, 0);
        assert!(lo.is_up());
        assert!(lo.carrier_ok());
    }

    #[test]
    fn mtu_validation_and_stats_follow_linux_bounds() {
        assert_eq!(validate_mtu(ETH_MIN_MTU - 1), Err(EINVAL));
        assert_eq!(validate_mtu(ETH_MAX_MTU + 1), Err(EINVAL));
        assert_eq!(validate_mtu(1500), Ok(()));

        let name = "netdev-stats0";
        let _ = unregister_netdevice(name);
        let dev = register_netdevice(name, 1500, [2, 0, 0, 0, 0, 3], &DUMMY_NETDEV_OPS).unwrap();
        let skb = crate::net::skbuff::alloc_skb(16).unwrap();
        transmit(&dev, skb).unwrap();
        record_rx(&dev);
        assert_eq!(
            dev.stats(),
            NetDeviceStats {
                tx_packets: 1,
                rx_packets: 1
            }
        );
        unregister_netdevice(name).unwrap();
    }

    #[test]
    fn register_netdevice_emits_linux_shaped_add_uevent() {
        let _guard = crate::net::uevent::test_lock();
        let _ = crate::net::uevent::drain_pending();
        let name = "netdev-uevent0";
        let _ = unregister_netdevice(name);
        let dev = register_netdevice(name, 1500, [2, 0, 0, 0, 0, 4], &DUMMY_NETDEV_OPS).unwrap();
        let events = crate::net::uevent::drain_pending();
        let payload = &events
            .iter()
            .find(|event| {
                event
                    .payload
                    .starts_with(b"add@/devices/virtual/net/netdev-uevent0\0")
            })
            .expect("netdev add uevent")
            .payload;
        let ifindex_record = alloc::format!("IFINDEX={}\0", dev.ifindex);
        assert!(
            payload
                .windows(ifindex_record.len())
                .any(|window| window == ifindex_record.as_bytes()),
            "payload must carry IFINDEX for the registered netdev"
        );
        unregister_netdevice(name).unwrap();
        let _ = crate::net::uevent::drain_pending();
    }
}
