//! linux-parity: partial
//! linux-source: vendor/linux/drivers/firmware/efi
//! UEFI runtime services facade.
//!
//! Lupos currently exposes the runtime-variable surface needed by the Linux
//! platform certificate loader. The architecture-specific OVMF `GetVariable`
//! backend can register variables here once EFI runtime mappings are live.

pub mod vars;
