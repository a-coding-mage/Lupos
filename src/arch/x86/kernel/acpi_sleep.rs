//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! x86 ACPI sleep-state model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/acpi/sleep.c

use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcpiSleepState {
    S0,
    S1,
    S3,
    S4,
    S5,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AcpiSleepControl {
    pub state: AcpiSleepState,
    pub slp_typ: u16,
    pub slp_en: bool,
}

pub const fn sleep_state_supported(state: AcpiSleepState, has_fadt: bool) -> bool {
    match state {
        AcpiSleepState::S0 => true,
        AcpiSleepState::S1 | AcpiSleepState::S3 | AcpiSleepState::S4 | AcpiSleepState::S5 => {
            has_fadt
        }
    }
}

pub const fn sleep_control(state: AcpiSleepState, slp_typ: u16) -> Result<AcpiSleepControl, i32> {
    if slp_typ > 7 {
        return Err(EINVAL);
    }
    Ok(AcpiSleepControl {
        state,
        slp_typ,
        slp_en: !matches!(state, AcpiSleepState::S0),
    })
}

pub const fn pm1_control_value(control: AcpiSleepControl) -> u16 {
    (control.slp_typ << 10) | if control.slp_en { 1 << 13 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pm1_control_places_typ_and_enable_bits() {
        let ctl = sleep_control(AcpiSleepState::S5, 5).unwrap();
        assert_eq!(pm1_control_value(ctl), (5 << 10) | (1 << 13));
        assert_eq!(sleep_control(AcpiSleepState::S3, 8), Err(EINVAL));
    }
}
