//! linux-parity: partial
//! linux-source: vendor/linux/lib/dynamic_debug.c
//! test-origin: linux:vendor/linux/lib/test_dynamic_debug.c
//! Dynamic-debug module table ownership and vendor C ABI emitters.

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::ffi::c_char;

use spin::Mutex;

pub const DDEBUG_DESCRIPTOR_SIZE: usize = 56;
pub const DDEBUG_CLASS_MAP_SIZE: usize = 56;
const DDEBUG_FLAGS_OFFSET: usize = 36;
const DPRINTK_FLAGS_PRINT: u8 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DynamicDebugTable {
    pub owner: usize,
    pub module_name: String,
    pub descriptors: usize,
    pub descriptor_count: usize,
    pub classes: usize,
    pub class_count: usize,
}

static TABLES: Mutex<Vec<DynamicDebugTable>> = Mutex::new(Vec::new());

/// `ddebug_add_module()` from the `MODULE_STATE_COMING` notifier.
pub fn module_coming(
    owner: usize,
    module_name: &str,
    descriptors: usize,
    descriptors_len: usize,
    classes: usize,
    classes_len: usize,
) -> Result<(), i32> {
    if descriptors_len % DDEBUG_DESCRIPTOR_SIZE != 0
        || classes_len % DDEBUG_CLASS_MAP_SIZE != 0
        || (descriptors_len != 0 && descriptors % 8 != 0)
        || (classes_len != 0 && classes % 8 != 0)
    {
        return Err(-8); // ENOEXEC
    }

    if descriptors_len == 0 {
        return Ok(());
    }

    let mut tables = TABLES.lock();
    if tables.iter().any(|table| table.owner == owner) {
        return Err(-17); // EEXIST
    }
    tables.push(DynamicDebugTable {
        owner,
        module_name: module_name.to_string(),
        descriptors,
        descriptor_count: descriptors_len / DDEBUG_DESCRIPTOR_SIZE,
        classes,
        class_count: classes_len / DDEBUG_CLASS_MAP_SIZE,
    });
    Ok(())
}

/// `ddebug_remove_module()` from the `MODULE_STATE_GOING` notifier.
pub fn module_going(owner: usize) {
    TABLES.lock().retain(|table| table.owner != owner);
}

pub fn module_table(owner: usize) -> Option<DynamicDebugTable> {
    TABLES
        .lock()
        .iter()
        .find(|table| table.owner == owner)
        .cloned()
}

/// Apply the `+p`/`-p` part of a dynamic-debug query to every descriptor in a
/// module.  The jump-label subsystem owns static-key patching; the descriptor
/// flag remains the authoritative policy bit used by the emitters.
///
/// # Safety
/// The registered descriptor section must remain writable module memory.
pub unsafe fn set_module_print(owner: usize, enabled: bool) -> Result<(), i32> {
    let table = module_table(owner).ok_or(-2)?;
    for index in 0..table.descriptor_count {
        let descriptor = table
            .descriptors
            .checked_add(index.checked_mul(DDEBUG_DESCRIPTOR_SIZE).ok_or(-8)?)
            .ok_or(-8)?;
        let flags = descriptor.checked_add(DDEBUG_FLAGS_OFFSET).ok_or(-8)? as *mut u8;
        let old = unsafe { flags.read_volatile() };
        let new = if enabled {
            old | DPRINTK_FLAGS_PRINT
        } else {
            old & !DPRINTK_FLAGS_PRINT
        };
        unsafe { flags.write_volatile(new) };
    }
    Ok(())
}

unsafe fn descriptor_print_enabled(descriptor: usize) -> bool {
    descriptor != 0
        && unsafe {
            ((descriptor + DDEBUG_FLAGS_OFFSET) as *const u8).read_volatile() & DPRINTK_FLAGS_PRINT
                != 0
        }
}

unsafe extern "C" fn dynamic_debug_emit(
    descriptor: usize,
    fmt: *const c_char,
    register_args: *const usize,
    register_count: usize,
    stack_args: *const usize,
) {
    if fmt.is_null() || !unsafe { descriptor_print_enabled(descriptor) } {
        return;
    }
    let mut message = [0u8; crate::kernel::printk::log::MSG_CAP];
    let length = unsafe {
        crate::linux_driver_abi::base::printf::vscnprintf_n(
            message.as_mut_ptr(),
            message.len(),
            fmt,
            register_args,
            register_count,
            stack_args,
        )
    };
    let text = core::str::from_utf8(&message[..length]).unwrap_or("");
    let text = text.strip_suffix('\n').unwrap_or(text);
    crate::kernel::printk::log::_log(
        crate::kernel::printk::log::Level::Debug,
        "dynamic_debug",
        format_args!("{text}"),
    );
}

/// `__dynamic_pr_debug(struct _ddebug *, const char *, ...)`.
#[unsafe(naked)]
pub unsafe extern "C" fn linux_dynamic_pr_debug() {
    core::arch::naked_asm!(
        "sub rsp, 40",
        "mov qword ptr [rsp], rdx",
        "mov qword ptr [rsp + 8], rcx",
        "mov qword ptr [rsp + 16], r8",
        "mov qword ptr [rsp + 24], r9",
        "mov rdx, rsp",
        "mov rcx, 4",
        "lea r8, [rsp + 48]",
        "call {helper}",
        "add rsp, 40",
        "ret",
        helper = sym dynamic_debug_emit,
    );
}

/// Common shape for `__dynamic_{dev,netdev,ibdev}_dbg`: the device pointer is
/// a fixed second argument and the format string is the third argument.
#[unsafe(naked)]
pub unsafe extern "C" fn linux_dynamic_device_debug() {
    core::arch::naked_asm!(
        "sub rsp, 40",
        "mov qword ptr [rsp], rcx",
        "mov qword ptr [rsp + 8], r8",
        "mov qword ptr [rsp + 16], r9",
        "mov rsi, rdx",
        "mov rdx, rsp",
        "mov rcx, 3",
        "lea r8, [rsp + 48]",
        "call {helper}",
        "add rsp, 40",
        "ret",
        helper = sym dynamic_debug_emit,
    );
}

pub fn register_module_exports() {
    for (name, address) in [
        ("__dynamic_pr_debug", linux_dynamic_pr_debug as usize),
        ("__dynamic_dev_dbg", linux_dynamic_device_debug as usize),
        ("__dynamic_netdev_dbg", linux_dynamic_device_debug as usize),
        ("__dynamic_ibdev_dbg", linux_dynamic_device_debug as usize),
    ] {
        if crate::kernel::module::find_symbol(name).is_none() {
            crate::kernel::module::export_symbol(name, address, false);
        }
    }
}
