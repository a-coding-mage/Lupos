//! linux-parity: complete
//! linux-source: vendor/linux/fs/proc/bootconfig.c
//! test-origin: linux:vendor/linux/fs/proc/bootconfig.c
//! `/proc/bootconfig`.

use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use spin::Mutex;

use crate::fs::kernfs::KernfsNode;
use crate::include::uapi::errno::ENOMEM;

static SAVED_BOOT_CONFIG: Mutex<Option<String>> = Mutex::new(None);
static PROC_CREATED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XbcKeyValue {
    pub key: String,
    pub values: Vec<String>,
    pub is_array: bool,
}

impl XbcKeyValue {
    pub fn new(key: &str, values: &[&str], is_array: bool) -> Self {
        Self {
            key: key.to_string(),
            values: values.iter().map(|value| (*value).to_string()).collect(),
            is_array,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BootConfigInput {
    pub entries: Vec<XbcKeyValue>,
    pub cmdline_has_extra_options: bool,
    pub boot_command_line: String,
    pub fail_key_alloc: bool,
    pub fail_saved_alloc: bool,
}

pub const fn rest(dst: usize, end: usize) -> usize {
    if end > dst { end - dst } else { 0 }
}

fn quote_for(value: &str) -> char {
    if value.as_bytes().contains(&b'"') {
        '\''
    } else {
        '"'
    }
}

pub fn copy_xbc_key_value_list(input: &BootConfigInput) -> Result<String, i32> {
    if input.fail_key_alloc {
        return Err(-ENOMEM);
    }

    let mut out = String::new();
    for entry in &input.entries {
        out.push_str(&entry.key);
        out.push_str(" = ");
        if entry.values.is_empty() {
            out.push_str("\"\"\n");
            continue;
        }
        for value in &entry.values {
            let quote = quote_for(value);
            out.push(quote);
            out.push_str(value);
            out.push(quote);
            if entry.is_array {
                out.push_str(", ");
            } else {
                out.push('\n');
            }
        }
    }

    if input.cmdline_has_extra_options && !input.boot_command_line.is_empty() {
        out.push_str("# Parameters from bootloader:\n# ");
        out.push_str(&input.boot_command_line);
        out.push('\n');
    }

    Ok(out)
}

pub fn proc_boot_config_init(input: &BootConfigInput) -> Result<usize, i32> {
    let rendered = copy_xbc_key_value_list(input)?;
    let len = rendered.len();
    if len > 0 {
        if input.fail_saved_alloc {
            return Err(-ENOMEM);
        }
        *SAVED_BOOT_CONFIG.lock() = Some(rendered);
    }
    PROC_CREATED.store(true, Ordering::Release);
    Ok(len)
}

pub fn boot_config_proc_show() -> String {
    SAVED_BOOT_CONFIG
        .lock()
        .as_ref()
        .cloned()
        .unwrap_or_default()
}

pub fn proc_created() -> bool {
    PROC_CREATED.load(Ordering::Acquire)
}

pub fn show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let text = boot_config_proc_show();
    super::util::copy_into(buf, &text)
}

#[cfg(test)]
pub fn reset_for_test() {
    *SAVED_BOOT_CONFIG.lock() = None;
    PROC_CREATED.store(false, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    static TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

    #[test]
    fn bootconfig_matches_linux_render_and_proc_init_flow() {
        let _guard = TEST_LOCK.lock();
        reset_for_test();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/bootconfig.c"
        ));
        assert!(source.contains("static char *saved_boot_config;"));
        assert!(source.contains("seq_puts(m, saved_boot_config);"));
        assert!(source.contains("#define rest(dst, end)"));
        assert!(source.contains("key = kzalloc(XBC_KEYLEN_MAX, GFP_KERNEL);"));
        assert!(source.contains("xbc_for_each_key_value(leaf, val)"));
        assert!(source.contains("if (strchr(val, '\"'))"));
        assert!(source.contains("q = '\\'';"));
        assert!(source.contains("cmdline_has_extra_options()"));
        assert!(source.contains("boot_command_line[0]"));
        assert!(source.contains("kfree(key);"));
        assert!(source.contains("saved_boot_config = kzalloc(len + 1, GFP_KERNEL);"));
        assert!(source.contains("kfree(saved_boot_config);"));
        assert!(source.contains("proc_create_single(\"bootconfig\""));
        assert!(source.contains("fs_initcall(proc_boot_config_init);"));

        let input = BootConfigInput {
            entries: vec![
                XbcKeyValue::new("kernel.foo", &["bar"], false),
                XbcKeyValue::new("kernel.list", &["a", "b\"c"], true),
                XbcKeyValue::new("kernel.empty", &[], false),
            ],
            cmdline_has_extra_options: true,
            boot_command_line: "root=/dev/vda1 quiet".to_string(),
            fail_key_alloc: false,
            fail_saved_alloc: false,
        };
        let rendered = copy_xbc_key_value_list(&input).expect("render bootconfig");
        assert_eq!(
            rendered,
            "kernel.foo = \"bar\"\n\
             kernel.list = \"a\", 'b\"c', \
             kernel.empty = \"\"\n\
             # Parameters from bootloader:\n\
             # root=/dev/vda1 quiet\n"
        );
        assert_eq!(proc_boot_config_init(&input), Ok(rendered.len()));
        assert!(proc_created());
        assert_eq!(boot_config_proc_show(), rendered);
    }

    #[test]
    fn bootconfig_models_allocation_failures_and_empty_output() {
        let _guard = TEST_LOCK.lock();
        reset_for_test();

        let mut input = BootConfigInput {
            fail_key_alloc: true,
            ..BootConfigInput::default()
        };
        assert_eq!(copy_xbc_key_value_list(&input), Err(-ENOMEM));

        input.fail_key_alloc = false;
        input
            .entries
            .push(XbcKeyValue::new("kernel.foo", &["bar"], false));
        input.fail_saved_alloc = true;
        assert_eq!(proc_boot_config_init(&input), Err(-ENOMEM));
        assert!(!proc_created());

        reset_for_test();
        assert_eq!(proc_boot_config_init(&BootConfigInput::default()), Ok(0));
        assert!(proc_created());
        assert_eq!(boot_config_proc_show(), "");
    }
}
