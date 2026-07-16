//! linux-parity: partial
//! linux-source: vendor/linux/kernel/bpf/btf.c
//! test-origin: linux:vendor/linux/tools/testing/selftests/bpf
//! Retained module BTF containers.
//!
//! `btf_parse_hdr()` performs these container checks before the semantic type
//! verifier.  Keeping the bytes in an owned object matches the COMING-module
//! notifier, which copies `.BTF` out of the temporary ELF image.

extern crate alloc;

use alloc::vec::Vec;

const BTF_MAGIC: u16 = 0xeb9f;
const BTF_VERSION: u8 = 1;
const LEGACY_HEADER_LEN: usize = 24;
const CURRENT_HEADER_LEN: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BtfError {
    HeaderMissing,
    InvalidMagic,
    UnsupportedVersion,
    UnsupportedFlags,
    UnsupportedHeader,
    InvalidSectionLayout,
    InvalidStringTable,
    MissingTypes,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BtfHeader {
    pub header_len: u32,
    pub type_offset: u32,
    pub type_len: u32,
    pub string_offset: u32,
    pub string_len: u32,
    pub layout_offset: u32,
    pub layout_len: u32,
}

#[derive(Clone, Debug)]
pub struct ModuleBtf {
    data: Vec<u8>,
    base_data: Option<Vec<u8>>,
    header: BtfHeader,
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16, BtfError> {
    Ok(u16::from_le_bytes(
        data.get(offset..offset + 2)
            .ok_or(BtfError::HeaderMissing)?
            .try_into()
            .map_err(|_| BtfError::HeaderMissing)?,
    ))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32, BtfError> {
    Ok(u32::from_le_bytes(
        data.get(offset..offset + 4)
            .ok_or(BtfError::HeaderMissing)?
            .try_into()
            .map_err(|_| BtfError::HeaderMissing)?,
    ))
}

fn parse_header(data: &[u8], has_base: bool) -> Result<BtfHeader, BtfError> {
    if data.len() < 8 {
        return Err(BtfError::HeaderMissing);
    }
    if read_u16(data, 0)? != BTF_MAGIC {
        return Err(BtfError::InvalidMagic);
    }
    if data[2] != BTF_VERSION {
        return Err(BtfError::UnsupportedVersion);
    }
    if data[3] != 0 {
        return Err(BtfError::UnsupportedFlags);
    }

    let header_len = read_u32(data, 4)? as usize;
    if header_len < LEGACY_HEADER_LEN || header_len > data.len() {
        return Err(BtfError::HeaderMissing);
    }
    if header_len > CURRENT_HEADER_LEN
        && data[CURRENT_HEADER_LEN..header_len]
            .iter()
            .any(|byte| *byte != 0)
    {
        return Err(BtfError::UnsupportedHeader);
    }

    let type_offset = read_u32(data, 8)?;
    let type_len = read_u32(data, 12)?;
    let string_offset = read_u32(data, 16)?;
    let string_len = read_u32(data, 20)?;
    let (layout_offset, layout_len) = if header_len >= CURRENT_HEADER_LEN {
        (read_u32(data, 24)?, read_u32(data, 28)?)
    } else {
        (0, 0)
    };

    if type_offset & 3 != 0 || (layout_len != 0 && layout_offset & 3 != 0) {
        return Err(BtfError::InvalidSectionLayout);
    }
    if !has_base && type_len == 0 {
        return Err(BtfError::MissingTypes);
    }

    let mut sections = [(0u32, 0u32); 3];
    sections[0] = (type_offset, type_len);
    sections[1] = (string_offset, string_len);
    let count = if header_len >= CURRENT_HEADER_LEN && layout_len != 0 {
        if layout_len < 4 || layout_len % 4 != 0 {
            return Err(BtfError::InvalidSectionLayout);
        }
        sections[2] = (layout_offset, layout_len);
        3
    } else {
        2
    };
    sections[..count].sort_unstable();

    let payload_len = data.len() - header_len;
    let mut consumed = 0usize;
    for (offset, len) in &sections[..count] {
        let offset = *offset as usize;
        let len = *len as usize;
        if offset != consumed {
            return Err(BtfError::InvalidSectionLayout);
        }
        consumed = consumed
            .checked_add(len)
            .ok_or(BtfError::InvalidSectionLayout)?;
        if consumed > payload_len {
            return Err(BtfError::InvalidSectionLayout);
        }
    }
    if consumed != payload_len {
        return Err(BtfError::InvalidSectionLayout);
    }

    let str_start = header_len
        .checked_add(string_offset as usize)
        .ok_or(BtfError::InvalidSectionLayout)?;
    let str_end = str_start
        .checked_add(string_len as usize)
        .ok_or(BtfError::InvalidSectionLayout)?;
    let strings = data
        .get(str_start..str_end)
        .ok_or(BtfError::InvalidSectionLayout)?;
    if has_base && strings.is_empty() {
        // Split BTF is allowed to inherit the complete base string table.
    } else if strings.is_empty()
        || strings.last() != Some(&0)
        || (!has_base && strings.first() != Some(&0))
    {
        return Err(BtfError::InvalidStringTable);
    }

    Ok(BtfHeader {
        header_len: header_len as u32,
        type_offset,
        type_len,
        string_offset,
        string_len,
        layout_offset,
        layout_len,
    })
}

impl ModuleBtf {
    /// Copy and validate the BTF containers while the input ELF is alive.
    pub fn parse(data: &[u8], base_data: Option<&[u8]>) -> Result<Self, BtfError> {
        if let Some(base) = base_data {
            // `.BTF.base` is a standalone distilled base and must validate as
            // such before it is used to interpret split module BTF.
            parse_header(base, false)?;
        }
        let header = parse_header(data, base_data.is_some())?;
        Ok(Self {
            data: data.to_vec(),
            base_data: base_data.map(|base| base.to_vec()),
            header,
        })
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn base_data(&self) -> Option<&[u8]> {
        self.base_data.as_deref()
    }

    pub fn header(&self) -> BtfHeader {
        self.header
    }
}
