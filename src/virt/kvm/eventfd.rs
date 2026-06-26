//! linux-parity: partial
//! linux-source: vendor/linux/virt/kvm/eventfd.c
//! test-origin: linux:vendor/linux/virt/kvm/eventfd.c
//! KVM irqfd and ioeventfd registration, collision, and match semantics.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::{EAGAIN, EBADF, EBUSY, EEXIST, EINVAL, ENOENT, EOPNOTSUPP};

pub const KVM_IRQFD_FLAG_DEASSIGN: u32 = 1 << 0;
pub const KVM_IRQFD_FLAG_RESAMPLE: u32 = 1 << 1;
pub const KVM_IRQFD_VALID_FLAG_MASK: u32 = KVM_IRQFD_FLAG_DEASSIGN | KVM_IRQFD_FLAG_RESAMPLE;

pub const KVM_IOEVENTFD_FLAG_DATAMATCH: u32 = 1 << 0;
pub const KVM_IOEVENTFD_FLAG_PIO: u32 = 1 << 1;
pub const KVM_IOEVENTFD_FLAG_DEASSIGN: u32 = 1 << 2;
pub const KVM_IOEVENTFD_FLAG_VIRTIO_CCW_NOTIFY: u32 = 1 << 3;
pub const KVM_IOEVENTFD_FLAG_FAST_MMIO: u32 = 1 << 4;
pub const KVM_IOEVENTFD_VALID_FLAG_MASK: u32 = (1 << 5) - 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmIrqfd {
    pub fd: i32,
    pub gsi: u32,
    pub flags: u32,
    pub resamplefd: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IrqfdAssignment {
    pub vm_id: u64,
    pub fd: i32,
    pub gsi: u32,
    pub resamplefd: Option<i32>,
    pub injections: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IrqfdSignal {
    Pulse { vm_id: u64, gsi: u32 },
    ResampleAssert { vm_id: u64, gsi: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IrqfdResampler {
    pub vm_id: u64,
    pub gsi: u32,
    pub users: u32,
    pub notifications: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KvmIrqfdRegistry {
    assignments: Vec<IrqfdAssignment>,
    resamplers: Vec<IrqfdResampler>,
}

impl KvmIrqfdRegistry {
    pub const fn new() -> Self {
        Self {
            assignments: Vec::new(),
            resamplers: Vec::new(),
        }
    }

    pub fn assignments(&self) -> &[IrqfdAssignment] {
        &self.assignments
    }

    pub fn resamplers(&self) -> &[IrqfdResampler] {
        &self.resamplers
    }

    pub fn irqfd(
        &mut self,
        vm_id: u64,
        args: KvmIrqfd,
        fd_open: bool,
        resamplefd_open: bool,
        intc_initialized: bool,
        arch_allowed: bool,
    ) -> Result<(), i32> {
        if args.flags & !KVM_IRQFD_VALID_FLAG_MASK != 0 {
            return Err(-EINVAL);
        }

        if args.flags & KVM_IRQFD_FLAG_DEASSIGN != 0 {
            return self.deassign(vm_id, args.fd, args.gsi, fd_open);
        }

        self.assign(
            vm_id,
            args,
            fd_open,
            resamplefd_open,
            intc_initialized,
            arch_allowed,
        )
    }

    fn assign(
        &mut self,
        vm_id: u64,
        args: KvmIrqfd,
        fd_open: bool,
        resamplefd_open: bool,
        intc_initialized: bool,
        arch_allowed: bool,
    ) -> Result<(), i32> {
        if !intc_initialized {
            return Err(-EAGAIN);
        }
        if !arch_allowed {
            return Err(-EINVAL);
        }
        if !fd_open {
            return Err(-EBADF);
        }
        if self.assignments.iter().any(|irqfd| irqfd.fd == args.fd) {
            return Err(-EBUSY);
        }

        let resamplefd = if args.flags & KVM_IRQFD_FLAG_RESAMPLE != 0 {
            if !resamplefd_open {
                return Err(-EBADF);
            }
            self.get_or_create_resampler(vm_id, args.gsi)?;
            Some(args.resamplefd)
        } else {
            None
        };

        self.assignments.push(IrqfdAssignment {
            vm_id,
            fd: args.fd,
            gsi: args.gsi,
            resamplefd,
            injections: 0,
        });
        Ok(())
    }

    fn get_or_create_resampler(&mut self, vm_id: u64, gsi: u32) -> Result<(), i32> {
        if let Some(resampler) = self
            .resamplers
            .iter_mut()
            .find(|resampler| resampler.vm_id == vm_id && resampler.gsi == gsi)
        {
            resampler.users = resampler.users.saturating_add(1);
            return Ok(());
        }

        self.resamplers.push(IrqfdResampler {
            vm_id,
            gsi,
            users: 1,
            notifications: 0,
        });
        Ok(())
    }

    fn deassign(&mut self, vm_id: u64, fd: i32, gsi: u32, fd_open: bool) -> Result<(), i32> {
        if !fd_open {
            return Err(-EBADF);
        }

        let mut index = 0;
        while index < self.assignments.len() {
            let irqfd = self.assignments[index];
            if irqfd.vm_id == vm_id && irqfd.fd == fd && irqfd.gsi == gsi {
                if irqfd.resamplefd.is_some() {
                    self.drop_resampler(vm_id, gsi);
                }
                self.assignments.remove(index);
            } else {
                index += 1;
            }
        }

        Ok(())
    }

    fn drop_resampler(&mut self, vm_id: u64, gsi: u32) {
        let Some(index) = self
            .resamplers
            .iter()
            .position(|resampler| resampler.vm_id == vm_id && resampler.gsi == gsi)
        else {
            return;
        };

        if self.resamplers[index].users > 1 {
            self.resamplers[index].users -= 1;
        } else {
            self.resamplers.remove(index);
        }
    }

    pub fn signal_eventfd(&mut self, fd: i32) -> Result<IrqfdSignal, i32> {
        let Some(irqfd) = self.assignments.iter_mut().find(|irqfd| irqfd.fd == fd) else {
            return Err(-ENOENT);
        };

        irqfd.injections = irqfd.injections.saturating_add(1);
        if irqfd.resamplefd.is_some() {
            Ok(IrqfdSignal::ResampleAssert {
                vm_id: irqfd.vm_id,
                gsi: irqfd.gsi,
            })
        } else {
            Ok(IrqfdSignal::Pulse {
                vm_id: irqfd.vm_id,
                gsi: irqfd.gsi,
            })
        }
    }

    pub fn notify_irqfd_resampler(&mut self, vm_id: u64, gsi: u32) -> bool {
        let Some(resampler) = self
            .resamplers
            .iter_mut()
            .find(|resampler| resampler.vm_id == vm_id && resampler.gsi == gsi)
        else {
            return false;
        };

        resampler.notifications = resampler
            .notifications
            .saturating_add(resampler.users as u64);
        true
    }

    pub fn release_vm(&mut self, vm_id: u64) {
        self.assignments.retain(|irqfd| irqfd.vm_id != vm_id);
        self.resamplers.retain(|resampler| resampler.vm_id != vm_id);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmIoeventfd {
    pub datamatch: u64,
    pub addr: u64,
    pub len: u32,
    pub fd: i32,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KvmBus {
    Mmio,
    Pio,
    VirtioCcwNotify,
    FastMmio,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IoeventfdEntry {
    pub fd: i32,
    pub addr: u64,
    pub len: u32,
    pub datamatch: u64,
    pub bus: KvmBus,
    pub wildcard: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KvmIoeventfds {
    entries: Vec<IoeventfdEntry>,
}

impl KvmIoeventfds {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn entries(&self) -> &[IoeventfdEntry] {
        &self.entries
    }

    pub fn ioeventfd(&mut self, args: KvmIoeventfd, fd_open: bool) -> Result<(), i32> {
        if args.flags & KVM_IOEVENTFD_FLAG_DEASSIGN != 0 {
            return self.deassign(args, fd_open);
        }

        self.assign(args, fd_open)
    }

    fn assign(&mut self, args: KvmIoeventfd, fd_open: bool) -> Result<(), i32> {
        if !fd_open {
            return Err(-EBADF);
        }
        if !matches!(args.len, 0 | 1 | 2 | 4 | 8) {
            return Err(-EINVAL);
        }
        if args.addr.checked_add(args.len as u64).is_none() {
            return Err(-EINVAL);
        }
        if args.flags & !KVM_IOEVENTFD_VALID_FLAG_MASK != 0 {
            return Err(-EINVAL);
        }
        if args.len == 0 && args.flags & KVM_IOEVENTFD_FLAG_DATAMATCH != 0 {
            return Err(-EINVAL);
        }

        let bus = ioeventfd_bus_from_flags(args.flags);
        self.assign_idx(bus, args)?;

        if args.len == 0 && bus == KvmBus::Mmio {
            if let Err(err) = self.assign_idx(KvmBus::FastMmio, args) {
                let _ = self.deassign_idx(bus, args);
                return Err(err);
            }
        }

        Ok(())
    }

    fn assign_idx(&mut self, bus: KvmBus, args: KvmIoeventfd) -> Result<(), i32> {
        let entry = IoeventfdEntry {
            fd: args.fd,
            addr: args.addr,
            len: args.len,
            datamatch: args.datamatch,
            bus,
            wildcard: args.flags & KVM_IOEVENTFD_FLAG_DATAMATCH == 0,
        };

        if self.check_collision(entry) {
            return Err(-EEXIST);
        }

        self.entries.push(entry);
        Ok(())
    }

    fn deassign(&mut self, args: KvmIoeventfd, fd_open: bool) -> Result<(), i32> {
        if !fd_open {
            return Err(-EBADF);
        }

        let bus = ioeventfd_bus_from_flags(args.flags);
        let ret = self.deassign_idx(bus, args);
        if args.len == 0 && bus == KvmBus::Mmio {
            let _ = self.deassign_idx(KvmBus::FastMmio, args);
        }
        ret
    }

    fn deassign_idx(&mut self, bus: KvmBus, args: KvmIoeventfd) -> Result<(), i32> {
        let wildcard = args.flags & KVM_IOEVENTFD_FLAG_DATAMATCH == 0;
        let Some(index) = self.entries.iter().position(|entry| {
            entry.bus == bus
                && entry.fd == args.fd
                && entry.addr == args.addr
                && entry.len == args.len
                && entry.wildcard == wildcard
                && (entry.wildcard || entry.datamatch == args.datamatch)
        }) else {
            return Err(-ENOENT);
        };

        self.entries.remove(index);
        Ok(())
    }

    fn check_collision(&self, entry: IoeventfdEntry) -> bool {
        self.entries.iter().any(|existing| {
            existing.bus == entry.bus
                && existing.addr == entry.addr
                && (existing.len == 0
                    || entry.len == 0
                    || (existing.len == entry.len
                        && (existing.wildcard
                            || entry.wildcard
                            || existing.datamatch == entry.datamatch)))
        })
    }

    pub fn write(&self, bus: KvmBus, addr: u64, value: &[u8]) -> Result<i32, i32> {
        for entry in &self.entries {
            if entry.bus == bus && ioeventfd_in_range(*entry, addr, value) {
                return Ok(entry.fd);
            }
        }
        Err(-EOPNOTSUPP)
    }
}

pub const fn ioeventfd_bus_from_flags(flags: u32) -> KvmBus {
    if flags & KVM_IOEVENTFD_FLAG_PIO != 0 {
        KvmBus::Pio
    } else if flags & KVM_IOEVENTFD_FLAG_VIRTIO_CCW_NOTIFY != 0 {
        KvmBus::VirtioCcwNotify
    } else {
        KvmBus::Mmio
    }
}

pub fn ioeventfd_in_range(entry: IoeventfdEntry, addr: u64, value: &[u8]) -> bool {
    if addr != entry.addr {
        return false;
    }
    if entry.len == 0 {
        return true;
    }
    if value.len() != entry.len as usize {
        return false;
    }
    if entry.wildcard {
        return true;
    }

    read_le_value(value).is_some_and(|value| value == entry.datamatch)
}

fn read_le_value(value: &[u8]) -> Option<u64> {
    match value.len() {
        1 => Some(value[0] as u64),
        2 => Some(u16::from_le_bytes([value[0], value[1]]) as u64),
        4 => Some(u32::from_le_bytes([value[0], value[1], value[2], value[3]]) as u64),
        8 => Some(u64::from_le_bytes([
            value[0], value[1], value[2], value[3], value[4], value[5], value[6], value[7],
        ])),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eventfd_source_and_irqfd_selftest_contract_match_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/virt/kvm/eventfd.c"
        ));
        assert!(
            source.contains(
                "if (args->flags & ~(KVM_IRQFD_FLAG_DEASSIGN | KVM_IRQFD_FLAG_RESAMPLE))"
            )
        );
        assert!(source.contains("return kvm_irqfd_deassign(kvm, args);"));
        assert!(source.contains("return kvm_irqfd_assign(kvm, args);"));
        assert!(source.contains("if (p->ret)"));
        assert!(source.contains("ret = -EEXIST;"));
        assert!(source.contains("if (!args->len && (args->flags & KVM_IOEVENTFD_FLAG_DATAMATCH))"));
        assert!(source.contains("if (!args->len && bus_idx == KVM_MMIO_BUS)"));

        let irqfd_test = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/kvm/irqfd_test.c"
        ));
        assert!(irqfd_test.contains("KVM de-assigns based on eventfd *and* GSI"));
        assert!(irqfd_test.contains("KVM disallows assigning a"));
        assert!(irqfd_test.contains("single eventfd to multiple GSIs"));
        assert!(irqfd_test.contains("errno == EBUSY"));
        assert!(irqfd_test.contains("KVM_IRQFD_FLAG_DEASSIGN"));
    }

    #[test]
    fn irqfd_rejects_duplicate_eventfd_across_vms_but_deassign_is_lenient() {
        let mut registry = KvmIrqfdRegistry::new();
        let args = KvmIrqfd {
            fd: 7,
            gsi: 10,
            flags: 0,
            resamplefd: 0,
        };

        registry.irqfd(1, args, true, true, true, true).unwrap();
        assert_eq!(
            registry.irqfd(1, KvmIrqfd { gsi: 11, ..args }, true, true, true, true),
            Err(-EBUSY)
        );
        assert_eq!(
            registry.irqfd(2, KvmIrqfd { gsi: 12, ..args }, true, true, true, true),
            Err(-EBUSY)
        );

        let missing_deassign = KvmIrqfd {
            flags: KVM_IRQFD_FLAG_DEASSIGN,
            gsi: 99,
            ..args
        };
        assert_eq!(
            registry.irqfd(1, missing_deassign, true, true, true, true),
            Ok(())
        );
        assert_eq!(registry.assignments().len(), 1);

        let deassign = KvmIrqfd {
            flags: KVM_IRQFD_FLAG_DEASSIGN,
            ..args
        };
        registry.irqfd(1, deassign, true, true, true, true).unwrap();
        assert!(registry.assignments().is_empty());
    }

    #[test]
    fn irqfd_resampler_is_shared_by_gsi_and_ack_notifies_all_users() {
        let mut registry = KvmIrqfdRegistry::new();
        registry
            .irqfd(
                1,
                KvmIrqfd {
                    fd: 10,
                    gsi: 32,
                    flags: KVM_IRQFD_FLAG_RESAMPLE,
                    resamplefd: 20,
                },
                true,
                true,
                true,
                true,
            )
            .unwrap();
        registry
            .irqfd(
                1,
                KvmIrqfd {
                    fd: 11,
                    gsi: 32,
                    flags: KVM_IRQFD_FLAG_RESAMPLE,
                    resamplefd: 21,
                },
                true,
                true,
                true,
                true,
            )
            .unwrap();

        assert_eq!(registry.resamplers()[0].users, 2);
        assert_eq!(
            registry.signal_eventfd(10),
            Ok(IrqfdSignal::ResampleAssert { vm_id: 1, gsi: 32 })
        );
        assert!(registry.notify_irqfd_resampler(1, 32));
        assert_eq!(registry.resamplers()[0].notifications, 2);
    }

    #[test]
    fn ioeventfd_validates_length_flags_and_collision_rules() {
        let mut ioevents = KvmIoeventfds::new();
        let base = KvmIoeventfd {
            datamatch: 0xaa55,
            addr: 0xfee0,
            len: 2,
            fd: 3,
            flags: KVM_IOEVENTFD_FLAG_DATAMATCH,
        };

        ioevents.ioeventfd(base, true).unwrap();
        assert_eq!(
            ioevents.ioeventfd(KvmIoeventfd { fd: 4, ..base }, true),
            Err(-EEXIST)
        );
        assert_eq!(
            ioevents.ioeventfd(
                KvmIoeventfd {
                    len: 3,
                    fd: 5,
                    ..base
                },
                true
            ),
            Err(-EINVAL)
        );
        assert_eq!(
            ioevents.ioeventfd(
                KvmIoeventfd {
                    len: 0,
                    fd: 5,
                    ..base
                },
                true
            ),
            Err(-EINVAL)
        );

        assert_eq!(
            ioevents.write(KvmBus::Mmio, 0xfee0, &0xaa55u16.to_le_bytes()),
            Ok(3)
        );
        assert_eq!(
            ioevents.write(KvmBus::Mmio, 0xfee0, &0xaa56u16.to_le_bytes()),
            Err(-EOPNOTSUPP)
        );
    }

    #[test]
    fn zero_length_mmio_registers_fast_bus_shadow_and_deassigns_both() {
        let mut ioevents = KvmIoeventfds::new();
        let args = KvmIoeventfd {
            datamatch: 0,
            addr: 0x1000,
            len: 0,
            fd: 8,
            flags: 0,
        };

        ioevents.ioeventfd(args, true).unwrap();
        assert_eq!(ioevents.entries().len(), 2);
        assert_eq!(ioevents.write(KvmBus::Mmio, 0x1000, &[1, 2, 3]), Ok(8));
        assert_eq!(ioevents.write(KvmBus::FastMmio, 0x1000, &[]), Ok(8));

        ioevents
            .ioeventfd(
                KvmIoeventfd {
                    flags: KVM_IOEVENTFD_FLAG_DEASSIGN,
                    ..args
                },
                true,
            )
            .unwrap();
        assert!(ioevents.entries().is_empty());
    }
}
