//! linux-parity: partial
//! linux-source: vendor/linux/net/802/mrp.c
//! test-origin: linux:vendor/linux/net/802/mrp.c
//! MRP applicant transitions, transmit actions, and vector attribute increments.

pub const MRP_END_MARK: u8 = 0x0;
pub const MRP_JOIN_TIME_MS: u32 = 200;
pub const MRP_PERIODIC_TIME_MS: u32 = 1000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MrpApplicantState {
    Invalid,
    Vo,
    Vp,
    Vn,
    An,
    Aa,
    Qa,
    La,
    Ao,
    Qo,
    Ap,
    Qp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MrpEvent {
    New,
    Join,
    Lv,
    Tx,
    RNew,
    RJoinIn,
    RIn,
    RJoinMt,
    RMt,
    RLv,
    RLa,
    Redeclare,
    Periodic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MrpTxAction {
    None,
    SendNew,
    SendJoinIn,
    SendJoinInOptional,
    SendInOptional,
    SendLv,
}

pub const fn mrp_transition(state: MrpApplicantState, event: MrpEvent) -> MrpApplicantState {
    use MrpApplicantState::*;
    use MrpEvent::*;
    match (state, event) {
        (Vo, New) => Vn,
        (Vo, Join) => Vp,
        (Vo, Lv) | (Vo, Tx) => Vo,
        (Vo, RJoinIn) => Ao,
        (Vp, New) => Vn,
        (Vp, Lv) => Vo,
        (Vp, Tx) => Aa,
        (Vp, RJoinIn) => Ap,
        (Vn, Lv) => La,
        (Vn, Tx) => An,
        (An, Tx) => Qa,
        (An, RLv) | (An, RLa) | (An, Redeclare) => Vn,
        (Aa, Tx) | (Aa, RJoinIn) => Qa,
        (Aa, RLv) | (Aa, RLa) | (Aa, Redeclare) => Vp,
        (Qa, RJoinMt) | (Qa, RMt) | (Qa, Periodic) => Aa,
        (Qa, Lv) => La,
        (La, Join) => Aa,
        (La, Tx) => Vo,
        (Ao, Join) => Ap,
        (Ao, RJoinIn) => Qo,
        (Ao, RLv) | (Ao, RLa) | (Ao, Redeclare) => Vo,
        (Qo, Join) => Qp,
        (Qo, RJoinMt) | (Qo, RMt) => Ao,
        (Qo, RLv) | (Qo, RLa) | (Qo, Redeclare) => Vo,
        (Ap, Lv) => Ao,
        (Ap, Tx) => Qa,
        (Ap, RJoinIn) => Qp,
        (Ap, RLv) | (Ap, RLa) | (Ap, Redeclare) => Vp,
        (Qp, Lv) => Qo,
        (Qp, RJoinMt) | (Qp, RMt) | (Qp, Periodic) => Ap,
        (Qp, RLv) | (Qp, RLa) | (Qp, Redeclare) => Vp,
        _ => state,
    }
}

pub const fn mrp_tx_action(state: MrpApplicantState) -> MrpTxAction {
    use MrpApplicantState::*;
    match state {
        Vo | Ao | Qo | Qp => MrpTxAction::SendInOptional,
        Vp | Aa | Ap => MrpTxAction::SendJoinIn,
        Vn | An => MrpTxAction::SendNew,
        Qa => MrpTxAction::SendJoinInOptional,
        La => MrpTxAction::SendLv,
        Invalid => MrpTxAction::None,
    }
}

pub fn mrp_attrvalue_inc(value: &mut [u8]) {
    let mut len = value.len();
    while len > 0 {
        len -= 1;
        value[len] = value[len].wrapping_add(1);
        if value[len] != 0 {
            break;
        }
    }
}

pub const fn mrp_attr_initial_state() -> MrpApplicantState {
    MrpApplicantState::Vo
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mrp_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/802/mrp.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/net/mrp.h"
        ));
        assert!(source.contains("static unsigned int mrp_join_time __read_mostly = 200;"));
        assert!(source.contains("static unsigned int mrp_periodic_time __read_mostly = 1000;"));
        assert!(source.contains("mrp_applicant_state_table"));
        assert!(source.contains("[MRP_APPLICANT_VO]"));
        assert!(source.contains("[MRP_EVENT_JOIN]\t= MRP_APPLICANT_VP"));
        assert!(source.contains("mrp_tx_action_table"));
        assert!(source.contains("[MRP_APPLICANT_LA] = MRP_TX_ACTION_S_LV"));
        assert!(source.contains("while (len > 0 && !++v[--len])"));
        assert!(source.contains("attr->state = MRP_APPLICANT_VO;"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(mrp_request_join);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(mrp_unregister_application);"));
        assert!(header.contains("#define MRP_END_MARK\t\t0x0"));
        assert!(header.contains("enum mrp_applicant_state"));
    }

    #[test]
    fn mrp_transitions_and_increment_follow_linux() {
        assert_eq!(mrp_attr_initial_state(), MrpApplicantState::Vo);
        assert_eq!(
            mrp_transition(MrpApplicantState::Vo, MrpEvent::Join),
            MrpApplicantState::Vp
        );
        assert_eq!(
            mrp_transition(MrpApplicantState::Vp, MrpEvent::Tx),
            MrpApplicantState::Aa
        );
        assert_eq!(mrp_tx_action(MrpApplicantState::La), MrpTxAction::SendLv);
        assert_eq!(
            mrp_tx_action(MrpApplicantState::Qp),
            MrpTxAction::SendInOptional
        );
        let mut value = [0x12, 0xff, 0xff];
        mrp_attrvalue_inc(&mut value);
        assert_eq!(value, [0x13, 0, 0]);
    }
}
