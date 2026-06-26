//! linux-parity: complete
//! linux-source: vendor/linux/net/mac80211/wbrf.c
//! test-origin: linux:vendor/linux/net/mac80211/wbrf.c
//! mac80211 WBRF frequency range registration.

pub const KHZ_PER_MHZ: u64 = 1_000;
pub const HZ_PER_KHZ: u64 = 1_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChanWidth {
    Mhz20,
    Mhz40,
    Mhz80,
    Mhz80P80,
    Mhz160,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cfg80211ChanDef {
    pub center_freq1: u32,
    pub center_freq2: u32,
    pub width: ChanWidth,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WbrfRange {
    pub start: u64,
    pub end: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WbrfRangesInOut {
    pub band_list: [Option<WbrfRange>; 2],
    pub num_of_ranges: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WbrfRecord {
    Add,
    Remove,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WbrfAction {
    pub record: WbrfRecord,
    pub ranges: WbrfRangesInOut,
}

pub const fn ieee80211_check_wbrf_support(
    has_wiphy: bool,
    has_parent_dev: bool,
    acpi_supported: bool,
) -> bool {
    has_wiphy && has_parent_dev && acpi_supported
}

pub const fn get_chan_freq_boundary(center_freq_mhz: u32, bandwidth_mhz: u32) -> WbrfRange {
    let center_khz = center_freq_mhz as u64 * KHZ_PER_MHZ;
    let bandwidth_khz = bandwidth_mhz as u64 * KHZ_PER_MHZ;
    WbrfRange {
        start: (center_khz - bandwidth_khz / 2) * HZ_PER_KHZ,
        end: (center_khz + bandwidth_khz / 2) * HZ_PER_KHZ,
    }
}

pub const fn cfg80211_chandef_get_width(chandef: Cfg80211ChanDef) -> u32 {
    match chandef.width {
        ChanWidth::Mhz20 => 20,
        ChanWidth::Mhz40 => 40,
        ChanWidth::Mhz80 | ChanWidth::Mhz80P80 => 80,
        ChanWidth::Mhz160 => 160,
    }
}

pub const fn get_ranges_from_chandef(chandef: Cfg80211ChanDef) -> WbrfRangesInOut {
    let bandwidth = cfg80211_chandef_get_width(chandef);
    let first = get_chan_freq_boundary(chandef.center_freq1, bandwidth);
    if let ChanWidth::Mhz80P80 = chandef.width {
        let second = get_chan_freq_boundary(chandef.center_freq2, bandwidth);
        WbrfRangesInOut {
            band_list: [Some(first), Some(second)],
            num_of_ranges: 2,
        }
    } else {
        WbrfRangesInOut {
            band_list: [Some(first), None],
            num_of_ranges: 1,
        }
    }
}

pub const fn ieee80211_add_wbrf(supported: bool, chandef: Cfg80211ChanDef) -> Option<WbrfAction> {
    if !supported {
        return None;
    }
    Some(WbrfAction {
        record: WbrfRecord::Add,
        ranges: get_ranges_from_chandef(chandef),
    })
}

pub const fn ieee80211_remove_wbrf(
    supported: bool,
    chandef: Cfg80211ChanDef,
) -> Option<WbrfAction> {
    if !supported {
        return None;
    }
    Some(WbrfAction {
        record: WbrfRecord::Remove,
        ranges: get_ranges_from_chandef(chandef),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wbrf_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/mac80211/wbrf.c"
        ));
        assert!(source.contains("Wifi Band Exclusion Interface for WLAN"));
        assert!(source.contains("void ieee80211_check_wbrf_support"));
        assert!(source.contains("if (!wiphy)"));
        assert!(source.contains("dev = wiphy->dev.parent;"));
        assert!(source.contains("if (!dev)"));
        assert!(source.contains("local->wbrf_supported = acpi_amd_wbrf_supported_producer(dev);"));
        assert!(source.contains("static void get_chan_freq_boundary"));
        assert!(source.contains("bandwidth *= KHZ_PER_MHZ;"));
        assert!(source.contains("center_freq *= KHZ_PER_MHZ;"));
        assert!(source.contains("*start = center_freq - bandwidth / 2;"));
        assert!(source.contains("*end = center_freq + bandwidth / 2;"));
        assert!(source.contains("*start = *start * HZ_PER_KHZ;"));
        assert!(source.contains("get_ranges_from_chandef"));
        assert!(source.contains("bandwidth = cfg80211_chandef_get_width(chandef);"));
        assert!(source.contains("ranges_in->num_of_ranges = 1;"));
        assert!(source.contains("if (chandef->width == NL80211_CHAN_WIDTH_80P80)"));
        assert!(source.contains("ranges_in->num_of_ranges++;"));
        assert!(source.contains("void ieee80211_add_wbrf"));
        assert!(source.contains("if (!local->wbrf_supported)"));
        assert!(source.contains("acpi_amd_wbrf_add_remove(dev, WBRF_RECORD_ADD, &ranges_in);"));
        assert!(source.contains("acpi_amd_wbrf_add_remove(dev, WBRF_RECORD_REMOVE, &ranges_in);"));
    }

    #[test]
    fn wbrf_ranges_are_hz_boundaries_and_80p80_has_two_ranges() {
        assert!(!ieee80211_check_wbrf_support(false, true, true));
        assert!(ieee80211_check_wbrf_support(true, true, true));
        assert_eq!(
            get_chan_freq_boundary(5_180, 80),
            WbrfRange {
                start: 5_140_000_000,
                end: 5_220_000_000,
            }
        );
        let chandef = Cfg80211ChanDef {
            center_freq1: 5_180,
            center_freq2: 5_290,
            width: ChanWidth::Mhz80P80,
        };
        let ranges = get_ranges_from_chandef(chandef);
        assert_eq!(ranges.num_of_ranges, 2);
        assert_eq!(ranges.band_list[1].unwrap().start, 5_250_000_000);
        assert_eq!(ieee80211_add_wbrf(false, chandef), None);
        assert_eq!(
            ieee80211_add_wbrf(true, chandef).unwrap().record,
            WbrfRecord::Add
        );
        assert_eq!(
            ieee80211_remove_wbrf(true, chandef).unwrap().record,
            WbrfRecord::Remove
        );
    }
}
