//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kernel/cpu/sgx/ioctl.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/sgx/ioctl.c
//! SGX driver ioctl command set.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/sgx/ioctl.c

// `ioctl.c` defines SGX_IOC_ENCLAVE_CREATE, ADD_PAGES, INIT,
// PROVISION, and RESTRICT_PERMISSIONS. The ioctl number layout uses
// _IOWR('s', N, struct). We model the command map.

use crate::include::uapi::errno::EINVAL;

pub const SGX_IOC_TYPE: u8 = b'S';

pub const SGX_IOC_ENCLAVE_CREATE: u32 = 0;
pub const SGX_IOC_ENCLAVE_ADD_PAGES: u32 = 1;
pub const SGX_IOC_ENCLAVE_INIT: u32 = 2;
pub const SGX_IOC_ENCLAVE_PROVISION: u32 = 3;
pub const SGX_IOC_ENCLAVE_RESTRICT_PERMISSIONS: u32 = 4;
pub const SGX_IOC_ENCLAVE_MODIFY_TYPES: u32 = 5;
pub const SGX_IOC_ENCLAVE_REMOVE_PAGES: u32 = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SgxIoctlCommand {
    Create,
    AddPages,
    Init,
    Provision,
    RestrictPermissions,
    ModifyTypes,
    RemovePages,
}

pub fn classify(nr: u32) -> Result<SgxIoctlCommand, i32> {
    match nr {
        SGX_IOC_ENCLAVE_CREATE => Ok(SgxIoctlCommand::Create),
        SGX_IOC_ENCLAVE_ADD_PAGES => Ok(SgxIoctlCommand::AddPages),
        SGX_IOC_ENCLAVE_INIT => Ok(SgxIoctlCommand::Init),
        SGX_IOC_ENCLAVE_PROVISION => Ok(SgxIoctlCommand::Provision),
        SGX_IOC_ENCLAVE_RESTRICT_PERMISSIONS => Ok(SgxIoctlCommand::RestrictPermissions),
        SGX_IOC_ENCLAVE_MODIFY_TYPES => Ok(SgxIoctlCommand::ModifyTypes),
        SGX_IOC_ENCLAVE_REMOVE_PAGES => Ok(SgxIoctlCommand::RemovePages),
        _ => Err(EINVAL),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_handles_all_known_commands() {
        assert_eq!(classify(0), Ok(SgxIoctlCommand::Create));
        assert_eq!(classify(2), Ok(SgxIoctlCommand::Init));
        assert_eq!(classify(99), Err(EINVAL));
    }
}
