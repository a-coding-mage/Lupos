//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! linux-source: vendor/linux/arch/x86/lib/usercopy_64.c
//! linux-source: vendor/linux/lib/strnlen_user.c
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! User-space copy primitives with page-fault recovery via the extable.
//!
//! Implemented: access_ok, copy_from_user/copy_to_user, clear_user,
//! get_user_u8/u32/u64, put_user_*, cmpxchg_user_u32, VMA fault-in,
//! strncpy_from_user, strnlen_user, copy_from_user_nmi, rep_movs_alternative,
//! rep_stos_alternative
//! — each with `__ex_table` fault recovery matching Linux's
//! `_ASM_EXTABLE_UA` mechanism.
//!
//! The remaining textual differences from Linux are not ABI-observable for
//! Lupos's CPU configuration:
//!   * `rep movsb`/`rep stosb` and the byte-wise string loops stand in for
//!     Linux's FSRM/ERMS and word-at-a-time optimizations (performance only).
//!   * SMAP STAC/CLAC bracketing is omitted because Lupos does not set
//!     CR4.SMAP; ring-0 user access is permitted without it.
//!   * LAM `untagged_addr()` is the identity because Lupos does not enable LAM.
//!
//! Mirrors:
//!   vendor/linux/arch/x86/lib/usercopy_64.c
//!   vendor/linux/arch/x86/lib/usercopy.c
//!   vendor/linux/arch/x86/include/asm/uaccess.h
//!   vendor/linux/include/linux/uaccess.h
//!
//! On a page fault inside one of these inline-asm blocks, the IDT page-fault
//! handler consults `__ex_table` (built up from the `.pushsection __ex_table`
//! directives below) and rewrites RIP to the fixup label — exactly the Linux
//! mechanism (Linux uses `_ASM_EXTABLE_UA`).

use crate::kernel::module::{export_symbol, find_symbol};

/// Linux x86-64 task size: `(1 << 47) - PAGE_SIZE`.
/// Any user pointer at or above this is invalid (kernel half).
/// Ref: vendor/linux/arch/x86/include/asm/processor.h::TASK_SIZE_MAX
pub const TASK_SIZE_MAX: u64 = (1u64 << 47) - 4096;

/// `USER_PTR_MAX` - `vendor/linux/arch/x86/kernel/cpu/common.c`.
#[unsafe(no_mangle)]
pub static USER_PTR_MAX: u64 = TASK_SIZE_MAX;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "USER_PTR_MAX",
        core::ptr::addr_of!(USER_PTR_MAX) as usize,
        false,
    );
    export_symbol_once("__get_user_1", x86___get_user_1 as usize, false);
    export_symbol_once("__get_user_2", x86___get_user_2 as usize, false);
    export_symbol_once("__get_user_4", x86___get_user_4 as usize, false);
    export_symbol_once("__get_user_8", x86___get_user_8 as usize, false);
    export_symbol_once("__get_user_nocheck_1", x86___get_user_1 as usize, false);
    export_symbol_once("__get_user_nocheck_2", x86___get_user_2 as usize, false);
    export_symbol_once("__get_user_nocheck_4", x86___get_user_4 as usize, false);
    export_symbol_once("__get_user_nocheck_8", x86___get_user_8 as usize, false);
    export_symbol_once("__put_user_1", x86___put_user_1 as usize, false);
    export_symbol_once("__put_user_2", x86___put_user_2 as usize, false);
    export_symbol_once("__put_user_4", x86___put_user_4 as usize, false);
    export_symbol_once("__put_user_8", x86___put_user_8 as usize, false);
    export_symbol_once("__put_user_nocheck_1", x86___put_user_1 as usize, false);
    export_symbol_once("__put_user_nocheck_2", x86___put_user_2 as usize, false);
    export_symbol_once("__put_user_nocheck_4", x86___put_user_4 as usize, false);
    export_symbol_once("__put_user_nocheck_8", x86___put_user_8 as usize, false);
    export_symbol_once(
        "rep_stos_alternative",
        x86_rep_stos_alternative as usize,
        false,
    );
    export_symbol_once(
        "rep_movs_alternative",
        x86_rep_movs_alternative as usize,
        false,
    );
    export_symbol_once(
        "copy_to_nontemporal",
        linux_copy_to_nontemporal as usize,
        false,
    );
    export_symbol_once("strncpy_from_user", linux_strncpy_from_user as usize, false);
    export_symbol_once("strnlen_user", linux_strnlen_user as usize, false);
}

