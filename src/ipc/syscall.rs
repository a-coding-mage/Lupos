//! linux-parity: complete
//! linux-source: vendor/linux/ipc/syscall.c
//! test-origin: linux:vendor/linux/ipc/syscall.c
//! Legacy `sys_ipc` demultiplexer command decoding.

use crate::include::uapi::errno::{EFAULT, EINVAL, ENOSYS};

pub const SEMOP: u32 = 1;
pub const SEMGET: u32 = 2;
pub const SEMCTL: u32 = 3;
pub const SEMTIMEDOP: u32 = 4;
pub const MSGSND: u32 = 11;
pub const MSGRCV: u32 = 12;
pub const MSGGET: u32 = 13;
pub const MSGCTL: u32 = 14;
pub const SHMAT: u32 = 21;
pub const SHMDT: u32 = 22;
pub const SHMGET: u32 = 23;
pub const SHMCTL: u32 = 24;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IpcRoute {
    SemTimedOp,
    SemGet,
    SemCtl,
    MsgSnd,
    MsgRcvKludge,
    MsgRcvDirect,
    MsgGet,
    MsgCtl,
    ShmAt,
    ShmAtVersion1Invalid,
    ShmDt,
    ShmGet,
    ShmCtl,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IpcTimeArg {
    None,
    KernelTimespec(u64),
    CompatOldTimespec32(u64),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpcKludge {
    pub msgp: u64,
    pub msgtyp: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpcSyscallEnv {
    pub is_64bit: bool,
    pub compat_32bit_time: bool,
    pub semctl_arg: Result<u64, i32>,
    pub msg_rcv_kludge: Result<IpcKludge, i32>,
    pub do_shmat_ret: i32,
    pub do_shmat_raddr: u64,
    pub put_user_ret: i32,
}

impl IpcSyscallEnv {
    pub const SUCCESS_64: Self = Self {
        is_64bit: true,
        compat_32bit_time: true,
        semctl_arg: Ok(0),
        msg_rcv_kludge: Ok(IpcKludge { msgp: 0, msgtyp: 0 }),
        do_shmat_ret: 0,
        do_shmat_raddr: 0,
        put_user_ret: 0,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpcDispatch {
    pub route: IpcRoute,
    pub first: i32,
    pub second: u64,
    pub third: u64,
    pub ptr: Option<u64>,
    pub fifth: i64,
    pub version: u32,
    pub semctl_arg: Option<u64>,
    pub msgtyp: Option<i64>,
    pub time_arg: IpcTimeArg,
    pub shmat_addr_written: Option<u64>,
    pub compat: bool,
}

pub const fn ipc_version(call: u32) -> u32 {
    call >> 16
}

pub const fn ipc_command(call: u32) -> u32 {
    call & 0xffff
}

pub const fn ksys_ipc_route(call: u32) -> IpcRoute {
    let version = ipc_version(call);
    match ipc_command(call) {
        SEMOP | SEMTIMEDOP => IpcRoute::SemTimedOp,
        SEMGET => IpcRoute::SemGet,
        SEMCTL => IpcRoute::SemCtl,
        MSGSND => IpcRoute::MsgSnd,
        MSGRCV if version == 0 => IpcRoute::MsgRcvKludge,
        MSGRCV => IpcRoute::MsgRcvDirect,
        MSGGET => IpcRoute::MsgGet,
        MSGCTL => IpcRoute::MsgCtl,
        SHMAT if version == 1 => IpcRoute::ShmAtVersion1Invalid,
        SHMAT => IpcRoute::ShmAt,
        SHMDT => IpcRoute::ShmDt,
        SHMGET => IpcRoute::ShmGet,
        SHMCTL => IpcRoute::ShmCtl,
        _ => IpcRoute::Unsupported,
    }
}

pub fn ksys_ipc(
    call: u32,
    first: i32,
    second: u64,
    third: u64,
    ptr: Option<u64>,
    fifth: i64,
    env: IpcSyscallEnv,
) -> Result<IpcDispatch, i32> {
    let version = ipc_version(call);
    let call = ipc_command(call);
    let base = |route, semctl_arg, msgtyp, time_arg, shmat_addr_written| IpcDispatch {
        route,
        first,
        second,
        third,
        ptr,
        fifth,
        version,
        semctl_arg,
        msgtyp,
        time_arg,
        shmat_addr_written,
        compat: false,
    };

    match call {
        SEMOP => Ok(base(
            IpcRoute::SemTimedOp,
            None,
            None,
            IpcTimeArg::None,
            None,
        )),
        SEMTIMEDOP => {
            if env.is_64bit {
                Ok(base(
                    IpcRoute::SemTimedOp,
                    None,
                    None,
                    IpcTimeArg::KernelTimespec(fifth as u64),
                    None,
                ))
            } else if env.compat_32bit_time {
                Ok(base(
                    IpcRoute::SemTimedOp,
                    None,
                    None,
                    IpcTimeArg::CompatOldTimespec32(fifth as u64),
                    None,
                ))
            } else {
                Err(-ENOSYS)
            }
        }
        SEMGET => Ok(base(IpcRoute::SemGet, None, None, IpcTimeArg::None, None)),
        SEMCTL => {
            if ptr.is_none() {
                return Err(-EINVAL);
            }
            let arg = env.semctl_arg.map_err(|_| -EFAULT)?;
            Ok(base(
                IpcRoute::SemCtl,
                Some(arg),
                None,
                IpcTimeArg::None,
                None,
            ))
        }
        MSGSND => Ok(base(IpcRoute::MsgSnd, None, None, IpcTimeArg::None, None)),
        MSGRCV if version == 0 => {
            if ptr.is_none() {
                return Err(-EINVAL);
            }
            let tmp = env.msg_rcv_kludge.map_err(|_| -EFAULT)?;
            Ok(IpcDispatch {
                ptr: Some(tmp.msgp),
                msgtyp: Some(tmp.msgtyp),
                ..base(IpcRoute::MsgRcvKludge, None, None, IpcTimeArg::None, None)
            })
        }
        MSGRCV => Ok(base(
            IpcRoute::MsgRcvDirect,
            None,
            Some(fifth),
            IpcTimeArg::None,
            None,
        )),
        MSGGET => Ok(base(IpcRoute::MsgGet, None, None, IpcTimeArg::None, None)),
        MSGCTL => Ok(base(IpcRoute::MsgCtl, None, None, IpcTimeArg::None, None)),
        SHMAT if version == 1 => Err(-EINVAL),
        SHMAT => {
            if env.do_shmat_ret != 0 {
                return Err(env.do_shmat_ret);
            }
            if env.put_user_ret != 0 {
                return Err(env.put_user_ret);
            }
            Ok(base(
                IpcRoute::ShmAt,
                None,
                None,
                IpcTimeArg::None,
                Some(env.do_shmat_raddr),
            ))
        }
        SHMDT => Ok(base(IpcRoute::ShmDt, None, None, IpcTimeArg::None, None)),
        SHMGET => Ok(base(IpcRoute::ShmGet, None, None, IpcTimeArg::None, None)),
        SHMCTL => Ok(base(IpcRoute::ShmCtl, None, None, IpcTimeArg::None, None)),
        _ => Err(-ENOSYS),
    }
}

pub fn sys_ipc(
    call: u32,
    first: i32,
    second: u64,
    third: u64,
    ptr: Option<u64>,
    fifth: i64,
    env: IpcSyscallEnv,
) -> Result<IpcDispatch, i32> {
    ksys_ipc(call, first, second, third, ptr, fifth, env)
}

pub fn compat_ksys_ipc(
    call: u32,
    first: i32,
    second: i32,
    third: u32,
    ptr: Option<u32>,
    fifth: u32,
    env: IpcSyscallEnv,
) -> Result<IpcDispatch, i32> {
    let version = ipc_version(call);
    let call = ipc_command(call);
    let ptr64 = ptr.map(u64::from);
    let base = |route, semctl_arg, msgtyp, time_arg, shmat_addr_written| IpcDispatch {
        route,
        first,
        second: second as u64,
        third: third as u64,
        ptr: ptr64,
        fifth: fifth as i64,
        version,
        semctl_arg,
        msgtyp,
        time_arg,
        shmat_addr_written,
        compat: true,
    };

    match call {
        SEMOP => Ok(base(
            IpcRoute::SemTimedOp,
            None,
            None,
            IpcTimeArg::None,
            None,
        )),
        SEMTIMEDOP => {
            if !env.compat_32bit_time {
                Err(-ENOSYS)
            } else {
                Ok(base(
                    IpcRoute::SemTimedOp,
                    None,
                    None,
                    IpcTimeArg::CompatOldTimespec32(fifth as u64),
                    None,
                ))
            }
        }
        SEMGET => Ok(base(IpcRoute::SemGet, None, None, IpcTimeArg::None, None)),
        SEMCTL => {
            if ptr.is_none() {
                return Err(-EINVAL);
            }
            let arg = env.semctl_arg.map_err(|_| -EFAULT)?;
            Ok(base(
                IpcRoute::SemCtl,
                Some(arg),
                None,
                IpcTimeArg::None,
                None,
            ))
        }
        MSGSND => Ok(base(IpcRoute::MsgSnd, None, None, IpcTimeArg::None, None)),
        MSGRCV => {
            if first < 0 || second < 0 {
                return Err(-EINVAL);
            }
            if version == 0 {
                let uptr = ptr.ok_or(-EINVAL)?;
                let tmp = env.msg_rcv_kludge.map_err(|_| -EFAULT)?;
                Ok(IpcDispatch {
                    ptr: Some(u64::from(uptr)),
                    msgtyp: Some(tmp.msgtyp),
                    ..base(IpcRoute::MsgRcvKludge, None, None, IpcTimeArg::None, None)
                })
            } else {
                Ok(base(
                    IpcRoute::MsgRcvDirect,
                    None,
                    Some(fifth as i64),
                    IpcTimeArg::None,
                    None,
                ))
            }
        }
        MSGGET => Ok(base(IpcRoute::MsgGet, None, None, IpcTimeArg::None, None)),
        MSGCTL => Ok(base(IpcRoute::MsgCtl, None, None, IpcTimeArg::None, None)),
        SHMAT if version == 1 => Err(-EINVAL),
        SHMAT => {
            if env.do_shmat_ret < 0 {
                return Err(env.do_shmat_ret);
            }
            if env.put_user_ret != 0 {
                return Err(env.put_user_ret);
            }
            Ok(base(
                IpcRoute::ShmAt,
                None,
                None,
                IpcTimeArg::None,
                Some(env.do_shmat_raddr),
            ))
        }
        SHMDT => Ok(base(IpcRoute::ShmDt, None, None, IpcTimeArg::None, None)),
        SHMGET => Ok(base(IpcRoute::ShmGet, None, None, IpcTimeArg::None, None)),
        SHMCTL => Ok(base(IpcRoute::ShmCtl, None, None, IpcTimeArg::None, None)),
        _ => Err(-ENOSYS),
    }
}

pub fn compat_sys_ipc(
    call: u32,
    first: i32,
    second: i32,
    third: u32,
    ptr: Option<u32>,
    fifth: u32,
    env: IpcSyscallEnv,
) -> Result<IpcDispatch, i32> {
    compat_ksys_ipc(call, first, second, third, ptr, fifth, env)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_sys_ipc_demux_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/ipc/syscall.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/ipc.h"
        ));
        assert!(source.contains("version = call >> 16;"));
        assert!(source.contains("call &= 0xffff;"));
        assert!(source.contains("case SEMOP:"));
        assert!(source.contains("case SEMTIMEDOP:"));
        assert!(source.contains("IS_ENABLED(CONFIG_64BIT)"));
        assert!(source.contains("IS_ENABLED(CONFIG_COMPAT_32BIT_TIME)"));
        assert!(source.contains("if (!ptr)"));
        assert!(source.contains("if (get_user(arg, (unsigned long __user *) ptr))"));
        assert!(source.contains("return -EFAULT;"));
        assert!(source.contains("case MSGRCV:"));
        assert!(source.contains("struct ipc_kludge tmp;"));
        assert!(source.contains("copy_from_user(&tmp"));
        assert!(source.contains("case SHMAT:"));
        assert!(source.contains("ret = do_shmat(first, (char __user *)ptr"));
        assert!(source.contains("return put_user(raddr, (unsigned long __user *) third);"));
        assert!(source.contains("return -ENOSYS;"));
        assert!(source.contains("SYSCALL_DEFINE6(ipc"));
        assert!(source.contains("int compat_ksys_ipc"));
        assert!(source.contains("struct compat_ipc_kludge"));
        assert!(source.contains("if (first < 0 || second < 0)"));
        assert!(source.contains("COMPAT_SYSCALL_DEFINE6(ipc"));
        assert!(header.contains("#define SEMOP\t\t 1"));
        assert!(header.contains("#define MSGRCV\t\t12"));
        assert!(header.contains("#define SHMCTL\t\t24"));

        assert_eq!(ipc_version((3 << 16) | MSGRCV), 3);
        assert_eq!(ipc_command((3 << 16) | MSGRCV), MSGRCV);
        assert_eq!(ksys_ipc_route(MSGRCV), IpcRoute::MsgRcvKludge);
        assert_eq!(ksys_ipc_route((2 << 16) | MSGRCV), IpcRoute::MsgRcvDirect);
        assert_eq!(
            ksys_ipc_route((1 << 16) | SHMAT),
            IpcRoute::ShmAtVersion1Invalid
        );
        assert_eq!(ksys_ipc_route(0xffff), IpcRoute::Unsupported);
    }

    #[test]
    fn native_ksys_ipc_dispatches_linux_cases_and_errors() {
        let env = IpcSyscallEnv {
            semctl_arg: Ok(0xabc),
            msg_rcv_kludge: Ok(IpcKludge {
                msgp: 0xfeed,
                msgtyp: -3,
            }),
            do_shmat_raddr: 0x7000,
            ..IpcSyscallEnv::SUCCESS_64
        };

        assert_eq!(
            ksys_ipc(SEMOP, 1, 2, 3, Some(4), 5, env).unwrap().route,
            IpcRoute::SemTimedOp
        );
        assert_eq!(
            ksys_ipc(SEMTIMEDOP, 1, 2, 3, Some(4), 0x55, env)
                .unwrap()
                .time_arg,
            IpcTimeArg::KernelTimespec(0x55)
        );
        assert_eq!(
            ksys_ipc(
                SEMTIMEDOP,
                1,
                2,
                3,
                Some(4),
                0x55,
                IpcSyscallEnv {
                    is_64bit: false,
                    compat_32bit_time: true,
                    ..env
                },
            )
            .unwrap()
            .time_arg,
            IpcTimeArg::CompatOldTimespec32(0x55)
        );
        assert_eq!(
            ksys_ipc(
                SEMTIMEDOP,
                1,
                2,
                3,
                Some(4),
                0x55,
                IpcSyscallEnv {
                    is_64bit: false,
                    compat_32bit_time: false,
                    ..env
                },
            ),
            Err(-ENOSYS)
        );

        assert_eq!(ksys_ipc(SEMCTL, 1, 2, 3, None, 0, env), Err(-EINVAL));
        assert_eq!(
            ksys_ipc(
                SEMCTL,
                1,
                2,
                3,
                Some(4),
                0,
                IpcSyscallEnv {
                    semctl_arg: Err(-EFAULT),
                    ..env
                },
            ),
            Err(-EFAULT)
        );
        assert_eq!(
            ksys_ipc(SEMCTL, 1, 2, 3, Some(4), 0, env)
                .unwrap()
                .semctl_arg,
            Some(0xabc)
        );

        let kludge = ksys_ipc(MSGRCV, 1, 2, 3, Some(4), 9, env).unwrap();
        assert_eq!(kludge.route, IpcRoute::MsgRcvKludge);
        assert_eq!(kludge.ptr, Some(0xfeed));
        assert_eq!(kludge.msgtyp, Some(-3));
        assert_eq!(ksys_ipc(MSGRCV, 1, 2, 3, None, 9, env), Err(-EINVAL));
        assert_eq!(
            ksys_ipc(
                MSGRCV,
                1,
                2,
                3,
                Some(4),
                9,
                IpcSyscallEnv {
                    msg_rcv_kludge: Err(-EFAULT),
                    ..env
                },
            ),
            Err(-EFAULT)
        );
        let direct = ksys_ipc((2 << 16) | MSGRCV, 1, 2, 3, Some(4), 9, env).unwrap();
        assert_eq!(direct.route, IpcRoute::MsgRcvDirect);
        assert_eq!(direct.msgtyp, Some(9));

        assert_eq!(
            ksys_ipc((1 << 16) | SHMAT, 1, 2, 3, Some(4), 5, env),
            Err(-EINVAL)
        );
        assert_eq!(
            ksys_ipc(
                SHMAT,
                1,
                2,
                3,
                Some(4),
                5,
                IpcSyscallEnv {
                    do_shmat_ret: -EFAULT,
                    ..env
                },
            ),
            Err(-EFAULT)
        );
        assert_eq!(
            ksys_ipc(
                SHMAT,
                1,
                2,
                3,
                Some(4),
                5,
                IpcSyscallEnv {
                    put_user_ret: -EFAULT,
                    ..env
                },
            ),
            Err(-EFAULT)
        );
        assert_eq!(
            sys_ipc(SHMAT, 1, 2, 3, Some(4), 5, env)
                .unwrap()
                .shmat_addr_written,
            Some(0x7000)
        );
        assert_eq!(ksys_ipc(0xffff, 1, 2, 3, Some(4), 5, env), Err(-ENOSYS));
    }

    #[test]
    fn compat_ksys_ipc_matches_compat_linux_edges() {
        let env = IpcSyscallEnv {
            semctl_arg: Ok(0x1234),
            msg_rcv_kludge: Ok(IpcKludge {
                msgp: 0x5678,
                msgtyp: 11,
            }),
            do_shmat_raddr: 0x9000,
            ..IpcSyscallEnv::SUCCESS_64
        };

        assert_eq!(
            compat_ksys_ipc(SEMTIMEDOP, 1, 2, 3, Some(4), 0x66, env)
                .unwrap()
                .time_arg,
            IpcTimeArg::CompatOldTimespec32(0x66)
        );
        assert_eq!(
            compat_ksys_ipc(
                SEMTIMEDOP,
                1,
                2,
                3,
                Some(4),
                0x66,
                IpcSyscallEnv {
                    compat_32bit_time: false,
                    ..env
                },
            ),
            Err(-ENOSYS)
        );

        assert_eq!(compat_ksys_ipc(SEMCTL, 1, 2, 3, None, 0, env), Err(-EINVAL));
        assert_eq!(
            compat_ksys_ipc(SEMCTL, 1, 2, 3, Some(4), 0, env)
                .unwrap()
                .semctl_arg,
            Some(0x1234)
        );
        assert_eq!(
            compat_ksys_ipc(MSGRCV, -1, 2, 3, Some(4), 5, env),
            Err(-EINVAL)
        );
        assert_eq!(
            compat_ksys_ipc(MSGRCV, 1, -1, 3, Some(4), 5, env),
            Err(-EINVAL)
        );
        assert_eq!(compat_ksys_ipc(MSGRCV, 1, 2, 3, None, 5, env), Err(-EINVAL));
        let kludge = compat_ksys_ipc(MSGRCV, 1, 2, 3, Some(4), 5, env).unwrap();
        assert!(kludge.compat);
        assert_eq!(kludge.route, IpcRoute::MsgRcvKludge);
        assert_eq!(kludge.ptr, Some(4));
        assert_eq!(kludge.msgtyp, Some(11));
        let direct = compat_sys_ipc((2 << 16) | MSGRCV, 1, 2, 3, Some(4), 5, env).unwrap();
        assert_eq!(direct.route, IpcRoute::MsgRcvDirect);
        assert_eq!(direct.msgtyp, Some(5));

        assert_eq!(
            compat_ksys_ipc((1 << 16) | SHMAT, 1, 2, 3, Some(4), 5, env),
            Err(-EINVAL)
        );
        assert_eq!(
            compat_ksys_ipc(
                SHMAT,
                1,
                2,
                3,
                Some(4),
                5,
                IpcSyscallEnv {
                    do_shmat_ret: -EFAULT,
                    ..env
                },
            ),
            Err(-EFAULT)
        );
        assert_eq!(
            compat_ksys_ipc(SHMAT, 1, 2, 3, Some(4), 5, env)
                .unwrap()
                .shmat_addr_written,
            Some(0x9000)
        );
        assert_eq!(
            compat_ksys_ipc(0xffff, 1, 2, 3, Some(4), 5, env),
            Err(-ENOSYS)
        );
    }
}
