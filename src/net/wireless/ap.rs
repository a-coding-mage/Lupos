//! linux-parity: complete
//! linux-source: vendor/linux/net/wireless/ap.c
//! test-origin: linux:vendor/linux/net/wireless/ap.c
//! cfg80211 AP stop control path.

use crate::include::uapi::errno::{ENOENT, EOPNOTSUPP};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IfType {
    Ap,
    P2pGo,
    Station,
    Ocb,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChannelDef {
    pub chan: Option<u32>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ApLink {
    pub beacon_interval: u16,
    pub chandef: ChannelDef,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cfg80211RegisteredDevice {
    pub stop_ap: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WirelessDev<const N: usize> {
    pub iftype: IfType,
    pub links: [ApLink; N],
    pub valid_links: [bool; N],
    pub conn_owner_nlportid: u32,
    pub ssid_len: usize,
    pub qos_map_set: bool,
    pub disconnect_work_scheduled: u32,
    pub ap_stopped_notifications: u32,
    pub dfs_updates: u32,
}

pub fn cfg80211_stop_ap<const N: usize>(
    rdev: Cfg80211RegisteredDevice,
    wdev: &mut WirelessDev<N>,
    link_id: i32,
    notify: bool,
    driver_rcs: &[i32],
) -> Result<(), i32> {
    if link_id >= 0 {
        return ___cfg80211_stop_ap(
            rdev,
            wdev,
            link_id as usize,
            notify,
            driver_rcs.first().copied().unwrap_or(0),
        );
    }

    let mut ret = Ok(());
    for link in 0..N {
        if wdev.valid_links[link] {
            let ret1 = ___cfg80211_stop_ap(
                rdev,
                wdev,
                link,
                notify,
                driver_rcs.get(link).copied().unwrap_or(0),
            );
            if ret1.is_err() {
                ret = ret1;
            }
        }
    }
    ret
}

pub fn ___cfg80211_stop_ap<const N: usize>(
    rdev: Cfg80211RegisteredDevice,
    wdev: &mut WirelessDev<N>,
    link_id: usize,
    notify: bool,
    driver_rc: i32,
) -> Result<(), i32> {
    if !rdev.stop_ap {
        return Err(-EOPNOTSUPP);
    }
    if wdev.iftype != IfType::Ap && wdev.iftype != IfType::P2pGo {
        return Err(-EOPNOTSUPP);
    }
    let Some(link) = wdev.links.get_mut(link_id) else {
        return Err(-ENOENT);
    };
    if link.beacon_interval == 0 {
        return Err(-ENOENT);
    }

    if driver_rc == 0 {
        wdev.conn_owner_nlportid = 0;
        link.beacon_interval = 0;
        link.chandef = ChannelDef::default();
        wdev.ssid_len = 0;
        wdev.qos_map_set = false;
        if notify {
            wdev.ap_stopped_notifications = wdev.ap_stopped_notifications.saturating_add(1);
        }
        wdev.dfs_updates = wdev.dfs_updates.saturating_add(1);
    }
    wdev.disconnect_work_scheduled = wdev.disconnect_work_scheduled.saturating_add(1);

    if driver_rc == 0 {
        Ok(())
    } else {
        Err(driver_rc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wdev(iftype: IfType) -> WirelessDev<2> {
        WirelessDev {
            iftype,
            links: [
                ApLink {
                    beacon_interval: 100,
                    chandef: ChannelDef { chan: Some(36) },
                },
                ApLink {
                    beacon_interval: 200,
                    chandef: ChannelDef { chan: Some(40) },
                },
            ],
            valid_links: [true, true],
            conn_owner_nlportid: 7,
            ssid_len: 4,
            qos_map_set: true,
            disconnect_work_scheduled: 0,
            ap_stopped_notifications: 0,
            dfs_updates: 0,
        }
    }

    #[test]
    fn wireless_ap_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/wireless/ap.c"
        ));
        assert!(source.contains("static int ___cfg80211_stop_ap"));
        assert!(source.contains("if (!rdev->ops->stop_ap)"));
        assert!(source.contains("return -EOPNOTSUPP;"));
        assert!(source.contains("if (dev->ieee80211_ptr->iftype != NL80211_IFTYPE_AP"));
        assert!(source.contains("NL80211_IFTYPE_P2P_GO"));
        assert!(source.contains("if (!wdev->links[link_id].ap.beacon_interval)"));
        assert!(source.contains("return -ENOENT;"));
        assert!(source.contains("err = rdev_stop_ap(rdev, dev, link_id);"));
        assert!(source.contains("wdev->conn_owner_nlportid = 0;"));
        assert!(source.contains("wdev->links[link_id].ap.beacon_interval = 0;"));
        assert!(source.contains("memset(&wdev->links[link_id].ap.chandef, 0"));
        assert!(source.contains("wdev->u.ap.ssid_len = 0;"));
        assert!(source.contains("rdev_set_qos_map(rdev, dev, NULL);"));
        assert!(source.contains("nl80211_send_ap_stopped(wdev, link_id);"));
        assert!(source.contains("cfg80211_sched_dfs_chan_update(rdev);"));
        assert!(source.contains("schedule_work(&cfg80211_disconnect_work);"));
        assert!(source.contains("for_each_valid_link(dev->ieee80211_ptr, link)"));
    }

    #[test]
    fn stop_ap_resets_ap_state_only_after_driver_success() {
        let rdev = Cfg80211RegisteredDevice { stop_ap: true };
        let mut sta = wdev(IfType::Station);
        assert_eq!(
            ___cfg80211_stop_ap(rdev, &mut sta, 0, true, 0),
            Err(-EOPNOTSUPP)
        );
        assert_eq!(sta.disconnect_work_scheduled, 0);

        let mut ap = wdev(IfType::Ap);
        assert_eq!(___cfg80211_stop_ap(rdev, &mut ap, 0, true, -5), Err(-5));
        assert_eq!(ap.links[0].beacon_interval, 100);
        assert_eq!(ap.disconnect_work_scheduled, 1);

        assert_eq!(___cfg80211_stop_ap(rdev, &mut ap, 0, true, 0), Ok(()));
        assert_eq!(ap.links[0], ApLink::default());
        assert_eq!(ap.conn_owner_nlportid, 0);
        assert_eq!(ap.ssid_len, 0);
        assert!(!ap.qos_map_set);
        assert_eq!(ap.ap_stopped_notifications, 1);
        assert_eq!(ap.dfs_updates, 1);
    }

    #[test]
    fn stop_all_links_keeps_trying_and_returns_last_error() {
        let rdev = Cfg80211RegisteredDevice { stop_ap: true };
        let mut ap = wdev(IfType::P2pGo);
        assert_eq!(
            cfg80211_stop_ap(rdev, &mut ap, -1, false, &[0, -9]),
            Err(-9)
        );
        assert_eq!(ap.links[0].beacon_interval, 0);
        assert_eq!(ap.links[1].beacon_interval, 200);
        assert_eq!(ap.disconnect_work_scheduled, 2);
    }
}
