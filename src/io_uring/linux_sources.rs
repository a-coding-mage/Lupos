//! linux-parity: complete
//! linux-source: vendor/linux/io_uring
//! test-origin: linux:vendor/linux/io_uring
//! Authoritative list of upstream `vendor/linux/io_uring/*.c` files covered
//! by the lupos io_uring port.
//!
//! Every entry here is paired with a Rust module under `src/io_uring/` whose
//! file stem matches the upstream stem.
//!
//! Adding a vendor file means: (a) drop it in `vendor/linux/io_uring/`, (b)
//! add the Rust module, (c) append the vendor path here.

pub const LINUX_SOURCES: &[&str] = &[
    "vendor/linux/io_uring/advise.c",
    "vendor/linux/io_uring/alloc_cache.c",
    "vendor/linux/io_uring/bpf-ops.c",
    "vendor/linux/io_uring/bpf_filter.c",
    "vendor/linux/io_uring/cancel.c",
    "vendor/linux/io_uring/cmd_net.c",
    "vendor/linux/io_uring/epoll.c",
    "vendor/linux/io_uring/eventfd.c",
    "vendor/linux/io_uring/fdinfo.c",
    "vendor/linux/io_uring/filetable.c",
    "vendor/linux/io_uring/fs.c",
    "vendor/linux/io_uring/futex.c",
    "vendor/linux/io_uring/io-wq.c",
    "vendor/linux/io_uring/io_uring.c",
    "vendor/linux/io_uring/kbuf.c",
    "vendor/linux/io_uring/loop.c",
    "vendor/linux/io_uring/memmap.c",
    "vendor/linux/io_uring/mock_file.c",
    "vendor/linux/io_uring/msg_ring.c",
    "vendor/linux/io_uring/napi.c",
    "vendor/linux/io_uring/net.c",
    "vendor/linux/io_uring/nop.c",
    "vendor/linux/io_uring/notif.c",
    "vendor/linux/io_uring/opdef.c",
    "vendor/linux/io_uring/openclose.c",
    "vendor/linux/io_uring/poll.c",
    "vendor/linux/io_uring/query.c",
    "vendor/linux/io_uring/register.c",
    "vendor/linux/io_uring/rsrc.c",
    "vendor/linux/io_uring/rw.c",
    "vendor/linux/io_uring/splice.c",
    "vendor/linux/io_uring/sqpoll.c",
    "vendor/linux/io_uring/statx.c",
    "vendor/linux/io_uring/sync.c",
    "vendor/linux/io_uring/tctx.c",
    "vendor/linux/io_uring/timeout.c",
    "vendor/linux/io_uring/truncate.c",
    "vendor/linux/io_uring/tw.c",
    "vendor/linux/io_uring/uring_cmd.c",
    "vendor/linux/io_uring/wait.c",
    "vendor/linux/io_uring/waitid.c",
    "vendor/linux/io_uring/xattr.c",
    "vendor/linux/io_uring/zcrx.c",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_sources_count_matches_phase_10_m60_scope() {
        // Every vendor/linux/io_uring/*.c file is enumerated here; 43 entries
        // matches the Phase 10 / M60 kernel-gap denominator.
        assert_eq!(LINUX_SOURCES.len(), 43);
    }

    #[test]
    fn linux_sources_sorted_for_diff_friendliness() {
        for w in LINUX_SOURCES.windows(2) {
            assert!(
                w[0] < w[1],
                "linux_sources.rs is not sorted: {} >= {}",
                w[0],
                w[1]
            );
        }
    }
}
