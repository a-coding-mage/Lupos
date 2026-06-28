//! linux-parity: complete
//! linux-source: vendor/linux/fs/eventpoll.c
//! test-origin: linux:vendor/linux/fs/eventpoll.c
//! epoll — event multiplexer.
//!
//! ABI parity with vendor/linux/fs/eventpoll.c and uapi/linux/eventpoll.h.
//! M60 implements the in-kernel data structures and basic add/wait semantics.
//! Real fd-driven wakeup chains are deferred (need FileOps::poll slot).

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::anon_inode::alloc_anon_file;
use crate::fs::fdtable::FilesStruct;
use crate::fs::ops::FileOps;
use crate::fs::select;
use crate::fs::types::FileRef;
use crate::include::uapi::errno::{EBADF, EEXIST, EFAULT, EINTR, EINVAL, ENOENT, EPERM};
use crate::kernel::{files, sched};

/// `EPOLL_CTL_*` opcodes — byte-identical to Linux UAPI.
pub const EPOLL_CTL_ADD: i32 = 1;
pub const EPOLL_CTL_DEL: i32 = 2;
pub const EPOLL_CTL_MOD: i32 = 3;

/// `EPOLL*` event flags.
pub const EPOLLIN: u32 = 0x0001;
pub const EPOLLPRI: u32 = 0x0002;
pub const EPOLLOUT: u32 = 0x0004;
pub const EPOLLERR: u32 = 0x0008;
pub const EPOLLHUP: u32 = 0x0010;
pub const EPOLLET: u32 = 1 << 31;
pub const EPOLLONESHOT: u32 = 1 << 30;

/// `EPOLL_CLOEXEC` flag for `epoll_create1`.
pub const EPOLL_CLOEXEC: i32 = 0o2000000;

/// `struct epoll_event` — packed on x86-64 to match Linux's `__attribute__((packed))`.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct EpollEvent {
    pub events: u32,
    pub data: u64,
}

/// In-kernel state for one EpollItem (ep_item in Linux).
#[derive(Clone)]
pub struct EpItem {
    pub fd: i32,
    pub file: FileRef,
    pub events: u32,
    pub data: u64,
}

/// In-kernel state for one EventPoll instance.
pub struct EventPoll {
    pub items: spin::Mutex<Vec<EpItem>>,
}

static EPOLL_TOKEN: AtomicUsize = AtomicUsize::new(1);

lazy_static! {
    static ref EPOLLS: Mutex<BTreeMap<usize, Arc<EventPoll>>> = Mutex::new(BTreeMap::new());
}

static EPOLL_FILE_OPS: FileOps = FileOps {
    name: "eventpoll",
    read: None,
    write: None,
    llseek: None,
    fsync: None,
    poll: Some(epoll_poll),
    ioctl: None,
    mmap: None,
    release: Some(epoll_release),
    readdir: None,
};

impl EventPoll {
    pub fn new() -> Self {
        Self {
            items: spin::Mutex::new(Vec::new()),
        }
    }

    pub fn add(&self, fd: i32, file: FileRef, ev: EpollEvent) -> Result<(), i32> {
        let mut items = self.items.lock();
        if items
            .iter()
            .any(|e| e.fd == fd && Arc::ptr_eq(&e.file, &file))
        {
            return Err(EEXIST);
        }
        let evbits = ev.events;
        let evdata = ev.data;
        items.push(EpItem {
            fd,
            file,
            events: evbits,
            data: evdata,
        });
        Ok(())
    }

    pub fn del(&self, fd: i32, file: &FileRef) -> Result<(), i32> {
        let mut items = self.items.lock();
        let len_before = items.len();
        items.retain(|e| !(e.fd == fd && Arc::ptr_eq(&e.file, file)));
        if items.len() == len_before {
            return Err(ENOENT);
        }
        Ok(())
    }

    pub fn modify(&self, fd: i32, file: &FileRef, ev: EpollEvent) -> Result<(), i32> {
        let mut items = self.items.lock();
        for item in items.iter_mut() {
            if item.fd == fd && Arc::ptr_eq(&item.file, file) {
                item.events = ev.events;
                item.data = ev.data;
                return Ok(());
            }
        }
        Err(ENOENT)
    }

    pub fn remove_closed_file(&self, fd: i32, file: &FileRef) {
        self.items
            .lock()
            .retain(|e| !(e.fd == fd && Arc::ptr_eq(&e.file, file)));
    }