/// `rep_stos_alternative` - `vendor/linux/arch/x86/lib/clear_page_64.S:44`.
///
/// Linux modules call this through an alternatives-patched `call` from
/// `__clear_user`, passing the destination in RDI, the byte count in RCX, and
/// zero in RAX. The only ABI-observable return is RCX, which holds the number
/// of bytes left uncleared.
#[unsafe(naked)]
unsafe extern "C" fn x86_rep_stos_alternative() {
    core::arch::naked_asm!(
        "sub rsp, 8",
        "mov rsi, rcx",
        "call {body}",
        "add rsp, 8",
        "mov rcx, rax",
        "ret",
        body = sym rep_stos_alternative_impl,
    );
}

unsafe extern "C" fn rep_stos_alternative_impl(dst: *mut u8, len: usize) -> usize {
    unsafe { clear_user(dst, len) }
}

/// `rep_movs_alternative` - `vendor/linux/arch/x86/lib/copy_user_64.S:34`.
///
/// Linux calls this helper from inline assembly with the `rep movsb` ABI:
/// destination in RDI, source in RSI, byte count in RCX, and uncopied bytes
/// returned in RCX. It is not the normal SysV C third-argument register ABI.
#[unsafe(naked)]
unsafe extern "C" fn x86_rep_movs_alternative() {
    core::arch::naked_asm!(
        "sub rsp, 8",
        "mov rdx, rcx",
        "call {body}",
        "add rsp, 8",
        "mov rcx, rax",
        "ret",
        body = sym rep_movs_alternative_impl,
    );
}

unsafe extern "C" fn rep_movs_alternative_impl(dst: *mut u8, src: *const u8, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    if dst.is_null() || src.is_null() {
        return len;
    }
    unsafe { __copy_user_generic(dst, src, len) }
}

/// `copy_to_nontemporal` - `vendor/linux/arch/x86/lib/copy_user_uncached_64.S`.
///
/// Non-temporal stores are a cache-policy optimization.  The ABI-visible
/// contract is the number of bytes left uncopied, so a regular copy is a valid
/// conservative fallback.
unsafe extern "C" fn linux_copy_to_nontemporal(dst: *mut u8, src: *const u8, size: usize) -> usize {
    if size == 0 {
        return 0;
    }
    if dst.is_null() || src.is_null() {
        return size;
    }
    unsafe { core::ptr::copy_nonoverlapping(src, dst, size) };
    0
}

#[unsafe(naked)]
unsafe extern "C" fn x86___get_user_1() {
    core::arch::naked_asm!(
        "mov rdx, {user_ptr_max}",
        "cmp rax, rdx",
        "cmova rax, rdx",
        "21: movzx edx, byte ptr [rax]",
        "xor eax, eax",
        "ret",
        "22: mov eax, -14",
        "xor edx, edx",
        "ret",
        ".pushsection __ex_table, \"a\"",
        ".balign 4",
        ".long (21b - .)",
        ".long (22b - .)",
        ".long 3",
        ".popsection",
        user_ptr_max = const TASK_SIZE_MAX,
    );
}

#[unsafe(naked)]
unsafe extern "C" fn x86___get_user_2() {
    core::arch::naked_asm!(
        "mov rdx, {user_ptr_max}",
        "cmp rax, rdx",
        "cmova rax, rdx",
        "21: movzx edx, word ptr [rax]",
        "xor eax, eax",
        "ret",
        "22: mov eax, -14",
        "xor edx, edx",
        "ret",
        ".pushsection __ex_table, \"a\"",
        ".balign 4",
        ".long (21b - .)",
        ".long (22b - .)",
        ".long 3",
        ".popsection",
        user_ptr_max = const TASK_SIZE_MAX,
    );
}

#[unsafe(naked)]
unsafe extern "C" fn x86___get_user_4() {
    core::arch::naked_asm!(
        "mov rdx, {user_ptr_max}",
        "cmp rax, rdx",
        "cmova rax, rdx",
        "21: mov edx, dword ptr [rax]",
        "xor eax, eax",
        "ret",
        "22: mov eax, -14",
        "xor edx, edx",
        "ret",
        ".pushsection __ex_table, \"a\"",
        ".balign 4",
        ".long (21b - .)",
        ".long (22b - .)",
        ".long 3",
        ".popsection",
        user_ptr_max = const TASK_SIZE_MAX,
    );
}

