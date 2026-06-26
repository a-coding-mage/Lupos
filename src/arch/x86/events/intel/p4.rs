//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/events/intel/p4.c
//! test-origin: linux:vendor/linux/arch/x86/events/intel/p4.c
//! Intel Pentium 4 PMU model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/events/intel/p4.c

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct P4EscrEvent {
    pub event_select: u8,
    pub event_mask: u16,
}

pub const fn p4_is_supported(family: u8) -> bool {
    family == 0x0f
}

pub const fn encode_escr(event: P4EscrEvent) -> u64 {
    ((event.event_mask as u64) << 9) | ((event.event_select as u64) << 25)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p4_escr_encoding_places_event_and_mask() {
        assert_eq!(
            encode_escr(P4EscrEvent {
                event_select: 0x04,
                event_mask: 0x02,
            }),
            (0x02 << 9) | (0x04 << 25)
        );
    }
}
