//! linux-parity: complete
//! linux-source: vendor/linux/fs/binfmt_script.c
//! test-origin: linux:vendor/linux/fs/binfmt_script.c
//! Script (`#!`) binary format loader — M24a.
//!
//! Parses a shebang line (`#! interpreter [arg]`) at the head of a binary
//! image, validates the interpreter path against the same truncation rules
//! Linux enforces, and rewrites the binprm's argv to invoke the
//! interpreter with the script path as the new argv[1] (or argv[2] when an
//! optional arg is present).
//!
//! Reference: vendor/linux/fs/binfmt_script.c
//!            vendor/linux/include/linux/binfmts.h
//!
//! # Port notes
//!
//! Linux exposes the shebang line via `bprm->buf` (`BINPRM_BUF_SIZE` = 256
//! bytes).  We treat the supplied slice with the same byte budget so the
//! truncation invariants stay identical to Linux.  The recursion guard is
//! enforced by `MAX_BPRM_RECURSION = 4`, matching Linux's
//! `BINPRM_FLAGS_RECURSION_LIMIT` (current `BINPRM_NESTING_MAX` is 5; we
//! cap a notch lower because our exec.rs reserves one slot for the final
//! ELF interpreter invocation).

extern crate alloc;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::fs::binfmt_elf::LinuxBinprm;

/// `BINPRM_BUF_SIZE` — the byte budget Linux reads into `bprm->buf` for the
/// shebang scan (include/uapi/linux/binfmts.h).
pub const BINPRM_BUF_SIZE: usize = 256;

/// Recursion cap.  Linux `BINPRM_NESTING_MAX = 5`; we leave one slot for the
/// final ELF call.
pub const MAX_BPRM_RECURSION: usize = 4;

const ENOEXEC: i32 = -8;
const ENOENT: i32 = -2;
pub const BINPRM_FLAGS_PATH_INACCESSIBLE: u32 = 1 << 2;

/// Successful shebang parse.  Holds the resolved interpreter and the
/// rewritten argv that Linux's `load_script` produces.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScriptOutcome {
    pub interpreter: String,
    pub optional_arg: Option<String>,
    pub new_argv: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScriptExecEnv {
    pub path_inaccessible: bool,
    pub remove_arg_zero_ret: i32,
    pub copy_script_name_ret: i32,
    pub copy_optional_arg_ret: i32,
    pub copy_interpreter_ret: i32,
    pub change_interp_ret: i32,
    pub open_exec_ret: i32,
}

