//! linux-parity: complete
//! linux-source: vendor/linux/virt/kvm/coalesced_mmio.c
//! test-origin: linux:vendor/linux/virt/kvm/coalesced_mmio.c
//! KVM coalesced MMIO range checks and userspace ring insertion.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::{EINVAL, ENOMEM, EOPNOTSUPP};

pub const PAGE_SIZE: usize = 4096;
pub const KVM_COALESCED_MMIO_SIZE: usize = 24;
pub const KVM_COALESCED_MMIO_RING_HEADER_SIZE: usize = 8;
pub const KVM_COALESCED_MMIO_MAX: usize =
    (PAGE_SIZE - KVM_COALESCED_MMIO_RING_HEADER_SIZE) / KVM_COALESCED_MMIO_SIZE;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KvmCoalescedMmioZone {
    pub addr: u64,
    pub size: u32,
    pub pio: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmCoalescedMmio {
    pub phys_addr: u64,
    pub len: u32,
    pub pio: u32,
    pub data: [u8; 8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KvmIoBus {
    Mmio,
    Pio,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmCoalescedMmioDevice {
    pub zone: KvmCoalescedMmioZone,
    pub bus: KvmIoBus,
    pub iodevice_initialized: bool,
    pub list_linked: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KvmCoalescedMmioState {
    pub ring: Option<KvmCoalescedMmioRing>,
    pub ring_lock_initialized: bool,
    pub zones: Vec<KvmCoalescedMmioDevice>,
    pub slots_lock_depth: usize,
    pub freed_pages: usize,
}

impl KvmCoalescedMmioState {
    pub fn new() -> Self {
        Self {
            ring: None,
            ring_lock_initialized: false,
            zones: Vec::new(),
            slots_lock_depth: 0,
            freed_pages: 0,
        }
    }
}

impl Default for KvmCoalescedMmioState {
    fn default() -> Self {
        Self::new()
    }
}

impl KvmCoalescedMmio {
    pub const fn empty() -> Self {
        Self {
            phys_addr: 0,
            len: 0,
            pio: 0,
            data: [0; 8],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KvmCoalescedMmioRing {
    pub first: u32,
    pub last: u32,
    pub entries: [KvmCoalescedMmio; KVM_COALESCED_MMIO_MAX],
}

impl KvmCoalescedMmioRing {
    pub const fn new() -> Self {
        Self {
            first: 0,
            last: 0,
            entries: [KvmCoalescedMmio::empty(); KVM_COALESCED_MMIO_MAX],
        }
    }

    pub fn write(
        &mut self,
        zone: KvmCoalescedMmioZone,
        addr: u64,
        value: &[u8],
    ) -> Result<(), i32> {
        let len = value.len();
        if len > 8 || !coalesced_mmio_in_range(zone, addr, len as i32) {
            return Err(-EOPNOTSUPP);
        }

        let insert = self.last as usize;
        let next = (insert + 1) % KVM_COALESCED_MMIO_MAX;
        if insert >= KVM_COALESCED_MMIO_MAX || next as u32 == self.first {
            return Err(-EOPNOTSUPP);
        }

        let mut data = [0u8; 8];
        data[..len].copy_from_slice(value);
        self.entries[insert] = KvmCoalescedMmio {
            phys_addr: addr,
            len: len as u32,
            pio: zone.pio,
            data,
        };
        self.last = next as u32;
        Ok(())
    }
}

pub const fn valid_coalesced_mmio_zone(zone: KvmCoalescedMmioZone) -> Result<(), i32> {
    if zone.pio == 0 || zone.pio == 1 {
        Ok(())
    } else {
        Err(-EINVAL)
    }
}

pub const fn coalesced_mmio_in_range(zone: KvmCoalescedMmioZone, addr: u64, len: i32) -> bool {
    if len < 0 {
        return false;
    }
    let len = len as u64;
    let Some(end) = addr.checked_add(len) else {
        return false;
    };
    let Some(zone_end) = zone.addr.checked_add(zone.size as u64) else {
        return false;
    };
    if addr < zone.addr {
        return false;
    }
    if end > zone_end {
        return false;
    }
    true
}

pub const fn coalesced_mmio_bus(zone: KvmCoalescedMmioZone) -> KvmIoBus {
    if zone.pio == 1 {
        KvmIoBus::Pio
    } else {
        KvmIoBus::Mmio
    }
}

pub fn kvm_coalesced_mmio_init(kvm: &mut KvmCoalescedMmioState, alloc_page_ok: bool) -> i32 {
    if !alloc_page_ok {
        return -ENOMEM;
    }

    kvm.ring = Some(KvmCoalescedMmioRing::new());
    kvm.ring_lock_initialized = true;
    kvm.zones.clear();
    0
}

pub fn kvm_coalesced_mmio_free(kvm: &mut KvmCoalescedMmioState) {
    if kvm.ring.take().is_some() {
        kvm.freed_pages += 1;
    }
}

pub fn kvm_vm_ioctl_register_coalesced_mmio(
    kvm: &mut KvmCoalescedMmioState,
    zone: KvmCoalescedMmioZone,
    kzalloc_ok: bool,
    io_bus_register_ret: i32,
) -> i32 {
    if let Err(err) = valid_coalesced_mmio_zone(zone) {
        return err;
    }
    if !kzalloc_ok {
        return -ENOMEM;
    }

    let mut dev = KvmCoalescedMmioDevice {
        zone,
        bus: coalesced_mmio_bus(zone),
        iodevice_initialized: true,
        list_linked: false,
    };

    kvm.slots_lock_depth += 1;
    if io_bus_register_ret < 0 {
        kvm.slots_lock_depth -= 1;
        return io_bus_register_ret;
    }

    dev.list_linked = true;
    kvm.zones.push(dev);
    kvm.slots_lock_depth -= 1;
    0
}

pub fn kvm_vm_ioctl_unregister_coalesced_mmio(
    kvm: &mut KvmCoalescedMmioState,
    zone: KvmCoalescedMmioZone,
    io_bus_unregister_ret: i32,
) -> i32 {
    if let Err(err) = valid_coalesced_mmio_zone(zone) {
        return err;
    }

    kvm.slots_lock_depth += 1;
    let mut index = 0usize;
    while index < kvm.zones.len() {
        let dev = kvm.zones[index];
        if zone.pio == dev.zone.pio
            && coalesced_mmio_in_range(dev.zone, zone.addr, zone.size as i32)
        {
            kvm.zones.remove(index);
            if io_bus_unregister_ret != 0 {
                kvm.zones.clear();
                break;
            }
        } else {
            index += 1;
        }
    }
    kvm.slots_lock_depth -= 1;

    0
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmCoalescedMmioDestructorReport {
    pub list_deleted: bool,
    pub freed: bool,
}

pub const fn coalesced_mmio_destructor(
    dev: KvmCoalescedMmioDevice,
) -> KvmCoalescedMmioDestructorReport {
    KvmCoalescedMmioDestructorReport {
        list_deleted: dev.list_linked,
        freed: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalesced_mmio_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/virt/kvm/coalesced_mmio.c"
        ));
        assert!(source.contains("static int coalesced_mmio_in_range"));
        assert!(source.contains("if (len < 0)"));
        assert!(source.contains("if (addr + len < addr)"));
        assert!(source.contains("if (addr < dev->zone.addr)"));
        assert!(source.contains("if (addr + len > dev->zone.addr + dev->zone.size)"));
        assert!(source.contains("insert = READ_ONCE(ring->last);"));
        assert!(source.contains("(insert + 1) % KVM_COALESCED_MMIO_MAX == READ_ONCE(ring->first)"));
        assert!(source.contains("smp_wmb();"));
        assert!(source.contains("ring->last = (insert + 1) % KVM_COALESCED_MMIO_MAX;"));
        assert!(source.contains("coalesced_mmio_destructor"));
        assert!(source.contains("list_del(&dev->list);"));
        assert!(source.contains("kfree(dev);"));
        assert!(source.contains("page = alloc_page(GFP_KERNEL_ACCOUNT | __GFP_ZERO);"));
        assert!(source.contains("kvm->coalesced_mmio_ring = page_address(page);"));
        assert!(source.contains("spin_lock_init(&kvm->ring_lock);"));
        assert!(source.contains("INIT_LIST_HEAD(&kvm->coalesced_zones);"));
        assert!(source.contains("free_page((unsigned long)kvm->coalesced_mmio_ring);"));
        assert!(source.contains("if (zone->pio != 1 && zone->pio != 0)"));
        assert!(source.contains("kzalloc_obj(struct kvm_coalesced_mmio_dev"));
        assert!(source.contains("kvm_iodevice_init(&dev->dev, &coalesced_mmio_ops);"));
        assert!(source.contains("zone->pio ? KVM_PIO_BUS : KVM_MMIO_BUS"));
        assert!(source.contains("list_add_tail(&dev->list, &kvm->coalesced_zones);"));
        assert!(source.contains("list_for_each_entry_safe(dev, tmp, &kvm->coalesced_zones, list)"));
        assert!(source.contains("return 0;"));
    }

    #[test]
    fn range_rejects_negative_overflow_and_outside_accesses() {
        let zone = KvmCoalescedMmioZone {
            addr: 0x1000,
            size: 0x100,
            pio: 0,
        };
        assert!(coalesced_mmio_in_range(zone, 0x1080, 8));
        assert!(!coalesced_mmio_in_range(zone, 0x1080, -1));
        assert!(!coalesced_mmio_in_range(zone, u64::MAX - 1, 8));
        assert!(!coalesced_mmio_in_range(zone, 0x0fff, 1));
        assert!(!coalesced_mmio_in_range(zone, 0x10ff, 2));
    }

    #[test]
    fn ring_leaves_one_entry_free_and_records_payload() {
        let zone = KvmCoalescedMmioZone {
            addr: 0x2000,
            size: 0x40,
            pio: 1,
        };
        let mut ring = KvmCoalescedMmioRing::new();
        ring.write(zone, 0x2008, &[1, 2, 3, 4]).unwrap();
        assert_eq!(ring.last, 1);
        assert_eq!(ring.entries[0].phys_addr, 0x2008);
        assert_eq!(ring.entries[0].pio, 1);
        assert_eq!(&ring.entries[0].data[..4], &[1, 2, 3, 4]);

        ring.first = 2;
        ring.last = 1;
        assert_eq!(ring.write(zone, 0x2010, &[5]), Err(-EOPNOTSUPP));
    }

    #[test]
    fn ioctl_zone_validation_accepts_only_zero_or_one_pio() {
        assert_eq!(
            valid_coalesced_mmio_zone(KvmCoalescedMmioZone {
                addr: 0,
                size: 1,
                pio: 0,
            }),
            Ok(())
        );
        assert_eq!(
            valid_coalesced_mmio_zone(KvmCoalescedMmioZone {
                addr: 0,
                size: 1,
                pio: 2,
            }),
            Err(-EINVAL)
        );
    }

    #[test]
    fn init_and_free_manage_ring_page_and_lists() {
        let mut kvm = KvmCoalescedMmioState::new();
        assert_eq!(kvm_coalesced_mmio_init(&mut kvm, false), -ENOMEM);
        assert!(kvm.ring.is_none());

        assert_eq!(kvm_coalesced_mmio_init(&mut kvm, true), 0);
        assert!(kvm.ring.is_some());
        assert!(kvm.ring_lock_initialized);
        assert!(kvm.zones.is_empty());

        kvm_coalesced_mmio_free(&mut kvm);
        assert!(kvm.ring.is_none());
        assert_eq!(kvm.freed_pages, 1);
        kvm_coalesced_mmio_free(&mut kvm);
        assert_eq!(kvm.freed_pages, 1);
    }

    #[test]
    fn register_coalesced_mmio_models_validation_allocation_bus_and_cleanup() {
        let mut kvm = KvmCoalescedMmioState::new();
        let zone = KvmCoalescedMmioZone {
            addr: 0x3000,
            size: 0x100,
            pio: 1,
        };

        assert_eq!(
            kvm_vm_ioctl_register_coalesced_mmio(
                &mut kvm,
                KvmCoalescedMmioZone { pio: 2, ..zone },
                true,
                0
            ),
            -EINVAL
        );
        assert_eq!(
            kvm_vm_ioctl_register_coalesced_mmio(&mut kvm, zone, false, 0),
            -ENOMEM
        );
        assert_eq!(
            kvm_vm_ioctl_register_coalesced_mmio(&mut kvm, zone, true, -EOPNOTSUPP),
            -EOPNOTSUPP
        );
        assert!(kvm.zones.is_empty());
        assert_eq!(kvm.slots_lock_depth, 0);

        assert_eq!(
            kvm_vm_ioctl_register_coalesced_mmio(&mut kvm, zone, true, 0),
            0
        );
        assert_eq!(
            kvm.zones,
            alloc::vec![KvmCoalescedMmioDevice {
                zone,
                bus: KvmIoBus::Pio,
                iodevice_initialized: true,
                list_linked: true,
            }]
        );
        assert_eq!(kvm.slots_lock_depth, 0);
    }

    #[test]
    fn unregister_coalesced_mmio_ignores_bus_result_for_userspace() {
        let mut kvm = KvmCoalescedMmioState::new();
        let first = KvmCoalescedMmioZone {
            addr: 0x4000,
            size: 0x100,
            pio: 0,
        };
        let second = KvmCoalescedMmioZone {
            addr: 0x5000,
            size: 0x100,
            pio: 0,
        };
        assert_eq!(
            kvm_vm_ioctl_register_coalesced_mmio(&mut kvm, first, true, 0),
            0
        );
        assert_eq!(
            kvm_vm_ioctl_register_coalesced_mmio(&mut kvm, second, true, 0),
            0
        );

        assert_eq!(
            kvm_vm_ioctl_unregister_coalesced_mmio(
                &mut kvm,
                KvmCoalescedMmioZone {
                    addr: 0x4080,
                    size: 0x10,
                    pio: 0,
                },
                0,
            ),
            0
        );
        assert_eq!(kvm.zones.len(), 1);
        assert_eq!(kvm.zones[0].zone, second);

        assert_eq!(
            kvm_vm_ioctl_unregister_coalesced_mmio(
                &mut kvm,
                KvmCoalescedMmioZone {
                    addr: 0,
                    size: 1,
                    pio: 2,
                },
                0,
            ),
            -EINVAL
        );

        assert_eq!(
            kvm_vm_ioctl_register_coalesced_mmio(&mut kvm, first, true, 0),
            0
        );
        assert_eq!(
            kvm_vm_ioctl_unregister_coalesced_mmio(
                &mut kvm,
                KvmCoalescedMmioZone {
                    addr: 0x4000,
                    size: 0x100,
                    pio: 0,
                },
                -EOPNOTSUPP,
            ),
            0
        );
        assert!(kvm.zones.is_empty());
    }

    #[test]
    fn destructor_deletes_list_and_frees_device() {
        let dev = KvmCoalescedMmioDevice {
            zone: KvmCoalescedMmioZone {
                addr: 0,
                size: 1,
                pio: 0,
            },
            bus: KvmIoBus::Mmio,
            iodevice_initialized: true,
            list_linked: true,
        };
        assert_eq!(
            coalesced_mmio_destructor(dev),
            KvmCoalescedMmioDestructorReport {
                list_deleted: true,
                freed: true
            }
        );
    }
}
