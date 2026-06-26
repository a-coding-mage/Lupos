//! linux-parity: complete
//! linux-source: vendor/linux/net/nfc/nci/lib.c
//! test-origin: linux:vendor/linux/net/nfc/nci/lib.c
//! NFC Controller Interface status to errno mapping.

use crate::include::uapi::errno::{EBADMSG, EBUSY, ECONNREFUSED, ENOSYS, EPROTO, ETIMEDOUT};

pub const ECOMM: i32 = 70;
pub const EBADRQC: i32 = 56;
pub const EMSGSIZE: i32 = 90;
pub const EHOSTDOWN: i32 = 112;
pub const EALREADY: i32 = 114;

pub const NCI_STATUS_OK: u8 = 0x00;
pub const NCI_STATUS_REJECTED: u8 = 0x01;
pub const NCI_STATUS_RF_FRAME_CORRUPTED: u8 = 0x02;
pub const NCI_STATUS_FAILED: u8 = 0x03;
pub const NCI_STATUS_NOT_INITIALIZED: u8 = 0x04;
pub const NCI_STATUS_SYNTAX_ERROR: u8 = 0x05;
pub const NCI_STATUS_SEMANTIC_ERROR: u8 = 0x06;
pub const NCI_STATUS_UNKNOWN_GID: u8 = 0x07;
pub const NCI_STATUS_UNKNOWN_OID: u8 = 0x08;
pub const NCI_STATUS_INVALID_PARAM: u8 = 0x09;
pub const NCI_STATUS_MESSAGE_SIZE_EXCEEDED: u8 = 0x0a;
pub const NCI_STATUS_DISCOVERY_ALREADY_STARTED: u8 = 0xa0;
pub const NCI_STATUS_DISCOVERY_TARGET_ACTIVATION_FAILED: u8 = 0xa1;
pub const NCI_STATUS_RF_TRANSMISSION_ERROR: u8 = 0xb0;
pub const NCI_STATUS_RF_PROTOCOL_ERROR: u8 = 0xb1;
pub const NCI_STATUS_RF_TIMEOUT_ERROR: u8 = 0xb2;
pub const NCI_STATUS_NFCEE_INTERFACE_ACTIVATION_FAILED: u8 = 0xc0;
pub const NCI_STATUS_NFCEE_TRANSMISSION_ERROR: u8 = 0xc1;
pub const NCI_STATUS_NFCEE_PROTOCOL_ERROR: u8 = 0xc2;
pub const NCI_STATUS_NFCEE_TIMEOUT_ERROR: u8 = 0xc3;

pub const fn nci_to_errno(code: u8) -> i32 {
    match code {
        NCI_STATUS_OK => 0,
        NCI_STATUS_REJECTED => -EBUSY,
        NCI_STATUS_RF_FRAME_CORRUPTED => -EBADMSG,
        NCI_STATUS_NOT_INITIALIZED => -EHOSTDOWN,
        NCI_STATUS_SYNTAX_ERROR
        | NCI_STATUS_SEMANTIC_ERROR
        | NCI_STATUS_INVALID_PARAM
        | NCI_STATUS_RF_PROTOCOL_ERROR
        | NCI_STATUS_NFCEE_PROTOCOL_ERROR => -EPROTO,
        NCI_STATUS_UNKNOWN_GID | NCI_STATUS_UNKNOWN_OID => -EBADRQC,
        NCI_STATUS_MESSAGE_SIZE_EXCEEDED => -EMSGSIZE,
        NCI_STATUS_DISCOVERY_ALREADY_STARTED => -EALREADY,
        NCI_STATUS_DISCOVERY_TARGET_ACTIVATION_FAILED
        | NCI_STATUS_NFCEE_INTERFACE_ACTIVATION_FAILED => -ECONNREFUSED,
        NCI_STATUS_RF_TRANSMISSION_ERROR | NCI_STATUS_NFCEE_TRANSMISSION_ERROR => -ECOMM,
        NCI_STATUS_RF_TIMEOUT_ERROR | NCI_STATUS_NFCEE_TIMEOUT_ERROR => -ETIMEDOUT,
        NCI_STATUS_FAILED => -ENOSYS,
        _ => -ENOSYS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nci_lib_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/nfc/nci/lib.c"
        ));
        assert!(source.contains("int nci_to_errno(__u8 code)"));
        assert!(source.contains("case NCI_STATUS_OK:"));
        assert!(source.contains("return -EBUSY;"));
        assert!(source.contains("return -EBADMSG;"));
        assert!(source.contains("return -EHOSTDOWN;"));
        assert!(source.contains("return -EPROTO;"));
        assert!(source.contains("return -EBADRQC;"));
        assert!(source.contains("return -EMSGSIZE;"));
        assert!(source.contains("return -EALREADY;"));
        assert!(source.contains("return -ECONNREFUSED;"));
        assert!(source.contains("return -ECOMM;"));
        assert!(source.contains("return -ETIMEDOUT;"));
        assert!(source.contains("case NCI_STATUS_FAILED:"));
        assert!(source.contains("return -ENOSYS;"));
        assert!(source.contains("EXPORT_SYMBOL(nci_to_errno);"));
    }

    #[test]
    fn nci_status_codes_map_to_linux_errno_values() {
        assert_eq!(nci_to_errno(NCI_STATUS_OK), 0);
        assert_eq!(nci_to_errno(NCI_STATUS_REJECTED), -EBUSY);
        assert_eq!(nci_to_errno(NCI_STATUS_RF_FRAME_CORRUPTED), -EBADMSG);
        assert_eq!(nci_to_errno(NCI_STATUS_NOT_INITIALIZED), -EHOSTDOWN);
        assert_eq!(nci_to_errno(NCI_STATUS_SYNTAX_ERROR), -EPROTO);
        assert_eq!(nci_to_errno(NCI_STATUS_NFCEE_PROTOCOL_ERROR), -EPROTO);
        assert_eq!(nci_to_errno(NCI_STATUS_UNKNOWN_GID), -EBADRQC);
        assert_eq!(nci_to_errno(NCI_STATUS_MESSAGE_SIZE_EXCEEDED), -EMSGSIZE);
        assert_eq!(
            nci_to_errno(NCI_STATUS_DISCOVERY_ALREADY_STARTED),
            -EALREADY
        );
        assert_eq!(
            nci_to_errno(NCI_STATUS_DISCOVERY_TARGET_ACTIVATION_FAILED),
            -ECONNREFUSED
        );
        assert_eq!(nci_to_errno(NCI_STATUS_RF_TRANSMISSION_ERROR), -ECOMM);
        assert_eq!(nci_to_errno(NCI_STATUS_RF_TIMEOUT_ERROR), -ETIMEDOUT);
        assert_eq!(nci_to_errno(NCI_STATUS_FAILED), -ENOSYS);
        assert_eq!(nci_to_errno(0xff), -ENOSYS);
    }
}