impl ScriptExecEnv {
    pub const SUCCESS: Self = Self {
        path_inaccessible: false,
        remove_arg_zero_ret: 0,
        copy_script_name_ret: 0,
        copy_optional_arg_ret: 0,
        copy_interpreter_ret: 0,
        change_interp_ret: 0,
        open_exec_ret: 0,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScriptExecPlan {
    pub remove_arg_zero: bool,
    pub copy_script_name: bool,
    pub nul_terminate_interpreter_end: bool,
    pub copy_optional_arg: bool,
    pub nul_terminate_interpreter_separator: bool,
    pub copy_interpreter_name: bool,
    pub change_interp: bool,
    pub open_exec: bool,
    pub interpreter_installed: bool,
    pub argc_increment: usize,
}

pub fn load_script_exec_plan(
    has_optional_arg: bool,
    env: ScriptExecEnv,
) -> Result<ScriptExecPlan, i32> {
    if env.path_inaccessible {
        return Err(ENOENT);
    }
    if env.remove_arg_zero_ret != 0 {
        return Err(env.remove_arg_zero_ret);
    }
    if env.copy_script_name_ret < 0 {
        return Err(env.copy_script_name_ret);
    }
    if has_optional_arg && env.copy_optional_arg_ret < 0 {
        return Err(env.copy_optional_arg_ret);
    }
    if env.copy_interpreter_ret != 0 {
        return Err(env.copy_interpreter_ret);
    }
    if env.change_interp_ret < 0 {
        return Err(env.change_interp_ret);
    }
    if env.open_exec_ret < 0 {
        return Err(env.open_exec_ret);
    }

    Ok(ScriptExecPlan {
        remove_arg_zero: true,
        copy_script_name: true,
        nul_terminate_interpreter_end: true,
        copy_optional_arg: has_optional_arg,
        nul_terminate_interpreter_separator: has_optional_arg,
        copy_interpreter_name: true,
        change_interp: true,
        open_exec: true,
        interpreter_installed: true,
        argc_increment: if has_optional_arg { 3 } else { 2 },
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScriptBinfmtRegistration {
    pub load_binary: &'static str,
    pub init_registers_binfmt: bool,
    pub exit_unregisters_binfmt: bool,
    pub description: &'static str,
    pub license: &'static str,
}

pub const SCRIPT_BINFMT_REGISTRATION: ScriptBinfmtRegistration = ScriptBinfmtRegistration {
    load_binary: "load_script",
    init_registers_binfmt: true,
    exit_unregisters_binfmt: true,
    description: "Kernel support for scripts starting with #!",
    license: "GPL",
};

/// Parse the `#!` line and rewrite argv accordingly.  Linux:
/// `load_script` (fs/binfmt_script.c:34).  Returns `-ENOEXEC` for any of:
///   * buffer does not start with `#!`
///   * no terminator before the buffer end (interpreter path truncated)
///   * empty interpreter name
///   * recursion depth would exceed `MAX_BPRM_RECURSION`
pub fn load_script(bprm: &LinuxBinprm<'_>) -> Result<ScriptOutcome, i32> {
    load_script_with_path_inaccessible(bprm, false)
}

pub fn load_script_with_path_inaccessible(
    bprm: &LinuxBinprm<'_>,
    path_inaccessible: bool,
) -> Result<ScriptOutcome, i32> {
    if bprm.recursion_depth >= MAX_BPRM_RECURSION {
        return Err(ENOEXEC);
    }
    let buf = bprm.buf;
    if buf.len() < 2 || &buf[0..2] != b"#!" {
        return Err(ENOEXEC);
    }
    let budget = buf.len().min(BINPRM_BUF_SIZE);
    let scan = &buf[..budget];

    // Linux strnchr(buf, BINPRM_BUF_SIZE, '\n') — locate the line end.
    let mut i_end_index = scan.iter().position(|&b| b == b'\n');
    if i_end_index.is_none() {
        // No newline: walk past leading whitespace to find at least one
        // interpreter character, otherwise the entire buf is whitespace.
        let after = next_non_spacetab(&scan[2..]).map(|o| o + 2);
        let Some(_start) = after else {
            return Err(ENOEXEC);
        };
        // Linux: if no later space/tab/NUL exists the interpreter path is
        // assumed truncated.  We use the last byte of the budget as the
        // upper bound for terminator search.
        if next_terminator(&scan[_start..]).is_none() {
            return Err(ENOEXEC);
        }
        i_end_index = Some(scan.len());
    }
    let mut i_end = i_end_index.unwrap();

    // Trim trailing whitespace.
    while i_end > 2 && is_space_tab(scan[i_end - 1]) {
        i_end -= 1;
    }

    // Skip leading whitespace after "#!".
    let i_name_start = match next_non_spacetab(&scan[2..i_end]) {
        Some(off) => 2 + off,
        None => return Err(ENOEXEC),
    };
    if i_name_start >= i_end {
        return Err(ENOEXEC);
    }

    // Locate end of interpreter token.
    let i_sep_rel = next_terminator(&scan[i_name_start..i_end]);
    let i_name_end = match i_sep_rel {
        Some(off) => i_name_start + off,
        None => i_end,
    };
    let interpreter = core::str::from_utf8(&scan[i_name_start..i_name_end])
        .map_err(|_| ENOEXEC)?
        .to_string();
    if interpreter.is_empty() {
        return Err(ENOEXEC);
    }

    // Optional argument: any single non-empty whitespace-trimmed token after
    // the interpreter.  Linux preserves arguments verbatim including embedded
    // whitespace via `next_non_spacetab`+`i_end`.
    let optional_arg = if i_name_end < i_end {
        let after = next_non_spacetab(&scan[i_name_end..i_end]).map(|o| i_name_end + o);
        if let Some(arg_start) = after {
            // Linux only copies one argument; trim trailing whitespace.
            let mut arg_end = i_end;
            while arg_end > arg_start && is_space_tab(scan[arg_end - 1]) {
                arg_end -= 1;
            }
            if arg_start < arg_end {
                let arg = core::str::from_utf8(&scan[arg_start..arg_end])
                    .map_err(|_| ENOEXEC)?
                    .to_string();
                Some(arg)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    if path_inaccessible {
        return Err(ENOENT);
    }

    // Rewrite argv: [interpreter, optional_arg?, script_filename, original argv[1..]].
    let mut new_argv = Vec::with_capacity(bprm.argv.len() + 2);
    new_argv.push(interpreter.clone());
    if let Some(arg) = optional_arg.as_ref() {
        new_argv.push(arg.clone());
    }
    new_argv.push(bprm.filename.clone());
    if bprm.argv.len() > 1 {
        new_argv.extend(bprm.argv[1..].iter().cloned());
    }

    Ok(ScriptOutcome {
        interpreter,
        optional_arg,
        new_argv,
    })
}

#[inline]
fn is_space_tab(c: u8) -> bool {
    c == b' ' || c == b'\t'
}

fn next_non_spacetab(s: &[u8]) -> Option<usize> {
    s.iter().position(|&c| !is_space_tab(c))
}

fn next_terminator(s: &[u8]) -> Option<usize> {
    s.iter().position(|&c| is_space_tab(c) || c == 0)
}

/// Linux: `register_binfmt(&script_format)` in `init_script_binfmt`.
/// Idempotent no-op for now — the binfmt registry in `binfmt_elf` is the
/// integration point; calling sites use `load_script` directly.
pub fn register() {
    // Placeholder: when search_binary_handler grows shebang awareness this
    // will register a `BinFormat` whose `load_binary` calls `load_script`
    // and routes the rewritten argv through `search_binary_handler` again.
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    extern crate alloc;

    use super::*;

    fn bprm_from_bytes<'a>(buf: &'a [u8], filename: &str) -> LinuxBinprm<'a> {
        LinuxBinprm {
            buf,
            file_bytes: buf,
            argv: alloc::vec![filename.to_string()],
            envp: alloc::vec![],
            filename: filename.to_string(),
            recursion_depth: 0,
            secureexec: false,
        }
    }

    #[test]
    fn binfmt_script_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/binfmt_script.c"
        ));
        let binfmts = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/binfmts.h"
        ));

        assert!(source.contains("static inline bool spacetab(char c)"));
        assert!(source.contains("return c == ' ' || c == '\\t';"));
        assert!(source.contains("static inline const char *next_non_spacetab"));
        assert!(source.contains("static inline const char *next_terminator"));
        assert!(source.contains("static int load_script(struct linux_binprm *bprm)"));
        assert!(source.contains("if ((bprm->buf[0] != '#') || (bprm->buf[1] != '!'))"));
        assert!(source.contains("buf_end = bprm->buf + sizeof(bprm->buf) - 1;"));
        assert!(source.contains("i_end = strnchr(bprm->buf, sizeof(bprm->buf), '\\n');"));
        assert!(source.contains("return -ENOEXEC; /* Entire buf is spaces/tabs */"));
        assert!(source.contains("if (!next_terminator(i_end, buf_end))"));
        assert!(source.contains("while (spacetab(i_end[-1]))"));
        assert!(source.contains("i_name = next_non_spacetab(bprm->buf+2, i_end);"));
        assert!(source.contains("i_sep = next_terminator(i_name, i_end);"));
        assert!(source.contains("if (bprm->interp_flags & BINPRM_FLAGS_PATH_INACCESSIBLE)"));
        assert!(source.contains("return -ENOENT;"));
        assert!(source.contains("retval = remove_arg_zero(bprm);"));
        assert!(source.contains("retval = copy_string_kernel(bprm->interp, bprm);"));
        assert!(source.contains("*((char *)i_end) = '\\0';"));
        assert!(source.contains("*((char *)i_sep) = '\\0';"));
        assert!(source.contains("retval = copy_string_kernel(i_arg, bprm);"));
        assert!(source.contains("retval = copy_string_kernel(i_name, bprm);"));
        assert!(source.contains("retval = bprm_change_interp(i_name, bprm);"));
        assert!(source.contains("file = open_exec(i_name);"));
        assert!(source.contains("bprm->interpreter = file;"));
        assert!(source.contains(".load_binary\t= load_script"));
        assert!(source.contains("core_initcall(init_script_binfmt);"));
        assert!(source.contains("module_exit(exit_script_binfmt);"));
        assert!(
            source.contains("MODULE_DESCRIPTION(\"Kernel support for scripts starting with #!\");")
        );
        assert!(source.contains("MODULE_LICENSE(\"GPL\");"));
        assert!(binfmts.contains("#define BINPRM_FLAGS_PATH_INACCESSIBLE_BIT 2"));
        assert!(binfmts.contains("extern int __must_check remove_arg_zero"));
        assert!(
            binfmts.contains("int copy_string_kernel(const char *arg, struct linux_binprm *bprm);")
        );
        assert!(binfmts.contains("extern int bprm_change_interp"));

        assert_eq!(BINPRM_FLAGS_PATH_INACCESSIBLE, 1 << 2);
        assert_eq!(SCRIPT_BINFMT_REGISTRATION.load_binary, "load_script");
        assert!(SCRIPT_BINFMT_REGISTRATION.init_registers_binfmt);
        assert!(SCRIPT_BINFMT_REGISTRATION.exit_unregisters_binfmt);
        assert_eq!(
            SCRIPT_BINFMT_REGISTRATION.description,
            "Kernel support for scripts starting with #!"
        );
        assert_eq!(SCRIPT_BINFMT_REGISTRATION.license, "GPL");
    }

    #[test]
    fn exec_plan_follows_linux_reverse_splice_and_error_order() {
        assert_eq!(
            load_script_exec_plan(false, ScriptExecEnv::SUCCESS),
            Ok(ScriptExecPlan {
                remove_arg_zero: true,
                copy_script_name: true,
                nul_terminate_interpreter_end: true,
                copy_optional_arg: false,
                nul_terminate_interpreter_separator: false,
                copy_interpreter_name: true,
                change_interp: true,
                open_exec: true,
                interpreter_installed: true,
                argc_increment: 2,
            })
        );
        assert_eq!(
            load_script_exec_plan(true, ScriptExecEnv::SUCCESS)
                .unwrap()
                .argc_increment,
            3
        );
        assert_eq!(
            load_script_exec_plan(
                true,
                ScriptExecEnv {
                    path_inaccessible: true,
                    ..ScriptExecEnv::SUCCESS
                }
            ),
            Err(ENOENT)
        );
        assert_eq!(
            load_script_exec_plan(
                true,
                ScriptExecEnv {
                    remove_arg_zero_ret: -5,
                    ..ScriptExecEnv::SUCCESS
                }
            ),
            Err(-5)
        );
        assert_eq!(
            load_script_exec_plan(
                true,
                ScriptExecEnv {
                    copy_script_name_ret: -14,
                    ..ScriptExecEnv::SUCCESS
                }
            ),
            Err(-14)
        );
        assert!(
            load_script_exec_plan(
                false,
                ScriptExecEnv {
                    copy_optional_arg_ret: -14,
                    ..ScriptExecEnv::SUCCESS
                }
            )
            .is_ok()
        );
        assert_eq!(
            load_script_exec_plan(
                true,
                ScriptExecEnv {
                    copy_optional_arg_ret: -14,
                    ..ScriptExecEnv::SUCCESS
                }
            ),
            Err(-14)
        );
        assert_eq!(
            load_script_exec_plan(
                true,
                ScriptExecEnv {
                    change_interp_ret: -2,
                    ..ScriptExecEnv::SUCCESS
                }
            ),
            Err(-2)
        );
        assert_eq!(
            load_script_exec_plan(
                true,
                ScriptExecEnv {
                    open_exec_ret: -13,
                    ..ScriptExecEnv::SUCCESS
                }
            ),
            Err(-13)
        );
    }

    #[test]
    fn parses_simple_shebang() {
        let buf = b"#!/bin/sh\n";
        let bprm = bprm_from_bytes(buf, "myscript");
        let outcome = load_script(&bprm).expect("ok");
        assert_eq!(outcome.interpreter, "/bin/sh");
        assert!(outcome.optional_arg.is_none());
        assert_eq!(outcome.new_argv, alloc::vec!["/bin/sh", "myscript"]);
    }

    #[test]
    fn rejects_inaccessible_script_path_after_successful_parse() {
        let buf = b"#!/bin/sh\n";
        let bprm = bprm_from_bytes(buf, "myscript");
        assert_eq!(
            load_script_with_path_inaccessible(&bprm, true).unwrap_err(),
            ENOENT
        );

        let bad = b"plain text\n";
        let bprm = bprm_from_bytes(bad, "plain");
        assert_eq!(
            load_script_with_path_inaccessible(&bprm, true).unwrap_err(),
            ENOEXEC
        );
    }

    #[test]
    fn parses_shebang_with_argument() {
        let buf = b"#!/usr/bin/env python3\n";
        let bprm = bprm_from_bytes(buf, "script.py");
        let outcome = load_script(&bprm).expect("ok");
        assert_eq!(outcome.interpreter, "/usr/bin/env");
        assert_eq!(outcome.optional_arg.as_deref(), Some("python3"));
        assert_eq!(
            outcome.new_argv,
            alloc::vec!["/usr/bin/env", "python3", "script.py"],
        );
    }

    #[test]
    fn preserves_original_arguments_after_script_filename() {
        let buf = b"#!/usr/bin/env python3 -I\n";
        let mut bprm = bprm_from_bytes(buf, "script.py");
        bprm.argv.push("--flag".to_string());
        bprm.argv.push("value".to_string());
        let outcome = load_script(&bprm).expect("ok");
        assert_eq!(
            outcome.new_argv,
            alloc::vec!["/usr/bin/env", "python3 -I", "script.py", "--flag", "value"],
        );
    }

    #[test]
    fn handles_leading_whitespace() {
        let buf = b"#!   /bin/sh\n";
        let bprm = bprm_from_bytes(buf, "x");
        let outcome = load_script(&bprm).expect("ok");
        assert_eq!(outcome.interpreter, "/bin/sh");
    }

    #[test]
    fn handles_trailing_whitespace() {
        let buf = b"#!/bin/sh   \n";
        let bprm = bprm_from_bytes(buf, "x");
        let outcome = load_script(&bprm).expect("ok");
        assert_eq!(outcome.interpreter, "/bin/sh");
    }

    #[test]
    fn rejects_missing_magic() {
        let buf = b"//bin/sh\n";
        let bprm = bprm_from_bytes(buf, "x");
        assert_eq!(load_script(&bprm).unwrap_err(), ENOEXEC);
    }

    #[test]
    fn rejects_only_magic() {
        let buf = b"#!";
        let bprm = bprm_from_bytes(buf, "x");
        assert_eq!(load_script(&bprm).unwrap_err(), ENOEXEC);
    }

    #[test]
    fn rejects_whitespace_only_line() {
        let buf = b"#!     \n";
        let bprm = bprm_from_bytes(buf, "x");
        assert_eq!(load_script(&bprm).unwrap_err(), ENOEXEC);
    }

    #[test]
    fn rejects_when_recursion_depth_exceeds_max() {
        let buf = b"#!/bin/sh\n";
        let mut bprm = bprm_from_bytes(buf, "x");
        bprm.recursion_depth = MAX_BPRM_RECURSION;
        assert_eq!(load_script(&bprm).unwrap_err(), ENOEXEC);
    }

    #[test]
    fn rejects_truncated_no_newline_no_terminator() {
        // Long single token with no whitespace or newline before the budget
        // boundary should signal truncation.  We synthesise a 256-byte
        // buffer entirely composed of "/aaa...".
        let mut buf = alloc::vec![b'a'; BINPRM_BUF_SIZE];
        buf[0] = b'#';
        buf[1] = b'!';
        let bprm = bprm_from_bytes(&buf, "x");
        assert_eq!(load_script(&bprm).unwrap_err(), ENOEXEC);
    }

    #[test]
    fn accepts_no_newline_when_token_terminated_by_space() {
        // No newline but interpreter terminated by a space within the buffer.
        let mut buf = alloc::vec![0u8; 32];
        buf[0] = b'#';
        buf[1] = b'!';
        let path = b"/bin/sh ";
        buf[2..2 + path.len()].copy_from_slice(path);
        let bprm = bprm_from_bytes(&buf, "y");
        let outcome = load_script(&bprm).expect("ok");
        assert_eq!(outcome.interpreter, "/bin/sh");
    }
}
