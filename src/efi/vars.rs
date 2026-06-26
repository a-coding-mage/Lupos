//! linux-parity: partial
//! linux-source: vendor/linux/drivers/firmware/efi/vars.c
//! test-origin: linux:vendor/linux/drivers/firmware/efi/vars.c
//! EFI runtime variable access.
//!
//! Mirrors the narrow shape of Linux's `efivars` registry: consumers lookup a
//! variable by UCS-2 name plus vendor GUID after an EFI runtime provider has
//! registered read operations. Lupos stores a bounded snapshot until the x86
//! OVMF `GetVariable` callback is wired to this facade.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use spin::Mutex;

use crate::include::uapi::errno::{EINVAL, ENODEV, ENOENT, EOPNOTSUPP, EOVERFLOW};

/// UEFI GUID bytes in the same little-endian in-memory form Linux compares
/// with `efi_guidcmp()`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Guid(pub [u8; 16]);

/// `EFI_IMAGE_SECURITY_DATABASE_GUID` from `include/linux/efi.h`.
pub const EFI_IMAGE_SECURITY_DATABASE_GUID: Guid = Guid([
    0xcb, 0xb2, 0x19, 0xd7, 0x3a, 0x3d, 0x96, 0x45, 0xa3, 0xbc, 0xda, 0xd0, 0x0e, 0x67, 0x65, 0x6f,
]);

/// `EFI_SHIM_LOCK_GUID` from `include/linux/efi.h`.
pub const EFI_SHIM_LOCK_GUID: Guid = Guid([
    0x50, 0xab, 0x5d, 0x60, 0x46, 0xe0, 0x00, 0x43, 0xab, 0xb6, 0x3d, 0xd8, 0x10, 0xdd, 0x8b, 0x23,
]);

pub const EFI_VARIABLE_NON_VOLATILE: u32 = 0x0000_0001;
pub const EFI_VARIABLE_BOOTSERVICE_ACCESS: u32 = 0x0000_0002;
pub const EFI_VARIABLE_RUNTIME_ACCESS: u32 = 0x0000_0004;
pub const EFI_VARIABLE_SECURE_BOOT_IMPORT_ATTRS: u32 =
    EFI_VARIABLE_NON_VOLATILE | EFI_VARIABLE_BOOTSERVICE_ACCESS | EFI_VARIABLE_RUNTIME_ACCESS;
pub const EFI_VARIABLE_MAX_DATA_SIZE: usize = 64 * 1024;

pub type EfiStatus = u64;

pub const EFI_SUCCESS: EfiStatus = 0;
pub const EFI_ERROR_BIT: EfiStatus = 1u64 << 63;
pub const EFI_INVALID_PARAMETER: EfiStatus = EFI_ERROR_BIT | 2;
pub const EFI_UNSUPPORTED: EfiStatus = EFI_ERROR_BIT | 3;
pub const EFI_BUFFER_TOO_SMALL: EfiStatus = EFI_ERROR_BIT | 5;
pub const EFI_NOT_FOUND: EfiStatus = EFI_ERROR_BIT | 14;
pub const EFI_ABORTED: EfiStatus = EFI_ERROR_BIT | 21;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeVariable {
    pub name: String,
    pub vendor: Guid,
    pub attributes: u32,
    pub data: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeVariableSource<'a> {
    pub name: &'static str,
    pub vendor: Guid,
    pub attributes: u32,
    pub data: &'a [u8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeVariableRequest {
    pub name: &'static str,
    pub vendor: Guid,
}

pub const SECURE_BOOT_VARIABLE_REQUESTS: &[RuntimeVariableRequest] = &[
    RuntimeVariableRequest {
        name: "MokIgnoreDB",
        vendor: EFI_SHIM_LOCK_GUID,
    },
    RuntimeVariableRequest {
        name: "db",
        vendor: EFI_IMAGE_SECURITY_DATABASE_GUID,
    },
    RuntimeVariableRequest {
        name: "MokListRT",
        vendor: EFI_SHIM_LOCK_GUID,
    },
];

pub trait GetVariableProvider {
    fn get_variable(
        &mut self,
        name: &str,
        vendor: Guid,
        attributes: &mut u32,
        data_size: &mut usize,
        data: Option<&mut [u8]>,
    ) -> EfiStatus;
}

static RUNTIME_VARIABLES: Mutex<Option<Vec<RuntimeVariable>>> = Mutex::new(None);

pub fn runtime_variables_available() -> bool {
    RUNTIME_VARIABLES.lock().is_some()
}

