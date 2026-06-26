//! linux-parity: complete
//! linux-source: vendor/linux/fs/cramfs/uncompress.c
//! test-origin: linux:vendor/linux/fs/cramfs/uncompress.c
//! Cramfs zlib stream lifecycle and block outcome helpers.

use crate::include::uapi::errno::{EIO, ENOMEM};

pub const Z_OK: i32 = 0;
pub const Z_STREAM_END: i32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CramfsUncompressState {
    pub initialized: i32,
    pub workspace_allocated: bool,
}

pub const fn cramfs_uncompress_block_result(
    reset_result: i32,
    inflate_result: i32,
    total_out: i32,
) -> i32 {
    if reset_result != Z_OK {
        return -EIO;
    }
    if inflate_result == Z_STREAM_END {
        total_out
    } else {
        -EIO
    }
}

pub const fn cramfs_uncompress_init(
    mut state: CramfsUncompressState,
    workspace_alloc_ok: bool,
) -> (CramfsUncompressState, i32) {
    if state.initialized == 0 {
        if !workspace_alloc_ok {
            state.initialized = 0;
            state.workspace_allocated = false;
            return (state, -ENOMEM);
        }
        state.workspace_allocated = true;
    }
    state.initialized += 1;
    (state, 0)
}

pub const fn cramfs_uncompress_exit(mut state: CramfsUncompressState) -> CramfsUncompressState {
    state.initialized -= 1;
    if state.initialized == 0 {
        state.workspace_allocated = false;
    }
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cramfs_uncompress_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/cramfs/uncompress.c"
        ));
        assert!(source.contains("#include <linux/zlib.h>"));
        assert!(source.contains("#include \"internal.h\""));
        assert!(source.contains("static z_stream stream;"));
        assert!(source.contains("static int initialized;"));
        assert!(source.contains("int cramfs_uncompress_block"));
        assert!(source.contains("zlib_inflateReset(&stream);"));
        assert!(source.contains("if (err != Z_OK)"));
        assert!(source.contains("zlib_inflate(&stream, Z_FINISH);"));
        assert!(source.contains("if (err != Z_STREAM_END)"));
        assert!(source.contains("return -EIO;"));
        assert!(source.contains("int cramfs_uncompress_init(void)"));
        assert!(source.contains("if (!initialized++)"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("void cramfs_uncompress_exit(void)"));
        assert!(source.contains("if (!--initialized)"));

        assert_eq!(cramfs_uncompress_block_result(Z_OK, Z_STREAM_END, 123), 123);
        assert_eq!(cramfs_uncompress_block_result(-1, Z_STREAM_END, 123), -EIO);
        assert_eq!(cramfs_uncompress_block_result(Z_OK, -1, 123), -EIO);

        let state = CramfsUncompressState {
            initialized: 0,
            workspace_allocated: false,
        };
        let (state, rc) = cramfs_uncompress_init(state, true);
        assert_eq!(rc, 0);
        assert_eq!(state.initialized, 1);
        assert!(state.workspace_allocated);
        assert_eq!(cramfs_uncompress_exit(state).workspace_allocated, false);
        assert_eq!(cramfs_uncompress_init(state, false).1, 0);
    }
}
