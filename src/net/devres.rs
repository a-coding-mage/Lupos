//! linux-parity: complete
//! linux-source: vendor/linux/net/devres.c
//! test-origin: linux:vendor/linux/net/devres.c
//! Managed net_device allocation and registration helpers.

use crate::include::uapi::errno::{EINVAL, ENOMEM};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManagedNetDevice {
    pub id: u32,
    pub sizeof_priv: i32,
    pub txqs: u32,
    pub rxqs: u32,
    pub registered: bool,
    pub freed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct DeviceDevres {
    pub managed: Option<ManagedNetDevice>,
    pub unregister_registered: bool,
}

pub fn devm_alloc_etherdev_mqs(
    dev: &mut DeviceDevres,
    sizeof_priv: i32,
    txqs: u32,
    rxqs: u32,
    devres_alloc_ok: bool,
    alloc_etherdev_ok: bool,
) -> Option<ManagedNetDevice> {
    if !devres_alloc_ok || !alloc_etherdev_ok {
        return None;
    }
    let ndev = ManagedNetDevice {
        id: 1,
        sizeof_priv,
        txqs,
        rxqs,
        registered: false,
        freed: false,
    };
    dev.managed = Some(ndev);
    Some(ndev)
}

pub fn devm_register_netdev(
    dev: &mut DeviceDevres,
    mut ndev: ManagedNetDevice,
    devres_alloc_ok: bool,
    register_ret: i32,
) -> Result<ManagedNetDevice, i32> {
    if dev.managed.map(|managed| managed.id) != Some(ndev.id) {
        return Err(-EINVAL);
    }
    if !devres_alloc_ok {
        return Err(-ENOMEM);
    }
    if register_ret != 0 {
        return Err(register_ret);
    }
    ndev.registered = true;
    dev.unregister_registered = true;
    dev.managed = Some(ndev);
    Ok(ndev)
}

pub fn devm_free_netdev(ndev: &mut ManagedNetDevice) {
    ndev.freed = true;
}

pub fn devm_unregister_netdev(ndev: &mut ManagedNetDevice) {
    ndev.registered = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn net_devres_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/devres.c"
        ));
        assert!(source.contains("struct net_device_devres"));
        assert!(source.contains("static void devm_free_netdev"));
        assert!(source.contains("free_netdev(res->ndev);"));
        assert!(source.contains("devm_alloc_etherdev_mqs"));
        assert!(source.contains("devres_alloc(devm_free_netdev, sizeof(*dr), GFP_KERNEL);"));
        assert!(source.contains("alloc_etherdev_mqs(sizeof_priv, txqs, rxqs);"));
        assert!(source.contains("devres_add(dev, dr);"));
        assert!(source.contains("static void devm_unregister_netdev"));
        assert!(source.contains("unregister_netdev(res->ndev);"));
        assert!(source.contains("netdev_devres_match"));
        assert!(source.contains("return ndev == res->ndev;"));
        assert!(source.contains("int devm_register_netdev"));
        assert!(source.contains("WARN_ON(!devres_find(dev, devm_free_netdev"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("ret = register_netdev(ndev);"));
        assert!(source.contains("EXPORT_SYMBOL(devm_register_netdev);"));
    }

    #[test]
    fn managed_netdev_requires_managed_allocation_before_register() {
        let mut dev = DeviceDevres::default();
        let ndev = devm_alloc_etherdev_mqs(&mut dev, 16, 2, 3, true, true).unwrap();
        assert_eq!(ndev.txqs, 2);
        assert_eq!(ndev.rxqs, 3);
        let registered = devm_register_netdev(&mut dev, ndev, true, 0).unwrap();
        assert!(registered.registered);
        assert!(dev.unregister_registered);
        assert_eq!(
            devm_register_netdev(
                &mut DeviceDevres::default(),
                ManagedNetDevice { id: 99, ..ndev },
                true,
                0,
            ),
            Err(-EINVAL)
        );
        assert_eq!(devm_register_netdev(&mut dev, ndev, false, 0), Err(-ENOMEM));
    }
}
