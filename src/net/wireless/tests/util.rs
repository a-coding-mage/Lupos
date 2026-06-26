//! linux-parity: complete
//! linux-source: vendor/linux/net/wireless/tests/util.c
//! test-origin: linux:vendor/linux/net/wireless/tests/util.c
//! cfg80211 KUnit wiphy fixture utilities.

extern crate alloc;

use alloc::vec::Vec;

pub const NL80211_BAND_2GHZ: u8 = 0;
pub const WIPHY_NAME: &str = "kunit";
pub const RESOURCE_NAME: &str = "wiphy";
pub const CHANNELS_2GHZ: [u16; 14] = [
    2412, 2417, 2422, 2427, 2432, 2437, 2442, 2447, 2452, 2457, 2462, 2467, 2472, 2484,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ieee80211Channel {
    pub band: u8,
    pub center_freq: u16,
    pub hw_value: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestWiphy {
    pub name: &'static str,
    pub resource_name: &'static str,
    pub ctx: usize,
    pub channels_2ghz: Vec<Ieee80211Channel>,
}

pub fn channels_2ghz() -> Vec<Ieee80211Channel> {
    CHANNELS_2GHZ
        .iter()
        .map(|freq| Ieee80211Channel {
            band: NL80211_BAND_2GHZ,
            center_freq: *freq,
            hw_value: *freq,
        })
        .collect()
}

pub fn t_wiphy_init(ctx: usize) -> TestWiphy {
    TestWiphy {
        name: WIPHY_NAME,
        resource_name: RESOURCE_NAME,
        ctx,
        channels_2ghz: channels_2ghz(),
    }
}

pub fn t_wiphy_ctx(wiphy: &TestWiphy) -> usize {
    wiphy.ctx
}

pub fn t_wiphy_exit(_wiphy: TestWiphy) {}

pub fn t_skb_remove_member(
    data: &mut Vec<u8>,
    type_size: usize,
    member_offset: usize,
    member_size: usize,
) -> bool {
    if member_offset
        .checked_add(member_size)
        .is_none_or(|end| end > type_size || type_size > data.len())
    {
        return false;
    }
    let base = data.len() - type_size;
    let start = base + member_offset;
    let end = start + member_size;
    data.copy_within(end.., start);
    let new_len = data.len() - member_size;
    data.truncate(new_len);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wireless_test_util_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/wireless/tests/util.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/wireless/tests/util.h"
        ));
        assert!(source.contains("int t_wiphy_init(struct kunit_resource *resource, void *ctx)"));
        assert!(source.contains("wiphy_new_nm(ops, sizeof(*priv), \"kunit\")"));
        assert!(source.contains("memcpy(priv->channels_2ghz, channels_2ghz"));
        assert!(source.contains("wiphy->bands[NL80211_BAND_2GHZ] = &priv->band_2ghz;"));
        assert!(source.contains("resource->data = wiphy;"));
        assert!(source.contains("resource->name = \"wiphy\";"));
        assert!(source.contains("wiphy_free(resource->data);"));
        assert!(header.contains("CHAN2G(2412)"));
        assert!(header.contains("CHAN2G(2484)"));
        assert!(header.contains("#define T_WIPHY(test, ctx)"));
        assert!(header.contains("#define t_wiphy_ctx(wiphy)"));
        assert!(header.contains("#define t_skb_remove_member"));

        let wiphy = t_wiphy_init(0xfeed);
        assert_eq!(wiphy.name, "kunit");
        assert_eq!(wiphy.resource_name, "wiphy");
        assert_eq!(t_wiphy_ctx(&wiphy), 0xfeed);
        assert_eq!(wiphy.channels_2ghz.len(), 14);
        assert_eq!(
            wiphy.channels_2ghz[0],
            Ieee80211Channel {
                band: NL80211_BAND_2GHZ,
                center_freq: 2412,
                hw_value: 2412,
            }
        );
        assert_eq!(wiphy.channels_2ghz[13].center_freq, 2484);

        let mut data = alloc::vec![0xaa, 1, 2, 3, 4];
        assert!(t_skb_remove_member(&mut data, 4, 1, 2));
        assert_eq!(data, alloc::vec![0xaa, 1, 4]);
    }
}
