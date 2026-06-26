//! linux-parity: complete
//! linux-source: vendor/linux/lib/ctype.c
//! test-origin: linux:vendor/linux/lib/ctype.c
//! Linux ctype classification table.

use crate::kernel::module::{export_symbol, find_symbol};

pub const _U: u8 = 0x01;
pub const _L: u8 = 0x02;
pub const _D: u8 = 0x04;
pub const _C: u8 = 0x08;
pub const _P: u8 = 0x10;
pub const _S: u8 = 0x20;
pub const _X: u8 = 0x40;
pub const _SP: u8 = 0x80;

pub static _CTYPE: [u8; 256] = build_ctype();

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("_ctype", _CTYPE.as_ptr() as usize, false);
}

pub const fn ctype(byte: u8) -> u8 {
    _CTYPE[byte as usize]
}

pub const fn isdigit(byte: u8) -> bool {
    byte >= b'0' && byte <= b'9'
}

pub const fn isxdigit(byte: u8) -> bool {
    (ctype(byte) & (_D | _X)) != 0
}

pub const fn isspace(byte: u8) -> bool {
    (ctype(byte) & _S) != 0
}

pub const fn isalpha(byte: u8) -> bool {
    (ctype(byte) & (_U | _L)) != 0
}

const fn build_ctype() -> [u8; 256] {
    let mut table = [0u8; 256];
    let mut idx = 0usize;
    while idx < 256 {
        table[idx] = ctype_entry(idx as u8);
        idx += 1;
    }
    table
}

const fn ctype_entry(byte: u8) -> u8 {
    if byte < 32 {
        if byte >= 9 && byte <= 13 { _C | _S } else { _C }
    } else if byte == b' ' {
        _S | _SP
    } else if byte >= b'0' && byte <= b'9' {
        _D
    } else if byte >= b'A' && byte <= b'F' {
        _U | _X
    } else if byte >= b'G' && byte <= b'Z' {
        _U
    } else if byte >= b'a' && byte <= b'f' {
        _L | _X
    } else if byte >= b'g' && byte <= b'z' {
        _L
    } else if byte == 127 {
        _C
    } else if byte < 128 {
        _P
    } else if byte < 160 {
        0
    } else if byte == 160 {
        _S | _SP
    } else if byte < 192 {
        _P
    } else if byte <= 214 {
        _U
    } else if byte == 215 {
        _P
    } else if byte <= 222 {
        _U
    } else if byte <= 246 {
        _L
    } else if byte == 247 {
        _P
    } else {
        _L
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctype_table_matches_linux_classes_and_export() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/ctype.c"
        ));
        assert!(source.contains("const unsigned char _ctype[]"));
        assert!(source.contains("_C,_C|_S,_C|_S,_C|_S,_C|_S,_C|_S,_C,_C"));
        assert!(source.contains("_S|_SP,_P,_P,_P,_P,_P,_P,_P"));
        assert!(source.contains("_D,_D,_D,_D,_D,_D,_D,_D"));
        assert!(source.contains("_P,_U|_X,_U|_X,_U|_X"));
        assert!(source.contains("EXPORT_SYMBOL(_ctype);"));
        assert!(isspace(b'\n'));
        assert!(isspace(b' '));
        assert!(isdigit(b'9'));
        assert!(isxdigit(b'f'));
        assert!(isalpha(b'Z'));
        assert_eq!(ctype(127), _C);
        assert_eq!(ctype(160), _S | _SP);
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("_ctype"),
            Some(_CTYPE.as_ptr() as usize)
        );
    }
}
