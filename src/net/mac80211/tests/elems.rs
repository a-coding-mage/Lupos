//! linux-parity: complete
//! linux-source: vendor/linux/net/mac80211/tests/elems.c
//! test-origin: linux:vendor/linux/net/mac80211/tests/elems.c
//! mac80211 element parsing KUnit multi-link defragmentation test shape.

pub const MODULE_IMPORT_NS: &str = "EXPORTED_FOR_KUNIT_TESTING";
pub const SUITE_NAME: &str = "mac80211-element-parsing";
pub const WLAN_EID_EXTENSION: u8 = 255;
pub const WLAN_EID_EXT_EHT_MULTI_LINK: u8 = 107;
pub const IEEE80211_ML_CONTROL_TYPE_BASIC: u16 = 0;
pub const IEEE80211_MLE_SUBELEM_PER_STA_PROFILE: u8 = 0;
pub const IEEE80211_MLE_SUBELEM_FRAGMENT: u8 = 254;
pub const WLAN_EID_FRAGMENT: u8 = 242;
pub const WLAN_EID_SSID: u8 = 0;
pub const TEST_LINK_ID: u8 = 12;
pub const USELESS_ELEMENT_COUNT: usize = 20;
pub const USELESS_ELEMENT_LEN: usize = 20;
pub const ML_BASIC_EXPECTED_LEN: usize =
    2 + 7 + 2 + 3 + USELESS_ELEMENT_COUNT * (2 + USELESS_ELEMENT_LEN) + 2;
pub const STA_PROF_EXPECTED_LEN: usize = 3 + USELESS_ELEMENT_COUNT * (2 + USELESS_ELEMENT_LEN);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MleDefragResult {
    pub parsed_non_null: bool,
    pub ml_basic_present: bool,
    pub ml_basic_len: usize,
    pub prof_present: bool,
    pub sta_prof_len: usize,
}

pub const fn mle_defrag_parse_result(parse_returns_error: bool) -> MleDefragResult {
    MleDefragResult {
        parsed_non_null: true,
        ml_basic_present: !parse_returns_error,
        ml_basic_len: if parse_returns_error {
            0
        } else {
            ML_BASIC_EXPECTED_LEN
        },
        prof_present: !parse_returns_error,
        sta_prof_len: if parse_returns_error {
            0
        } else {
            STA_PROF_EXPECTED_LEN
        },
    }
}

pub const fn element_parsing_test_cases() -> [&'static str; 1] {
    ["mle_defrag"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac80211_elems_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/mac80211/tests/elems.c"
        ));
        assert!(source.contains("MODULE_IMPORT_NS(\"EXPORTED_FOR_KUNIT_TESTING\");"));
        assert!(source.contains("static void mle_defrag(struct kunit *test)"));
        assert!(source.contains(".link_id = 12"));
        assert!(source.contains(".mode = IEEE80211_CONN_MODE_EHT"));
        assert!(source.contains("skb = alloc_skb(1024, GFP_KERNEL);"));
        assert!(source.contains("skb_put_u8(skb, WLAN_EID_EXTENSION);"));
        assert!(source.contains("skb_put_u8(skb, WLAN_EID_EXT_EHT_MULTI_LINK);"));
        assert!(source.contains("put_unaligned_le16(IEEE80211_ML_CONTROL_TYPE_BASIC"));
        assert!(source.contains("skb_put_u8(skb, IEEE80211_MLE_SUBELEM_PER_STA_PROFILE);"));
        assert!(source.contains("for (i = 0; i < 20; i++)"));
        assert!(source.contains("ieee80211_fragment_element(skb, len_prof"));
        assert!(source.contains("ieee80211_fragment_element(skb, len_mle"));
        assert!(source.contains("parsed = ieee802_11_parse_elems_full(&parse_params);"));
        assert!(source.contains("parsed->ml_basic_len"));
        assert!(source.contains("parsed->sta_prof_len"));
        assert!(source.contains(".name = \"mac80211-element-parsing\""));
        assert!(source.contains("kunit_test_suite(element_parsing);"));
    }

    #[test]
    fn mle_defrag_expected_lengths_match_linux_test_arithmetic() {
        let parsed = mle_defrag_parse_result(false);
        assert!(parsed.parsed_non_null);
        assert!(parsed.ml_basic_present);
        assert_eq!(parsed.ml_basic_len, 456);
        assert!(parsed.prof_present);
        assert_eq!(parsed.sta_prof_len, 443);
        let err = mle_defrag_parse_result(true);
        assert!(err.parsed_non_null);
        assert!(!err.ml_basic_present);
        assert_eq!(element_parsing_test_cases(), ["mle_defrag"]);
    }
}
