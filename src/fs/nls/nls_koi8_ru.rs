//! linux-parity: complete
//! linux-source: vendor/linux/fs/nls/nls_koi8-ru.c
//! test-origin: linux:vendor/linux/fs/nls/nls_koi8-ru.c
//! KOI8-RU overlay behavior on top of KOI8-U.

use crate::include::uapi::errno::ENAMETOOLONG;

pub const NLS_KOI8_RU_CHARSET: &str = "koi8-ru";
pub const BASE_CHARSET: &str = "koi8-u";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Koi8RuUni2Char {
    Byte(u8),
    Delegate,
    Unmapped,
    Error(i32),
}

pub const fn koi8_ru_uni2char_overlay(uni: u16, boundlen: i32) -> Koi8RuUni2Char {
    if boundlen <= 0 {
        return Koi8RuUni2Char::Error(-ENAMETOOLONG);
    }

    if (uni & 0xffaf) == 0x040e || (uni & 0xffce) == 0x254c {
        if uni == 0x040e {
            Koi8RuUni2Char::Byte(0xbe)
        } else if uni == 0x045e {
            Koi8RuUni2Char::Byte(0xae)
        } else if uni == 0x255d || uni == 0x256c {
            Koi8RuUni2Char::Unmapped
        } else {
            Koi8RuUni2Char::Delegate
        }
    } else {
        Koi8RuUni2Char::Delegate
    }
}

pub const fn koi8_ru_char2uni_overlay(byte: u8) -> Option<u16> {
    if (byte & 0xef) != 0xae {
        Some(if byte & 0x10 != 0 { 0x040e } else { 0x045e })
    } else {
        None
    }
}

pub const fn init_requires_base_charset(loaded_base: bool) -> Result<(), i32> {
    if loaded_base {
        Ok(())
    } else {
        Err(-crate::include::uapi::errno::EINVAL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn koi8_ru_overlay_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/nls/nls_koi8-ru.c"
        ));
        assert!(source.contains("#include <linux/module.h>"));
        assert!(source.contains("#include <linux/kernel.h>"));
        assert!(source.contains("#include <linux/string.h>"));
        assert!(source.contains("#include <linux/nls.h>"));
        assert!(source.contains("#include <linux/errno.h>"));
        assert!(source.contains("static struct nls_table *p_nls;"));
        assert!(source.contains("static int uni2char"));
        assert!(source.contains("return -ENAMETOOLONG;"));
        assert!(source.contains("if ((uni & 0xffaf) == 0x040e || (uni & 0xffce) == 0x254c)"));
        assert!(source.contains("out[0] = 0xbe;"));
        assert!(source.contains("out[0] = 0xae;"));
        assert!(source.contains("return p_nls->uni2char(uni, out, boundlen);"));
        assert!(source.contains("static int char2uni"));
        assert!(source.contains("(*rawstring & 0xef) != 0xae"));
        assert!(source.contains(".charset\t= \"koi8-ru\""));
        assert!(source.contains("p_nls = load_nls(\"koi8-u\");"));
        assert!(source.contains("register_nls(&table)"));
        assert!(source.contains("MODULE_DESCRIPTION(\"NLS KOI8-RU (Belarusian)\")"));
        assert!(source.contains("MODULE_LICENSE(\"Dual BSD/GPL\")"));

        assert_eq!(
            koi8_ru_uni2char_overlay(0x040e, 1),
            Koi8RuUni2Char::Byte(0xbe)
        );
        assert_eq!(
            koi8_ru_uni2char_overlay(0x045e, 1),
            Koi8RuUni2Char::Byte(0xae)
        );
        assert_eq!(
            koi8_ru_uni2char_overlay(0x255d, 1),
            Koi8RuUni2Char::Unmapped
        );
        assert_eq!(
            koi8_ru_uni2char_overlay(b'A' as u16, 1),
            Koi8RuUni2Char::Delegate
        );
        assert_eq!(
            koi8_ru_uni2char_overlay(0x040e, 0),
            Koi8RuUni2Char::Error(-36)
        );
        assert_eq!(koi8_ru_char2uni_overlay(0x00), Some(0x045e));
        assert_eq!(koi8_ru_char2uni_overlay(0x10), Some(0x040e));
        assert_eq!(koi8_ru_char2uni_overlay(0xae), None);
        assert_eq!(init_requires_base_charset(true), Ok(()));
        assert_eq!(init_requires_base_charset(false), Err(-22));
    }
}
