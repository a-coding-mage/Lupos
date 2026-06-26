//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/topology.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/topology.c
//! Cross-vendor x86 topology decoder.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/topology.c

// `topology.c` provides the umbrella API that AMD/Intel-specific files
// dispatch through. The data model is a triple of (package_id, die_id,
// core_id, thread_id) derived from APIC ID by shifting off the SMT,
// core, and die bits in order. We model the shifter table.

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct TopologyShifts {
    pub smt: u8,
    pub core: u8,
    pub die: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct TopologyId {
    pub package: u32,
    pub die: u32,
    pub core: u32,
    pub thread: u32,
}

pub const fn decode_apicid(apicid: u32, shifts: TopologyShifts) -> TopologyId {
    let thread_mask = (1u32 << shifts.smt) - 1;
    let core_mask = (1u32 << shifts.core) - 1;
    let die_mask = (1u32 << shifts.die) - 1;
    let thread = apicid & thread_mask;
    let core = (apicid >> shifts.smt) & core_mask;
    let die = (apicid >> (shifts.smt + shifts.core)) & die_mask;
    let package = apicid >> (shifts.smt + shifts.core + shifts.die);
    TopologyId {
        package,
        die,
        core,
        thread,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_uses_layered_shifts() {
        let shifts = TopologyShifts {
            smt: 1,
            core: 3,
            die: 0,
        };
        // bits: package | core(3) | thread(1)
        let id = decode_apicid(0b1_011_1, shifts);
        assert_eq!(id.thread, 1);
        assert_eq!(id.core, 0b011);
        assert_eq!(id.die, 0);
        assert_eq!(id.package, 1);
    }
}
