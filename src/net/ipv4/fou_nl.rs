//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv4/fou_nl.c
//! test-origin: linux:vendor/linux/net/ipv4/fou_nl.c
//! Foo-over-UDP generic netlink policy and operations.

pub const FOU_ATTR_UNSPEC: usize = 0;
pub const FOU_ATTR_PORT: usize = 1;
pub const FOU_ATTR_AF: usize = 2;
pub const FOU_ATTR_IPPROTO: usize = 3;
pub const FOU_ATTR_TYPE: usize = 4;
pub const FOU_ATTR_REMCSUM_NOPARTIAL: usize = 5;
pub const FOU_ATTR_LOCAL_V4: usize = 6;
pub const FOU_ATTR_LOCAL_V6: usize = 7;
pub const FOU_ATTR_PEER_V4: usize = 8;
pub const FOU_ATTR_PEER_V6: usize = 9;
pub const FOU_ATTR_PEER_PORT: usize = 10;
pub const FOU_ATTR_IFINDEX: usize = 11;
pub const FOU_ATTR_MAX: usize = FOU_ATTR_IFINDEX;

pub const FOU_CMD_UNSPEC: u8 = 0;
pub const FOU_CMD_ADD: u8 = 1;
pub const FOU_CMD_DEL: u8 = 2;
pub const FOU_CMD_GET: u8 = 3;
pub const FOU_CMD_MAX: u8 = FOU_CMD_GET;

pub const GENL_DONT_VALIDATE_STRICT: u32 = 0x1;
pub const GENL_DONT_VALIDATE_DUMP: u32 = 0x2;
pub const GENL_ADMIN_PERM: u32 = 0x1;
pub const FOU_VALIDATE: u32 = GENL_DONT_VALIDATE_STRICT | GENL_DONT_VALIDATE_DUMP;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NlaType {
    Be16,
    U8,
    Flag,
    U32,
    S32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NlaPolicy {
    Unspec,
    Type(NlaType),
    Min(NlaType, u32),
    ExactLen(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenlSmallOp {
    pub cmd: u8,
    pub validate: u32,
    pub doit: &'static str,
    pub dumpit: Option<&'static str>,
    pub flags: u32,
}

pub const FOU_NL_POLICY: [NlaPolicy; FOU_ATTR_IFINDEX + 1] = [
    NlaPolicy::Unspec,
    NlaPolicy::Type(NlaType::Be16),
    NlaPolicy::Type(NlaType::U8),
    NlaPolicy::Min(NlaType::U8, 1),
    NlaPolicy::Type(NlaType::U8),
    NlaPolicy::Type(NlaType::Flag),
    NlaPolicy::Type(NlaType::U32),
    NlaPolicy::ExactLen(16),
    NlaPolicy::Type(NlaType::U32),
    NlaPolicy::ExactLen(16),
    NlaPolicy::Type(NlaType::Be16),
    NlaPolicy::Type(NlaType::S32),
];

pub const FOU_NL_OPS: [GenlSmallOp; 3] = [
    GenlSmallOp {
        cmd: FOU_CMD_ADD,
        validate: FOU_VALIDATE,
        doit: "fou_nl_add_doit",
        dumpit: None,
        flags: GENL_ADMIN_PERM,
    },
    GenlSmallOp {
        cmd: FOU_CMD_DEL,
        validate: FOU_VALIDATE,
        doit: "fou_nl_del_doit",
        dumpit: None,
        flags: GENL_ADMIN_PERM,
    },
    GenlSmallOp {
        cmd: FOU_CMD_GET,
        validate: FOU_VALIDATE,
        doit: "fou_nl_get_doit",
        dumpit: Some("fou_nl_get_dumpit"),
        flags: 0,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fou_netlink_tables_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv4/fou_nl.c"
        ));
        assert!(source.contains("const struct nla_policy fou_nl_policy[FOU_ATTR_IFINDEX + 1]"));
        assert!(source.contains("[FOU_ATTR_PORT] = { .type = NLA_BE16, }"));
        assert!(source.contains("[FOU_ATTR_AF] = { .type = NLA_U8, }"));
        assert!(source.contains("[FOU_ATTR_IPPROTO] = NLA_POLICY_MIN(NLA_U8, 1)"));
        assert!(source.contains("[FOU_ATTR_REMCSUM_NOPARTIAL] = { .type = NLA_FLAG, }"));
        assert!(source.contains("[FOU_ATTR_LOCAL_V6] = NLA_POLICY_EXACT_LEN(16)"));
        assert!(source.contains("[FOU_ATTR_IFINDEX] = { .type = NLA_S32, }"));
        assert!(source.contains("const struct genl_small_ops fou_nl_ops[3]"));
        assert!(source.contains(".cmd\t\t= FOU_CMD_ADD"));
        assert!(source.contains(".doit\t\t= fou_nl_add_doit"));
        assert!(source.contains(".cmd\t\t= FOU_CMD_DEL"));
        assert!(source.contains(".doit\t\t= fou_nl_del_doit"));
        assert!(source.contains(".cmd\t\t= FOU_CMD_GET"));
        assert!(source.contains(".dumpit\t\t= fou_nl_get_dumpit"));

        assert_eq!(FOU_NL_POLICY.len(), FOU_ATTR_MAX + 1);
        assert_eq!(FOU_NL_POLICY[FOU_ATTR_PORT], NlaPolicy::Type(NlaType::Be16));
        assert_eq!(
            FOU_NL_POLICY[FOU_ATTR_IPPROTO],
            NlaPolicy::Min(NlaType::U8, 1)
        );
        assert_eq!(FOU_NL_POLICY[FOU_ATTR_LOCAL_V6], NlaPolicy::ExactLen(16));
        assert_eq!(FOU_NL_POLICY[FOU_ATTR_PEER_V6], NlaPolicy::ExactLen(16));
        assert_eq!(FOU_NL_OPS[0].cmd, FOU_CMD_ADD);
        assert_eq!(FOU_NL_OPS[0].flags, GENL_ADMIN_PERM);
        assert_eq!(FOU_NL_OPS[1].doit, "fou_nl_del_doit");
        assert_eq!(FOU_NL_OPS[2].dumpit, Some("fou_nl_get_dumpit"));
        assert_eq!(FOU_CMD_MAX, 3);
    }
}
