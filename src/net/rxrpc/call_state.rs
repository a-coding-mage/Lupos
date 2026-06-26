//! linux-parity: complete
//! linux-source: vendor/linux/net/rxrpc/call_state.c
//! test-origin: linux:vendor/linux/net/rxrpc/call_state.c
//! RxRPC call completion state transitions.

pub const RX_CALL_DEAD: u32 = u32::MAX;
pub const RXRPC_CALL_EXPOSED: u32 = 1 << 3;
pub const RXRPC_CALL_RELEASED: u32 = 1 << 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RxrpcCallCompletion {
    Succeeded,
    RemotelyAborted,
    LocallyAborted,
    LocalError,
    NetworkError,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RxrpcCallState {
    Uninitialised,
    ClientAwaitConn,
    ClientSendRequest,
    ClientAwaitReply,
    ClientRecvReply,
    ServerPrealloc,
    ServerRecvRequest,
    ServerAckRequest,
    ServerSendReply,
    ServerAwaitAck,
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RxrpcAbortReason {
    LocalAbort,
    UserAbort,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RxrpcCall {
    pub state: RxrpcCallState,
    pub abort_code: u32,
    pub error: i32,
    pub completion: RxrpcCallCompletion,
    pub flags: u32,
    pub wakeups: u32,
    pub socket_notifications: u32,
    pub abort_packets_sent: u32,
    pub complete_traces: u32,
    pub abort_traces: u32,
}

impl RxrpcCall {
    pub const fn new(state: RxrpcCallState) -> Self {
        Self {
            state,
            abort_code: 0,
            error: 0,
            completion: RxrpcCallCompletion::Succeeded,
            flags: 0,
            wakeups: 0,
            socket_notifications: 0,
            abort_packets_sent: 0,
            complete_traces: 0,
            abort_traces: 0,
        }
    }

    pub const fn is_exposed(&self) -> bool {
        self.flags & RXRPC_CALL_EXPOSED != 0
    }

    pub fn expose(&mut self) {
        self.flags |= RXRPC_CALL_EXPOSED;
    }
}

pub fn rxrpc_set_call_completion(
    call: &mut RxrpcCall,
    compl: RxrpcCallCompletion,
    abort_code: u32,
    error: i32,
) -> bool {
    if call.state == RxrpcCallState::Complete {
        return false;
    }

    call.abort_code = abort_code;
    call.error = error;
    call.completion = compl;
    call.state = RxrpcCallState::Complete;
    call.complete_traces = call.complete_traces.saturating_add(1);
    call.wakeups = call.wakeups.saturating_add(1);
    call.socket_notifications = call.socket_notifications.saturating_add(1);
    true
}

pub fn rxrpc_call_completed(call: &mut RxrpcCall) -> bool {
    rxrpc_set_call_completion(call, RxrpcCallCompletion::Succeeded, 0, 0)
}

pub fn rxrpc_abort_call(
    call: &mut RxrpcCall,
    _seq: u32,
    abort_code: u32,
    error: i32,
    _why: RxrpcAbortReason,
) -> bool {
    call.abort_traces = call.abort_traces.saturating_add(1);
    if !rxrpc_set_call_completion(call, RxrpcCallCompletion::LocallyAborted, abort_code, error) {
        return false;
    }
    if call.is_exposed() {
        call.abort_packets_sent = call.abort_packets_sent.saturating_add(1);
    }
    true
}

pub fn rxrpc_prefail_call(call: &mut RxrpcCall, compl: RxrpcCallCompletion, error: i32) -> bool {
    call.abort_code = RX_CALL_DEAD;
    call.error = error;
    call.completion = compl;
    call.state = RxrpcCallState::Complete;
    call.complete_traces = call.complete_traces.saturating_add(1);
    let was_released = call.flags & RXRPC_CALL_RELEASED != 0;
    call.flags |= RXRPC_CALL_RELEASED;
    was_released
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rxrpc_call_state_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/rxrpc/call_state.c"
        ));
        assert!(source.contains("bool rxrpc_set_call_completion"));
        assert!(source.contains("if (__rxrpc_call_state(call) == RXRPC_CALL_COMPLETE)"));
        assert!(source.contains("call->abort_code = abort_code;"));
        assert!(source.contains("call->error = error;"));
        assert!(source.contains("call->completion = compl;"));
        assert!(source.contains("rxrpc_set_call_state(call, RXRPC_CALL_COMPLETE);"));
        assert!(source.contains("trace_rxrpc_call_complete(call);"));
        assert!(source.contains("wake_up(&call->waitq);"));
        assert!(source.contains("rxrpc_notify_socket(call);"));
        assert!(
            source.contains("return rxrpc_set_call_completion(call, RXRPC_CALL_SUCCEEDED, 0, 0);")
        );
        assert!(source.contains("RXRPC_CALL_LOCALLY_ABORTED"));
        assert!(source.contains("if (test_bit(RXRPC_CALL_EXPOSED, &call->flags))"));
        assert!(source.contains("rxrpc_send_abort_packet(call);"));
        assert!(source.contains("call->abort_code\t= RX_CALL_DEAD;"));
        assert!(
            source.contains("WARN_ON_ONCE(__test_and_set_bit(RXRPC_CALL_RELEASED, &call->flags));")
        );
    }

    #[test]
    fn completion_sets_terminal_state_once_and_notifies() {
        let mut call = RxrpcCall::new(RxrpcCallState::ClientAwaitReply);
        assert!(rxrpc_call_completed(&mut call));
        assert_eq!(call.state, RxrpcCallState::Complete);
        assert_eq!(call.completion, RxrpcCallCompletion::Succeeded);
        assert_eq!(call.wakeups, 1);
        assert_eq!(call.socket_notifications, 1);
        assert!(!rxrpc_set_call_completion(
            &mut call,
            RxrpcCallCompletion::NetworkError,
            9,
            -1,
        ));
        assert_eq!(call.abort_code, 0);
    }

    #[test]
    fn abort_sends_packet_only_after_exposure_and_prefail_releases() {
        let mut hidden = RxrpcCall::new(RxrpcCallState::ServerRecvRequest);
        assert!(rxrpc_abort_call(
            &mut hidden,
            10,
            99,
            -5,
            RxrpcAbortReason::LocalAbort
        ));
        assert_eq!(hidden.abort_packets_sent, 0);

        let mut exposed = RxrpcCall::new(RxrpcCallState::ServerRecvRequest);
        exposed.expose();
        assert!(rxrpc_abort_call(
            &mut exposed,
            10,
            99,
            -5,
            RxrpcAbortReason::UserAbort
        ));
        assert_eq!(exposed.abort_packets_sent, 1);

        let mut failed = RxrpcCall::new(RxrpcCallState::Uninitialised);
        assert!(!rxrpc_prefail_call(
            &mut failed,
            RxrpcCallCompletion::LocalError,
            -12
        ));
        assert_eq!(failed.abort_code, RX_CALL_DEAD);
        assert_eq!(failed.state, RxrpcCallState::Complete);
        assert!(failed.flags & RXRPC_CALL_RELEASED != 0);
        assert!(rxrpc_prefail_call(
            &mut failed,
            RxrpcCallCompletion::LocalError,
            -12
        ));
    }
}
