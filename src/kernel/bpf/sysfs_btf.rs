//! linux-parity: complete
//! linux-source: vendor/linux/kernel/bpf/sysfs_btf.c
//! test-origin: linux:vendor/linux/kernel/bpf/sysfs_btf.c
//! `/sys/kernel/btf/vmlinux` binary attribute mapping rules.

use crate::include::uapi::errno::{EACCES, EINVAL, ENOMEM};

pub const BTF_ATTR_NAME: &str = "vmlinux";
pub const BTF_ATTR_MODE: u32 = 0o444;
pub const VM_WRITE: u32 = 1 << 0;
pub const VM_EXEC: u32 = 1 << 1;
pub const VM_MAYSHARE: u32 = 1 << 2;
pub const VM_DONTDUMP: u32 = 1 << 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BtfMmapRequest {
    pub attr_private_is_start: bool,
    pub phys_aligned: bool,
    pub vm_pgoff: u64,
    pub vm_flags: u32,
    pub pfn: u64,
    pub pages: u64,
    pub vm_pages: u64,
}

pub const fn btf_sysfs_vmlinux_mmap(req: BtfMmapRequest) -> Result<u32, i32> {
    if !req.attr_private_is_start || !req.phys_aligned {
        return Err(-EINVAL);
    }
    if req.vm_pgoff != 0 {
        return Err(-EINVAL);
    }
    if req.vm_flags & (VM_WRITE | VM_EXEC | VM_MAYSHARE) != 0 {
        return Err(-EACCES);
    }
    if req.pfn + req.pages < req.pfn {
        return Err(-EINVAL);
    }
    if req.vm_pages > req.pages {
        return Err(-EINVAL);
    }
    Ok(VM_DONTDUMP)
}

pub const fn btf_vmlinux_init(size: usize, kobject_created: bool) -> Result<bool, i32> {
    if size == 0 {
        return Ok(false);
    }
    if !kobject_created {
        return Err(-ENOMEM);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn btf_sysfs_mmap_rules_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/bpf/sysfs_btf.c"
        ));
        assert!(source.contains("Provide kernel BTF information"));
        assert!(source.contains("btf_sysfs_vmlinux_mmap"));
        assert!(source.contains("PAGE_ALIGN(attr->size) >> PAGE_SHIFT"));
        assert!(source.contains("attr->private != __start_BTF"));
        assert!(source.contains("if (vma->vm_pgoff)"));
        assert!(source.contains("VM_WRITE | VM_EXEC | VM_MAYSHARE"));
        assert!(source.contains("pfn + pages < pfn"));
        assert!(source.contains("VM_DONTDUMP"));
        assert!(source.contains(".attr = { .name = \"vmlinux\", .mode = 0444, }"));
        assert!(source.contains("kobject_create_and_add(\"btf\", kernel_kobj);"));
        assert!(source.contains("subsys_initcall(btf_vmlinux_init);"));

        let req = BtfMmapRequest {
            attr_private_is_start: true,
            phys_aligned: true,
            vm_pgoff: 0,
            vm_flags: 0,
            pfn: 8,
            pages: 2,
            vm_pages: 1,
        };
        assert_eq!(btf_sysfs_vmlinux_mmap(req), Ok(VM_DONTDUMP));
        assert_eq!(
            btf_sysfs_vmlinux_mmap(BtfMmapRequest {
                vm_flags: VM_WRITE,
                ..req
            }),
            Err(-EACCES)
        );
        assert_eq!(btf_vmlinux_init(0, false), Ok(false));
        assert_eq!(btf_vmlinux_init(10, false), Err(-ENOMEM));
    }
}
