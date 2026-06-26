//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/entry/common.c
//! test-origin: linux:vendor/linux/arch/x86/entry/common.c
//! Common x86 entry helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/entry/common.c
//! - vendor/linux/arch/x86/include/asm/trapnr.h
//! - vendor/linux/include/linux/hrtimer_rearm.h

pub const EVENT_TYPE_EXTINT: u32 = 0;
pub const EVENT_TYPE_NMI: u32 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmEntryConfig {
    pub kvm_intel_enabled: bool,
    pub x86_64: bool,
    pub fred_enabled: bool,
}

impl Default for KvmEntryConfig {
    fn default() -> Self {
        Self {
            kvm_intel_enabled: true,
            x86_64: true,
            fred_enabled: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KvmEntryDispatch {
    NotBuilt,
    Fred { event_type: u32, vector: u32 },
    IdtExtInt { vector: u32 },
    IdtNmiIrqoff,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmEntryAction {
    pub dispatch: KvmEntryDispatch,
    pub hrtimer_rearm_deferred: bool,
    pub warn_unexpected_event_type: bool,
}

pub const fn x86_entry_from_kvm(
    event_type: u32,
    vector: u32,
    config: KvmEntryConfig,
) -> KvmEntryAction {
    if !config.kvm_intel_enabled {
        return KvmEntryAction {
            dispatch: KvmEntryDispatch::NotBuilt,
            hrtimer_rearm_deferred: false,
            warn_unexpected_event_type: false,
        };
    }

    if event_type == EVENT_TYPE_EXTINT {
        return KvmEntryAction {
            dispatch: if config.x86_64 {
                KvmEntryDispatch::Fred { event_type, vector }
            } else {
                KvmEntryDispatch::IdtExtInt { vector }
            },
            hrtimer_rearm_deferred: true,
            warn_unexpected_event_type: false,
        };
    }

    let warn_unexpected_event_type = event_type != EVENT_TYPE_NMI;
    let dispatch = if config.x86_64 && config.fred_enabled {
        KvmEntryDispatch::Fred { event_type, vector }
    } else {
        KvmEntryDispatch::IdtNmiIrqoff
    };

    KvmEntryAction {
        dispatch,
        hrtimer_rearm_deferred: false,
        warn_unexpected_event_type,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_type_constants_match_trapnr_h() {
        assert_eq!(EVENT_TYPE_EXTINT, 0);
        assert_eq!(EVENT_TYPE_NMI, 2);
    }

    #[test]
    fn extint_uses_fred_dispatch_on_x86_64_and_rearms_timer() {
        let action = x86_entry_from_kvm(
            EVENT_TYPE_EXTINT,
            0x31,
            KvmEntryConfig {
                x86_64: true,
                fred_enabled: false,
                ..Default::default()
            },
        );

        assert_eq!(
            action,
            KvmEntryAction {
                dispatch: KvmEntryDispatch::Fred {
                    event_type: EVENT_TYPE_EXTINT,
                    vector: 0x31
                },
                hrtimer_rearm_deferred: true,
                warn_unexpected_event_type: false,
            }
        );
    }

    #[test]
    fn extint_uses_idt_dispatch_on_32_bit_builds() {
        let action = x86_entry_from_kvm(
            EVENT_TYPE_EXTINT,
            0x40,
            KvmEntryConfig {
                x86_64: false,
                ..Default::default()
            },
        );

        assert_eq!(
            action.dispatch,
            KvmEntryDispatch::IdtExtInt { vector: 0x40 }
        );
        assert!(action.hrtimer_rearm_deferred);
    }

    #[test]
    fn nmi_uses_fred_only_when_fred_feature_is_enabled() {
        let fred = x86_entry_from_kvm(
            EVENT_TYPE_NMI,
            2,
            KvmEntryConfig {
                fred_enabled: true,
                ..Default::default()
            },
        );
        let idt = x86_entry_from_kvm(EVENT_TYPE_NMI, 2, KvmEntryConfig::default());

        assert_eq!(
            fred.dispatch,
            KvmEntryDispatch::Fred {
                event_type: EVENT_TYPE_NMI,
                vector: 2
            }
        );
        assert_eq!(idt.dispatch, KvmEntryDispatch::IdtNmiIrqoff);
        assert!(!fred.hrtimer_rearm_deferred);
    }

    #[test]
    fn unexpected_non_extint_non_nmi_warns_but_follows_nmi_path() {
        let action = x86_entry_from_kvm(99, 2, KvmEntryConfig::default());

        assert!(action.warn_unexpected_event_type);
        assert_eq!(action.dispatch, KvmEntryDispatch::IdtNmiIrqoff);
    }

    #[test]
    fn disabled_kvm_intel_config_has_no_exported_action() {
        let action = x86_entry_from_kvm(
            EVENT_TYPE_NMI,
            2,
            KvmEntryConfig {
                kvm_intel_enabled: false,
                ..Default::default()
            },
        );

        assert_eq!(action.dispatch, KvmEntryDispatch::NotBuilt);
    }
}
