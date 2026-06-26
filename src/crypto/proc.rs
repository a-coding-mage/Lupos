//! linux-parity: complete
//! linux-source: vendor/linux/crypto/proc.c
//! test-origin: linux:vendor/linux/crypto/proc.c
//! `/proc/crypto` record formatting for crypto algorithms.

extern crate alloc;

use alloc::{format, string::String, vec::Vec};

pub const CRYPTO_ALG_TYPE_MASK: u32 = 0x0000_000f;
pub const CRYPTO_ALG_TYPE_CIPHER: u32 = 0x0000_0001;
pub const CRYPTO_ALG_LARVAL: u32 = 0x0000_0010;
pub const CRYPTO_ALG_TESTED: u32 = 0x0000_0400;
pub const CRYPTO_ALG_INTERNAL: u32 = 0x0000_2000;
pub const CRYPTO_ALG_FIPS_INTERNAL: u32 = 0x0002_0000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CipherInfo {
    pub blocksize: u32,
    pub min_keysize: u32,
    pub max_keysize: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CryptoProcKind<'a> {
    Larval,
    Cipher(CipherInfo),
    Custom(&'a [&'a str]),
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CryptoProcEntry<'a> {
    pub name: &'a str,
    pub driver: &'a str,
    pub module: &'a str,
    pub priority: i32,
    pub refcnt: u32,
    pub flags: u32,
    pub kind: CryptoProcKind<'a>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CryptoProcSeqState {
    pub read_locked: bool,
}

impl<'a> CryptoProcEntry<'a> {
    pub const fn new(name: &'a str, driver: &'a str, module: &'a str) -> Self {
        Self {
            name,
            driver,
            module,
            priority: 0,
            refcnt: 0,
            flags: 0,
            kind: CryptoProcKind::Unknown,
        }
    }
}

pub fn c_start(state: &mut CryptoProcSeqState, pos: usize, alg_count: usize) -> Option<usize> {
    state.read_locked = true;
    (pos < alg_count).then_some(pos)
}

pub fn c_next(_state: &CryptoProcSeqState, pos: &mut usize, alg_count: usize) -> Option<usize> {
    *pos += 1;
    (*pos < alg_count).then_some(*pos)
}

pub fn c_stop(state: &mut CryptoProcSeqState) {
    state.read_locked = false;
}

pub fn crypto_proc_lines(entry: &CryptoProcEntry<'_>, fips_enabled: bool) -> Vec<String> {
    let mut out = Vec::new();
    out.push(format!("name         : {}", entry.name));
    out.push(format!("driver       : {}", entry.driver));
    out.push(format!("module       : {}", entry.module));
    out.push(format!("priority     : {}", entry.priority));
    out.push(format!("refcnt       : {}", entry.refcnt));
    out.push(format!(
        "selftest     : {}",
        if entry.flags & CRYPTO_ALG_TESTED != 0 {
            "passed"
        } else {
            "unknown"
        }
    ));
    out.push(format!(
        "internal     : {}",
        if entry.flags & CRYPTO_ALG_INTERNAL != 0 {
            "yes"
        } else {
            "no"
        }
    ));

    if fips_enabled {
        out.push(format!(
            "fips         : {}",
            if entry.flags & CRYPTO_ALG_FIPS_INTERNAL != 0 {
                "no"
            } else {
                "yes"
            }
        ));
    }

    if entry.flags & CRYPTO_ALG_LARVAL != 0 {
        out.push(String::from("type         : larval"));
        out.push(format!("flags        : 0x{:x}", entry.flags));
    } else {
        match entry.kind {
            CryptoProcKind::Larval => {
                out.push(String::from("type         : larval"));
                out.push(format!("flags        : 0x{:x}", entry.flags));
            }
            CryptoProcKind::Custom(lines) => {
                for line in lines {
                    out.push(String::from(*line));
                }
            }
            CryptoProcKind::Cipher(info)
                if entry.flags & CRYPTO_ALG_TYPE_MASK == CRYPTO_ALG_TYPE_CIPHER =>
            {
                out.push(String::from("type         : cipher"));
                out.push(format!("blocksize    : {}", info.blocksize));
                out.push(format!("min keysize  : {}", info.min_keysize));
                out.push(format!("max keysize  : {}", info.max_keysize));
            }
            _ => out.push(String::from("type         : unknown")),
        }
    }

    out.push(String::new());
    out
}

