//! linux-parity: complete
//! linux-source: vendor/linux/security/apparmor/ipc.c
//! test-origin: linux:vendor/linux/security/apparmor/ipc.c
//! AppArmor signal IPC mediation helpers.

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

pub const SIGUNKNOWN: i32 = 0;
pub const SIGRT_BASE: i32 = 128;
pub const SIGRTMIN: i32 = 32;
pub const SIGRTMAX: i32 = 64;
pub const MAXMAPPED_SIG: i32 = 32;
pub const MAY_READ: u32 = 0x04;
pub const MAY_WRITE: u32 = 0x02;
pub const AA_SIGNAL_PERM_MASK: u32 = MAY_READ | MAY_WRITE;
pub const AA_CLASS_SIGNAL: u16 = 14;
pub const OP_SIGNAL: &str = "signal";

pub const SIG_MAP: [i32; MAXMAPPED_SIG as usize] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31,
];

pub const fn map_signal_num(sig: i32) -> i32 {
    if sig > SIGRTMAX {
        SIGUNKNOWN
    } else if sig >= SIGRTMIN {
        sig - SIGRTMIN + SIGRT_BASE
    } else if sig >= 0 && sig < MAXMAPPED_SIG {
        SIG_MAP[sig as usize]
    } else {
        SIGUNKNOWN
    }
}

pub const fn audit_signal_mask(mask: u32) -> &'static str {
    if (mask & MAY_READ) != 0 {
        "receive"
    } else if (mask & MAY_WRITE) != 0 {
        "send"
    } else {
        ""
    }
}

