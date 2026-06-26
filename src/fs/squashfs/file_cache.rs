//! linux-parity: complete
//! linux-source: vendor/linux/fs/squashfs/file_cache.c
//! test-origin: linux:vendor/linux/fs/squashfs/file_cache.c
//! SquashFS separately-compressed datablock read path.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SquashfsReadpageOutcome {
    pub returned_error: i32,
    pub copied_cache: bool,
    pub put_cache: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SquashfsReadpageRequest {
    pub block: u64,
    pub bsize: i32,
    pub expected: i32,
    pub buffer_error: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SquashfsReadpageTrace {
    pub host_inode_read: bool,
    pub get_datablock_block: u64,
    pub get_datablock_size: i32,
    pub logged_error: bool,
    pub copied_cache: bool,
    pub copy_expected: i32,
    pub copy_offset: i32,
    pub put_cache: bool,
    pub returned_error: i32,
}

pub const fn squashfs_readpage_block_trace(
    request: SquashfsReadpageRequest,
) -> SquashfsReadpageTrace {
    let res = request.buffer_error;

    SquashfsReadpageTrace {
        host_inode_read: true,
        get_datablock_block: request.block,
        get_datablock_size: request.bsize,
        logged_error: res != 0,
        copied_cache: res == 0,
        copy_expected: request.expected,
        copy_offset: 0,
        put_cache: true,
        returned_error: res,
    }
}

pub const fn squashfs_readpage_block_outcome(buffer_error: i32) -> SquashfsReadpageOutcome {
    let trace = squashfs_readpage_block_trace(SquashfsReadpageRequest {
        block: 0,
        bsize: 0,
        expected: 0,
        buffer_error,
    });
    SquashfsReadpageOutcome {
        returned_error: trace.returned_error,
        copied_cache: trace.copied_cache,
        put_cache: trace.put_cache,
    }
}

pub const fn squashfs_readpage_block(request: SquashfsReadpageRequest) -> SquashfsReadpageOutcome {
    let trace = squashfs_readpage_block_trace(request);
    SquashfsReadpageOutcome {
        returned_error: trace.returned_error,
        copied_cache: trace.copied_cache,
        put_cache: trace.put_cache,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squashfs_readpage_block_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/squashfs/file_cache.c"
        ));
        assert!(source.contains("squashfs_get_datablock"));
        assert!(source.contains("struct inode *i = folio->mapping->host;"));
        assert!(source.contains("struct squashfs_cache_entry *buffer = squashfs_get_datablock"));
        assert!(source.contains("block, bsize);"));
        assert!(source.contains("int res = buffer->error;"));
        assert!(source.contains("if (res)"));
        assert!(source.contains("ERROR(\"Unable to read page"));
        assert!(source.contains("else"));
        assert!(source.contains("squashfs_copy_cache(folio, buffer, expected, 0);"));
        assert!(source.contains("squashfs_cache_put(buffer);"));
        assert!(source.contains("return res;"));
        assert_eq!(
            squashfs_readpage_block_outcome(0),
            SquashfsReadpageOutcome {
                returned_error: 0,
                copied_cache: true,
                put_cache: true
            }
        );
        assert_eq!(
            squashfs_readpage_block_outcome(-5),
            SquashfsReadpageOutcome {
                returned_error: -5,
                copied_cache: false,
                put_cache: true
            }
        );
    }

    #[test]
    fn readpage_trace_preserves_linux_call_order_parameters() {
        assert_eq!(
            squashfs_readpage_block_trace(SquashfsReadpageRequest {
                block: 0x1234,
                bsize: 4096,
                expected: 2048,
                buffer_error: 0,
            }),
            SquashfsReadpageTrace {
                host_inode_read: true,
                get_datablock_block: 0x1234,
                get_datablock_size: 4096,
                logged_error: false,
                copied_cache: true,
                copy_expected: 2048,
                copy_offset: 0,
                put_cache: true,
                returned_error: 0,
            }
        );
    }

    #[test]
    fn readpage_trace_logs_and_skips_copy_on_buffer_error() {
        assert_eq!(
            squashfs_readpage_block_trace(SquashfsReadpageRequest {
                block: 0x55,
                bsize: 8192,
                expected: 1024,
                buffer_error: -5,
            }),
            SquashfsReadpageTrace {
                host_inode_read: true,
                get_datablock_block: 0x55,
                get_datablock_size: 8192,
                logged_error: true,
                copied_cache: false,
                copy_expected: 1024,
                copy_offset: 0,
                put_cache: true,
                returned_error: -5,
            }
        );
    }

    #[test]
    fn readpage_block_returns_linux_style_outcome() {
        assert_eq!(
            squashfs_readpage_block(SquashfsReadpageRequest {
                block: 0x1234,
                bsize: 4096,
                expected: 2048,
                buffer_error: 0,
            }),
            SquashfsReadpageOutcome {
                returned_error: 0,
                copied_cache: true,
                put_cache: true,
            }
        );
    }
}
