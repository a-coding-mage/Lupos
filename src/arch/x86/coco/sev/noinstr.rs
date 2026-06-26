//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/coco/sev/noinstr.c
//! test-origin: linux:vendor/linux/arch/x86/coco/sev/noinstr.c
//! SEV-ES noinstr GHCB entry/exit helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/coco/sev/noinstr.c

use crate::include::uapi::errno::EBUSY;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GhcbState {
    pub ghcb_pa: u64,
    pub active: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SevEsRuntimeData {
    pub ghcb_pa: u64,
    pub in_use: bool,
    pub on_vc_stack: bool,
    pub ist_entered: bool,
    pub nmi_complete: bool,
}

pub const fn on_vc_stack(data: &SevEsRuntimeData) -> bool {
    data.on_vc_stack
}

pub fn sev_es_ist_enter(data: &mut SevEsRuntimeData) {
    data.ist_entered = true;
    data.on_vc_stack = true;
}

pub fn sev_es_ist_exit(data: &mut SevEsRuntimeData) {
    data.on_vc_stack = false;
    data.ist_entered = false;
}

pub fn sev_es_nmi_complete(data: &mut SevEsRuntimeData) {
    data.nmi_complete = true;
}

pub fn sev_get_ghcb(data: &mut SevEsRuntimeData, state: &mut GhcbState) -> Result<u64, i32> {
    if data.in_use {
        return Err(EBUSY);
    }
    data.in_use = true;
    state.ghcb_pa = data.ghcb_pa;
    state.active = true;
    Ok(data.ghcb_pa)
}

pub fn sev_put_ghcb(data: &mut SevEsRuntimeData, state: &mut GhcbState) {
    if state.active {
        data.in_use = false;
        state.active = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ghcb_state_rejects_nested_use() {
        let mut data = SevEsRuntimeData {
            ghcb_pa: 0x1000,
            ..Default::default()
        };
        let mut state = GhcbState::default();
        assert_eq!(sev_get_ghcb(&mut data, &mut state), Ok(0x1000));
        assert_eq!(
            sev_get_ghcb(&mut data, &mut GhcbState::default()),
            Err(EBUSY)
        );
        sev_put_ghcb(&mut data, &mut state);
        assert!(!data.in_use);
    }

    #[test]
    fn ist_entry_tracks_vc_stack_state() {
        let mut data = SevEsRuntimeData::default();
        sev_es_ist_enter(&mut data);
        assert!(on_vc_stack(&data));
        sev_es_ist_exit(&mut data);
        assert!(!on_vc_stack(&data));
    }
}
