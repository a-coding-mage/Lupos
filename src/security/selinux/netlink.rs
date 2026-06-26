//! linux-parity: complete
//! linux-source: vendor/linux/security/selinux/netlink.c
//! test-origin: linux:vendor/linux/security/selinux/netlink.c
//! SELinux netlink notification payload routing.

extern crate alloc;

use alloc::vec::Vec;

pub const SELNL_MSG_SETENFORCE: i32 = 1;
pub const SELNL_MSG_POLICYLOAD: i32 = 2;
pub const SELNLGRP_AVC: u32 = 1;
pub const SELNLGRP_MAX: u32 = 1;
pub const NETLINK_SELINUX: i32 = 7;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelinuxNetlinkPayload {
    SetEnforce { val: i32 },
    PolicyLoad { seqno: u32 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelinuxNetlinkNotification {
    pub msgtype: i32,
    pub len: usize,
    pub payload: SelinuxNetlinkPayload,
    pub dst_group: u32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SelinuxNetlink {
    pub socket_created: bool,
    pub groups: u32,
    pub nonroot_recv: bool,
    pub notifications: Vec<SelinuxNetlinkNotification>,
    pub oom_events: u32,
}

pub const fn selnl_msglen(msgtype: i32) -> Option<usize> {
    match msgtype {
        SELNL_MSG_SETENFORCE => Some(core::mem::size_of::<i32>()),
        SELNL_MSG_POLICYLOAD => Some(core::mem::size_of::<u32>()),
        _ => None,
    }
}

pub const fn selnl_payload(msgtype: i32, data: u32) -> Option<SelinuxNetlinkPayload> {
    match msgtype {
        SELNL_MSG_SETENFORCE => Some(SelinuxNetlinkPayload::SetEnforce { val: data as i32 }),
        SELNL_MSG_POLICYLOAD => Some(SelinuxNetlinkPayload::PolicyLoad { seqno: data }),
        _ => None,
    }
}

pub const fn sel_netlink_cfg() -> (u32, bool) {
    (SELNLGRP_MAX, true)
}

impl SelinuxNetlink {
    pub fn sel_netlink_init(&mut self, create_ok: bool) -> Result<(), &'static str> {
        let (groups, nonroot_recv) = sel_netlink_cfg();
        self.groups = groups;
        self.nonroot_recv = nonroot_recv;
        if !create_ok {
            return Err("SELinux:  Cannot create netlink socket.");
        }
        self.socket_created = true;
        Ok(())
    }

    pub fn selnl_notify(&mut self, msgtype: i32, data: u32) -> Result<(), &'static str> {
        let Some(len) = selnl_msglen(msgtype) else {
            return Err("BUG");
        };
        let Some(payload) = selnl_payload(msgtype, data) else {
            return Err("BUG");
        };
        if !self.socket_created {
            self.oom_events = self.oom_events.saturating_add(1);
            return Err("SELinux:  OOM in selnl_notify");
        }
        self.notifications.push(SelinuxNetlinkNotification {
            msgtype,
            len,
            payload,
            dst_group: SELNLGRP_AVC,
        });
        Ok(())
    }

    pub fn selnl_notify_setenforce(&mut self, val: i32) -> Result<(), &'static str> {
        self.selnl_notify(SELNL_MSG_SETENFORCE, val as u32)
    }

    pub fn selnl_notify_policyload(&mut self, seqno: u32) -> Result<(), &'static str> {
        self.selnl_notify(SELNL_MSG_POLICYLOAD, seqno)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selinux_netlink_messages_match_linux_source() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/selinux/netlink.c"
        ));
        assert!(source.contains("static int selnl_msglen(int msgtype)"));
        assert!(source.contains("case SELNL_MSG_SETENFORCE:"));
        assert!(source.contains("sizeof(struct selnl_msg_setenforce)"));
        assert!(source.contains("case SELNL_MSG_POLICYLOAD:"));
        assert!(source.contains("sizeof(struct selnl_msg_policyload)"));
        assert!(source.contains("static void selnl_add_payload"));
        assert!(source.contains("msg->val = *((int *)data);"));
        assert!(source.contains("msg->seqno = *((u32 *)data);"));
        assert!(source.contains("NETLINK_CB(skb).dst_group = SELNLGRP_AVC;"));
        assert!(source.contains("netlink_broadcast(selnl, skb, 0, SELNLGRP_AVC, GFP_USER);"));
        assert!(source.contains("void selnl_notify_setenforce(int val)"));
        assert!(source.contains("void selnl_notify_policyload(u32 seqno)"));
        assert!(source.contains("panic(\"SELinux:  Cannot create netlink socket.\""));
        assert!(source.contains(".groups\t= SELNLGRP_MAX"));
        assert!(source.contains(".flags\t= NL_CFG_F_NONROOT_RECV"));
        assert!(source.contains("NETLINK_SELINUX"));

        assert_eq!(selnl_msglen(SELNL_MSG_SETENFORCE), Some(4));
        assert_eq!(selnl_msglen(SELNL_MSG_POLICYLOAD), Some(4));
        assert_eq!(selnl_msglen(99), None);
        assert_eq!(
            selnl_payload(SELNL_MSG_SETENFORCE, 1),
            Some(SelinuxNetlinkPayload::SetEnforce { val: 1 })
        );
        assert_eq!(
            selnl_payload(SELNL_MSG_POLICYLOAD, 42),
            Some(SelinuxNetlinkPayload::PolicyLoad { seqno: 42 })
        );
        assert_eq!(sel_netlink_cfg(), (SELNLGRP_MAX, true));

        let mut nl = SelinuxNetlink::default();
        assert_eq!(nl.sel_netlink_init(true), Ok(()));
        assert!(nl.socket_created);
        assert_eq!(nl.groups, SELNLGRP_MAX);
        assert!(nl.nonroot_recv);
        assert_eq!(nl.selnl_notify_setenforce(1), Ok(()));
        assert_eq!(nl.selnl_notify_policyload(42), Ok(()));
        assert_eq!(
            nl.notifications[0],
            SelinuxNetlinkNotification {
                msgtype: SELNL_MSG_SETENFORCE,
                len: 4,
                payload: SelinuxNetlinkPayload::SetEnforce { val: 1 },
                dst_group: SELNLGRP_AVC,
            }
        );
        assert_eq!(
            nl.notifications[1].payload,
            SelinuxNetlinkPayload::PolicyLoad { seqno: 42 }
        );

        let mut failed = SelinuxNetlink::default();
        assert_eq!(
            failed.sel_netlink_init(false),
            Err("SELinux:  Cannot create netlink socket.")
        );
    }
}
