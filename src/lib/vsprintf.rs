//! linux-parity: partial
//! linux-source: vendor/linux/lib/vsprintf.c
//! test-origin: linux:vendor/linux/lib/vsprintf.c
//! Minimal exported printf formatting used by Linux-built modules.

use core::ffi::{c_char, c_void};

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("snprintf", linux_snprintf as usize, false);
    export_symbol_once("sprintf", linux_sprintf as usize, false);
    export_symbol_once("vsnprintf", linux_vsnprintf as usize, false);
    export_symbol_once("vsprintf", linux_vsprintf as usize, false);
}

/// `snprintf` - `vendor/linux/lib/vsprintf.c:3036`.
///
/// A true C-variadic function. The assembly wrapper spills the remaining
/// register arguments and hands the native x86-64 SysV varargs to the shared
/// vendor-module formatter, exactly as `sysfs_emit_at` does.
///
/// This previously accepted a single `arg0` and substituted it for *every*
/// conversion specifier, so any module calling `snprintf(buf, n, "%s %s %s",
/// a, b, c)` got `a a a`. That silently renamed every multi-argument kernel
/// string; in particular `snd_hda_gen` built its mixer control as
/// `"Master Master Master"` instead of `"Master Playback Volume"`, which made
/// the card unreachable by name from `amixer`/ALSA and left it muted at 0 dB.
#[unsafe(naked)]
pub unsafe extern "C" fn linux_snprintf() {
    core::arch::naked_asm!(
        // buf/size/fmt arrive in rdi/rsi/rdx; the varargs begin at rcx.
        "sub rsp, 40",
        "mov qword ptr [rsp], rcx",
        "mov qword ptr [rsp + 8], r8",
        "mov qword ptr [rsp + 16], r9",
        "lea rcx, [rsp]",
        // 40 bytes of spill + the 8-byte return address.
        "lea r8, [rsp + 48]",
        "call {helper}",
        "add rsp, 40",
        "ret",
        helper = sym linux_snprintf_helper,
    );
}

#[inline(never)]
unsafe extern "C" fn linux_snprintf_helper(
    buf: *mut c_char,
    size: usize,
    fmt: *const c_char,
    register_args: *const usize,
    stack_args: *const usize,
) -> i32 {
    // Linux `snprintf` returns what *would* have been written (C semantics),
    // unlike `scnprintf`. Three integer registers remain for varargs after
    // buf/size/fmt consume rdi/rsi/rdx.
    unsafe {
        crate::linux_driver_abi::base::printf::vsnprintf_n(
            buf.cast::<u8>(),
            size,
            fmt,
            register_args,
            3,
            stack_args,
        )
        .min(i32::MAX as usize) as i32
    }
}

/// `sprintf` - `vendor/linux/lib/vsprintf.c:3105`.
///
/// Same variadic correction as `snprintf` above.
#[unsafe(naked)]
pub unsafe extern "C" fn linux_sprintf() {
    core::arch::naked_asm!(
        // buf/fmt arrive in rdi/rsi; the varargs begin at rdx.
        "sub rsp, 40",
        "mov qword ptr [rsp], rdx",
        "mov qword ptr [rsp + 8], rcx",
        "mov qword ptr [rsp + 16], r8",
        "mov qword ptr [rsp + 24], r9",
        "lea rdx, [rsp]",
        "lea rcx, [rsp + 48]",
        "call {helper}",
        "add rsp, 40",
        "ret",
        helper = sym linux_sprintf_helper,
    );
}

#[inline(never)]
unsafe extern "C" fn linux_sprintf_helper(
    buf: *mut c_char,
    fmt: *const c_char,
    register_args: *const usize,
    stack_args: *const usize,
) -> i32 {
    unsafe {
        crate::linux_driver_abi::base::printf::vsnprintf_n(
            buf.cast::<u8>(),
            i32::MAX as usize,
            fmt,
            register_args,
            4,
            stack_args,
        )
        .min(i32::MAX as usize) as i32
    }
}

/// `vsprintf` - `vendor/linux/lib/vsprintf.c:3088`.
#[unsafe(export_name = "vsprintf")]
pub unsafe extern "C" fn linux_vsprintf(
    buf: *mut c_char,
    fmt: *const c_char,
    args: *const c_void,
) -> i32 {
    unsafe {
        crate::linux_driver_abi::base::printf::vscnprintf_va_list(
            buf.cast::<u8>(),
            i32::MAX as usize,
            fmt,
            args,
        )
        .min(i32::MAX as usize) as i32
    }
}

