//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/hyperv/hv_spinlock.c
//! test-origin: linux:vendor/linux/arch/x86/hyperv/hv_spinlock.c
//! Hyper-V enlightened spinlock model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/hyperv/hv_spinlock.c

pub const X86_PLATFORM_IPI_VECTOR: u8 = 0xf7;
pub const HV_X64_MSR_GUEST_IDLE: u32 = 0x4000_00f0;
pub const HV_MSR_GUEST_IDLE_AVAILABLE: u64 = 1 << 10;
pub const HV_X64_CLUSTER_IPI_RECOMMENDED: u64 = 1 << 10;

pub const QUEUED_SPIN_LOCK_SLOWPATH: &str = "__pv_queued_spin_lock_slowpath";
pub const QUEUED_SPIN_UNLOCK: &str = "__pv_queued_spin_unlock";
pub const WAIT_OP: &str = "hv_qlock_wait";
pub const KICK_OP: &str = "hv_qlock_kick";
pub const VCPU_IS_PREEMPTED_OP: &str = "hv_vcpu_is_preempted";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HvSpinlockConfig {
    pub hv_pvspin: bool,
    pub apic_present: bool,
    pub hyperv_hints: u64,
    pub hyperv_features: u64,
}

impl Default for HvSpinlockConfig {
    fn default() -> Self {
        Self {
            hv_pvspin: true,
            apic_present: false,
            hyperv_hints: 0,
            hyperv_features: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HvKick {
    pub cpu: i32,
    pub vector: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HvQlockWait {
    pub irq_saved: bool,
    pub read_guest_idle_msr: bool,
    pub msr: Option<u32>,
    pub irq_restored: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HvPvLockOps {
    pub lock_hash_initialized: bool,
    pub queued_spin_lock_slowpath: &'static str,
    pub queued_spin_unlock: &'static str,
    pub wait: &'static str,
    pub kick: &'static str,
    pub vcpu_is_preempted: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HvSpinlockInit {
    pub enabled: bool,
    pub disabled_message: Option<&'static str>,
    pub ops: Option<HvPvLockOps>,
}

pub const fn hv_qlock_kick(cpu: i32) -> HvKick {
    HvKick {
        cpu,
        vector: X86_PLATFORM_IPI_VECTOR,
    }
}

pub const fn hv_qlock_wait(lock_byte: u8, val: u8, in_nmi: bool) -> HvQlockWait {
    if in_nmi {
        return HvQlockWait {
            irq_saved: false,
            read_guest_idle_msr: false,
            msr: None,
            irq_restored: false,
        };
    }

    let unchanged = lock_byte == val;
    HvQlockWait {
        irq_saved: true,
        read_guest_idle_msr: unchanged,
        msr: if unchanged {
            Some(HV_X64_MSR_GUEST_IDLE)
        } else {
            None
        },
        irq_restored: true,
    }
}

pub const fn hv_vcpu_is_preempted(_vcpu: i32) -> bool {
    false
}

pub const fn hv_parse_nopvspin(config: &mut HvSpinlockConfig) -> i32 {
    config.hv_pvspin = false;
    0
}

pub const fn hv_init_spinlocks(config: HvSpinlockConfig) -> HvSpinlockInit {
    if !config.hv_pvspin
        || !config.apic_present
        || (config.hyperv_hints & HV_X64_CLUSTER_IPI_RECOMMENDED) == 0
        || (config.hyperv_features & HV_MSR_GUEST_IDLE_AVAILABLE) == 0
    {
        return HvSpinlockInit {
            enabled: false,
            disabled_message: Some("PV spinlocks disabled"),
            ops: None,
        };
    }

    HvSpinlockInit {
        enabled: true,
        disabled_message: None,
        ops: Some(HvPvLockOps {
            lock_hash_initialized: true,
            queued_spin_lock_slowpath: QUEUED_SPIN_LOCK_SLOWPATH,
            queued_spin_unlock: QUEUED_SPIN_UNLOCK,
            wait: WAIT_OP,
            kick: KICK_OP,
            vcpu_is_preempted: VCPU_IS_PREEMPTED_OP,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hv_spinlock_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/hyperv/hv_spinlock.c"
        ));
        let irq_vectors = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/asm/irq_vectors.h"
        ));
        let hvgdk = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/hyperv/hvgdk_mini.h"
        ));
        assert!(source.contains("static bool hv_pvspin __initdata = true;"));
        assert!(source.contains("__apic_send_IPI(cpu, X86_PLATFORM_IPI_VECTOR);"));
        assert!(source.contains("if (in_nmi())"));
        assert!(source.contains("local_irq_save(flags);"));
        assert!(source.contains("if (READ_ONCE(*byte) == val)"));
        assert!(source.contains("rdmsrq(HV_X64_MSR_GUEST_IDLE, msr_val);"));
        assert!(source.contains("local_irq_restore(flags);"));
        assert!(source.contains("return false;"));
        assert!(source.contains("__pv_init_lock_hash();"));
        assert!(source.contains("pv_ops_lock.wait = hv_qlock_wait;"));
        assert!(source.contains("pv_ops_lock.kick = hv_qlock_kick;"));
        assert!(source.contains("early_param(\"hv_nopvspin\", hv_parse_nopvspin);"));
        assert!(irq_vectors.contains("#define X86_PLATFORM_IPI_VECTOR\t\t0xf7"));
        assert!(hvgdk.contains("#define HV_X64_MSR_GUEST_IDLE\t\t\t0x400000F0"));
        assert!(hvgdk.contains("#define HV_MSR_GUEST_IDLE_AVAILABLE"));
        assert!(hvgdk.contains("#define HV_X64_CLUSTER_IPI_RECOMMENDED"));

        assert_eq!(X86_PLATFORM_IPI_VECTOR, 0xf7);
        assert_eq!(HV_X64_MSR_GUEST_IDLE, 0x4000_00f0);
    }

    #[test]
    fn qlock_kick_wait_and_preempted_match_linux_edges() {
        assert_eq!(
            hv_qlock_kick(7),
            HvKick {
                cpu: 7,
                vector: X86_PLATFORM_IPI_VECTOR,
            }
        );
        assert_eq!(
            hv_qlock_wait(3, 3, false),
            HvQlockWait {
                irq_saved: true,
                read_guest_idle_msr: true,
                msr: Some(HV_X64_MSR_GUEST_IDLE),
                irq_restored: true,
            }
        );
        assert_eq!(
            hv_qlock_wait(2, 3, false),
            HvQlockWait {
                irq_saved: true,
                read_guest_idle_msr: false,
                msr: None,
                irq_restored: true,
            }
        );
        assert_eq!(
            hv_qlock_wait(3, 3, true),
            HvQlockWait {
                irq_saved: false,
                read_guest_idle_msr: false,
                msr: None,
                irq_restored: false,
            }
        );
        assert!(!hv_vcpu_is_preempted(0));
    }

    #[test]
    fn init_spinlocks_gates_and_installs_pv_ops() {
        let disabled = hv_init_spinlocks(HvSpinlockConfig::default());
        assert_eq!(
            disabled,
            HvSpinlockInit {
                enabled: false,
                disabled_message: Some("PV spinlocks disabled"),
                ops: None,
            }
        );

        let mut config = HvSpinlockConfig {
            hv_pvspin: true,
            apic_present: true,
            hyperv_hints: HV_X64_CLUSTER_IPI_RECOMMENDED,
            hyperv_features: HV_MSR_GUEST_IDLE_AVAILABLE,
        };
        let enabled = hv_init_spinlocks(config);
        assert!(enabled.enabled);
        let ops = enabled.ops.unwrap();
        assert!(ops.lock_hash_initialized);
        assert_eq!(ops.wait, "hv_qlock_wait");
        assert_eq!(ops.kick, "hv_qlock_kick");
        assert_eq!(ops.vcpu_is_preempted, "hv_vcpu_is_preempted");

        assert_eq!(hv_parse_nopvspin(&mut config), 0);
        assert!(!hv_init_spinlocks(config).enabled);
    }
}
