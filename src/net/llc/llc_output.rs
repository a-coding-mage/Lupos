//! linux-parity: complete
//! linux-source: vendor/linux/net/llc/llc_output.c
//! test-origin: linux:vendor/linux/net/llc/llc_output.c
//! LLC minimal output path.

use crate::include::uapi::errno::EINVAL;

pub const ARPHRD_ETHER: u16 = 1;
pub const ARPHRD_LOOPBACK: u16 = 772;
pub const ETH_P_802_2: u16 = 0x0004;
pub const LLC_PDU_TYPE_U: u8 = 3;
pub const LLC_PDU_CMD: u8 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NetDevice {
    pub dev_type: u16,
    pub dev_addr: [u8; 6],
    pub hard_header_rc: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MacHeader {
    pub proto: u16,
    pub da: [u8; 6],
    pub sa: [u8; 6],
    pub len: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LlcPduHeader {
    pub pdu_type: u8,
    pub ssap: u8,
    pub dsap: u8,
    pub command: u8,
    pub ui_command: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LlcSap {
    pub lsap: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LlcSkBuff {
    pub dev: NetDevice,
    pub len: usize,
    pub mac_header: Option<MacHeader>,
    pub pdu_header: Option<LlcPduHeader>,
    pub queued: bool,
    pub freed: bool,
}

pub fn llc_mac_hdr_init(skb: &mut LlcSkBuff, sa: [u8; 6], da: [u8; 6]) -> Result<(), i32> {
    match skb.dev.dev_type {
        ARPHRD_ETHER | ARPHRD_LOOPBACK => {
            if skb.dev.hard_header_rc < 0 {
                Err(skb.dev.hard_header_rc)
            } else {
                skb.mac_header = Some(MacHeader {
                    proto: ETH_P_802_2,
                    da,
                    sa,
                    len: skb.len,
                });
                Ok(())
            }
        }
        _ => Err(-EINVAL),
    }
}

pub fn llc_build_and_send_ui_pkt(
    sap: LlcSap,
    skb: &mut LlcSkBuff,
    dmac: [u8; 6],
    dsap: u8,
) -> Result<(), i32> {
    skb.pdu_header = Some(LlcPduHeader {
        pdu_type: LLC_PDU_TYPE_U,
        ssap: sap.lsap,
        dsap,
        command: LLC_PDU_CMD,
        ui_command: true,
    });
    match llc_mac_hdr_init(skb, skb.dev.dev_addr, dmac) {
        Ok(()) => {
            skb.queued = true;
            Ok(())
        }
        Err(err) => {
            skb.freed = true;
            Err(err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skb(dev_type: u16) -> LlcSkBuff {
        LlcSkBuff {
            dev: NetDevice {
                dev_type,
                dev_addr: [1, 2, 3, 4, 5, 6],
                hard_header_rc: 14,
            },
            len: 42,
            mac_header: None,
            pdu_header: None,
            queued: false,
            freed: false,
        }
    }

    #[test]
    fn llc_output_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/llc/llc_output.c"
        ));
        assert!(source.contains("int llc_mac_hdr_init(struct sk_buff *skb"));
        assert!(source.contains("case ARPHRD_ETHER:"));
        assert!(source.contains("case ARPHRD_LOOPBACK:"));
        assert!(source.contains("dev_hard_header(skb, skb->dev, ETH_P_802_2"));
        assert!(source.contains("if (rc > 0)"));
        assert!(source.contains("rc = 0;"));
        assert!(source.contains("int llc_build_and_send_ui_pkt"));
        assert!(source.contains("llc_pdu_header_init(skb, LLC_PDU_TYPE_U, sap->laddr.lsap"));
        assert!(source.contains("llc_pdu_init_as_ui_cmd(skb);"));
        assert!(source.contains("rc = dev_queue_xmit(skb);"));
        assert!(source.contains("kfree_skb(skb);"));
        assert!(source.contains("EXPORT_SYMBOL(llc_mac_hdr_init);"));
        assert!(source.contains("EXPORT_SYMBOL(llc_build_and_send_ui_pkt);"));
    }

    #[test]
    fn llc_ui_packet_sets_pdu_header_and_queues_or_frees() {
        let mut ok = skb(ARPHRD_ETHER);
        assert_eq!(
            llc_build_and_send_ui_pkt(LlcSap { lsap: 0x42 }, &mut ok, [6, 5, 4, 3, 2, 1], 0xaa),
            Ok(())
        );
        assert!(ok.queued);
        assert_eq!(
            ok.pdu_header,
            Some(LlcPduHeader {
                pdu_type: LLC_PDU_TYPE_U,
                ssap: 0x42,
                dsap: 0xaa,
                command: LLC_PDU_CMD,
                ui_command: true,
            })
        );
        assert_eq!(ok.mac_header.unwrap().proto, ETH_P_802_2);

        let mut unsupported = skb(99);
        assert_eq!(
            llc_build_and_send_ui_pkt(LlcSap { lsap: 1 }, &mut unsupported, [0; 6], 2),
            Err(-EINVAL)
        );
        assert!(unsupported.freed);

        let mut hard_header_failed = skb(ARPHRD_ETHER);
        hard_header_failed.dev.hard_header_rc = -7;
        assert_eq!(
            llc_build_and_send_ui_pkt(LlcSap { lsap: 1 }, &mut hard_header_failed, [0; 6], 2),
            Err(-7)
        );
        assert_eq!(hard_header_failed.mac_header, None);
        assert!(hard_header_failed.freed);
    }
}