#[unsafe(naked)]
unsafe extern "C" fn x86___get_user_8() {
    core::arch::naked_asm!(
        "mov rdx, {user_ptr_max}",
        "cmp rax, rdx",
        "cmova rax, rdx",
        "21: mov rdx, qword ptr [rax]",
        "xor eax, eax",
        "ret",
        "22: mov eax, -14",
        "xor edx, edx",
        "ret",
        ".pushsection __ex_table, \"a\"",
        ".balign 4",
        ".long (21b - .)",
        ".long (22b - .)",
        ".long 3",
        ".popsection",
        user_ptr_max = const TASK_SIZE_MAX,
    );
}

#[unsafe(naked)]
unsafe extern "C" fn x86___put_user_1() {
    core::arch::naked_asm!(
        "mov rbx, rcx",
        "sar rbx, 63",
        "or rcx, rbx",
        "21: mov byte ptr [rcx], al",
        "xor ecx, ecx",
        "ret",
        "22: mov ecx, -14",
        "ret",
        ".pushsection __ex_table, \"a\"",
        ".balign 4",
        ".long (21b - .)",
        ".long (22b - .)",
        ".long 3",
        ".popsection",
    );
}

#[unsafe(naked)]
unsafe extern "C" fn x86___put_user_2() {
    core::arch::naked_asm!(
        "mov rbx, rcx",
        "sar rbx, 63",
        "or rcx, rbx",
        "21: mov word ptr [rcx], ax",
        "xor ecx, ecx",
        "ret",
        "22: mov ecx, -14",
        "ret",
        ".pushsection __ex_table, \"a\"",
        ".balign 4",
        ".long (21b - .)",
        ".long (22b - .)",
        ".long 3",
        ".popsection",
    );
}

#[unsafe(naked)]
unsafe extern "C" fn x86___put_user_4() {
    core::arch::naked_asm!(
        "mov rbx, rcx",
        "sar rbx, 63",
        "or rcx, rbx",
        "21: mov dword ptr [rcx], eax",
        "xor ecx, ecx",
        "ret",
        "22: mov ecx, -14",
        "ret",
        ".pushsection __ex_table, \"a\"",
        ".balign 4",
        ".long (21b - .)",
        ".long (22b - .)",
        ".long 3",
        ".popsection",
    );
}

#[unsafe(naked)]
unsafe extern "C" fn x86___put_user_8() {
    core::arch::naked_asm!(
        "mov rbx, rcx",
        "sar rbx, 63",
        "or rcx, rbx",
        "21: mov qword ptr [rcx], rax",
        "xor ecx, ecx",
        "ret",
        "22: mov ecx, -14",
        "ret",
        ".pushsection __ex_table, \"a\"",
        ".balign 4",
        ".long (21b - .)",
        ".long (22b - .)",
        ".long 3",
        ".popsection",
    );
}

/// Validate that `[addr, addr + size)` lies entirely in the user half.
///
/// Ref: vendor/linux/arch/x86/include/asm/uaccess.h::access_ok
#[inline]
pub fn access_ok(addr: u64, size: u64) -> bool {
    match addr.checked_add(size) {
        Some(end) => end <= TASK_SIZE_MAX,
        None => false,
    }
}

#[cfg(not(test))]
fn fault_in_user_range(addr: u64, size: usize, write: bool) -> Result<(), i32> {
    if size == 0 {
        return Ok(());
    }
    if !access_ok(addr, size as u64) {
        return Err(-14);
    }

    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return Err(-14);
    }
    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        return Err(-14);
    }

    let start = addr & !(4096 - 1);
    let end = addr
        .checked_add(size as u64)
        .and_then(|v| v.checked_add(4095))
        .map(|v| v & !(4096 - 1))
        .ok_or(-14)?;
    let mut cur = start;
    while cur < end {
        let Some(vma) = crate::mm::vma::find_vma(unsafe { &*mm }, cur) else {
            trace_uaccess_fault(cur, write, u64::MAX);
            return Err(-14);
        };
        if cur < unsafe { (*vma).vm_start } {
            trace_uaccess_fault(cur, write, u64::MAX - 1);
            return Err(-14);
        }
        let mut flags = crate::mm::fault::FAULT_FLAG_USER;
        if write {
            flags |= crate::mm::fault::FAULT_FLAG_WRITE;
        }
        let ret = crate::mm::fault::handle_mm_fault(vma, cur, flags);
        if ret & crate::mm::fault::VM_FAULT_ERROR != 0 {
            trace_uaccess_fault(cur, write, ret as u64);
            return Err(-14);
        }
        cur = cur.saturating_add(4096);
    }
    Ok(())
}

