// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/linux/clocksource_ids.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S013602

/// Unique identifier for a clocksource.
///
/// The discriminants follow the Linux `enum clocksource_ids` declaration and
/// are stored in Linux clocksource and timekeeping structures.
#[repr(C)]
pub enum clocksource_ids {
    CSID_GENERIC = 0,
    CSID_ARM_ARCH_COUNTER,
    CSID_S390_TOD,
    CSID_X86_TSC_EARLY,
    CSID_X86_TSC,
    CSID_X86_KVM_CLK,
    CSID_X86_ART,
    CSID_MAX,
}
