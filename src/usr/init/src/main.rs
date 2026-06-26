#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

//! test-origin: lupos-specific:userland init parser and guest PID1 smoke tests
#[cfg(not(test))]
use core::arch::{asm, global_asm};

const DEFAULT_RUNLEVEL: u8 = b'3';
const MAX_ENTRIES: usize = 16;
const MAX_PROCESS: usize = 192;
#[cfg(not(test))]
const MAX_ARGS: usize = 12;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct InittabEntry {
    runlevels: [u8; 8],
    runlevels_len: usize,
    action: Action,
    process: [u8; MAX_PROCESS],
    process_len: usize,
}

impl InittabEntry {
    const fn empty() -> Self {
        Self {
            runlevels: [0; 8],
            runlevels_len: 0,
            action: Action::Other,
            process: [0; MAX_PROCESS],
            process_len: 0,
        }
    }

    fn runlevel_matches(&self, runlevel: u8) -> bool {
        self.runlevels_len == 0 || self.runlevels[..self.runlevels_len].contains(&runlevel)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    InitDefault,
    SysInit,
    Wait,
    Respawn,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Inittab {
    entries: [InittabEntry; MAX_ENTRIES],
    len: usize,
}

impl Inittab {
    const fn empty() -> Self {
        Self {
            entries: [InittabEntry::empty(); MAX_ENTRIES],
            len: 0,
        }
    }
}

fn parse_inittab_bytes(input: &[u8]) -> Inittab {
    let mut table = Inittab::empty();
    let mut line_start = 0usize;
    while line_start < input.len() {
        let mut line_end = line_start;
        while line_end < input.len() && input[line_end] != b'\n' {
            line_end += 1;
        }
        parse_line(&input[line_start..line_end], &mut table);
        line_start = line_end.saturating_add(1);
    }
    table
}

fn parse_line(mut line: &[u8], table: &mut Inittab) {
    line = trim(line);
    if line.is_empty() || line[0] == b'#' || table.len >= MAX_ENTRIES {
        return;
    }

    let Some((_, rest)) = split_once(line, b':') else {
        return;
    };
    let Some((runlevels, rest)) = split_once(rest, b':') else {
        return;
    };
    let Some((action, process)) = split_once(rest, b':') else {
        return;
    };

    let mut entry = InittabEntry::empty();
    entry.action = parse_action(action);
    entry.runlevels_len = copy_bytes(runlevels, &mut entry.runlevels);
    entry.process_len = copy_bytes(trim(process), &mut entry.process);
    table.entries[table.len] = entry;
    table.len += 1;
}

fn trim(mut s: &[u8]) -> &[u8] {
    while matches!(s.first(), Some(b' ' | b'\t' | b'\r')) {
        s = &s[1..];
    }
    while matches!(s.last(), Some(b' ' | b'\t' | b'\r')) {
        s = &s[..s.len() - 1];
    }
    s
}

fn split_once(s: &[u8], byte: u8) -> Option<(&[u8], &[u8])> {
    let mut i = 0usize;
    while i < s.len() {
        if s[i] == byte {
            return Some((&s[..i], &s[i + 1..]));
        }
        i += 1;
    }
    None
}

fn parse_action(s: &[u8]) -> Action {
    match s {
        b"initdefault" => Action::InitDefault,
        b"sysinit" => Action::SysInit,
        b"wait" => Action::Wait,
        b"respawn" => Action::Respawn,
        _ => Action::Other,
    }
}

fn copy_bytes(src: &[u8], dst: &mut [u8]) -> usize {
    let mut i = 0usize;
    while i < src.len() && i + 1 < dst.len() {
        dst[i] = src[i];
        i += 1;
    }
    if !dst.is_empty() {
        dst[i] = 0;
    }
    i
}

fn default_runlevel(table: &Inittab) -> u8 {
    let mut i = 0usize;
    while i < table.len {
        let entry = table.entries[i];
        if entry.action == Action::InitDefault && entry.runlevels_len > 0 {
            return entry.runlevels[0];
        }
        i += 1;
    }
    DEFAULT_RUNLEVEL
}

#[cfg(not(test))]
const AT_FDCWD: isize = -100;
#[cfg(not(test))]
const O_RDONLY: usize = 0;
#[cfg(not(test))]
const SYS_SETSID: usize = 112;

#[cfg(not(test))]
unsafe fn syscall1(n: usize, a0: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!("syscall", inlateout("rax") n as isize => ret, in("rdi") a0, out("rcx") _, out("r11") _, options(nostack));
    }
    ret
}

#[cfg(not(test))]
unsafe fn syscall0(n: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!("syscall", inlateout("rax") n as isize => ret, out("rcx") _, out("r11") _, options(nostack));
    }
    ret
}

