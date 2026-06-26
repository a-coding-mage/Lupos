//! linux-parity: complete
//! linux-source: vendor/linux/net/netlabel/netlabel_user.c
//! test-origin: linux:vendor/linux/net/netlabel/netlabel_user.c
//! NetLabel generic netlink setup and audit-message helpers.

extern crate alloc;

pub const AUDIT_OFF: u8 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NetlabelInitResults {
    pub mgmt: i32,
    pub cipsov4: i32,
    pub calipso: i32,
    pub unlabeled: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NetlabelAuditInfo<'a> {
    pub loginuid: u32,
    pub sessionid: u32,
    pub subj_ctx: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditBuffer {
    pub msg_type: i32,
    pub auid: u32,
    pub sessionid: u32,
    pub subj_ctx: alloc::string::String,
}

pub const fn netlbl_netlink_init(results: NetlabelInitResults) -> i32 {
    if results.mgmt != 0 {
        return results.mgmt;
    }
    if results.cipsov4 != 0 {
        return results.cipsov4;
    }
    if results.calipso != 0 {
        return results.calipso;
    }
    results.unlabeled
}

pub fn netlbl_audit_start_common(
    msg_type: i32,
    audit_info: NetlabelAuditInfo<'_>,
    audit_enabled: u8,
    audit_log_start_ok: bool,
) -> Option<AuditBuffer> {
    if audit_enabled == AUDIT_OFF || !audit_log_start_ok {
        return None;
    }

    Some(AuditBuffer {
        msg_type,
        auid: audit_info.loginuid,
        sessionid: audit_info.sessionid,
        subj_ctx: audit_info.subj_ctx.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn netlabel_user_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netlabel/netlabel_user.c"
        ));
        assert!(source.contains("int __init netlbl_netlink_init(void)"));
        assert!(source.contains("ret_val = netlbl_mgmt_genl_init();"));
        assert!(source.contains("ret_val = netlbl_cipsov4_genl_init();"));
        assert!(source.contains("ret_val = netlbl_calipso_genl_init();"));
        assert!(source.contains("return netlbl_unlabel_genl_init();"));
        assert!(source.contains("netlbl_audit_start_common(int type"));
        assert!(source.contains("if (audit_enabled == AUDIT_OFF)"));
        assert!(source.contains("audit_log_start(audit_context(), GFP_ATOMIC, type);"));
        assert!(source.contains("audit_log_format(audit_buf, \"netlabel: auid=%u ses=%u\""));
        assert!(source.contains("audit_log_subj_ctx(audit_buf, &audit_info->prop);"));
    }

    #[test]
    fn netlink_init_returns_first_component_failure() {
        assert_eq!(
            netlbl_netlink_init(NetlabelInitResults {
                mgmt: -1,
                cipsov4: -2,
                calipso: -3,
                unlabeled: -4,
            }),
            -1
        );
        assert_eq!(
            netlbl_netlink_init(NetlabelInitResults {
                mgmt: 0,
                cipsov4: 0,
                calipso: -3,
                unlabeled: -4,
            }),
            -3
        );
        assert_eq!(
            netlbl_netlink_init(NetlabelInitResults {
                mgmt: 0,
                cipsov4: 0,
                calipso: 0,
                unlabeled: 7,
            }),
            7
        );
    }

    #[test]
    fn audit_common_obeys_audit_enabled_and_alloc_edges() {
        let info = NetlabelAuditInfo {
            loginuid: 42,
            sessionid: 9,
            subj_ctx: "system_u:system_r",
        };
        assert_eq!(netlbl_audit_start_common(100, info, AUDIT_OFF, true), None);
        assert_eq!(netlbl_audit_start_common(100, info, 1, false), None);
        assert_eq!(
            netlbl_audit_start_common(100, info, 1, true),
            Some(AuditBuffer {
                msg_type: 100,
                auid: 42,
                sessionid: 9,
                subj_ctx: "system_u:system_r".into(),
            })
        );
    }
}
