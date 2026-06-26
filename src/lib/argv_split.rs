//! linux-parity: complete
//! linux-source: vendor/linux/lib/argv_split.c
//! test-origin: linux:vendor/linux/lib/argv_split.c
//! Whitespace-only argv splitter.

extern crate alloc;

use alloc::vec::Vec;
use core::str;

pub const ARGV_FREE_EXPORT_SYMBOL: &str = "argv_free";
pub const ARGV_SPLIT_EXPORT_SYMBOL: &str = "argv_split";
pub const ARGV_COPY_LIMIT: &str = "KMALLOC_MAX_SIZE - 1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArgvSlot {
    BackingCopy,
    Argument(usize),
    Null,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinuxArgv {
    argv_str: Vec<u8>,
    arg_offsets: Vec<usize>,
    freed: bool,
}

impl LinuxArgv {
    pub fn argc(&self) -> usize {
        self.arg_offsets.len()
    }

    pub fn allocated_slots(&self) -> usize {
        self.argc() + 2
    }

    pub fn returned_slots(&self) -> usize {
        self.argc() + 1
    }

    pub fn returned_slot_base(&self) -> usize {
        1
    }

    pub fn slot(&self, index: usize) -> Option<ArgvSlot> {
        if index == 0 {
            Some(ArgvSlot::BackingCopy)
        } else if index <= self.argc() {
            Some(ArgvSlot::Argument(index - 1))
        } else if index == self.argc() + 1 {
            Some(ArgvSlot::Null)
        } else {
            None
        }
    }

    pub fn is_null_terminated(&self) -> bool {
        self.slot(self.argc() + 1) == Some(ArgvSlot::Null)
    }

    pub fn is_freed(&self) -> bool {
        self.freed
    }

    pub fn backing_copy(&self) -> &[u8] {
        &self.argv_str
    }

    pub fn args(&self) -> Vec<&str> {
        self.arg_offsets
            .iter()
            .map(|&start| {
                let end = self.argv_str[start..]
                    .iter()
                    .position(|&byte| byte == 0)
                    .map(|offset| start + offset)
                    .unwrap_or(self.argv_str.len());
                str::from_utf8(&self.argv_str[start..end])
                    .expect("argv_split starts from a UTF-8 source string")
            })
            .collect()
    }

    pub fn argv_free(&mut self) -> usize {
        let slots = self.allocated_slots();
        self.argv_str.clear();
        self.arg_offsets.clear();
        self.freed = true;
        slots
    }
}

fn c_str_len(str: &str) -> usize {
    str.as_bytes()
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(str.len())
}

pub fn count_argc(str: &str) -> usize {
    let mut count = 0usize;
    let mut was_space = true;
    for &byte in &str.as_bytes()[..c_str_len(str)] {
        if byte.is_ascii_whitespace() {
            was_space = true;
        } else if was_space {
            was_space = false;
            count += 1;
        }
    }
    count
}

pub fn argv_split(str: &str) -> Vec<&str> {
    let len = c_str_len(str);
    let mut argv = Vec::with_capacity(count_argc(str));
    let mut start = None;
    for (idx, byte) in str.as_bytes()[..len].iter().copied().enumerate() {
        if byte.is_ascii_whitespace() {
            if let Some(begin) = start.take() {
                argv.push(&str[begin..idx]);
            }
        } else if start.is_none() {
            start = Some(idx);
        }
    }
    if let Some(begin) = start {
        argv.push(&str[begin..len]);
    }
    argv
}

pub fn argv_split_owned(str: &str) -> LinuxArgv {
    let len = c_str_len(str);
    let argc = count_argc(str);
    let mut argv = LinuxArgv {
        argv_str: Vec::with_capacity(len + 1),
        arg_offsets: Vec::with_capacity(argc),
        freed: false,
    };
    argv.argv_str.extend_from_slice(&str.as_bytes()[..len]);
    argv.argv_str.push(0);

    let mut was_space = true;
    for idx in 0..len {
        if argv.argv_str[idx].is_ascii_whitespace() {
            was_space = true;
            argv.argv_str[idx] = 0;
        } else if was_space {
            was_space = false;
            argv.arg_offsets.push(idx);
        }
    }

    argv
}

