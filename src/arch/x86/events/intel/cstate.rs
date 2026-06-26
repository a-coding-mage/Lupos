//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/events/intel/cstate.c
//! test-origin: linux:vendor/linux/arch/x86/events/intel/cstate.c
//! Intel C-state residency PMU model.

use crate::include::uapi::errno::{EINVAL, ENODEV, ENOENT};

pub const PERF_EF_START: i32 = 0x01;
pub const PERF_EF_UPDATE: i32 = 0x04;
pub const PERF_PMU_CAP_NO_INTERRUPT: u64 = 0x0001;
pub const PERF_PMU_CAP_NO_EXCLUDE: u64 = 0x0040;

pub const MSR_PKG_C3_RESIDENCY: u64 = 0x0000_03f8;
pub const MSR_PKG_C6_RESIDENCY: u64 = 0x0000_03f9;
pub const MSR_PKG_C7_RESIDENCY: u64 = 0x0000_03fa;
pub const MSR_CORE_C3_RESIDENCY: u64 = 0x0000_03fc;
pub const MSR_CORE_C6_RESIDENCY: u64 = 0x0000_03fd;
pub const MSR_CORE_C7_RESIDENCY: u64 = 0x0000_03fe;
pub const MSR_KNL_CORE_C6_RESIDENCY: u64 = 0x0000_03ff;
pub const MSR_PKG_C2_RESIDENCY: u64 = 0x0000_060d;
pub const MSR_PKG_C8_RESIDENCY: u64 = 0x0000_0630;
pub const MSR_PKG_C9_RESIDENCY: u64 = 0x0000_0631;
pub const MSR_PKG_C10_RESIDENCY: u64 = 0x0000_0632;
pub const MSR_CORE_C1_RES: u64 = 0x0000_0660;
pub const MSR_MODULE_C6_RES_MS: u64 = 0x0000_0664;

pub const SLM_PKG_C6_USE_C7_MSR: u64 = 1 << 0;
pub const KNL_CORE_C6_MSR: u64 = 1 << 1;

pub const PERF_CSTATE_CORE_C1_RES: usize = 0;
pub const PERF_CSTATE_CORE_C3_RES: usize = 1;
pub const PERF_CSTATE_CORE_C6_RES: usize = 2;
pub const PERF_CSTATE_CORE_C7_RES: usize = 3;
pub const PERF_CSTATE_CORE_EVENT_MAX: usize = 4;

pub const PERF_CSTATE_PKG_C2_RES: usize = 0;
pub const PERF_CSTATE_PKG_C3_RES: usize = 1;
pub const PERF_CSTATE_PKG_C6_RES: usize = 2;
pub const PERF_CSTATE_PKG_C7_RES: usize = 3;
pub const PERF_CSTATE_PKG_C8_RES: usize = 4;
pub const PERF_CSTATE_PKG_C9_RES: usize = 5;
pub const PERF_CSTATE_PKG_C10_RES: usize = 6;
pub const PERF_CSTATE_PKG_EVENT_MAX: usize = 7;

