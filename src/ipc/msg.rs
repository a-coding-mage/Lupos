//! linux-parity: partial
//! linux-source: vendor/linux/ipc/msg.c
//! test-origin: linux:vendor/linux/ipc/msg.c
//! System V message queue search, control, and namespace defaults.

pub const SEARCH_ANY: i32 = 1;
pub const SEARCH_EQUAL: i32 = 2;
pub const SEARCH_NOTEQUAL: i32 = 3;
pub const SEARCH_LESSEQUAL: i32 = 4;
pub const SEARCH_NUMBER: i32 = 5;

pub const IPC_NOWAIT: i32 = 0o4000;
pub const IPC_RMID: i32 = 0;
pub const IPC_SET: i32 = 1;
pub const IPC_STAT: i32 = 2;
pub const IPC_INFO: i32 = 3;
pub const IPC_64: i32 = 0x0100;
pub const MSG_STAT: i32 = 11;
pub const MSG_INFO: i32 = 12;
pub const MSG_STAT_ANY: i32 = 13;
pub const MSG_NOERROR: i32 = 0o10000;
pub const MSG_EXCEPT: i32 = 0o20000;
pub const MSG_COPY: i32 = 0o40000;
pub const MSGMNI: usize = 32_000;
pub const MSGMAX: usize = 8192;
pub const MSGMNB: usize = 16_384;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MsgQueueState {
    pub q_cbytes: usize,
    pub q_qnum: usize,
    pub q_qbytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MsgNamespaceDefaults {
    pub msg_ctlmax: usize,
    pub msg_ctlmnb: usize,
    pub msg_ctlmni: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MsgCtlRoute {
    Info,
    StatByIndex,
    StatById,
    Set,
    Remove,
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReceiveMode {
    pub msgtyp: i64,
    pub mode: i32,
}

pub const fn msg_fits_inqueue(queue: MsgQueueState, msgsz: usize) -> bool {
    msgsz + queue.q_cbytes <= queue.q_qbytes && 1 + queue.q_qnum <= queue.q_qbytes
}

pub const fn testmsg(message_type: i64, requested_type: i64, mode: i32) -> bool {
    match mode {
        SEARCH_ANY | SEARCH_NUMBER => true,
        SEARCH_LESSEQUAL => message_type <= requested_type,
        SEARCH_EQUAL => message_type == requested_type,
        SEARCH_NOTEQUAL => message_type != requested_type,
        _ => false,
    }
}

pub const fn convert_mode(msgtyp: i64, msgflg: i32) -> ReceiveMode {
    if (msgflg & MSG_COPY) != 0 {
        return ReceiveMode {
            msgtyp,
            mode: SEARCH_NUMBER,
        };
    }
    if msgtyp == 0 {
        ReceiveMode {
            msgtyp,
            mode: SEARCH_ANY,
        }
    } else if msgtyp < 0 {
        ReceiveMode {
            msgtyp: if msgtyp == i64::MIN {
                i64::MAX
            } else {
                -msgtyp
            },
            mode: SEARCH_LESSEQUAL,
        }
    } else if (msgflg & MSG_EXCEPT) != 0 {
        ReceiveMode {
            msgtyp,
            mode: SEARCH_NOTEQUAL,
        }
    } else {
        ReceiveMode {
            msgtyp,
            mode: SEARCH_EQUAL,
        }
    }
}

pub const fn msg_copy_flags_valid(msgflg: i32) -> bool {
    if (msgflg & MSG_COPY) == 0 {
        true
    } else {
        (msgflg & MSG_EXCEPT) == 0 && (msgflg & IPC_NOWAIT) != 0
    }
}

pub const fn msgctl_route(msqid: i32, cmd: i32) -> MsgCtlRoute {
    if msqid < 0 || cmd < 0 {
        return MsgCtlRoute::Invalid;
    }
    match cmd {
        IPC_INFO | MSG_INFO => MsgCtlRoute::Info,
        MSG_STAT | MSG_STAT_ANY => MsgCtlRoute::StatByIndex,
        IPC_STAT => MsgCtlRoute::StatById,
        IPC_SET => MsgCtlRoute::Set,
        IPC_RMID => MsgCtlRoute::Remove,
        _ => MsgCtlRoute::Invalid,
    }
}

pub const fn msg_init_ns_defaults() -> MsgNamespaceDefaults {
    MsgNamespaceDefaults {
        msg_ctlmax: MSGMAX,
        msg_ctlmnb: MSGMNB,
        msg_ctlmni: MSGMNI,
    }
}

pub const fn receive_result_size(message_size: usize, buffer_size: usize) -> usize {
    if buffer_size > message_size {
        message_size
    } else {
        buffer_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sysv_msg_queue_rules_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/ipc/msg.c"
        ));
        let msg_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/msg.h"
        ));
        let ipc_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/ipc.h"
        ));
        assert!(source.contains("#define SEARCH_ANY\t\t1"));
        assert!(source.contains("#define SEARCH_EQUAL\t\t2"));
        assert!(source.contains("#define SEARCH_NOTEQUAL\t\t3"));
        assert!(source.contains("#define SEARCH_LESSEQUAL\t4"));
        assert!(source.contains("#define SEARCH_NUMBER\t\t5"));
        assert!(source.contains("msgsz + msq->q_cbytes <= msq->q_qbytes"));
        assert!(source.contains("1 + msq->q_qnum <= msq->q_qbytes"));
        assert!(source.contains("static inline int convert_mode(long *msgtyp, int msgflg)"));
        assert!(source.contains("if (msgflg & MSG_COPY)"));
        assert!(source.contains("if (*msgtyp == LONG_MIN)"));
        assert!(source.contains("if (msgflg & MSG_EXCEPT)"));
        assert!(source.contains("static int testmsg(struct msg_msg *msg, long type, int mode)"));
        assert!(source.contains("case SEARCH_LESSEQUAL:"));
        assert!(source.contains("case IPC_INFO:"));
        assert!(source.contains("case MSG_INFO:"));
        assert!(source.contains("case MSG_STAT_ANY:"));
        assert!(source.contains("case IPC_SET:"));
        assert!(source.contains("case IPC_RMID:"));
        assert!(source.contains("if ((msgflg & MSG_EXCEPT) || !(msgflg & IPC_NOWAIT))"));
        assert!(source.contains("ns->msg_ctlmax = MSGMAX;"));
        assert!(source.contains("ns->msg_ctlmnb = MSGMNB;"));
        assert!(source.contains("ns->msg_ctlmni = MSGMNI;"));
        assert!(msg_header.contains("#define MSG_COPY        040000"));
        assert!(ipc_header.contains("#define IPC_NOWAIT 00004000"));

        assert!(msg_fits_inqueue(
            MsgQueueState {
                q_cbytes: 10,
                q_qnum: 2,
                q_qbytes: 16,
            },
            6
        ));
        assert!(!msg_fits_inqueue(
            MsgQueueState {
                q_cbytes: 11,
                q_qnum: 2,
                q_qbytes: 16,
            },
            6
        ));
        assert!(testmsg(7, 5, SEARCH_NOTEQUAL));
        assert!(testmsg(4, 5, SEARCH_LESSEQUAL));
        assert!(!testmsg(6, 5, SEARCH_LESSEQUAL));
        assert_eq!(
            convert_mode(-9, 0),
            ReceiveMode {
                msgtyp: 9,
                mode: SEARCH_LESSEQUAL,
            }
        );
        assert_eq!(convert_mode(0, 0).mode, SEARCH_ANY);
        assert_eq!(convert_mode(4, MSG_EXCEPT).mode, SEARCH_NOTEQUAL);
        assert_eq!(convert_mode(3, MSG_COPY).mode, SEARCH_NUMBER);
        assert!(msg_copy_flags_valid(MSG_COPY | IPC_NOWAIT));
        assert!(!msg_copy_flags_valid(MSG_COPY));
        assert!(!msg_copy_flags_valid(MSG_COPY | IPC_NOWAIT | MSG_EXCEPT));
        assert_eq!(msgctl_route(1, MSG_INFO), MsgCtlRoute::Info);
        assert_eq!(msgctl_route(1, MSG_STAT_ANY), MsgCtlRoute::StatByIndex);
        assert_eq!(msgctl_route(1, IPC_STAT), MsgCtlRoute::StatById);
        assert_eq!(msgctl_route(1, IPC_SET), MsgCtlRoute::Set);
        assert_eq!(msgctl_route(1, IPC_RMID), MsgCtlRoute::Remove);
        assert_eq!(msgctl_route(-1, IPC_STAT), MsgCtlRoute::Invalid);
        assert_eq!(msg_init_ns_defaults().msg_ctlmnb, MSGMNB);
        assert_eq!(receive_result_size(12, 8), 8);
        assert_eq!(receive_result_size(12, 16), 12);
    }
}
