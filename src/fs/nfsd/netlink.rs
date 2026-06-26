//! linux-parity: complete
//! linux-source: vendor/linux/fs/nfsd/netlink.c
//! test-origin: linux:vendor/linux/fs/nfsd/netlink.c
//! NFSD generic-netlink family description.

pub const NFSD_FAMILY_NAME: &str = "nfsd";
pub const NFSD_FAMILY_VERSION: u8 = 1;

pub const NFSD_A_SOCK_ADDR: u16 = 1;
pub const NFSD_A_SOCK_TRANSPORT_NAME: u16 = 2;
pub const NFSD_A_VERSION_MAJOR: u16 = 1;
pub const NFSD_A_VERSION_MINOR: u16 = 2;
pub const NFSD_A_VERSION_ENABLED: u16 = 3;
pub const NFSD_A_SERVER_THREADS: u16 = 1;
pub const NFSD_A_SERVER_GRACETIME: u16 = 2;
pub const NFSD_A_SERVER_LEASETIME: u16 = 3;
pub const NFSD_A_SERVER_SCOPE: u16 = 4;
pub const NFSD_A_SERVER_MIN_THREADS: u16 = 5;
pub const NFSD_A_SERVER_FH_KEY: u16 = 6;
pub const NFSD_A_SERVER_PROTO_VERSION: u16 = 1;
pub const NFSD_A_SERVER_SOCK_ADDR: u16 = 1;
pub const NFSD_A_POOL_MODE_MODE: u16 = 1;

pub const NFSD_CMD_RPC_STATUS_GET: u8 = 1;
pub const NFSD_CMD_THREADS_SET: u8 = 2;
pub const NFSD_CMD_THREADS_GET: u8 = 3;
pub const NFSD_CMD_VERSION_SET: u8 = 4;
pub const NFSD_CMD_VERSION_GET: u8 = 5;
pub const NFSD_CMD_LISTENER_SET: u8 = 6;
pub const NFSD_CMD_LISTENER_GET: u8 = 7;
pub const NFSD_CMD_POOL_MODE_SET: u8 = 8;
pub const NFSD_CMD_POOL_MODE_GET: u8 = 9;

