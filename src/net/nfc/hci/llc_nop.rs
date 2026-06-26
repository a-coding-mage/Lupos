//! linux-parity: complete
//! linux-source: vendor/linux/net/nfc/hci/llc_nop.c
//! test-origin: linux:vendor/linux/net/nfc/hci/llc_nop.c
//! NFC HCI no-op link-layer control.

pub const LLC_NOP_NAME: &str = "nop";

pub type XmitToDrv = fn(u32, &[u8]) -> i32;
pub type RcvToHci = fn(u32, &[u8]) -> usize;
pub type LlcFailure = fn(u32);

#[derive(Clone, Copy)]
pub struct LlcNop {
    pub hdev: u32,
    pub xmit_to_drv: XmitToDrv,
    pub rcv_to_hci: RcvToHci,
    pub tx_headroom: i32,
    pub tx_tailroom: i32,
    pub llc_failure: LlcFailure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LlcRegistration {
    pub name: &'static str,
    pub has_init: bool,
    pub has_deinit: bool,
    pub has_start: bool,
    pub has_stop: bool,
    pub has_rcv_from_drv: bool,
    pub has_xmit_from_hci: bool,
}

pub const LLC_NOP_OPS: LlcRegistration = LlcRegistration {
    name: LLC_NOP_NAME,
    has_init: true,
    has_deinit: true,
    has_start: true,
    has_stop: true,
    has_rcv_from_drv: true,
    has_xmit_from_hci: true,
};

pub fn llc_nop_init(
    hdev: u32,
    xmit_to_drv: XmitToDrv,
    rcv_to_hci: RcvToHci,
    tx_headroom: i32,
    tx_tailroom: i32,
    rx_headroom: &mut i32,
    rx_tailroom: &mut i32,
    llc_failure: LlcFailure,
) -> LlcNop {
    *rx_headroom = 0;
    *rx_tailroom = 0;
    LlcNop {
        hdev,
        xmit_to_drv,
        rcv_to_hci,
        tx_headroom,
        tx_tailroom,
        llc_failure,
    }
}

pub const fn llc_nop_deinit(llc: LlcNop) -> u32 {
    llc.hdev
}

pub const fn llc_nop_start() -> i32 {
    0
}

pub const fn llc_nop_stop() -> i32 {
    0
}

pub fn llc_nop_rcv_from_drv(llc: &LlcNop, skb: &[u8]) -> usize {
    (llc.rcv_to_hci)(llc.hdev, skb)
}

pub fn llc_nop_xmit_from_hci(llc: &LlcNop, skb: &[u8]) -> i32 {
    (llc.xmit_to_drv)(llc.hdev, skb)
}

pub const fn nfc_llc_nop_register() -> &'static LlcRegistration {
    &LLC_NOP_OPS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xmit(hdev: u32, skb: &[u8]) -> i32 {
        hdev as i32 + skb.len() as i32
    }

    fn rcv(hdev: u32, skb: &[u8]) -> usize {
        hdev as usize + skb.len()
    }

    fn failure(_hdev: u32) {}

    #[test]
    fn llc_nop_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/nfc/hci/llc_nop.c"
        ));
        assert!(source.contains("struct llc_nop"));
        assert!(source.contains("static void *llc_nop_init"));
        assert!(source.contains("*rx_headroom = 0;"));
        assert!(source.contains("*rx_tailroom = 0;"));
        assert!(source.contains("llc_nop = kzalloc_obj(struct llc_nop);"));
        assert!(source.contains("llc_nop->hdev = hdev;"));
        assert!(source.contains("llc_nop->xmit_to_drv = xmit_to_drv;"));
        assert!(source.contains("llc_nop->rcv_to_hci = rcv_to_hci;"));
        assert!(source.contains("llc_nop->tx_headroom = tx_headroom;"));
        assert!(source.contains("llc_nop->tx_tailroom = tx_tailroom;"));
        assert!(source.contains("llc_nop->llc_failure = llc_failure;"));
        assert!(source.contains("static int llc_nop_start"));
        assert!(source.contains("return 0;"));
        assert!(source.contains("llc_nop->rcv_to_hci(llc_nop->hdev, skb);"));
        assert!(source.contains("return llc_nop->xmit_to_drv(llc_nop->hdev, skb);"));
        assert!(source.contains("return nfc_llc_register(LLC_NOP_NAME, &llc_nop_ops);"));

        let mut rx_headroom = 8;
        let mut rx_tailroom = 9;
        let llc = llc_nop_init(
            4,
            xmit,
            rcv,
            2,
            3,
            &mut rx_headroom,
            &mut rx_tailroom,
            failure,
        );
        assert_eq!(rx_headroom, 0);
        assert_eq!(rx_tailroom, 0);
        assert_eq!(llc.tx_headroom, 2);
        assert_eq!(llc.tx_tailroom, 3);
        assert_eq!(llc_nop_start(), 0);
        assert_eq!(llc_nop_stop(), 0);
        assert_eq!(llc_nop_rcv_from_drv(&llc, b"abc"), 7);
        assert_eq!(llc_nop_xmit_from_hci(&llc, b"abc"), 7);
        assert_eq!(llc_nop_deinit(llc), 4);
        assert_eq!(nfc_llc_nop_register(), &LLC_NOP_OPS);
    }
}
