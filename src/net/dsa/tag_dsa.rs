//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/tag_dsa.c
//! test-origin: linux:vendor/linux/net/dsa/tag_dsa.c
//! Regular and Ethertype DSA tag formats.

pub const DSA_NAME: &str = "dsa";
pub const EDSA_NAME: &str = "edsa";
pub const DSA_HLEN: usize = 4;
pub const EDSA_HLEN: usize = 8;
pub const ETH_P_8021Q: u16 = 0x8100;
pub const ETH_P_EDSA: u16 = 0xdada;
pub const VLAN_N_VID: u16 = 4096;
pub const MV88E6XXX_VID_STANDALONE: u16 = 0;
pub const MV88E6XXX_VID_BRIDGED: u16 = VLAN_N_VID - 1;
pub const DSA_TAG_PROTO_DSA_VALUE: u8 = 3;
pub const DSA_TAG_PROTO_EDSA_VALUE: u8 = 4;
pub const MODULE_DESCRIPTION: &str = "DSA tag driver for Marvell switches using DSA headers";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DsaCmd {
    ToCpu = 0,
    FromCpu = 1,
    ToSniffer = 2,
    Forward = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DsaCode {
    MgmtTrap = 0,
    Frame2Reg = 1,
    IgmpMldTrap = 2,
    PolicyTrap = 3,
    ArpMirror = 4,
    PolicyMirror = 5,
    Reserved6 = 6,
    Reserved7 = 7,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaDeviceOps {
    pub name: &'static str,
    pub proto: u8,
    pub needed_headroom: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaXmitContext {
    pub offload_fwd_mark: bool,
    pub bridge_num: u8,
    pub last_switch: u8,
    pub switch_index: u8,
    pub port_index: u8,
    pub vlan_tagged: bool,
    pub vlan_tci: u16,
    pub has_bridge: bool,
    pub bridge_vlan_enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaXmitFrame {
    pub header: [u8; DSA_HLEN],
    pub extra: usize,
    pub cmd: DsaCmd,
    pub tag_dev: u8,
    pub tag_port: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaRcvFrame {
    pub source_device: u8,
    pub source_port: u8,
    pub trunk: bool,
    pub offload_fwd_mark: bool,
    pub restored_vlan: Option<[u8; DSA_HLEN]>,
    pub strip_len: usize,
}

pub const DSA_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: DSA_NAME,
    proto: DSA_TAG_PROTO_DSA_VALUE,
    needed_headroom: DSA_HLEN,
};
pub const EDSA_NETDEV_OPS: DsaDeviceOps = DsaDeviceOps {
    name: EDSA_NAME,
    proto: DSA_TAG_PROTO_EDSA_VALUE,
    needed_headroom: EDSA_HLEN,
};

const fn cmd_from_u8(cmd: u8) -> Option<DsaCmd> {
    match cmd {
        0 => Some(DsaCmd::ToCpu),
        1 => Some(DsaCmd::FromCpu),
        2 => Some(DsaCmd::ToSniffer),
        3 => Some(DsaCmd::Forward),
        _ => None,
    }
}

const fn code_from_u8(code: u8) -> Option<DsaCode> {
    match code {
        0 => Some(DsaCode::MgmtTrap),
        1 => Some(DsaCode::Frame2Reg),
        2 => Some(DsaCode::IgmpMldTrap),
        3 => Some(DsaCode::PolicyTrap),
        4 => Some(DsaCode::ArpMirror),
        5 => Some(DsaCode::PolicyMirror),
        6 => Some(DsaCode::Reserved6),
        7 => Some(DsaCode::Reserved7),
        _ => None,
    }
}

pub const fn dsa_xmit_ll(ctx: DsaXmitContext, extra: usize) -> DsaXmitFrame {
    let (cmd, tag_dev, tag_port) = if ctx.offload_fwd_mark {
        (
            DsaCmd::Forward,
            ctx.last_switch.wrapping_add(ctx.bridge_num),
            0,
        )
    } else {
        (DsaCmd::FromCpu, ctx.switch_index, ctx.port_index)
    };

    if ctx.vlan_tagged && (!ctx.has_bridge || ctx.bridge_vlan_enabled) {
        let mut h1 = tag_port << 3;
        let mut h2 = (ctx.vlan_tci >> 8) as u8;
        if (h2 & 0x10) != 0 {
            h1 |= 0x01;
            h2 &= !0x10;
        }
        DsaXmitFrame {
            header: [
                ((cmd as u8) << 6) | 0x20 | tag_dev,
                h1,
                h2,
                ctx.vlan_tci as u8,
            ],
            extra,
            cmd,
            tag_dev,
            tag_port,
        }
    } else {
        let vid = if ctx.has_bridge {
            MV88E6XXX_VID_BRIDGED
        } else {
            MV88E6XXX_VID_STANDALONE
        };
        DsaXmitFrame {
            header: [
                ((cmd as u8) << 6) | tag_dev,
                tag_port << 3,
                (vid >> 8) as u8,
                vid as u8,
            ],
            extra,
            cmd,
            tag_dev,
            tag_port,
        }
    }
}

pub const fn dsa_xmit(ctx: DsaXmitContext) -> DsaXmitFrame {
    dsa_xmit_ll(ctx, 0)
}

pub const fn edsa_xmit(ctx: DsaXmitContext) -> ([u8; EDSA_HLEN], DsaXmitFrame) {
    let frame = dsa_xmit_ll(ctx, EDSA_HLEN - DSA_HLEN);
    (
        [
            (ETH_P_EDSA >> 8) as u8,
            ETH_P_EDSA as u8,
            0,
            0,
            frame.header[0],
            frame.header[1],
            frame.header[2],
            frame.header[3],
        ],
        frame,
    )
}

pub const fn dsa_rcv_ll(
    header: [u8; DSA_HLEN],
    extra: usize,
    user_found: bool,
) -> Option<DsaRcvFrame> {
    let cmd = match cmd_from_u8(header[0] >> 6) {
        Some(cmd) => cmd,
        None => return None,
    };
    let mut trap = false;
    let mut trunk = false;
    match cmd {
        DsaCmd::Forward => {
            trunk = (header[1] & 4) != 0;
        }
        DsaCmd::ToCpu => {
            let code_value = (header[1] & 0x6) | ((header[2] >> 4) & 1);
            match code_from_u8(code_value) {
                Some(DsaCode::Frame2Reg | DsaCode::Reserved6 | DsaCode::Reserved7) => return None,
                Some(DsaCode::MgmtTrap | DsaCode::IgmpMldTrap | DsaCode::PolicyTrap) => {
                    trap = true;
                }
                Some(DsaCode::ArpMirror | DsaCode::PolicyMirror) => {}
                None => return None,
            }
        }
        DsaCmd::FromCpu | DsaCmd::ToSniffer => return None,
    }

    if !user_found {
        return None;
    }

    let source_device = header[0] & 0x1f;
    let source_port = (header[1] >> 3) & 0x1f;
    if (header[0] & 0x20) != 0 {
        let mut new_header = [
            (ETH_P_8021Q >> 8) as u8,
            ETH_P_8021Q as u8,
            header[2] & !0x10,
            header[3],
        ];
        if (header[1] & 0x01) != 0 {
            new_header[2] |= 0x10;
        }
        Some(DsaRcvFrame {
            source_device,
            source_port,
            trunk,
            offload_fwd_mark: trunk || !trap,
            restored_vlan: Some(new_header),
            strip_len: extra,
        })
    } else {
        Some(DsaRcvFrame {
            source_device,
            source_port,
            trunk,
            offload_fwd_mark: trunk || !trap,
            restored_vlan: None,
            strip_len: DSA_HLEN + extra,
        })
    }
}

pub const fn dsa_rcv(header: [u8; DSA_HLEN], user_found: bool) -> Option<DsaRcvFrame> {
    dsa_rcv_ll(header, 0, user_found)
}

pub const fn edsa_rcv(edsa_header: [u8; EDSA_HLEN], user_found: bool) -> Option<DsaRcvFrame> {
    if edsa_header[0] != (ETH_P_EDSA >> 8) as u8
        || edsa_header[1] != ETH_P_EDSA as u8
        || edsa_header[2] != 0
        || edsa_header[3] != 0
    {
        return None;
    }
    dsa_rcv_ll(
        [
            edsa_header[4],
            edsa_header[5],
            edsa_header[6],
            edsa_header[7],
        ],
        EDSA_HLEN - DSA_HLEN,
        user_found,
    )
}

pub fn module_aliases() -> [&'static str; 4] {
    [
        "dsa_tag:dsa",
        "dsa_tag:id-3",
        "dsa_tag:edsa",
        "dsa_tag:id-4",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_ctx() -> DsaXmitContext {
        DsaXmitContext {
            offload_fwd_mark: false,
            bridge_num: 0,
            last_switch: 2,
            switch_index: 1,
            port_index: 3,
            vlan_tagged: false,
            vlan_tci: 0,
            has_bridge: false,
            bridge_vlan_enabled: false,
        }
    }

    #[test]
    fn tag_dsa_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/tag_dsa.c"
        ));
        let mv88e6xxx = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/dsa/mv88e6xxx.h"
        ));
        assert!(source.contains("#define DSA_NAME\t\"dsa\""));
        assert!(source.contains("#define EDSA_NAME\t\"edsa\""));
        assert!(source.contains("#define DSA_HLEN\t4"));
        assert!(source.contains("cmd = DSA_CMD_FORWARD;"));
        assert!(source.contains("cmd = DSA_CMD_FROM_CPU;"));
        assert!(source.contains("dsa_header[0] = (cmd << 6) | 0x20 | tag_dev;"));
        assert!(source.contains("if (dsa_header[2] & 0x10)"));
        assert!(
            source.contains("vid = br_dev ? MV88E6XXX_VID_BRIDGED : MV88E6XXX_VID_STANDALONE;")
        );
        assert!(source.contains("cmd = dsa_header[0] >> 6;"));
        assert!(source.contains("trunk = !!(dsa_header[1] & 4);"));
        assert!(source.contains("code = (dsa_header[1] & 0x6) | ((dsa_header[2] >> 4) & 1);"));
        assert!(source.contains("source_device = dsa_header[0] & 0x1f;"));
        assert!(source.contains("source_port = (dsa_header[1] >> 3) & 0x1f;"));
        assert!(source.contains("new_header[0] = (ETH_P_8021Q >> 8) & 0xff;"));
        assert!(source.contains("if (dsa_header[1] & 0x01)"));
        assert!(source.contains("#define EDSA_HLEN 8"));
        assert!(source.contains("edsa_header[0] = (ETH_P_EDSA >> 8) & 0xff;"));
        assert!(mv88e6xxx.contains("#define MV88E6XXX_VID_STANDALONE\t0"));
        assert!(mv88e6xxx.contains("#define MV88E6XXX_VID_BRIDGED\t\t(VLAN_N_VID - 1)"));
    }

    #[test]
    fn xmit_builds_from_cpu_forward_and_vlan_headers() {
        let tx = dsa_xmit(base_ctx());
        assert_eq!(tx.header, [0x40 | 1, 3 << 3, 0, 0]);
        assert_eq!(tx.cmd, DsaCmd::FromCpu);

        let bridged = dsa_xmit(DsaXmitContext {
            has_bridge: true,
            ..base_ctx()
        });
        assert_eq!(bridged.header[2], 0x0f);
        assert_eq!(bridged.header[3], 0xff);

        let tagged = dsa_xmit(DsaXmitContext {
            vlan_tagged: true,
            vlan_tci: 0x1234,
            ..base_ctx()
        });
        assert_eq!(tagged.header, [0x40 | 0x20 | 1, (3 << 3) | 1, 0x02, 0x34]);

        let fwd = dsa_xmit(DsaXmitContext {
            offload_fwd_mark: true,
            bridge_num: 4,
            ..base_ctx()
        });
        assert_eq!(fwd.header[0], 0xc0 | 6);
        assert_eq!(fwd.header[1], 0);
    }

    #[test]
    fn receive_accepts_only_linux_commands_and_restores_vlan() {
        let rx = dsa_rcv([0x20 | 2, (5 << 3) | 4, 0x02, 0x64], true).unwrap();
        assert_eq!(rx.source_device, 2);
        assert_eq!(rx.source_port, 5);
        assert!(rx.offload_fwd_mark);
        assert_eq!(rx.restored_vlan, Some([0x81, 0x00, 0x02, 0x64]));

        let trap = dsa_rcv([0, 5 << 3, 0, 0], true).unwrap();
        assert!(!trap.offload_fwd_mark);
        assert_eq!(trap.strip_len, DSA_HLEN);

        assert!(dsa_rcv([0, 0, 0x10, 0], true).is_none());
        assert!(dsa_rcv([0x40, 0, 0, 0], true).is_none());
        assert!(dsa_rcv([0xc0, 0x04, 0, 0], true).unwrap().trunk);
    }

    #[test]
    fn edsa_adds_ethertype_prefix_and_uses_extra_strip() {
        let (wire, frame) = edsa_xmit(base_ctx());
        assert_eq!(&wire[..4], &[0xda, 0xda, 0, 0]);
        assert_eq!(&wire[4..], &frame.header);
        let rx = edsa_rcv([0xda, 0xda, 0, 0, 0xc1, 3 << 3, 0, 0], true).unwrap();
        assert_eq!(rx.strip_len, EDSA_HLEN);
        assert_eq!(
            module_aliases(),
            [
                "dsa_tag:dsa",
                "dsa_tag:id-3",
                "dsa_tag:edsa",
                "dsa_tag:id-4"
            ]
        );
    }
}
