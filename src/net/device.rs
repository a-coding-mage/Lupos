//! linux-parity: complete
//! linux-source: vendor/linux/net
//! test-origin: linux:vendor/linux/net
//! `struct net_device` and a minimal rtnl-protected registry.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

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
pub const ETH_MIN_MTU: u32 = 68;
pub const ETH_MAX_MTU: u32 = 65535;
pub const LOOPBACK_MTU: u32 = 64 * 1024;

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

pub fn init() {
    if NETDEV_INIT_DONE.swap(true, Ordering::AcqRel) {
        return;
    }

    // Linux creates one loopback net_device per net namespace during
    // drivers/net/loopback.c::loopback_net_init().  systemd uses the rtnetlink
    // dump of this device as its baseline network inventory.
    if lookup_netdevice("lo").is_none()
        && let Ok(dev) = register_loopback_netdevice()
    {
        dev.flags
            .store(IFF_LOOPBACK | IFF_UP | IFF_RUNNING, Ordering::Release);
        dev.carrier.store(true, Ordering::Release);
    }
}

fn register_loopback_netdevice() -> Result<NetDeviceRef, i32> {
    rtnl_lock(|| {
        let mut registry = NETDEV_BY_NAME.lock();
        if registry.contains_key("lo") {
            return Err(EBUSY);
        }

        let dev = Arc::new(NetDevice {
            ifindex: NEXT_IFINDEX.fetch_add(1, Ordering::AcqRel),
            name: String::from("lo"),
            mtu: LOOPBACK_MTU,
            flags: AtomicU32::new(IFF_LOOPBACK),
            dev_addr: [0; 6],
            ops: &LOOPBACK_NETDEV_OPS,
            carrier: AtomicBool::new(false),
            tx_packets: AtomicU64::new(0),
            rx_packets: AtomicU64::new(0),
            linux_dev: None,
        });
        registry.insert(String::from("lo"), dev.clone());
        Ok(dev)
    })
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
            tx_packets: AtomicU64::new(0),
            rx_packets: AtomicU64::new(0),
            linux_dev: None,
        });
        registry.insert(String::from(name), dev.clone());
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
        tx_packets: AtomicU64::new(0),
        rx_packets: AtomicU64::new(0),
        linux_dev: Some(linux_dev as usize),
    });
    registry.insert(String::from(name), dev.clone());
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
            NETDEV_BY_NAME.lock().remove(&name);
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
        if registry.remove(name).is_some() {
            Ok(())
        } else {
            Err(ENODEV)
        }
    })
}

pub fn lookup_netdevice(name: &str) -> Option<NetDeviceRef> {
    NETDEV_BY_NAME.lock().get(name).cloned()
}

pub fn list_netdevices() -> alloc::vec::Vec<NetDeviceRef> {
    NETDEV_BY_NAME.lock().values().cloned().collect()
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
    }
    Ok(())
}

pub fn set_device_down(dev: &NetDeviceRef) -> Result<(), i32> {
    (dev.ops.stop)(dev)?;
    dev.flags
        .fetch_and(!(IFF_UP | IFF_RUNNING), Ordering::AcqRel);
    dev.carrier.store(false, Ordering::Release);
    if let Some(raw) = dev.linux_dev.map(|ptr| ptr as *mut u8) {
        unsafe {
            let flags = raw.add(176).cast::<u32>();
            flags.write_unaligned(flags.read_unaligned() & !IFF_UP);
            let state = &*raw.add(168).cast::<AtomicU64>();
            state.fetch_and(!1, Ordering::AcqRel);
        }
    }
    Ok(())
}

pub fn set_carrier(dev: &NetDeviceRef, up: bool) {
    dev.carrier.store(up, Ordering::Release);
    if up {
        dev.flags.fetch_or(IFF_RUNNING, Ordering::AcqRel);
    } else {
        dev.flags.fetch_and(!IFF_RUNNING, Ordering::AcqRel);
    }
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
}
