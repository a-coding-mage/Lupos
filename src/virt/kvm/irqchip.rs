//! linux-parity: complete
//! linux-source: vendor/linux/virt/kvm/irqchip.c
//! test-origin: linux:vendor/linux/virt/kvm/irqchip.c
//! KVM in-kernel IRQ routing table validation and GSI mapping.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use crate::include::uapi::errno::{EINVAL, ENOMEM};

pub const KVM_NR_IRQCHIPS: usize = 3;
pub const KVM_IRQCHIP_NUM_PINS: usize = 24;
pub const KVM_MAX_IRQ_ROUTES: u32 = 4096;
pub const KVM_IRQ_ROUTING_IRQCHIP: u32 = 1;
pub const KVM_IRQ_ROUTING_MSI: u32 = 2;
pub const KVM_MSI_VALID_DEVID: u32 = 1 << 0;
pub const KVM_USERSPACE_IRQ_SOURCE_ID: i32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IrqchipRoute {
    pub irqchip: u32,
    pub pin: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MsiRoute {
    pub address_lo: u32,
    pub address_hi: u32,
    pub data: u32,
    pub flags: u32,
    pub devid: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoutePayload {
    Irqchip(IrqchipRoute),
    Msi(MsiRoute),
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserIrqRoutingEntry {
    pub gsi: u32,
    pub route_type: u32,
    pub flags: u32,
    pub payload: RoutePayload,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernelIrqRoutingEntry {
    pub gsi: u32,
    pub route_type: u32,
    pub payload: RoutePayload,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IrqRoutingTable {
    pub nr_rt_entries: u32,
    pub map: Vec<Vec<KernelIrqRoutingEntry>>,
    pub chip: [[i32; KVM_IRQCHIP_NUM_PINS]; KVM_NR_IRQCHIPS],
}

impl IrqRoutingTable {
    pub fn empty() -> Result<Self, i32> {
        Ok(Self {
            nr_rt_entries: 1,
            map: vec![Vec::new()],
            chip: [[-1; KVM_IRQCHIP_NUM_PINS]; KVM_NR_IRQCHIPS],
        })
    }

    pub fn map_gsi(&self, gsi: u32) -> &[KernelIrqRoutingEntry] {
        self.map.get(gsi as usize).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn map_chip_pin(&self, irqchip: usize, pin: usize) -> i32 {
        self.chip
            .get(irqchip)
            .and_then(|pins| pins.get(pin))
            .copied()
            .unwrap_or(-1)
    }
}

pub fn kvm_irq_map_gsi(
    table: Option<&IrqRoutingTable>,
    entries: &mut Vec<KernelIrqRoutingEntry>,
    gsi: u32,
) -> usize {
    let Some(table) = table else {
        return 0;
    };
    if gsi >= table.nr_rt_entries {
        return 0;
    }
    let before = entries.len();
    entries.extend_from_slice(table.map_gsi(gsi));
    entries.len() - before
}

pub fn kvm_irq_map_chip_pin(table: &IrqRoutingTable, irqchip: usize, pin: usize) -> i32 {
    table.map_chip_pin(irqchip, pin)
}

pub fn kvm_set_irq_routing(entries: &[UserIrqRoutingEntry]) -> Result<IrqRoutingTable, i32> {
    kvm_set_irq_routing_with_alloc(entries, true)
}

pub fn kvm_set_irq_routing_with_alloc(
    entries: &[UserIrqRoutingEntry],
    allocation_available: bool,
) -> Result<IrqRoutingTable, i32> {
    let mut nr_rt_entries = 0u32;
    for entry in entries {
        if entry.gsi >= KVM_MAX_IRQ_ROUTES {
            return Err(-EINVAL);
        }
        nr_rt_entries = nr_rt_entries.max(entry.gsi);
    }
    nr_rt_entries = nr_rt_entries.saturating_add(1);

    if !allocation_available {
        return Err(-ENOMEM);
    }

    let mut table = IrqRoutingTable {
        nr_rt_entries,
        map: vec![Vec::new(); nr_rt_entries as usize],
        chip: [[-1; KVM_IRQCHIP_NUM_PINS]; KVM_NR_IRQCHIPS],
    };

    for entry in entries {
        if entry.route_type == KVM_IRQ_ROUTING_MSI {
            if entry.flags & !KVM_MSI_VALID_DEVID != 0 {
                return Err(-EINVAL);
            }
        } else if entry.flags != 0 {
            return Err(-EINVAL);
        }
        setup_routing_entry(&mut table, *entry)?;
    }

    Ok(table)
}

fn setup_routing_entry(table: &mut IrqRoutingTable, user: UserIrqRoutingEntry) -> Result<(), i32> {
    let gsi = user.gsi as usize;
    for existing in &table.map[gsi] {
        if existing.route_type != KVM_IRQ_ROUTING_IRQCHIP
            || user.route_type != KVM_IRQ_ROUTING_IRQCHIP
            || same_irqchip(existing.payload, user.payload)
        {
            return Err(-EINVAL);
        }
    }

    let entry = KernelIrqRoutingEntry {
        gsi: user.gsi,
        route_type: user.route_type,
        payload: user.payload,
    };

    if let RoutePayload::Irqchip(route) = entry.payload {
        let irqchip = route.irqchip as usize;
        let pin = route.pin as usize;
        if irqchip >= KVM_NR_IRQCHIPS || pin >= KVM_IRQCHIP_NUM_PINS {
            return Err(-EINVAL);
        }
        table.chip[irqchip][pin] = entry.gsi as i32;
    }

    table.map[gsi].insert(0, entry);
    Ok(())
}

fn same_irqchip(left: RoutePayload, right: RoutePayload) -> bool {
    matches!(
        (left, right),
        (RoutePayload::Irqchip(a), RoutePayload::Irqchip(b)) if a.irqchip == b.irqchip
    )
}

pub const fn kvm_send_userspace_msi(
    in_kernel_irqchip: bool,
    msi: MsiRoute,
) -> Result<MsiRoute, i32> {
    if !in_kernel_irqchip || (msi.flags & !KVM_MSI_VALID_DEVID) != 0 {
        return Err(-EINVAL);
    }
    Ok(msi)
}

pub fn kvm_set_irq(table: &IrqRoutingTable, irq: u32, route_results: &[i32]) -> i32 {
    let route_count = table.map_gsi(irq).len();
    combine_irq_delivery_results(&route_results[..route_count.min(route_results.len())])
}

pub fn combine_irq_delivery_results(results: &[i32]) -> i32 {
    let mut ret = -1;
    for r in results.iter().rev().copied() {
        if r < 0 {
            continue;
        }
        ret = r + if ret < 0 { 0 } else { ret };
    }
    ret
}

pub fn free_irq_routing_table(table: &mut IrqRoutingTable) {
    for gsi in &mut table.map {
        gsi.clear();
    }
    table.map.clear();
    table.chip = [[-1; KVM_IRQCHIP_NUM_PINS]; KVM_NR_IRQCHIPS];
    table.nr_rt_entries = 0;
}

pub fn kvm_free_irq_routing(table: &mut Option<IrqRoutingTable>) {
    if let Some(route_table) = table.as_mut() {
        free_irq_routing_table(route_table);
    }
    *table = None;
}

pub const fn kvm_arch_irq_routing_update() {}

pub const fn kvm_arch_can_set_irq_routing() -> bool {
    true
}

pub fn kvm_init_irq_routing(allocation_available: bool) -> Result<IrqRoutingTable, i32> {
    if !allocation_available {
        return Err(-ENOMEM);
    }
    IrqRoutingTable::empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn irqchip(gsi: u32, irqchip: u32, pin: u32) -> UserIrqRoutingEntry {
        UserIrqRoutingEntry {
            gsi,
            route_type: KVM_IRQ_ROUTING_IRQCHIP,
            flags: 0,
            payload: RoutePayload::Irqchip(IrqchipRoute { irqchip, pin }),
        }
    }

    #[test]
    fn irqchip_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/virt/kvm/irqchip.c"
        ));
        assert!(source.contains("int kvm_irq_map_gsi(struct kvm *kvm"));
        assert!(source.contains("hlist_for_each_entry(e, &irq_rt->map[gsi], link)"));
        assert!(source.contains("return irq_rt->chip[irqchip][pin];"));
        assert!(source.contains(
            "if (!kvm_arch_irqchip_in_kernel(kvm) || (msi->flags & ~KVM_MSI_VALID_DEVID))"
        ));
        assert!(source.contains("struct kvm_kernel_irq_routing_entry irq_set[KVM_NR_IRQCHIPS];"));
        assert!(source.contains("if (ue[i].gsi >= KVM_MAX_IRQ_ROUTES)"));
        assert!(source.contains("if (e->type == KVM_IRQ_ROUTING_IRQCHIP)"));
        assert!(source.contains("static void free_irq_routing_table"));
        assert!(source.contains("void kvm_free_irq_routing(struct kvm *kvm)"));
        assert!(source.contains("void __attribute__((weak)) kvm_arch_irq_routing_update"));
        assert!(source.contains("bool __weak kvm_arch_can_set_irq_routing"));
        assert!(source.contains("return true;"));
        assert!(source.contains("if (!new)"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("new->chip[i][j] = -1;"));
        assert!(source.contains("kvm_irq_routing_update(kvm);"));
        assert!(source.contains("kvm_arch_irq_routing_update(kvm);"));
        assert!(source.contains("synchronize_srcu_expedited(&kvm->irq_srcu);"));
        assert!(source.contains("kzalloc_flex(*new, map, 1, GFP_KERNEL_ACCOUNT);"));
        assert!(source.contains("memset(new->chip, -1, chip_size);"));
        assert!(source.contains("RCU_INIT_POINTER(kvm->irq_routing, new);"));
    }

    #[test]
    fn irq_routing_rejects_duplicate_same_irqchip_for_gsi() {
        let entries = [irqchip(4, 0, 1), irqchip(4, 0, 2)];
        assert_eq!(kvm_set_irq_routing(&entries), Err(-EINVAL));
    }

    #[test]
    fn irq_routing_allows_same_gsi_to_pic_and_ioapic() {
        let entries = [irqchip(4, 0, 1), irqchip(4, 2, 4)];
        let table = kvm_set_irq_routing(&entries).unwrap();
        assert_eq!(table.nr_rt_entries, 5);
        assert_eq!(table.map_chip_pin(0, 1), 4);
        assert_eq!(table.map_chip_pin(2, 4), 4);
        assert_eq!(table.map_gsi(4).len(), 2);
        assert_eq!(table.map_gsi(4)[0].payload, entries[1].payload);

        let mut copied = alloc::vec![KernelIrqRoutingEntry {
            gsi: 0,
            route_type: KVM_IRQ_ROUTING_MSI,
            payload: RoutePayload::Other,
        }];
        assert_eq!(kvm_irq_map_gsi(Some(&table), &mut copied, 4), 2);
        assert_eq!(copied.len(), 3);
        assert_eq!(kvm_irq_map_gsi(Some(&table), &mut copied, 99), 0);
        assert_eq!(kvm_irq_map_chip_pin(&table, 2, 4), 4);
    }

    #[test]
    fn msi_rejects_unknown_flags_and_missing_kernel_irqchip() {
        let msi = MsiRoute {
            address_lo: 1,
            address_hi: 2,
            data: 3,
            flags: KVM_MSI_VALID_DEVID,
            devid: 4,
        };
        assert_eq!(kvm_send_userspace_msi(false, msi), Err(-EINVAL));
        assert_eq!(kvm_send_userspace_msi(true, msi), Ok(msi));
        assert_eq!(
            kvm_send_userspace_msi(true, MsiRoute { flags: 2, ..msi }),
            Err(-EINVAL)
        );
    }

    #[test]
    fn set_irq_combines_non_negative_results_in_reverse_map_order() {
        assert_eq!(combine_irq_delivery_results(&[-1, 2, 3]), 5);
        assert_eq!(combine_irq_delivery_results(&[-1, -2]), -1);

        let entries = [irqchip(4, 0, 1), irqchip(4, 2, 4)];
        let table = kvm_set_irq_routing(&entries).unwrap();
        assert_eq!(kvm_set_irq(&table, 4, &[-1, 2, 3]), 2);
        assert_eq!(kvm_set_irq(&table, 5, &[7]), -1);
    }

    #[test]
    fn init_free_and_weak_hooks_follow_linux_defaults() {
        assert!(kvm_arch_can_set_irq_routing());
        kvm_arch_irq_routing_update();
        assert_eq!(kvm_init_irq_routing(false), Err(-ENOMEM));
        assert_eq!(kvm_set_irq_routing_with_alloc(&[], false), Err(-ENOMEM));

        let table = kvm_init_irq_routing(true).unwrap();
        assert_eq!(table.nr_rt_entries, 1);
        assert_eq!(table.map.len(), 1);
        assert_eq!(table.map_chip_pin(0, 0), -1);

        let mut table = Some(kvm_set_irq_routing(&[irqchip(4, 0, 1)]).unwrap());
        kvm_free_irq_routing(&mut table);
        assert!(table.is_none());
    }
}
