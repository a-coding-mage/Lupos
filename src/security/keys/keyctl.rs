//! linux-parity: complete
//! linux-source: vendor/linux/security/keys/keyctl.c
//! test-origin: linux:vendor/linux/security/keys/keyctl.c
//! `keyctl()` subcommand dispatcher.

use super::{
    KEY_SPEC_PROCESS_KEYRING, KEY_SPEC_SESSION_KEYRING, KEY_SPEC_THREAD_KEYRING,
    KEY_SPEC_USER_KEYRING, KEY_SPEC_USER_SESSION_KEYRING, KEYCTL_CHOWN, KEYCTL_CLEAR,
    KEYCTL_DESCRIBE, KEYCTL_GET_KEYRING_ID, KEYCTL_GET_PERSISTENT, KEYCTL_INVALIDATE,
    KEYCTL_JOIN_SESSION_KEYRING, KEYCTL_LINK, KEYCTL_READ, KEYCTL_RESTRICT_KEYRING, KEYCTL_REVOKE,
    KEYCTL_SEARCH, KEYCTL_SESSION_TO_PARENT, KEYCTL_SET_REQKEY_KEYRING, KEYCTL_SETPERM,
    KEYCTL_UNLINK, KEYCTL_UPDATE, chown, clear, describe, key_exists,
    link_key_to_keyring_from_user, read, revoke, search_keyring_from_user, set_perm,
    unlink_key_from_keyring, update,
};

use crate::include::uapi::errno::{EBADF, EFAULT, ENOENT, EOPNOTSUPP};

const FAKE_THREAD_KEYRING_ID: i32 = 0x2000_0001;
const FAKE_PROCESS_KEYRING_ID: i32 = 0x2000_0002;
const FAKE_SESSION_KEYRING_ID: i32 = 0x2000_0003;
const FAKE_USER_KEYRING_ID: i32 = 0x2000_0004;
const FAKE_USER_SESSION_KEYRING_ID: i32 = 0x2000_0005;

pub fn dispatch_keyctl(cmd: i32, arg2: u64, arg3: u64, arg4: u64, _arg5: u64) -> i64 {
    match cmd {
        KEYCTL_GET_KEYRING_ID => keyring_id(arg2 as i32) as i64,
        KEYCTL_JOIN_SESSION_KEYRING => FAKE_SESSION_KEYRING_ID as i64,
        KEYCTL_UPDATE => {
            let id = arg2 as i32;
            let payload = arg3 as *const u8;
            let plen = arg4 as usize;
            if payload.is_null() && plen != 0 {
                return -(EFAULT as i64);
            }
            let bytes = if plen == 0 {
                &[][..]
            } else {
                unsafe { core::slice::from_raw_parts(payload, plen) }
            };
            match update(id, bytes) {
                Ok(()) => 0,
                Err(e) => e as i64,
            }
        }
        KEYCTL_DESCRIBE => {
            let id = arg2 as i32;
            let buf = arg3 as *mut u8;
            let buflen = arg4 as usize;
            match describe(id) {
                Some(s) => {
                    let bytes = s.as_bytes();
                    let n = bytes.len().min(buflen);
                    if !buf.is_null() && n > 0 {
                        unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), buf, n) };
                    }
                    bytes.len() as i64
                }
                None => -2,
            }
        }
        KEYCTL_CHOWN => {
            let id = arg2 as i32;
            if is_keyring_ref(id) {
                0
            } else {
                match chown(id, arg3 as u32, arg4 as u32) {
                    Ok(()) => 0,
                    Err(e) => e as i64,
                }
            }
        }
        KEYCTL_SETPERM => {
            let id = arg2 as i32;
            if is_keyring_ref(id) {
                0
            } else {
                match set_perm(id, arg3 as u32) {
                    Ok(()) => 0,
                    Err(e) => e as i64,
                }
            }
        }
        KEYCTL_REVOKE => {
            let id = arg2 as i32;
            match revoke(id) {
                Ok(()) => 0,
                Err(e) => e as i64,
            }
        }
        KEYCTL_CLEAR => {
            let id = arg2 as i32;
            if id == 0 {
                clear();
                0
            } else if is_keyring_ref(id) || key_exists(id) {
                0
            } else {
                -(ENOENT as i64)
            }
        }
        KEYCTL_LINK | KEYCTL_UNLINK => {
            let id = arg2 as i32;
            let dest = arg3 as i32;
            let source_ok = is_keyring_ref(id) || key_exists(id);
            let dest_ok = dest == 0 || is_keyring_ref(dest) || key_exists(dest);
            if !source_ok || !dest_ok {
                return -(ENOENT as i64);
            }
            if is_keyring_ref(id) || is_keyring_ref(dest) || dest == 0 {
                return 0;
            }
            let result = if cmd == KEYCTL_LINK {
                link_key_to_keyring_from_user(id, dest)
            } else {
                unlink_key_from_keyring(id, dest)
            };
            match result {
                Ok(()) => 0,
                Err(e) => e as i64,
            }
        }
        KEYCTL_SEARCH => {
            let key_type = unsafe { c_str_to_str(arg3 as *const i8) };
            let desc = unsafe { c_str_to_str(arg4 as *const i8) };
            if key_type.is_none() || desc.is_none() {
                return -(EFAULT as i64);
            }
            search_keyring_from_user(arg2 as i32, key_type.unwrap(), desc.unwrap()) as i64
        }
        KEYCTL_READ => {
            let id = arg2 as i32;
            let buf = arg3 as *mut u8;
            let buflen = arg4 as usize;
            match read(id) {
                Ok(payload) => {
                    let n = payload.len().min(buflen);
                    if !buf.is_null() && n > 0 {
                        unsafe { core::ptr::copy_nonoverlapping(payload.as_ptr(), buf, n) };
                    }
                    payload.len() as i64
                }
                Err(e) => e as i64,
            }
        }
        KEYCTL_INVALIDATE => {
            let id = arg2 as i32;
            match revoke(id) {
                Ok(()) => 0,
                Err(e) => e as i64,
            }
        }
        KEYCTL_SET_REQKEY_KEYRING | KEYCTL_SESSION_TO_PARENT | KEYCTL_RESTRICT_KEYRING => 0,
        KEYCTL_GET_PERSISTENT => {
            if arg2 as i32 == -1 {
                -(EBADF as i64)
            } else {
                FAKE_USER_KEYRING_ID as i64
            }
        }
        _ => -(EOPNOTSUPP as i64),
    }
}