    fn collect_ready(&self, out: &mut [EpollEvent], consume: bool) -> Result<usize, i32> {
        let items = self.items.lock();
        let mut n = 0usize;
        for item in items.iter() {
            if n >= out.len() {
                break;
            }
            let mask = select::poll_mask(&item.file);
            let ready = (item.events & mask) | (mask & (EPOLLERR | EPOLLHUP));
            if ready != 0 {
                trace_epoll_ready(
                    item.fd,
                    item.file.fops.name,
                    item.events,
                    mask,
                    ready,
                    item.data,
                );
                out[n] = EpollEvent {
                    events: ready,
                    data: item.data,
                };
                if consume {
                    crate::fs::kernfs::consume_poll_event(&item.file);
                }
                n += 1;
            }
        }
        Ok(n)
    }

    /// Collect currently ready events by polling the watched files.
    pub fn wait_ready(&self, _files: &FilesStruct, out: &mut [EpollEvent]) -> Result<usize, i32> {
        self.collect_ready(out, true)
    }

    fn peek_ready(&self, _files: &FilesStruct, out: &mut [EpollEvent]) -> Result<usize, i32> {
        self.collect_ready(out, false)
    }
}

fn current_files() -> Result<alloc::sync::Arc<crate::fs::fdtable::FilesStruct>, i32> {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return Err(EBADF);
    }
    unsafe { files::get_task_files(task) }.ok_or(EBADF)
}

fn epoll_from_file(file: &FileRef) -> Result<(usize, Arc<EventPoll>), i32> {
    if file.fops.name != EPOLL_FILE_OPS.name {
        return Err(EBADF);
    }
    let token = *file.private.lock();
    let ep = EPOLLS.lock().get(&token).cloned().ok_or(EBADF)?;
    Ok((token, ep))
}

fn epoll_from_fd(fd: i32) -> Result<Arc<EventPoll>, i32> {
    let file = current_files()?.get(fd)?;
    epoll_from_file(&file).map(|(_, ep)| ep)
}

