//! linux-parity: complete
//! linux-source: vendor/linux/net/mac80211/uhr.c
//! test-origin: linux:vendor/linux/net/mac80211/uhr.c
//! mac80211 UHR capability import.

use crate::include::uapi::errno::EINVAL;

pub const IEEE80211_UHR_MAC_CAP1_DBE_SUPP: u8 = 0x04;
pub const IEEE80211_UHR_MAC_CAP_DBE_EHT_MCS_MAP_160_PRES: u8 = 0x08;
pub const IEEE80211_UHR_MAC_CAP_DBE_EHT_MCS_MAP_320_PRES: u8 = 0x10;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ieee80211UhrCapMac {
    pub mac_cap: [u8; 5],
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ieee80211UhrCapPhy {
    pub cap: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ieee80211UhrCap<'a> {
    pub mac: Ieee80211UhrCapMac,
    pub variable: &'a [u8],
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ieee80211StaUhrCap {
    pub has_uhr: bool,
    pub mac: Ieee80211UhrCapMac,
    pub phy: Ieee80211UhrCapPhy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Nl80211IfType {
    Station,
    Ap,
    Other(u8),
}

pub fn ieee80211_uhr_phy_cap(
    cap: &Ieee80211UhrCap<'_>,
    from_ap: bool,
) -> Option<Ieee80211UhrCapPhy> {
    let mut offset = 0usize;

    if from_ap && cap.mac.mac_cap[1] & IEEE80211_UHR_MAC_CAP1_DBE_SUPP != 0 {
        let dbe = *cap.variable.first()?;
        offset += 1;

        if dbe & IEEE80211_UHR_MAC_CAP_DBE_EHT_MCS_MAP_160_PRES != 0 {
            offset += 3;
        }

        if dbe & IEEE80211_UHR_MAC_CAP_DBE_EHT_MCS_MAP_320_PRES != 0 {
            offset += 3;
        }
    }

    cap.variable
        .get(offset)
        .copied()
        .map(|cap| Ieee80211UhrCapPhy { cap })
}

pub fn ieee80211_uhr_cap_ie_to_sta_uhr_cap(
    sband_has_uhr_iftype_cap: bool,
    iftype: Nl80211IfType,
    uhr_cap: &Ieee80211UhrCap<'_>,
    _uhr_cap_len: u8,
    sta_uhr_cap: &mut Ieee80211StaUhrCap,
) -> Result<(), i32> {
    *sta_uhr_cap = Ieee80211StaUhrCap::default();

    if !sband_has_uhr_iftype_cap {
        return Ok(());
    }

    sta_uhr_cap.has_uhr = true;
    sta_uhr_cap.mac = uhr_cap.mac;
    sta_uhr_cap.phy =
        ieee80211_uhr_phy_cap(uhr_cap, iftype == Nl80211IfType::Station).ok_or(EINVAL)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uhr_cap_ie_to_sta_uhr_cap_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/mac80211/uhr.c"
        ));
        assert!(source.contains("ieee80211_uhr_cap_ie_to_sta_uhr_cap"));
        assert!(source.contains("memset(sta_uhr_cap, 0, sizeof(*sta_uhr_cap));"));
        assert!(source.contains("if (!ieee80211_get_uhr_iftype_cap_vif(sband, &sdata->vif))"));
        assert!(source.contains("sta_uhr_cap->has_uhr = true;"));
        assert!(source.contains("sta_uhr_cap->mac = uhr_cap->mac;"));
        assert!(source.contains("from_ap = sdata->vif.type == NL80211_IFTYPE_STATION;"));
        assert!(source.contains("sta_uhr_cap->phy = *ieee80211_uhr_phy_cap(uhr_cap, from_ap);"));

        let mac = Ieee80211UhrCapMac {
            mac_cap: [0xaa, IEEE80211_UHR_MAC_CAP1_DBE_SUPP, 0, 0, 0],
        };
        let cap = Ieee80211UhrCap {
            mac,
            variable: &[
                IEEE80211_UHR_MAC_CAP_DBE_EHT_MCS_MAP_160_PRES,
                1,
                2,
                3,
                0x44,
            ],
        };
        let mut sta = Ieee80211StaUhrCap {
            has_uhr: true,
            mac: Ieee80211UhrCapMac { mac_cap: [9; 5] },
            phy: Ieee80211UhrCapPhy { cap: 9 },
        };

        ieee80211_uhr_cap_ie_to_sta_uhr_cap(
            true,
            Nl80211IfType::Station,
            &cap,
            cap.variable.len() as u8,
            &mut sta,
        )
        .unwrap();
        assert_eq!(
            sta,
            Ieee80211StaUhrCap {
                has_uhr: true,
                mac,
                phy: Ieee80211UhrCapPhy { cap: 0x44 },
            }
        );
    }

    #[test]
    fn uhr_cap_import_clears_sta_when_sband_lacks_uhr() {
        let cap = Ieee80211UhrCap {
            mac: Ieee80211UhrCapMac { mac_cap: [1; 5] },
            variable: &[0x55],
        };
        let mut sta = Ieee80211StaUhrCap {
            has_uhr: true,
            mac: Ieee80211UhrCapMac { mac_cap: [9; 5] },
            phy: Ieee80211UhrCapPhy { cap: 9 },
        };

        ieee80211_uhr_cap_ie_to_sta_uhr_cap(
            false,
            Nl80211IfType::Ap,
            &cap,
            cap.variable.len() as u8,
            &mut sta,
        )
        .unwrap();
        assert_eq!(sta, Ieee80211StaUhrCap::default());
    }

    #[test]
    fn uhr_phy_offset_uses_ap_dbe_extension_layout() {
        let cap = Ieee80211UhrCap {
            mac: Ieee80211UhrCapMac {
                mac_cap: [0, IEEE80211_UHR_MAC_CAP1_DBE_SUPP, 0, 0, 0],
            },
            variable: &[
                IEEE80211_UHR_MAC_CAP_DBE_EHT_MCS_MAP_160_PRES
                    | IEEE80211_UHR_MAC_CAP_DBE_EHT_MCS_MAP_320_PRES,
                0x10,
                0x11,
                0x12,
                0x20,
                0x21,
                0x22,
                0x77,
            ],
        };

        assert_eq!(
            ieee80211_uhr_phy_cap(&cap, true),
            Some(Ieee80211UhrCapPhy { cap: 0x77 })
        );
        assert_eq!(
            ieee80211_uhr_phy_cap(&cap, false),
            Some(Ieee80211UhrCapPhy {
                cap: IEEE80211_UHR_MAC_CAP_DBE_EHT_MCS_MAP_160_PRES
                    | IEEE80211_UHR_MAC_CAP_DBE_EHT_MCS_MAP_320_PRES
            })
        );
    }
}
