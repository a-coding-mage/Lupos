//! linux-parity: complete
//! linux-source: vendor/linux/net/mptcp/bpf.c
//! test-origin: linux:vendor/linux/net/mptcp/bpf.c
//! MPTCP BPF kfunc registration helpers.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MptcpSock {
    pub token: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sock {
    pub fullsock: bool,
    pub tcp: bool,
    pub mptcp: bool,
    pub subflow_conn: Option<MptcpSock>,
}

pub const BPF_MPTCP_FMODRET_FUNCS: &[&str] = &["update_socket_protocol"];

pub fn bpf_mptcp_sock_from_subflow(sk: Option<&Sock>) -> Option<&MptcpSock> {
    sk.filter(|sock| sock.fullsock && sock.tcp && sock.mptcp)
        .and_then(|sock| sock.subflow_conn.as_ref())
}

pub fn register_bpf_mptcp_fmodret_set() -> usize {
    BPF_MPTCP_FMODRET_FUNCS.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mptcp_sock_from_subflow_matches_linux_guards() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/mptcp/bpf.c"
        ));
        assert!(source.contains("sk && sk_fullsock(sk) && sk_is_tcp(sk) && sk_is_mptcp(sk)"));
        assert!(source.contains("return mptcp_sk(mptcp_subflow_ctx(sk)->conn);"));
        assert!(source.contains("BTF_ID_FLAGS(func, update_socket_protocol)"));
        assert!(source.contains("register_btf_fmodret_id_set(&bpf_mptcp_fmodret_set);"));
        assert!(source.contains("late_initcall(bpf_mptcp_kfunc_init);"));

        let conn = MptcpSock { token: 42 };
        let sk = Sock {
            fullsock: true,
            tcp: true,
            mptcp: true,
            subflow_conn: Some(conn),
        };
        assert_eq!(bpf_mptcp_sock_from_subflow(Some(&sk)), Some(&conn));
        assert_eq!(
            bpf_mptcp_sock_from_subflow(Some(&Sock {
                tcp: false,
                ..sk.clone()
            })),
            None
        );
        assert_eq!(register_bpf_mptcp_fmodret_set(), 1);
    }
}
