//! linux-parity: complete
//! linux-source: vendor/linux/lib/glob.c
//! test-origin: linux:vendor/linux/lib/glob.c
//! Shell-style glob matching helper.

use core::ffi::c_char;

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("glob_match", glob_match as usize, false);
}

#[inline]
fn byte_at(bytes: &[u8], index: usize) -> u8 {
    bytes.get(index).copied().unwrap_or(0)
}

pub fn glob_match_bytes(pat: &[u8], str: &[u8]) -> bool {
    let mut pat_pos = 0usize;
    let mut str_pos = 0usize;
    let mut back_pat = None;
    let mut back_str = 0usize;

    loop {
        let c = byte_at(str, str_pos);
        str_pos += 1;
        let mut d = byte_at(pat, pat_pos);
        pat_pos += 1;

        match d {
            b'?' => {
                if c == 0 {
                    return false;
                }
            }
            b'*' => {
                if byte_at(pat, pat_pos) == 0 {
                    return true;
                }
                back_pat = Some(pat_pos);
                str_pos -= 1;
                back_str = str_pos;
            }
            b'[' => {
                if c == 0 {
                    return false;
                }

                let inverted = byte_at(pat, pat_pos) == b'!';
                let mut class_pos = if inverted { pat_pos + 1 } else { pat_pos };
                let mut a = byte_at(pat, class_pos);
                class_pos += 1;
                let mut class_match = false;
                let malformed = loop {
                    let mut b = a;

                    if a == 0 {
                        break true;
                    }

                    if byte_at(pat, class_pos) == b'-' && byte_at(pat, class_pos + 1) != b']' {
                        b = byte_at(pat, class_pos + 1);
                        if b == 0 {
                            break true;
                        }
                        class_pos += 2;
                    }
                    if a <= c && c <= b {
                        class_match = true;
                    }

                    a = byte_at(pat, class_pos);
                    class_pos += 1;
                    if a == b']' {
                        break false;
                    }
                };

                if malformed {
                    if c == d {
                        if d == 0 {
                            return true;
                        }
                        continue;
                    }
                } else {
                    if class_match != inverted {
                        pat_pos = class_pos;
                        continue;
                    }
                }

                if c == 0 {
                    return false;
                }
                let Some(saved_pat) = back_pat else {
                    return false;
                };
                pat_pos = saved_pat;
                back_str += 1;
                str_pos = back_str;
            }
            b'\\' => {
                d = byte_at(pat, pat_pos);
                pat_pos += 1;
                if c == d {
                    if d == 0 {
                        return true;
                    }
                } else {
                    if c == 0 {
                        return false;
                    }
                    let Some(saved_pat) = back_pat else {
                        return false;
                    };
                    pat_pos = saved_pat;
                    back_str += 1;
                    str_pos = back_str;
                }
            }
            _ => {
                if c == d {
                    if d == 0 {
                        return true;
                    }
                } else {
                    if c == 0 {
                        return false;
                    }
                    let Some(saved_pat) = back_pat else {
                        return false;
                    };
                    pat_pos = saved_pat;
                    back_str += 1;
                    str_pos = back_str;
                }
            }
        }
    }
}

unsafe fn c_string_bytes<'a>(ptr: *const c_char) -> &'a [u8] {
    let mut len = 0usize;
    while unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    unsafe { core::slice::from_raw_parts(ptr as *const u8, len) }
}

