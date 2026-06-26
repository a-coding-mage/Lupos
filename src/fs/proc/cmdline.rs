//! linux-parity: complete
//! linux-source: vendor/linux/fs/proc/cmdline.c
//! test-origin: linux:vendor/linux/fs/proc/cmdline.c
//! `/proc/cmdline`.
//!
//! Ref: `vendor/linux/fs/proc/cmdline.c`

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::sync::Arc;
use spin::Mutex;

use crate::fs::kernfs::KernfsNode;

static SAVED_COMMAND_LINE: Mutex<Option<String>> = Mutex::new(None);

pub fn set_saved_command_line(cmdline: &str) {
    *SAVED_COMMAND_LINE.lock() = Some(cmdline.to_string());
}

pub fn saved_command_line() -> String {
    SAVED_COMMAND_LINE.lock().clone().unwrap_or_default()
}

pub fn cmdline_proc_text(saved: &str) -> String {
    let mut out = String::with_capacity(saved.len() + 1);
    out.push_str(saved);
    out.push('\n');
    out
}

pub fn cmdline_proc_size() -> usize {
    saved_command_line().len() + 1
}

pub fn show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, &cmdline_proc_text(&saved_command_line()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmdline_proc_show_matches_linux_saved_command_line_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/cmdline.c"
        ));
        assert!(source.contains("seq_puts(m, saved_command_line);"));
        assert!(source.contains("seq_putc(m, '\\n');"));
        assert!(source.contains("proc_create_single(\"cmdline\", 0, NULL, cmdline_proc_show)"));
        assert!(source.contains("pde_make_permanent(pde);"));
        assert!(source.contains("pde->size = saved_command_line_len + 1;"));

        let cmdline = "root=/dev/vda rw quiet";
        set_saved_command_line(cmdline);
        assert_eq!(saved_command_line(), cmdline);
        assert_eq!(cmdline_proc_text(cmdline), "root=/dev/vda rw quiet\n");
        assert_eq!(cmdline_proc_size(), cmdline.len() + 1);
    }
}