pub const PERF_CSTATE_MODULE_C6_RES: usize = 0;
pub const PERF_CSTATE_MODULE_EVENT_MAX: usize = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CStateScope {
    Core,
    Package,
    Die,
    Cluster,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CStatePmuKind {
    Core,
    Package,
    Module,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CStateEvent {
    pub scope: CStateScope,
    pub state: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CStateEventDescriptor {
    pub event: CStateEvent,
    pub name: &'static str,
    pub attr: &'static str,
    pub group: &'static str,
    pub msr: u64,
}

pub const CSTATE_CORE_EVENTS: [CStateEventDescriptor; PERF_CSTATE_CORE_EVENT_MAX] = [
    CStateEventDescriptor {
        event: CStateEvent {
            scope: CStateScope::Core,
            state: PERF_CSTATE_CORE_C1_RES as u8,
        },
        name: "c1-residency",
        attr: "event=0x00",
        group: "cstate_core_c1",
        msr: MSR_CORE_C1_RES,
    },
    CStateEventDescriptor {
        event: CStateEvent {
            scope: CStateScope::Core,
            state: PERF_CSTATE_CORE_C3_RES as u8,
        },
        name: "c3-residency",
        attr: "event=0x01",
        group: "cstate_core_c3",
        msr: MSR_CORE_C3_RESIDENCY,
    },
    CStateEventDescriptor {
        event: CStateEvent {
            scope: CStateScope::Core,
            state: PERF_CSTATE_CORE_C6_RES as u8,
        },
        name: "c6-residency",
        attr: "event=0x02",
        group: "cstate_core_c6",
        msr: MSR_CORE_C6_RESIDENCY,
    },
    CStateEventDescriptor {
        event: CStateEvent {
            scope: CStateScope::Core,
            state: PERF_CSTATE_CORE_C7_RES as u8,
        },
        name: "c7-residency",
        attr: "event=0x03",
        group: "cstate_core_c7",
        msr: MSR_CORE_C7_RESIDENCY,
    },
];

pub const CSTATE_PKG_EVENTS: [CStateEventDescriptor; PERF_CSTATE_PKG_EVENT_MAX] = [
    CStateEventDescriptor {
        event: CStateEvent {
            scope: CStateScope::Package,
            state: PERF_CSTATE_PKG_C2_RES as u8,
        },
        name: "c2-residency",
        attr: "event=0x00",
        group: "cstate_pkg_c2",
        msr: MSR_PKG_C2_RESIDENCY,
    },
    CStateEventDescriptor {
        event: CStateEvent {
            scope: CStateScope::Package,
            state: PERF_CSTATE_PKG_C3_RES as u8,
        },
        name: "c3-residency",
        attr: "event=0x01",
        group: "cstate_pkg_c3",
        msr: MSR_PKG_C3_RESIDENCY,
    },
    CStateEventDescriptor {
        event: CStateEvent {
            scope: CStateScope::Package,
            state: PERF_CSTATE_PKG_C6_RES as u8,
        },
        name: "c6-residency",
        attr: "event=0x02",
        group: "cstate_pkg_c6",
        msr: MSR_PKG_C6_RESIDENCY,
    },
    CStateEventDescriptor {
        event: CStateEvent {
            scope: CStateScope::Package,
            state: PERF_CSTATE_PKG_C7_RES as u8,
        },
        name: "c7-residency",
        attr: "event=0x03",
        group: "cstate_pkg_c7",
        msr: MSR_PKG_C7_RESIDENCY,
    },
    CStateEventDescriptor {
        event: CStateEvent {
            scope: CStateScope::Package,
            state: PERF_CSTATE_PKG_C8_RES as u8,
        },
        name: "c8-residency",
        attr: "event=0x04",
        group: "cstate_pkg_c8",
        msr: MSR_PKG_C8_RESIDENCY,
    },
    CStateEventDescriptor {
        event: CStateEvent {
            scope: CStateScope::Package,
            state: PERF_CSTATE_PKG_C9_RES as u8,
        },
        name: "c9-residency",
        attr: "event=0x05",
        group: "cstate_pkg_c9",
        msr: MSR_PKG_C9_RESIDENCY,
    },
    CStateEventDescriptor {
        event: CStateEvent {
            scope: CStateScope::Package,
            state: PERF_CSTATE_PKG_C10_RES as u8,
        },
        name: "c10-residency",
        attr: "event=0x06",
        group: "cstate_pkg_c10",
        msr: MSR_PKG_C10_RESIDENCY,
    },
];

pub const CSTATE_MODULE_EVENTS: [CStateEventDescriptor; PERF_CSTATE_MODULE_EVENT_MAX] =
    [CStateEventDescriptor {
        event: CStateEvent {
            scope: CStateScope::Cluster,
            state: PERF_CSTATE_MODULE_C6_RES as u8,
        },
        name: "c6-residency",
        attr: "event=0x00",
        group: "cstate_module_c6",
        msr: MSR_MODULE_C6_RES_MS,
    }];

pub const CSTATE_ATTR_GROUPS: [&str; 2] = ["events", "format"];
pub const CSTATE_FORMAT_ATTR: (&str, &str) = ("event", "config:0-63");
pub const CSTATE_CORE_ATTR_UPDATE: [&str; 4] = [
    "cstate_core_c1",
    "cstate_core_c3",
    "cstate_core_c6",
    "cstate_core_c7",
];
pub const CSTATE_PKG_ATTR_UPDATE: [&str; 7] = [
    "cstate_pkg_c2",
    "cstate_pkg_c3",
    "cstate_pkg_c6",
    "cstate_pkg_c7",
    "cstate_pkg_c8",
    "cstate_pkg_c9",
    "cstate_pkg_c10",
];
pub const CSTATE_MODULE_ATTR_UPDATE: [&str; 1] = ["cstate_module_c6"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CStatePmuDescriptor {
    pub name: &'static str,
    pub task_ctx_nr: &'static str,
    pub capabilities: u64,
    pub scope: CStateScope,
    pub attr_groups: &'static [&'static str],
    pub attr_update: &'static [&'static str],
    pub format: (&'static str, &'static str),
}

pub const CSTATE_CORE_PMU: CStatePmuDescriptor = CStatePmuDescriptor {
    name: "cstate_core",
    task_ctx_nr: "perf_invalid_context",
    capabilities: PERF_PMU_CAP_NO_INTERRUPT | PERF_PMU_CAP_NO_EXCLUDE,
    scope: CStateScope::Core,
    attr_groups: &CSTATE_ATTR_GROUPS,
    attr_update: &CSTATE_CORE_ATTR_UPDATE,
    format: CSTATE_FORMAT_ATTR,
};

pub const CSTATE_PKG_PMU: CStatePmuDescriptor = CStatePmuDescriptor {
    name: "cstate_pkg",
    task_ctx_nr: "perf_invalid_context",
    capabilities: PERF_PMU_CAP_NO_INTERRUPT | PERF_PMU_CAP_NO_EXCLUDE,
    scope: CStateScope::Package,
    attr_groups: &CSTATE_ATTR_GROUPS,
    attr_update: &CSTATE_PKG_ATTR_UPDATE,
    format: CSTATE_FORMAT_ATTR,
};

pub const CSTATE_MODULE_PMU: CStatePmuDescriptor = CStatePmuDescriptor {
    name: "cstate_module",
    task_ctx_nr: "perf_invalid_context",
    capabilities: PERF_PMU_CAP_NO_INTERRUPT | PERF_PMU_CAP_NO_EXCLUDE,
    scope: CStateScope::Cluster,
    attr_groups: &CSTATE_ATTR_GROUPS,
    attr_update: &CSTATE_MODULE_ATTR_UPDATE,
    format: CSTATE_FORMAT_ATTR,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CStateModel {
    pub core_events: u64,
    pub pkg_events: u64,
    pub module_events: u64,
    pub quirks: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CStateModelKind {
    Nhm,
    Snb,
    HswUlt,
    Cnl,
    Icl,
    Icx,
    Adl,
    Lnl,
    Nvl,
    Slm,
    Knl,
    Glm,
    Grr,
    Srf,
}

pub const fn bit(index: usize) -> u64 {
    1u64 << index
}

pub const NHM_CSTATES: CStateModel = CStateModel {
    core_events: bit(PERF_CSTATE_CORE_C3_RES) | bit(PERF_CSTATE_CORE_C6_RES),
    pkg_events: bit(PERF_CSTATE_PKG_C3_RES)
        | bit(PERF_CSTATE_PKG_C6_RES)
        | bit(PERF_CSTATE_PKG_C7_RES),
    module_events: 0,
    quirks: 0,
};

pub const SNB_CSTATES: CStateModel = CStateModel {
    core_events: bit(PERF_CSTATE_CORE_C3_RES)
        | bit(PERF_CSTATE_CORE_C6_RES)
        | bit(PERF_CSTATE_CORE_C7_RES),
    pkg_events: bit(PERF_CSTATE_PKG_C2_RES)
        | bit(PERF_CSTATE_PKG_C3_RES)
        | bit(PERF_CSTATE_PKG_C6_RES)
        | bit(PERF_CSTATE_PKG_C7_RES),
    module_events: 0,
    quirks: 0,
};

pub const HSWULT_CSTATES: CStateModel = CStateModel {
    core_events: SNB_CSTATES.core_events,
    pkg_events: bit(PERF_CSTATE_PKG_C2_RES)
        | bit(PERF_CSTATE_PKG_C3_RES)
        | bit(PERF_CSTATE_PKG_C6_RES)
        | bit(PERF_CSTATE_PKG_C7_RES)
        | bit(PERF_CSTATE_PKG_C8_RES)
        | bit(PERF_CSTATE_PKG_C9_RES)
        | bit(PERF_CSTATE_PKG_C10_RES),
    module_events: 0,
    quirks: 0,
};

pub const CNL_CSTATES: CStateModel = CStateModel {
    core_events: bit(PERF_CSTATE_CORE_C1_RES)
        | bit(PERF_CSTATE_CORE_C3_RES)
        | bit(PERF_CSTATE_CORE_C6_RES)
        | bit(PERF_CSTATE_CORE_C7_RES),
    pkg_events: HSWULT_CSTATES.pkg_events,
    module_events: 0,
    quirks: 0,
};

pub const ICL_CSTATES: CStateModel = CStateModel {
    core_events: bit(PERF_CSTATE_CORE_C6_RES) | bit(PERF_CSTATE_CORE_C7_RES),
    pkg_events: HSWULT_CSTATES.pkg_events,
    module_events: 0,
    quirks: 0,
};

pub const ICX_CSTATES: CStateModel = CStateModel {
    core_events: bit(PERF_CSTATE_CORE_C1_RES) | bit(PERF_CSTATE_CORE_C6_RES),
    pkg_events: bit(PERF_CSTATE_PKG_C2_RES) | bit(PERF_CSTATE_PKG_C6_RES),
    module_events: 0,
    quirks: 0,
};

pub const ADL_CSTATES: CStateModel = CStateModel {
    core_events: bit(PERF_CSTATE_CORE_C1_RES)
        | bit(PERF_CSTATE_CORE_C6_RES)
        | bit(PERF_CSTATE_CORE_C7_RES),
    pkg_events: bit(PERF_CSTATE_PKG_C2_RES)
        | bit(PERF_CSTATE_PKG_C3_RES)
        | bit(PERF_CSTATE_PKG_C6_RES)
        | bit(PERF_CSTATE_PKG_C8_RES)
        | bit(PERF_CSTATE_PKG_C10_RES),
    module_events: 0,
    quirks: 0,
};

pub const LNL_CSTATES: CStateModel = CStateModel {
    core_events: ADL_CSTATES.core_events,
    pkg_events: bit(PERF_CSTATE_PKG_C2_RES)
        | bit(PERF_CSTATE_PKG_C6_RES)
        | bit(PERF_CSTATE_PKG_C10_RES),
    module_events: 0,
    quirks: 0,
};

pub const NVL_CSTATES: CStateModel = CStateModel {
    core_events: ADL_CSTATES.core_events,
    pkg_events: LNL_CSTATES.pkg_events,
    module_events: bit(PERF_CSTATE_MODULE_C6_RES),
    quirks: 0,
};

pub const SLM_CSTATES: CStateModel = CStateModel {
    core_events: bit(PERF_CSTATE_CORE_C1_RES) | bit(PERF_CSTATE_CORE_C6_RES),
    pkg_events: bit(PERF_CSTATE_PKG_C6_RES),
    module_events: 0,
    quirks: SLM_PKG_C6_USE_C7_MSR,
};

pub const KNL_CSTATES: CStateModel = CStateModel {
    core_events: bit(PERF_CSTATE_CORE_C6_RES),
    pkg_events: bit(PERF_CSTATE_PKG_C2_RES)
        | bit(PERF_CSTATE_PKG_C3_RES)
        | bit(PERF_CSTATE_PKG_C6_RES),
    module_events: 0,
    quirks: KNL_CORE_C6_MSR,
};

pub const GLM_CSTATES: CStateModel = CStateModel {
    core_events: bit(PERF_CSTATE_CORE_C1_RES)
        | bit(PERF_CSTATE_CORE_C3_RES)
        | bit(PERF_CSTATE_CORE_C6_RES),
    pkg_events: bit(PERF_CSTATE_PKG_C2_RES)
        | bit(PERF_CSTATE_PKG_C3_RES)
        | bit(PERF_CSTATE_PKG_C6_RES)
        | bit(PERF_CSTATE_PKG_C10_RES),
    module_events: 0,
    quirks: 0,
};

pub const GRR_CSTATES: CStateModel = CStateModel {
    core_events: bit(PERF_CSTATE_CORE_C1_RES) | bit(PERF_CSTATE_CORE_C6_RES),
    pkg_events: 0,
    module_events: bit(PERF_CSTATE_MODULE_C6_RES),
    quirks: 0,
};

pub const SRF_CSTATES: CStateModel = CStateModel {
    core_events: bit(PERF_CSTATE_CORE_C1_RES) | bit(PERF_CSTATE_CORE_C6_RES),
    pkg_events: bit(PERF_CSTATE_PKG_C2_RES) | bit(PERF_CSTATE_PKG_C6_RES),
    module_events: bit(PERF_CSTATE_MODULE_C6_RES),
    quirks: 0,
};

pub const fn cstate_model(kind: CStateModelKind) -> CStateModel {
    match kind {
        CStateModelKind::Nhm => NHM_CSTATES,
        CStateModelKind::Snb => SNB_CSTATES,
        CStateModelKind::HswUlt => HSWULT_CSTATES,
        CStateModelKind::Cnl => CNL_CSTATES,
        CStateModelKind::Icl => ICL_CSTATES,
        CStateModelKind::Icx => ICX_CSTATES,
        CStateModelKind::Adl => ADL_CSTATES,
        CStateModelKind::Lnl => LNL_CSTATES,
        CStateModelKind::Nvl => NVL_CSTATES,
        CStateModelKind::Slm => SLM_CSTATES,
        CStateModelKind::Knl => KNL_CSTATES,
        CStateModelKind::Glm => GLM_CSTATES,
        CStateModelKind::Grr => GRR_CSTATES,
        CStateModelKind::Srf => SRF_CSTATES,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CStateCpuMatch {
    pub vfm: &'static str,
    pub model: CStateModelKind,
}

pub const INTEL_CSTATES_MATCH: &[CStateCpuMatch] = &[
    CStateCpuMatch {
        vfm: "INTEL_NEHALEM",
        model: CStateModelKind::Nhm,
    },
    CStateCpuMatch {
        vfm: "INTEL_NEHALEM_EP",
        model: CStateModelKind::Nhm,
    },
    CStateCpuMatch {
        vfm: "INTEL_NEHALEM_EX",
        model: CStateModelKind::Nhm,
    },
    CStateCpuMatch {
        vfm: "INTEL_WESTMERE",
        model: CStateModelKind::Nhm,
    },
    CStateCpuMatch {
        vfm: "INTEL_WESTMERE_EP",
        model: CStateModelKind::Nhm,
    },
    CStateCpuMatch {
        vfm: "INTEL_WESTMERE_EX",
        model: CStateModelKind::Nhm,
    },
    CStateCpuMatch {
        vfm: "INTEL_SANDYBRIDGE",
        model: CStateModelKind::Snb,
    },
    CStateCpuMatch {
        vfm: "INTEL_SANDYBRIDGE_X",
        model: CStateModelKind::Snb,
    },
    CStateCpuMatch {
        vfm: "INTEL_IVYBRIDGE",
        model: CStateModelKind::Snb,
    },
    CStateCpuMatch {
        vfm: "INTEL_IVYBRIDGE_X",
        model: CStateModelKind::Snb,
    },
    CStateCpuMatch {
        vfm: "INTEL_HASWELL",
        model: CStateModelKind::Snb,
    },
    CStateCpuMatch {
        vfm: "INTEL_HASWELL_X",
        model: CStateModelKind::Snb,
    },
    CStateCpuMatch {
        vfm: "INTEL_HASWELL_G",
        model: CStateModelKind::Snb,
    },
    CStateCpuMatch {
        vfm: "INTEL_HASWELL_L",
        model: CStateModelKind::HswUlt,
    },
    CStateCpuMatch {
        vfm: "INTEL_ATOM_SILVERMONT",
        model: CStateModelKind::Slm,
    },
    CStateCpuMatch {
        vfm: "INTEL_ATOM_SILVERMONT_D",
        model: CStateModelKind::Slm,
    },
    CStateCpuMatch {
        vfm: "INTEL_ATOM_AIRMONT",
        model: CStateModelKind::Slm,
    },
    CStateCpuMatch {
        vfm: "INTEL_ATOM_AIRMONT_NP",
        model: CStateModelKind::Slm,
    },
    CStateCpuMatch {
        vfm: "INTEL_BROADWELL",
        model: CStateModelKind::Snb,
    },
    CStateCpuMatch {
        vfm: "INTEL_BROADWELL_D",
        model: CStateModelKind::Snb,
    },
    CStateCpuMatch {
        vfm: "INTEL_BROADWELL_G",
        model: CStateModelKind::Snb,
    },
    CStateCpuMatch {
        vfm: "INTEL_BROADWELL_X",
        model: CStateModelKind::Snb,
    },
    CStateCpuMatch {
        vfm: "INTEL_SKYLAKE_L",
        model: CStateModelKind::Snb,
    },
    CStateCpuMatch {
        vfm: "INTEL_SKYLAKE",
        model: CStateModelKind::Snb,
    },
    CStateCpuMatch {
        vfm: "INTEL_SKYLAKE_X",
        model: CStateModelKind::Snb,
    },
    CStateCpuMatch {
        vfm: "INTEL_KABYLAKE_L",
        model: CStateModelKind::HswUlt,
    },
    CStateCpuMatch {
        vfm: "INTEL_KABYLAKE",
        model: CStateModelKind::HswUlt,
    },
    CStateCpuMatch {
        vfm: "INTEL_COMETLAKE_L",
        model: CStateModelKind::HswUlt,
    },
    CStateCpuMatch {
        vfm: "INTEL_COMETLAKE",
        model: CStateModelKind::HswUlt,
    },
    CStateCpuMatch {
        vfm: "INTEL_CANNONLAKE_L",
        model: CStateModelKind::Cnl,
    },
    CStateCpuMatch {
        vfm: "INTEL_XEON_PHI_KNL",
        model: CStateModelKind::Knl,
    },
    CStateCpuMatch {
        vfm: "INTEL_XEON_PHI_KNM",
        model: CStateModelKind::Knl,
    },
    CStateCpuMatch {
        vfm: "INTEL_ATOM_GOLDMONT",
        model: CStateModelKind::Glm,
    },
    CStateCpuMatch {
        vfm: "INTEL_ATOM_GOLDMONT_D",
        model: CStateModelKind::Glm,
    },
    CStateCpuMatch {
        vfm: "INTEL_ATOM_GOLDMONT_PLUS",
        model: CStateModelKind::Glm,
    },
    CStateCpuMatch {
        vfm: "INTEL_ATOM_TREMONT_D",
        model: CStateModelKind::Glm,
    },
    CStateCpuMatch {
        vfm: "INTEL_ATOM_TREMONT",
        model: CStateModelKind::Glm,
    },
    CStateCpuMatch {
        vfm: "INTEL_ATOM_TREMONT_L",
        model: CStateModelKind::Glm,
    },
    CStateCpuMatch {
        vfm: "INTEL_ATOM_GRACEMONT",
        model: CStateModelKind::Adl,
    },
    CStateCpuMatch {
        vfm: "INTEL_ATOM_CRESTMONT_X",
        model: CStateModelKind::Srf,
    },
    CStateCpuMatch {
        vfm: "INTEL_ATOM_CRESTMONT",
        model: CStateModelKind::Grr,
    },
    CStateCpuMatch {
        vfm: "INTEL_ATOM_DARKMONT_X",
        model: CStateModelKind::Srf,
    },
    CStateCpuMatch {
        vfm: "INTEL_ICELAKE_L",
        model: CStateModelKind::Icl,
    },
    CStateCpuMatch {
        vfm: "INTEL_ICELAKE",
        model: CStateModelKind::Icl,
    },
    CStateCpuMatch {
        vfm: "INTEL_ICELAKE_X",
        model: CStateModelKind::Icx,
    },
    CStateCpuMatch {
        vfm: "INTEL_ICELAKE_D",
        model: CStateModelKind::Icx,
    },
    CStateCpuMatch {
        vfm: "INTEL_SAPPHIRERAPIDS_X",
        model: CStateModelKind::Icx,
    },
    CStateCpuMatch {
        vfm: "INTEL_EMERALDRAPIDS_X",
        model: CStateModelKind::Icx,
    },
    CStateCpuMatch {
        vfm: "INTEL_GRANITERAPIDS_X",
        model: CStateModelKind::Icx,
    },
    CStateCpuMatch {
        vfm: "INTEL_GRANITERAPIDS_D",
        model: CStateModelKind::Icx,
    },
    CStateCpuMatch {
        vfm: "INTEL_DIAMONDRAPIDS_X",
        model: CStateModelKind::Srf,
    },
    CStateCpuMatch {
        vfm: "INTEL_TIGERLAKE_L",
        model: CStateModelKind::Icl,
    },
    CStateCpuMatch {
        vfm: "INTEL_TIGERLAKE",
        model: CStateModelKind::Icl,
    },
    CStateCpuMatch {
        vfm: "INTEL_ROCKETLAKE",
        model: CStateModelKind::Icl,
    },
    CStateCpuMatch {
        vfm: "INTEL_ALDERLAKE",
        model: CStateModelKind::Adl,
    },
    CStateCpuMatch {
        vfm: "INTEL_ALDERLAKE_L",
        model: CStateModelKind::Adl,
    },
    CStateCpuMatch {
        vfm: "INTEL_RAPTORLAKE",
        model: CStateModelKind::Adl,
    },
    CStateCpuMatch {
        vfm: "INTEL_RAPTORLAKE_P",
        model: CStateModelKind::Adl,
    },
    CStateCpuMatch {
        vfm: "INTEL_RAPTORLAKE_S",
        model: CStateModelKind::Adl,
    },
    CStateCpuMatch {
        vfm: "INTEL_METEORLAKE",
        model: CStateModelKind::Adl,
    },
    CStateCpuMatch {
        vfm: "INTEL_METEORLAKE_L",
        model: CStateModelKind::Adl,
    },
    CStateCpuMatch {
        vfm: "INTEL_ARROWLAKE",
        model: CStateModelKind::Adl,
    },
    CStateCpuMatch {
        vfm: "INTEL_ARROWLAKE_H",
        model: CStateModelKind::Adl,
    },
    CStateCpuMatch {
        vfm: "INTEL_ARROWLAKE_U",
        model: CStateModelKind::Adl,
    },
    CStateCpuMatch {
        vfm: "INTEL_LUNARLAKE_M",
        model: CStateModelKind::Lnl,
    },
    CStateCpuMatch {
        vfm: "INTEL_PANTHERLAKE_L",
        model: CStateModelKind::Lnl,
    },
    CStateCpuMatch {
        vfm: "INTEL_WILDCATLAKE_L",
        model: CStateModelKind::Lnl,
    },
    CStateCpuMatch {
        vfm: "INTEL_NOVALAKE",
        model: CStateModelKind::Nvl,
    },
    CStateCpuMatch {
        vfm: "INTEL_NOVALAKE_L",
        model: CStateModelKind::Nvl,
    },
];

pub fn cstate_model_for_vfm(vfm: &str) -> Option<CStateModelKind> {
    INTEL_CSTATES_MATCH
        .iter()
        .find(|entry| entry.vfm == vfm)
        .map(|entry| entry.model)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CStateAvailableMsrs {
    pub core: u64,
    pub pkg: u64,
    pub module: u64,
}

impl CStateAvailableMsrs {
    pub const fn all() -> Self {
        Self {
            core: mask_for_len(PERF_CSTATE_CORE_EVENT_MAX),
            pkg: mask_for_len(PERF_CSTATE_PKG_EVENT_MAX),
            module: mask_for_len(PERF_CSTATE_MODULE_EVENT_MAX),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CStateProbePlan {
    pub core_events: [CStateEventDescriptor; PERF_CSTATE_CORE_EVENT_MAX],
    pub pkg_events: [CStateEventDescriptor; PERF_CSTATE_PKG_EVENT_MAX],
    pub module_events: [CStateEventDescriptor; PERF_CSTATE_MODULE_EVENT_MAX],
    pub core_msr_mask: u64,
    pub pkg_msr_mask: u64,
    pub module_msr_mask: u64,
    pub has_core: bool,
    pub has_pkg: bool,
    pub has_module: bool,
}

pub const fn mask_for_len(len: usize) -> u64 {
    if len >= 64 {
        u64::MAX
    } else {
        (1u64 << len) - 1
    }
}

pub const fn perf_msr_probe_mask(candidate_events: u64, available_events: u64, len: usize) -> u64 {
    candidate_events & available_events & mask_for_len(len)
}

pub fn cstate_probe(
    model: CStateModel,
    available: CStateAvailableMsrs,
) -> Result<CStateProbePlan, i32> {
    let mut pkg_events = CSTATE_PKG_EVENTS;

    if (model.quirks & SLM_PKG_C6_USE_C7_MSR) != 0 {
        pkg_events[PERF_CSTATE_PKG_C6_RES].msr = MSR_PKG_C7_RESIDENCY;
    }
    if (model.quirks & KNL_CORE_C6_MSR) != 0 {
        pkg_events[PERF_CSTATE_CORE_C6_RES].msr = MSR_KNL_CORE_C6_RESIDENCY;
    }

    let core_msr_mask = perf_msr_probe_mask(
        model.core_events,
        available.core,
        PERF_CSTATE_CORE_EVENT_MAX,
    );
    let pkg_msr_mask =
        perf_msr_probe_mask(model.pkg_events, available.pkg, PERF_CSTATE_PKG_EVENT_MAX);
    let module_msr_mask = perf_msr_probe_mask(
        model.module_events,
        available.module,
        PERF_CSTATE_MODULE_EVENT_MAX,
    );
    let plan = CStateProbePlan {
        core_events: CSTATE_CORE_EVENTS,
        pkg_events,
        module_events: CSTATE_MODULE_EVENTS,
        core_msr_mask,
        pkg_msr_mask,
        module_msr_mask,
        has_core: core_msr_mask != 0,
        has_pkg: pkg_msr_mask != 0,
        has_module: module_msr_mask != 0,
    };

    if plan.has_core || plan.has_pkg || plan.has_module {
        Ok(plan)
    } else {
        Err(ENODEV)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CStatePerfEventAttr {
    pub typ: u32,
    pub config: u64,
    pub sample_period: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CStatePerfEvent {
    pub kind: CStatePmuKind,
    pub pmu_type: u32,
    pub attr: CStatePerfEventAttr,
    pub cpu: i32,
    pub idx: i32,
    pub event_base: u64,
    pub config: u64,
    pub prev_count: u64,
    pub count: u64,
}

pub fn cstate_pmu_event_init(
    kind: CStatePmuKind,
    event_type: u32,
    pmu_type: u32,
    sample_period: u64,
    cpu: i32,
    config: u64,
    probe: &CStateProbePlan,
) -> Result<CStatePerfEvent, i32> {
    if event_type != pmu_type {
        return Err(ENOENT);
    }
    if sample_period != 0 || cpu < 0 {
        return Err(EINVAL);
    }

    let event_base = match kind {
        CStatePmuKind::Core => {
            if config >= PERF_CSTATE_CORE_EVENT_MAX as u64 {
                return Err(EINVAL);
            }
            if (probe.core_msr_mask & bit(config as usize)) == 0 {
                return Err(EINVAL);
            }
            probe.core_events[config as usize].msr
        }
        CStatePmuKind::Package => {
            if config >= PERF_CSTATE_PKG_EVENT_MAX as u64 {
                return Err(EINVAL);
            }
            if (probe.pkg_msr_mask & bit(config as usize)) == 0 {
                return Err(EINVAL);
            }
            probe.pkg_events[config as usize].msr
        }
        CStatePmuKind::Module => {
            if config >= PERF_CSTATE_MODULE_EVENT_MAX as u64 {
                return Err(EINVAL);
            }
            if (probe.module_msr_mask & bit(config as usize)) == 0 {
                return Err(EINVAL);
            }
            probe.module_events[config as usize].msr
        }
        CStatePmuKind::Unknown => return Err(ENOENT),
    };

    Ok(CStatePerfEvent {
        kind,
        pmu_type,
        attr: CStatePerfEventAttr {
            typ: event_type,
            config,
            sample_period,
        },
        cpu,
        idx: -1,
        event_base,
        config,
        prev_count: 0,
        count: 0,
    })
}

pub const fn cstate_event_code(event: CStateEvent) -> u64 {
    event.state as u64
}

pub const fn cstate_pmu_read_counter(rdmsr_value: u64) -> u64 {
    rdmsr_value
}

pub fn cstate_pmu_event_update(event: &mut CStatePerfEvent, new_raw_count: u64) {
    let prev_raw_count = event.prev_count;
    event.prev_count = cstate_pmu_read_counter(new_raw_count);
    event.count = event
        .count
        .wrapping_add(event.prev_count.wrapping_sub(prev_raw_count));
}

pub fn cstate_pmu_event_start(event: &mut CStatePerfEvent, raw_count: u64) {
    event.prev_count = cstate_pmu_read_counter(raw_count);
}

pub fn cstate_pmu_event_stop(event: &mut CStatePerfEvent, raw_count: u64) {
    cstate_pmu_event_update(event, raw_count);
}

pub fn cstate_pmu_event_del(event: &mut CStatePerfEvent, raw_count: u64) {
    let _ = PERF_EF_UPDATE;
    cstate_pmu_event_stop(event, raw_count);
}

pub fn cstate_pmu_event_add(event: &mut CStatePerfEvent, mode: i32, raw_count: u64) -> i32 {
    if (mode & PERF_EF_START) != 0 {
        cstate_pmu_event_start(event, raw_count);
    }
    0
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CStateRegisterErrors {
    pub core: i32,
    pub pkg: i32,
    pub module: i32,
}

pub const CSTATE_REGISTER_OK: CStateRegisterErrors = CStateRegisterErrors {
    core: 0,
    pkg: 0,
    module: 0,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CStateRegistration {
    pub name: &'static str,
    pub scope: CStateScope,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CStateInitOutcome {
    pub result: Result<(), i32>,
    pub core_registration: Option<CStateRegistration>,
    pub pkg_registration: Option<CStateRegistration>,
    pub module_registration: Option<CStateRegistration>,
    pub has_core: bool,
    pub has_pkg: bool,
    pub has_module: bool,
    pub cleanup_called: bool,
    pub cleanup_core: bool,
    pub cleanup_pkg: bool,
    pub cleanup_module: bool,
}

fn cstate_init_error(
    err: i32,
    has_core: bool,
    has_pkg: bool,
    has_module: bool,
    core_registration: Option<CStateRegistration>,
    pkg_registration: Option<CStateRegistration>,
    module_registration: Option<CStateRegistration>,
) -> CStateInitOutcome {
    CStateInitOutcome {
        result: Err(err),
        core_registration,
        pkg_registration,
        module_registration,
        has_core,
        has_pkg,
        has_module,
        cleanup_called: true,
        cleanup_core: has_core,
        cleanup_pkg: has_pkg,
        cleanup_module: has_module,
    }
}

pub fn cstate_init(
    probe: CStateProbePlan,
    max_dies_per_package: u32,
    errors: CStateRegisterErrors,
) -> CStateInitOutcome {
    let mut has_core = probe.has_core;
    let mut has_pkg = probe.has_pkg;
    let mut has_module = probe.has_module;
    let mut core_registration = None;
    let mut pkg_registration = None;
    let mut module_registration = None;

    if has_core {
        if errors.core != 0 {
            has_core = false;
            return cstate_init_error(
                errors.core,
                has_core,
                has_pkg,
                has_module,
                core_registration,
                pkg_registration,
                module_registration,
            );
        }
        core_registration = Some(CStateRegistration {
            name: CSTATE_CORE_PMU.name,
            scope: CSTATE_CORE_PMU.scope,
        });
    }

    if has_pkg {
        let registration = if max_dies_per_package > 1 {
            CStateRegistration {
                name: "cstate_die",
                scope: CStateScope::Die,
            }
        } else {
            CStateRegistration {
                name: CSTATE_PKG_PMU.name,
                scope: CSTATE_PKG_PMU.scope,
            }
        };
        if errors.pkg != 0 {
            has_pkg = false;
            return cstate_init_error(
                errors.pkg,
                has_core,
                has_pkg,
                has_module,
                core_registration,
                pkg_registration,
                module_registration,
            );
        }
        pkg_registration = Some(registration);
    }

    if has_module {
        if errors.module != 0 {
            has_module = false;
            return cstate_init_error(
                errors.module,
                has_core,
                has_pkg,
                has_module,
                core_registration,
                pkg_registration,
                module_registration,
            );
        }
        module_registration = Some(CStateRegistration {
            name: CSTATE_MODULE_PMU.name,
            scope: CSTATE_MODULE_PMU.scope,
        });
    }

    CStateInitOutcome {
        result: Ok(()),
        core_registration,
        pkg_registration,
        module_registration,
        has_core,
        has_pkg,
        has_module,
        cleanup_called: false,
        cleanup_core: false,
        cleanup_pkg: false,
        cleanup_module: false,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CStatePmuInitInput {
    pub hypervisor: bool,
    pub matched_model: Option<CStateModelKind>,
    pub available: CStateAvailableMsrs,
    pub max_dies_per_package: u32,
    pub register_errors: CStateRegisterErrors,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CStatePmuInitPlan {
    pub probe: CStateProbePlan,
    pub init: CStateInitOutcome,
}

pub fn cstate_pmu_init(input: CStatePmuInitInput) -> Result<CStatePmuInitPlan, i32> {
    if input.hypervisor {
        return Err(ENODEV);
    }

    let Some(kind) = input.matched_model else {
        return Err(ENODEV);
    };

    let probe = cstate_probe(cstate_model(kind), input.available)?;
    let init = cstate_init(probe, input.max_dies_per_package, input.register_errors);
    match init.result {
        Ok(()) => Ok(CStatePmuInitPlan { probe, init }),
        Err(err) => Err(err),
    }
}

pub const fn cstate_pmu_exit_plan() -> &'static str {
    "cstate_cleanup()"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_probe(kind: CStateModelKind) -> CStateProbePlan {
        cstate_probe(cstate_model(kind), CStateAvailableMsrs::all()).unwrap()
    }

    #[test]
    fn cstate_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/events/intel/cstate.c"
        ));
        let perf_event = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/perf_event.h"
        ));
        let msr_index = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/asm/msr-index.h"
        ));

        assert!(source.contains("enum perf_cstate_core_events"));
        assert!(source.contains("PERF_CSTATE_CORE_C1_RES = 0"));
        assert!(source.contains("PERF_CSTATE_PKG_C10_RES"));
        assert!(source.contains("PERF_CSTATE_MODULE_C6_RES = 0"));
        assert!(source.contains("PMU_EVENT_ATTR_STRING(c1-residency"));
        assert!(source.contains("PMU_EVENT_ATTR_STRING(c10-residency"));
        assert!(source.contains("DEFINE_CSTATE_FORMAT_ATTR(cstate_event, event, \"config:0-63\")"));
        assert!(source.contains(".name\t\t= \"cstate_core\""));
        assert!(source.contains(".name\t\t= \"cstate_pkg\""));
        assert!(source.contains(".name\t\t= \"cstate_module\""));
        assert!(source.contains("PERF_PMU_CAP_NO_INTERRUPT | PERF_PMU_CAP_NO_EXCLUDE"));
        assert!(source.contains("if (event->attr.type != event->pmu->type)"));
        assert!(source.contains("if (event->attr.sample_period) /* no sampling */"));
        assert!(source.contains("if (event->cpu < 0)"));
        assert!(source.contains("event->hw.idx = -1"));
        assert!(source.contains("rdmsrq(event->hw.event_base, val);"));
        assert!(source.contains("local64_add(new_raw_count - prev_raw_count, &event->count);"));
        assert!(source.contains("cstate_pmu_event_stop(event, PERF_EF_UPDATE);"));
        assert!(source.contains("if (mode & PERF_EF_START)"));
        assert!(source.contains("pkg_msr[PERF_CSTATE_PKG_C6_RES].msr = MSR_PKG_C7_RESIDENCY;"));
        assert!(
            source.contains("pkg_msr[PERF_CSTATE_CORE_C6_RES].msr = MSR_KNL_CORE_C6_RESIDENCY;")
        );
        assert!(source.contains("perf_msr_probe(core_msr, PERF_CSTATE_CORE_EVENT_MAX"));
        assert!(source.contains("topology_max_dies_per_package() > 1"));
        assert!(source.contains("perf_pmu_register(&cstate_pkg_pmu,"));
        assert!(source.contains("\"cstate_die\", -1);"));
        assert!(source.contains("if (boot_cpu_has(X86_FEATURE_HYPERVISOR))"));
        assert!(source.contains("x86_match_cpu(intel_cstates_match)"));
        assert!(perf_event.contains("#define PERF_PMU_CAP_NO_INTERRUPT\t0x0001"));
        assert!(perf_event.contains("#define PERF_PMU_CAP_NO_EXCLUDE\t\t0x0040"));
        assert!(perf_event.contains("#define PERF_EF_START\t\t\t0x01"));
        assert!(perf_event.contains("#define PERF_EF_UPDATE\t\t\t0x04"));
        assert!(perf_event.contains("PERF_PMU_SCOPE_CLUSTER"));
        assert!(msr_index.contains("#define MSR_CORE_C1_RES"));
        assert!(msr_index.contains("#define MSR_MODULE_C6_RES_MS"));
    }

    #[test]
    fn event_tables_and_pmu_metadata_match_linux_order() {
        assert_eq!(CSTATE_FORMAT_ATTR, ("event", "config:0-63"));
        assert_eq!(
            CSTATE_CORE_EVENTS[PERF_CSTATE_CORE_C1_RES].attr,
            "event=0x00"
        );
        assert_eq!(
            CSTATE_CORE_EVENTS[PERF_CSTATE_CORE_C6_RES].msr,
            MSR_CORE_C6_RESIDENCY
        );
        assert_eq!(
            CSTATE_PKG_EVENTS[PERF_CSTATE_PKG_C2_RES].name,
            "c2-residency"
        );
        assert_eq!(
            CSTATE_PKG_EVENTS[PERF_CSTATE_PKG_C10_RES].attr,
            "event=0x06"
        );
        assert_eq!(
            CSTATE_MODULE_EVENTS[PERF_CSTATE_MODULE_C6_RES].msr,
            MSR_MODULE_C6_RES_MS
        );
        assert_eq!(CSTATE_CORE_PMU.name, "cstate_core");
        assert_eq!(CSTATE_PKG_PMU.scope, CStateScope::Package);
        assert_eq!(CSTATE_MODULE_PMU.scope, CStateScope::Cluster);
        assert_eq!(
            CSTATE_CORE_PMU.capabilities,
            PERF_PMU_CAP_NO_INTERRUPT | PERF_PMU_CAP_NO_EXCLUDE
        );
        assert_eq!(CSTATE_CORE_ATTR_UPDATE[3], "cstate_core_c7");
        assert_eq!(CSTATE_PKG_ATTR_UPDATE[6], "cstate_pkg_c10");
    }

    #[test]
    fn model_masks_and_cpu_match_table_follow_source() {
        assert_eq!(
            NHM_CSTATES.core_events,
            bit(PERF_CSTATE_CORE_C3_RES) | bit(PERF_CSTATE_CORE_C6_RES)
        );
        assert_eq!(
            HSWULT_CSTATES.pkg_events,
            bit(PERF_CSTATE_PKG_C2_RES)
                | bit(PERF_CSTATE_PKG_C3_RES)
                | bit(PERF_CSTATE_PKG_C6_RES)
                | bit(PERF_CSTATE_PKG_C7_RES)
                | bit(PERF_CSTATE_PKG_C8_RES)
                | bit(PERF_CSTATE_PKG_C9_RES)
                | bit(PERF_CSTATE_PKG_C10_RES)
        );
        assert_eq!(NVL_CSTATES.module_events, bit(PERF_CSTATE_MODULE_C6_RES));
        assert_eq!(SLM_CSTATES.quirks, SLM_PKG_C6_USE_C7_MSR);
        assert_eq!(KNL_CSTATES.quirks, KNL_CORE_C6_MSR);
        assert_eq!(
            cstate_model_for_vfm("INTEL_HASWELL_L"),
            Some(CStateModelKind::HswUlt)
        );
        assert_eq!(
            cstate_model_for_vfm("INTEL_ATOM_CRESTMONT"),
            Some(CStateModelKind::Grr)
        );
        assert_eq!(
            cstate_model_for_vfm("INTEL_DIAMONDRAPIDS_X"),
            Some(CStateModelKind::Srf)
        );
        assert_eq!(
            cstate_model_for_vfm("INTEL_NOVALAKE_L"),
            Some(CStateModelKind::Nvl)
        );
        assert_eq!(cstate_model_for_vfm("INTEL_UNKNOWN"), None);
    }

    #[test]
    fn probe_applies_linux_quirks_and_detects_empty_masks() {
        let slm = all_probe(CStateModelKind::Slm);
        assert_eq!(
            slm.pkg_events[PERF_CSTATE_PKG_C6_RES].msr,
            MSR_PKG_C7_RESIDENCY
        );
        assert_eq!(slm.pkg_msr_mask, bit(PERF_CSTATE_PKG_C6_RES));
        assert!(slm.has_core);
        assert!(slm.has_pkg);
        assert!(!slm.has_module);

        let knl = all_probe(CStateModelKind::Knl);
        assert_eq!(
            knl.pkg_events[PERF_CSTATE_CORE_C6_RES].msr,
            MSR_KNL_CORE_C6_RESIDENCY
        );
        assert_eq!(
            knl.core_events[PERF_CSTATE_CORE_C6_RES].msr,
            MSR_CORE_C6_RESIDENCY
        );

        let nvl = all_probe(CStateModelKind::Nvl);
        assert_eq!(nvl.module_msr_mask, bit(PERF_CSTATE_MODULE_C6_RES));
        assert!(nvl.has_module);

        assert_eq!(
            cstate_probe(
                cstate_model(CStateModelKind::Nhm),
                CStateAvailableMsrs {
                    core: 0,
                    pkg: 0,
                    module: 0,
                },
            ),
            Err(ENODEV)
        );
    }

    #[test]
    fn event_init_validates_type_sampling_cpu_config_and_mask() {
        let probe = all_probe(CStateModelKind::Adl);
        assert_eq!(
            cstate_pmu_event_init(CStatePmuKind::Core, 1, 2, 0, 0, 0, &probe),
            Err(ENOENT)
        );
        assert_eq!(
            cstate_pmu_event_init(CStatePmuKind::Core, 1, 1, 7, 0, 0, &probe),
            Err(EINVAL)
        );
        assert_eq!(
            cstate_pmu_event_init(CStatePmuKind::Core, 1, 1, 0, -1, 0, &probe),
            Err(EINVAL)
        );
        assert_eq!(
            cstate_pmu_event_init(
                CStatePmuKind::Package,
                1,
                1,
                0,
                0,
                PERF_CSTATE_PKG_EVENT_MAX as u64,
                &probe,
            ),
            Err(EINVAL)
        );
        assert_eq!(
            cstate_pmu_event_init(
                CStatePmuKind::Package,
                1,
                1,
                0,
                0,
                PERF_CSTATE_PKG_C7_RES as u64,
                &probe,
            ),
            Err(EINVAL)
        );
        assert_eq!(
            cstate_pmu_event_init(CStatePmuKind::Unknown, 1, 1, 0, 0, 0, &probe),
            Err(ENOENT)
        );

        let core = cstate_pmu_event_init(
            CStatePmuKind::Core,
            1,
            1,
            0,
            3,
            PERF_CSTATE_CORE_C6_RES as u64,
            &probe,
        )
        .unwrap();
        assert_eq!(core.idx, -1);
        assert_eq!(core.config, PERF_CSTATE_CORE_C6_RES as u64);
        assert_eq!(core.event_base, MSR_CORE_C6_RESIDENCY);

        let pkg = cstate_pmu_event_init(
            CStatePmuKind::Package,
            1,
            1,
            0,
            3,
            PERF_CSTATE_PKG_C10_RES as u64,
            &probe,
        )
        .unwrap();
        assert_eq!(pkg.event_base, MSR_PKG_C10_RESIDENCY);
    }

    #[test]
    fn read_update_start_stop_del_and_add_follow_free_running_counter_flow() {
        let probe = all_probe(CStateModelKind::Cnl);
        let mut event = cstate_pmu_event_init(
            CStatePmuKind::Core,
            1,
            1,
            0,
            0,
            PERF_CSTATE_CORE_C1_RES as u64,
            &probe,
        )
        .unwrap();

        assert_eq!(cstate_event_code(CSTATE_CORE_EVENTS[0].event), 0);
        assert_eq!(cstate_pmu_read_counter(42), 42);
        assert_eq!(cstate_pmu_event_add(&mut event, PERF_EF_START, 100), 0);
        assert_eq!(event.prev_count, 100);
        cstate_pmu_event_update(&mut event, 150);
        assert_eq!(event.prev_count, 150);
        assert_eq!(event.count, 50);
        cstate_pmu_event_stop(&mut event, 175);
        assert_eq!(event.count, 75);
        cstate_pmu_event_del(&mut event, 180);
        assert_eq!(event.count, 80);

        event.prev_count = u64::MAX - 1;
        cstate_pmu_event_update(&mut event, 2);
        assert_eq!(event.count, 84);
    }

    #[test]
    fn init_rejects_hypervisor_and_match_miss_then_registers_die_scope_pkg() {
        assert_eq!(
            cstate_pmu_init(CStatePmuInitInput {
                hypervisor: true,
                matched_model: Some(CStateModelKind::Nhm),
                available: CStateAvailableMsrs::all(),
                max_dies_per_package: 1,
                register_errors: CSTATE_REGISTER_OK,
            }),
            Err(ENODEV)
        );
        assert_eq!(
            cstate_pmu_init(CStatePmuInitInput {
                hypervisor: false,
                matched_model: None,
                available: CStateAvailableMsrs::all(),
                max_dies_per_package: 1,
                register_errors: CSTATE_REGISTER_OK,
            }),
            Err(ENODEV)
        );

        let plan = cstate_pmu_init(CStatePmuInitInput {
            hypervisor: false,
            matched_model: Some(CStateModelKind::Nvl),
            available: CStateAvailableMsrs::all(),
            max_dies_per_package: 2,
            register_errors: CSTATE_REGISTER_OK,
        })
        .unwrap();
        assert_eq!(plan.init.result, Ok(()));
        assert_eq!(
            plan.init.core_registration,
            Some(CStateRegistration {
                name: "cstate_core",
                scope: CStateScope::Core,
            })
        );
        assert_eq!(
            plan.init.pkg_registration,
            Some(CStateRegistration {
                name: "cstate_die",
                scope: CStateScope::Die,
            })
        );
        assert_eq!(
            plan.init.module_registration,
            Some(CStateRegistration {
                name: "cstate_module",
                scope: CStateScope::Cluster,
            })
        );
        assert_eq!(cstate_pmu_exit_plan(), "cstate_cleanup()");
    }

    #[test]
    fn init_failure_clears_failed_pmu_and_runs_cleanup_path() {
        let probe = all_probe(CStateModelKind::Nvl);
        let outcome = cstate_init(
            probe,
            1,
            CStateRegisterErrors {
                core: 0,
                pkg: -22,
                module: 0,
            },
        );
        assert_eq!(outcome.result, Err(-22));
        assert!(outcome.has_core);
        assert!(!outcome.has_pkg);
        assert!(outcome.has_module);
        assert!(outcome.cleanup_called);
        assert!(outcome.cleanup_core);
        assert!(!outcome.cleanup_pkg);
        assert!(outcome.cleanup_module);
        assert_eq!(
            outcome.core_registration,
            Some(CStateRegistration {
                name: "cstate_core",
                scope: CStateScope::Core,
            })
        );
        assert_eq!(outcome.pkg_registration, None);
    }
}
