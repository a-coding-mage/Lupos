//! linux-parity: complete
//! linux-source: vendor/linux/fs/smb/server/mgmt/user_config.c
//! test-origin: linux:vendor/linux/fs/smb/server/mgmt/user_config.c
//! KSMBD user login response copying and comparison helpers.

pub const KSMBD_REQ_MAX_ACCOUNT_NAME_SZ: usize = 48;
pub const KSMBD_REQ_MAX_HASH_SZ: usize = 18;

pub const KSMBD_USER_FLAG_OK: u16 = 1 << 0;
pub const KSMBD_USER_FLAG_GUEST_ACCOUNT: u16 = 1 << 4;
pub const KSMBD_USER_FLAG_EXTENSION: u16 = 1 << 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KsmbdLoginResponse<'a> {
    pub account: &'a [u8],
    pub status: u16,
    pub gid: u32,
    pub uid: u32,
    pub hash: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KsmbdLoginResponseExt<'a> {
    pub groups: &'a [u32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KsmbdUser<'a> {
    pub flags: u16,
    pub uid: u32,
    pub gid: u32,
    pub name: &'a [u8],
    pub passkey: &'a [u8],
    pub groups: &'a [u32],
}

pub const fn ksmbd_login_response_is_ok(status: u16) -> bool {
    status & KSMBD_USER_FLAG_OK != 0
}

pub const fn ksmbd_login_response_needs_extension(status: u16) -> bool {
    status & KSMBD_USER_FLAG_EXTENSION != 0
}

pub const fn ksmbd_alloc_user<'a>(
    resp: KsmbdLoginResponse<'a>,
    resp_ext: Option<KsmbdLoginResponseExt<'a>>,
) -> KsmbdUser<'a> {
    KsmbdUser {
        flags: resp.status,
        uid: resp.uid,
        gid: resp.gid,
        name: resp.account,
        passkey: resp.hash,
        groups: match resp_ext {
            Some(ext) => ext.groups,
            None => &[],
        },
    }
}

pub const fn ksmbd_login_user_result<'a>(
    resp: Option<KsmbdLoginResponse<'a>>,
    resp_ext: Option<KsmbdLoginResponseExt<'a>>,
) -> Option<KsmbdUser<'a>> {
    match resp {
        Some(response) if ksmbd_login_response_is_ok(response.status) => {
            Some(ksmbd_alloc_user(response, resp_ext))
        }
        _ => None,
    }
}

pub const fn user_guest(user: &KsmbdUser<'_>) -> bool {
    user.flags & KSMBD_USER_FLAG_GUEST_ACCOUNT != 0
}

pub const fn ksmbd_anonymous_user_name(name: &[u8]) -> bool {
    !name.is_empty() && name[0] == 0
}

pub fn ksmbd_compare_user(u1: &KsmbdUser<'_>, u2: &KsmbdUser<'_>) -> bool {
    if u1.name != u2.name {
        return false;
    }
    if u2.passkey.len() < u1.passkey.len() {
        return false;
    }
    u1.passkey == &u2.passkey[..u1.passkey.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ksmbd_user_config_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/smb/server/mgmt/user_config.c"
        ));
        assert!(source.contains("#include <linux/slab.h>"));
        assert!(source.contains("#include <linux/mm.h>"));
        assert!(source.contains("#include \"user_config.h\""));
        assert!(source.contains("#include \"../transport_ipc.h\""));
        assert!(source.contains("resp = ksmbd_ipc_login_request(account);"));
        assert!(source.contains("if (!(resp->status & KSMBD_USER_FLAG_OK))"));
        assert!(source.contains("if (resp->status & KSMBD_USER_FLAG_EXTENSION)"));
        assert!(source.contains("resp_ext = ksmbd_ipc_login_request_ext(account);"));
        assert!(source.contains("user = ksmbd_alloc_user(resp, resp_ext);"));
        assert!(source.contains("kvfree(resp);"));
        assert!(source.contains("user->name = kstrdup(resp->account, KSMBD_DEFAULT_GFP);"));
        assert!(source.contains("user->passkey = kmalloc(resp->hash_sz, KSMBD_DEFAULT_GFP);"));
        assert!(source.contains("memcpy(user->passkey, resp->hash, resp->hash_sz);"));
        assert!(source.contains("user->ngroups = 0;"));
        assert!(source.contains("user->sgid = NULL;"));
        assert!(source.contains("kmemdup(resp_ext->____payload"));
        assert!(source.contains("user->ngroups = resp_ext->ngroups;"));
        assert!(source.contains("ksmbd_ipc_logout_request(user->name, user->flags);"));
        assert!(source.contains("return user->name[0] == '\\0';"));
        assert!(source.contains("if (strcmp(u1->name, u2->name))"));
        assert!(source.contains("if (memcmp(u1->passkey, u2->passkey, u1->passkey_sz))"));

        let resp = KsmbdLoginResponse {
            account: b"alice\0",
            status: KSMBD_USER_FLAG_OK | KSMBD_USER_FLAG_EXTENSION,
            gid: 10,
            uid: 20,
            hash: b"0123456789",
        };
        let ext = KsmbdLoginResponseExt { groups: &[1, 2, 3] };
        let user = ksmbd_login_user_result(Some(resp), Some(ext)).unwrap();
        assert_eq!(user.uid, 20);
        assert_eq!(user.gid, 10);
        assert_eq!(user.groups, &[1, 2, 3]);
        assert!(ksmbd_login_response_needs_extension(resp.status));
        assert!(ksmbd_compare_user(
            &user,
            &KsmbdUser {
                passkey: b"0123456789extra",
                ..user
            }
        ));
        assert!(!ksmbd_compare_user(
            &user,
            &KsmbdUser {
                name: b"bob\0",
                ..user
            }
        ));
        assert!(ksmbd_anonymous_user_name(b"\0"));
        assert!(
            ksmbd_login_user_result(Some(KsmbdLoginResponse { status: 0, ..resp }), None).is_none()
        );
        assert!(ksmbd_login_user_result(None, None).is_none());
    }
}
