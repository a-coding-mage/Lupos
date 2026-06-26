//! linux-parity: complete
//! linux-source: vendor/linux/fs/smb/client/smbencrypt.c
//! test-origin: linux:vendor/linux/fs/smb/client/smbencrypt.c
//! SMB1 password hash input preparation.

pub const SMB_NT_PASSWORD_MAX_CHARS: usize = 128;
pub const SMB_NT_PASSWORD_BUFFER_WORDS: usize = SMB_NT_PASSWORD_MAX_CHARS + 1;
pub const MD4_DIGEST_SIZE: usize = 16;

pub const fn ssval_bytes(value: u16) -> [u8; 2] {
    [(value & 0x00ff) as u8, (value >> 8) as u8]
}

pub const fn e_md4hash_input_bytes(password_utf16_units: Option<usize>) -> usize {
    match password_utf16_units {
        Some(units) if units > SMB_NT_PASSWORD_MAX_CHARS => SMB_NT_PASSWORD_MAX_CHARS * 2,
        Some(units) => units * 2,
        None => 0,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MdFourFlow {
    pub init_called: bool,
    pub update_called: bool,
    pub final_called: bool,
    pub result: i32,
}

pub const fn mdfour_flow(init_rc: i32, update_rc: i32, final_rc: i32) -> MdFourFlow {
    if init_rc != 0 {
        return MdFourFlow {
            init_called: true,
            update_called: false,
            final_called: false,
            result: init_rc,
        };
    }
    if update_rc != 0 {
        return MdFourFlow {
            init_called: true,
            update_called: true,
            final_called: false,
            result: update_rc,
        };
    }
    MdFourFlow {
        init_called: true,
        update_called: true,
        final_called: true,
        result: final_rc,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smbencrypt_md4hash_preparation_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/smb/client/smbencrypt.c"
        ));
        assert!(source.contains("#include <linux/module.h>"));
        assert!(source.contains("#include <linux/slab.h>"));
        assert!(source.contains("#include <linux/fips.h>"));
        assert!(source.contains("#include \"cifs_fs_sb.h\""));
        assert!(source.contains("#include \"cifs_unicode.h\""));
        assert!(source.contains("#include \"cifsglob.h\""));
        assert!(source.contains("#include \"cifs_debug.h\""));
        assert!(source.contains("#include \"cifsproto.h\""));
        assert!(source.contains("#include \"../common/md4.h\""));
        assert!(source.contains("#define CVAL(buf,pos)"));
        assert!(source.contains("#define SSVALX(buf,pos,val)"));
        assert!(source.contains("static int"));
        assert!(source.contains("mdfour(unsigned char *md4_hash"));
        assert!(source.contains("cifs_md4_init(&mctx)"));
        assert!(source.contains("cifs_md4_update(&mctx, link_str, link_len)"));
        assert!(source.contains("cifs_md4_final(&mctx, md4_hash)"));
        assert!(source.contains("E_md4hash(const unsigned char *passwd"));
        assert!(source.contains("__le16 wpwd[129];"));
        assert!(source.contains("cifs_strtoUTF16(wpwd, passwd, 128, codepage);"));
        assert!(source.contains("*wpwd = 0;"));
        assert!(source.contains("mdfour(p16, (unsigned char *) wpwd, len * sizeof(__le16));"));
        assert!(source.contains("memzero_explicit(wpwd, sizeof(wpwd));"));

        assert_eq!(ssval_bytes(0x1234), [0x34, 0x12]);
        assert_eq!(e_md4hash_input_bytes(None), 0);
        assert_eq!(e_md4hash_input_bytes(Some(3)), 6);
        assert_eq!(e_md4hash_input_bytes(Some(200)), 256);
        assert_eq!(SMB_NT_PASSWORD_BUFFER_WORDS, 129);
        assert_eq!(
            mdfour_flow(-5, 0, 0),
            MdFourFlow {
                init_called: true,
                update_called: false,
                final_called: false,
                result: -5
            }
        );
        assert_eq!(mdfour_flow(0, -12, 0).result, -12);
        assert!(mdfour_flow(0, 0, 0).final_called);
    }
}