#[cfg(not(test))]
unsafe fn syscall3(n: usize, a0: usize, a1: usize, a2: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!("syscall", inlateout("rax") n as isize => ret, in("rdi") a0, in("rsi") a1, in("rdx") a2, out("rcx") _, out("r11") _, options(nostack));
    }
    ret
}

#[cfg(not(test))]
unsafe fn syscall4(n: usize, a0: usize, a1: usize, a2: usize, a3: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!("syscall", inlateout("rax") n as isize => ret, in("rdi") a0, in("rsi") a1, in("rdx") a2, in("r10") a3, out("rcx") _, out("r11") _, options(nostack));
    }
    ret
}

#[cfg(not(test))]
fn read_inittab(buf: &mut [u8]) -> usize {
    let path = b"/etc/inittab\0";
    let fd = unsafe { syscall4(257, AT_FDCWD as usize, path.as_ptr() as usize, O_RDONLY, 0) };
    if fd < 0 {
        return 0;
    }
    let n = unsafe { syscall3(0, fd as usize, buf.as_mut_ptr() as usize, buf.len()) };
    unsafe {
        let _ = syscall1(3, fd as usize);
    }
    if n > 0 { n as usize } else { 0 }
}

#[cfg(not(test))]
fn wait_child(pid: isize) -> isize {
    let mut status = 0i32;
    loop {
        let ret = unsafe { syscall4(61, pid as usize, &mut status as *mut i32 as usize, 0, 0) };
        if ret == pid || ret < 0 {
            return ret;
        }
        unsafe {
            let _ = syscall0(24);
        }
    }
}

#[cfg(not(test))]
fn command_buf(entry: &InittabEntry) -> [u8; MAX_PROCESS] {
    let mut out = [0u8; MAX_PROCESS];
    let mut i = 0usize;
    while i < entry.process_len && i + 1 < out.len() {
        out[i] = entry.process[i];
        i += 1;
    }
    out
}

#[cfg(not(test))]
fn script_command_buf(entry: &InittabEntry) -> [u8; MAX_PROCESS] {
    let mut out = [0u8; MAX_PROCESS];
    let prefix = b"/bin/sh ";
    let mut i = 0usize;
    while i < prefix.len() {
        out[i] = prefix[i];
        i += 1;
    }
    let mut j = 0usize;
    while j < entry.process_len && i + 1 < out.len() {
        out[i] = entry.process[j];
        i += 1;
        j += 1;
    }
    out
}

#[cfg(not(test))]
fn split_argv(buf: &mut [u8]) -> ([*const u8; MAX_ARGS], usize) {
    let mut argv = [core::ptr::null(); MAX_ARGS];
    let mut argc = 0usize;
    let mut i = 0usize;
    while i < buf.len() && buf[i] != 0 && argc + 1 < MAX_ARGS {
        while i < buf.len() && matches!(buf[i], b' ' | b'\t') {
            buf[i] = 0;
            i += 1;
        }
        if i >= buf.len() || buf[i] == 0 {
            break;
        }
        argv[argc] = buf.as_ptr().wrapping_add(i);
        argc += 1;
        while i < buf.len() && !matches!(buf[i], 0 | b' ' | b'\t') {
            i += 1;
        }
        if i < buf.len() && buf[i] != 0 {
            buf[i] = 0;
            i += 1;
        }
    }
    (argv, argc)
}

#[cfg(not(test))]
fn envp() -> [*const u8; 8] {
    [
        b"PATH=/sbin:/bin:/usr/sbin:/usr/bin\0".as_ptr(),
        b"TERM=linux\0".as_ptr(),
        b"HOME=/root\0".as_ptr(),
        b"SHELL=/bin/bash\0".as_ptr(),
        b"USER=root\0".as_ptr(),
        b"LOGNAME=root\0".as_ptr(),
        b"INIT_VERSION=lupos\0".as_ptr(),
        core::ptr::null(),
    ]
}