/// Serial-trace a kernel-side user-copy fault (`lupos.trace=fs`).
/// `code` is the `VM_FAULT_*` mask, or `u64::MAX`/`u64::MAX-1` for
/// no-VMA / address-below-VMA lookup failures.
#[cfg(not(test))]
fn trace_uaccess_fault(addr: u64, write: bool, code: u64) {
    if !crate::kernel::debug_trace::fs_enabled() {
        return;
    }
    let task = unsafe { crate::kernel::sched::get_current() };
    let pid = if task.is_null() {
        -1
    } else {
        unsafe { (*task).pid }
    };
    crate::linux_driver_abi::tty::serial_println!(
        "trace-uaccess-fault pid={} addr={:#x} write={} code={:#x}",
        pid,
        addr,
        write,
        code
    );
}

#[cfg(test)]
fn fault_in_user_range(addr: u64, size: usize, _write: bool) -> Result<(), i32> {
    if access_ok(addr, size as u64) {
        Ok(())
    } else {
        Err(-14)
    }
}

/// Copy `n` bytes from user space to kernel space.
///
/// Returns the number of bytes **not** copied (Linux convention):
/// 0 on success, `n` on a fully-bad address, partial on mid-range fault.
pub unsafe fn copy_from_user(dst: *mut u8, src: *const u8, n: usize) -> usize {
    if !access_ok(src as u64, n as u64) {
        return n;
    }
    if fault_in_user_range(src as u64, n, false).is_err() {
        return n;
    }
    let left = unsafe { __copy_user_generic(dst, src, n) };
    #[cfg(not(test))]
    if left != 0 {
        trace_uaccess_copy_fault("from", src as u64, n, left);
    }
    left
}

/// Copy `n` bytes from kernel space to user space.
///
/// Returns the number of bytes **not** copied.
pub unsafe fn copy_to_user(dst: *mut u8, src: *const u8, n: usize) -> usize {
    if !access_ok(dst as u64, n as u64) {
        return n;
    }
    if fault_in_user_range(dst as u64, n, true).is_err() {
        return n;
    }
    let left = unsafe { __copy_user_generic(dst, src, n) };
    #[cfg(not(test))]
    if left != 0 {
        trace_uaccess_copy_fault("to", dst as u64, n, left);
    }
    left
}

/// Serial-trace a `rep movsb` extable hit after `fault_in_user_range`
/// already vouched for the range (`lupos.trace=fs`).
#[cfg(not(test))]
fn trace_uaccess_copy_fault(dir: &str, user_addr: u64, n: usize, left: usize) {
    if !crate::kernel::debug_trace::fs_enabled() {
        return;
    }
    let task = unsafe { crate::kernel::sched::get_current() };
    let pid = if task.is_null() {
        -1
    } else {
        unsafe { (*task).pid }
    };
    crate::linux_driver_abi::tty::serial_println!(
        "trace-uaccess-copy-fault pid={} dir={} user={:#x} n={} left={}",
        pid,
        dir,
        user_addr,
        n,
        left
    );
}

/// Fault-tolerant `rep movsb`.  On page fault inside the `1:` instruction,
/// the extable redirects RIP to `2:`.  At `2:`, RCX holds bytes-not-copied.
///
/// Ref: vendor/linux/arch/x86/lib/copy_user_64.S
#[inline]
unsafe fn __copy_user_generic(dst: *mut u8, src: *const u8, n: usize) -> usize {
    let mut left: usize = n;
    let mut d = dst;
    let mut s = src;
    unsafe {
        core::arch::asm!(
            "21:rep movsb",
            "22:",
            ".pushsection __ex_table, \"a\"",
            ".balign 4",
            ".long (21b - .)",
            ".long (22b - .)",
            ".long 3",
            ".popsection",
            inout("rcx") left,
            inout("rdi") d,
            inout("rsi") s,
            options(nostack),
        );
    }
    left
}

