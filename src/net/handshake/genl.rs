//! linux-parity: complete
//! linux-source: vendor/linux/net/handshake/genl.c
//! test-origin: linux:vendor/linux/net/handshake/genl.c
//! Handshake generic netlink family metadata.

pub const HANDSHAKE_FAMILY_NAME: &str = "handshake";
pub const HANDSHAKE_FAMILY_VERSION: u8 = 1;

pub const HANDSHAKE_A_ACCEPT_HANDLER_CLASS: usize = 2;
pub const HANDSHAKE_A_DONE_STATUS: usize = 1;
pub const HANDSHAKE_A_DONE_SOCKFD: usize = 2;
pub const HANDSHAKE_A_DONE_REMOTE_AUTH: usize = 3;

pub const HANDSHAKE_CMD_ACCEPT: u8 = 2;
pub const HANDSHAKE_CMD_DONE: u8 = 3;
pub const HANDSHAKE_NLGRP_NONE: usize = 0;
pub const HANDSHAKE_NLGRP_TLSHD: usize = 1;

pub const MAX_ERRNO: u32 = 4095;
pub const GENL_ADMIN_PERM: u32 = 0x01;
pub const GENL_CMD_CAP_DO: u32 = 0x02;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NlaType {
    U32,
    S32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NlaPolicy {
    Unspec,
    Type(NlaType),
    Max(NlaType, u32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenlSplitOp {
    pub cmd: u8,
    pub doit: &'static str,
    pub policy: &'static str,
    pub maxattr: usize,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenlMulticastGroup {
    pub name: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenlFamily {
    pub name: &'static str,
    pub version: u8,
    pub netnsok: bool,
    pub parallel_ops: bool,
    pub module: &'static str,
    pub split_ops: &'static [GenlSplitOp],
    pub mcgrps: &'static [GenlMulticastGroup],
}

pub const HANDSHAKE_ACCEPT_NL_POLICY: [NlaPolicy; HANDSHAKE_A_ACCEPT_HANDLER_CLASS + 1] = [
    NlaPolicy::Unspec,
    NlaPolicy::Unspec,
    NlaPolicy::Max(NlaType::U32, 2),
];

pub const HANDSHAKE_DONE_NL_POLICY: [NlaPolicy; HANDSHAKE_A_DONE_REMOTE_AUTH + 1] = [
    NlaPolicy::Unspec,
    NlaPolicy::Max(NlaType::U32, MAX_ERRNO),
    NlaPolicy::Type(NlaType::S32),
    NlaPolicy::Type(NlaType::U32),
];

pub const HANDSHAKE_NL_OPS: [GenlSplitOp; 2] = [
    GenlSplitOp {
        cmd: HANDSHAKE_CMD_ACCEPT,
        doit: "handshake_nl_accept_doit",
        policy: "handshake_accept_nl_policy",
        maxattr: HANDSHAKE_A_ACCEPT_HANDLER_CLASS,
        flags: GENL_ADMIN_PERM | GENL_CMD_CAP_DO,
    },
    GenlSplitOp {
        cmd: HANDSHAKE_CMD_DONE,
        doit: "handshake_nl_done_doit",
        policy: "handshake_done_nl_policy",
        maxattr: HANDSHAKE_A_DONE_REMOTE_AUTH,
        flags: GENL_CMD_CAP_DO,
    },
];

pub const HANDSHAKE_NL_MCGRPS: [GenlMulticastGroup; 2] = [
    GenlMulticastGroup { name: "none" },
    GenlMulticastGroup { name: "tlshd" },
];

pub const HANDSHAKE_NL_FAMILY: GenlFamily = GenlFamily {
    name: HANDSHAKE_FAMILY_NAME,
    version: HANDSHAKE_FAMILY_VERSION,
    netnsok: true,
    parallel_ops: true,
    module: "THIS_MODULE",
    split_ops: &HANDSHAKE_NL_OPS,
    mcgrps: &HANDSHAKE_NL_MCGRPS,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_genl_tables_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/handshake/genl.c"
        ));
        assert!(source.contains("auto-generated from:"));
        assert!(source.contains("static const struct nla_policy handshake_accept_nl_policy"));
        assert!(source.contains("[HANDSHAKE_A_ACCEPT_HANDLER_CLASS] = NLA_POLICY_MAX(NLA_U32, 2)"));
        assert!(source.contains("[HANDSHAKE_A_DONE_STATUS] = NLA_POLICY_MAX(NLA_U32, MAX_ERRNO)"));
        assert!(source.contains("[HANDSHAKE_A_DONE_SOCKFD] = { .type = NLA_S32, }"));
        assert!(source.contains(".cmd\t\t= HANDSHAKE_CMD_ACCEPT"));
        assert!(source.contains(".doit\t\t= handshake_nl_accept_doit"));
        assert!(source.contains(".flags\t\t= GENL_ADMIN_PERM | GENL_CMD_CAP_DO"));
        assert!(source.contains("[HANDSHAKE_NLGRP_TLSHD] = { \"tlshd\", }"));
        assert!(source.contains(".name\t\t= HANDSHAKE_FAMILY_NAME"));
        assert!(source.contains(".parallel_ops\t= true"));

        assert_eq!(
            HANDSHAKE_ACCEPT_NL_POLICY[HANDSHAKE_A_ACCEPT_HANDLER_CLASS],
            NlaPolicy::Max(NlaType::U32, 2)
        );
        assert_eq!(
            HANDSHAKE_DONE_NL_POLICY[HANDSHAKE_A_DONE_STATUS],
            NlaPolicy::Max(NlaType::U32, MAX_ERRNO)
        );
        assert_eq!(
            HANDSHAKE_DONE_NL_POLICY[HANDSHAKE_A_DONE_SOCKFD],
            NlaPolicy::Type(NlaType::S32)
        );
        assert_eq!(HANDSHAKE_NL_OPS[0].cmd, HANDSHAKE_CMD_ACCEPT);
        assert_eq!(HANDSHAKE_NL_OPS[1].doit, "handshake_nl_done_doit");
        assert_eq!(HANDSHAKE_NL_MCGRPS[HANDSHAKE_NLGRP_TLSHD].name, "tlshd");
        assert!(HANDSHAKE_NL_FAMILY.netnsok);
        assert!(HANDSHAKE_NL_FAMILY.parallel_ops);
    }
}
