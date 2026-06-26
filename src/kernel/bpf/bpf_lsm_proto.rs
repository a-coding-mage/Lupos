//! linux-parity: complete
//! linux-source: vendor/linux/kernel/bpf/bpf_lsm_proto.c
//! test-origin: linux:vendor/linux/kernel/bpf/bpf_lsm_proto.c
//! BPF LSM mmap_file hook prototype.

use core::ffi::c_void;

#[unsafe(no_mangle)]
pub extern "C" fn bpf_lsm_mmap_file(
    _file__nullable: *mut c_void,
    _reqprot: usize,
    _prot: usize,
    _flags: usize,
) -> i32 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bpf_lsm_mmap_file_allows_access_and_requires_nullable_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/bpf/bpf_lsm_proto.c"
        ));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include <linux/bpf_lsm.h>"));
        assert!(source.contains("struct file *file__nullable"));
        assert!(source.contains("return 0;"));
        assert_eq!(bpf_lsm_mmap_file(core::ptr::null_mut(), 0, 0, 0), 0);
    }
}