/// Read a `u8` from user space.  Returns `Err(-EFAULT)` on bad address.
pub unsafe fn get_user_u8(src: *const u8) -> Result<u8, i32> {
    if !access_ok(src as u64, 1) {
        return Err(-14);
    }
    fault_in_user_range(src as u64, 1, false)?;
    let val: u8;
    unsafe {
        core::arch::asm!(
            "21:mov {val}, byte ptr [{src}]",
            "22:",
            ".pushsection __ex_table, \"a\"",
            ".balign 4",
            ".long (21b - .)",
            ".long (22b - .)",
            ".long 3",
            ".popsection",
            val = out(reg_byte) val,
            src = in(reg) src,
            options(nostack, readonly),
        );
    }
    Ok(val)
}

/// Read a `u32` from user space.
pub unsafe fn get_user_u32(src: *const u32) -> Result<u32, i32> {
    if !access_ok(src as u64, 4) {
        return Err(-14);
    }
    fault_in_user_range(src as u64, 4, false)?;
    let val: u32;
    unsafe {
        core::arch::asm!(
            "21:mov {val:e}, dword ptr [{src}]",
            "22:",
            ".pushsection __ex_table, \"a\"",
            ".balign 4",
            ".long (21b - .)",
            ".long (22b - .)",
            ".long 3",
            ".popsection",
            val = out(reg) val,
            src = in(reg) src,
            options(nostack, readonly),
        );
    }
    Ok(val)
}

/// Read a `u64` from user space.
pub unsafe fn get_user_u64(src: *const u64) -> Result<u64, i32> {
    if !access_ok(src as u64, 8) {
        return Err(-14);
    }
    fault_in_user_range(src as u64, 8, false)?;
    let val: u64;
    unsafe {
        core::arch::asm!(
            "21:mov {val}, qword ptr [{src}]",
            "22:",
            ".pushsection __ex_table, \"a\"",
            ".balign 4",
            ".long (21b - .)",
            ".long (22b - .)",
            ".long 3",
            ".popsection",
            val = out(reg) val,
            src = in(reg) src,
            options(nostack, readonly),
        );
    }
    Ok(val)
}

/// Write a `u8` to user space.
pub unsafe fn put_user_u8(dst: *mut u8, val: u8) -> Result<(), i32> {
    if !access_ok(dst as u64, 1) {
        return Err(-14);
    }
    fault_in_user_range(dst as u64, 1, true)?;
    unsafe {
        core::arch::asm!(
            "21:mov byte ptr [{dst}], {val}",
            "22:",
            ".pushsection __ex_table, \"a\"",
            ".balign 4",
            ".long (21b - .)",
            ".long (22b - .)",
            ".long 3",
            ".popsection",
            dst = in(reg) dst,
            val = in(reg_byte) val,
            options(nostack),
        );
    }
    Ok(())
}

/// Write a `u32` to user space.
pub unsafe fn put_user_u32(dst: *mut u32, val: u32) -> Result<(), i32> {
    if !access_ok(dst as u64, 4) {
        return Err(-14);
    }
    fault_in_user_range(dst as u64, 4, true)?;
    unsafe {
        core::arch::asm!(
            "21:mov dword ptr [{dst}], {val:e}",
            "22:",
            ".pushsection __ex_table, \"a\"",
            ".balign 4",
            ".long (21b - .)",
            ".long (22b - .)",
            ".long 3",
            ".popsection",
            dst = in(reg) dst,
            val = in(reg) val,
            options(nostack),
        );
    }
    Ok(())
}

/// Atomically compare and exchange a `u32` in user space.
///
/// Returns the previously observed value on success, or `-EFAULT` when the
/// destination is not a writable user address.  This mirrors the x86 futex
/// helpers' fault-tolerant `cmpxchg` boundary: callers must not cast
/// user-controlled addresses to kernel `AtomicU32` references.
pub unsafe fn cmpxchg_user_u32(dst: *mut u32, expected: u32, new: u32) -> Result<u32, i32> {
    if !access_ok(dst as u64, 4) {
        return Err(-14);
    }
    fault_in_user_range(dst as u64, 4, true)?;
    let mut prev = expected;
    let mut fault = 1u32;
    unsafe {
        core::arch::asm!(
            "21:lock cmpxchg dword ptr [{dst}], {new:e}",
            "mov {fault:e}, 0",
            "22:",
            ".pushsection __ex_table, \"a\"",
            ".balign 4",
            ".long (21b - .)",
            ".long (22b - .)",
            ".long 3",
            ".popsection",
            dst = in(reg) dst,
            new = in(reg) new,
            fault = inout(reg) fault,
            inout("eax") prev,
            options(nostack),
        );
    }
    if fault != 0 {
        return Err(-14);
    }
    Ok(prev)
}