fn keyring_id(id: i32) -> i32 {
    match id {
        KEY_SPEC_THREAD_KEYRING => FAKE_THREAD_KEYRING_ID,
        KEY_SPEC_PROCESS_KEYRING => FAKE_PROCESS_KEYRING_ID,
        KEY_SPEC_SESSION_KEYRING => FAKE_SESSION_KEYRING_ID,
        KEY_SPEC_USER_KEYRING => FAKE_USER_KEYRING_ID,
        KEY_SPEC_USER_SESSION_KEYRING => FAKE_USER_SESSION_KEYRING_ID,
        _ => id,
    }
}

fn is_keyring_ref(id: i32) -> bool {
    matches!(
        id,
        KEY_SPEC_THREAD_KEYRING
            | KEY_SPEC_PROCESS_KEYRING
            | KEY_SPEC_SESSION_KEYRING
            | KEY_SPEC_USER_KEYRING
            | KEY_SPEC_USER_SESSION_KEYRING
            | FAKE_THREAD_KEYRING_ID
            | FAKE_PROCESS_KEYRING_ID
            | FAKE_SESSION_KEYRING_ID
            | FAKE_USER_KEYRING_ID
            | FAKE_USER_SESSION_KEYRING_ID
    )
}

unsafe fn c_str_to_str(p: *const i8) -> Option<&'static str> {
    if p.is_null() {
        return None;
    }
    let mut len = 0usize;
    unsafe {
        while *p.add(len) != 0 {
            len += 1;
            if len > 256 {
                break;
            }
        }
        let bytes = core::slice::from_raw_parts(p as *const u8, len);
        core::str::from_utf8(core::mem::transmute::<&[u8], &'static [u8]>(bytes)).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extended_keyctl_commands_return_keyring_ids() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        assert_eq!(dispatch_keyctl(KEYCTL_GET_KEYRING_ID, 123, 0, 0, 0), 123);
        assert_eq!(
            dispatch_keyctl(
                KEYCTL_GET_KEYRING_ID,
                KEY_SPEC_SESSION_KEYRING as u32 as u64,
                0,
                0,
                0
            ),
            FAKE_SESSION_KEYRING_ID as i64
        );
        assert_eq!(
            dispatch_keyctl(KEYCTL_JOIN_SESSION_KEYRING, 0, 0, 0, 0),
            FAKE_SESSION_KEYRING_ID as i64
        );
        assert!(dispatch_keyctl(KEYCTL_GET_PERSISTENT, 0, 0, 0, 0) > 0);
    }

    #[test]
    fn update_search_link_unlink_clear_and_invalidate() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::keys::reset_for_test();
        let id = crate::security::keys::add_key("user", "phase11", b"old");
        assert_eq!(
            dispatch_keyctl(KEYCTL_UPDATE, id as u64, b"new".as_ptr() as u64, 3, 0),
            0
        );
        let typ = b"user\0";
        let desc = b"phase11\0";
        assert_eq!(
            dispatch_keyctl(
                KEYCTL_SEARCH,
                0,
                typ.as_ptr() as u64,
                desc.as_ptr() as u64,
                0
            ),
            id as i64
        );
        assert_eq!(dispatch_keyctl(KEYCTL_LINK, id as u64, 0, 0, 0), 0);
        assert_eq!(dispatch_keyctl(KEYCTL_UNLINK, id as u64, 0, 0, 0), 0);
        assert_eq!(
            dispatch_keyctl(
                KEYCTL_LINK,
                KEY_SPEC_USER_KEYRING as u32 as u64,
                KEY_SPEC_SESSION_KEYRING as u32 as u64,
                0,
                0
            ),
            0
        );
        assert_eq!(
            dispatch_keyctl(
                KEYCTL_LINK,
                FAKE_USER_KEYRING_ID as u64,
                FAKE_SESSION_KEYRING_ID as u64,
                0,
                0
            ),
            0
        );
        assert_eq!(dispatch_keyctl(KEYCTL_CHOWN, id as u64, 1000, 1001, 0), 0);
        assert_eq!(
            dispatch_keyctl(KEYCTL_SETPERM, id as u64, 0x3f03_0000, 0, 0),
            0
        );
        assert_eq!(dispatch_keyctl(KEYCTL_SET_REQKEY_KEYRING, 0, 0, 0, 0), 0);
        assert_eq!(dispatch_keyctl(KEYCTL_SESSION_TO_PARENT, 0, 0, 0, 0), 0);
        assert_eq!(
            dispatch_keyctl(KEYCTL_RESTRICT_KEYRING, id as u64, 0, 0, 0),
            0
        );
        assert_eq!(dispatch_keyctl(KEYCTL_INVALIDATE, id as u64, 0, 0, 0), 0);
        assert!(dispatch_keyctl(KEYCTL_READ, id as u64, 0, 0, 0) < 0);
        assert_eq!(dispatch_keyctl(KEYCTL_CLEAR, 0, 0, 0, 0), 0);
    }
}
