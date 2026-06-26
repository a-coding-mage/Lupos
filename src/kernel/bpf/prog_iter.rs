//! linux-parity: complete
//! linux-source: vendor/linux/kernel/bpf/prog_iter.c
//! test-origin: linux:vendor/linux/kernel/bpf/prog_iter.c
//! BPF program iterator sequence state.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BpfProgSeqInfo {
    pub prog_id: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SeqStep {
    Yield { id: u32, pos: i64 },
    Stop,
}

pub const BPF_PROG_ITER_TARGET: &str = "bpf_prog";
pub const BPF_PROG_SEQ_PRIV_SIZE: usize = core::mem::size_of::<BpfProgSeqInfo>();

pub fn bpf_prog_seq_start(
    info: &mut BpfProgSeqInfo,
    pos: &mut i64,
    next_id: Option<u32>,
) -> SeqStep {
    match next_id {
        Some(id) => {
            info.prog_id = id;
            if *pos == 0 {
                *pos += 1;
            }
            SeqStep::Yield { id, pos: *pos }
        }
        None => SeqStep::Stop,
    }
}

pub fn bpf_prog_seq_next(
    info: &mut BpfProgSeqInfo,
    pos: &mut i64,
    next_id: Option<u32>,
) -> SeqStep {
    *pos += 1;
    info.prog_id = info.prog_id.saturating_add(1);
    match next_id {
        Some(id) => {
            info.prog_id = id;
            SeqStep::Yield { id, pos: *pos }
        }
        None => SeqStep::Stop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bpf_prog_iterator_matches_linux_seq_ops_shape() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/bpf/prog_iter.c"
        ));
        assert!(source.contains("struct bpf_iter_seq_prog_info"));
        assert!(source.contains("u32 prog_id;"));
        assert!(source.contains("bpf_prog_get_curr_or_next(&info->prog_id);"));
        assert!(source.contains("if (*pos == 0)"));
        assert!(source.contains("++*pos;"));
        assert!(source.contains("++info->prog_id;"));
        assert!(source.contains("bpf_prog_put((struct bpf_prog *)v);"));
        assert!(source.contains("DEFINE_BPF_ITER_FUNC(bpf_prog"));
        assert!(source.contains(".target\t\t\t= \"bpf_prog\""));
        assert!(source.contains(".seq_priv_size\t\t= sizeof(struct bpf_iter_seq_prog_info)"));
        assert!(source.contains("late_initcall(bpf_prog_iter_init);"));

        let mut info = BpfProgSeqInfo::default();
        let mut pos = 0;
        assert_eq!(
            bpf_prog_seq_start(&mut info, &mut pos, Some(11)),
            SeqStep::Yield { id: 11, pos: 1 }
        );
        assert_eq!(info.prog_id, 11);
        assert_eq!(
            bpf_prog_seq_next(&mut info, &mut pos, Some(13)),
            SeqStep::Yield { id: 13, pos: 2 }
        );
        assert_eq!(bpf_prog_seq_next(&mut info, &mut pos, None), SeqStep::Stop);
        assert_eq!(BPF_PROG_ITER_TARGET, "bpf_prog");
        assert_eq!(BPF_PROG_SEQ_PRIV_SIZE, 4);
    }
}
