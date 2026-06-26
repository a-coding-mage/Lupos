//! linux-parity: complete
//! linux-source: vendor/linux/security/selinux/status.c
//! test-origin: linux:vendor/linux/security/selinux/status.c
//! SELinux mmap status-page state and seqlock update protocol.

pub const SELINUX_KERNEL_STATUS_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SelinuxKernelStatus {
    pub version: u32,
    pub sequence: u32,
    pub enforcing: u32,
    pub policyload: u32,
    pub deny_unknown: u32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SelinuxStatusPage {
    status: Option<SelinuxKernelStatus>,
    pub lock_count: u32,
    pub unlock_count: u32,
}

impl SelinuxStatusPage {
    pub const fn new() -> Self {
        Self {
            status: None,
            lock_count: 0,
            unlock_count: 0,
        }
    }

    pub fn kernel_status_page(
        &mut self,
        enforcing_enabled: bool,
        allow_unknown: bool,
    ) -> &SelinuxKernelStatus {
        self.lock_count = self.lock_count.saturating_add(1);
        if self.status.is_none() {
            self.status = Some(SelinuxKernelStatus {
                version: SELINUX_KERNEL_STATUS_VERSION,
                sequence: 0,
                enforcing: enforcing_enabled as u32,
                policyload: 0,
                deny_unknown: (!allow_unknown) as u32,
            });
        }
        self.unlock_count = self.unlock_count.saturating_add(1);
        self.status.as_ref().expect("status page initialized")
    }

    pub fn update_setenforce(&mut self, enforcing: bool) {
        self.lock_count = self.lock_count.saturating_add(1);
        if let Some(status) = self.status.as_mut() {
            status.sequence = status.sequence.wrapping_add(1);
            status.enforcing = enforcing as u32;
            status.sequence = status.sequence.wrapping_add(1);
        }
        self.unlock_count = self.unlock_count.saturating_add(1);
    }

    pub fn update_policyload(&mut self, seqno: u32, allow_unknown: bool) {
        self.lock_count = self.lock_count.saturating_add(1);
        if let Some(status) = self.status.as_mut() {
            status.sequence = status.sequence.wrapping_add(1);
            status.policyload = seqno;
            status.deny_unknown = (!allow_unknown) as u32;
            status.sequence = status.sequence.wrapping_add(1);
        }
        self.unlock_count = self.unlock_count.saturating_add(1);
    }

    pub const fn current(&self) -> Option<SelinuxKernelStatus> {
        self.status
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selinux_status_page_matches_linux_seqlock_updates() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/selinux/status.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/selinux/include/security.h"
        ));

        assert!(source.contains("status->version = SELINUX_KERNEL_STATUS_VERSION;"));
        assert!(source.contains("status->sequence = 0;"));
        assert!(source.contains("status->enforcing = enforcing_enabled();"));
        assert!(source.contains("status->policyload = 0;"));
        assert!(source.contains("status->deny_unknown ="));
        assert!(source.contains("status->sequence++;"));
        assert!(source.contains("smp_wmb();"));
        assert!(source.contains("status->policyload = seqno;"));
        assert!(source.contains("mutex_lock(&selinux_state.status_lock);"));
        assert!(source.contains("mutex_unlock(&selinux_state.status_lock);"));
        assert!(header.contains("#define SELINUX_KERNEL_STATUS_VERSION 1"));
        assert!(header.contains("struct selinux_kernel_status"));

        let mut page = SelinuxStatusPage::new();
        let initial = *page.kernel_status_page(true, false);
        assert_eq!((page.lock_count, page.unlock_count), (1, 1));
        assert_eq!(
            initial,
            SelinuxKernelStatus {
                version: SELINUX_KERNEL_STATUS_VERSION,
                sequence: 0,
                enforcing: 1,
                policyload: 0,
                deny_unknown: 1,
            }
        );

        page.update_setenforce(false);
        assert_eq!(page.current().unwrap().sequence, 2);
        assert_eq!(page.current().unwrap().enforcing, 0);

        page.update_policyload(17, true);
        let status = page.current().unwrap();
        assert_eq!((page.lock_count, page.unlock_count), (3, 3));
        assert_eq!(status.sequence, 4);
        assert_eq!(status.policyload, 17);
        assert_eq!(status.deny_unknown, 0);
    }

    #[test]
    fn status_updates_are_noops_before_page_allocation() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let mut page = SelinuxStatusPage::new();
        page.update_setenforce(true);
        page.update_policyload(9, false);
        assert_eq!(page.current(), None);
        assert_eq!((page.lock_count, page.unlock_count), (2, 2));
    }
}
