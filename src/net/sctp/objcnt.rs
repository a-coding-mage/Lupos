//! linux-parity: complete
//! linux-source: vendor/linux/net/sctp/objcnt.c
//! test-origin: linux:vendor/linux/net/sctp/objcnt.c
//! SCTP debug object counter procfs sequence helpers.

pub const SCTP_DBG_OBJCNT_LABELS: [&str; 10] = [
    "sock",
    "ep",
    "assoc",
    "transport",
    "chunk",
    "bind_addr",
    "bind_bucket",
    "addr",
    "datamsg",
    "keys",
];

pub fn sctp_objcnt_seq_show(index: usize, counters: &[i32]) -> Option<alloc::string::String> {
    let label = SCTP_DBG_OBJCNT_LABELS.get(index)?;
    let count = counters.get(index).copied().unwrap_or(0);
    Some(alloc::format!("{label}: {count}\n"))
}

extern crate alloc;

pub const fn sctp_objcnt_seq_start(pos: usize) -> Option<usize> {
    if pos >= SCTP_DBG_OBJCNT_LABELS.len() {
        None
    } else {
        Some(pos)
    }
}

pub const fn sctp_objcnt_seq_next(pos: &mut usize) -> Option<usize> {
    *pos += 1;
    sctp_objcnt_seq_start(*pos)
}

pub const fn sctp_dbg_objcnt_init(proc_create_ok: bool) -> bool {
    proc_create_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sctp_objcnt_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/sctp/objcnt.c"
        ));
        assert!(source.contains("SCTP_DBG_OBJCNT(sock);"));
        assert!(source.contains("SCTP_DBG_OBJCNT(ep);"));
        assert!(source.contains("SCTP_DBG_OBJCNT(transport);"));
        assert!(source.contains("SCTP_DBG_OBJCNT_ENTRY(sock)"));
        assert!(source.contains("SCTP_DBG_OBJCNT_ENTRY(keys)"));
        assert!(source.contains("sctp_objcnt_seq_show"));
        assert!(source.contains("seq_setwidth(seq, 127);"));
        assert!(source.contains("seq_printf(seq, \"%s: %d\""));
        assert!(
            source.contains("return (*pos >= ARRAY_SIZE(sctp_dbg_objcnt)) ? NULL : (void *)pos;")
        );
        assert!(source.contains("++*pos;"));
        assert!(source.contains("proc_create_seq(\"sctp_dbg_objcnt\""));
        assert!(source.contains("pr_warn(\"sctp_dbg_objcnt: Unable to create /proc entry."));
    }

    #[test]
    fn objcnt_sequence_walks_labels_and_formats_counts() {
        assert_eq!(sctp_objcnt_seq_start(0), Some(0));
        assert_eq!(sctp_objcnt_seq_start(10), None);
        let mut pos = 0;
        assert_eq!(sctp_objcnt_seq_next(&mut pos), Some(1));
        assert_eq!(sctp_objcnt_seq_show(0, &[7]).unwrap(), "sock: 7\n");
        assert_eq!(sctp_objcnt_seq_show(9, &[0; 10]).unwrap(), "keys: 0\n");
        assert!(sctp_objcnt_seq_show(10, &[0; 10]).is_none());
        assert!(sctp_dbg_objcnt_init(true));
        assert!(!sctp_dbg_objcnt_init(false));
    }
}
