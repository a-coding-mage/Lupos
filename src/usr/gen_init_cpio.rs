//! linux-parity: partial
//! linux-source: vendor/linux/usr/gen_init_cpio.c
//! test-origin: linux:vendor/linux/usr/gen_init_cpio.c
//! Build-time initramfs `newc` cpio generator rules.

pub const CPIO_HDR_LEN: usize = 110;
pub const CPIO_TRAILER: &str = "TRAILER!!!";
pub const CPIO_MAGIC_NEWC: &str = "070701";
pub const CPIO_MAGIC_CRC: &str = "070702";
pub const INITIAL_INO: u32 = 721;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CpioEntryType {
    File,
    Node,
    Directory,
    Symlink,
    Pipe,
    Socket,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileHandler {
    pub entry_type: CpioEntryType,
    pub token: &'static str,
    pub handler: &'static str,
}

pub const FILE_HANDLERS: &[FileHandler] = &[
    FileHandler {
        entry_type: CpioEntryType::File,
        token: "file",
        handler: "cpio_mkfile_line",
    },
    FileHandler {
        entry_type: CpioEntryType::Node,
        token: "nod",
        handler: "cpio_mknod_line",
    },
    FileHandler {
        entry_type: CpioEntryType::Directory,
        token: "dir",
        handler: "cpio_mkdir_line",
    },
    FileHandler {
        entry_type: CpioEntryType::Symlink,
        token: "slink",
        handler: "cpio_mkslink_line",
    },
    FileHandler {
        entry_type: CpioEntryType::Pipe,
        token: "pipe",
        handler: "cpio_mkpipe_line",
    },
    FileHandler {
        entry_type: CpioEntryType::Socket,
        token: "sock",
        handler: "cpio_mksock_line",
    },
];

pub const fn cpio_magic(do_csum: bool) -> &'static str {
    if do_csum {
        CPIO_MAGIC_CRC
    } else {
        CPIO_MAGIC_NEWC
    }
}

pub const fn padlen(offset: usize, align: usize) -> usize {
    (align - (offset & (align - 1))) % align
}

pub fn handler_for_token(token: &str) -> Option<FileHandler> {
    FILE_HANDLERS
        .iter()
        .copied()
        .find(|handler| handler.token == token)
}

pub fn archive_name(name: &str) -> &str {
    name.strip_prefix('/').unwrap_or(name)
}

pub const fn timestamp_valid(mtime: i64) -> bool {
    mtime >= 0 && mtime <= 0xffff_ffff
}

pub const fn data_align_valid(align: u32) -> bool {
    (align & 3) == 0
}

pub fn line_is_comment_or_blank(line: &str) -> bool {
    let trimmed = line.trim_start_matches([' ', '\t']);
    trimmed.is_empty() || trimmed.starts_with('#') || trimmed == "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gen_init_cpio_rules_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/usr/gen_init_cpio.c"
        ));
        assert!(source.contains("#define CPIO_HDR_LEN 110"));
        assert!(source.contains("#define CPIO_TRAILER \"TRAILER!!!\""));
        assert!(source.contains("do_csum ? \"070702\" : \"070701\""));
        assert!(source.contains("static unsigned int ino = 721;"));
        assert!(source.contains("#define padlen(_off, _align)"));
        assert!(source.contains("static const struct file_handler file_handler_table[]"));
        assert!(source.contains(".type    = \"file\""));
        assert!(source.contains(".type    = \"nod\""));
        assert!(source.contains(".type    = \"dir\""));
        assert!(source.contains(".type    = \"slink\""));
        assert!(source.contains(".type    = \"pipe\""));
        assert!(source.contains(".type    = \"sock\""));
        assert!(source.contains("getopt(argc, argv, \"t:cho:a:\")"));
        assert!(source.contains("default_mtime > 0xffffffff || default_mtime < 0"));
        assert!(source.contains("dalign = strtoul(optarg, &invalid, 10);"));
        assert!(source.contains("(dalign & 3)"));

        assert_eq!(cpio_magic(false), CPIO_MAGIC_NEWC);
        assert_eq!(cpio_magic(true), CPIO_MAGIC_CRC);
        assert_eq!(padlen(CPIO_HDR_LEN + 11, 4), 3);
        assert_eq!(
            handler_for_token("slink").map(|handler| handler.entry_type),
            Some(CpioEntryType::Symlink)
        );
        assert_eq!(handler_for_token("missing"), None);
        assert_eq!(archive_name("/dev/console"), "dev/console");
        assert_eq!(archive_name("etc/passwd"), "etc/passwd");
        assert!(timestamp_valid(0xffff_ffff));
        assert!(!timestamp_valid(-1));
        assert!(!timestamp_valid(0x1_0000_0000));
        assert!(data_align_valid(512));
        assert!(!data_align_valid(6));
        assert!(line_is_comment_or_blank("# comment"));
        assert!(line_is_comment_or_blank("   "));
        assert!(!line_is_comment_or_blank("file /init init 0755 0 0"));
    }
}
