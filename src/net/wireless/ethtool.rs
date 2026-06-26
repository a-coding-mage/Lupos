//! linux-parity: complete
//! linux-source: vendor/linux/net/wireless/ethtool.c
//! test-origin: linux:vendor/linux/net/wireless/ethtool.c
//! cfg80211 ethtool driver-info projection.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cfg80211DrvInfo<'a> {
    pub driver: &'a str,
    pub version: &'a str,
    pub fw_version: &'a str,
    pub bus_info: &'a str,
}

pub fn cfg80211_get_drvinfo<'a>(
    driver_name: Option<&'a str>,
    uts_release: &'a str,
    fw_version: Option<&'a str>,
    bus_info: &'a str,
) -> Cfg80211DrvInfo<'a> {
    Cfg80211DrvInfo {
        driver: driver_name.unwrap_or("N/A"),
        version: uts_release,
        fw_version: fw_version
            .filter(|version| !version.is_empty())
            .unwrap_or("N/A"),
        bus_info,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cfg80211_get_drvinfo_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/wireless/ethtool.c"
        ));
        assert!(source.contains("#include <linux/utsname.h>"));
        assert!(source.contains("#include <net/cfg80211.h>"));
        assert!(source.contains("strscpy(info->driver"));
        assert!(source.contains("strscpy(info->version, init_utsname()->release"));
        assert!(source.contains("strscpy(info->fw_version"));
        assert!(source.contains("strscpy(info->bus_info, dev_name(pdev)"));
        assert!(source.contains("EXPORT_SYMBOL(cfg80211_get_drvinfo);"));
        assert_eq!(
            cfg80211_get_drvinfo(Some("iwlwifi"), "6.17.0", Some("fw"), "pci0"),
            Cfg80211DrvInfo {
                driver: "iwlwifi",
                version: "6.17.0",
                fw_version: "fw",
                bus_info: "pci0"
            }
        );
        assert_eq!(
            cfg80211_get_drvinfo(None, "6.17.0", Some(""), "platform0").driver,
            "N/A"
        );
        assert_eq!(
            cfg80211_get_drvinfo(None, "6.17.0", Some(""), "platform0").fw_version,
            "N/A"
        );
    }
}
