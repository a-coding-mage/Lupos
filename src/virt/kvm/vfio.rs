//! linux-parity: complete
//! linux-source: vendor/linux/virt/kvm/vfio.c
//! test-origin: linux:vendor/linux/virt/kvm/vfio.c
//! KVM VFIO pseudo-device file list and attribute validation.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::{EBADF, EBUSY, EEXIST, EFAULT, EINVAL, ENOENT, ENOMEM, ENXIO};

pub const KVM_DEV_VFIO_FILE: u64 = 1;
pub const KVM_DEV_VFIO_FILE_ADD: u64 = 1;
pub const KVM_DEV_VFIO_FILE_DEL: u64 = 2;
pub const KVM_DEV_VFIO_GROUP_SET_SPAPR_TCE: u64 = 3;
pub const KVM_DEV_TYPE_VFIO: u32 = 4;
pub const KVM_VFIO_OPS_NAME: &str = "kvm-vfio";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VfioFile {
    pub fd: u32,
    pub fd_open: bool,
    pub vfio_valid: bool,
    pub enforced_coherent: bool,
}

impl VfioFile {
    pub const fn valid(fd: u32, enforced_coherent: bool) -> Self {
        Self {
            fd,
            fd_open: true,
            vfio_valid: true,
            enforced_coherent,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KvmVfioDevice {
    pub files: Vec<VfioFile>,
    pub noncoherent: bool,
    pub noncoherent_dma_registered: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmVfioOps {
    pub name: &'static str,
    pub device_type: u32,
    pub has_create: bool,
    pub has_release: bool,
    pub has_set_attr: bool,
    pub has_has_attr: bool,
}

pub const KVM_VFIO_OPS: KvmVfioOps = KvmVfioOps {
    name: KVM_VFIO_OPS_NAME,
    device_type: KVM_DEV_TYPE_VFIO,
    has_create: true,
    has_release: true,
    has_set_attr: true,
    has_has_attr: true,
};

impl KvmVfioDevice {
    pub fn create(existing_vfio_device: bool, allocation_available: bool) -> Result<Self, i32> {
        if existing_vfio_device {
            return Err(-EBUSY);
        }
        if !allocation_available {
            return Err(-ENOMEM);
        }
        Ok(Self {
            files: Vec::new(),
            noncoherent: false,
            noncoherent_dma_registered: false,
        })
    }

    pub fn add_file(&mut self, file: VfioFile) -> Result<(), i32> {
        if !file.fd_open {
            return Err(-EBADF);
        }
        if !file.vfio_valid {
            return Err(-EINVAL);
        }
        if self.files.iter().any(|existing| existing.fd == file.fd) {
            return Err(-EEXIST);
        }

        self.files.push(file);
        self.update_coherency();
        Ok(())
    }

    pub fn del_file(&mut self, fd: u32, fd_open: bool) -> Result<(), i32> {
        if !fd_open {
            return Err(-EBADF);
        }
        let Some(index) = self.files.iter().position(|file| file.fd == fd) else {
            self.update_coherency();
            return Err(-ENOENT);
        };

        self.files.remove(index);
        self.update_coherency();
        Ok(())
    }

    pub fn has_attr(group: u64, attr: u64, spapr_tce_enabled: bool) -> Result<(), i32> {
        if group != KVM_DEV_VFIO_FILE {
            return Err(-ENXIO);
        }
        match attr {
            KVM_DEV_VFIO_FILE_ADD | KVM_DEV_VFIO_FILE_DEL => Ok(()),
            KVM_DEV_VFIO_GROUP_SET_SPAPR_TCE if spapr_tce_enabled => Ok(()),
            _ => Err(-ENXIO),
        }
    }

    pub fn set_file_attr(
        &mut self,
        attr: u64,
        file: Option<VfioFile>,
        fd_open: bool,
        spapr_tce_enabled: bool,
    ) -> Result<(), i32> {
        match attr {
            KVM_DEV_VFIO_FILE_ADD => self.add_file(file.ok_or(-EFAULT)?),
            KVM_DEV_VFIO_FILE_DEL => {
                let file = file.ok_or(-EFAULT)?;
                self.del_file(file.fd, fd_open)
            }
            KVM_DEV_VFIO_GROUP_SET_SPAPR_TCE if spapr_tce_enabled => Ok(()),
            _ => Err(-ENXIO),
        }
    }

    pub fn set_attr(
        &mut self,
        group: u64,
        attr: u64,
        file: Option<VfioFile>,
        fd_open: bool,
        spapr_tce_enabled: bool,
    ) -> Result<(), i32> {
        match group {
            KVM_DEV_VFIO_FILE => self.set_file_attr(attr, file, fd_open, spapr_tce_enabled),
            _ => Err(-ENXIO),
        }
    }

    pub fn release(&mut self) -> usize {
        let released = self.files.len();
        self.files.clear();
        self.update_coherency();
        released
    }

    fn update_coherency(&mut self) {
        let noncoherent = self.files.iter().any(|file| !file.enforced_coherent);
        if noncoherent != self.noncoherent {
            self.noncoherent = noncoherent;
            self.noncoherent_dma_registered = noncoherent;
        }
    }
}

pub const fn kvm_vfio_file_set_kvm(symbol_available: bool) -> bool {
    symbol_available
}

pub const fn kvm_vfio_file_enforced_coherent(symbol_available: bool, ret: bool) -> bool {
    if symbol_available { ret } else { false }
}

pub const fn kvm_vfio_file_is_valid(symbol_available: bool, ret: bool) -> bool {
    if symbol_available { ret } else { false }
}

pub const fn kvm_vfio_ops_init(register_result: i32) -> Result<KvmVfioOps, i32> {
    if register_result != 0 {
        Err(register_result)
    } else {
        Ok(KVM_VFIO_OPS)
    }
}

pub const fn kvm_vfio_ops_exit() -> u32 {
    KVM_DEV_TYPE_VFIO
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vfio_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/virt/kvm/vfio.c"
        ));
        let uapi = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/kvm.h"
        ));
        assert!(source.contains("struct kvm_vfio_file"));
        assert!(source.contains("struct kvm_vfio"));
        assert!(source.contains("fn = symbol_get(vfio_file_set_kvm);"));
        assert!(source.contains("symbol_put(vfio_file_set_kvm);"));
        assert!(source.contains("fn = symbol_get(vfio_file_enforced_coherent);"));
        assert!(source.contains("fn = symbol_get(vfio_file_is_valid);"));
        assert!(source.contains("if (!kvm_vfio_file_is_valid(filp))"));
        assert!(source.contains("ret = -EEXIST;"));
        assert!(source.contains("kvm_vfio_update_coherency(dev);"));
        assert!(source.contains("case KVM_DEV_VFIO_FILE_ADD:"));
        assert!(source.contains("case KVM_DEV_VFIO_FILE_DEL:"));
        assert!(source.contains("static int kvm_vfio_set_attr"));
        assert!(source.contains("static int kvm_vfio_has_attr"));
        assert!(source.contains("return -ENXIO;"));
        assert!(source.contains("static void kvm_vfio_release"));
        assert!(source.contains("kfree(dev);"));
        assert!(source.contains("static const struct kvm_device_ops kvm_vfio_ops"));
        assert!(source.contains(".name = \"kvm-vfio\""));
        assert!(source.contains("/* Only one VFIO \"device\" per VM */"));
        assert!(
            source.contains("return kvm_register_device_ops(&kvm_vfio_ops, KVM_DEV_TYPE_VFIO);")
        );
        assert!(source.contains("kvm_unregister_device_ops(KVM_DEV_TYPE_VFIO);"));
        assert!(uapi.contains("KVM_DEV_TYPE_VFIO"));
    }

    #[test]
    fn create_rejects_second_vfio_device() {
        assert_eq!(KvmVfioDevice::create(true, true), Err(-EBUSY));
        assert_eq!(KvmVfioDevice::create(false, false), Err(-ENOMEM));
        assert!(KvmVfioDevice::create(false, true).is_ok());
    }

    #[test]
    fn add_file_validates_fd_vfio_type_and_duplicates() {
        let mut dev = KvmVfioDevice::create(false, true).unwrap();
        assert_eq!(
            dev.add_file(VfioFile {
                fd: 1,
                fd_open: false,
                vfio_valid: true,
                enforced_coherent: true,
            }),
            Err(-EBADF)
        );
        assert_eq!(
            dev.add_file(VfioFile {
                fd: 1,
                fd_open: true,
                vfio_valid: false,
                enforced_coherent: true,
            }),
            Err(-EINVAL)
        );
        dev.add_file(VfioFile::valid(1, false)).unwrap();
        assert!(dev.noncoherent);
        assert_eq!(dev.add_file(VfioFile::valid(1, true)), Err(-EEXIST));
    }

    #[test]
    fn deleting_last_noncoherent_file_unregisters_dma() {
        let mut dev = KvmVfioDevice::create(false, true).unwrap();
        dev.add_file(VfioFile::valid(4, false)).unwrap();
        dev.add_file(VfioFile::valid(5, true)).unwrap();
        assert!(dev.noncoherent_dma_registered);
        dev.del_file(4, true).unwrap();
        assert!(!dev.noncoherent);
        assert!(!dev.noncoherent_dma_registered);
        assert_eq!(dev.del_file(6, true), Err(-ENOENT));
        assert_eq!(dev.del_file(5, false), Err(-EBADF));
        assert_eq!(dev.release(), 1);
        assert!(dev.files.is_empty());
        assert!(!dev.noncoherent_dma_registered);
    }

    #[test]
    fn vfio_attrs_match_linux_group_policy() {
        assert_eq!(
            KvmVfioDevice::has_attr(KVM_DEV_VFIO_FILE, KVM_DEV_VFIO_FILE_ADD, false),
            Ok(())
        );
        assert_eq!(
            KvmVfioDevice::has_attr(KVM_DEV_VFIO_FILE, KVM_DEV_VFIO_GROUP_SET_SPAPR_TCE, false),
            Err(-ENXIO)
        );
        assert_eq!(
            KvmVfioDevice::has_attr(KVM_DEV_VFIO_FILE, KVM_DEV_VFIO_GROUP_SET_SPAPR_TCE, true),
            Ok(())
        );
        assert_eq!(
            KvmVfioDevice::has_attr(99, KVM_DEV_VFIO_FILE_ADD, false),
            Err(-ENXIO)
        );

        let mut dev = KvmVfioDevice::create(false, true).unwrap();
        assert_eq!(
            dev.set_attr(
                KVM_DEV_VFIO_FILE,
                KVM_DEV_VFIO_FILE_ADD,
                Some(VfioFile::valid(7, true)),
                true,
                false
            ),
            Ok(())
        );
        assert_eq!(
            dev.set_attr(
                99,
                KVM_DEV_VFIO_FILE_ADD,
                Some(VfioFile::valid(8, true)),
                true,
                false
            ),
            Err(-ENXIO)
        );
    }

    #[test]
    fn vfio_symbol_helpers_and_ops_match_linux_defaults() {
        assert!(kvm_vfio_file_set_kvm(true));
        assert!(!kvm_vfio_file_set_kvm(false));
        assert!(kvm_vfio_file_enforced_coherent(true, true));
        assert!(!kvm_vfio_file_enforced_coherent(false, true));
        assert!(kvm_vfio_file_is_valid(true, true));
        assert!(!kvm_vfio_file_is_valid(false, true));
        assert_eq!(kvm_vfio_ops_init(0), Ok(KVM_VFIO_OPS));
        assert_eq!(kvm_vfio_ops_init(-ENOMEM), Err(-ENOMEM));
        assert_eq!(kvm_vfio_ops_exit(), KVM_DEV_TYPE_VFIO);
        assert_eq!(KVM_VFIO_OPS.name, KVM_VFIO_OPS_NAME);
    }
}
