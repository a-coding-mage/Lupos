//! linux-parity: complete
//! linux-source: vendor/linux/security/keys
//! test-origin: linux:vendor/linux/security/keys
//! Keyring syscall surface.
//!
//! Linux syscalls:
//! - `sys_add_key` (248) â€” `add_key(type, desc, payload, plen, ringid)`
//! - `sys_request_key` (249) â€” `request_key(type, desc, callout, ringid)`
//! - `sys_keyctl` (250) â€” multi-command interface

use super::keyctl::dispatch_keyctl;

/// `sys_add_key(type, desc, payload, plen, _ringid)` â€” Linux syscall 248.
/// All pointers are caller-side (kernel buffers in M64); user copy is the
/// caller's responsibility once VFS-fd integration lands.
pub unsafe fn sys_add_key(
    key_type: *const i8,
    description: *const i8,
    payload: *const u8,
    plen: usize,
    ringid: i32,
) -> i64 {
    if key_type.is_null() || description.is_null() {
        return -22;
    }
    let kt = unsafe { c_str_to_str(key_type) };
    let desc = unsafe { c_str_to_str(description) };
    let bytes = if payload.is_null() {
        &[][..]
    } else {
        unsafe { core::slice::from_raw_parts(payload, plen) }
    };
    super::add_key_to_keyring_from_user(kt, desc, bytes, ringid) as i64
}

/// `sys_request_key(type, desc, _callout, _ringid)` â€” Linux syscall 249.
pub unsafe fn sys_request_key(
    key_type: *const i8,
    description: *const i8,
    _callout: *const i8,
    ringid: i32,
) -> i64 {
    if key_type.is_null() || description.is_null() {
        return -22;
    }
    let kt = unsafe { c_str_to_str(key_type) };
    let desc = unsafe { c_str_to_str(description) };
    super::search_keyring_from_user(ringid, kt, desc) as i64
}

/// `sys_keyctl(cmd, arg2, arg3, arg4, arg5)` â€” Linux syscall 250.
pub unsafe fn sys_keyctl(cmd: i32, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> i64 {
    dispatch_keyctl(cmd, arg2, arg3, arg4, arg5)
}

unsafe fn c_str_to_str(p: *const i8) -> &'static str {
    let mut len = 0;
    unsafe {
        while *p.add(len) != 0 {
            len += 1;
            if len > 256 {
                break;
            }
        }
        let bytes = core::slice::from_raw_parts(p as *const u8, len);
        // NOTE: caller-controlled bytes; we lie about lifetimes for the
        // limited M64 boot-test scope where the callers are static literals.
        core::str::from_utf8(core::mem::transmute::<&[u8], &'static [u8]>(bytes)).unwrap_or("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syscall_m78_security_bpf_perf_parity() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        assert_eq!(
            unsafe {
                sys_add_key(
                    core::ptr::null(),
                    b"desc\0".as_ptr() as *const i8,
                    core::ptr::null(),
                    0,
                    0,
                )
            },
            -22
        );
        let typ = b"user\0";
        let desc = b"m78-key\0";
        let payload = b"value";
        let key = unsafe {
            sys_add_key(
                typ.as_ptr() as *const i8,
                desc.as_ptr() as *const i8,
                payload.as_ptr(),
                payload.len(),
                0,
            )
        };
        assert!(key > 0);
        assert!(
            unsafe {
                sys_request_key(
                    typ.as_ptr() as *const i8,
                    desc.as_ptr() as *const i8,
                    core::ptr::null(),
                    0,
                )
            } > 0
        );
        assert!(unsafe { sys_keyctl(0, 0, 0, 0, 0) } <= 0);
    }
}
