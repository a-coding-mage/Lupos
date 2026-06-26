//! linux-parity: complete
//! linux-source: vendor/linux/net/bridge/netfilter/ebt_802_3.c
//! test-origin: linux:vendor/linux/net/bridge/netfilter/ebt_802_3.c
//! Ebtables DSAP/SSAP and SNAP type match.

use crate::include::uapi::errno::EINVAL;

pub const EBT_802_3_SAP: u8 = 0x01;
pub const EBT_802_3_TYPE: u8 = 0x02;
pub const EBT_802_3_MASK: u8 = EBT_802_3_SAP | EBT_802_3_TYPE;
pub const CHECK_TYPE: u8 = 0xaa;
pub const IS_UI: u8 = 0x03;
pub const NFPROTO_BRIDGE: u8 = 7;
pub const MODULE_DESCRIPTION: &str = "Ebtables: DSAP/SSAP field and SNAP type matching";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ebt8023Header {
    pub dsap: u8,
    pub ssap: u8,
    pub ctrl: u8,
    pub ui_type: u16,
    pub ni_type: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ebt8023Info {
    pub sap: u8,
    pub type_: u16,
    pub bitmask: u8,
    pub invflags: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
    pub matchsize: usize,
}

pub const EBT_802_3_MT_REG: XtMatch = XtMatch {
    name: "802_3",
    revision: 0,
    family: NFPROTO_BRIDGE,
    matchsize: core::mem::size_of::<Ebt8023Info>(),
};

pub fn ebt_802_3_mt(hdr: Ebt8023Header, info: Ebt8023Info) -> bool {
    let frame_type = if hdr.ctrl & IS_UI != 0 {
        hdr.ui_type
    } else {
        hdr.ni_type
    };

    if info.bitmask & EBT_802_3_SAP != 0 {
        if nf_invf(info, EBT_802_3_SAP, info.sap != hdr.ssap) {
            return false;
        }
        if nf_invf(info, EBT_802_3_SAP, info.sap != hdr.dsap) {
            return false;
        }
    }

    if info.bitmask & EBT_802_3_TYPE != 0 {
        if !(hdr.dsap == CHECK_TYPE && hdr.ssap == CHECK_TYPE) {
            return false;
        }
        if nf_invf(info, EBT_802_3_TYPE, info.type_ != frame_type) {
            return false;
        }
    }

    true
}

pub fn ebt_802_3_mt_check(info: Ebt8023Info) -> Result<(), i32> {
    if info.bitmask & !EBT_802_3_MASK != 0 || info.invflags & !EBT_802_3_MASK != 0 {
        Err(-EINVAL)
    } else {
        Ok(())
    }
}

pub const fn ebt_802_3_init() -> &'static XtMatch {
    &EBT_802_3_MT_REG
}

const fn nf_invf(info: Ebt8023Info, flag: u8, boolean: bool) -> bool {
    boolean != (info.invflags & flag != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ebt_802_3_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/bridge/netfilter/ebt_802_3.c"
        ));
        assert!(source.contains("static struct ebt_802_3_hdr *ebt_802_3_hdr"));
        assert!(source.contains("skb_mac_header(skb)"));
        assert!(source.contains("hdr->llc.ui.ctrl & IS_UI ? hdr->llc.ui.type : hdr->llc.ni.type"));
        assert!(source.contains("if (info->bitmask & EBT_802_3_SAP)"));
        assert!(source.contains("info->sap != hdr->llc.ui.ssap"));
        assert!(source.contains("info->sap != hdr->llc.ui.dsap"));
        assert!(source.contains("if (info->bitmask & EBT_802_3_TYPE)"));
        assert!(source.contains("hdr->llc.ui.dsap == CHECK_TYPE"));
        assert!(source.contains("info->type != type"));
        assert!(source.contains("info->bitmask & ~EBT_802_3_MASK"));
        assert!(source.contains("info->invflags & ~EBT_802_3_MASK"));
        assert!(source.contains(".name\t\t= \"802_3\""));
        assert!(source.contains(".family\t\t= NFPROTO_BRIDGE"));
        assert!(
            source.contains(
                "MODULE_DESCRIPTION(\"Ebtables: DSAP/SSAP field and SNAP type matching\")"
            )
        );
    }

    #[test]
    fn ebt_802_3_match_checks_sap_type_and_inversion() {
        let snap = Ebt8023Header {
            dsap: CHECK_TYPE,
            ssap: CHECK_TYPE,
            ctrl: IS_UI,
            ui_type: 0x0800,
            ni_type: 0x86dd,
        };
        let info = Ebt8023Info {
            sap: CHECK_TYPE,
            type_: 0x0800,
            bitmask: EBT_802_3_SAP | EBT_802_3_TYPE,
            invflags: 0,
        };
        assert_eq!(ebt_802_3_mt_check(info), Ok(()));
        assert!(ebt_802_3_mt(snap, info));
        assert!(!ebt_802_3_mt(
            snap,
            Ebt8023Info {
                type_: 0x86dd,
                ..info
            }
        ));
        assert!(ebt_802_3_mt(
            snap,
            Ebt8023Info {
                type_: 0x86dd,
                invflags: EBT_802_3_TYPE,
                ..info
            }
        ));
        assert_eq!(
            ebt_802_3_mt_check(Ebt8023Info {
                bitmask: 0x80,
                ..info
            }),
            Err(-EINVAL)
        );
        assert_eq!(ebt_802_3_init(), &EBT_802_3_MT_REG);
    }
}
