//! linux-parity: complete
//! linux-source: vendor/linux/lib/decompress.c
//! test-origin: linux:vendor/linux/lib/decompress.c
//! Compression-format magic detection.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompressFormat {
    pub magic: [u8; 2],
    pub name: &'static str,
}

pub const COMPRESSED_FORMATS: &[CompressFormat] = &[
    CompressFormat {
        magic: [0x1f, 0x8b],
        name: "gzip",
    },
    CompressFormat {
        magic: [0x1f, 0x9e],
        name: "gzip",
    },
    CompressFormat {
        magic: [0x42, 0x5a],
        name: "bzip2",
    },
    CompressFormat {
        magic: [0x5d, 0x00],
        name: "lzma",
    },
    CompressFormat {
        magic: [0xfd, 0x37],
        name: "xz",
    },
    CompressFormat {
        magic: [0x89, 0x4c],
        name: "lzo",
    },
    CompressFormat {
        magic: [0x02, 0x21],
        name: "lz4",
    },
    CompressFormat {
        magic: [0x28, 0xb5],
        name: "zstd",
    },
];

pub fn decompress_method_name(inbuf: &[u8]) -> Option<&'static str> {
    if inbuf.len() < 2 {
        return None;
    }
    COMPRESSED_FORMATS
        .iter()
        .find(|format| inbuf[..2] == format.magic)
        .map(|format| format.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decompress_method_matches_linux_magic_table() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/decompress.c"
        ));
        assert!(source.contains("struct compress_format"));
        assert!(
            source.contains("{ .magic = {0x1f, 0x8b}, .name = \"gzip\", .decompressor = gunzip }")
        );
        assert!(
            source
                .contains("{ .magic = {0x42, 0x5a}, .name = \"bzip2\", .decompressor = bunzip2 }")
        );
        assert!(source.contains("{ .magic = {0xfd, 0x37}, .name = \"xz\", .decompressor = unxz }"));
        assert!(
            source.contains("{ .magic = {0x28, 0xb5}, .name = \"zstd\", .decompressor = unzstd }")
        );
        assert!(source.contains("if (len < 2)"));
        assert!(source.contains("if (!memcmp(inbuf, cf->magic, 2))"));

        assert_eq!(decompress_method_name(&[0x1f, 0x8b, 0x08]), Some("gzip"));
        assert_eq!(decompress_method_name(&[0x1f, 0x9e]), Some("gzip"));
        assert_eq!(decompress_method_name(&[0x42, 0x5a]), Some("bzip2"));
        assert_eq!(decompress_method_name(&[0x28, 0xb5]), Some("zstd"));
        assert_eq!(decompress_method_name(&[0x00]), None);
        assert_eq!(decompress_method_name(&[0xff, 0xff]), None);
    }
}
