//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nf_conntrack_labels.c
//! test-origin: linux:vendor/linux/net/netfilter/nf_conntrack_labels.c
//! Conntrack label replacement and reference accounting.

use crate::include::uapi::errno::{ENOSPC, ERANGE};

pub const XT_CONNLABEL_MAXBIT: usize = 127;
pub const NF_CT_LABELS_MAX_SIZE: usize = (XT_CONNLABEL_MAXBIT + 1) / 8;
pub const NF_CT_LABELS_WORDS: usize = NF_CT_LABELS_MAX_SIZE / core::mem::size_of::<u32>();
pub const BITS_PER_LONG: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NfConnLabels {
    pub bits: [u32; NF_CT_LABELS_WORDS],
    pub label_event_cached: bool,
}

impl NfConnLabels {
    pub const fn new(bits: [u32; NF_CT_LABELS_WORDS]) -> Self {
        Self {
            bits,
            label_event_cached: false,
        }
    }
}

pub fn replace_u32(address: &mut u32, mask: u32, new: u32) -> bool {
    let old = *address;
    let tmp = (old & mask) ^ new;
    if old == tmp {
        return false;
    }
    *address = tmp;
    true
}

pub fn nf_connlabels_replace(
    labels: Option<&mut NfConnLabels>,
    data: &[u32],
    mask: Option<&[u32]>,
    words32: usize,
) -> Result<(), i32> {
    let Some(labels) = labels else {
        return Err(-ENOSPC);
    };

    let words32 = core::cmp::min(words32, NF_CT_LABELS_WORDS);
    let mut changed = false;
    for i in 0..words32 {
        let keep_mask = mask.and_then(|m| m.get(i)).map(|m| !*m).unwrap_or(0);
        let new = data.get(i).copied().unwrap_or(0);
        changed |= replace_u32(&mut labels.bits[i], keep_mask, new);
    }
    for i in words32..NF_CT_LABELS_WORDS {
        let _ = replace_u32(&mut labels.bits[i], 0, 0);
    }

    if changed {
        labels.label_event_cached = true;
    }
    Ok(())
}

pub fn nf_connlabels_get(labels_used: &mut i32, bits: usize) -> Result<(), i32> {
    if bits / BITS_PER_LONG >= NF_CT_LABELS_MAX_SIZE / core::mem::size_of::<usize>() {
        return Err(-ERANGE);
    }
    *labels_used = labels_used.saturating_add(1);
    Ok(())
}

pub fn nf_connlabels_put(labels_used: &mut i32) {
    *labels_used = labels_used.saturating_sub(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nf_conntrack_labels_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nf_conntrack_labels.c"
        ));
        assert!(source.contains("static int replace_u32(u32 *address, u32 mask, u32 new)"));
        assert!(source.contains("tmp = (old & mask) ^ new;"));
        assert!(source.contains("cmpxchg(address, old, tmp)"));
        assert!(source.contains("int nf_connlabels_replace"));
        assert!(source.contains("if (!labels)"));
        assert!(source.contains("return -ENOSPC;"));
        assert!(source.contains("changed |= replace_u32(&dst[i], mask ? ~mask[i] : 0, data[i]);"));
        assert!(source.contains("for (i = words32; i < size; i++) /* pad */"));
        assert!(source.contains("nf_conntrack_event_cache(IPCT_LABEL, ct);"));
        assert!(source.contains("if (BIT_WORD(bits) >= NF_CT_LABELS_MAX_SIZE / sizeof(long))"));
        assert!(source.contains("return -ERANGE;"));
        assert!(source.contains("atomic_inc_return_relaxed(&net->ct.labels_used);"));
        assert!(source.contains("atomic_dec_return_relaxed(&net->ct.labels_used);"));

        let mut labels = NfConnLabels::new([0xffff_0000, 0, 0xffff_ffff, 7]);
        nf_connlabels_replace(
            Some(&mut labels),
            &[0x0000_00aa, 0x55],
            Some(&[0x0000_ff00, 0xffff_ffff]),
            2,
        )
        .unwrap();
        assert_eq!(labels.bits, [0xffff_00aa, 0x55, 0, 0]);
        assert!(labels.label_event_cached);
        assert_eq!(nf_connlabels_replace(None, &[1], None, 1), Err(-ENOSPC));

        let mut used = 0;
        assert_eq!(nf_connlabels_get(&mut used, 127), Ok(()));
        assert_eq!(used, 1);
        assert_eq!(nf_connlabels_get(&mut used, 128), Err(-ERANGE));
        nf_connlabels_put(&mut used);
        assert_eq!(used, 0);
    }
}