pub fn register_runtime_variables(vars: &[RuntimeVariableSource<'_>]) -> Result<(), i32> {
    if vars
        .iter()
        .any(|var| var.name.is_empty() || var.data.len() > EFI_VARIABLE_MAX_DATA_SIZE)
    {
        return Err(-EINVAL);
    }

    let snapshot = vars
        .iter()
        .map(|var| RuntimeVariable {
            name: String::from(var.name),
            vendor: var.vendor,
            attributes: var.attributes,
            data: var.data.to_vec(),
        })
        .collect();
    *RUNTIME_VARIABLES.lock() = Some(snapshot);
    Ok(())
}

pub fn register_runtime_variables_from_get_variable_provider<P: GetVariableProvider>(
    provider: &mut P,
    requests: &[RuntimeVariableRequest],
) -> Result<usize, i32> {
    if requests.iter().any(|request| request.name.is_empty()) {
        return Err(-EINVAL);
    }

    let mut snapshot = Vec::new();
    for request in requests {
        match read_provider_variable(provider, *request)? {
            Some(var) => snapshot.push(var),
            None => {}
        }
    }

    let count = snapshot.len();
    *RUNTIME_VARIABLES.lock() = Some(snapshot);
    Ok(count)
}

pub fn unregister_runtime_variables() {
    *RUNTIME_VARIABLES.lock() = None;
}

pub fn get_variable(name: &str, vendor: Guid) -> Result<RuntimeVariable, i32> {
    let guard = RUNTIME_VARIABLES.lock();
    let vars = guard.as_ref().ok_or(-ENODEV)?;
    vars.iter()
        .find(|var| var.name == name && var.vendor == vendor)
        .cloned()
        .ok_or(-ENOENT)
}

pub fn variable_exists(name: &str, vendor: Guid) -> Result<bool, i32> {
    match get_variable(name, vendor) {
        Ok(_) => Ok(true),
        Err(err) if err == -ENOENT => Ok(false),
        Err(err) => Err(err),
    }
}

fn read_provider_variable<P: GetVariableProvider>(
    provider: &mut P,
    request: RuntimeVariableRequest,
) -> Result<Option<RuntimeVariable>, i32> {
    let mut attributes = 0u32;
    let mut required_size = 0usize;
    let status = provider.get_variable(
        request.name,
        request.vendor,
        &mut attributes,
        &mut required_size,
        None,
    );

    match status {
        EFI_BUFFER_TOO_SMALL => {}
        EFI_NOT_FOUND => return Ok(None),
        EFI_UNSUPPORTED => return Err(-EOPNOTSUPP),
        EFI_SUCCESS if required_size == 0 => {
            return Ok(Some(RuntimeVariable {
                name: String::from(request.name),
                vendor: request.vendor,
                attributes,
                data: Vec::new(),
            }));
        }
        EFI_SUCCESS => {}
        other => return Err(efi_status_to_errno(other)),
    }

    if required_size > EFI_VARIABLE_MAX_DATA_SIZE {
        return Err(-EOVERFLOW);
    }

    let mut data = alloc::vec![0u8; required_size];
    let mut actual_size = data.len();
    let status = provider.get_variable(
        request.name,
        request.vendor,
        &mut attributes,
        &mut actual_size,
        Some(data.as_mut_slice()),
    );

    match status {
        EFI_SUCCESS => {
            if actual_size > data.len() {
                return Err(-EOVERFLOW);
            }
            data.truncate(actual_size);
            Ok(Some(RuntimeVariable {
                name: String::from(request.name),
                vendor: request.vendor,
                attributes,
                data,
            }))
        }
        EFI_NOT_FOUND => Ok(None),
        EFI_BUFFER_TOO_SMALL => Err(-EOVERFLOW),
        other => Err(efi_status_to_errno(other)),
    }
}

pub const fn efi_status_to_errno(status: EfiStatus) -> i32 {
    match status {
        EFI_SUCCESS => 0,
        EFI_INVALID_PARAMETER => -EINVAL,
        EFI_UNSUPPORTED => -EOPNOTSUPP,
        EFI_NOT_FOUND => -ENOENT,
        EFI_BUFFER_TOO_SMALL => -EOVERFLOW,
        EFI_ABORTED => -ENODEV,
        _ => -EINVAL,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn linux_secure_boot_variable_guids_are_little_endian() {
        assert_eq!(
            EFI_IMAGE_SECURITY_DATABASE_GUID,
            Guid([
                0xcb, 0xb2, 0x19, 0xd7, 0x3a, 0x3d, 0x96, 0x45, 0xa3, 0xbc, 0xda, 0xd0, 0x0e, 0x67,
                0x65, 0x6f,
            ])
        );
        assert_eq!(
            EFI_SHIM_LOCK_GUID,
            Guid([
                0x50, 0xab, 0x5d, 0x60, 0x46, 0xe0, 0x00, 0x43, 0xab, 0xb6, 0x3d, 0xd8, 0x10, 0xdd,
                0x8b, 0x23,
            ])
        );
    }

    #[test]
    fn runtime_variable_snapshot_round_trips_by_name_and_guid() {
        unregister_runtime_variables();
        register_runtime_variables(&[RuntimeVariableSource {
            name: "db",
            vendor: EFI_IMAGE_SECURITY_DATABASE_GUID,
            attributes: EFI_VARIABLE_SECURE_BOOT_IMPORT_ATTRS,
            data: b"sig-list",
        }])
        .expect("install vars");

        let var = get_variable("db", EFI_IMAGE_SECURITY_DATABASE_GUID).expect("db var");
        assert_eq!(var.name, "db");
        assert_eq!(var.attributes, EFI_VARIABLE_SECURE_BOOT_IMPORT_ATTRS);
        assert_eq!(var.data, b"sig-list");
        assert_eq!(variable_exists("MokListRT", EFI_SHIM_LOCK_GUID), Ok(false));
        unregister_runtime_variables();
    }

    #[test]
    fn runtime_variable_snapshot_is_bounded_like_linux_fallback() {
        let oversize = alloc::vec![0u8; EFI_VARIABLE_MAX_DATA_SIZE + 1];
        assert_eq!(
            register_runtime_variables(&[RuntimeVariableSource {
                name: "db",
                vendor: EFI_IMAGE_SECURITY_DATABASE_GUID,
                attributes: EFI_VARIABLE_SECURE_BOOT_IMPORT_ATTRS,
                data: &oversize,
            }]),
            Err(-EINVAL)
        );
    }

    #[derive(Default)]
    struct MockGetVariableProvider {
        variables: Vec<RuntimeVariable>,
        queried: Vec<String>,
    }

    impl MockGetVariableProvider {
        fn with_var(name: &'static str, vendor: Guid, attributes: u32, data: &[u8]) -> Self {
            Self {
                variables: alloc::vec![RuntimeVariable {
                    name: name.to_string(),
                    vendor,
                    attributes,
                    data: data.to_vec(),
                }],
                queried: Vec::new(),
            }
        }
    }

    impl GetVariableProvider for MockGetVariableProvider {
        fn get_variable(
            &mut self,
            name: &str,
            vendor: Guid,
            attributes: &mut u32,
            data_size: &mut usize,
            data: Option<&mut [u8]>,
        ) -> EfiStatus {
            self.queried.push(name.to_string());
            let Some(var) = self
                .variables
                .iter()
                .find(|var| var.name == name && var.vendor == vendor)
            else {
                return EFI_NOT_FOUND;
            };

            *attributes = var.attributes;
            *data_size = var.data.len();
            let Some(out) = data else {
                return EFI_BUFFER_TOO_SMALL;
            };
            if out.len() < var.data.len() {
                return EFI_BUFFER_TOO_SMALL;
            }
            out[..var.data.len()].copy_from_slice(&var.data);
            EFI_SUCCESS
        }
    }

    #[test]
    fn get_variable_provider_snapshot_uses_efi_two_call_flow() {
        unregister_runtime_variables();
        let mut provider = MockGetVariableProvider::with_var(
            "db",
            EFI_IMAGE_SECURITY_DATABASE_GUID,
            EFI_VARIABLE_SECURE_BOOT_IMPORT_ATTRS,
            b"efi-sig-list",
        );

        let loaded = register_runtime_variables_from_get_variable_provider(
            &mut provider,
            SECURE_BOOT_VARIABLE_REQUESTS,
        )
        .expect("provider snapshot");

        assert_eq!(loaded, 1);
        assert_eq!(provider.queried, ["MokIgnoreDB", "db", "db", "MokListRT"]);
        let var = get_variable("db", EFI_IMAGE_SECURITY_DATABASE_GUID).expect("db var");
        assert_eq!(var.attributes, EFI_VARIABLE_SECURE_BOOT_IMPORT_ATTRS);
        assert_eq!(var.data, b"efi-sig-list");
        assert_eq!(variable_exists("MokListRT", EFI_SHIM_LOCK_GUID), Ok(false));
        unregister_runtime_variables();
    }

    #[test]
    fn get_variable_provider_rejects_oversized_firmware_value() {
        struct OversizedProvider;

        impl GetVariableProvider for OversizedProvider {
            fn get_variable(
                &mut self,
                _name: &str,
                _vendor: Guid,
                _attributes: &mut u32,
                data_size: &mut usize,
                _data: Option<&mut [u8]>,
            ) -> EfiStatus {
                *data_size = EFI_VARIABLE_MAX_DATA_SIZE + 1;
                EFI_BUFFER_TOO_SMALL
            }
        }

        let mut provider = OversizedProvider;
        assert_eq!(
            register_runtime_variables_from_get_variable_provider(
                &mut provider,
                &[RuntimeVariableRequest {
                    name: "db",
                    vendor: EFI_IMAGE_SECURITY_DATABASE_GUID,
                }],
            ),
            Err(-EOVERFLOW)
        );
    }
}