/// `vsnprintf` - `vendor/linux/lib/vsprintf.c:2860`.
#[unsafe(export_name = "vsnprintf")]
pub unsafe extern "C" fn linux_vsnprintf(
    buf: *mut c_char,
    size: usize,
    fmt: *const c_char,
    args: *const c_void,
) -> i32 {
    if size > i32::MAX as usize {
        return 0;
    }
    unsafe {
        crate::linux_driver_abi::base::printf::vsnprintf_va_list(buf.cast::<u8>(), size, fmt, args)
            .min(i32::MAX as usize) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snprintf_export_registers_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("snprintf"),
            Some(linux_snprintf as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("sprintf"),
            Some(linux_sprintf as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("vsnprintf"),
            Some(linux_vsnprintf as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("vsprintf"),
            Some(linux_vsprintf as usize)
        );
    }

    #[test]
    fn vsnprintf_export_matches_linux_source_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/vsprintf.c"
        ));

        assert!(
            source.contains(
                "int vsnprintf(char *buf, size_t size, const char *fmt_str, va_list args)"
            )
        );
        assert!(source.contains("EXPORT_SYMBOL(vsnprintf);"));
    }

    /// The exported symbols are C-variadic, so tests must call them through a
    /// variadic function pointer rather than a fixed-arity Rust signature.
    type SnprintfFn = unsafe extern "C" fn(*mut c_char, usize, *const c_char, ...) -> i32;
    type SprintfFn = unsafe extern "C" fn(*mut c_char, *const c_char, ...) -> i32;

    fn snprintf_symbol() -> SnprintfFn {
        unsafe { core::mem::transmute::<usize, SnprintfFn>(linux_snprintf as usize) }
    }

    fn sprintf_symbol() -> SprintfFn {
        unsafe { core::mem::transmute::<usize, SprintfFn>(linux_sprintf as usize) }
    }

    fn as_str(buf: &[c_char], len: usize) -> &str {
        unsafe {
            core::str::from_utf8(core::slice::from_raw_parts(buf.as_ptr().cast::<u8>(), len))
                .expect("utf-8")
        }
    }

    #[test]
    fn snprintf_formats_queue_name_integer() {
        unsafe {
            let mut buf = [0i8; 16];
            let fmt = b"req.%u\0";
            assert_eq!(
                snprintf_symbol()(buf.as_mut_ptr(), buf.len(), fmt.as_ptr().cast(), 7),
                5
            );
            assert_eq!(as_str(&buf, 5), "req.7");
        }
    }

    #[test]
    fn sprintf_formats_virtio_admin_queue_name() {
        unsafe {
            let mut buf = [0i8; 16];
            let fmt = b"avq.%u\0";
            assert_eq!(
                sprintf_symbol()(buf.as_mut_ptr(), fmt.as_ptr().cast(), 12),
                6
            );
            assert_eq!(as_str(&buf, 6), "avq.12");
        }
    }

    /// Every conversion specifier must consume its **own** argument.
    ///
    /// The old exported `snprintf` took a single `arg0` and reused it for all
    /// of them, so `snd_hda_gen`'s `"%s %s %s"` control name came out as
    /// `"Master Master Master"` instead of `"Master Playback Volume"`. ALSA
    /// then had no control named `Master`, so nothing could unmute the codec
    /// and the card emitted digital silence.
    ///
    /// test-origin: linux:vendor/linux/lib/vsprintf.c:snprintf
    #[test]
    fn snprintf_advances_through_every_vararg() {
        unsafe {
            let mut buf = [0i8; 64];
            let fmt = b"%s %s %s\0";
            let master = b"Master\0";
            let playback = b"Playback\0";
            let volume = b"Volume\0";
            let written = snprintf_symbol()(
                buf.as_mut_ptr(),
                buf.len(),
                fmt.as_ptr().cast(),
                master.as_ptr(),
                playback.as_ptr(),
                volume.as_ptr(),
            );
            assert_eq!(written, 22);
            assert_eq!(as_str(&buf, written as usize), "Master Playback Volume");
        }
    }

    /// Mixed specifiers, and enough arguments to spill past the six SysV
    /// integer registers onto the stack.
    #[test]
    fn snprintf_mixes_specifiers_and_spills_to_the_stack() {
        unsafe {
            let mut buf = [0i8; 96];
            let fmt = b"%s%u %s%u %s%u\0";
            let a = b"a\0";
            let b = b"b\0";
            let c = b"c\0";
            let written = snprintf_symbol()(
                buf.as_mut_ptr(),
                buf.len(),
                fmt.as_ptr().cast(),
                a.as_ptr(),
                1u32,
                b.as_ptr(),
                22u32,
                c.as_ptr(),
                333u32,
            );
            assert_eq!(as_str(&buf, written as usize), "a1 b22 c333");
        }
    }

    /// Linux `snprintf` returns the length it *would* have produced, and never
    /// writes past `size`.
    #[test]
    fn snprintf_truncates_but_reports_full_length() {
        unsafe {
            let mut buf = [0i8; 8];
            let fmt = b"%s %s\0";
            let long = b"Playback\0";
            let written = snprintf_symbol()(
                buf.as_mut_ptr(),
                buf.len(),
                fmt.as_ptr().cast(),
                long.as_ptr(),
                long.as_ptr(),
            );
            assert_eq!(written, 17, "must report the untruncated length");
            assert_eq!(buf[buf.len() - 1], 0, "must stay NUL-terminated");
            assert_eq!(as_str(&buf, 7), "Playbac");
        }
    }

    #[test]
    fn sprintf_advances_through_every_vararg() {
        unsafe {
            let mut buf = [0i8; 64];
            let fmt = b"%s-%s-%u\0";
            let left = b"front\0";
            let right = b"left\0";
            let written = sprintf_symbol()(
                buf.as_mut_ptr(),
                fmt.as_ptr().cast(),
                left.as_ptr(),
                right.as_ptr(),
                9u32,
            );
            assert_eq!(as_str(&buf, written as usize), "front-left-9");
        }
    }
}
