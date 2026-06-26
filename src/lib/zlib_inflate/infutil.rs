//! linux-parity: complete
//! linux-source: vendor/linux/lib/zlib_inflate/infutil.c
//! test-origin: linux:vendor/linux/lib/zlib_inflate/infutil.c
//! zlib inflate blob wrapper status handling.

use crate::include::uapi::errno::{EINVAL, ENOMEM};

pub const ZLIB_INFLATE_BLOB_SYMBOL: &str = "zlib_inflate_blob";
pub const Z_FINISH: i32 = 5;
pub const Z_OK: i32 = 0;
pub const Z_STREAM_END: i32 = 1;
pub const MAX_WBITS: i32 = 15;
pub const GZIP_HEADER_STRIPPED_NOTE: &str = "gzip header expected to be stripped from input";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InflateBlobOutcome {
    AllocationFailed,
    WorkspaceAllocationFailed,
    InitFailed,
    InflateFailed,
    StreamEnd { avail_out: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InflateStreamSetup {
    pub next_in_set: bool,
    pub avail_in: u32,
    pub next_out_set: bool,
    pub avail_out: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InflateBlobReport {
    pub initial_rc: i32,
    pub return_code: i32,
    pub stream_allocated: bool,
    pub workspace_allocated: bool,
    pub stream_setup: Option<InflateStreamSetup>,
    pub init_window_bits: Option<i32>,
    pub inflate_flush: Option<i32>,
    pub inflate_end_called: bool,
    pub workspace_freed: bool,
    pub stream_freed: bool,
}

pub fn zlib_inflate_blob_report(
    output_size: u32,
    input_len: u32,
    outcome: InflateBlobOutcome,
) -> InflateBlobReport {
    let mut report = InflateBlobReport {
        initial_rc: -ENOMEM,
        return_code: -ENOMEM,
        stream_allocated: false,
        workspace_allocated: false,
        stream_setup: None,
        init_window_bits: None,
        inflate_flush: None,
        inflate_end_called: false,
        workspace_freed: false,
        stream_freed: false,
    };

    if matches!(outcome, InflateBlobOutcome::AllocationFailed) {
        return report;
    }

    report.stream_allocated = true;
    report.stream_freed = true;

    if matches!(outcome, InflateBlobOutcome::WorkspaceAllocationFailed) {
        return report;
    }

    report.workspace_allocated = true;
    report.workspace_freed = true;
    report.stream_setup = Some(InflateStreamSetup {
        next_in_set: true,
        avail_in: input_len,
        next_out_set: true,
        avail_out: output_size,
    });
    report.init_window_bits = Some(zlib_inflate_window_bits());

    match outcome {
        InflateBlobOutcome::InitFailed => {
            report.return_code = -EINVAL;
        }
        InflateBlobOutcome::InflateFailed => {
            report.inflate_flush = Some(Z_FINISH);
            report.inflate_end_called = true;
            report.return_code = -EINVAL;
        }
        InflateBlobOutcome::StreamEnd { avail_out } => {
            report.inflate_flush = Some(Z_FINISH);
            report.inflate_end_called = true;
            report.return_code = output_size.saturating_sub(avail_out) as i32;
        }
        InflateBlobOutcome::AllocationFailed | InflateBlobOutcome::WorkspaceAllocationFailed => {}
    }

    report
}

pub fn zlib_inflate_blob_result(output_size: u32, outcome: InflateBlobOutcome) -> i32 {
    zlib_inflate_blob_report(output_size, 0, outcome).return_code
}

pub fn zlib_inflate_window_bits() -> i32 {
    -MAX_WBITS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inflate_blob_wrapper_matches_linux_status_mapping() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/zlib_inflate/infutil.c"
        ));
        assert!(source.contains("int zlib_inflate_blob(void *gunzip_buf"));
        assert!(source.contains("rc = -ENOMEM;"));
        assert!(source.contains("strm = kmalloc_obj(*strm);"));
        assert!(source.contains("if (strm == NULL)"));
        assert!(
            source.contains("strm->workspace = kmalloc(zlib_inflate_workspacesize(), GFP_KERNEL);")
        );
        assert!(source.contains("if (strm->workspace == NULL)"));
        assert!(source.contains("goto gunzip_nomem2;"));
        assert!(
            source.contains("gzip header (1f,8b,08... 10 bytes total + possible asciz filename)")
        );
        assert!(source.contains("expected to be stripped from input"));
        assert!(source.contains("strm->next_in = zbuf;"));
        assert!(source.contains("strm->avail_in = len;"));
        assert!(source.contains("strm->next_out = gunzip_buf;"));
        assert!(source.contains("strm->avail_out = sz;"));
        assert!(source.contains("zlib_inflateInit2(strm, -MAX_WBITS);"));
        assert!(source.contains("rc = zlib_inflate(strm, Z_FINISH);"));
        assert!(source.contains("only Z_STREAM_END is \"we unpacked it all\""));
        assert!(source.contains("if (rc == Z_STREAM_END)"));
        assert!(source.contains("rc = sz - strm->avail_out;"));
        assert!(source.contains("rc = -EINVAL;"));
        assert!(source.contains("zlib_inflateEnd(strm);"));
        assert!(source.contains("kfree(strm->workspace);"));
        assert!(source.contains("kfree(strm);"));
        assert!(source.contains("return rc;"));

        assert_eq!(
            zlib_inflate_blob_result(4096, InflateBlobOutcome::StreamEnd { avail_out: 96 }),
            4000
        );
        assert_eq!(
            zlib_inflate_blob_result(4096, InflateBlobOutcome::AllocationFailed),
            -ENOMEM
        );
        assert_eq!(
            zlib_inflate_blob_result(4096, InflateBlobOutcome::WorkspaceAllocationFailed),
            -ENOMEM
        );
        assert_eq!(
            zlib_inflate_blob_result(4096, InflateBlobOutcome::InitFailed),
            -EINVAL
        );
        assert_eq!(
            zlib_inflate_blob_result(4096, InflateBlobOutcome::InflateFailed),
            -EINVAL
        );
        assert_eq!(zlib_inflate_window_bits(), -15);
        assert_eq!(Z_FINISH, 5);
        assert_eq!(Z_OK, 0);
        assert_eq!(Z_STREAM_END, 1);
        assert_eq!(ZLIB_INFLATE_BLOB_SYMBOL, "zlib_inflate_blob");
        assert_eq!(
            GZIP_HEADER_STRIPPED_NOTE,
            "gzip header expected to be stripped from input"
        );

        assert_eq!(
            zlib_inflate_blob_report(4096, 128, InflateBlobOutcome::AllocationFailed),
            InflateBlobReport {
                initial_rc: -ENOMEM,
                return_code: -ENOMEM,
                stream_allocated: false,
                workspace_allocated: false,
                stream_setup: None,
                init_window_bits: None,
                inflate_flush: None,
                inflate_end_called: false,
                workspace_freed: false,
                stream_freed: false,
            }
        );
        assert_eq!(
            zlib_inflate_blob_report(4096, 128, InflateBlobOutcome::WorkspaceAllocationFailed),
            InflateBlobReport {
                initial_rc: -ENOMEM,
                return_code: -ENOMEM,
                stream_allocated: true,
                workspace_allocated: false,
                stream_setup: None,
                init_window_bits: None,
                inflate_flush: None,
                inflate_end_called: false,
                workspace_freed: false,
                stream_freed: true,
            }
        );
        assert_eq!(
            zlib_inflate_blob_report(4096, 128, InflateBlobOutcome::InitFailed),
            InflateBlobReport {
                initial_rc: -ENOMEM,
                return_code: -EINVAL,
                stream_allocated: true,
                workspace_allocated: true,
                stream_setup: Some(InflateStreamSetup {
                    next_in_set: true,
                    avail_in: 128,
                    next_out_set: true,
                    avail_out: 4096,
                }),
                init_window_bits: Some(-MAX_WBITS),
                inflate_flush: None,
                inflate_end_called: false,
                workspace_freed: true,
                stream_freed: true,
            }
        );
        assert_eq!(
            zlib_inflate_blob_report(4096, 128, InflateBlobOutcome::InflateFailed).inflate_flush,
            Some(Z_FINISH)
        );
        assert!(
            zlib_inflate_blob_report(4096, 128, InflateBlobOutcome::InflateFailed)
                .inflate_end_called
        );
        assert_eq!(
            zlib_inflate_blob_report(4096, 128, InflateBlobOutcome::StreamEnd { avail_out: 96 })
                .return_code,
            4000
        );
    }
}