pub const GENL_ADMIN_PERM: u16 = 1 << 0;
pub const GENL_CMD_CAP_DO: u16 = 1 << 1;
pub const GENL_CMD_CAP_DUMP: u16 = 1 << 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NlaPolicyKind {
    Binary,
    NulString,
    U32,
    Flag,
    ExactLen(u16),
    Nested(&'static [NlaPolicy]),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NlaPolicy {
    pub attr: u16,
    pub kind: NlaPolicyKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenlSplitOp {
    pub cmd: u8,
    pub maxattr: Option<u16>,
    pub flags: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenlFamily {
    pub name: &'static str,
    pub version: u8,
    pub netnsok: bool,
    pub parallel_ops: bool,
    pub n_split_ops: usize,
}

pub const NFSD_SOCK_NL_POLICY: &[NlaPolicy] = &[
    NlaPolicy {
        attr: NFSD_A_SOCK_ADDR,
        kind: NlaPolicyKind::Binary,
    },
    NlaPolicy {
        attr: NFSD_A_SOCK_TRANSPORT_NAME,
        kind: NlaPolicyKind::NulString,
    },
];

pub const NFSD_VERSION_NL_POLICY: &[NlaPolicy] = &[
    NlaPolicy {
        attr: NFSD_A_VERSION_MAJOR,
        kind: NlaPolicyKind::U32,
    },
    NlaPolicy {
        attr: NFSD_A_VERSION_MINOR,
        kind: NlaPolicyKind::U32,
    },
    NlaPolicy {
        attr: NFSD_A_VERSION_ENABLED,
        kind: NlaPolicyKind::Flag,
    },
];

pub const NFSD_THREADS_SET_NL_POLICY: &[NlaPolicy] = &[
    NlaPolicy {
        attr: NFSD_A_SERVER_THREADS,
        kind: NlaPolicyKind::U32,
    },
    NlaPolicy {
        attr: NFSD_A_SERVER_GRACETIME,
        kind: NlaPolicyKind::U32,
    },
    NlaPolicy {
        attr: NFSD_A_SERVER_LEASETIME,
        kind: NlaPolicyKind::U32,
    },
    NlaPolicy {
        attr: NFSD_A_SERVER_SCOPE,
        kind: NlaPolicyKind::NulString,
    },
    NlaPolicy {
        attr: NFSD_A_SERVER_MIN_THREADS,
        kind: NlaPolicyKind::U32,
    },
    NlaPolicy {
        attr: NFSD_A_SERVER_FH_KEY,
        kind: NlaPolicyKind::ExactLen(16),
    },
];

pub const NFSD_VERSION_SET_NL_POLICY: &[NlaPolicy] = &[NlaPolicy {
    attr: NFSD_A_SERVER_PROTO_VERSION,
    kind: NlaPolicyKind::Nested(NFSD_VERSION_NL_POLICY),
}];

pub const NFSD_LISTENER_SET_NL_POLICY: &[NlaPolicy] = &[NlaPolicy {
    attr: NFSD_A_SERVER_SOCK_ADDR,
    kind: NlaPolicyKind::Nested(NFSD_SOCK_NL_POLICY),
}];

pub const NFSD_POOL_MODE_SET_NL_POLICY: &[NlaPolicy] = &[NlaPolicy {
    attr: NFSD_A_POOL_MODE_MODE,
    kind: NlaPolicyKind::NulString,
}];

pub const NFSD_NL_OPS: &[GenlSplitOp] = &[
    GenlSplitOp {
        cmd: NFSD_CMD_RPC_STATUS_GET,
        maxattr: None,
        flags: GENL_CMD_CAP_DUMP,
    },
    GenlSplitOp {
        cmd: NFSD_CMD_THREADS_SET,
        maxattr: Some(NFSD_A_SERVER_FH_KEY),
        flags: GENL_ADMIN_PERM | GENL_CMD_CAP_DO,
    },
    GenlSplitOp {
        cmd: NFSD_CMD_THREADS_GET,
        maxattr: None,
        flags: GENL_CMD_CAP_DO,
    },
    GenlSplitOp {
        cmd: NFSD_CMD_VERSION_SET,
        maxattr: Some(NFSD_A_SERVER_PROTO_VERSION),
        flags: GENL_ADMIN_PERM | GENL_CMD_CAP_DO,
    },
    GenlSplitOp {
        cmd: NFSD_CMD_VERSION_GET,
        maxattr: None,
        flags: GENL_CMD_CAP_DO,
    },
    GenlSplitOp {
        cmd: NFSD_CMD_LISTENER_SET,
        maxattr: Some(NFSD_A_SERVER_SOCK_ADDR),
        flags: GENL_ADMIN_PERM | GENL_CMD_CAP_DO,
    },
    GenlSplitOp {
        cmd: NFSD_CMD_LISTENER_GET,
        maxattr: None,
        flags: GENL_CMD_CAP_DO,
    },
    GenlSplitOp {
        cmd: NFSD_CMD_POOL_MODE_SET,
        maxattr: Some(NFSD_A_POOL_MODE_MODE),
        flags: GENL_ADMIN_PERM | GENL_CMD_CAP_DO,
    },
    GenlSplitOp {
        cmd: NFSD_CMD_POOL_MODE_GET,
        maxattr: None,
        flags: GENL_CMD_CAP_DO,
    },
];

pub const NFSD_NL_FAMILY: GenlFamily = GenlFamily {
    name: NFSD_FAMILY_NAME,
    version: NFSD_FAMILY_VERSION,
    netnsok: true,
    parallel_ops: true,
    n_split_ops: NFSD_NL_OPS.len(),
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfsd_netlink_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/nfsd/netlink.c"
        ));
        assert!(source.contains("auto-generated from:"));
        assert!(source.contains("Documentation/netlink/specs/nfsd.yaml"));
        assert!(source.contains("#include <net/netlink.h>"));
        assert!(source.contains("#include <net/genetlink.h>"));
        assert!(source.contains("#include \"netlink.h\""));
        assert!(source.contains("#include <uapi/linux/nfsd_netlink.h>"));
        assert!(source.contains("const struct nla_policy nfsd_sock_nl_policy"));
        assert!(source.contains("[NFSD_A_SOCK_ADDR] = { .type = NLA_BINARY, }"));
        assert!(source.contains("[NFSD_A_SOCK_TRANSPORT_NAME] = { .type = NLA_NUL_STRING, }"));
        assert!(source.contains("const struct nla_policy nfsd_version_nl_policy"));
        assert!(source.contains("[NFSD_A_VERSION_MAJOR] = { .type = NLA_U32, }"));
        assert!(source.contains("[NFSD_A_VERSION_ENABLED] = { .type = NLA_FLAG, }"));
        assert!(source.contains("static const struct nla_policy nfsd_threads_set_nl_policy"));
        assert!(source.contains("[NFSD_A_SERVER_FH_KEY] = NLA_POLICY_EXACT_LEN(16)"));
        assert!(source.contains("NLA_POLICY_NESTED(nfsd_version_nl_policy)"));
        assert!(source.contains("NLA_POLICY_NESTED(nfsd_sock_nl_policy)"));
        assert!(source.contains("static const struct genl_split_ops nfsd_nl_ops[]"));
        assert!(source.contains(".cmd\t= NFSD_CMD_RPC_STATUS_GET"));
        assert!(source.contains(".dumpit\t= nfsd_nl_rpc_status_get_dumpit"));
        assert!(source.contains(".flags\t= GENL_CMD_CAP_DUMP"));
        assert!(source.contains(".cmd\t\t= NFSD_CMD_THREADS_SET"));
        assert!(source.contains(".flags\t\t= GENL_ADMIN_PERM | GENL_CMD_CAP_DO"));
        assert!(source.contains("struct genl_family nfsd_nl_family __ro_after_init"));
        assert!(source.contains(".name\t\t= NFSD_FAMILY_NAME"));
        assert!(source.contains(".version\t= NFSD_FAMILY_VERSION"));
        assert!(source.contains(".netnsok\t= true"));
        assert!(source.contains(".parallel_ops\t= true"));
        assert!(source.contains(".n_split_ops\t= ARRAY_SIZE(nfsd_nl_ops)"));

        assert_eq!(NFSD_NL_FAMILY.name, "nfsd");
        assert_eq!(NFSD_NL_FAMILY.version, 1);
        assert!(NFSD_NL_FAMILY.netnsok);
        assert!(NFSD_NL_FAMILY.parallel_ops);
        assert_eq!(NFSD_NL_FAMILY.n_split_ops, 9);
        assert_eq!(
            NFSD_THREADS_SET_NL_POLICY[5].kind,
            NlaPolicyKind::ExactLen(16)
        );
        assert_eq!(NFSD_NL_OPS[0].flags, GENL_CMD_CAP_DUMP);
        assert_eq!(NFSD_NL_OPS[1].flags, GENL_ADMIN_PERM | GENL_CMD_CAP_DO);
        assert_eq!(NFSD_NL_OPS[8].cmd, NFSD_CMD_POOL_MODE_GET);
    }
}