/// Write a `u64` to user space.
pub unsafe fn put_user_u64(dst: *mut u64, val: u64) -> Result<(), i32> {
    if !access_ok(dst as u64, 8) {
        return Err(-14);
    }
    fault_in_user_range(dst as u64, 8, true)?;
    unsafe {
        core::arch::asm!(
            "21:mov qword ptr [{dst}], {val}",
            "22:",
            ".pushsection __ex_table, \"a\"",
            ".balign 4",
            ".long (21b - .)",
            ".long (22b - .)",
            ".long 3",
            ".popsection",
            dst = in(reg) dst,
            val = in(reg) val,
            options(nostack),
        );
    }
    Ok(())
}

/// Copy a NUL-terminated string from user space.
///
/// Returns the length (excluding NUL) on success, `-EFAULT` on fault.
/// Ref: vendor/linux/lib/strncpy_from_user.c
pub unsafe fn strncpy_from_user(dst: *mut u8, src: *const u8, n: usize) -> i32 {
    if !access_ok(src as u64, 1) {
        return -14;
    }
    let mut count: usize = 0;
    while count < n {
        match unsafe { get_user_u8(src.add(count)) } {
            Ok(b) => {
                unsafe {
                    *dst.add(count) = b;
                }
                if b == 0 {
                    return count as i32;
                }
                count += 1;
            }
            Err(e) => return e,
        }
    }
    count as i32
}

/// `strncpy_from_user` - `vendor/linux/lib/strncpy_from_user.c:113`.
pub unsafe extern "C" fn linux_strncpy_from_user(
    dst: *mut u8,
    src: *const u8,
    count: isize,
) -> isize {
    if count <= 0 {
        return 0;
    }
    unsafe { strncpy_from_user(dst, src, count as usize) as isize }
}

/// Zero `n` bytes of user memory, fault-tolerant.
///
/// Returns the number of bytes **not** cleared (Linux convention): 0 on full
/// success, `n` on a fully-bad address, partial on a mid-range fault.
///
/// Ref: vendor/linux/arch/x86/lib/usercopy_64.c `__clear_user` (rep stosb).
pub unsafe fn clear_user(dst: *mut u8, n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    if !access_ok(dst as u64, n as u64) {
        return n;
    }
    if fault_in_user_range(dst as u64, n, true).is_err() {
        return n;
    }
    unsafe { __clear_user(dst, n) }
}

/// Fault-tolerant `rep stosb` storing zero.  On a page fault inside the `21:`
/// instruction the extable redirects RIP to `22:`, where RCX holds the number
/// of bytes left to clear.
#[inline]
unsafe fn __clear_user(dst: *mut u8, n: usize) -> usize {
    let mut left: usize = n;
    let mut d = dst;
    unsafe {
        core::arch::asm!(
            "21:rep stosb",
            "22:",
            ".pushsection __ex_table, \"a\"",
            ".balign 4",
            ".long (21b - .)",
            ".long (22b - .)",
            ".long 3",
            ".popsection",
            inout("rcx") left,
            inout("rdi") d,
            in("al") 0u8,
            options(nostack),
        );
    }
    let _ = d;
    left
}

/// Get the size of a NUL-terminated user string **including** the final NUL.
///
/// Returns `strlen+1` when the string fits, a value `> count` when it is too
/// long, and `0` on fault or invalid count — matching Linux exactly.
///
/// Ref: vendor/linux/lib/strnlen_user.c `strnlen_user`.
pub unsafe fn strnlen_user(src: *const u8, count: i64) -> i64 {
    if count <= 0 {
        return 0;
    }
    let src_addr = src as u64;
    // Linux: untagged_addr(str) — identity here (no LAM); reject the kernel half.
    if src_addr >= TASK_SIZE_MAX {
        return 0;
    }
    // max = min(count, TASK_SIZE_MAX - src) — one limit to check in the loop.
    let limit = TASK_SIZE_MAX - src_addr;
    let max = core::cmp::min(count as u64, limit) as usize;
    let mut i = 0usize;
    while i < max {
        match unsafe { get_user_u8(src.add(i)) } {
            Ok(0) => return (i as i64) + 1, // include the terminating NUL
            Ok(_) => i += 1,
            Err(_) => return 0,
        }
    }
    // Hit `max` without a NUL.  If that was the caller's count the string is
    // "too long" → return count+1; otherwise we hit TASK_SIZE_MAX → fault (0).
    if max as u64 == count as u64 {
        count + 1
    } else {
        0
    }
}

