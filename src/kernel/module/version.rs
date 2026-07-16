//! linux-parity: partial
//! linux-source: vendor/linux/kernel/module/version.c
//! test-origin: linux:vendor/linux/tools/testing/selftests/module
//! Module symbol-version metadata.
//!
//! This is the data-plane counterpart of Linux's `check_version()`.  The ELF
//! loader supplies the three non-retained version sections while the module
//! image is still available and asks this object about every undefined symbol
//! before committing its relocation.

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

const MODULE_NAME_LEN: usize = 56;
const BASIC_ENTRY_SIZE: usize = core::mem::size_of::<u64>() + MODULE_NAME_LEN;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModuleVersionError {
    MalformedBasic,
    MalformedExtended,
    Mismatch {
        symbol: String,
        expected: u32,
        found: u32,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct VersionEntry {
    name: String,
    crc: u32,
}

/// Parsed `__versions`, or the extended CRC/name pair used for long symbols.
#[derive(Clone, Debug, Default)]
pub struct ModuleVersions {
    entries: Vec<VersionEntry>,
    extended: bool,
}

impl ModuleVersions {
    /// Parse the metadata selected by `CONFIG_MODVERSIONS`.
    ///
    /// As in `check_version()`, a present extended table takes precedence over
    /// `__versions`. Both extended sections are required as a pair.
    pub fn parse(
        basic: Option<&[u8]>,
        extended_crcs: Option<&[u8]>,
        extended_names: Option<&[u8]>,
    ) -> Result<Self, ModuleVersionError> {
        if extended_crcs.is_some() || extended_names.is_some() {
            return Self::parse_extended(
                extended_crcs.ok_or(ModuleVersionError::MalformedExtended)?,
                extended_names.ok_or(ModuleVersionError::MalformedExtended)?,
            );
        }

        let Some(bytes) = basic else {
            return Ok(Self::default());
        };
        if bytes.len() % BASIC_ENTRY_SIZE != 0 {
            return Err(ModuleVersionError::MalformedBasic);
        }

        let mut entries = Vec::with_capacity(bytes.len() / BASIC_ENTRY_SIZE);
        for entry in bytes.chunks_exact(BASIC_ENTRY_SIZE) {
            let crc = u64::from_le_bytes(
                entry[..8]
                    .try_into()
                    .map_err(|_| ModuleVersionError::MalformedBasic)?,
            ) as u32;
            let name_field = &entry[8..];
            let end = name_field
                .iter()
                .position(|byte| *byte == 0)
                .ok_or(ModuleVersionError::MalformedBasic)?;
            let name = core::str::from_utf8(&name_field[..end])
                .map_err(|_| ModuleVersionError::MalformedBasic)?;
            entries.push(VersionEntry {
                name: name.to_string(),
                crc,
            });
        }
        Ok(Self {
            entries,
            extended: false,
        })
    }

    fn parse_extended(crcs: &[u8], names: &[u8]) -> Result<Self, ModuleVersionError> {
        if crcs.len() % core::mem::size_of::<u32>() != 0 {
            return Err(ModuleVersionError::MalformedExtended);
        }
        let count = crcs.len() / core::mem::size_of::<u32>();
        let mut entries = Vec::with_capacity(count);
        let mut name_offset = 0usize;

        for crc_bytes in crcs.chunks_exact(4) {
            let remaining = names
                .get(name_offset..)
                .ok_or(ModuleVersionError::MalformedExtended)?;
            let length = remaining
                .iter()
                .position(|byte| *byte == 0)
                .ok_or(ModuleVersionError::MalformedExtended)?;
            let name = core::str::from_utf8(&remaining[..length])
                .map_err(|_| ModuleVersionError::MalformedExtended)?;
            let crc = u32::from_le_bytes(
                crc_bytes
                    .try_into()
                    .map_err(|_| ModuleVersionError::MalformedExtended)?,
            );
            entries.push(VersionEntry {
                name: name.to_string(),
                crc,
            });
            name_offset = name_offset
                .checked_add(length + 1)
                .ok_or(ModuleVersionError::MalformedExtended)?;
        }

        // modpost emits exactly one NUL-terminated name for every CRC. Extra
        // bytes would make the two generated sections disagree.
        if name_offset != names.len() {
            return Err(ModuleVersionError::MalformedExtended);
        }
        Ok(Self {
            entries,
            extended: true,
        })
    }

    pub fn is_extended(&self) -> bool {
        self.extended
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Linux `check_version()` semantics for one resolved export.
    ///
    /// No exporter CRC, or no importer record, is accepted. A record which is
    /// present but disagrees is fatal.
    pub fn check_symbol(
        &self,
        symbol: &str,
        exporter_crc: Option<u32>,
    ) -> Result<(), ModuleVersionError> {
        let Some(expected) = exporter_crc else {
            return Ok(());
        };
        let Some(entry) = self.entries.iter().find(|entry| entry.name == symbol) else {
            return Ok(());
        };
        if entry.crc == expected {
            return Ok(());
        }
        Err(ModuleVersionError::Mismatch {
            symbol: symbol.to_string(),
            expected,
            found: entry.crc,
        })
    }
}