pub const fn aa_signal_requests(sig: i32) -> (i32, u32, u32) {
    (map_signal_num(sig), MAY_WRITE, MAY_READ)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignalProfile {
    pub name: String,
    pub unconfined: bool,
    pub mediates_signal: bool,
    pub allow: u32,
}

impl SignalProfile {
    pub fn confined(name: &str, allow: u32) -> Self {
        Self {
            name: name.to_string(),
            unconfined: false,
            mediates_signal: true,
            allow,
        }
    }

    pub fn unconfined(name: &str) -> Self {
        Self {
            name: name.to_string(),
            unconfined: true,
            mediates_signal: false,
            allow: AA_SIGNAL_PERM_MASK,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignalAuditData {
    pub signal: i32,
    pub unmappedsig: i32,
    pub request: u32,
    pub denied: u32,
    pub peer: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignalPermissionResult {
    pub allowed: bool,
    pub audit: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AaMaySignalResult {
    pub signal: i32,
    pub sender_request: u32,
    pub target_request: u32,
    pub allowed: bool,
    pub audits: Vec<String>,
}

pub fn audit_signal_cb(ad: &SignalAuditData) -> String {
    let mut fields = String::new();
    if (ad.request & AA_SIGNAL_PERM_MASK) != 0 {
        fields.push_str(" requested_mask=\"");
        fields.push_str(audit_signal_mask(ad.request));
        fields.push('"');
        if (ad.denied & AA_SIGNAL_PERM_MASK) != 0 {
            fields.push_str(" denied_mask=\"");
            fields.push_str(audit_signal_mask(ad.denied));
            fields.push('"');
        }
    }
    if ad.signal == SIGUNKNOWN {
        fields.push_str(" signal=unknown(");
        fields.push_str(&ad.unmappedsig.to_string());
        fields.push(')');
    } else if ad.signal < MAXMAPPED_SIG {
        fields.push_str(" signal=");
        fields.push_str(&ad.signal.to_string());
    } else {
        fields.push_str(" signal=rtmin+");
        fields.push_str(&(ad.signal - SIGRT_BASE).to_string());
    }
    fields.push_str(" peer=");
    fields.push_str(&ad.peer);
    fields
}

pub fn profile_signal_perm(
    profile: &SignalProfile,
    peer: &str,
    request: u32,
    signal: i32,
    unmappedsig: i32,
) -> SignalPermissionResult {
    if profile.unconfined || !profile.mediates_signal {
        return SignalPermissionResult {
            allowed: true,
            audit: None,
        };
    }

    let denied = request & !profile.allow;
    let ad = SignalAuditData {
        signal,
        unmappedsig,
        request,
        denied,
        peer: peer.to_string(),
    };
    SignalPermissionResult {
        allowed: denied == 0,
        audit: Some(audit_signal_cb(&ad)),
    }
}

pub fn aa_may_signal(
    sender: &[SignalProfile],
    target: &[SignalProfile],
    sig: i32,
) -> AaMaySignalResult {
    let signal = map_signal_num(sig);
    let mut allowed = true;
    let mut audits = Vec::new();

    for profile in sender {
        let result = profile_signal_perm(profile, "target", MAY_WRITE, signal, sig);
        allowed &= result.allowed;
        if let Some(audit) = result.audit {
            audits.push(audit);
        }
    }
    for profile in target {
        let result = profile_signal_perm(profile, "sender", MAY_READ, signal, sig);
        allowed &= result.allowed;
        if let Some(audit) = result.audit {
            audits.push(audit);
        }
    }

    AaMaySignalResult {
        signal,
        sender_request: MAY_WRITE,
        target_request: MAY_READ,
        allowed,
        audits,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apparmor_ipc_signal_rules_match_linux_source() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/apparmor/ipc.c"
        ));
        assert!(source.contains("static inline int map_signal_num(int sig)"));
        assert!(source.contains("if (sig > SIGRTMAX)"));
        assert!(source.contains("else if (sig >= SIGRTMIN)"));
        assert!(source.contains("return sig - SIGRTMIN + SIGRT_BASE;"));
        assert!(source.contains("return sig_map[sig];"));
        assert!(source.contains("static const char *audit_signal_mask(u32 mask)"));
        assert!(source.contains("if (mask & MAY_READ)"));
        assert!(source.contains("return \"receive\";"));
        assert!(source.contains("if (mask & MAY_WRITE)"));
        assert!(source.contains("return \"send\";"));
        assert!(source.contains("int aa_may_signal"));
        assert!(source.contains("profile_signal_perm(subj_cred, profile, target"));
        assert!(source.contains("aa_state_t state;"));
        assert!(source.contains("if (profile_unconfined(profile))"));
        assert!(source.contains("state = RULE_MEDIATES(rules, AA_CLASS_SIGNAL);"));
        assert!(source.contains("aa_apply_modes_to_perms(profile, &perms);"));
        assert!(source.contains("aa_check_perms(profile, &perms, request, ad, audit_signal_cb);"));
        assert!(source.contains("MAY_WRITE"));
        assert!(source.contains("profile_signal_perm(target_cred, profile, sender"));
        assert!(source.contains("MAY_READ"));

        assert_eq!(map_signal_num(SIGRTMAX + 1), SIGUNKNOWN);
        assert_eq!(map_signal_num(SIGRTMIN + 3), SIGRT_BASE + 3);
        assert_eq!(map_signal_num(9), SIG_MAP[9]);
        assert_eq!(audit_signal_mask(MAY_READ), "receive");
        assert_eq!(audit_signal_mask(MAY_WRITE), "send");
        assert_eq!(
            aa_signal_requests(SIGRTMIN),
            (SIGRT_BASE, MAY_WRITE, MAY_READ)
        );

        let sender = [SignalProfile::confined("sender", MAY_WRITE)];
        let target = [SignalProfile::confined("target", 0)];
        let denied = aa_may_signal(&sender, &target, SIGRTMIN + 2);
        assert_eq!(denied.signal, SIGRT_BASE + 2);
        assert!(!denied.allowed);
        assert!(denied.audits[0].contains("requested_mask=\"send\""));
        assert!(denied.audits[1].contains("requested_mask=\"receive\""));
        assert!(denied.audits[1].contains("denied_mask=\"receive\""));

        let unconfined = [SignalProfile::unconfined("unconfined")];
        let allowed = aa_may_signal(&unconfined, &unconfined, 9);
        assert!(allowed.allowed);
        assert!(allowed.audits.is_empty());
    }
}
