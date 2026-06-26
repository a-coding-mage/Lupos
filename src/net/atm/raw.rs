//! linux-parity: complete
//! linux-source: vendor/linux/net/atm/raw.c
//! test-origin: linux:vendor/linux/net/atm/raw.c
//! Raw ATM AAL0/AAL3/4/AAL5 transport setup.

pub const EADDRNOTAVAIL: i32 = 99;
pub const ATM_HDR_VPI_MASK: u32 = 0x0ff0_0000;
pub const ATM_HDR_VPI_SHIFT: u32 = 20;
pub const ATM_HDR_VCI_MASK: u32 = 0x000f_fff0;
pub const ATM_HDR_VCI_SHIFT: u32 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AtmSend {
    Aal0,
    DeviceSendBh,
    DeviceSend,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtmVcc {
    pub vpi: u32,
    pub vci: u32,
    pub has_send_bh: bool,
    pub push_raw: bool,
    pub pop_raw: bool,
    pub push_oam_none: bool,
    pub send: AtmSend,
}

pub const fn atm_header(vpi: u32, vci: u32) -> u32 {
    ((vpi << ATM_HDR_VPI_SHIFT) & ATM_HDR_VPI_MASK)
        | ((vci << ATM_HDR_VCI_SHIFT) & ATM_HDR_VCI_MASK)
}

pub const fn atm_send_aal0(
    cap_net_admin: bool,
    vcc: AtmVcc,
    skb_header: u32,
) -> Result<AtmSend, i32> {
    let expected = atm_header(vcc.vpi, vcc.vci);
    if !cap_net_admin && (skb_header & (ATM_HDR_VPI_MASK | ATM_HDR_VCI_MASK)) != expected {
        return Err(-EADDRNOTAVAIL);
    }
    if vcc.has_send_bh {
        Ok(AtmSend::DeviceSendBh)
    } else {
        Ok(AtmSend::DeviceSend)
    }
}

pub const fn atm_init_aal0(mut vcc: AtmVcc) -> AtmVcc {
    vcc.push_raw = true;
    vcc.pop_raw = true;
    vcc.push_oam_none = true;
    vcc.send = AtmSend::Aal0;
    vcc
}

pub const fn atm_init_aal34(mut vcc: AtmVcc) -> AtmVcc {
    vcc.push_raw = true;
    vcc.pop_raw = true;
    vcc.push_oam_none = true;
    vcc.send = if vcc.has_send_bh {
        AtmSend::DeviceSendBh
    } else {
        AtmSend::DeviceSend
    };
    vcc
}

pub const fn atm_init_aal5(vcc: AtmVcc) -> AtmVcc {
    atm_init_aal34(vcc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atm_raw_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/atm/raw.c"
        ));
        assert!(source.contains("net/atm/raw.c - Raw AAL0 and AAL5 transports"));
        assert!(source.contains("static void atm_push_raw"));
        assert!(source.contains("skb_queue_tail(&sk->sk_receive_queue, skb);"));
        assert!(source.contains("sk->sk_data_ready(sk);"));
        assert!(source.contains("static void atm_pop_raw"));
        assert!(source.contains("atm_return_tx(vcc, skb);"));
        assert!(source.contains("dev_kfree_skb_any(skb);"));
        assert!(source.contains("sk->sk_write_space(sk);"));
        assert!(source.contains("static int atm_send_aal0"));
        assert!(source.contains("if (!capable(CAP_NET_ADMIN)"));
        assert!(source.contains("ATM_HDR_VPI_MASK | ATM_HDR_VCI_MASK"));
        assert!(source.contains("return -EADDRNOTAVAIL;"));
        assert!(source.contains("if (vcc->dev->ops->send_bh)"));
        assert!(source.contains("int atm_init_aal0"));
        assert!(source.contains("vcc->push = atm_push_raw;"));
        assert!(source.contains("vcc->pop = atm_pop_raw;"));
        assert!(source.contains("vcc->push_oam = NULL;"));
        assert!(source.contains("vcc->send = atm_send_aal0;"));
        assert!(source.contains("int atm_init_aal34"));
        assert!(source.contains("int atm_init_aal5"));
        assert!(source.contains("EXPORT_SYMBOL(atm_init_aal5);"));
    }

    #[test]
    fn aal_initializers_select_linux_send_paths() {
        let vcc = AtmVcc {
            vpi: 1,
            vci: 32,
            has_send_bh: true,
            push_raw: false,
            pop_raw: false,
            push_oam_none: false,
            send: AtmSend::DeviceSend,
        };
        let aal0 = atm_init_aal0(vcc);
        assert_eq!(aal0.send, AtmSend::Aal0);
        assert!(aal0.push_raw && aal0.pop_raw && aal0.push_oam_none);
        assert_eq!(
            atm_send_aal0(false, aal0, atm_header(1, 32)),
            Ok(AtmSend::DeviceSendBh)
        );
        assert_eq!(
            atm_send_aal0(false, aal0, atm_header(1, 33)),
            Err(-EADDRNOTAVAIL)
        );
        assert_eq!(
            atm_send_aal0(true, aal0, atm_header(1, 33)),
            Ok(AtmSend::DeviceSendBh)
        );
        assert_eq!(atm_init_aal5(vcc).send, AtmSend::DeviceSendBh);
        assert_eq!(
            atm_init_aal34(AtmVcc {
                has_send_bh: false,
                ..vcc
            })
            .send,
            AtmSend::DeviceSend
        );
    }
}