#[cfg(not(test))]
fn exec_argv(buf: &mut [u8; MAX_PROCESS]) -> ! {
    let (argv, argc) = split_argv(buf);
    if argc == 0 {
        unsafe {
            let _ = syscall1(60, 0);
        }
    }
    let env = envp();
    unsafe {
        let _ = syscall3(59, argv[0] as usize, argv.as_ptr() as usize, env.as_ptr() as usize);
        let _ = syscall1(60, 127);
    }
    loop {}
}

#[cfg(not(test))]
fn fork_exec(command: &mut [u8; MAX_PROCESS], new_session: bool) -> isize {
    let pid = unsafe { syscall0(57) };
    if pid == 0 {
        if new_session {
            unsafe {
                let _ = syscall0(SYS_SETSID);
            }
        }
        exec_argv(command);
    }
    pid
}

#[cfg(not(test))]
fn bytes_starts_with(haystack: &[u8], needle: &[u8]) -> bool {
    if haystack.len() < needle.len() {
        return false;
    }
    let mut i = 0usize;
    while i < needle.len() {
        if haystack[i] != needle[i] {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(not(test))]
fn run_entries(table: &Inittab, runlevel: u8, action: Action) {
    let mut i = 0usize;
    while i < table.len {
        let entry = table.entries[i];
        if entry.action == action && entry.runlevel_matches(runlevel) && entry.process_len > 0 {
            let mut command = if bytes_starts_with(&entry.process[..entry.process_len], b"/etc/rc.d/") {
                script_command_buf(&entry)
            } else {
                command_buf(&entry)
            };
            let pid = fork_exec(&mut command, false);
            if pid > 0 {
                let _ = wait_child(pid);
            }
        }
        i += 1;
    }
}

#[cfg(not(test))]
fn spawn_first_respawn(table: &Inittab, runlevel: u8) -> ! {
    loop {
        let mut i = 0usize;
        while i < table.len {
            let entry = table.entries[i];
            if entry.action == Action::Respawn
                && entry.runlevel_matches(runlevel)
                && entry.process_len > 0
            {
                let mut command = command_buf(&entry);
                let pid = fork_exec(&mut command, true);
                if pid > 0 {
                    let _ = wait_child(pid);
                }
                break;
            }
            i += 1;
        }
    }
}

#[cfg(not(test))]
global_asm!(
    r#"
    .global _start
    .type _start,@function
_start:
    and rsp, -16
    call lupos_init_main
1:
    jmp 1b
"#
);

#[cfg(not(test))]
#[unsafe(no_mangle)]
pub extern "C" fn lupos_init_main() -> ! {
    let mut buf = [0u8; 4096];
    let n = read_inittab(&mut buf);
    let table = if n > 0 {
        parse_inittab_bytes(&buf[..n])
    } else {
        parse_inittab_bytes(b"id:3:initdefault:\nsi::sysinit:/etc/rc.d/rcS\nc1:12345:respawn:/sbin/agetty 115200 tty1 linux\n")
    };
    let runlevel = default_runlevel(&table);
    run_entries(&table, runlevel, Action::SysInit);
    run_entries(&table, runlevel, Action::Wait);
    spawn_first_respawn(&table, runlevel);
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe {
        let _ = syscall1(60, 127);
    }
    loop {}
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0usize;
    while i < n {
        unsafe {
            *dst.add(i) = *src.add(i);
        }
        i += 1;
    }
    dst
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memset(dst: *mut u8, value: i32, n: usize) -> *mut u8 {
    let mut i = 0usize;
    while i < n {
        unsafe {
            *dst.add(i) = value as u8;
        }
        i += 1;
    }
    dst
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sysv_inittab_actions() {
        let entries = parse_inittab_bytes(
            b"id:3:initdefault:\nsi::sysinit:/etc/rc.d/rcS\nc1:12345:respawn:/sbin/agetty 115200 tty1 linux\n",
        );
        assert_eq!(default_runlevel(&entries), b'3');
        assert_eq!(entries.len, 3);
        assert!(
            entries.entries[..entries.len]
                .iter()
                .any(|entry| entry.action == Action::SysInit)
        );
        assert!(
            entries.entries[..entries.len]
                .iter()
                .any(|entry| entry.action == Action::Respawn)
        );
    }
}
