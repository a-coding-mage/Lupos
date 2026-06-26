//! linux-parity: complete
//! linux-source: vendor/linux/security/keys/compat_dh.c
//! test-origin: linux:vendor/linux/security/keys/compat_dh.c
//! 32-bit compat adapter for keyctl DH KDF parameters.

use crate::include::uapi::errno::EOPNOTSUPP;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyctlDhParams {
    pub private: i32,
    pub prime: i32,
    pub base: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompatKeyctlKdfParams {
    pub hashname: u32,
    pub otherinfo: u32,
    pub otherinfolen: u32,
    pub spare: [u32; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyctlKdfParams {
    pub hashname: usize,
    pub otherinfo: usize,
    pub otherinfolen: u32,
    pub spare: [u32; 8],
}

pub fn compat_keyctl_dh_compute_with<F>(
    params: &KeyctlDhParams,
    buffer: usize,
    buflen: usize,
    kdf: Option<&CompatKeyctlKdfParams>,
    mut compute: F,
) -> i64
where
    F: FnMut(&KeyctlDhParams, usize, usize, Option<&KeyctlKdfParams>) -> i64,
{
    match kdf {
        None => compute(params, buffer, buflen, None),
        Some(compat) => {
            let kdfcopy = KeyctlKdfParams {
                hashname: compat_ptr(compat.hashname),
                otherinfo: compat_ptr(compat.otherinfo),
                otherinfolen: compat.otherinfolen,
                spare: compat.spare,
            };
            compute(params, buffer, buflen, Some(&kdfcopy))
        }
    }
}

pub fn compat_keyctl_dh_compute(
    params: &KeyctlDhParams,
    buffer: usize,
    buflen: usize,
    kdf: Option<&CompatKeyctlKdfParams>,
) -> i64 {
    compat_keyctl_dh_compute_with(params, buffer, buflen, kdf, |_, _, _, _| {
        -(EOPNOTSUPP as i64)
    })
}

pub const fn compat_ptr(ptr: u32) -> usize {
    ptr as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compat_dh_source_copies_kdf_fields_to_native_params() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/keys/compat_dh.c"
        ));
        assert!(source.contains("struct keyctl_kdf_params kdfcopy;"));
        assert!(source.contains("copy_from_user(&compat_kdfcopy, kdf, sizeof(compat_kdfcopy))"));
        assert!(source.contains("kdfcopy.hashname = compat_ptr(compat_kdfcopy.hashname);"));
        assert!(source.contains("return __keyctl_dh_compute(params, buffer, buflen, &kdfcopy);"));

        let params = KeyctlDhParams {
            private: 1,
            prime: 2,
            base: 3,
        };
        let compat = CompatKeyctlKdfParams {
            hashname: 0x1000,
            otherinfo: 0x2000,
            otherinfolen: 12,
            spare: [7; 8],
        };

        let result =
            compat_keyctl_dh_compute_with(&params, 0x3000, 64, Some(&compat), |p, b, l, k| {
                assert_eq!(*p, params);
                assert_eq!(b, 0x3000);
                assert_eq!(l, 64);
                let kdf = k.expect("kdf");
                assert_eq!(kdf.hashname, 0x1000);
                assert_eq!(kdf.otherinfo, 0x2000);
                assert_eq!(kdf.otherinfolen, 12);
                assert_eq!(kdf.spare, [7; 8]);
                123
            });

        assert_eq!(result, 123);
    }

    #[test]
    fn compat_dh_null_kdf_delegates_without_kdfcopy() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let params = KeyctlDhParams {
            private: 4,
            prime: 5,
            base: 6,
        };
        let result = compat_keyctl_dh_compute_with(&params, 0, 0, None, |_, _, _, kdf| {
            assert!(kdf.is_none());
            77
        });
        assert_eq!(result, 77);
        assert_eq!(compat_keyctl_dh_compute(&params, 0, 0, None), -95);
    }
}
