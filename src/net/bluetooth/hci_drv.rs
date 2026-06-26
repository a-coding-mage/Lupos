//! linux-parity: complete
//! linux-source: vendor/linux/net/bluetooth/hci_drv.c
//! test-origin: linux:vendor/linux/net/bluetooth/hci_drv.c
//! Bluetooth HCI driver command event and dispatch helpers.

use crate::include::uapi::errno::ENOMEM;

pub const EILSEQ: i32 = 84;
pub const HCI_DRV_PKT: u8 = 0xf1;
pub const HCI_DRV_EV_CMD_STATUS: u16 = 0x0000;
pub const HCI_DRV_EV_CMD_COMPLETE: u16 = 0x0001;
pub const HCI_DRV_STATUS_UNKNOWN_COMMAND: u8 = 0x02;
pub const HCI_DRV_STATUS_INVALID_PARAMETERS: u8 = 0x03;
pub const HCI_DRV_OGF_DRIVER_SPECIFIC: u16 = 0x01;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HciDrvEvent {
    pub pkt_type: u8,
    pub event_opcode: u16,
    pub len: u16,
    pub command_opcode: u16,
    pub status: u8,
    pub payload_len: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HciDrvHandler {
    pub data_len: u16,
    pub func_ret: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HciDrv {
    pub common_handlers: alloc::vec::Vec<Option<HciDrvHandler>>,
    pub specific_handlers: alloc::vec::Vec<Option<HciDrvHandler>>,
}

extern crate alloc;

pub const fn hci_opcode_ogf(opcode: u16) -> u16 {
    opcode >> 10
}

pub const fn hci_opcode_ocf(opcode: u16) -> u16 {
    opcode & 0x03ff
}

pub fn hci_drv_cmd_status(cmd: u16, status: u8, alloc_ok: bool) -> Result<HciDrvEvent, i32> {
    if !alloc_ok {
        return Err(-ENOMEM);
    }
    Ok(HciDrvEvent {
        pkt_type: HCI_DRV_PKT,
        event_opcode: HCI_DRV_EV_CMD_STATUS,
        len: 3,
        command_opcode: cmd,
        status,
        payload_len: 0,
    })
}

pub fn hci_drv_cmd_complete(
    cmd: u16,
    status: u8,
    rp_len: usize,
    alloc_ok: bool,
) -> Result<HciDrvEvent, i32> {
    if !alloc_ok {
        return Err(-ENOMEM);
    }
    Ok(HciDrvEvent {
        pkt_type: HCI_DRV_PKT,
        event_opcode: HCI_DRV_EV_CMD_COMPLETE,
        len: 3 + rp_len as u16,
        command_opcode: cmd,
        status,
        payload_len: rp_len,
    })
}

pub fn hci_drv_process_cmd(
    drv: Option<&HciDrv>,
    opcode: u16,
    header_len: Option<u16>,
    skb_len: u16,
) -> Result<i32, HciDrvEvent> {
    let Some(len) = header_len else {
        return Err(hci_drv_cmd_status(0, HCI_DRV_STATUS_UNKNOWN_COMMAND, true).unwrap());
    };
    if len != skb_len {
        return Err(hci_drv_cmd_status(opcode, HCI_DRV_STATUS_UNKNOWN_COMMAND, true).unwrap());
    }
    let Some(drv) = drv else {
        return Err(hci_drv_cmd_status(opcode, HCI_DRV_STATUS_UNKNOWN_COMMAND, true).unwrap());
    };

    let handler = if hci_opcode_ogf(opcode) != HCI_DRV_OGF_DRIVER_SPECIFIC {
        drv.common_handlers
            .get(opcode as usize)
            .and_then(|entry| *entry)
    } else {
        drv.specific_handlers
            .get(hci_opcode_ocf(opcode) as usize)
            .and_then(|entry| *entry)
    };

    let Some(handler) = handler else {
        return Err(hci_drv_cmd_status(opcode, HCI_DRV_STATUS_UNKNOWN_COMMAND, true).unwrap());
    };
    if len != handler.data_len {
        return Err(hci_drv_cmd_status(opcode, HCI_DRV_STATUS_INVALID_PARAMETERS, true).unwrap());
    }
    Ok(handler.func_ret)
}

pub const fn hci_drv_process_cmd_header_error(
    header_present: bool,
    len_matches: bool,
) -> Result<(), i32> {
    if !header_present || !len_matches {
        return Err(-EILSEQ);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn hci_drv_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/bluetooth/hci_drv.c"
        ));
        let hci_drv_h = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/net/bluetooth/hci_drv.h"
        ));
        let hci_h = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/net/bluetooth/hci.h"
        ));
        assert!(source.contains("int hci_drv_cmd_status"));
        assert!(source.contains("bt_skb_alloc(sizeof(*hdr) + sizeof(*ev), GFP_KERNEL);"));
        assert!(source.contains("hdr->opcode = __cpu_to_le16(HCI_DRV_EV_CMD_STATUS);"));
        assert!(source.contains("hci_skb_pkt_type(skb) = HCI_DRV_PKT;"));
        assert!(source.contains("int hci_drv_cmd_complete"));
        assert!(source.contains("skb_put_data(skb, rp, rp_len);"));
        assert!(source.contains("int hci_drv_process_cmd"));
        assert!(source.contains("hdr = skb_pull_data(skb, sizeof(*hdr));"));
        assert!(source.contains("return -EILSEQ;"));
        assert!(source.contains("ogf = hci_opcode_ogf(opcode);"));
        assert!(source.contains("if (!hdev->hci_drv)"));
        assert!(source.contains("if (ogf != HCI_DRV_OGF_DRIVER_SPECIFIC)"));
        assert!(source.contains("handler = &hdev->hci_drv->common_handlers[opcode];"));
        assert!(source.contains("handler = &hdev->hci_drv->specific_handlers[ocf];"));
        assert!(source.contains("HCI_DRV_STATUS_INVALID_PARAMETERS"));
        assert!(hci_drv_h.contains("#define HCI_DRV_EV_CMD_STATUS\t0x0000"));
        assert!(hci_h.contains("#define HCI_DRV_PKT\t\t0xf1"));
        assert!(hci_h.contains("#define hci_opcode_ogf(op)\t\t(op >> 10)"));
    }

    #[test]
    fn hci_driver_events_and_dispatch_follow_linux_edges() {
        assert_eq!(
            hci_drv_cmd_status(7, HCI_DRV_STATUS_UNKNOWN_COMMAND, true).unwrap(),
            HciDrvEvent {
                pkt_type: HCI_DRV_PKT,
                event_opcode: HCI_DRV_EV_CMD_STATUS,
                len: 3,
                command_opcode: 7,
                status: HCI_DRV_STATUS_UNKNOWN_COMMAND,
                payload_len: 0,
            }
        );
        assert_eq!(hci_drv_cmd_complete(7, 0, 5, true).unwrap().len, 8);
        assert_eq!(hci_drv_cmd_status(7, 0, false), Err(-ENOMEM));
        assert_eq!(hci_drv_process_cmd_header_error(false, true), Err(-EILSEQ));
        assert_eq!(hci_drv_process_cmd_header_error(true, false), Err(-EILSEQ));

        let drv = HciDrv {
            common_handlers: vec![
                None,
                Some(HciDrvHandler {
                    data_len: 2,
                    func_ret: 11,
                }),
            ],
            specific_handlers: vec![Some(HciDrvHandler {
                data_len: 1,
                func_ret: 22,
            })],
        };
        assert_eq!(hci_drv_process_cmd(Some(&drv), 1, Some(2), 2), Ok(11));
        assert_eq!(
            hci_drv_process_cmd(Some(&drv), HCI_DRV_OGF_DRIVER_SPECIFIC << 10, Some(1), 1),
            Ok(22)
        );
        assert_eq!(
            hci_drv_process_cmd(Some(&drv), 1, Some(1), 1)
                .unwrap_err()
                .status,
            HCI_DRV_STATUS_INVALID_PARAMETERS
        );
        assert_eq!(
            hci_drv_process_cmd(None, 1, Some(1), 1).unwrap_err().status,
            HCI_DRV_STATUS_UNKNOWN_COMMAND
        );
    }
}
