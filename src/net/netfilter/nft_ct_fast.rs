//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nft_ct_fast.c
//! test-origin: linux:vendor/linux/net/netfilter/nft_ct_fast.c
//! Fast conntrack nftables expression evaluation.

pub const NFT_BREAK: i32 = -1;
pub const NF_CT_STATE_INVALID_BIT: u32 = 1 << 0;
pub const NF_CT_STATE_ESTABLISHED_BIT: u32 = 1 << 1;
pub const NF_CT_STATE_RELATED_BIT: u32 = 1 << 2;
pub const NF_CT_STATE_NEW_BIT: u32 = 1 << 3;
pub const NF_CT_STATE_UNTRACKED_BIT: u32 = 1 << 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NftCtKey {
    State,
    Direction,
    Status,
    Mark,
    Secmark,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IpConntrackInfo {
    New,
    Established,
    Related,
    NewReply,
    EstablishedReply,
    RelatedReply,
    Untracked,
    Invalid,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NfConn {
    pub status: u32,
    pub mark: u32,
    pub secmark: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftCt {
    pub key: NftCtKey,
    pub dreg: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NftPktInfo {
    pub conn: Option<NfConn>,
    pub ctinfo: IpConntrackInfo,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NftRegs {
    pub data: [u32; 8],
    pub verdict_code: i32,
}

impl Default for NftRegs {
    fn default() -> Self {
        Self {
            data: [0; 8],
            verdict_code: 0,
        }
    }
}

pub fn nft_ct_get_fast_eval(expr: NftCt, regs: &mut NftRegs, pkt: NftPktInfo) {
    if expr.dreg >= regs.data.len() {
        regs.verdict_code = NFT_BREAK;
        return;
    }

    if expr.key == NftCtKey::State {
        regs.data[expr.dreg] = match pkt.conn {
            Some(_) => nf_ct_state_bit(pkt.ctinfo),
            None if pkt.ctinfo == IpConntrackInfo::Untracked => NF_CT_STATE_UNTRACKED_BIT,
            None => NF_CT_STATE_INVALID_BIT,
        };
        return;
    }

    let Some(conn) = pkt.conn else {
        regs.verdict_code = NFT_BREAK;
        return;
    };

    regs.data[expr.dreg] = match expr.key {
        NftCtKey::Direction => ctinfo_direction(pkt.ctinfo),
        NftCtKey::Status => conn.status,
        NftCtKey::Mark => conn.mark,
        NftCtKey::Secmark => conn.secmark,
        NftCtKey::State | NftCtKey::Unsupported => {
            regs.verdict_code = NFT_BREAK;
            return;
        }
    };
}

pub const fn ctinfo_direction(ctinfo: IpConntrackInfo) -> u32 {
    match ctinfo {
        IpConntrackInfo::NewReply
        | IpConntrackInfo::EstablishedReply
        | IpConntrackInfo::RelatedReply => 1,
        _ => 0,
    }
}

pub const fn nf_ct_state_bit(ctinfo: IpConntrackInfo) -> u32 {
    match ctinfo {
        IpConntrackInfo::New | IpConntrackInfo::NewReply => NF_CT_STATE_NEW_BIT,
        IpConntrackInfo::Established | IpConntrackInfo::EstablishedReply => {
            NF_CT_STATE_ESTABLISHED_BIT
        }
        IpConntrackInfo::Related | IpConntrackInfo::RelatedReply => NF_CT_STATE_RELATED_BIT,
        IpConntrackInfo::Untracked => NF_CT_STATE_UNTRACKED_BIT,
        IpConntrackInfo::Invalid => NF_CT_STATE_INVALID_BIT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nft_ct_fast_eval_matches_linux_source_shape() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nft_ct_fast.c"
        ));
        assert!(source.contains("#if IS_ENABLED(CONFIG_NFT_CT)"));
        assert!(source.contains("void nft_ct_get_fast_eval"));
        assert!(source.contains("ct = nf_ct_get(pkt->skb, &ctinfo);"));
        assert!(source.contains("case NFT_CT_STATE:"));
        assert!(source.contains("state = NF_CT_STATE_BIT(ctinfo);"));
        assert!(source.contains("NF_CT_STATE_UNTRACKED_BIT"));
        assert!(source.contains("regs->verdict.code = NFT_BREAK;"));
        assert!(source.contains("case NFT_CT_DIRECTION:"));
        assert!(source.contains("nft_reg_store8(dest, CTINFO2DIR(ctinfo));"));
        assert!(source.contains("case NFT_CT_STATUS:"));
        assert!(source.contains("case NFT_CT_MARK:"));
        assert!(source.contains("case NFT_CT_SECMARK:"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(nft_ct_get_fast_eval);"));
    }

    #[test]
    fn nft_ct_fast_eval_sets_state_or_breaks_like_linux() {
        let mut regs = NftRegs::default();
        nft_ct_get_fast_eval(
            NftCt {
                key: NftCtKey::State,
                dreg: 1,
            },
            &mut regs,
            NftPktInfo {
                conn: None,
                ctinfo: IpConntrackInfo::Untracked,
            },
        );
        assert_eq!(regs.data[1], NF_CT_STATE_UNTRACKED_BIT);
        assert_eq!(regs.verdict_code, 0);

        nft_ct_get_fast_eval(
            NftCt {
                key: NftCtKey::Status,
                dreg: 2,
            },
            &mut regs,
            NftPktInfo {
                conn: None,
                ctinfo: IpConntrackInfo::Invalid,
            },
        );
        assert_eq!(regs.verdict_code, NFT_BREAK);

        let mut regs = NftRegs::default();
        nft_ct_get_fast_eval(
            NftCt {
                key: NftCtKey::Direction,
                dreg: 0,
            },
            &mut regs,
            NftPktInfo {
                conn: Some(NfConn {
                    status: 0xaa,
                    mark: 0xbb,
                    secmark: 0xcc,
                }),
                ctinfo: IpConntrackInfo::EstablishedReply,
            },
        );
        assert_eq!(regs.data[0], 1);

        nft_ct_get_fast_eval(
            NftCt {
                key: NftCtKey::Mark,
                dreg: 3,
            },
            &mut regs,
            NftPktInfo {
                conn: Some(NfConn {
                    status: 0xaa,
                    mark: 0xbb,
                    secmark: 0xcc,
                }),
                ctinfo: IpConntrackInfo::Established,
            },
        );
        assert_eq!(regs.data[3], 0xbb);
    }
}
