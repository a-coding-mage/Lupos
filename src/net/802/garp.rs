//! linux-parity: partial
//! linux-source: vendor/linux/net/802/garp.c
//! test-origin: linux:vendor/linux/net/802/garp.c
//! GARP applicant state transitions used by VLAN registration protocols.

pub const GARP_PROTOCOL_ID: u16 = 0x1;
pub const GARP_END_MARK: u8 = 0x0;
pub const GARP_JOIN_TIME_MS: u32 = 200;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GarpApplicantState {
    Invalid,
    Va,
    Aa,
    Qa,
    La,
    Vp,
    Ap,
    Qp,
    Vo,
    Ao,
    Qo,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GarpEvent {
    ReqJoin,
    ReqLeave,
    RJoinIn,
    RJoinEmpty,
    REmpty,
    RLeaveIn,
    RLeaveEmpty,
    TransmitPdu,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GarpAction {
    None,
    SendJoinIn,
    SendLeaveEmpty,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GarpTransition {
    pub state: GarpApplicantState,
    pub action: GarpAction,
    pub destroy_after_action: bool,
}

pub const fn garp_transition(state: GarpApplicantState, event: GarpEvent) -> GarpTransition {
    use GarpAction::*;
    use GarpApplicantState::*;
    use GarpEvent::*;
    let next = match (state, event) {
        (Vo, ReqJoin) => (Vp, None, false),
        (Vp, ReqLeave) => (Vo, None, false),
        (Va, ReqLeave) | (Aa, ReqLeave) | (Qa, ReqLeave) => (La, None, false),
        (La, ReqJoin) => (Va, None, false),
        (Vp, RJoinIn) => (Ap, None, false),
        (Ap, RJoinIn) => (Qp, None, false),
        (Ao, ReqJoin) => (Ap, None, false),
        (Qo, ReqJoin) => (Qp, None, false),
        (Va, TransmitPdu) | (Vp, TransmitPdu) => (Aa, SendJoinIn, false),
        (Aa, TransmitPdu) | (Ap, TransmitPdu) => (Qa, SendJoinIn, false),
        (La, TransmitPdu) => (Vo, SendLeaveEmpty, true),
        _ => (state, None, false),
    };
    GarpTransition {
        state: next.0,
        action: next.1,
        destroy_after_action: next.2,
    }
}

pub const fn garp_attr_initial_state() -> GarpApplicantState {
    GarpApplicantState::Vo
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garp_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/802/garp.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/net/garp.h"
        ));
        assert!(source.contains("static unsigned int garp_join_time __read_mostly = 200;"));
        assert!(source.contains("garp_applicant_state_table"));
        assert!(source.contains("[GARP_APPLICANT_VO]"));
        assert!(source.contains("[GARP_EVENT_REQ_JOIN]\t\t= { .state = GARP_APPLICANT_VP }"));
        assert!(source.contains("[GARP_EVENT_REQ_LEAVE]\t\t= { .state = GARP_APPLICANT_VO }"));
        assert!(source.contains("GARP_ACTION_S_JOIN_IN"));
        assert!(source.contains("GARP_ACTION_S_LEAVE_EMPTY"));
        assert!(source.contains("attr->state = GARP_APPLICANT_VO;"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(garp_request_join);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(garp_unregister_application);"));
        assert!(header.contains("#define GARP_PROTOCOL_ID\t0x1"));
        assert!(header.contains("enum garp_applicant_state"));
    }

    #[test]
    fn garp_request_and_tx_transitions_follow_linux_table() {
        assert_eq!(garp_attr_initial_state(), GarpApplicantState::Vo);
        assert_eq!(
            garp_transition(GarpApplicantState::Vo, GarpEvent::ReqJoin),
            GarpTransition {
                state: GarpApplicantState::Vp,
                action: GarpAction::None,
                destroy_after_action: false,
            }
        );
        assert_eq!(
            garp_transition(GarpApplicantState::Vp, GarpEvent::ReqLeave).state,
            GarpApplicantState::Vo
        );
        assert_eq!(
            garp_transition(GarpApplicantState::Vp, GarpEvent::TransmitPdu).action,
            GarpAction::SendJoinIn
        );
        assert_eq!(
            garp_transition(GarpApplicantState::La, GarpEvent::TransmitPdu),
            GarpTransition {
                state: GarpApplicantState::Vo,
                action: GarpAction::SendLeaveEmpty,
                destroy_after_action: true,
            }
        );
    }
}
