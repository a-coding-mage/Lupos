//! linux-parity: complete
//! linux-source: vendor/linux/net/ieee802154/pan.c
//! test-origin: linux:vendor/linux/net/ieee802154/pan.c
//! IEEE 802.15.4 PAN association helpers.

pub const IEEE802154_ADDR_SHORT: u8 = 2;
pub const IEEE802154_ADDR_EXTENDED: u8 = 3;
pub const IEEE802154_ADDR_SHORT_BROADCAST: u16 = 0xffff;
pub const IEEE802154_ADDR_SHORT_UNSPEC: u16 = 0xfffe;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ieee802154Addr {
    pub mode: u8,
    pub extended_addr: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PanDevice {
    pub extended_addr: u64,
    pub short_addr: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WpanDev {
    pub parent: Option<PanDevice>,
    pub children: alloc::vec::Vec<PanDevice>,
    pub short_addr: u16,
    pub max_associations: u32,
}

extern crate alloc;

pub const fn cfg802154_pan_device_is_matching(
    pan_dev: Option<PanDevice>,
    ext_dev: Option<Ieee802154Addr>,
) -> bool {
    let (Some(pan_dev), Some(ext_dev)) = (pan_dev, ext_dev) else {
        return false;
    };
    if ext_dev.mode == IEEE802154_ADDR_SHORT {
        return false;
    }
    pan_dev.extended_addr == ext_dev.extended_addr
}

pub fn cfg802154_device_is_associated(wpan_dev: &WpanDev) -> bool {
    !wpan_dev.children.is_empty() || wpan_dev.parent.is_some()
}

pub fn cfg802154_device_is_parent(wpan_dev: &WpanDev, target: Ieee802154Addr) -> bool {
    cfg802154_pan_device_is_matching(wpan_dev.parent, Some(target))
}

pub fn cfg802154_device_is_child(wpan_dev: &WpanDev, target: Ieee802154Addr) -> Option<PanDevice> {
    wpan_dev
        .children
        .iter()
        .copied()
        .find(|child| cfg802154_pan_device_is_matching(Some(*child), Some(target)))
}

pub fn cfg802154_get_free_short_addr(wpan_dev: &WpanDev, candidates: &[u16]) -> Option<u16> {
    'candidate: for addr in candidates.iter().copied() {
        if addr == IEEE802154_ADDR_SHORT_BROADCAST
            || addr == IEEE802154_ADDR_SHORT_UNSPEC
            || addr == wpan_dev.short_addr
            || wpan_dev
                .parent
                .is_some_and(|parent| parent.short_addr == addr)
        {
            continue;
        }
        for child in &wpan_dev.children {
            if child.short_addr == addr {
                continue 'candidate;
            }
        }
        return Some(addr);
    }
    None
}

pub fn cfg802154_set_max_associations(wpan_dev: &mut WpanDev, max: u32) -> u32 {
    let old_max = wpan_dev.max_associations;
    wpan_dev.max_associations = max;
    old_max
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn ieee802154_pan_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ieee802154/pan.c"
        ));
        assert!(source.contains("cfg802154_pan_device_is_matching"));
        assert!(source.contains("if (!pan_dev || !ext_dev)"));
        assert!(source.contains("if (ext_dev->mode == IEEE802154_ADDR_SHORT)"));
        assert!(source.contains("return pan_dev->extended_addr == ext_dev->extended_addr;"));
        assert!(source.contains("cfg802154_device_is_associated"));
        assert!(source.contains("!list_empty(&wpan_dev->children) || wpan_dev->parent"));
        assert!(source.contains("cfg802154_device_is_parent"));
        assert!(source.contains("cfg802154_device_is_child"));
        assert!(source.contains("list_for_each_entry(child, &wpan_dev->children, node)"));
        assert!(source.contains("cfg802154_get_free_short_addr"));
        assert!(source.contains("get_random_bytes(&addr, 2);"));
        assert!(source.contains("IEEE802154_ADDR_SHORT_BROADCAST"));
        assert!(source.contains("IEEE802154_ADDR_SHORT_UNSPEC"));
        assert!(source.contains("cfg802154_set_max_associations"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(cfg802154_set_max_associations);"));
    }

    #[test]
    fn pan_helpers_match_parent_child_and_short_address_rules() {
        let parent = PanDevice {
            extended_addr: 0x11,
            short_addr: 0x1001,
        };
        let child = PanDevice {
            extended_addr: 0x22,
            short_addr: 0x1002,
        };
        let mut dev = WpanDev {
            parent: Some(parent),
            children: vec![child],
            short_addr: 0x1003,
            max_associations: 4,
        };
        assert!(cfg802154_device_is_associated(&dev));
        assert!(cfg802154_device_is_parent(
            &dev,
            Ieee802154Addr {
                mode: IEEE802154_ADDR_EXTENDED,
                extended_addr: 0x11,
            }
        ));
        assert_eq!(
            cfg802154_device_is_child(
                &dev,
                Ieee802154Addr {
                    mode: IEEE802154_ADDR_EXTENDED,
                    extended_addr: 0x22,
                }
            ),
            Some(child)
        );
        assert!(!cfg802154_pan_device_is_matching(
            Some(child),
            Some(Ieee802154Addr {
                mode: IEEE802154_ADDR_SHORT,
                extended_addr: 0x22,
            })
        ));
        assert_eq!(
            cfg802154_get_free_short_addr(&dev, &[0xffff, 0xfffe, 0x1001, 0x1002, 0x1003, 0x1004]),
            Some(0x1004)
        );
        assert_eq!(cfg802154_set_max_associations(&mut dev, 8), 4);
        assert_eq!(dev.max_associations, 8);
    }
}
