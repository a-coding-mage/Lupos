//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kernel/cpu/microcode/core.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/microcode/core.c
//! Unified microcode driver state machine.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/microcode/core.c

// Linux drives microcode loading through a stop_machine() handshake.
// The state machine is: Idle -> Requested -> Loaded -> Activated.
// We model the transitions so observability code can reason about
// stop-machine progress without invoking the actual stop primitive.

use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MicrocodeLoadState {
    Idle,
    Requested,
    Loaded,
    Activated,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MicrocodeDriver {
    pub state: MicrocodeLoadState,
    pub current_revision: u32,
    pub target_revision: u32,
}

impl MicrocodeDriver {
    pub const fn new(current_revision: u32) -> Self {
        Self {
            state: MicrocodeLoadState::Idle,
            current_revision,
            target_revision: current_revision,
        }
    }

    pub fn request(&mut self, target_revision: u32) -> Result<(), i32> {
        if !matches!(self.state, MicrocodeLoadState::Idle) {
            return Err(EINVAL);
        }
        if target_revision <= self.current_revision {
            self.state = MicrocodeLoadState::Failed;
            return Err(EINVAL);
        }
        self.target_revision = target_revision;
        self.state = MicrocodeLoadState::Requested;
        Ok(())
    }

    pub fn mark_loaded(&mut self) -> Result<(), i32> {
        if !matches!(self.state, MicrocodeLoadState::Requested) {
            return Err(EINVAL);
        }
        self.state = MicrocodeLoadState::Loaded;
        Ok(())
    }

    pub fn activate(&mut self) -> Result<(), i32> {
        if !matches!(self.state, MicrocodeLoadState::Loaded) {
            return Err(EINVAL);
        }
        self.current_revision = self.target_revision;
        self.state = MicrocodeLoadState::Activated;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_machine_flows_idle_to_activated() {
        let mut d = MicrocodeDriver::new(0x100);
        d.request(0x200).unwrap();
        assert_eq!(d.state, MicrocodeLoadState::Requested);
        d.mark_loaded().unwrap();
        d.activate().unwrap();
        assert_eq!(d.state, MicrocodeLoadState::Activated);
        assert_eq!(d.current_revision, 0x200);
    }

    #[test]
    fn target_must_be_newer_revision() {
        let mut d = MicrocodeDriver::new(0x200);
        assert_eq!(d.request(0x100), Err(EINVAL));
        assert_eq!(d.state, MicrocodeLoadState::Failed);
    }

    #[test]
    fn out_of_order_transitions_are_rejected() {
        let mut d = MicrocodeDriver::new(0x100);
        assert_eq!(d.mark_loaded(), Err(EINVAL));
        assert_eq!(d.activate(), Err(EINVAL));
    }
}
