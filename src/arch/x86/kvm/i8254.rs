//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/i8254.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/i8254.c
//! KVM-emulated i8254 PIT (programmable interval timer).
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/i8254.c

// KVM emulates the legacy i8254 for guests that depend on PIT ticks.
// The PIT has 3 channels; each runs in one of 6 modes selected by the
// command port. We model the channel state machine.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PitMode {
    InterruptOnTerminalCount,
    HardwareRetriggerable,
    RateGenerator,
    SquareWave,
    SoftwareStrobe,
    HardwareStrobe,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct PitChannel {
    pub count: u16,
    pub latch: u16,
    pub running: bool,
}

pub const fn mode_from_command(command_bits: u8) -> PitMode {
    match (command_bits >> 1) & 0x7 {
        0 => PitMode::InterruptOnTerminalCount,
        1 => PitMode::HardwareRetriggerable,
        2 | 6 => PitMode::RateGenerator,
        3 | 7 => PitMode::SquareWave,
        4 => PitMode::SoftwareStrobe,
        5 => PitMode::HardwareStrobe,
        _ => PitMode::InterruptOnTerminalCount,
    }
}

pub fn tick(channel: &mut PitChannel) -> bool {
    if !channel.running {
        return false;
    }
    if channel.count == 0 {
        channel.count = channel.latch;
        return true;
    }
    channel.count -= 1;
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_decoder_matches_command_bits() {
        assert_eq!(
            mode_from_command(0b0000_0000),
            PitMode::InterruptOnTerminalCount
        );
        assert_eq!(mode_from_command(0b0000_0110), PitMode::SquareWave);
        assert_eq!(mode_from_command(0b0000_1000), PitMode::SoftwareStrobe);
    }

    #[test]
    fn tick_reloads_count_from_latch_on_terminal() {
        let mut c = PitChannel {
            count: 0,
            latch: 100,
            running: true,
        };
        assert!(tick(&mut c));
        assert_eq!(c.count, 100);
    }
}