pub fn crypto_init_proc_name() -> &'static str {
    "crypto"
}

pub fn crypto_exit_proc_name() -> &'static str {
    "crypto"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crypto_proc_format_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/proc.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/crypto.h"
        ));
        assert!(source.contains("down_read(&crypto_alg_sem);"));
        assert!(source.contains("return seq_list_start(&crypto_alg_list, *pos);"));
        assert!(source.contains("return seq_list_next(p, &crypto_alg_list, pos);"));
        assert!(source.contains("up_read(&crypto_alg_sem);"));
        assert!(source.contains("seq_printf(m, \"name         : %s\\n\", alg->cra_name);"));
        assert!(source.contains("(alg->cra_flags & CRYPTO_ALG_TESTED) ?"));
        assert!(source.contains("str_yes_no(alg->cra_flags & CRYPTO_ALG_INTERNAL)"));
        assert!(source.contains("str_no_yes(alg->cra_flags & CRYPTO_ALG_FIPS_INTERNAL)"));
        assert!(source.contains("seq_printf(m, \"type         : larval\\n\");"));
        assert!(source.contains("case CRYPTO_ALG_TYPE_CIPHER:"));
        assert!(source.contains("proc_create_seq(\"crypto\", 0, NULL, &crypto_seq_ops);"));
        assert!(source.contains("remove_proc_entry(\"crypto\", NULL);"));
        assert!(header.contains("#define CRYPTO_ALG_TYPE_MASK\t\t0x0000000f"));
        assert!(header.contains("#define CRYPTO_ALG_TESTED\t\t0x00000400"));
        assert!(header.contains("#define CRYPTO_ALG_FIPS_INTERNAL\t0x00020000"));

        let mut entry = CryptoProcEntry::new("aes", "aes-generic", "kernel");
        entry.priority = 100;
        entry.refcnt = 2;
        entry.flags = CRYPTO_ALG_TESTED | CRYPTO_ALG_TYPE_CIPHER;
        entry.kind = CryptoProcKind::Cipher(CipherInfo {
            blocksize: 16,
            min_keysize: 16,
            max_keysize: 32,
        });
        let mut seq = CryptoProcSeqState::default();
        assert_eq!(c_start(&mut seq, 0, 2), Some(0));
        assert!(seq.read_locked);
        let mut pos = 0;
        assert_eq!(c_next(&seq, &mut pos, 2), Some(1));
        assert_eq!(c_next(&seq, &mut pos, 2), None);
        c_stop(&mut seq);
        assert!(!seq.read_locked);

        let lines = crypto_proc_lines(&entry, true);
        assert_eq!(lines[0], "name         : aes");
        assert_eq!(lines[5], "selftest     : passed");
        assert_eq!(lines[6], "internal     : no");
        assert_eq!(lines[7], "fips         : yes");
        assert_eq!(lines[8], "type         : cipher");
        assert_eq!(lines[9], "blocksize    : 16");
        assert_eq!(lines[11], "max keysize  : 32");

        entry.flags |= CRYPTO_ALG_FIPS_INTERNAL;
        let fips_internal = crypto_proc_lines(&entry, true);
        assert_eq!(fips_internal[7], "fips         : no");

        entry.flags = CRYPTO_ALG_TESTED;
        let unknown = crypto_proc_lines(&entry, false);
        assert!(unknown.contains(&String::from("type         : unknown")));

        entry.flags = CRYPTO_ALG_LARVAL | 0x55;
        entry.kind = CryptoProcKind::Unknown;
        let larval = crypto_proc_lines(&entry, false);
        assert!(larval.contains(&String::from("type         : larval")));
        assert!(larval.contains(&String::from("flags        : 0x55")));
        assert_eq!(crypto_init_proc_name(), "crypto");
        assert_eq!(crypto_exit_proc_name(), "crypto");
    }
}
