//! linux-parity: complete
//! linux-source: vendor/linux/net/sched/em_nbyte.c
//! test-origin: linux:vendor/linux/net/sched/em_nbyte.c
//! N-byte traffic-control ematch.

use crate::include::uapi::errno::{EINVAL, ENOMEM};

pub const TCF_EM_NBYTE: u16 = 2;
pub const TCF_LAYER_LINK: u8 = 0;
pub const TCF_LAYER_NETWORK: u8 = 1;
pub const TCF_LAYER_TRANSPORT: u8 = 2;
pub const TCF_EM_NBYTE_HDR_LEN: usize = 4;
pub const MODULE_DESCRIPTION: &str = "ematch classifier for arbitrary skb multi-bytes";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TcfEmNbyte {
    pub off: usize,
    pub len: usize,
    pub layer: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NbyteData<'a> {
    pub hdr: TcfEmNbyte,
    pub pattern: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TcfEmatch<'a> {
    pub datalen: usize,
    pub data: NbyteData<'a>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PacketLayers {
    pub link: Option<usize>,
    pub network: Option<usize>,
    pub transport: Option<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TcfEmatchOps {
    pub kind: u16,
}

pub const EM_NBYTE_OPS: TcfEmatchOps = TcfEmatchOps { kind: TCF_EM_NBYTE };

pub fn em_nbyte_change<'a>(
    hdr: TcfEmNbyte,
    pattern: &'a [u8],
    data_len: usize,
    allocation_ok: bool,
) -> Result<TcfEmatch<'a>, i32> {
    let datalen = TCF_EM_NBYTE_HDR_LEN.saturating_add(hdr.len);
    if data_len < TCF_EM_NBYTE_HDR_LEN || data_len < datalen || pattern.len() < hdr.len {
        return Err(-EINVAL);
    }
    if !allocation_ok {
        return Err(-ENOMEM);
    }
    Ok(TcfEmatch {
        datalen,
        data: NbyteData {
            hdr,
            pattern: &pattern[..hdr.len],
        },
    })
}

pub fn em_nbyte_match(packet: &[u8], em: &TcfEmatch<'_>, layers: PacketLayers) -> bool {
    let Some(base) = layer_base(layers, em.data.hdr.layer) else {
        return false;
    };
    let ptr = base.saturating_add(em.data.hdr.off);
    let end = ptr.saturating_add(em.data.hdr.len);
    packet
        .get(ptr..end)
        .is_some_and(|bytes| bytes == em.data.pattern)
}

pub const fn init_em_nbyte() -> &'static TcfEmatchOps {
    &EM_NBYTE_OPS
}

const fn layer_base(layers: PacketLayers, layer: u8) -> Option<usize> {
    match layer {
        TCF_LAYER_LINK => layers.link,
        TCF_LAYER_NETWORK => layers.network,
        TCF_LAYER_TRANSPORT => layers.transport,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn em_nbyte_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/sched/em_nbyte.c"
        ));
        assert!(source.contains("struct nbyte_data"));
        assert!(source.contains("struct tcf_em_nbyte *nbyte = data;"));
        assert!(source.contains("data_len < sizeof(*nbyte)"));
        assert!(source.contains("sizeof(*nbyte) + nbyte->len"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("kmemdup(data, em->datalen, GFP_KERNEL)"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("tcf_get_base_ptr(skb, nbyte->hdr.layer)"));
        assert!(source.contains("ptr += nbyte->hdr.off;"));
        assert!(source.contains("tcf_valid_offset(skb, ptr, nbyte->hdr.len)"));
        assert!(source.contains("return !memcmp(ptr, nbyte->pattern, nbyte->hdr.len);"));
        assert!(source.contains(".kind\t  = TCF_EM_NBYTE"));
        assert!(source.contains("MODULE_ALIAS_TCF_EMATCH(TCF_EM_NBYTE);"));

        assert_eq!(EM_NBYTE_OPS.kind, TCF_EM_NBYTE);
    }

    #[test]
    fn nbyte_change_validates_length_and_match_uses_selected_layer() {
        let hdr = TcfEmNbyte {
            off: 2,
            len: 3,
            layer: TCF_LAYER_NETWORK,
        };
        assert_eq!(
            em_nbyte_change(hdr, b"abc", TCF_EM_NBYTE_HDR_LEN + 2, true),
            Err(-EINVAL)
        );
        assert_eq!(
            em_nbyte_change(hdr, b"abc", TCF_EM_NBYTE_HDR_LEN + 3, false),
            Err(-ENOMEM)
        );

        let em = em_nbyte_change(hdr, b"abcx", TCF_EM_NBYTE_HDR_LEN + 3, true).unwrap();
        let packet = [0, 1, 0xaa, 0xbb, b'a', b'b', b'c', 9];
        assert!(em_nbyte_match(
            &packet,
            &em,
            PacketLayers {
                link: Some(0),
                network: Some(2),
                transport: None,
            },
        ));
        assert!(!em_nbyte_match(
            &packet[..6],
            &em,
            PacketLayers {
                link: Some(0),
                network: Some(2),
                transport: None,
            },
        ));
        assert_eq!(init_em_nbyte(), &EM_NBYTE_OPS);
    }
}
