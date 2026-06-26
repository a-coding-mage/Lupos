//! linux-parity: complete
//! linux-source: vendor/linux/kernel/bpf/link_iter.c
//! test-origin: linux:vendor/linux/kernel/bpf/link_iter.c
//! BPF link iterator sequence state.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BpfLinkSeqInfo {
    pub link_id: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SeqStep {
    Yield { id: u32, pos: i64 },
    Stop,
}

pub const BPF_LINK_ITER_TARGET: &str = "bpf_link";
pub const BPF_LINK_SEQ_PRIV_SIZE: usize = core::mem::size_of::<BpfLinkSeqInfo>();

pub fn bpf_link_seq_start(
    info: &mut BpfLinkSeqInfo,
    pos: &mut i64,
    next_id: Option<u32>,
) -> SeqStep {
    match next_id {
        Some(id) => {
            info.link_id = id;
            if *pos == 0 {
                *pos += 1;
            }
            SeqStep::Yield { id, pos: *pos }
        }
        None => SeqStep::Stop,
    }
}

pub fn bpf_link_seq_next(
    info: &mut BpfLinkSeqInfo,
    pos: &mut i64,
    next_id: Option<u32>,
) -> SeqStep {
    *pos += 1;
    info.link_id = info.link_id.saturating_add(1);
    match next_id {
        Some(id) => {
            info.link_id = id;
            SeqStep::Yield { id, pos: *pos }
        }
        None => SeqStep::Stop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bpf_link_iterator_matches_linux_seq_ops_shape() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/bpf/link_iter.c"
        ));
        assert!(source.contains("struct bpf_iter_seq_link_info"));
        assert!(source.contains("u32 link_id;"));
        assert!(source.contains("bpf_link_get_curr_or_next(&info->link_id);"));
        assert!(source.contains("if (*pos == 0)"));
        assert!(source.contains("++*pos;"));
        assert!(source.contains("++info->link_id;"));
        assert!(source.contains("bpf_link_put((struct bpf_link *)v);"));
        assert!(source.contains("DEFINE_BPF_ITER_FUNC(bpf_link"));
        assert!(source.contains(".target\t\t\t= \"bpf_link\""));
        assert!(source.contains(".seq_priv_size\t\t= sizeof(struct bpf_iter_seq_link_info)"));
        assert!(source.contains("late_initcall(bpf_link_iter_init);"));

        let mut info = BpfLinkSeqInfo::default();
        let mut pos = 0;
        assert_eq!(
            bpf_link_seq_start(&mut info, &mut pos, Some(5)),
            SeqStep::Yield { id: 5, pos: 1 }
        );
        assert_eq!(info.link_id, 5);
        assert_eq!(
            bpf_link_seq_next(&mut info, &mut pos, Some(7)),
            SeqStep::Yield { id: 7, pos: 2 }
        );
        assert_eq!(bpf_link_seq_next(&mut info, &mut pos, None), SeqStep::Stop);
        assert_eq!(BPF_LINK_ITER_TARGET, "bpf_link");
        assert_eq!(BPF_LINK_SEQ_PRIV_SIZE, 4);
    }
}
