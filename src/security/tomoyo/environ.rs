//! linux-parity: complete
//! linux-source: vendor/linux/security/tomoyo/environ.c
//! test-origin: linux:vendor/linux/security/tomoyo/environ.c
//! TOMOYO environment-variable ACL parsing and matching.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::include::uapi::errno::EINVAL;

pub const TOMOYO_TYPE_ENV_ACL: u8 = 7;
pub const TOMOYO_RETRY_REQUEST: i32 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TomoyoEnvAcl {
    pub acl_type: u8,
    pub env: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TomoyoEnvPermReport {
    pub allowed: bool,
    pub param_type: u8,
    pub name: Option<String>,
    pub audit_attempts: usize,
}

pub fn tomoyo_check_env_acl(env_name: &str, acl: &TomoyoEnvAcl) -> bool {
    tomoyo_path_matches_pattern(env_name, &acl.env)
}

pub fn tomoyo_env_perm(env: Option<&str>, acls: &[TomoyoEnvAcl]) -> Result<bool, i32> {
    let Some(env) = env else {
        return Ok(true);
    };
    if env.is_empty() {
        return Ok(true);
    }
    Ok(acls.iter().any(|acl| tomoyo_check_env_acl(env, acl)))
}

pub fn tomoyo_env_perm_with_audit(
    env: Option<&str>,
    acls: &[TomoyoEnvAcl],
    audit_results: &[i32],
) -> Result<TomoyoEnvPermReport, i32> {
    let Some(env) = env else {
        return Ok(TomoyoEnvPermReport {
            allowed: true,
            param_type: 0,
            name: None,
            audit_attempts: 0,
        });
    };
    if env.is_empty() {
        return Ok(TomoyoEnvPermReport {
            allowed: true,
            param_type: 0,
            name: None,
            audit_attempts: 0,
        });
    }

    let allowed = acls.iter().any(|acl| tomoyo_check_env_acl(env, acl));
    let mut attempts = 0;
    loop {
        let error = audit_results.get(attempts).copied().unwrap_or(0);
        attempts += 1;
        if error != TOMOYO_RETRY_REQUEST {
            return if error == 0 {
                Ok(TomoyoEnvPermReport {
                    allowed,
                    param_type: TOMOYO_TYPE_ENV_ACL,
                    name: Some(String::from(env)),
                    audit_attempts: attempts,
                })
            } else {
                Err(error)
            };
        }
    }
}

pub fn tomoyo_write_misc(data: &str) -> Result<TomoyoEnvAcl, i32> {
    let Some(rest) = data.strip_prefix("env ") else {
        return Err(-EINVAL);
    };
    tomoyo_write_env(rest)
}

pub fn tomoyo_write_env(data: &str) -> Result<TomoyoEnvAcl, i32> {
    let token = read_token(data);
    if !tomoyo_correct_word(token) || token.as_bytes().contains(&b'=') {
        return Err(-EINVAL);
    }
    Ok(TomoyoEnvAcl {
        acl_type: TOMOYO_TYPE_ENV_ACL,
        env: String::from(token),
    })
}

pub fn tomoyo_correct_word(string: &str) -> bool {
    let bytes = string.as_bytes();
    if bytes.is_empty() {
        return false;
    }

    let mut index = 0;
    let mut recursion = 20u8;
    let mut in_repetition = false;
    while index < bytes.len() {
        let mut c = bytes[index];
        index += 1;
        if c == b'\\' {
            if index >= bytes.len() {
                return false;
            }
            c = bytes[index];
            index += 1;
            if (b'0'..=b'3').contains(&c) {
                if index + 1 >= bytes.len() {
                    return false;
                }
                let d = bytes[index];
                let e = bytes[index + 1];
                index += 2;
                if !(b'0'..=b'7').contains(&d) || !(b'0'..=b'7').contains(&e) {
                    return false;
                }
                let decoded = ((c - b'0') << 6) | ((d - b'0') << 3) | (e - b'0');
                if decoded <= b' ' || decoded >= 127 {
                    continue;
                }
                return false;
            }
            if matches!(c, b'\\' | b'+' | b'?' | b'x' | b'a' | b'-') {
                continue;
            }
            if recursion == 0 {
                return false;
            }
            recursion -= 1;
            match c {
                b'*' | b'@' | b'$' | b'X' | b'A' => continue,
                b'{' => {
                    if index < 3 || bytes[index - 3] != b'/' {
                        return false;
                    }
                    in_repetition = true;
                }
                b'}' => {
                    if bytes.get(index).copied() != Some(b'/') || !in_repetition {
                        return false;
                    }
                    in_repetition = false;
                }
                _ => return false,
            }
        } else if (in_repetition && c == b'/') || c <= b' ' || c >= 127 {
            return false;
        }
    }
    !in_repetition
}

pub fn tomoyo_path_matches_pattern(filename: &str, pattern: &str) -> bool {
    if !pattern.as_bytes().contains(&b'\\') {
        return filename == pattern;
    }
    path_matches_pattern2(filename.as_bytes(), pattern.as_bytes())
}

fn read_token(data: &str) -> &str {
    data.split_ascii_whitespace().next().unwrap_or("")
}

fn path_matches_pattern2(filename: &[u8], pattern: &[u8]) -> bool {
    let mut f_parts = split_components(filename);
    let mut p_parts = split_components(pattern);
    while !f_parts.is_empty() && !p_parts.is_empty() {
        if !file_matches_pattern(f_parts[0], p_parts[0]) {
            return false;
        }
        f_parts.remove(0);
        p_parts.remove(0);
    }
    if !f_parts.is_empty() {
        return false;
    }
    while let Some(part) = p_parts.first() {
        if matches!(*part, b"\\*" | b"\\@") {
            p_parts.remove(0);
        } else {
            break;
        }
    }
    p_parts.is_empty()
}

fn split_components(bytes: &[u8]) -> Vec<&[u8]> {
    bytes.split(|byte| *byte == b'/').collect()
}

fn file_matches_pattern(filename: &[u8], pattern: &[u8]) -> bool {
    let mut start = 0;
    let mut first = true;
    let mut index = 0;
    while index + 1 < pattern.len() {
        if pattern[index] == b'\\' && pattern[index + 1] == b'-' {
            let result = file_matches_pattern2(filename, &pattern[start..index]);
            let reject = if first { !result } else { result };
            if reject {
                return false;
            }
            first = false;
            start = index + 2;
            index += 2;
        } else {
            index += 1;
        }
    }
    let result = file_matches_pattern2(filename, &pattern[start..]);
    if first { result } else { !result }
}

fn file_matches_pattern2(filename: &[u8], pattern: &[u8]) -> bool {
    if pattern.is_empty() {
        return filename.is_empty();
    }
    if filename.is_empty() {
        return trailing_star_pattern(pattern);
    }

    if pattern[0] != b'\\' {
        return filename[0] == pattern[0] && file_matches_pattern2(&filename[1..], &pattern[1..]);
    }
    if pattern.len() < 2 {
        return false;
    }

    match pattern[1] {
        b'?' => filename[0] != b'/' && file_matches_pattern2(&filename[1..], &pattern[2..]),
        b'\\' => {
            filename.len() >= 2
                && filename[0] == b'\\'
                && filename[1] == b'\\'
                && file_matches_pattern2(&filename[2..], &pattern[2..])
        }
        b'+' => {
            filename[0].is_ascii_digit() && file_matches_pattern2(&filename[1..], &pattern[2..])
        }
        b'x' => {
            filename[0].is_ascii_hexdigit() && file_matches_pattern2(&filename[1..], &pattern[2..])
        }
        b'a' => ascii_alpha(&filename[0]) && file_matches_pattern2(&filename[1..], &pattern[2..]),
        b'0'..=b'3' => {
            filename.len() >= 4
                && filename[0] == b'\\'
                && filename[1..4] == pattern[1..4]
                && file_matches_pattern2(&filename[4..], &pattern[4..])
        }
        b'*' | b'@' => {
            for len in 0..=filename.len() {
                if file_matches_pattern2(&filename[len..], &pattern[2..]) {
                    return true;
                }
                if len == filename.len() || filename[len] == b'/' {
                    break;
                }
                if pattern[1] == b'@' && filename[len] == b'.' {
                    break;
                }
            }
            false
        }
        b'$' => repeat_class(filename, &pattern[2..], u8::is_ascii_digit),
        b'X' => repeat_class(filename, &pattern[2..], u8::is_ascii_hexdigit),
        b'A' => repeat_class(filename, &pattern[2..], ascii_alpha),
        _ => false,
    }
}

fn trailing_star_pattern(mut pattern: &[u8]) -> bool {
    loop {
        match pattern {
            [b'\\', b'*', rest @ ..] | [b'\\', b'@', rest @ ..] => pattern = rest,
            _ => break,
        }
    }
    pattern.is_empty()
}

fn repeat_class(filename: &[u8], rest: &[u8], class: fn(&u8) -> bool) -> bool {
    let mut len = 0;
    while filename.get(len).is_some_and(class) {
        len += 1;
    }
    for take in 1..=len {
        if file_matches_pattern2(&filename[take..], rest) {
            return true;
        }
    }
    false
}

fn ascii_alpha(byte: &u8) -> bool {
    byte.is_ascii_alphabetic()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tomoyo_env_acl_matches_linux_environ_source() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/tomoyo/environ.c"
        ));
        let common = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/tomoyo/common.h"
        ));
        let util = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/tomoyo/util.c"
        ));

        assert!(source.contains("tomoyo_path_matches_pattern(r->param.environ.name, acl->env);"));
        assert!(source.contains("tomoyo_supervisor(r, \"misc env %s\\n\","));
        assert!(source.contains("if (!env || !*env)"));
        assert!(source.contains("tomoyo_fill_path_info(&environ);"));
        assert!(source.contains("r->param_type = TOMOYO_TYPE_ENV_ACL;"));
        assert!(source.contains("r->param.environ.name = &environ;"));
        assert!(source.contains("tomoyo_check_acl(r, tomoyo_check_env_acl);"));
        assert!(source.contains("} while (error == TOMOYO_RETRY_REQUEST);"));
        assert!(source.contains("return p1->env == p2->env;"));
        assert!(source.contains(".head.type = TOMOYO_TYPE_ENV_ACL"));
        assert!(source.contains("data = tomoyo_read_token(param);"));
        assert!(source.contains("if (!tomoyo_correct_word(data) || strchr(data, '='))"));
        assert!(source.contains("e.env = tomoyo_get_name(data);"));
        assert!(source.contains("tomoyo_update_domain(&e.head, sizeof(e), param,"));
        assert!(source.contains("tomoyo_put_name(e.env);"));
        assert!(source.contains("if (tomoyo_str_starts(&param->data, \"env \"))"));
        assert!(common.contains("TOMOYO_TYPE_ENV_ACL,"));
        assert!(util.contains("tomoyo_correct_word(const char *string)"));

        let acl = tomoyo_write_misc("env PATH").expect("env acl");
        assert_eq!(acl.acl_type, TOMOYO_TYPE_ENV_ACL);
        assert!(tomoyo_check_env_acl("PATH", &acl));
        assert!(!tomoyo_check_env_acl("HOME", &acl));
        assert_eq!(tomoyo_env_perm(None, core::slice::from_ref(&acl)), Ok(true));
        assert_eq!(
            tomoyo_env_perm(Some(""), core::slice::from_ref(&acl)),
            Ok(true)
        );
        assert_eq!(
            tomoyo_env_perm(Some("PATH"), core::slice::from_ref(&acl)),
            Ok(true)
        );
        assert_eq!(tomoyo_env_perm(Some("HOME"), &[acl]), Ok(false));
    }

    #[test]
    fn tomoyo_env_permission_models_retrying_audit_loop() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let acl = tomoyo_write_misc("env LD_\\*").expect("env acl");
        let report =
            tomoyo_env_perm_with_audit(Some("LD_PRELOAD"), &[acl], &[TOMOYO_RETRY_REQUEST, 0])
                .expect("retry then allow");
        assert!(report.allowed);
        assert_eq!(report.param_type, TOMOYO_TYPE_ENV_ACL);
        assert_eq!(report.name.as_deref(), Some("LD_PRELOAD"));
        assert_eq!(report.audit_attempts, 2);

        assert_eq!(
            tomoyo_env_perm_with_audit(Some("PATH"), &[], &[TOMOYO_RETRY_REQUEST, -EINVAL]),
            Err(-EINVAL)
        );
        assert_eq!(
            tomoyo_env_perm_with_audit(None, &[], &[])
                .unwrap()
                .audit_attempts,
            0
        );
    }

    #[test]
    fn tomoyo_env_writer_rejects_invalid_names_and_equals() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        assert_eq!(tomoyo_write_misc("file PATH").unwrap_err(), -EINVAL);
        assert_eq!(tomoyo_write_misc("env A=B").unwrap_err(), -EINVAL);
        assert_eq!(tomoyo_write_misc("env ").unwrap_err(), -EINVAL);
        assert!(tomoyo_write_misc("env LD_\\*").is_ok());
    }

    #[test]
    fn tomoyo_pattern_subset_matches_environment_names() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let wildcard = tomoyo_write_misc("env LD_\\*").expect("wildcard acl");
        assert!(tomoyo_check_env_acl("LD_PRELOAD", &wildcard));
        assert!(tomoyo_check_env_acl("LD_", &wildcard));
        assert!(!tomoyo_check_env_acl("PATH", &wildcard));

        let digit = tomoyo_write_misc("env VAR\\+").expect("digit acl");
        assert!(tomoyo_check_env_acl("VAR7", &digit));
        assert!(!tomoyo_check_env_acl("VARX", &digit));
    }
}
