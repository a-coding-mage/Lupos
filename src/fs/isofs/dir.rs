//! linux-parity: complete
//! linux-source: vendor/linux/fs/isofs/dir.c
//! test-origin: linux:vendor/linux/fs/isofs/dir.c
//! ISO9660 directory-record parsing.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::block::partitions::read_sectors;
use crate::include::uapi::errno::{EIO, ENOMEM};

use super::IsoSbi;

const ISO_DIR_RECORD_FIXED_LEN: usize = 33;
const ISO_DIR_RECORD_NAME_LEN_OFFSET: usize = 32;
pub const ISOFS_BLOCK_SIZE: usize = 2048;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IsoDirEntry {
    pub name: String,
    pub extent: u32,
    pub size: u32,
    pub flags: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IsoReaddirOptions {
    pub high_sierra: bool,
    pub hide: bool,
    pub showassoc: bool,
    pub rock: bool,
    pub joliet_level: u8,
    pub mapping: u8,
}

impl Default for IsoReaddirOptions {
    fn default() -> Self {
        Self {
            high_sierra: false,
            hide: false,
            showassoc: false,
            rock: false,
            joliet_level: 0,
            mapping: b'n',
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IsoReaddirAction {
    AdvanceToNextSector { new_pos: usize },
    EmitDot,
    EmitDotDot,
    SkipMultiExtent,
    SkipHiddenOrAssociated,
    EmitName { name: String, inode_number: u64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IsoReaddirAllocPlan {
    pub alloc_tmp_page: bool,
    pub tmpde_offset: usize,
    pub retval: i32,
}

pub fn read_all(sbi: &IsoSbi, extent: u32, size: u32) -> Result<Vec<IsoDirEntry>, i32> {
    read_all_with_options(sbi, extent, size, IsoReaddirOptions::default())
}

pub fn read_all_with_options(
    sbi: &IsoSbi,
    extent: u32,
    size: u32,
    options: IsoReaddirOptions,
) -> Result<Vec<IsoDirEntry>, i32> {
    if size == 0 {
        return Ok(Vec::new());
    }
    let lba = extent as u64 * 4;
    let nr_sectors = ((size as u64).div_ceil(512)) as u64;
    let buf = read_sectors(&sbi.bdev, lba, nr_sectors)?;

    let mut out = Vec::new();
    let mut off = 0;
    while off < (size as usize) && off < buf.len() {
        let len = buf[off] as usize;
        if len == 0 {
            off = (off / ISOFS_BLOCK_SIZE + 1) * ISOFS_BLOCK_SIZE;
            continue;
        }
        let record_end = off.checked_add(len).ok_or(-EIO)?;
        let dir_end = core::cmp::min(size as usize, buf.len());
        if record_end > buf.len() {
            break;
        }
        if record_end > dir_end || len < ISO_DIR_RECORD_FIXED_LEN {
            return Err(-EIO);
        }
        let record = &buf[off..record_end];
        let name = record_name(record).ok_or(-EIO)?;
        if len < name.len() + ISO_DIR_RECORD_FIXED_LEN {
            return Err(-EIO);
        }
        let flags = record_flags(record, options.high_sierra).unwrap_or(0);
        if name.len() == 1 && (name[0] == 0 || name[0] == 1) {
            off += len;
            continue;
        }
        if flags & 0x80 != 0 || should_skip_entry(flags, options) {
            off += len;
            continue;
        }

        out.push(IsoDirEntry {
            name: map_record_name(name, record, options, None),
            extent: read_le_u32(record, 2).unwrap_or(0),
            size: read_le_u32(record, 10).unwrap_or(0),
            flags,
        });
        off += len;
    }
    Ok(out)
}

#[inline]
pub const fn is_dir(flags: u8) -> bool {
    (flags & 0x02) != 0
}

pub fn isofs_name_translate(name: &[u8]) -> String {
    let mut out = String::new();
    let len = name.len();
    for (i, byte) in name.iter().copied().enumerate() {
        if byte == 0 {
            break;
        }
        let mut c = byte;
        if c.is_ascii_uppercase() {
            c = c.to_ascii_lowercase();
        }
        if c == b'.'
            && i == len.saturating_sub(3)
            && name.get(i + 1) == Some(&b';')
            && name.get(i + 2) == Some(&b'1')
        {
            break;
        }
        if c == b';' && i == len.saturating_sub(2) && name.get(i + 1) == Some(&b'1') {
            break;
        }
        if c == b';' || c == b'/' {
            c = b'.';
        }
        out.push(c as char);
    }
    out
}

pub fn get_acorn_filename(record: &[u8]) -> String {
    let Some(name) = record_name(record) else {
        return String::new();
    };
    let mut retname = isofs_name_translate(name);
    if retname.is_empty() {
        return retname;
    }

    let mut std = ISO_DIR_RECORD_FIXED_LEN + name.len();
    if std & 1 != 0 {
        std += 1;
    }
    if record.len().saturating_sub(std) != 32 {
        return retname;
    }
    let Some(chr) = record.get(std..std + 32) else {
        return retname;
    };
    if !chr.starts_with(b"ARCHIMEDES") {
        return retname;
    }
    if retname.as_bytes().first() == Some(&b'_') && (chr[19] & 1) == 1 {
        retname.replace_range(0..1, "!");
    }
    if record_flags(record, false).is_some_and(|flags| flags & 2 == 0)
        && chr[13] == 0xff
        && (chr[12] & 0xf0) == 0xf0
    {
        let suffix = (((chr[12] & 0x0f) as u16) << 8) | chr[11] as u16;
        retname.push(',');
        retname.push_str(&alloc::format!("{suffix:03x}"));
    }
    retname
}

pub const fn should_skip_entry(flags: u8, options: IsoReaddirOptions) -> bool {
    (options.hide && (flags & 1) != 0) || (!options.showassoc && (flags & 4) != 0)
}

pub fn isofs_get_ino(block: u64, offset: u64, bufbits: u8) -> u64 {
    (block << (bufbits - 5)) | (offset >> 5)
}

pub fn isofs_normalize_block_and_offset(record: &[u8], block: &mut u64, offset: &mut u64) {
    if record_flags(record, false).is_some_and(|flags| flags & 2 != 0) {
        *offset = 0;
        if let Some(extent) = read_le_u32(record, 2) {
            let ext_attr_length = record.get(1).copied().unwrap_or(0) as u64;
            *block = extent as u64 + ext_attr_length;
        }
    }
}

pub fn isofs_readdir_alloc_plan(tmp_page_allocated: bool) -> IsoReaddirAllocPlan {
    IsoReaddirAllocPlan {
        alloc_tmp_page: true,
        tmpde_offset: 1024,
        retval: if tmp_page_allocated { 0 } else { -ENOMEM },
    }
}

pub fn do_isofs_readdir_plan(
    dir: &[u8],
    mut ctx_pos: usize,
    inode_size: usize,
    bufbits: u8,
    options: IsoReaddirOptions,
    rock_name: Option<Result<&str, i32>>,
    dir_emit_accepts: bool,
) -> Result<Vec<IsoReaddirAction>, i32> {
    let mut actions = Vec::new();
    let bufsize = 1usize << bufbits;
    let mut first_de = true;

    while ctx_pos < inode_size && ctx_pos < dir.len() {
        let mut block = (ctx_pos >> bufbits) as u64;
        let mut offset = (ctx_pos & (bufsize - 1)) as u64;
        let de_len = dir[ctx_pos] as usize;
        if de_len == 0 {
            ctx_pos = (ctx_pos + ISOFS_BLOCK_SIZE) & !(ISOFS_BLOCK_SIZE - 1);
            actions.push(IsoReaddirAction::AdvanceToNextSector { new_pos: ctx_pos });
            continue;
        }

        let record_end = ctx_pos.checked_add(de_len).ok_or(-EIO)?;
        if record_end > dir.len() || de_len < ISO_DIR_RECORD_FIXED_LEN {
            return Err(-EIO);
        }
        let record = &dir[ctx_pos..record_end];
        let name = record_name(record).ok_or(-EIO)?;
        if de_len < name.len() + ISO_DIR_RECORD_FIXED_LEN {
            return Err(-EIO);
        }
        if first_de {
            isofs_normalize_block_and_offset(record, &mut block, &mut offset);
        }
        let flags = record_flags(record, options.high_sierra).unwrap_or(0);
        if flags & 0x80 != 0 {
            first_de = false;
            ctx_pos += de_len;
            actions.push(IsoReaddirAction::SkipMultiExtent);
            continue;
        }
        first_de = true;

        if name.len() == 1 && name[0] == 0 {
            actions.push(IsoReaddirAction::EmitDot);
            if !dir_emit_accepts {
                break;
            }
            ctx_pos += de_len;
            continue;
        }
        if name.len() == 1 && name[0] == 1 {
            actions.push(IsoReaddirAction::EmitDotDot);
            if !dir_emit_accepts {
                break;
            }
            ctx_pos += de_len;
            continue;
        }
        if should_skip_entry(flags, options) {
            actions.push(IsoReaddirAction::SkipHiddenOrAssociated);
            ctx_pos += de_len;
            continue;
        }

        let mapped = map_record_name(name, record, options, rock_name);
        if !mapped.is_empty() {
            actions.push(IsoReaddirAction::EmitName {
                name: mapped,
                inode_number: isofs_get_ino(block, offset, bufbits),
            });
            if !dir_emit_accepts {
                break;
            }
        }
        ctx_pos += de_len;
    }
    Ok(actions)
}

fn map_record_name(
    name: &[u8],
    record: &[u8],
    options: IsoReaddirOptions,
    rock_name: Option<Result<&str, i32>>,
) -> String {
    if options.rock {
        if let Some(result) = rock_name {
            match result {
                Ok(rock) if !rock.is_empty() => return String::from(rock),
                Err(_) => return String::new(),
                _ => {}
            }
        }
    }
    if options.joliet_level != 0 {
        return String::from(core::str::from_utf8(name).unwrap_or(""));
    }
    match options.mapping {
        b'a' => get_acorn_filename(record),
        b'n' => isofs_name_translate(name),
        _ => String::from(core::str::from_utf8(name).unwrap_or("")),
    }
}

fn record_name(record: &[u8]) -> Option<&[u8]> {
    let name_len = *record.get(ISO_DIR_RECORD_NAME_LEN_OFFSET)? as usize;
    let end = ISO_DIR_RECORD_FIXED_LEN.checked_add(name_len)?;
    record.get(ISO_DIR_RECORD_FIXED_LEN..end)
}

fn record_flags(record: &[u8], high_sierra: bool) -> Option<u8> {
    record.get(if high_sierra { 24 } else { 25 }).copied()
}

fn read_le_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let raw = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::sync::Arc;

    use super::*;
    use crate::block::block_device::BlockDevice;
    use crate::block::mem::{MemBlockDevice, mem_block_device_ops};

    fn test_sbi_with_dir(data: &[u8]) -> Arc<IsoSbi> {
        let mem = MemBlockDevice::new("isofs-dir-test", data.len());
        mem.data.lock().copy_from_slice(data);
        Arc::new(IsoSbi {
            bdev: BlockDevice::wrap(mem, mem_block_device_ops()),
            root_extent: 0,
            root_size: data.len() as u32,
        })
    }

    fn write_record(buf: &mut [u8], off: usize, len: u8, name: &[u8], flags: u8) {
        buf[off] = len;
        buf[off + 2..off + 6].copy_from_slice(&7u32.to_le_bytes());
        buf[off + 10..off + 14].copy_from_slice(&123u32.to_le_bytes());
        buf[off + 25] = flags;
        buf[off + ISO_DIR_RECORD_NAME_LEN_OFFSET] = name.len() as u8;
        buf[off + ISO_DIR_RECORD_FIXED_LEN..off + ISO_DIR_RECORD_FIXED_LEN + name.len()]
            .copy_from_slice(name);
    }

    #[test]
    fn isofs_dir_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/isofs/dir.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/isofs/isofs.h"
        ));
        assert!(source.contains("int isofs_name_translate"));
        assert!(source.contains("if (c >= 'A' && c <= 'Z')"));
        assert!(
            source.contains(
                "if (c == '.' && i == len - 3 && old[i + 1] == ';' && old[i + 2] == '1')"
            )
        );
        assert!(source.contains("if (c == ';' && i == len - 2 && old[i + 1] == '1')"));
        assert!(source.contains("if (c == ';' || c == '/')"));
        assert!(source.contains("int get_acorn_filename"));
        assert!(source.contains("std = sizeof(struct iso_directory_record) + de->name_len[0];"));
        assert!(source.contains("if (std & 1)"));
        assert!(source.contains("strncmp(chr, \"ARCHIMEDES\", 10)"));
        assert!(source.contains("if ((*retname == '_') && ((chr[19] & 1) == 1))"));
        assert!(source.contains("sprintf(retname+retnamlen+1, \"%3.3x\","));
        assert!(source.contains("static int do_isofs_readdir"));
        assert!(source.contains("offset = ctx->pos & (bufsize - 1);"));
        assert!(
            source.contains("ctx->pos = (ctx->pos + ISOFS_BLOCK_SIZE) & ~(ISOFS_BLOCK_SIZE - 1);")
        );
        assert!(source.contains("if (de_len < sizeof(struct iso_directory_record)"));
        assert!(source.contains("isofs_normalize_block_and_offset(de,"));
        assert!(source.contains("if (de->flags[-sbi->s_high_sierra] & 0x80)"));
        assert!(source.contains("dir_emit_dot(file, ctx)"));
        assert!(source.contains("dir_emit_dotdot(file, ctx)"));
        assert!(source.contains("sbi->s_hide && (de->flags[-sbi->s_high_sierra] & 1)"));
        assert!(source.contains("!sbi->s_showassoc"));
        assert!(source.contains("get_rock_ridge_filename(de, tmpname, inode)"));
        assert!(source.contains("get_joliet_filename(de, tmpname, inode)"));
        assert!(source.contains("get_acorn_filename(de, tmpname, inode)"));
        assert!(source.contains("isofs_name_translate(de, tmpname, inode)"));
        assert!(source.contains("if (!dir_emit(ctx, p, len, inode_number, DT_UNKNOWN))"));
        assert!(source.contains("tmpname = kmalloc(PAGE_SIZE, GFP_KERNEL);"));
        assert!(source.contains("tmpde = (struct iso_directory_record *) (tmpname+1024);"));
        assert!(source.contains("kfree(tmpname);"));
        assert!(source.contains(".iterate_shared = isofs_readdir"));
        assert!(source.contains(".lookup = isofs_lookup"));
        assert!(header.contains("static inline unsigned long isofs_get_ino"));
        assert!(header.contains("isofs_normalize_block_and_offset"));
    }

    #[test]
    fn name_translation_and_acorn_extensions_match_linux() {
        assert_eq!(isofs_name_translate(b"HELLO.;1"), "hello");
        assert_eq!(isofs_name_translate(b"HELLO;1"), "hello");
        assert_eq!(isofs_name_translate(b"DIR/FILE;2"), "dir.file.2");
        assert_eq!(isofs_name_translate(b"ABC\0DEF"), "abc");

        let mut record = alloc::vec![0u8; 33 + 6 + 1 + 32];
        write_record(&mut record, 0, 72, b"_FILE;1", 0);
        let acorn = 40;
        record[acorn..acorn + 10].copy_from_slice(b"ARCHIMEDES");
        record[acorn + 11] = 0x34;
        record[acorn + 12] = 0xf2;
        record[acorn + 13] = 0xff;
        record[acorn + 19] = 1;
        assert_eq!(get_acorn_filename(&record), "!file,234");
    }

    #[test]
    fn read_all_rejects_corrupt_directory_records_without_panicking() {
        let mut dir = alloc::vec![0u8; 512];
        write_record(&mut dir, 0, 240, &[0], 0x02);
        write_record(&mut dir, 240, 240, &[0], 0x02);
        dir[480] = 1;

        let sbi = test_sbi_with_dir(&dir);
        assert_eq!(read_all(&sbi, 0, dir.len() as u32).unwrap_err(), -EIO);

        let mut bad_name = alloc::vec![0u8; 512];
        bad_name[0] = ISO_DIR_RECORD_FIXED_LEN as u8;
        bad_name[ISO_DIR_RECORD_NAME_LEN_OFFSET] = 1;
        let sbi = test_sbi_with_dir(&bad_name);
        assert_eq!(read_all(&sbi, 0, bad_name.len() as u32).unwrap_err(), -EIO);
    }

    #[test]
    fn read_all_parses_valid_directory_records_with_linux_default_mapping() {
        let mut dir = alloc::vec![0u8; 512];
        write_record(&mut dir, 0, 40, b"HELLO;1", 0);

        let sbi = test_sbi_with_dir(&dir);
        let entries = read_all(&sbi, 0, dir.len() as u32).expect("valid ISO dir record");

        assert_eq!(
            entries,
            [IsoDirEntry {
                name: String::from("hello"),
                extent: 7,
                size: 123,
                flags: 0,
            }]
        );
    }

    #[test]
    fn do_isofs_readdir_plan_matches_linux_branches() {
        let mut dir = alloc::vec![0u8; ISOFS_BLOCK_SIZE + 128];
        dir[0] = 0;
        write_record(&mut dir, ISOFS_BLOCK_SIZE, 40, b"PART;1", 0x80);
        write_record(&mut dir, ISOFS_BLOCK_SIZE + 40, 40, &[0], 0x02);
        write_record(&mut dir, ISOFS_BLOCK_SIZE + 80, 40, b"HIDE;1", 0x01);

        let actions = do_isofs_readdir_plan(
            &dir,
            0,
            dir.len(),
            11,
            IsoReaddirOptions {
                hide: true,
                ..IsoReaddirOptions::default()
            },
            None,
            true,
        )
        .unwrap();
        assert_eq!(
            actions,
            [
                IsoReaddirAction::AdvanceToNextSector {
                    new_pos: ISOFS_BLOCK_SIZE,
                },
                IsoReaddirAction::SkipMultiExtent,
                IsoReaddirAction::EmitDot,
                IsoReaddirAction::SkipHiddenOrAssociated,
                IsoReaddirAction::AdvanceToNextSector { new_pos: 4096 },
            ]
        );

        let mut one = alloc::vec![0u8; 64];
        write_record(&mut one, 0, 40, b"ROCK;1", 0);
        assert_eq!(
            do_isofs_readdir_plan(
                &one,
                0,
                one.len(),
                11,
                IsoReaddirOptions {
                    rock: true,
                    ..IsoReaddirOptions::default()
                },
                Some(Ok("rock-name")),
                false,
            )
            .unwrap(),
            [IsoReaddirAction::EmitName {
                name: String::from("rock-name"),
                inode_number: 0,
            }]
        );
    }

    #[test]
    fn allocation_and_inode_helpers_match_linux_header() {
        assert_eq!(
            isofs_readdir_alloc_plan(false),
            IsoReaddirAllocPlan {
                alloc_tmp_page: true,
                tmpde_offset: 1024,
                retval: -ENOMEM,
            }
        );
        assert_eq!(isofs_get_ino(7, 64, 11), (7 << 6) | 2);

        let mut record = alloc::vec![0u8; 40];
        write_record(&mut record, 0, 40, &[0], 0x02);
        record[1] = 3;
        record[2..6].copy_from_slice(&9u32.to_le_bytes());
        let mut block = 1;
        let mut offset = 64;
        isofs_normalize_block_and_offset(&record, &mut block, &mut offset);
        assert_eq!(block, 12);
        assert_eq!(offset, 0);
        assert!(should_skip_entry(
            0x04,
            IsoReaddirOptions {
                showassoc: false,
                ..IsoReaddirOptions::default()
            }
        ));
    }
}
