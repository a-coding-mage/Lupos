//! linux-parity: partial
//! linux-source: vendor/linux/drivers/video/cmdline.c
//! Linux `video=` command-line option storage and module ABI.
//!
//! Linux's early-parameter parser NUL-terminates each option in the mutable
//! command-line buffer. Lupos keeps the bootloader command line immutable, so
//! this port owns equivalent `CString` copies while preserving lookup order,
//! the `FB_MAX` limit, global fallback, and `ofonly` behavior.

extern crate alloc;

use alloc::ffi::CString;
use alloc::vec::Vec;
use core::ffi::{CStr, c_char};

use lazy_static::lazy_static;
use spin::Mutex;

/// `FB_MAX` from `vendor/linux/include/uapi/linux/fb.h`.
const FB_MAX: usize = 32;

#[derive(Default)]
struct VideoOptionState {
    named: Vec<CString>,
    global: Option<CString>,
    of_only: bool,
}

lazy_static! {
    static ref VIDEO_OPTIONS: Mutex<VideoOptionState> = Mutex::new(VideoOptionState::default());
}

/// Run Linux's `__setup("video=", video_setup)` logic over the immutable
/// bootloader command line. This is an init-only operation; returned C pointers
/// remain valid because the state is not changed after module loading starts.
pub fn configure_from_cmdline(cmdline: &str) {
    let mut state = VIDEO_OPTIONS.lock();
    state.named.clear();
    state.global = None;
    state.of_only = false;

    for token in cmdline.split_ascii_whitespace() {
        let Some(options) = token.strip_prefix("video=") else {
            continue;
        };
        if options.is_empty() {
            continue;
        }
        if options.starts_with("ofonly") {
            state.of_only = true;
            continue;
        }

        let Ok(options) = CString::new(options) else {
            continue;
        };
        if options.as_bytes().contains(&b':') {
            if state.named.len() < FB_MAX {
                state.named.push(options);
            }
        } else {
            state.global = Some(options);
        }
    }
}

fn option_for_name(state: &VideoOptionState, name: Option<&CStr>) -> *const c_char {
    let mut options = core::ptr::null();
    if let Some(name) = name {
        let name = name.to_bytes();
        if !name.is_empty() {
            for entry in &state.named {
                let bytes = entry.as_bytes();
                if bytes.len() > name.len() && bytes.starts_with(name) && bytes[name.len()] == b':'
                {
                    options = unsafe { entry.as_ptr().add(name.len() + 1) };
                }
            }
        }
    }
    if options.is_null() {
        options = state
            .global
            .as_ref()
            .map_or(core::ptr::null(), |entry| entry.as_ptr());
    }
    options
}

/// `video_get_options()` from `drivers/video/cmdline.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn video_get_options(name: *const c_char) -> *const c_char {
    let name = if name.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(name) })
    };
    option_for_name(&VIDEO_OPTIONS.lock(), name)
}

/// `__video_get_options()` from `drivers/video/cmdline.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __video_get_options(
    name: *const c_char,
    options: *mut *const c_char,
    is_of: bool,
) -> bool {
    let name = if name.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(name) })
    };
    let state = VIDEO_OPTIONS.lock();
    if !options.is_null() {
        unsafe {
            *options = option_for_name(&state, name);
        }
    }
    !state.of_only || is_of
}
