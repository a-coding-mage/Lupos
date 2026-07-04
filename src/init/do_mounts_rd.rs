//! linux-parity: complete
//! linux-source: vendor/linux/init/do_mounts_rd.c
//! test-origin: linux:vendor/linux/init/do_mounts_rd.c
//! Legacy initial ramdisk image probing.
//!
//! This module is a pure decision model of `do_mounts_rd.c`: it preserves the
//! parser/probe/load branch contracts without doing kernel file I/O.

use crate::include::uapi::errno::{EINVAL, ENOEXEC, ENOMEM, EOPNOTSUPP};

pub const BLOCK_SIZE: usize = 1024;
pub const BLOCK_SIZE_BITS: u32 = 10;
pub const PROBE_SIZE: usize = 512;
pub const CRAMFS_MAGIC_LE: [u8; 4] = [0x45, 0x3d, 0xcd, 0x28];
pub const SQUASHFS_MAGIC_LE: [u8; 4] = [0x68, 0x73, 0x71, 0x73];
pub const MINIX_SUPER_MAGIC: u16 = 0x137f;
pub const MINIX_SUPER_MAGIC2: u16 = 0x138f;
pub const EXT2_SUPER_MAGIC: u16 = 0xef53;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RamdiskImageKind {
    Gzip,
    Bzip2,
    Lzma,
    Xz,
    Lzo,
    Lz4,
    Romfs,
    Cramfs,
    Squashfs,
    Minix,
    Ext2,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RamdiskIdentification {
    pub kind: RamdiskImageKind,
    pub nblocks: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RamdiskStartSetupPlan {
    pub accepted: bool,
    pub rd_image_start: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RdLoadDecision {
    InvalidImage { errno: i32 },
    Decompress { kind: RamdiskImageKind },
    CopyBlocks { nblocks: u64 },
    ImageTooBig { nblocks: u64, rd_blocks: u64 },
    MissingRamdiskDevice,
    Unsupported { kind: RamdiskImageKind, errno: i32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RdLoadImagePlan {
    pub out_opened: bool,
    pub in_opened: bool,
    pub identify_called: bool,
    pub crd_load_called: bool,
    pub rd_blocks_checked: bool,
    pub buffer_allocated: bool,
    pub copy_blocks: u64,
    pub fput_input: bool,
    pub fput_output: bool,
    pub unlink_dev_ram: bool,
    pub result: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComprFillPlan {
    pub retval: i64,
    pub log_read_error: bool,
    pub log_eof: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComprFlushPlan {
    pub retval: i64,
    pub set_decompress_error: bool,
    pub log_incomplete_write: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecompressErrorPlan {
    pub exit_code: i32,
    pub decompress_error: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CrdLoadPlan {
    PanicNoDecompressor,
    Done { result: i32, decompress_error: bool },
}

pub fn ramdisk_start_setup(arg: &str) -> Result<i32, i32> {
    parse_int(arg).ok_or(-EINVAL)
}

pub fn ramdisk_start_setup_plan(arg: &str) -> RamdiskStartSetupPlan {
    match parse_int(arg) {
        Some(value) => RamdiskStartSetupPlan {
            accepted: true,
            rd_image_start: value,
        },
        None => RamdiskStartSetupPlan {
            accepted: false,
            rd_image_start: 0,
        },
    }
}

pub fn identify_ramdisk_image(bytes: &[u8], rd_image_start: u32) -> RamdiskIdentification {
    let Some(start) = (rd_image_start as usize).checked_mul(BLOCK_SIZE) else {
        return unknown();
    };
    if start >= bytes.len() {
        return unknown();
    }
    let image = &bytes[start..];

    if let Some(kind) = compressed_kind(image) {
        return RamdiskIdentification { kind, nblocks: 0 };
    }

    if image.starts_with(b"-rom1fs-") {
        let size = read_be_u32(image, 8).unwrap_or(0) as u64;
        return blocks_for_size(RamdiskImageKind::Romfs, size);
    }

    if image.starts_with(&CRAMFS_MAGIC_LE) {
        let size = read_le_u32(image, 4).unwrap_or(0) as u64;
        return blocks_for_size(RamdiskImageKind::Cramfs, size);
    }

    if image.starts_with(&SQUASHFS_MAGIC_LE) {
        let size = read_le_u64(image, 40).unwrap_or(0);
        return blocks_for_size(RamdiskImageKind::Squashfs, size);
    }

    if image
        .get(0x200..)
        .is_some_and(|tail| tail.starts_with(&CRAMFS_MAGIC_LE))
    {
        let size = read_le_u32(image, 0x204).unwrap_or(0) as u64;
        return blocks_for_size(RamdiskImageKind::Cramfs, size);
    }

    let Some(block1) = image.get(BLOCK_SIZE..BLOCK_SIZE + PROBE_SIZE) else {
        return unknown();
    };

    let minix_magic = read_le_u16(block1, 16).unwrap_or(0);
    if minix_magic == MINIX_SUPER_MAGIC || minix_magic == MINIX_SUPER_MAGIC2 {
        let zones = read_le_u16(block1, 2).unwrap_or(0) as i64;
        let log_zone_size = read_le_u16(block1, 10).unwrap_or(0) as u32;
        return RamdiskIdentification {
            kind: RamdiskImageKind::Minix,
            nblocks: zones.checked_shl(log_zone_size).unwrap_or(i64::MAX),
        };
    }

    if read_le_u16(block1, 0x38).unwrap_or(0) == EXT2_SUPER_MAGIC {
        let blocks = read_le_u32(block1, 0x04).unwrap_or(0) as i64;
        let block_shift = read_le_u32(block1, 0x18).unwrap_or(0);
        return RamdiskIdentification {
            kind: RamdiskImageKind::Ext2,
            nblocks: blocks.checked_shl(block_shift).unwrap_or(i64::MAX),
        };
    }

    unknown()
}

pub fn rd_load_decision(
    identification: RamdiskIdentification,
    rd_blocks: u64,
    decompressor_configured: bool,
) -> RdLoadDecision {
    if identification.nblocks < 0 {
        return RdLoadDecision::InvalidImage { errno: ENOEXEC };
    }
    if identification.kind == RamdiskImageKind::Unknown {
        return RdLoadDecision::InvalidImage { errno: ENOEXEC };
    }
    if identification.nblocks == 0 {
        if decompressor_configured {
            return RdLoadDecision::Decompress {
                kind: identification.kind,
            };
        }
        return RdLoadDecision::Unsupported {
            kind: identification.kind,
            errno: EOPNOTSUPP,
        };
    }
    if identification.nblocks as u64 > rd_blocks {
        return RdLoadDecision::ImageTooBig {
            nblocks: identification.nblocks as u64,
            rd_blocks,
        };
    }
    RdLoadDecision::CopyBlocks {
        nblocks: identification.nblocks as u64,
    }
}

pub const fn nr_blocks(is_block_device: bool, inode_size: u64) -> u64 {
    if is_block_device {
        inode_size >> BLOCK_SIZE_BITS
    } else {
        0
    }
}

pub const fn rd_load_image_plan(
    out_open_ok: bool,
    in_open_ok: bool,
    identification: RamdiskIdentification,
    crd_load_result: i32,
    rd_blocks: u64,
    buffer_alloc_succeeds: bool,
) -> RdLoadImagePlan {
    if !out_open_ok {
        return RdLoadImagePlan {
            out_opened: false,
            in_opened: false,
            identify_called: false,
            crd_load_called: false,
            rd_blocks_checked: false,
            buffer_allocated: false,
            copy_blocks: 0,
            fput_input: false,
            fput_output: false,
            unlink_dev_ram: true,
            result: 0,
        };
    }
    if !in_open_ok {
        return RdLoadImagePlan {
            out_opened: true,
            in_opened: false,
            identify_called: false,
            crd_load_called: false,
            rd_blocks_checked: false,
            buffer_allocated: false,
            copy_blocks: 0,
            fput_input: false,
            fput_output: true,
            unlink_dev_ram: true,
            result: 0,
        };
    }

    let mut plan = RdLoadImagePlan {
        out_opened: true,
        in_opened: true,
        identify_called: true,
        crd_load_called: false,
        rd_blocks_checked: false,
        buffer_allocated: false,
        copy_blocks: 0,
        fput_input: true,
        fput_output: true,
        unlink_dev_ram: true,
        result: 0,
    };

    if identification.nblocks < 0 {
        return plan;
    }
    if identification.nblocks == 0 {
        plan.crd_load_called = true;
        plan.result = if crd_load_result == 0 { 1 } else { 0 };
        return plan;
    }

    plan.rd_blocks_checked = true;
    let nblocks = identification.nblocks as u64;
    if nblocks > rd_blocks {
        return plan;
    }
    if !buffer_alloc_succeeds {
        return plan;
    }

    plan.buffer_allocated = true;
    plan.copy_blocks = nblocks;
    plan.result = 1;
    plan
}

pub const fn compr_fill_plan(read_result: i64) -> ComprFillPlan {
    ComprFillPlan {
        retval: read_result,
        log_read_error: read_result < 0,
        log_eof: read_result == 0,
    }
}

pub const fn compr_flush_plan(
    written: i64,
    outcnt: u64,
    decompress_error_already_set: bool,
) -> ComprFlushPlan {
    if written == outcnt as i64 {
        ComprFlushPlan {
            retval: written,
            set_decompress_error: false,
            log_incomplete_write: false,
        }
    } else {
        ComprFlushPlan {
            retval: -1,
            set_decompress_error: true,
            log_incomplete_write: !decompress_error_already_set,
        }
    }
}

pub const fn decompressor_error_plan() -> DecompressErrorPlan {
    DecompressErrorPlan {
        exit_code: 1,
        decompress_error: true,
    }
}

pub const fn crd_load_plan(
    decompressor_configured: bool,
    decompress_result: i32,
    decompress_error: bool,
) -> CrdLoadPlan {
    if !decompressor_configured {
        return CrdLoadPlan::PanicNoDecompressor;
    }
    CrdLoadPlan::Done {
        result: if decompress_error {
            1
        } else {
            decompress_result
        },
        decompress_error,
    }
}

fn compressed_kind(image: &[u8]) -> Option<RamdiskImageKind> {
    if image.starts_with(b"\x1f\x8b") {
        Some(RamdiskImageKind::Gzip)
    } else if image.starts_with(b"BZh") {
        Some(RamdiskImageKind::Bzip2)
    } else if image.starts_with(&[0x5d, 0x00, 0x00]) {
        Some(RamdiskImageKind::Lzma)
    } else if image.starts_with(b"\xfd7zXZ\x00") {
        Some(RamdiskImageKind::Xz)
    } else if image.starts_with(b"\x89LZO") {
        Some(RamdiskImageKind::Lzo)
    } else if image.starts_with(&[0x04, 0x22, 0x4d, 0x18]) {
        Some(RamdiskImageKind::Lz4)
    } else {
        None
    }
}

const fn unknown() -> RamdiskIdentification {
    RamdiskIdentification {
        kind: RamdiskImageKind::Unknown,
        nblocks: -1,
    }
}

const fn blocks_for_size(kind: RamdiskImageKind, size: u64) -> RamdiskIdentification {
    RamdiskIdentification {
        kind,
        nblocks: ((size + BLOCK_SIZE as u64 - 1) >> BLOCK_SIZE_BITS) as i64,
    }
}

fn parse_int(arg: &str) -> Option<i32> {
    let (negative, body) = if let Some(rest) = arg.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = arg.strip_prefix('+') {
        (false, rest)
    } else {
        (false, arg)
    };
    let (radix, digits) = if let Some(hex) = body.strip_prefix("0x") {
        (16, hex)
    } else if let Some(hex) = body.strip_prefix("0X") {
        (16, hex)
    } else if let Some(octal) = body.strip_prefix('0') {
        (8, if octal.is_empty() { "0" } else { octal })
    } else {
        (10, body)
    };
    let value = i32::from_str_radix(digits, radix).ok()?;
    if negative {
        value.checked_neg()
    } else {
        Some(value)
    }
}

fn read_le_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let raw = bytes.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([raw[0], raw[1]]))
}

fn read_le_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let raw = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn read_be_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let raw = bytes.get(offset..offset + 4)?;
    Some(u32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn read_le_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    let raw = bytes.get(offset..offset + 8)?;
    Some(u64::from_le_bytes([
        raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
    ]))
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use super::*;
    use alloc::vec;

    #[test]
    fn do_mounts_rd_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/init/do_mounts_rd.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/init/do_mounts.h"
        ));
        let squashfs = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/squashfs/squashfs_fs.h"
        ));
        assert!(source.contains("int __initdata rd_image_start;"));
        assert!(source.contains("return kstrtoint(str, 0, &rd_image_start) == 0;"));
        assert!(source.contains("__setup(\"ramdisk_start=\", ramdisk_start_setup);"));
        assert!(source.contains("identify_ramdisk_image(struct file *file, loff_t pos"));
        assert!(source.contains("const int size = 512;"));
        assert!(source.contains("pos = start_block * BLOCK_SIZE;"));
        assert!(source.contains("*decompressor = decompress_method(buf, size, &compress_name);"));
        assert!(source.contains("pos = start_block * BLOCK_SIZE + 0x200;"));
        assert!(source.contains("pos = (start_block + 1) * BLOCK_SIZE;"));
        assert!(source.contains("nblocks = minixsb->s_nzones << minixsb->s_log_zone_size;"));
        assert!(source.contains("n = ext2_image_size(buf);"));
        assert!(source.contains("static unsigned long nr_blocks(struct file *file)"));
        assert!(source.contains("if (!S_ISBLK(inode->i_mode))"));
        assert!(source.contains("return i_size_read(inode) >> 10;"));
        assert!(source.contains("int __init rd_load_image(void)"));
        assert!(source.contains("out_file = filp_open(\"/dev/ram\", O_RDWR, 0);"));
        assert!(source.contains("in_file = filp_open(\"/initrd.image\", O_RDONLY, 0);"));
        assert!(source.contains("if (nblocks < 0)"));
        assert!(source.contains("if (nblocks == 0)"));
        assert!(source.contains("if (crd_load(decompressor) == 0)"));
        assert!(source.contains("rd_blocks = nr_blocks(out_file);"));
        assert!(source.contains("if (nblocks > rd_blocks)"));
        assert!(source.contains("buf = kmalloc(BLOCK_SIZE, GFP_KERNEL);"));
        assert!(source.contains("for (i = 0; i < nblocks; i++)"));
        assert!(source.contains("kernel_read(in_file, buf, BLOCK_SIZE, &in_pos);"));
        assert!(source.contains("kernel_write(out_file, buf, BLOCK_SIZE, &out_pos);"));
        assert!(source.contains("init_unlink(\"/dev/ram\");"));
        assert!(source.contains("static long __init compr_fill"));
        assert!(source.contains("RAMDISK: error while reading compressed data"));
        assert!(source.contains("RAMDISK: EOF while reading compressed data"));
        assert!(source.contains("static long __init compr_flush"));
        assert!(source.contains("RAMDISK: incomplete write"));
        assert!(source.contains("static void __init error(char *x)"));
        assert!(source.contains("exit_code = 1;"));
        assert!(source.contains("decompress_error = 1;"));
        assert!(source.contains("static int __init crd_load(decompress_fn deco)"));
        assert!(source.contains("panic(\"Could not decompress initial ramdisk image.\");"));
        assert!(source.contains("if (decompress_error)"));
        assert!(header.contains("int __init rd_load_image(void);"));
        assert!(header.contains("static inline int rd_load_image(void) { return 0; }"));
        assert!(squashfs.contains("#define SQUASHFS_MAJOR"));

        assert_eq!(
            ramdisk_start_setup_plan("0x20"),
            RamdiskStartSetupPlan {
                accepted: true,
                rd_image_start: 32,
            }
        );
        assert_eq!(
            ramdisk_start_setup_plan("not-a-number"),
            RamdiskStartSetupPlan {
                accepted: false,
                rd_image_start: 0,
            }
        );
        assert_eq!(nr_blocks(true, 4096), 4);
        assert_eq!(nr_blocks(false, 4096), 0);
    }

    #[test]
    fn detects_compressed_images_at_ramdisk_start_block() {
        let mut image = vec![0u8; BLOCK_SIZE + 8];
        image[BLOCK_SIZE..BLOCK_SIZE + 3].copy_from_slice(b"BZh");
        assert_eq!(
            identify_ramdisk_image(&image, 1),
            RamdiskIdentification {
                kind: RamdiskImageKind::Bzip2,
                nblocks: 0,
            }
        );
    }

    #[test]
    fn detects_romfs_cramfs_squashfs_and_ext2_block_counts() {
        let mut romfs = vec![0u8; 64];
        romfs[..8].copy_from_slice(b"-rom1fs-");
        romfs[8..12].copy_from_slice(&(4097u32).to_be_bytes());
        assert_eq!(
            identify_ramdisk_image(&romfs, 0),
            RamdiskIdentification {
                kind: RamdiskImageKind::Romfs,
                nblocks: 5,
            }
        );

        let mut cramfs = vec![0u8; 0x208];
        cramfs[0x200..0x204].copy_from_slice(&CRAMFS_MAGIC_LE);
        cramfs[0x204..0x208].copy_from_slice(&(2048u32).to_le_bytes());
        assert_eq!(identify_ramdisk_image(&cramfs, 0).nblocks, 2);

        let mut squashfs = vec![0u8; 64];
        squashfs[..4].copy_from_slice(&SQUASHFS_MAGIC_LE);
        squashfs[40..48].copy_from_slice(&(3073u64).to_le_bytes());
        assert_eq!(identify_ramdisk_image(&squashfs, 0).nblocks, 4);

        let mut ext2 = vec![0u8; BLOCK_SIZE + PROBE_SIZE];
        ext2[BLOCK_SIZE + 0x38..BLOCK_SIZE + 0x3a].copy_from_slice(&EXT2_SUPER_MAGIC.to_le_bytes());
        ext2[BLOCK_SIZE + 0x04..BLOCK_SIZE + 0x08].copy_from_slice(&(8u32).to_le_bytes());
        ext2[BLOCK_SIZE + 0x18..BLOCK_SIZE + 0x1c].copy_from_slice(&(1u32).to_le_bytes());
        assert_eq!(
            identify_ramdisk_image(&ext2, 0),
            RamdiskIdentification {
                kind: RamdiskImageKind::Ext2,
                nblocks: 16,
            }
        );
    }

    #[test]
    fn rd_load_decision_matches_linux_branches() {
        let compressed = RamdiskIdentification {
            kind: RamdiskImageKind::Gzip,
            nblocks: 0,
        };
        assert_eq!(
            rd_load_decision(compressed, 0, true),
            RdLoadDecision::Decompress {
                kind: RamdiskImageKind::Gzip,
            }
        );
        assert_eq!(
            rd_load_decision(compressed, 0, false),
            RdLoadDecision::Unsupported {
                kind: RamdiskImageKind::Gzip,
                errno: EOPNOTSUPP,
            }
        );

        let ext2 = RamdiskIdentification {
            kind: RamdiskImageKind::Ext2,
            nblocks: 5,
        };
        assert_eq!(
            rd_load_decision(ext2, 0, true),
            RdLoadDecision::ImageTooBig {
                nblocks: 5,
                rd_blocks: 0,
            }
        );
        assert_eq!(
            rd_load_decision(ext2, 4, true),
            RdLoadDecision::ImageTooBig {
                nblocks: 5,
                rd_blocks: 4,
            }
        );
        assert_eq!(
            rd_load_decision(ext2, 5, true),
            RdLoadDecision::CopyBlocks { nblocks: 5 }
        );
    }

    #[test]
    fn rd_load_image_plan_matches_linux_goto_paths() {
        let ext2 = RamdiskIdentification {
            kind: RamdiskImageKind::Ext2,
            nblocks: 5,
        };
        assert_eq!(
            rd_load_image_plan(false, true, ext2, 0, 5, true),
            RdLoadImagePlan {
                out_opened: false,
                in_opened: false,
                identify_called: false,
                crd_load_called: false,
                rd_blocks_checked: false,
                buffer_allocated: false,
                copy_blocks: 0,
                fput_input: false,
                fput_output: false,
                unlink_dev_ram: true,
                result: 0,
            }
        );
        assert_eq!(
            rd_load_image_plan(true, false, ext2, 0, 5, true),
            RdLoadImagePlan {
                out_opened: true,
                in_opened: false,
                identify_called: false,
                crd_load_called: false,
                rd_blocks_checked: false,
                buffer_allocated: false,
                copy_blocks: 0,
                fput_input: false,
                fput_output: true,
                unlink_dev_ram: true,
                result: 0,
            }
        );
        assert_eq!(
            rd_load_image_plan(true, true, unknown(), 0, 5, true).result,
            0
        );

        let compressed = RamdiskIdentification {
            kind: RamdiskImageKind::Gzip,
            nblocks: 0,
        };
        let plan = rd_load_image_plan(true, true, compressed, 0, 0, false);
        assert!(plan.crd_load_called);
        assert_eq!(plan.result, 1);
        assert!(!plan.buffer_allocated);
        assert_eq!(
            rd_load_image_plan(true, true, compressed, 1, 0, false).result,
            0
        );

        let too_big = rd_load_image_plan(true, true, ext2, 0, 4, true);
        assert!(too_big.rd_blocks_checked);
        assert_eq!(too_big.result, 0);
        assert_eq!(too_big.copy_blocks, 0);

        let alloc_fail = rd_load_image_plan(true, true, ext2, 0, 5, false);
        assert!(alloc_fail.rd_blocks_checked);
        assert!(!alloc_fail.buffer_allocated);
        assert_eq!(alloc_fail.result, 0);

        let copied = rd_load_image_plan(true, true, ext2, 0, 5, true);
        assert!(copied.buffer_allocated);
        assert_eq!(copied.copy_blocks, 5);
        assert_eq!(copied.result, 1);
        assert!(copied.fput_input);
        assert!(copied.fput_output);
        assert!(copied.unlink_dev_ram);
    }

    #[test]
    fn ramdisk_start_setup_accepts_base_zero_numbers() {
        assert_eq!(ramdisk_start_setup("010"), Ok(8));
        assert_eq!(ramdisk_start_setup("0x10"), Ok(16));
        assert_eq!(ramdisk_start_setup("10"), Ok(10));
    }

    #[test]
    fn compressed_ramdisk_callbacks_match_linux() {
        assert_eq!(
            compr_fill_plan(-ENOMEM as i64),
            ComprFillPlan {
                retval: -ENOMEM as i64,
                log_read_error: true,
                log_eof: false,
            }
        );
        assert_eq!(
            compr_fill_plan(0),
            ComprFillPlan {
                retval: 0,
                log_read_error: false,
                log_eof: true,
            }
        );
        assert_eq!(
            compr_flush_plan(128, 128, false),
            ComprFlushPlan {
                retval: 128,
                set_decompress_error: false,
                log_incomplete_write: false,
            }
        );
        assert_eq!(
            compr_flush_plan(64, 128, false),
            ComprFlushPlan {
                retval: -1,
                set_decompress_error: true,
                log_incomplete_write: true,
            }
        );
        assert_eq!(
            decompressor_error_plan(),
            DecompressErrorPlan {
                exit_code: 1,
                decompress_error: true,
            }
        );
        assert_eq!(
            crd_load_plan(false, 0, false),
            CrdLoadPlan::PanicNoDecompressor
        );
        assert_eq!(
            crd_load_plan(true, 0, true),
            CrdLoadPlan::Done {
                result: 1,
                decompress_error: true,
            }
        );
        assert_eq!(
            crd_load_plan(true, 7, false),
            CrdLoadPlan::Done {
                result: 7,
                decompress_error: false,
            }
        );
    }
}