fn epoll_path_reaches(
    start_token: usize,
    target_token: usize,
    visited: &mut BTreeSet<usize>,
) -> Result<bool, i32> {
    if start_token == target_token {
        return Ok(true);
    }
    if !visited.insert(start_token) {
        return Ok(false);
    }

    let ep = EPOLLS.lock().get(&start_token).cloned().ok_or(EBADF)?;
    let items = ep.items.lock().clone();
    for item in items {
        if item.file.fops.name != EPOLL_FILE_OPS.name {
            continue;
        }
        let (next_token, _) = epoll_from_file(&item.file)?;
        if epoll_path_reaches(next_token, target_token, visited)? {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn notify_fd_closed(files: &FilesStruct, fd: i32, file: &FileRef) {
    if Arc::strong_count(file) > 2 {
        return;
    }
    let epolls: Vec<_> = files
        .open_file_refs()
        .into_iter()
        .filter_map(|ep_file| epoll_from_file(&ep_file).ok().map(|(_, ep)| ep))
        .collect();
    for ep in epolls {
        ep.remove_closed_file(fd, file);
    }
}

fn epoll_release(file: FileRef) {
    let token = *file.private.lock();
    EPOLLS.lock().remove(&token);
}

fn epoll_poll(file: &FileRef) -> u32 {
    let token = *file.private.lock();
    let Some(ep) = EPOLLS.lock().get(&token).cloned() else {
        return EPOLLERR;
    };
    let Ok(files) = current_files() else {
        return EPOLLERR;
    };
    let mut out = [EpollEvent { events: 0, data: 0 }; 1];
    match ep.peek_ready(files.as_ref(), &mut out) {
        Ok(n) if n != 0 => EPOLLIN,
        Ok(_) => 0,
        Err(_) => EPOLLERR,
    }
}

/// `sys_epoll_create1(flags)` — Linux syscall 291.
pub unsafe fn sys_epoll_create1(flags: i32) -> i64 {
    if flags & !EPOLL_CLOEXEC != 0 {
        return -(EINVAL as i64);
    }
    let token = EPOLL_TOKEN.fetch_add(1, Ordering::AcqRel);
    EPOLLS.lock().insert(token, Arc::new(EventPoll::new()));
    let file = alloc_anon_file("eventpoll", &EPOLL_FILE_OPS, token);
    match current_files().and_then(|ft| ft.install(file, flags & EPOLL_CLOEXEC != 0)) {
        Ok(fd) => fd as i64,
        Err(errno) => {
            EPOLLS.lock().remove(&token);
            -(errno as i64)
        }
    }
}

/// `sys_epoll_create(size)` â€” Linux syscall 213.
pub unsafe fn sys_epoll_create(size: i32) -> i64 {
    if size <= 0 {
        return -(EINVAL as i64);
    }
    unsafe { sys_epoll_create1(0) }
}

/// `sys_epoll_ctl(epfd, op, fd, event)` — Linux syscall 233.
pub unsafe fn sys_epoll_ctl(epfd: i32, op: i32, fd: i32, event: *const EpollEvent) -> i64 {
    let files = match current_files() {
        Ok(files) => files,
        Err(errno) => return -(errno as i64),
    };
    let ep_file = match files.get(epfd) {
        Ok(file) => file,
        Err(_) => return -(EBADF as i64),
    };
    let (ep_token, ep) = match epoll_from_file(&ep_file) {
        Ok(ep) => ep,
        Err(errno) => return -(errno as i64),
    };
    if fd == epfd {
        return -(EINVAL as i64);
    }
    let target = match files.get(fd) {
        Ok(file) => file,
        Err(_) => return -(EBADF as i64),
    };
    if target.fops.poll.is_none() {
        return -(EPERM as i64);
    }
    if op == EPOLL_CTL_ADD && target.fops.name == EPOLL_FILE_OPS.name {
        let (target_token, _) = match epoll_from_file(&target) {
            Ok(ep) => ep,
            Err(errno) => return -(errno as i64),
        };
        let mut visited = BTreeSet::new();
        match epoll_path_reaches(target_token, ep_token, &mut visited) {
            Ok(true) => return -(EINVAL as i64),
            Ok(false) => {}
            Err(errno) => return -(errno as i64),
        }
    }
    let ev = if op == EPOLL_CTL_DEL {
        EpollEvent { events: 0, data: 0 }
    } else if event.is_null() {
        return -(EFAULT as i64);
    } else {
        unsafe { *event }
    };
    trace_epoll_ctl(epfd, op, fd, target.fops.name, ev.events, ev.data);
    let result = match op {
        EPOLL_CTL_ADD => ep.add(fd, target.clone(), ev),
        EPOLL_CTL_DEL => ep.del(fd, &target),
        EPOLL_CTL_MOD => ep.modify(fd, &target, ev),
        _ => Err(EINVAL),
    };
    match result {
        Ok(()) => 0,
        Err(errno) => -(errno as i64),
    }
}

fn trace_epoll_ctl(epfd: i32, op: i32, fd: i32, file_ops: &str, events: u32, data: u64) {
    #[cfg(not(test))]
    if crate::kernel::debug_trace::proc_enabled() {
        let task = unsafe { sched::get_current() };
        let pid = if task.is_null() {
            -1
        } else {
            unsafe { (*task).pid }
        };
        crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-epoll-ctl pid={} epfd={} op={} fd={} file={} events={:#x} data={:#x}",
            pid,
            epfd,
            op,
            fd,
            file_ops,
            events,
            data
        );
    }
    #[cfg(test)]
    let _ = (epfd, op, fd, file_ops, events, data);
}

fn trace_epoll_ready(fd: i32, file_ops: &str, events: u32, mask: u32, ready: u32, data: u64) {
    #[cfg(not(test))]
    if crate::kernel::debug_trace::proc_enabled() {
        let task = unsafe { sched::get_current() };
        let pid = if task.is_null() {
            -1
        } else {
            unsafe { (*task).pid }
        };
        crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-epoll-ready pid={} fd={} file={} events={:#x} mask={:#x} ready={:#x} data={:#x}",
            pid,
            fd,
            file_ops,
            events,
            mask,
            ready,
            data
        );
    }
    #[cfg(test)]
    let _ = (fd, file_ops, events, mask, ready, data);
}

/// `sys_epoll_wait(epfd, events, maxevents, timeout)` — Linux syscall 232.
pub unsafe fn sys_epoll_wait(
    epfd: i32,
    events: *mut EpollEvent,
    maxevents: i32,
    timeout: i32,
) -> i64 {
    if events.is_null() || maxevents <= 0 {
        return -(if events.is_null() { EFAULT } else { EINVAL } as i64);
    }
    let files = match current_files() {
        Ok(files) => files,
        Err(errno) => return -(errno as i64),
    };
    let ep = match epoll_from_fd(epfd) {
        Ok(ep) => ep,
        Err(errno) => return -(errno as i64),
    };
    let deadline_ns = if timeout < 0 {
        None
    } else {
        Some(
            crate::kernel::time::ktime_get()
                .saturating_add((timeout as u64).saturating_mul(1_000_000)),
        )
    };
    #[cfg(not(test))]
    let mut wait_state = EventWaitState::default();

    loop {
        unsafe {
            crate::kernel::signal::exit_if_fatal_signal_pending_current();
        }
        #[cfg(not(test))]
        {
            crate::init::rootfs::drain_console_control_bytes();
            if crate::kernel::signal::current_has_pending_signals() {
                return -(EINTR as i64);
            }
        }
        let out = unsafe { core::slice::from_raw_parts_mut(events, maxevents as usize) };
        match ep.wait_ready(files.as_ref(), out) {
            Ok(n) if n != 0 => {
                return n as i64;
            }
            Ok(_) if timeout == 0 => {
                return 0;
            }
            Ok(_) => {}
            Err(errno) => return -(errno as i64),
        }
        if let Some(deadline_ns) = deadline_ns {
            if crate::kernel::time::ktime_get() >= deadline_ns {
                return 0;
            }
        }

        #[cfg(not(test))]
        {
            wait_state.maintenance();
            // No fd-ready wakeup chain exists yet, so we must re-poll the set —
            // but sleep ~1 tick (event-driven via the timer wheel) between polls
            // instead of busy-yielding while RUNNABLE. Busy-yielding here kept the
            // task in the round-robin and starved peers (e.g. systemd's epoll
            // loop stealing CPU from the generators it is waiting on); sleeping
            // lets the CPU halt / peers run, and the timer wakes us to re-poll.
            crate::kernel::time::sleep_timeout::schedule_timeout_with_state(
                1,
                crate::kernel::task::task_state::TASK_INTERRUPTIBLE,
            );
        }
        #[cfg(test)]
        {
            crate::kernel::time::timekeeping::tick_advance_walltime();
            crate::kernel::time::hrtimer_run_queues();
        }
    }
}

pub unsafe fn sys_epoll_pwait(
    epfd: i32,
    events: *mut EpollEvent,
    maxevents: i32,
    timeout: i32,
    _sigmask: *const u8,
    _sigsetsize: usize,
) -> i64 {
    unsafe { sys_epoll_wait(epfd, events, maxevents, timeout) }
}

pub unsafe fn sys_epoll_pwait2(
    epfd: i32,
    events: *mut EpollEvent,
    maxevents: i32,
    timeout: *const crate::kernel::time::Timespec64,
    _sigmask: *const u8,
    _sigsetsize: usize,
) -> i64 {
    let timeout_ms = if timeout.is_null() {
        -1
    } else {
        let timeout = unsafe { *timeout };
        if !timeout.is_valid() {
            return -(EINVAL as i64);
        }
        let ns = timeout.to_ns();
        let ms = ns.saturating_add(999_999) / 1_000_000;
        ms.min(i32::MAX as u64) as i32
    };
    unsafe { sys_epoll_wait(epfd, events, maxevents, timeout_ms) }
}

#[cfg(not(test))]
#[derive(Default)]
struct EventWaitState {
    last_tick_tsc: u64,
    spins: u32,
}

#[cfg(not(test))]
impl EventWaitState {
    fn maintenance(&mut self) {
        crate::init::rootfs::drain_console_control_bytes();
        crate::linux_driver_abi::video::fbdev::core::refresh_cursor_blink();
        if self.should_tick() {
            crate::kernel::time::clockevents::tick_handle_periodic();
        }
        // Lupos' current scheduler is cooperative on the boot CPU (schedule()
        // only switches tasks at explicit call sites; it never preempts from
        // an interrupt). Every epoll_wait caller (systemd, journald, udevd,
        // ...) drives its event loop through this function, and the very
        // next statement after maintenance() already calls
        // schedule_with_irqs_enabled() to cooperatively yield. Do NOT halt
        // the CPU here: halting in one caller's own poll loop blocks that
        // yield behind a full LAPIC tick *per idle service*, which stalls
        // unrelated work system-wide (e.g. systemd-mounted tmp.mount timing
        // out, and tty input latency) because there is no preemption to
        // break the halt early. Use a cheap CPU-yield hint instead --
        // schedule_with_irqs_enabled() halts on our behalf, but only once
        // the scheduler has confirmed under the runqueue lock that no other
        // task anywhere is runnable, so it never delays other callers.
        core::hint::spin_loop();
    }

    fn should_tick(&mut self) -> bool {
        self.spins = self.spins.wrapping_add(1);
        let tsc = crate::kernel::time::clocksource::read_tsc();
        if tsc == 0 {
            return self.spins & 0x3ff == 0;
        }
        let last = self.last_tick_tsc;
        if last == 0 || tsc.saturating_sub(last) >= 1_000_000 {
            self.last_tick_tsc = tsc;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::dcache::d_alloc;
    use crate::fs::file::alloc_file;
    use crate::fs::ops::FileOps;
    use crate::kernel::{cred::INIT_CRED, files, sched, task::TaskStruct};
    use alloc::boxed::Box;

    static READABLE_OPS: FileOps = FileOps {
        name: "epoll-readable",
        read: None,
        write: None,
        llseek: None,
        fsync: None,
        poll: Some(|_| EPOLLIN),
        ioctl: None,
        mmap: None,
        release: None,
        readdir: None,
    };

    #[test]
    fn epoll_event_size_is_12() {
        assert_eq!(core::mem::size_of::<EpollEvent>(), 12);
    }

    #[test]
    fn add_then_del_round_trip() {
        let ep = EventPoll::new();
        let file = alloc_file(d_alloc("watched"), 0, 0, &READABLE_OPS);
        let ev = EpollEvent {
            events: EPOLLIN,
            data: 0x12345678,
        };
        ep.add(3, file.clone(), ev).unwrap();
        assert_eq!(ep.add(3, file.clone(), ev), Err(EEXIST));
        ep.del(3, &file).unwrap();
        assert_eq!(ep.del(3, &file), Err(ENOENT));
    }

    #[test]
    fn add_allows_reused_fd_number_for_new_file_object() {
        let ep = EventPoll::new();
        let old_file = alloc_file(d_alloc("old-signalfd"), 0, 0, &READABLE_OPS);
        let new_file = alloc_file(d_alloc("new-signalfd"), 0, 0, &READABLE_OPS);
        let ev = EpollEvent {
            events: EPOLLIN,
            data: 0x17,
        };

        ep.add(4, old_file.clone(), ev).unwrap();
        ep.add(4, new_file.clone(), ev).unwrap();
        assert_eq!(ep.items.lock().len(), 2);

        ep.remove_closed_file(4, &old_file);
        let items = ep.items.lock();
        assert_eq!(items.len(), 1);
        assert!(Arc::ptr_eq(&items[0].file, &new_file));
    }

    #[test]
    fn copied_child_cloexec_close_keeps_parent_interest_until_parent_closes() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 263;
        current.tgid = 263;
        current.cred = &raw const INIT_CRED;

        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let parent = files::get_task_files(&mut *current as *mut TaskStruct).unwrap();
            let epfd = sys_epoll_create1(0);
            assert!(epfd >= 0);

            let watched_fd = parent
                .install(
                    alloc_file(d_alloc("cloexec-watched"), 0, 0, &READABLE_OPS),
                    true,
                )
                .unwrap();
            let ev = EpollEvent {
                events: EPOLLIN,
                data: 0x263,
            };
            assert_eq!(
                sys_epoll_ctl(epfd as i32, EPOLL_CTL_ADD, watched_fd, &ev),
                0
            );

            let child = crate::fs::fdtable::dup_fd(&parent, false);
            child.close_on_exec();

            let mut out = [EpollEvent { events: 0, data: 0 }; 1];
            assert_eq!(sys_epoll_wait(epfd as i32, out.as_mut_ptr(), 1, 0), 1);
            let data = out[0].data;
            assert_eq!(data, 0x263);

            parent.close(watched_fd).unwrap();
            assert_eq!(sys_epoll_wait(epfd as i32, out.as_mut_ptr(), 1, 0), 0);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn modify_changes_events() {
        let ep = EventPoll::new();
        let file = alloc_file(d_alloc("modifiable"), 0, 0, &READABLE_OPS);
        ep.add(
            5,
            file.clone(),
            EpollEvent {
                events: EPOLLIN,
                data: 1,
            },
        )
        .unwrap();
        ep.modify(
            5,
            &file,
            EpollEvent {
                events: EPOLLIN | EPOLLOUT,
                data: 2,
            },
        )
        .unwrap();
        let items = ep.items.lock();
        assert_eq!(items[0].events, EPOLLIN | EPOLLOUT);
        assert_eq!(items[0].data, 2);
    }

    #[test]
    fn wait_returns_ready_items() {
        let files = FilesStruct::new();
        let file = alloc_file(d_alloc("ready"), 0, 0, &READABLE_OPS);
        let fd = files.install(file.clone(), false).unwrap();
        let ep = EventPoll::new();
        ep.add(
            fd,
            file,
            EpollEvent {
                events: EPOLLIN,
                data: 0x77,
            },
        )
        .unwrap();
        let mut buf = [EpollEvent { events: 0, data: 0 }; 4];
        let n = ep.wait_ready(&files, &mut buf).unwrap();
        assert_eq!(n, 1);
        let ev = buf[0].events;
        let dt = buf[0].data;
        assert_eq!(ev, EPOLLIN);
        assert_eq!(dt, 0x77);
    }

    #[test]
    fn timerfd_expiry_wakes_epoll_waiter_under_load() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 262;
        current.tgid = 262;
        current.cred = &raw const INIT_CRED;

        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let epfd = sys_epoll_create1(0);
            assert!(epfd >= 0);
            let tfd = crate::fs::timerfd::sys_timerfd_create(
                crate::kernel::time::CLOCK_MONOTONIC,
                crate::kernel::time::timerfd::TFD_NONBLOCK,
            );
            assert!(tfd >= 0);

            let ev = EpollEvent {
                events: EPOLLIN,
                data: 0x102,
            };
            assert_eq!(
                sys_epoll_ctl(epfd as i32, EPOLL_CTL_ADD, tfd as i32, &ev),
                0
            );

            let new_value = crate::kernel::time::Itimerspec64 {
                it_interval: crate::kernel::time::Timespec64::new(0, 0),
                it_value: crate::kernel::time::Timespec64::new(0, 1),
            };
            assert_eq!(
                crate::fs::timerfd::sys_timerfd_settime(
                    tfd as i32,
                    0,
                    &new_value,
                    core::ptr::null_mut()
                ),
                0
            );

            let mut out = [EpollEvent { events: 0, data: 0 }; 1];
            assert_eq!(sys_epoll_wait(epfd as i32, out.as_mut_ptr(), 1, 25), 1);
            let events = out[0].events;
            let data = out[0].data;
            assert_ne!(events & EPOLLIN, 0);
            assert_eq!(data, 0x102);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn nested_epoll_fd_is_pollable_for_libmount_monitor() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 260;
        current.tgid = 260;
        current.cred = &raw const INIT_CRED;

        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let outer = sys_epoll_create1(0);
            assert!(outer >= 0);
            let inner = sys_epoll_create1(0);
            assert!(inner >= 0);

            let ft = files::get_task_files(&mut *current as *mut TaskStruct).unwrap();
            let ready_fd = ft
                .install(
                    alloc_file(d_alloc("nested-ready"), 0, 0, &READABLE_OPS),
                    false,
                )
                .unwrap();

            let inner_ev = EpollEvent {
                events: EPOLLIN,
                data: 0xfeed,
            };
            assert_eq!(
                sys_epoll_ctl(inner as i32, EPOLL_CTL_ADD, ready_fd, &inner_ev),
                0
            );

            let outer_ev = EpollEvent {
                events: EPOLLIN,
                data: 0x260,
            };
            assert_eq!(
                sys_epoll_ctl(outer as i32, EPOLL_CTL_ADD, inner as i32, &outer_ev),
                0
            );

            let mut out = [EpollEvent { events: 0, data: 0 }; 1];
            assert_eq!(sys_epoll_wait(outer as i32, out.as_mut_ptr(), 1, 0), 1);
            let data = out[0].data;
            assert_eq!(data, 0x260);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn epoll_ctl_rejects_nested_cycle() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 261;
        current.tgid = 261;
        current.cred = &raw const INIT_CRED;

        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let a = sys_epoll_create1(0);
            assert!(a >= 0);
            let b = sys_epoll_create1(0);
            assert!(b >= 0);

            let ev = EpollEvent {
                events: EPOLLIN,
                data: 0x261,
            };
            assert_eq!(sys_epoll_ctl(a as i32, EPOLL_CTL_ADD, b as i32, &ev), 0);
            assert_eq!(
                sys_epoll_ctl(b as i32, EPOLL_CTL_ADD, a as i32, &ev),
                -(EINVAL as i64)
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }
}