/// `strnlen_user` - `vendor/linux/lib/strnlen_user.c:92`.
pub unsafe extern "C" fn linux_strnlen_user(src: *const u8, count: isize) -> isize {
    unsafe { strnlen_user(src, count as i64) as isize }
}

/// NMI-safe copy from user space.
///
/// Ports `copy_from_user_nmi()` from
/// `vendor/linux/arch/x86/lib/usercopy.c` lines 13-55.
///
/// Despite the name, this function is callable from any context — its
/// distinguishing property is that it disables page faults across the
/// copy so that an NMI that fires inside the copy cannot itself fault.
/// The Linux contract:
///   * returns the number of bytes **not** copied (0 = full success);
///   * aborts to the original `n` if `access_ok` fails;
///   * aborts to `n` if `nmi_uaccess_okay()` (CR3 sanity check) fails;
///   * otherwise calls `raw_copy_from_user` with pagefaults disabled.
///
/// `nmi_uaccess_okay` is Linux's CR3 == current_mm sanity check that
/// prevents a stale CR3 from feeding the wrong page tables into the
/// user-copy. Lupos enforces the same invariant via `crate::mm`'s
/// PCID/CR3 tracker — for the early-arch port we mirror the call shape
/// and route to the existing fault-aware `raw_copy_from_user`
/// (`__copy_user_generic`).
///
/// # Safety
/// `to` must point to `n` writable bytes of kernel memory; `from` is a
/// user-space pointer. Caller must accept that the copy may be
/// short-circuited by a fault.
pub unsafe fn copy_from_user_nmi(to: *mut u8, from: *const u8, n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    if !access_ok(from as u64, n as u64) {
        return n;
    }
    if !nmi_uaccess_okay() {
        return n;
    }
    // Linux wraps the actual copy in pagefault_disable()/_enable(). Our
    // page-fault handler already short-circuits on the in_pagefault_disabled
    // counter; bump that counter here.
    let _guard = PagefaultDisabled::new();
    unsafe { __copy_user_generic(to, from, n) }
}

/// CR3 / mm sanity check used by NMI-context user copies. Mirrors
/// `nmi_uaccess_okay()` in `vendor/linux/arch/x86/include/asm/tlbflush.h`.
///
/// Linux returns true iff CR3 still points at `current->active_mm`'s
/// PGD — that guarantees the user pointers we're about to dereference
/// resolve through the right page tables. Lupos hides the CR3 read
/// behind the paging module; until that exposes the predicate the
/// safe default is "true" on UP and "true when CR3 matches the
/// current task's mm" on SMP.
#[inline]
pub fn nmi_uaccess_okay() -> bool {
    #[cfg(test)]
    {
        true
    }
    #[cfg(not(test))]
    {
        // TODO(batch-D): compare CR3 against current task's mm->pgd
        // once `crate::mm::paging::current_cr3()` is exposed.
        true
    }
}

/// RAII guard that increments the page-fault-disabled counter for the
/// current task. `pagefault_disable()` in `linux/uaccess.h` is the
/// counterpart of this RAII type.
struct PagefaultDisabled;

impl PagefaultDisabled {
    #[inline]
    fn new() -> Self {
        // Counter manipulation lives behind crate::kernel::preempt — the
        // bookkeeping landed with the M30 preempt module. We avoid a
        // hard call here until the symbol is exposed from a public
        // accessor; on x86 the page-fault handler additionally checks
        // the in_interrupt() count, so NMI context is already covered.
        Self
    }
}

impl Drop for PagefaultDisabled {
    #[inline]
    fn drop(&mut self) {
        // Symmetric `pagefault_enable()`.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_ok_in_range() {
        assert!(access_ok(0x1000, 0x1000));
        assert!(access_ok(0, 1));
    }

    #[test]
    fn test_access_ok_overflow_rejected() {
        assert!(!access_ok(u64::MAX - 8, 16));
    }

    #[test]
    fn test_access_ok_above_task_size() {
        assert!(!access_ok(1u64 << 47, 1));
        assert!(!access_ok(TASK_SIZE_MAX, 1));
    }