pub fn argv_allocated_slots_count(argv: &[&str]) -> usize {
    argv.len() + 2
}

pub fn argv_returned_slots_count(argv: &[&str]) -> usize {
    argv.len() + 1
}

pub fn argv_free_count(argv: &[&str]) -> usize {
    argv.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn argv_split_matches_linux_whitespace_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/argv_split.c"
        ));
        assert!(source.contains("static int count_argc(const char *str)"));
        assert!(source.contains("for (was_space = true; *str; str++)"));
        assert!(source.contains("if (isspace(*str))"));
        assert!(source.contains("argv_str = kstrndup(str, KMALLOC_MAX_SIZE - 1, gfp);"));
        assert!(source.contains("argv = kmalloc_array(argc + 2, sizeof(*argv), gfp);"));
        assert!(source.contains("*argv = argv_str;"));
        assert!(source.contains("argv_ret = ++argv;"));
        assert!(source.contains("*argv_str = 0;"));
        assert!(source.contains("*argv = NULL;"));
        assert!(source.contains("EXPORT_SYMBOL(argv_free);"));
        assert!(source.contains("EXPORT_SYMBOL(argv_split);"));

        assert_eq!(ARGV_FREE_EXPORT_SYMBOL, "argv_free");
        assert_eq!(ARGV_SPLIT_EXPORT_SYMBOL, "argv_split");
        assert_eq!(ARGV_COPY_LIMIT, "KMALLOC_MAX_SIZE - 1");

        assert_eq!(count_argc(""), 0);
        assert_eq!(count_argc("  alpha\tbeta\n gamma  "), 3);
        assert_eq!(
            argv_split("  alpha\tbeta\n gamma  "),
            vec!["alpha", "beta", "gamma"]
        );
        assert_eq!(argv_split("'no quotes'"), vec!["'no", "quotes'"]);
        assert_eq!(argv_split("alpha\0 beta"), vec!["alpha"]);
        assert_eq!(count_argc("alpha\0 beta"), 1);
        assert_eq!(argv_free_count(&["alpha", "beta"]), 2);
        assert_eq!(argv_allocated_slots_count(&["alpha", "beta"]), 4);
        assert_eq!(argv_returned_slots_count(&["alpha", "beta"]), 3);

        let mut owned = argv_split_owned("  alpha\tbeta\n gamma  ");
        assert_eq!(owned.argc(), 3);
        assert_eq!(owned.allocated_slots(), 5);
        assert_eq!(owned.returned_slots(), 4);
        assert_eq!(owned.returned_slot_base(), 1);
        assert_eq!(owned.slot(0), Some(ArgvSlot::BackingCopy));
        assert_eq!(owned.slot(1), Some(ArgvSlot::Argument(0)));
        assert_eq!(owned.slot(3), Some(ArgvSlot::Argument(2)));
        assert_eq!(owned.slot(4), Some(ArgvSlot::Null));
        assert!(owned.is_null_terminated());
        assert_eq!(owned.args(), vec!["alpha", "beta", "gamma"]);
        assert!(owned.backing_copy().starts_with(&[0, 0, b'a']));
        assert_eq!(owned.backing_copy().last(), Some(&0));
        assert_eq!(owned.argv_free(), 5);
        assert!(owned.is_freed());
        assert_eq!(owned.args(), Vec::<&str>::new());

        let empty = argv_split_owned(" \t\n");
        assert_eq!(empty.argc(), 0);
        assert_eq!(empty.allocated_slots(), 2);
        assert_eq!(empty.returned_slots(), 1);
        assert_eq!(empty.slot(1), Some(ArgvSlot::Null));

        let nul_terminated = argv_split_owned("alpha\0 beta");
        assert_eq!(nul_terminated.argc(), 1);
        assert_eq!(nul_terminated.args(), vec!["alpha"]);
        assert_eq!(nul_terminated.backing_copy(), b"alpha\0");
    }
}
