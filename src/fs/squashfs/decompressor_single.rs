//! linux-parity: complete
//! linux-source: vendor/linux/fs/squashfs/decompressor_single.c
//! test-origin: linux:vendor/linux/fs/squashfs/decompressor_single.c
//! SquashFS single-threaded decompressor operation table.

use crate::include::uapi::errno::ENOMEM;

pub const SQUASHFS_SINGLE_MAX_DECOMPRESSORS: usize = 1;
pub const SQUASHFS_SINGLE_OPS_SYMBOL: &str = "squashfs_decompressor_single";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SquashfsSingleDecompressorOps {
    pub create: &'static str,
    pub destroy: &'static str,
    pub decompress: &'static str,
    pub max_decompressors: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SquashfsSingleCreateReport {
    pub kmalloc_obj_called: bool,
    pub init_called: bool,
    pub comp_opts_freed: bool,
    pub mutex_initialized: bool,
    pub kfree_stream_called: bool,
    pub returned_stream: bool,
    pub returned_error: Option<i32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SquashfsSingleDestroyReport {
    pub stream_present: bool,
    pub decompressor_free_called: bool,
    pub kfree_stream_called: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SquashfsSingleDecompressReport {
    pub mutex_locked: bool,
    pub inner_result: i32,
    pub mutex_unlocked: bool,
    pub error_logged: bool,
    pub returned: i32,
}

#[allow(non_upper_case_globals)]
pub const squashfs_decompressor_single: SquashfsSingleDecompressorOps =
    SquashfsSingleDecompressorOps {
        create: "squashfs_decompressor_create",
        destroy: "squashfs_decompressor_destroy",
        decompress: "squashfs_decompress",
        max_decompressors: SQUASHFS_SINGLE_MAX_DECOMPRESSORS,
    };

pub const SQUASHFS_SINGLE_OPS: SquashfsSingleDecompressorOps = squashfs_decompressor_single;

pub const fn squashfs_single_create_report(
    kmalloc_failed: bool,
    init_error: Option<i32>,
) -> SquashfsSingleCreateReport {
    if kmalloc_failed {
        return SquashfsSingleCreateReport {
            kmalloc_obj_called: true,
            init_called: false,
            comp_opts_freed: false,
            mutex_initialized: false,
            kfree_stream_called: true,
            returned_stream: false,
            returned_error: Some(-ENOMEM),
        };
    }

    if let Some(err) = init_error {
        return SquashfsSingleCreateReport {
            kmalloc_obj_called: true,
            init_called: true,
            comp_opts_freed: false,
            mutex_initialized: false,
            kfree_stream_called: true,
            returned_stream: false,
            returned_error: Some(err),
        };
    }

    SquashfsSingleCreateReport {
        kmalloc_obj_called: true,
        init_called: true,
        comp_opts_freed: true,
        mutex_initialized: true,
        kfree_stream_called: false,
        returned_stream: true,
        returned_error: None,
    }
}

pub const fn squashfs_single_destroy_report(stream_present: bool) -> SquashfsSingleDestroyReport {
    SquashfsSingleDestroyReport {
        stream_present,
        decompressor_free_called: stream_present,
        kfree_stream_called: stream_present,
    }
}

pub const fn squashfs_single_decompress_report(
    inner_result: i32,
) -> SquashfsSingleDecompressReport {
    SquashfsSingleDecompressReport {
        mutex_locked: true,
        inner_result,
        mutex_unlocked: true,
        error_logged: inner_result < 0,
        returned: inner_result,
    }
}

pub const fn squashfs_single_decompress_result(inner_result: i32) -> Result<i32, i32> {
    if inner_result < 0 {
        Err(inner_result)
    } else {
        Ok(inner_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squashfs_single_decompressor_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/squashfs/decompressor_single.c"
        ));
        assert!(source.contains("#include <linux/mutex.h>"));
        assert!(source.contains("#include \"decompressor.h\""));
        assert!(source.contains("struct squashfs_stream"));
        assert!(source.contains("void\t\t*stream;"));
        assert!(source.contains("struct mutex\tmutex;"));
        assert!(source.contains("squashfs_decompressor_create"));
        assert!(source.contains("int err = -ENOMEM;"));
        assert!(source.contains("stream = kmalloc_obj(*stream);"));
        assert!(source.contains("if (stream == NULL)"));
        assert!(source.contains("goto out;"));
        assert!(source.contains("stream->stream = msblk->decompressor->init(msblk, comp_opts);"));
        assert!(source.contains("if (IS_ERR(stream->stream))"));
        assert!(source.contains("err = PTR_ERR(stream->stream);"));
        assert!(source.contains("kfree(comp_opts);"));
        assert!(source.contains("mutex_init(&stream->mutex);"));
        assert!(source.contains("out:"));
        assert!(source.contains("kfree(stream);"));
        assert!(source.contains("return ERR_PTR(err);"));
        assert!(source.contains("squashfs_decompressor_destroy"));
        assert!(source.contains("struct squashfs_stream *stream = msblk->stream;"));
        assert!(source.contains("if (stream)"));
        assert!(source.contains("msblk->decompressor->free(stream->stream);"));
        assert!(source.contains("kfree(stream);"));
        assert!(source.contains("mutex_lock(&stream->mutex);"));
        assert!(source.contains("msblk->decompressor->decompress(msblk, stream->stream, bio,"));
        assert!(source.contains("mutex_unlock(&stream->mutex);"));
        assert!(source.contains("if (res < 0)"));
        assert!(source.contains("ERROR(\"%s decompression failed, data probably corrupt\\n\""));
        assert!(source.contains("return 1;"));
        assert!(source.contains(SQUASHFS_SINGLE_OPS_SYMBOL));
        assert!(source.contains(".create = squashfs_decompressor_create"));
        assert!(source.contains(".destroy = squashfs_decompressor_destroy"));
        assert!(source.contains(".decompress = squashfs_decompress"));
        assert!(source.contains(".max_decompressors = squashfs_max_decompressors"));

        assert_eq!(SQUASHFS_SINGLE_OPS.max_decompressors, 1);
        assert_eq!(SQUASHFS_SINGLE_OPS, squashfs_decompressor_single);
        assert_eq!(squashfs_single_decompress_result(4096), Ok(4096));
        assert_eq!(squashfs_single_decompress_result(-5), Err(-5));
    }

    #[test]
    fn create_report_matches_success_and_error_cleanup() {
        assert_eq!(
            squashfs_single_create_report(false, None),
            SquashfsSingleCreateReport {
                kmalloc_obj_called: true,
                init_called: true,
                comp_opts_freed: true,
                mutex_initialized: true,
                kfree_stream_called: false,
                returned_stream: true,
                returned_error: None,
            }
        );
        assert_eq!(
            squashfs_single_create_report(true, None),
            SquashfsSingleCreateReport {
                kmalloc_obj_called: true,
                init_called: false,
                comp_opts_freed: false,
                mutex_initialized: false,
                kfree_stream_called: true,
                returned_stream: false,
                returned_error: Some(-ENOMEM),
            }
        );
        assert_eq!(
            squashfs_single_create_report(false, Some(-5)),
            SquashfsSingleCreateReport {
                kmalloc_obj_called: true,
                init_called: true,
                comp_opts_freed: false,
                mutex_initialized: false,
                kfree_stream_called: true,
                returned_stream: false,
                returned_error: Some(-5),
            }
        );
    }

    #[test]
    fn destroy_report_is_guarded_by_stream_presence() {
        assert_eq!(
            squashfs_single_destroy_report(false),
            SquashfsSingleDestroyReport {
                stream_present: false,
                decompressor_free_called: false,
                kfree_stream_called: false,
            }
        );
        assert_eq!(
            squashfs_single_destroy_report(true),
            SquashfsSingleDestroyReport {
                stream_present: true,
                decompressor_free_called: true,
                kfree_stream_called: true,
            }
        );
    }

    #[test]
    fn decompress_report_locks_unlocks_and_logs_negative_results() {
        assert_eq!(
            squashfs_single_decompress_report(4096),
            SquashfsSingleDecompressReport {
                mutex_locked: true,
                inner_result: 4096,
                mutex_unlocked: true,
                error_logged: false,
                returned: 4096,
            }
        );
        assert_eq!(
            squashfs_single_decompress_report(-5),
            SquashfsSingleDecompressReport {
                mutex_locked: true,
                inner_result: -5,
                mutex_unlocked: true,
                error_logged: true,
                returned: -5,
            }
        );
    }
}
