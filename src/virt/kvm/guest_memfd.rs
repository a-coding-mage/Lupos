//! linux-parity: partial
//! linux-source: vendor/linux/virt/kvm/guest_memfd.c
//! test-origin: linux:vendor/linux/virt/kvm/guest_memfd.c
//! KVM guest_memfd creation, fallocate, mmap policy, and binding checks.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use crate::include::uapi::errno::{EBADF, EEXIST, EFAULT, EINVAL, ENODEV, ENOENT, EOPNOTSUPP};

pub const PAGE_SIZE: u64 = 4096;
pub const PAGE_SHIFT: u32 = 12;

pub const FALLOC_FL_KEEP_SIZE: u32 = 0x01;
pub const FALLOC_FL_PUNCH_HOLE: u32 = 0x02;

pub const GUEST_MEMFD_FLAG_MMAP: u64 = 1 << 0;
pub const GUEST_MEMFD_FLAG_INIT_SHARED: u64 = 1 << 1;

pub const KVM_MEM_GUEST_MEMFD: u32 = 1 << 2;
pub const KVM_MEMSLOT_GMEM_ONLY: u32 = 1 << 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmCreateGuestMemfd {
    pub size: u64,
    pub flags: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuestMemfdBinding {
    pub slot_id: u32,
    pub base_gfn: u64,
    pub npages: u64,
    pub pgoff: u64,
}

impl GuestMemfdBinding {
    pub const fn start(&self) -> u64 {
        self.pgoff
    }

    pub const fn end(&self) -> u64 {
        self.pgoff + self.npages
    }

    pub const fn contains_index(&self, index: u64) -> bool {
        self.start() <= index && index < self.end()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuestMemfdFile {
    pub fd: i32,
    pub inode: u64,
    pub vm_id: u64,
    pub size: u64,
    pub flags: u64,
    pub allocated: Vec<bool>,
    pub bindings: Vec<GuestMemfdBinding>,
    pub released: bool,
}

impl GuestMemfdFile {
    pub fn supports_mmap(&self) -> bool {
        self.flags & GUEST_MEMFD_FLAG_MMAP != 0
    }

    pub fn init_shared(&self) -> bool {
        self.flags & GUEST_MEMFD_FLAG_INIT_SHARED != 0
    }

    pub const fn page_count(&self) -> u64 {
        self.size >> PAGE_SHIFT
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuestMemfdRegistry {
    files: Vec<GuestMemfdFile>,
    next_fd: i32,
    next_inode: u64,
    init_shared_supported: bool,
}

impl GuestMemfdRegistry {
    pub const fn new(init_shared_supported: bool) -> Self {
        Self {
            files: Vec::new(),
            next_fd: 3,
            next_inode: 1,
            init_shared_supported,
        }
    }

    pub fn files(&self) -> &[GuestMemfdFile] {
        &self.files
    }

    pub fn supported_flags(&self) -> u64 {
        kvm_gmem_get_supported_flags(self.init_shared_supported)
    }

    pub fn create(&mut self, vm_id: u64, args: KvmCreateGuestMemfd) -> Result<i32, i32> {
        kvm_gmem_validate_create(args.size, args.flags, self.supported_flags())?;

        let fd = self.next_fd;
        let inode = self.next_inode;
        self.next_fd = self.next_fd.saturating_add(1);
        self.next_inode = self.next_inode.saturating_add(1);

        self.files.push(GuestMemfdFile {
            fd,
            inode,
            vm_id,
            size: args.size,
            flags: args.flags,
            allocated: vec![false; (args.size / PAGE_SIZE) as usize],
            bindings: Vec::new(),
            released: false,
        });
        Ok(fd)
    }

    pub fn file(&self, fd: i32) -> Option<&GuestMemfdFile> {
        self.files
            .iter()
            .find(|file| file.fd == fd && !file.released)
    }

    pub fn file_mut(&mut self, fd: i32) -> Option<&mut GuestMemfdFile> {
        self.files
            .iter_mut()
            .find(|file| file.fd == fd && !file.released)
    }

    pub fn release(&mut self, fd: i32) -> Result<(), i32> {
        let Some(file) = self.files.iter_mut().find(|file| file.fd == fd) else {
            return Err(-EBADF);
        };
        file.released = true;
        file.bindings.clear();
        Ok(())
    }

    pub fn fallocate(&mut self, fd: i32, mode: u32, offset: u64, len: u64) -> Result<(), i32> {
        let Some(file) = self.file_mut(fd) else {
            return Err(-EBADF);
        };
        kvm_gmem_validate_fallocate(file.size, mode, offset, len)?;

        let start = offset / PAGE_SIZE;
        let end = checked_page_end(offset, len)?;
        if mode & FALLOC_FL_PUNCH_HOLE != 0 {
            for page in start..end.min(file.page_count()) {
                file.allocated[page as usize] = false;
            }
        } else {
            for page in start..end {
                file.allocated[page as usize] = true;
            }
        }
        Ok(())
    }

    pub fn mmap(&self, fd: i32, shared: bool, may_share: bool) -> Result<(), i32> {
        let Some(file) = self.file(fd) else {
            return Err(-EBADF);
        };
        if !file.supports_mmap() {
            return Err(-ENODEV);
        }
        if !(shared && may_share) {
            return Err(-EINVAL);
        }
        Ok(())
    }

    pub fn fault_user_mapping(&mut self, fd: i32, pgoff: u64) -> Result<u64, GuestMemfdFault> {
        let Some(file) = self.file_mut(fd) else {
            return Err(GuestMemfdFault::Errno(-EBADF));
        };
        if pgoff >= file.page_count() {
            return Err(GuestMemfdFault::Sigbus);
        }
        if !file.init_shared() {
            return Err(GuestMemfdFault::Sigbus);
        }
        file.allocated[pgoff as usize] = true;
        Ok(pgoff)
    }

    pub fn bind(
        &mut self,
        vm_id: u64,
        fd: i32,
        slot_id: u32,
        base_gfn: u64,
        npages: u64,
        offset: u64,
    ) -> Result<BoundGuestMemfdSlot, i32> {
        let Some(file) = self.file_mut(fd) else {
            return Err(-EBADF);
        };
        if file.vm_id != vm_id {
            return Err(-EINVAL);
        }
        if !page_aligned(offset) {
            return Err(-EINVAL);
        }

        let size = npages.checked_shl(PAGE_SHIFT).ok_or(-EINVAL)?;
        if offset.checked_add(size).is_none_or(|end| end > file.size) {
            return Err(-EINVAL);
        }

        let start = offset >> PAGE_SHIFT;
        let end = start.checked_add(npages).ok_or(-EINVAL)?;
        if file
            .bindings
            .iter()
            .any(|binding| ranges_overlap(start, end, binding.start(), binding.end()))
        {
            return Err(-EEXIST);
        }

        let binding = GuestMemfdBinding {
            slot_id,
            base_gfn,
            npages,
            pgoff: start,
        };
        file.bindings.push(binding);

        let mut flags = KVM_MEM_GUEST_MEMFD;
        if file.supports_mmap() {
            flags |= KVM_MEMSLOT_GMEM_ONLY;
        }

        Ok(BoundGuestMemfdSlot {
            slot_id,
            base_gfn,
            npages,
            flags,
            gmem_fd: fd,
            gmem_pgoff: start,
        })
    }

    pub fn unbind(&mut self, fd: i32, slot_id: u32) -> Result<(), i32> {
        let Some(file) = self.file_mut(fd) else {
            return Err(-EFAULT);
        };
        let Some(index) = file
            .bindings
            .iter()
            .position(|binding| binding.slot_id == slot_id)
        else {
            return Err(-ENOENT);
        };
        file.bindings.remove(index);
        Ok(())
    }

    pub fn get_pfn(&mut self, fd: i32, slot_id: u32, gfn: u64) -> Result<u64, i32> {
        let Some(file) = self.file_mut(fd) else {
            return Err(-EFAULT);
        };
        let Some(binding) = file
            .bindings
            .iter()
            .find(|binding| binding.slot_id == slot_id)
            .copied()
        else {
            return Err(-EFAULT);
        };
        if gfn < binding.base_gfn || gfn >= binding.base_gfn + binding.npages {
            return Err(-EFAULT);
        }
        let index = binding.pgoff + (gfn - binding.base_gfn);
        if !binding.contains_index(index) {
            return Err(-EFAULT);
        }
        file.allocated[index as usize] = true;
        Ok(index)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoundGuestMemfdSlot {
    pub slot_id: u32,
    pub base_gfn: u64,
    pub npages: u64,
    pub flags: u32,
    pub gmem_fd: i32,
    pub gmem_pgoff: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuestMemfdFault {
    Sigbus,
    Errno(i32),
}

pub const fn kvm_gmem_get_supported_flags(init_shared_supported: bool) -> u64 {
    let mut flags = GUEST_MEMFD_FLAG_MMAP;
    if init_shared_supported {
        flags |= GUEST_MEMFD_FLAG_INIT_SHARED;
    }
    flags
}

pub const fn kvm_gmem_get_index(slot_base_gfn: u64, slot_pgoff: u64, gfn: u64) -> u64 {
    gfn - slot_base_gfn + slot_pgoff
}

pub const fn folio_file_pfn(folio_pfn: u64, folio_nr_pages: u64, index: u64) -> u64 {
    folio_pfn + (index & (folio_nr_pages - 1))
}

pub const fn page_aligned(value: u64) -> bool {
    value & (PAGE_SIZE - 1) == 0
}

pub const fn kvm_gmem_validate_create(
    size: u64,
    flags: u64,
    supported_flags: u64,
) -> Result<(), i32> {
    if flags & !supported_flags != 0 {
        return Err(-EINVAL);
    }
    if size == 0 || !page_aligned(size) {
        return Err(-EINVAL);
    }
    Ok(())
}

pub fn kvm_gmem_validate_fallocate(
    file_size: u64,
    mode: u32,
    offset: u64,
    len: u64,
) -> Result<(), i32> {
    if mode & FALLOC_FL_KEEP_SIZE == 0 {
        return Err(-EOPNOTSUPP);
    }
    if mode & !(FALLOC_FL_KEEP_SIZE | FALLOC_FL_PUNCH_HOLE) != 0 {
        return Err(-EOPNOTSUPP);
    }
    if !page_aligned(offset) || !page_aligned(len) {
        return Err(-EINVAL);
    }
    let end = offset.checked_add(len).ok_or(-EINVAL)?;
    if mode & FALLOC_FL_PUNCH_HOLE == 0 && end > file_size {
        return Err(-EINVAL);
    }
    Ok(())
}

fn checked_page_end(offset: u64, len: u64) -> Result<u64, i32> {
    offset
        .checked_add(len)
        .map(|end| end >> PAGE_SHIFT)
        .ok_or(-EINVAL)
}

const fn ranges_overlap(left_start: u64, left_end: u64, right_start: u64, right_end: u64) -> bool {
    left_start < right_end && right_start < left_end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guest_memfd_source_and_selftest_contract_match_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/virt/kvm/guest_memfd.c"
        ));
        assert!(source.contains("if (flags & ~kvm_gmem_get_supported_flags(kvm))"));
        assert!(source.contains("if (size <= 0 || !PAGE_ALIGNED(size))"));
        assert!(source.contains("if (!(mode & FALLOC_FL_KEEP_SIZE))"));
        assert!(source.contains("if (!PAGE_ALIGNED(offset) || !PAGE_ALIGNED(len))"));
        assert!(source.contains("offset + size > i_size_read(inode)"));
        assert!(source.contains("xa_find(&f->bindings, &start, end - 1, XA_PRESENT)"));
        assert!(source.contains("slot->flags |= KVM_MEMSLOT_GMEM_ONLY;"));

        let gmem_test = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/kvm/guest_memfd_test.c"
        ));
        assert!(gmem_test.contains("test_create_guest_memfd_invalid_sizes"));
        assert!(gmem_test.contains("test_create_guest_memfd_multiple"));
        assert!(gmem_test.contains("test_guest_memfd_flags"));
        assert!(gmem_test.contains("fallocate with unaligned offset should fail"));
        assert!(gmem_test.contains("Copy-on-write not allowed by guest_memfd."));

        let memslot_test = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/kvm/set_memory_region_test.c"
        ));
        assert!(memslot_test.contains("Testing ADD of KVM_MEM_GUEST_MEMFD memory regions"));
        assert!(memslot_test.contains("Other VM's guest_memfd() should fail"));
        assert!(
            memslot_test.contains("Overlapping guest_memfd() bindings should fail with EEXIST")
        );
    }

    #[test]
    fn create_rejects_unsupported_flags_and_unaligned_sizes() {
        let mut registry = GuestMemfdRegistry::new(false);
        assert_eq!(
            registry.create(
                1,
                KvmCreateGuestMemfd {
                    size: PAGE_SIZE,
                    flags: GUEST_MEMFD_FLAG_INIT_SHARED,
                },
            ),
            Err(-EINVAL)
        );
        assert_eq!(
            registry.create(
                1,
                KvmCreateGuestMemfd {
                    size: PAGE_SIZE - 1,
                    flags: 0,
                },
            ),
            Err(-EINVAL)
        );

        let fd1 = registry
            .create(
                1,
                KvmCreateGuestMemfd {
                    size: PAGE_SIZE,
                    flags: 0,
                },
            )
            .unwrap();
        let fd2 = registry
            .create(
                1,
                KvmCreateGuestMemfd {
                    size: PAGE_SIZE * 2,
                    flags: 0,
                },
            )
            .unwrap();
        assert_ne!(
            registry.file(fd1).unwrap().inode,
            registry.file(fd2).unwrap().inode
        );
        assert_eq!(registry.file(fd1).unwrap().size, PAGE_SIZE);
    }

    #[test]
    fn fallocate_matches_linux_keep_size_alignment_and_punch_hole_policy() {
        let mut registry = GuestMemfdRegistry::new(true);
        let fd = registry
            .create(
                1,
                KvmCreateGuestMemfd {
                    size: PAGE_SIZE * 4,
                    flags: GUEST_MEMFD_FLAG_MMAP | GUEST_MEMFD_FLAG_INIT_SHARED,
                },
            )
            .unwrap();

        assert_eq!(
            registry.fallocate(fd, FALLOC_FL_KEEP_SIZE, 0, PAGE_SIZE * 4),
            Ok(())
        );
        assert!(
            registry
                .file(fd)
                .unwrap()
                .allocated
                .iter()
                .all(|page| *page)
        );
        assert_eq!(
            registry.fallocate(fd, FALLOC_FL_KEEP_SIZE, PAGE_SIZE - 1, PAGE_SIZE),
            Err(-EINVAL)
        );
        assert_eq!(
            registry.fallocate(fd, FALLOC_FL_KEEP_SIZE, PAGE_SIZE * 4, PAGE_SIZE),
            Err(-EINVAL)
        );
        assert_eq!(
            registry.fallocate(
                fd,
                FALLOC_FL_KEEP_SIZE | FALLOC_FL_PUNCH_HOLE,
                PAGE_SIZE * 4,
                PAGE_SIZE
            ),
            Ok(())
        );
        assert_eq!(
            registry.fallocate(
                fd,
                FALLOC_FL_KEEP_SIZE | FALLOC_FL_PUNCH_HOLE,
                PAGE_SIZE,
                PAGE_SIZE
            ),
            Ok(())
        );
        assert!(!registry.file(fd).unwrap().allocated[1]);
    }

    #[test]
    fn mmap_requires_mmap_flag_shared_mapping_and_init_shared_for_faults() {
        let mut registry = GuestMemfdRegistry::new(true);
        let private_fd = registry
            .create(
                1,
                KvmCreateGuestMemfd {
                    size: PAGE_SIZE,
                    flags: GUEST_MEMFD_FLAG_MMAP,
                },
            )
            .unwrap();
        let shared_fd = registry
            .create(
                1,
                KvmCreateGuestMemfd {
                    size: PAGE_SIZE,
                    flags: GUEST_MEMFD_FLAG_MMAP | GUEST_MEMFD_FLAG_INIT_SHARED,
                },
            )
            .unwrap();
        let nommap_fd = registry
            .create(
                1,
                KvmCreateGuestMemfd {
                    size: PAGE_SIZE,
                    flags: 0,
                },
            )
            .unwrap();

        assert_eq!(registry.mmap(nommap_fd, true, true), Err(-ENODEV));
        assert_eq!(registry.mmap(private_fd, false, true), Err(-EINVAL));
        assert_eq!(
            registry.fault_user_mapping(private_fd, 0),
            Err(GuestMemfdFault::Sigbus)
        );
        assert_eq!(
            registry.fault_user_mapping(shared_fd, 1),
            Err(GuestMemfdFault::Sigbus)
        );
        assert_eq!(registry.fault_user_mapping(shared_fd, 0), Ok(0));
    }

    #[test]
    fn bind_rejects_wrong_vm_unaligned_offset_and_overlapping_ranges() {
        let mut registry = GuestMemfdRegistry::new(true);
        let fd = registry
            .create(
                1,
                KvmCreateGuestMemfd {
                    size: PAGE_SIZE * 4,
                    flags: GUEST_MEMFD_FLAG_MMAP,
                },
            )
            .unwrap();

        assert_eq!(registry.bind(2, fd, 0, 0, 1, 0), Err(-EINVAL));
        assert_eq!(registry.bind(1, fd, 0, 0, 1, 1), Err(-EINVAL));

        let slot = registry.bind(1, fd, 0, 0x1000, 2, 0).unwrap();
        assert_eq!(slot.gmem_pgoff, 0);
        assert_eq!(slot.flags, KVM_MEM_GUEST_MEMFD | KVM_MEMSLOT_GMEM_ONLY);
        assert_eq!(registry.bind(1, fd, 1, 0x3000, 2, PAGE_SIZE), Err(-EEXIST));
        assert_eq!(
            registry.bind(1, fd, 2, 0x5000, 2, PAGE_SIZE * 2),
            Ok(BoundGuestMemfdSlot {
                slot_id: 2,
                base_gfn: 0x5000,
                npages: 2,
                flags: KVM_MEM_GUEST_MEMFD | KVM_MEMSLOT_GMEM_ONLY,
                gmem_fd: fd,
                gmem_pgoff: 2,
            })
        );
    }

    #[test]
    fn get_pfn_uses_slot_base_and_guest_memfd_offset() {
        let mut registry = GuestMemfdRegistry::new(false);
        let fd = registry
            .create(
                1,
                KvmCreateGuestMemfd {
                    size: PAGE_SIZE * 4,
                    flags: 0,
                },
            )
            .unwrap();
        registry.bind(1, fd, 9, 100, 2, PAGE_SIZE).unwrap();

        assert_eq!(kvm_gmem_get_index(100, 1, 101), 2);
        assert_eq!(registry.get_pfn(fd, 9, 101), Ok(2));
        assert!(registry.file(fd).unwrap().allocated[2]);
        assert_eq!(registry.get_pfn(fd, 9, 102), Err(-EFAULT));
    }
}
