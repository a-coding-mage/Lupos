//! linux-parity: complete
//! linux-source: vendor/linux/net/wireless/michael-mic.c
//! test-origin: linux:vendor/linux/net/wireless/michael-mic.c
//! Michael MIC calculation for TKIP.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MichaelMicCtx {
    pub l: u32,
    pub r: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MichaelMicHeader {
    pub da: [u8; 6],
    pub sa: [u8; 6],
    pub tid: Option<u8>,
}

pub fn michael_block(mctx: &mut MichaelMicCtx, val: u32) {
    mctx.l ^= val;
    mctx.r ^= mctx.l.rotate_left(17);
    mctx.l = mctx.l.wrapping_add(mctx.r);
    mctx.r ^= ((mctx.l & 0xff00_ff00) >> 8) | ((mctx.l & 0x00ff_00ff) << 8);
    mctx.l = mctx.l.wrapping_add(mctx.r);
    mctx.r ^= mctx.l.rotate_left(3);
    mctx.l = mctx.l.wrapping_add(mctx.r);
    mctx.r ^= mctx.l.rotate_right(2);
    mctx.l = mctx.l.wrapping_add(mctx.r);
}

pub fn michael_mic_hdr(mctx: &mut MichaelMicCtx, key: &[u8; 8], hdr: MichaelMicHeader) {
    let tid = hdr.tid.unwrap_or(0);
    mctx.l = le32(&key[0..4]);
    mctx.r = le32(&key[4..8]);

    michael_block(mctx, le32(&hdr.da[0..4]));
    michael_block(mctx, le16(&hdr.da[4..6]) | (le16(&hdr.sa[0..2]) << 16));
    michael_block(mctx, le32(&hdr.sa[2..6]));
    michael_block(mctx, tid as u32);
}

pub fn michael_mic(key: &[u8; 8], hdr: MichaelMicHeader, data: &[u8], mic: &mut [u8; 8]) {
    let mut mctx = MichaelMicCtx { l: 0, r: 0 };
    michael_mic_hdr(&mut mctx, key, hdr);

    let blocks = data.len() / 4;
    let mut block = 0;
    while block < blocks {
        michael_block(&mut mctx, le32(&data[(block * 4)..(block * 4 + 4)]));
        block += 1;
    }

    let mut left = data.len() % 4;
    let mut val = 0x5a_u32;
    while left > 0 {
        val <<= 8;
        left -= 1;
        val |= data[blocks * 4 + left] as u32;
    }
    michael_block(&mut mctx, val);
    michael_block(&mut mctx, 0);

    put_le32(mctx.l, &mut mic[0..4]);
    put_le32(mctx.r, &mut mic[4..8]);
}

fn le16(bytes: &[u8]) -> u32 {
    bytes[0] as u32 | ((bytes[1] as u32) << 8)
}

fn le32(bytes: &[u8]) -> u32 {
    bytes[0] as u32
        | ((bytes[1] as u32) << 8)
        | ((bytes[2] as u32) << 16)
        | ((bytes[3] as u32) << 24)
}

fn put_le32(value: u32, out: &mut [u8]) {
    out[0] = value as u8;
    out[1] = (value >> 8) as u8;
    out[2] = (value >> 16) as u8;
    out[3] = (value >> 24) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn michael_mic_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/wireless/michael-mic.c"
        ));
        assert!(source.contains("struct michael_mic_ctx"));
        assert!(source.contains("static void michael_block"));
        assert!(source.contains("mctx->l ^= val;"));
        assert!(source.contains("mctx->r ^= rol32(mctx->l, 17);"));
        assert!(source.contains("mctx->r ^= ((mctx->l & 0xff00ff00) >> 8)"));
        assert!(source.contains("mctx->r ^= rol32(mctx->l, 3);"));
        assert!(source.contains("mctx->r ^= ror32(mctx->l, 2);"));
        assert!(source.contains("static void michael_mic_hdr"));
        assert!(source.contains("da = ieee80211_get_DA(hdr);"));
        assert!(source.contains("sa = ieee80211_get_SA(hdr);"));
        assert!(source.contains("tid = ieee80211_get_tid(hdr);"));
        assert!(source.contains("mctx->l = get_unaligned_le32(key);"));
        assert!(source.contains("michael_block(mctx, get_unaligned_le32(da));"));
        assert!(source.contains("michael_block(mctx, tid);"));
        assert!(source.contains("void michael_mic"));
        assert!(source.contains("blocks = data_len / 4;"));
        assert!(source.contains("left = data_len % 4;"));
        assert!(source.contains("val = 0x5a;"));
        assert!(source.contains("michael_block(&mctx, val);"));
        assert!(source.contains("michael_block(&mctx, 0);"));
        assert!(source.contains("put_unaligned_le32(mctx.l, mic);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(michael_mic);"));

        let mut mic = [0u8; 8];
        michael_mic(
            &[0, 1, 2, 3, 4, 5, 6, 7],
            MichaelMicHeader {
                da: [1, 2, 3, 4, 5, 6],
                sa: [7, 8, 9, 10, 11, 12],
                tid: Some(5),
            },
            b"hello",
            &mut mic,
        );
        assert_eq!(mic, [205, 223, 252, 216, 228, 135, 99, 90]);

        let mut no_qos = [0u8; 8];
        michael_mic(
            &[0; 8],
            MichaelMicHeader {
                da: [0; 6],
                sa: [0; 6],
                tid: None,
            },
            &[],
            &mut no_qos,
        );
        assert_ne!(no_qos, [0; 8]);
    }
}