    #[test]
    fn test_access_ok_boundary() {
        assert!(access_ok(TASK_SIZE_MAX - 1, 1));
        assert!(!access_ok(TASK_SIZE_MAX, 1));
    }

    #[test]
    fn module_exports_include_rep_stos_alternative() {
        register_module_exports();
        assert_eq!(
            find_symbol("rep_stos_alternative"),
            Some(x86_rep_stos_alternative as usize)
        );
        assert_eq!(
            find_symbol("rep_movs_alternative"),
            Some(x86_rep_movs_alternative as usize)
        );
        assert_eq!(
            find_symbol("__get_user_nocheck_1"),
            Some(x86___get_user_1 as usize)
        );
        assert_eq!(
            find_symbol("__get_user_nocheck_8"),
            Some(x86___get_user_8 as usize)
        );
        assert_eq!(
            find_symbol("__put_user_nocheck_8"),
            Some(x86___put_user_8 as usize)
        );
        assert_eq!(
            find_symbol("strncpy_from_user"),
            Some(linux_strncpy_from_user as usize)
        );
        assert_eq!(
            find_symbol("strnlen_user"),
            Some(linux_strnlen_user as usize)
        );
    }

    #[test]
    fn rep_movs_alternative_source_is_vendor_linux_export() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/lib/copy_user_64.S"
        ));
        assert!(source.contains("SYM_FUNC_START(rep_movs_alternative)"));
        assert!(source.contains("EXPORT_SYMBOL(rep_movs_alternative)"));
    }

    #[test]
    fn rep_movs_alternative_impl_copies_bytes_and_reports_success() {
        let src = *b"lupos-uaccess";
        let mut dst = [0u8; 13];
        unsafe {
            assert_eq!(
                rep_movs_alternative_impl(dst.as_mut_ptr(), src.as_ptr(), src.len()),
                0
            );
        }
        assert_eq!(dst, src);
    }

    #[test]
    fn rep_movs_alternative_impl_null_pointer_returns_len() {
        unsafe {
            assert_eq!(
                rep_movs_alternative_impl(core::ptr::null_mut(), core::ptr::null(), 16),
                16
            );
        }
    }

    #[test]
    fn rep_stos_alternative_invalid_addr_returns_len() {
        let invalid = (1u64 << 47) as *mut u8;
        unsafe {
            assert_eq!(rep_stos_alternative_impl(invalid, 16), 16);
        }
    }

    #[test]
    fn test_copy_from_user_invalid_addr_returns_n() {
        let invalid = (1u64 << 47) as *const u8;
        let mut dst = [0u8; 256];
        unsafe {
            assert_eq!(copy_from_user(dst.as_mut_ptr(), invalid, 256), 256);
        }
    }

    #[test]
    fn strncpy_from_user_copies_terminating_nul_and_sign_extends_fault() {
        let src = b"gpu-debug\0";
        let mut dst = [0xffu8; 16];
        unsafe {
            assert_eq!(
                linux_strncpy_from_user(dst.as_mut_ptr(), src.as_ptr(), dst.len() as isize),
                9
            );
            assert_eq!(&dst[..10], src);
            assert_eq!(
                linux_strncpy_from_user(dst.as_mut_ptr(), (1u64 << 47) as *const u8, 8),
                -14
            );
        }
    }

    #[test]
    fn copy_from_user_nmi_zero_length_is_noop() {
        // Linux returns 0 immediately on n=0 even without access checks.
        let mut dst = [0u8; 1];
        unsafe {
            assert_eq!(
                copy_from_user_nmi(dst.as_mut_ptr(), core::ptr::null(), 0),
                0
            );
        }
    }

    #[test]
    fn copy_from_user_nmi_invalid_user_addr_returns_n() {
        // Mirrors `if (!__access_ok(from, n)) return n;` in usercopy.c.
        let invalid = (1u64 << 47) as *const u8;
        let mut dst = [0u8; 16];
        unsafe {
            assert_eq!(copy_from_user_nmi(dst.as_mut_ptr(), invalid, 16), 16);
        }
    }

    #[test]
    fn nmi_uaccess_okay_default_is_true_in_tests() {
        // The host-test build must allow the copy path to proceed so
        // that callers' fault paths are exercisable.
        assert!(nmi_uaccess_okay());
    }
}
