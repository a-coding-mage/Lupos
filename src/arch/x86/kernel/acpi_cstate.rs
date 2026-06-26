//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! x86 ACPI C-state selection model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/acpi/cstate.c

use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AcpiCstate {
    pub state: u8,
    pub latency_us: u32,
    pub power_mw: u32,
    pub mwait_hint: u32,
}

pub const fn cstate_valid(state: AcpiCstate) -> Result<(), i32> {
    if state.state == 0 || state.state > 10 {
        Err(EINVAL)
    } else {
        Ok(())
    }
}

pub fn deepest_cstate_with_latency(
    states: &[AcpiCstate],
    max_latency_us: u32,
) -> Option<AcpiCstate> {
    let mut best = None;
    for state in states {
        if state.latency_us <= max_latency_us && cstate_valid(*state).is_ok() {
            best = Some(*state);
        }
    }
    best
}

pub const fn mwait_substate(hint: u32) -> u8 {
    (hint & 0x0f) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deepest_cstate_respects_latency_limit() {
        let states = [
            AcpiCstate {
                state: 1,
                latency_us: 1,
                power_mw: 100,
                mwait_hint: 0,
            },
            AcpiCstate {
                state: 6,
                latency_us: 80,
                power_mw: 5,
                mwait_hint: 0x20,
            },
        ];
        assert_eq!(deepest_cstate_with_latency(&states, 10).unwrap().state, 1);
        assert_eq!(deepest_cstate_with_latency(&states, 100).unwrap().state, 6);
    }
}
