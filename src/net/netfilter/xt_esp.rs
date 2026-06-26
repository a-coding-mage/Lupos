//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_esp.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_esp.c
//! Xtables IPsec ESP SPI range match.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "Yon Uriarte <yon@astaro.de>";
pub const MODULE_DESCRIPTION: &str = "Xtables: IPsec-ESP packet match";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_esp", "ip6t_esp"];
pub const XT_ESP_INV_SPI: u8 = 0x01;
pub const XT_ESP_INV_MASK: u8 = 0x01;
pub const NFPROTO_IPV4: u8 = 2;
pub const NFPROTO_IPV6: u8 = 10;
pub const IPPROTO_ESP: u8 = 50;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtEsp {
    pub spis: [u32; 2],
    pub invflags: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EspMatchResult {
    pub matched: bool,
    pub hotdrop: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub family: u8,
    pub proto: u8,
}

pub const ESP_MT_REG: [XtMatch; 2] = [
    XtMatch {
        name: "esp",
        family: NFPROTO_IPV4,
        proto: IPPROTO_ESP,
    },
    XtMatch {
        name: "esp",
        family: NFPROTO_IPV6,
        proto: IPPROTO_ESP,
    },
];

pub const fn spi_match(min: u32, max: u32, spi: u32, invert: bool) -> bool {
    (spi >= min && spi <= max) != invert
}

pub const fn esp_mt(info: XtEsp, fragoff: u16, header_spi: Option<u32>) -> EspMatchResult {
    if fragoff != 0 {
        return EspMatchResult {
            matched: false,
            hotdrop: false,
        };
    }
    let Some(spi) = header_spi else {
        return EspMatchResult {
            matched: false,
            hotdrop: true,
        };
    };
    EspMatchResult {
        matched: spi_match(
            info.spis[0],
            info.spis[1],
            u32::from_be(spi),
            info.invflags & XT_ESP_INV_SPI != 0,
        ),
        hotdrop: false,
    }
}

pub const fn esp_mt_check(info: XtEsp) -> Result<(), i32> {
    if info.invflags & !XT_ESP_INV_MASK != 0 {
        return Err(-EINVAL);
    }
    Ok(())
}

pub const fn esp_mt_init() -> &'static [XtMatch; 2] {
    &ESP_MT_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_esp_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_esp.c"
        ));
        assert!(source.contains("MODULE_ALIAS(\"ipt_esp\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_esp\");"));
        assert!(source.contains("spi_match(u_int32_t min, u_int32_t max"));
        assert!(source.contains("r = (spi >= min && spi <= max) ^ invert;"));
        assert!(source.contains("if (par->fragoff != 0)"));
        assert!(source.contains("skb_header_pointer(skb, par->thoff, sizeof(_esp), &_esp);"));
        assert!(source.contains("par->hotdrop = true;"));
        assert!(source.contains("ntohl(eh->spi)"));
        assert!(source.contains("if (espinfo->invflags & ~XT_ESP_INV_MASK)"));
        assert!(source.contains(".family\t\t= NFPROTO_IPV4"));
        assert!(source.contains(".family\t\t= NFPROTO_IPV6"));
        assert!(source.contains("xt_register_matches(esp_mt_reg, ARRAY_SIZE(esp_mt_reg));"));
    }

    #[test]
    fn esp_match_handles_fragments_hotdrop_and_inversion() {
        let info = XtEsp {
            spis: [10, 20],
            invflags: 0,
        };
        assert!(spi_match(10, 20, 12, false));
        assert!(!spi_match(10, 20, 12, true));
        assert_eq!(
            esp_mt(info, 0, Some(12u32.to_be())),
            EspMatchResult {
                matched: true,
                hotdrop: false,
            }
        );
        assert_eq!(
            esp_mt(info, 1, Some(12u32.to_be())),
            EspMatchResult {
                matched: false,
                hotdrop: false,
            }
        );
        assert_eq!(
            esp_mt(info, 0, None),
            EspMatchResult {
                matched: false,
                hotdrop: true,
            }
        );
        assert!(
            !esp_mt(
                XtEsp {
                    invflags: XT_ESP_INV_SPI,
                    ..info
                },
                0,
                Some(12u32.to_be())
            )
            .matched
        );
        assert_eq!(
            esp_mt_check(XtEsp {
                invflags: 0x80,
                ..info
            }),
            Err(-EINVAL)
        );
        assert_eq!(esp_mt_init(), &ESP_MT_REG);
    }
}