pub unsafe extern "C" fn glob_match(pat: *const c_char, str: *const c_char) -> bool {
    let pat = unsafe { c_string_bytes(pat) };
    let str = unsafe { c_string_bytes(str) };
    glob_match_bytes(pat, str)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GLOB_TEST_CASES: &[(&[u8], &[u8], bool)] = &[
        (b"a", b"a", true),
        (b"a", b"b", false),
        (b"a", b"aa", false),
        (b"a", b"", false),
        (b"", b"", true),
        (b"", b"a", false),
        (b"[a]", b"a", true),
        (b"[a]", b"b", false),
        (b"[!a]", b"a", false),
        (b"[!a]", b"b", true),
        (b"[ab]", b"a", true),
        (b"[ab]", b"b", true),
        (b"[ab]", b"c", false),
        (b"[!ab]", b"c", true),
        (b"[a-c]", b"b", true),
        (b"[a-c]", b"d", false),
        (b"[a-c-e-g]", b"-", true),
        (b"[a-c-e-g]", b"d", false),
        (b"[a-c-e-g]", b"f", true),
        (b"[]a-ceg-ik[]", b"a", true),
        (b"[]a-ceg-ik[]", b"]", true),
        (b"[]a-ceg-ik[]", b"[", true),
        (b"[]a-ceg-ik[]", b"h", true),
        (b"[]a-ceg-ik[]", b"f", false),
        (b"[!]a-ceg-ik[]", b"h", false),
        (b"[!]a-ceg-ik[]", b"]", false),
        (b"[!]a-ceg-ik[]", b"f", true),
        (b"?", b"a", true),
        (b"?", b"aa", false),
        (b"??", b"a", false),
        (b"?x?", b"axb", true),
        (b"?x?", b"abx", false),
        (b"?x?", b"xab", false),
        (b"*??", b"a", false),
        (b"*??", b"ab", true),
        (b"*??", b"abc", true),
        (b"*??", b"abcd", true),
        (b"??*", b"a", false),
        (b"??*", b"ab", true),
        (b"??*", b"abc", true),
        (b"??*", b"abcd", true),
        (b"?*?", b"a", false),
        (b"?*?", b"ab", true),
        (b"?*?", b"abc", true),
        (b"?*?", b"abcd", true),
        (b"*b", b"b", true),
        (b"*b", b"ab", true),
        (b"*b", b"ba", false),
        (b"*b", b"bb", true),
        (b"*b", b"abb", true),
        (b"*b", b"bab", true),
        (b"*bc", b"abbc", true),
        (b"*bc", b"bc", true),
        (b"*bc", b"bbc", true),
        (b"*bc", b"bcbc", true),
        (b"*ac*", b"abacadaeafag", true),
        (b"*ac*ae*ag*", b"abacadaeafag", true),
        (b"*a*b*[bc]*[ef]*g*", b"abacadaeafag", true),
        (b"*a*b*[ef]*[cd]*g*", b"abacadaeafag", false),
        (b"*abcd*", b"abcabcabcabcdefg", true),
        (b"*ab*cd*", b"abcabcabcabcdefg", true),
        (b"*abcd*abcdef*", b"abcabcdabcdeabcdefg", true),
        (b"*abcd*", b"abcabcabcabcefg", false),
        (b"*ab*cd*", b"abcabcabcabcefg", false),
    ];

    #[test]
    fn glob_kunit_vectors_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/tests/glob_kunit.c"
        ));
        assert!(source.contains("static const struct glob_test_case glob_test_cases[]"));
        assert!(source.contains("{ .pat = \"[a-c-e-g]\", .str = \"-\", .expected = true }"));
        assert!(source.contains("KUNIT_CASE_PARAM(glob_test_match, glob_gen_params)"));
        assert!(source.contains(".name = \"glob\""));
        assert_eq!(GLOB_TEST_CASES.len(), 64);

        for &(pat, str, expected) in GLOB_TEST_CASES {
            assert_eq!(glob_match_bytes(pat, str), expected, "{pat:?} {str:?}");
        }
    }

    #[test]
    fn glob_source_control_flow_edges_are_preserved() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/glob.c"
        ));
        assert!(source.contains("back_pat = pat;"));
        assert!(source.contains("back_str = --str;"));
        assert!(source.contains("goto literal;"));
        assert!(source.contains("d = *pat++;"));
        assert!(source.contains("EXPORT_SYMBOL(glob_match);"));

        assert!(glob_match_bytes(b"[abc", b"[abc"));
        assert!(!glob_match_bytes(b"[abc", b"a"));
        assert!(glob_match_bytes(b"\\", b""));
        assert!(!glob_match_bytes(b"\\", b"\\"));

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("glob_match"),
            Some(glob_match as usize)
        );
    }
}
