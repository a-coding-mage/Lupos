//! linux-parity: complete
//! linux-source: vendor/linux/fs/lockd/netlink.c
//! test-origin: linux:vendor/linux/fs/lockd/netlink.c
//! lockd generic-netlink family metadata.

pub const LOCKD_FAMILY_NAME: &str = "lockd";
pub const LOCKD_FAMILY_VERSION: u8 = 1;

pub const LOCKD_A_SERVER_GRACETIME: u8 = 1;
pub const LOCKD_A_SERVER_TCP_PORT: u8 = 2;
pub const LOCKD_A_SERVER_UDP_PORT: u8 = 3;

pub const LOCKD_CMD_SERVER_SET: u8 = 1;
pub const LOCKD_CMD_SERVER_GET: u8 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LockdNlaType {
    U16,
    U32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LockdNlaPolicy {
    pub attr: u8,
    pub nla_type: LockdNlaType,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LockdGenlOp {
    pub cmd: u8,
    pub doit: &'static str,
    pub maxattr: Option<u8>,
    pub admin_perm: bool,
    pub cmd_cap_do: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LockdGenlFamily {
    pub name: &'static str,
    pub version: u8,
    pub netnsok: bool,
    pub parallel_ops: bool,
}

pub const LOCKD_SERVER_SET_NL_POLICY: &[LockdNlaPolicy] = &[
    LockdNlaPolicy {
        attr: LOCKD_A_SERVER_GRACETIME,
        nla_type: LockdNlaType::U32,
    },
    LockdNlaPolicy {
        attr: LOCKD_A_SERVER_TCP_PORT,
        nla_type: LockdNlaType::U16,
    },
    LockdNlaPolicy {
        attr: LOCKD_A_SERVER_UDP_PORT,
        nla_type: LockdNlaType::U16,
    },
];

pub const LOCKD_NL_OPS: &[LockdGenlOp] = &[
    LockdGenlOp {
        cmd: LOCKD_CMD_SERVER_SET,
        doit: "lockd_nl_server_set_doit",
        maxattr: Some(LOCKD_A_SERVER_UDP_PORT),
        admin_perm: true,
        cmd_cap_do: true,
    },
    LockdGenlOp {
        cmd: LOCKD_CMD_SERVER_GET,
        doit: "lockd_nl_server_get_doit",
        maxattr: None,
        admin_perm: false,
        cmd_cap_do: true,
    },
];

pub const LOCKD_NL_FAMILY: LockdGenlFamily = LockdGenlFamily {
    name: LOCKD_FAMILY_NAME,
    version: LOCKD_FAMILY_VERSION,
    netnsok: true,
    parallel_ops: true,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockd_netlink_family_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/lockd/netlink.c"
        ));
        assert!(source.contains("#include <net/netlink.h>"));
        assert!(source.contains("#include <net/genetlink.h>"));
        assert!(source.contains("#include \"netlink.h\""));
        assert!(source.contains("#include <uapi/linux/lockd_netlink.h>"));
        assert!(source.contains("lockd_server_set_nl_policy"));
        assert!(source.contains("[LOCKD_A_SERVER_GRACETIME] = { .type = NLA_U32"));
        assert!(source.contains("[LOCKD_A_SERVER_TCP_PORT] = { .type = NLA_U16"));
        assert!(source.contains("[LOCKD_A_SERVER_UDP_PORT] = { .type = NLA_U16"));
        assert!(source.contains(".cmd\t\t= LOCKD_CMD_SERVER_SET"));
        assert!(source.contains(".doit\t\t= lockd_nl_server_set_doit"));
        assert!(source.contains(".maxattr\t= LOCKD_A_SERVER_UDP_PORT"));
        assert!(source.contains("GENL_ADMIN_PERM | GENL_CMD_CAP_DO"));
        assert!(source.contains(".cmd\t= LOCKD_CMD_SERVER_GET"));
        assert!(source.contains(".doit\t= lockd_nl_server_get_doit"));
        assert!(source.contains("struct genl_family lockd_nl_family"));
        assert!(source.contains(".name\t\t= LOCKD_FAMILY_NAME"));
        assert!(source.contains(".version\t= LOCKD_FAMILY_VERSION"));
        assert!(source.contains(".netnsok\t= true"));
        assert!(source.contains(".parallel_ops\t= true"));

        assert_eq!(LOCKD_SERVER_SET_NL_POLICY.len(), 3);
        assert_eq!(
            LOCKD_SERVER_SET_NL_POLICY[0],
            LockdNlaPolicy {
                attr: LOCKD_A_SERVER_GRACETIME,
                nla_type: LockdNlaType::U32,
            }
        );
        assert_eq!(LOCKD_NL_OPS[0].maxattr, Some(LOCKD_A_SERVER_UDP_PORT));
        assert!(LOCKD_NL_OPS[0].admin_perm);
        assert!(LOCKD_NL_OPS[1].cmd_cap_do);
        assert_eq!(LOCKD_NL_FAMILY.name, "lockd");
        assert!(LOCKD_NL_FAMILY.netnsok);
        assert!(LOCKD_NL_FAMILY.parallel_ops);
    }
}
