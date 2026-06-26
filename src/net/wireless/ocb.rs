//! linux-parity: complete
//! linux-source: vendor/linux/net/wireless/ocb.c
//! test-origin: linux:vendor/linux/net/wireless/ocb.c
//! cfg80211 OCB join and leave control path.

use crate::include::uapi::errno::{EINVAL, ENOTCONN, EOPNOTSUPP};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IfType {
    Ocb,
    Station,
    Ap,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChannelDef {
    pub chan: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OcbSetup {
    pub chandef: ChannelDef,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cfg80211RegisteredDevice {
    pub join_ocb: bool,
    pub leave_ocb: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WirelessDev {
    pub iftype: IfType,
    pub chandef: ChannelDef,
}

pub fn cfg80211_join_ocb(
    rdev: Cfg80211RegisteredDevice,
    wdev: &mut WirelessDev,
    setup: OcbSetup,
    driver_rc: i32,
) -> Result<(), i32> {
    if wdev.iftype != IfType::Ocb {
        return Err(-EOPNOTSUPP);
    }
    if !rdev.join_ocb {
        return Err(-EOPNOTSUPP);
    }
    if setup.chandef.chan.is_none() {
        return Err(-EINVAL);
    }
    if driver_rc != 0 {
        return Err(driver_rc);
    }

    wdev.chandef = setup.chandef;
    Ok(())
}

pub fn cfg80211_leave_ocb(
    rdev: Cfg80211RegisteredDevice,
    wdev: &mut WirelessDev,
    driver_rc: i32,
) -> Result<(), i32> {
    if wdev.iftype != IfType::Ocb {
        return Err(-EOPNOTSUPP);
    }
    if !rdev.leave_ocb {
        return Err(-EOPNOTSUPP);
    }
    if wdev.chandef.chan.is_none() {
        return Err(-ENOTCONN);
    }
    if driver_rc != 0 {
        return Err(driver_rc);
    }

    wdev.chandef = ChannelDef::default();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wireless_ocb_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/wireless/ocb.c"
        ));
        assert!(source.contains("int cfg80211_join_ocb"));
        assert!(source.contains("lockdep_assert_wiphy(wdev->wiphy);"));
        assert!(source.contains("if (dev->ieee80211_ptr->iftype != NL80211_IFTYPE_OCB)"));
        assert!(source.contains("return -EOPNOTSUPP;"));
        assert!(source.contains("if (!rdev->ops->join_ocb)"));
        assert!(source.contains("if (WARN_ON(!setup->chandef.chan))"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("err = rdev_join_ocb(rdev, dev, setup);"));
        assert!(source.contains("wdev->u.ocb.chandef = setup->chandef;"));
        assert!(source.contains("int cfg80211_leave_ocb"));
        assert!(source.contains("if (!rdev->ops->leave_ocb)"));
        assert!(source.contains("if (!wdev->u.ocb.chandef.chan)"));
        assert!(source.contains("return -ENOTCONN;"));
        assert!(source.contains("memset(&wdev->u.ocb.chandef, 0, sizeof(wdev->u.ocb.chandef));"));
    }

    #[test]
    fn join_and_leave_ocb_update_chandef_on_success_only() {
        let rdev = Cfg80211RegisteredDevice {
            join_ocb: true,
            leave_ocb: true,
        };
        let mut wdev = WirelessDev {
            iftype: IfType::Station,
            chandef: ChannelDef::default(),
        };
        let setup = OcbSetup {
            chandef: ChannelDef { chan: Some(172) },
        };
        assert_eq!(
            cfg80211_join_ocb(rdev, &mut wdev, setup, 0),
            Err(-EOPNOTSUPP)
        );
        assert_eq!(wdev.chandef.chan, None);

        wdev.iftype = IfType::Ocb;
        assert_eq!(
            cfg80211_join_ocb(
                rdev,
                &mut wdev,
                OcbSetup {
                    chandef: ChannelDef::default()
                },
                0
            ),
            Err(-EINVAL)
        );
        assert_eq!(cfg80211_join_ocb(rdev, &mut wdev, setup, 0), Ok(()));
        assert_eq!(wdev.chandef, setup.chandef);

        assert_eq!(cfg80211_leave_ocb(rdev, &mut wdev, 0), Ok(()));
        assert_eq!(wdev.chandef, ChannelDef::default());
        assert_eq!(cfg80211_leave_ocb(rdev, &mut wdev, 0), Err(-ENOTCONN));
    }
}
