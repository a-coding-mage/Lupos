//! linux-parity: complete
//! linux-source: vendor/linux/fs/smb/client/netlink.c
//! test-origin: linux:vendor/linux/fs/smb/client/netlink.c
//! CIFS generic-netlink family metadata.

pub const CIFS_GENL_NAME: &str = "cifs";
pub const CIFS_GENL_VERSION: u8 = 1;
pub const CIFS_GENL_MCGRP_SWN_NAME: &str = "cifs_mcgrp_swn";
pub const CIFS_GENL_ATTR_MAX: u8 = 14;
pub const CIFS_GENL_CMD_SWN_NOTIFY: u8 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CifsNlaType {
    U32,
    String,
    SockaddrStorage,
    Flag,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CifsGenlPolicy {
    pub attr: u8,
    pub nla_type: CifsNlaType,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CifsGenlFamily {
    pub name: &'static str,
    pub version: u8,
    pub maxattr: u8,
    pub notify_cmd: u8,
    pub multicast_group: &'static str,
    pub admin_perm: bool,
}

pub const CIFS_GENL_POLICY: &[CifsGenlPolicy] = &[
    CifsGenlPolicy {
        attr: 1,
        nla_type: CifsNlaType::U32,
    },
    CifsGenlPolicy {
        attr: 2,
        nla_type: CifsNlaType::String,
    },
    CifsGenlPolicy {
        attr: 3,
        nla_type: CifsNlaType::String,
    },
    CifsGenlPolicy {
        attr: 4,
        nla_type: CifsNlaType::SockaddrStorage,
    },
    CifsGenlPolicy {
        attr: 5,
        nla_type: CifsNlaType::Flag,
    },
    CifsGenlPolicy {
        attr: 6,
        nla_type: CifsNlaType::Flag,
    },
    CifsGenlPolicy {
        attr: 7,
        nla_type: CifsNlaType::Flag,
    },
    CifsGenlPolicy {
        attr: 8,
        nla_type: CifsNlaType::Flag,
    },
    CifsGenlPolicy {
        attr: 9,
        nla_type: CifsNlaType::String,
    },
    CifsGenlPolicy {
        attr: 10,
        nla_type: CifsNlaType::String,
    },
    CifsGenlPolicy {
        attr: 11,
        nla_type: CifsNlaType::String,
    },
    CifsGenlPolicy {
        attr: 12,
        nla_type: CifsNlaType::U32,
    },
    CifsGenlPolicy {
        attr: 13,
        nla_type: CifsNlaType::U32,
    },
    CifsGenlPolicy {
        attr: 14,
        nla_type: CifsNlaType::String,
    },
];

pub const CIFS_GENL_FAMILY: CifsGenlFamily = CifsGenlFamily {
    name: CIFS_GENL_NAME,
    version: CIFS_GENL_VERSION,
    maxattr: CIFS_GENL_ATTR_MAX,
    notify_cmd: CIFS_GENL_CMD_SWN_NOTIFY,
    multicast_group: CIFS_GENL_MCGRP_SWN_NAME,
    admin_perm: true,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cifs_netlink_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/smb/client/netlink.c"
        ));
        assert!(source.contains("#include <net/genetlink.h>"));
        assert!(source.contains("#include <uapi/linux/cifs/cifs_netlink.h>"));
        assert!(source.contains("#include \"netlink.h\""));
        assert!(source.contains("static const struct nla_policy cifs_genl_policy"));
        assert!(source.contains("[CIFS_GENL_ATTR_SWN_REGISTRATION_ID]\t= { .type = NLA_U32 }"));
        assert!(source.contains("[CIFS_GENL_ATTR_SWN_NET_NAME]\t\t= { .type = NLA_STRING }"));
        assert!(
            source.contains(
                "[CIFS_GENL_ATTR_SWN_IP]\t\t\t= { .len = sizeof(struct sockaddr_storage) }"
            )
        );
        assert!(source.contains("[CIFS_GENL_ATTR_SWN_KRB_AUTH]\t\t= { .type = NLA_FLAG }"));
        assert!(source.contains(".cmd = CIFS_GENL_CMD_SWN_NOTIFY"));
        assert!(source.contains(".flags = GENL_ADMIN_PERM"));
        assert!(source.contains(".doit = cifs_swn_notify"));
        assert!(source.contains(".name = CIFS_GENL_MCGRP_SWN_NAME"));
        assert!(source.contains(".flags = GENL_MCAST_CAP_NET_ADMIN"));
        assert!(source.contains("struct genl_family cifs_genl_family"));
        assert!(source.contains(".name\t\t= CIFS_GENL_NAME"));
        assert!(source.contains(".version\t= CIFS_GENL_VERSION"));
        assert!(source.contains(".maxattr\t= CIFS_GENL_ATTR_MAX"));
        assert!(source.contains(".resv_start_op\t= CIFS_GENL_CMD_SWN_NOTIFY + 1"));
        assert!(source.contains("genl_register_family(&cifs_genl_family);"));
        assert!(source.contains("genl_unregister_family(&cifs_genl_family);"));

        assert_eq!(CIFS_GENL_POLICY.len(), CIFS_GENL_ATTR_MAX as usize);
        assert_eq!(CIFS_GENL_POLICY[0].nla_type, CifsNlaType::U32);
        assert_eq!(CIFS_GENL_POLICY[3].nla_type, CifsNlaType::SockaddrStorage);
        assert_eq!(CIFS_GENL_FAMILY.name, "cifs");
        assert_eq!(CIFS_GENL_FAMILY.notify_cmd + 1, 4);
        assert!(CIFS_GENL_FAMILY.admin_perm);
    }
}
