//! linux-parity: complete
//! linux-source: vendor/linux/fs/squashfs/decompressor_multi_percpu.c
//! test-origin: linux:vendor/linux/fs/squashfs/decompressor_multi_percpu.c
//! SquashFS percpu decompressor lifecycle decisions.

use crate::include::uapi::errno::ENOMEM;

pub const SQUASHFS_PERCPU_OPS_SYMBOL: &str = "squashfs_decompressor_percpu";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SquashfsPerCpuDecompressorOps {
    pub create: &'static str,
    pub destroy: &'static str,
    pub decompress: &'static str,
    pub max_decompressors: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SquashfsPerCpuCreate {
    pub streams_initialized: usize,
    pub comp_opts_freed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SquashfsPerCpuCreateReport {
    pub alloc_percpu_called: bool,
    pub possible_cpus: usize,
    pub init_attempts: usize,
    pub locks_initialized: usize,
    pub streams_freed_on_error: usize,
    pub free_percpu_called: bool,
    pub comp_opts_freed: bool,
    pub returned_percpu: bool,
    pub returned_error: Option<i32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SquashfsPerCpuDestroyReport {
    pub stream_present: bool,
    pub free_calls: usize,
    pub free_percpu_called: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SquashfsPerCpuDecompressReport {
    pub local_lock_taken: bool,
    pub this_cpu_stream_selected: bool,
    pub inner_result: i32,
    pub local_lock_released: bool,
    pub error_logged: bool,
    pub returned: i32,
}

#[allow(non_upper_case_globals)]
pub const squashfs_decompressor_percpu: SquashfsPerCpuDecompressorOps =
    SquashfsPerCpuDecompressorOps {
        create: "squashfs_decompressor_create",
        destroy: "squashfs_decompressor_destroy",
        decompress: "squashfs_decompress",
        max_decompressors: "squashfs_max_decompressors",
    };

pub const SQUASHFS_PERCPU_OPS: SquashfsPerCpuDecompressorOps = squashfs_decompressor_percpu;

pub const fn squashfs_decompressor_create_report(
    cpus: usize,
    alloc_percpu_ok: bool,
    failed_cpu: Option<usize>,
    stream_err: i32,
) -> SquashfsPerCpuCreateReport {
    if !alloc_percpu_ok {
        return SquashfsPerCpuCreateReport {
            alloc_percpu_called: true,
            possible_cpus: cpus,
            init_attempts: 0,
            locks_initialized: 0,
            streams_freed_on_error: 0,
            free_percpu_called: false,
            comp_opts_freed: false,
            returned_percpu: false,
            returned_error: Some(-ENOMEM),
        };
    }

    if let Some(cpu) = failed_cpu {
        if cpu < cpus {
            return SquashfsPerCpuCreateReport {
                alloc_percpu_called: true,
                possible_cpus: cpus,
                init_attempts: cpu + 1,
                locks_initialized: cpu,
                streams_freed_on_error: cpu,
                free_percpu_called: true,
                comp_opts_freed: false,
                returned_percpu: false,
                returned_error: Some(stream_err),
            };
        }
    }

    SquashfsPerCpuCreateReport {
        alloc_percpu_called: true,
        possible_cpus: cpus,
        init_attempts: cpus,
        locks_initialized: cpus,
        streams_freed_on_error: 0,
        free_percpu_called: false,
        comp_opts_freed: true,
        returned_percpu: true,
        returned_error: None,
    }
}

pub const fn squashfs_decompressor_create_result(
    cpus: usize,
    alloc_percpu_ok: bool,
    failed_cpu: Option<usize>,
    stream_err: i32,
) -> Result<SquashfsPerCpuCreate, i32> {
    let report = squashfs_decompressor_create_report(cpus, alloc_percpu_ok, failed_cpu, stream_err);
    if let Some(err) = report.returned_error {
        Err(err)
    } else {
        Ok(SquashfsPerCpuCreate {
            streams_initialized: report.init_attempts,
            comp_opts_freed: report.comp_opts_freed,
        })
    }
}

pub const fn squashfs_decompressor_destroy_report(
    stream_present: bool,
    cpus: usize,
) -> SquashfsPerCpuDestroyReport {
    SquashfsPerCpuDestroyReport {
        stream_present,
        free_calls: if stream_present { cpus } else { 0 },
        free_percpu_called: stream_present,
    }
}

pub const fn squashfs_decompress_report_detail(
    decompress_ret: i32,
) -> SquashfsPerCpuDecompressReport {
    SquashfsPerCpuDecompressReport {
        local_lock_taken: true,
        this_cpu_stream_selected: true,
        inner_result: decompress_ret,
        local_lock_released: true,
        error_logged: decompress_ret < 0,
        returned: decompress_ret,
    }
}

pub const fn squashfs_decompress_result(decompress_ret: i32) -> Result<i32, i32> {
    if decompress_ret < 0 {
        Err(decompress_ret)
    } else {
        Ok(decompress_ret)
    }
}

pub const fn squashfs_max_decompressors(num_possible_cpus: usize) -> usize {
    num_possible_cpus
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squashfs_decompressor_multi_percpu_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/squashfs/decompressor_multi_percpu.c"
        ));
        assert!(source.contains("#include <linux/percpu.h>"));
        assert!(source.contains("#include <linux/local_lock.h>"));
        assert!(source.contains("#include \"decompressor.h\""));
        assert!(source.contains("struct squashfs_stream"));
        assert!(source.contains("local_lock_t\tlock;"));
        assert!(source.contains("percpu = alloc_percpu(struct squashfs_stream);"));
        assert!(source.contains("return ERR_PTR(-ENOMEM);"));
        assert!(source.contains("for_each_possible_cpu(cpu)"));
        assert!(source.contains("stream->stream = msblk->decompressor->init(msblk, comp_opts);"));
        assert!(source.contains("if (IS_ERR(stream->stream))"));
        assert!(source.contains("local_lock_init(&stream->lock);"));
        assert!(source.contains("kfree(comp_opts);"));
        assert!(source.contains("free_percpu(percpu);"));
        assert!(source.contains("local_lock(&percpu->lock);"));
        assert!(source.contains("stream = this_cpu_ptr(percpu);"));
        assert!(source.contains("msblk->decompressor->decompress(msblk, stream->stream, bio,"));
        assert!(source.contains("local_unlock(&percpu->lock);"));
        assert!(source.contains("if (res < 0)"));
        assert!(source.contains("return num_possible_cpus();"));
        assert!(source.contains(
            "const struct squashfs_decompressor_thread_ops squashfs_decompressor_percpu"
        ));
        assert!(source.contains(".create = squashfs_decompressor_create"));
        assert!(source.contains(".destroy = squashfs_decompressor_destroy"));
        assert!(source.contains(".decompress = squashfs_decompress"));
        assert!(source.contains(".max_decompressors = squashfs_max_decompressors"));

        assert_eq!(
            SQUASHFS_PERCPU_OPS,
            SquashfsPerCpuDecompressorOps {
                create: "squashfs_decompressor_create",
                destroy: "squashfs_decompressor_destroy",
                decompress: "squashfs_decompress",
                max_decompressors: "squashfs_max_decompressors",
            }
        );
        assert_eq!(SQUASHFS_PERCPU_OPS, squashfs_decompressor_percpu);
        assert_eq!(SQUASHFS_PERCPU_OPS_SYMBOL, "squashfs_decompressor_percpu");
        assert_eq!(
            squashfs_decompressor_create_result(4, true, None, -5),
            Ok(SquashfsPerCpuCreate {
                streams_initialized: 4,
                comp_opts_freed: true,
            })
        );
        assert_eq!(
            squashfs_decompressor_create_result(4, false, None, -5),
            Err(-ENOMEM)
        );
        assert_eq!(
            squashfs_decompressor_create_result(4, true, Some(2), -5),
            Err(-5)
        );
        assert_eq!(squashfs_decompress_result(12), Ok(12));
        assert_eq!(squashfs_decompress_result(-5), Err(-5));
        assert_eq!(squashfs_max_decompressors(8), 8);
    }

    #[test]
    fn create_report_matches_success_and_error_cleanup() {
        assert_eq!(
            squashfs_decompressor_create_report(4, true, None, -5),
            SquashfsPerCpuCreateReport {
                alloc_percpu_called: true,
                possible_cpus: 4,
                init_attempts: 4,
                locks_initialized: 4,
                streams_freed_on_error: 0,
                free_percpu_called: false,
                comp_opts_freed: true,
                returned_percpu: true,
                returned_error: None,
            }
        );
        assert_eq!(
            squashfs_decompressor_create_report(4, false, None, -5),
            SquashfsPerCpuCreateReport {
                alloc_percpu_called: true,
                possible_cpus: 4,
                init_attempts: 0,
                locks_initialized: 0,
                streams_freed_on_error: 0,
                free_percpu_called: false,
                comp_opts_freed: false,
                returned_percpu: false,
                returned_error: Some(-ENOMEM),
            }
        );
        assert_eq!(
            squashfs_decompressor_create_report(4, true, Some(2), -5),
            SquashfsPerCpuCreateReport {
                alloc_percpu_called: true,
                possible_cpus: 4,
                init_attempts: 3,
                locks_initialized: 2,
                streams_freed_on_error: 2,
                free_percpu_called: true,
                comp_opts_freed: false,
                returned_percpu: false,
                returned_error: Some(-5),
            }
        );
    }

    #[test]
    fn destroy_report_frees_each_possible_cpu_stream() {
        assert_eq!(
            squashfs_decompressor_destroy_report(false, 4),
            SquashfsPerCpuDestroyReport {
                stream_present: false,
                free_calls: 0,
                free_percpu_called: false,
            }
        );
        assert_eq!(
            squashfs_decompressor_destroy_report(true, 4),
            SquashfsPerCpuDestroyReport {
                stream_present: true,
                free_calls: 4,
                free_percpu_called: true,
            }
        );
    }

    #[test]
    fn decompress_report_locks_current_cpu_stream_and_logs_errors() {
        assert_eq!(
            squashfs_decompress_report_detail(4096),
            SquashfsPerCpuDecompressReport {
                local_lock_taken: true,
                this_cpu_stream_selected: true,
                inner_result: 4096,
                local_lock_released: true,
                error_logged: false,
                returned: 4096,
            }
        );
        assert_eq!(
            squashfs_decompress_report_detail(-5),
            SquashfsPerCpuDecompressReport {
                local_lock_taken: true,
                this_cpu_stream_selected: true,
                inner_result: -5,
                local_lock_released: true,
                error_logged: true,
                returned: -5,
            }
        );
    }
}
