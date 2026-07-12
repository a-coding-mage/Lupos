//! linux-parity: partial
//! linux-source: vendor/linux/fs
//! test-origin: linux:vendor/linux/fs
//! inotify â€” file-system change notifications.
//!
//! ABI parity with vendor/linux/fs/notify/inotify/inotify_user.c.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::arch::x86::kernel::uaccess;
use crate::fs::anon_inode::alloc_anon_file;
use crate::fs::ops::FileOps;
use crate::fs::types::{DentryRef, FileRef, InodeKind, InodeRef};
use crate::include::uapi::errno::{
    EAGAIN, EBADF, EEXIST, EFAULT, EINVAL, EMFILE, ENOENT, ENOSPC, ENOTDIR,
};
use crate::kernel::sched::wait::WaitQueueHead;
use crate::kernel::{files, sched};

/// `struct inotify_event` â€” variable-length: trailing `name` array follows.
/// Byte-identical to Linux uapi/linux/inotify.h.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct InotifyEvent {
    pub wd: i32,     // 0:  watch descriptor
    pub mask: u32,   // 4:  mask of events
    pub cookie: u32, // 8:  unique cookie associating related events
    pub len: u32,    // 12: size of name field
                     // name: [u8; len] â€” flexible array follows
}

/// IN_* event flags.
pub const IN_ACCESS: u32 = 0x0000_0001;
pub const IN_MODIFY: u32 = 0x0000_0002;
pub const IN_ATTRIB: u32 = 0x0000_0004;
pub const IN_CLOSE_WRITE: u32 = 0x0000_0008;
pub const IN_CLOSE_NOWRITE: u32 = 0x0000_0010;
pub const IN_OPEN: u32 = 0x0000_0020;
pub const IN_MOVED_FROM: u32 = 0x0000_0040;
pub const IN_MOVED_TO: u32 = 0x0000_0080;
pub const IN_CREATE: u32 = 0x0000_0100;
pub const IN_DELETE: u32 = 0x0000_0200;
pub const IN_DELETE_SELF: u32 = 0x0000_0400;
pub const IN_MOVE_SELF: u32 = 0x0000_0800;
pub const IN_UNMOUNT: u32 = 0x0000_2000;
pub const IN_Q_OVERFLOW: u32 = 0x0000_4000;
pub const IN_IGNORED: u32 = 0x0000_8000;
pub const IN_ONLYDIR: u32 = 0x0100_0000;
pub const IN_DONT_FOLLOW: u32 = 0x0200_0000;
pub const IN_EXCL_UNLINK: u32 = 0x0400_0000;
pub const IN_MASK_CREATE: u32 = 0x1000_0000;
pub const IN_MASK_ADD: u32 = 0x2000_0000;
pub const IN_ISDIR: u32 = 0x4000_0000;
pub const IN_ONESHOT: u32 = 0x8000_0000;
pub const IN_NONBLOCK: i32 = 0o0004000;
pub const IN_CLOEXEC: i32 = 0o2000000;

pub const IN_ALL_EVENTS: u32 = IN_ACCESS
    | IN_MODIFY
    | IN_ATTRIB
    | IN_CLOSE_WRITE
    | IN_CLOSE_NOWRITE
    | IN_OPEN
    | IN_MOVED_FROM
    | IN_MOVED_TO
    | IN_CREATE
    | IN_DELETE
    | IN_DELETE_SELF
    | IN_MOVE_SELF;

const ALL_INOTIFY_BITS: u32 = IN_ALL_EVENTS
    | IN_UNMOUNT
    | IN_Q_OVERFLOW
    | IN_IGNORED
    | IN_ONLYDIR
    | IN_DONT_FOLLOW
    | IN_EXCL_UNLINK
    | IN_MASK_ADD
    | IN_MASK_CREATE
    | IN_ISDIR
    | IN_ONESHOT;

#[derive(Clone)]
struct InotifyWatch {
    wd: i32,
    mask: u32,
    inode_key: usize,
}

struct InotifyQueue {
    events: VecDeque<Vec<u8>>,
    overflow_queued: bool,
}

impl InotifyQueue {
    fn new() -> Self {
        Self {
            events: VecDeque::new(),
            overflow_queued: false,
        }
    }

    fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    fn front(&self) -> Option<&Vec<u8>> {
        self.events.front()
    }

    fn pop_front(&mut self) -> Option<Vec<u8>> {
        let event = self.events.pop_front()?;
        if event_mask(&event) & IN_Q_OVERFLOW != 0 {
            self.overflow_queued = false;
        }
        Some(event)
    }
}

struct InotifyInstance {
    watches: Mutex<Vec<InotifyWatch>>,
    queue: Mutex<InotifyQueue>,
    notification_waitq: WaitQueueHead,
    next_wd: AtomicI32,
}

#[cfg(not(test))]
const MAX_QUEUED_EVENTS: usize = 16_384;
#[cfg(test)]
const MAX_QUEUED_EVENTS: usize = 4;

#[cfg(not(test))]
const MAX_USER_WATCHES: usize = 8_192;
#[cfg(test)]
const MAX_USER_WATCHES: usize = 4;

#[cfg(not(test))]
const MAX_USER_INSTANCES: usize = 128;
#[cfg(test)]
const MAX_USER_INSTANCES: usize = 8;

static INOTIFY_TOKEN: AtomicUsize = AtomicUsize::new(1);

lazy_static! {
    static ref INOTIFIES: Mutex<BTreeMap<usize, Arc<InotifyInstance>>> =
        Mutex::new(BTreeMap::new());
}

static INOTIFY_FILE_OPS: FileOps = FileOps {
    name: "inotify",
    read: Some(inotify_read),
    write: None,
    llseek: None,
    fsync: None,
    poll: Some(inotify_poll),
    ioctl: None,
    mmap: None,
    release: Some(inotify_release),
    readdir: None,
};

fn current_files() -> Result<alloc::sync::Arc<crate::fs::fdtable::FilesStruct>, i32> {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return Err(EBADF);
    }
    unsafe { files::get_task_files(task) }.ok_or(EBADF)
}

fn inotify_token(fd: i32) -> Result<usize, i32> {
    let file = current_files()?.get(fd)?;
    if file.fops.name != INOTIFY_FILE_OPS.name {
        return Err(EINVAL);
    }
    Ok(*file.private.lock())
}

fn inotify_release(file: FileRef) {
    let token = *file.private.lock();
    INOTIFIES.lock().remove(&token);
}

fn inotify_read(file: &FileRef, buf: &mut [u8], _pos: &mut u64) -> Result<usize, i32> {
    let token = *file.private.lock();
    let table = INOTIFIES.lock();
    let instance = table.get(&token).ok_or(EBADF)?;
    let mut queue = instance.queue.lock();
    let Some(first) = queue.front() else {
        return Err(EAGAIN);
    };
    if first.len() > buf.len() {
        return Err(EINVAL);
    }

    let mut copied = 0usize;
    while let Some(event) = queue.front() {
        if copied + event.len() > buf.len() {
            break;
        }
        let event = queue.pop_front().expect("front checked");
        buf[copied..copied + event.len()].copy_from_slice(&event);
        copied += event.len();
    }
    Ok(copied)
}

fn inotify_poll(file: &FileRef, table: Option<&mut crate::fs::select::PollTable>) -> u32 {
    let token = *file.private.lock();
    let instances = INOTIFIES.lock();
    let Some(instance) = instances.get(&token) else {
        return 0;
    };
    crate::fs::select::poll_wait(file, &instance.notification_waitq, table);
    if instance.queue.lock().is_empty() {
        0
    } else {
        crate::fs::eventpoll::EPOLLIN | crate::fs::eventpoll::EPOLLRDNORM
    }
}

/// `sys_inotify_init1(flags)` â€” Linux syscall 294.
pub unsafe fn sys_inotify_init1(flags: i32) -> i64 {
    let allowed = IN_NONBLOCK | IN_CLOEXEC;
    if flags & !allowed != 0 {
        return -(EINVAL as i64);
    }
    let mut table = INOTIFIES.lock();
    if table.len() >= MAX_USER_INSTANCES {
        return -(EMFILE as i64);
    }
    let token = INOTIFY_TOKEN.fetch_add(1, Ordering::AcqRel);
    table.insert(
        token,
        Arc::new(InotifyInstance {
            watches: Mutex::new(Vec::new()),
            queue: Mutex::new(InotifyQueue::new()),
            notification_waitq: WaitQueueHead::new(),
            next_wd: AtomicI32::new(1),
        }),
    );
    drop(table);
    let file = alloc_anon_file("inotify", &INOTIFY_FILE_OPS, token);
    match current_files().and_then(|ft| ft.install(file, flags & IN_CLOEXEC != 0)) {
        Ok(fd) => fd as i64,
        Err(errno) => {
            INOTIFIES.lock().remove(&token);
            -(errno as i64)
        }
    }
}

/// `sys_inotify_add_watch(fd, pathname, mask)` â€” Linux syscall 254.
pub unsafe fn sys_inotify_add_watch(fd: i32, pathname: *const i8, mask: u32) -> i64 {
    if mask & !ALL_INOTIFY_BITS != 0 || mask & ALL_INOTIFY_BITS == 0 {
        return -(EINVAL as i64);
    }
    if mask & IN_MASK_ADD != 0 && mask & IN_MASK_CREATE != 0 {
        return -(EINVAL as i64);
    }
    let path = match unsafe { copy_user_path(pathname) } {
        Ok(path) => path,
        Err(errno) => return -(errno as i64),
    };
    let inode = match lookup_watch_inode(&path, mask) {
        Ok(inode) => inode,
        Err(errno) => return -(errno as i64),
    };
    let inode_key = inode_key(&inode);
    let token = match inotify_token(fd) {
        Ok(token) => token,
        Err(errno) => return -(errno as i64),
    };
    let mut table = INOTIFIES.lock();
    let Some(instance) = table.get_mut(&token) else {
        return -(EBADF as i64);
    };
    let mut watches = instance.watches.lock();
    if let Some(existing) = watches
        .iter_mut()
        .find(|watch| watch.inode_key == inode_key)
    {
        if mask & IN_MASK_CREATE != 0 {
            return -(EEXIST as i64);
        }
        if mask & IN_MASK_ADD != 0 {
            existing.mask |= mask;
        } else {
            existing.mask = mask;
        }
        return existing.wd as i64;
    }

    if watches.len() >= MAX_USER_WATCHES {
        return -(ENOSPC as i64);
    }

    let wd = instance.next_wd.fetch_add(1, Ordering::AcqRel);
    watches.push(InotifyWatch {
        wd,
        mask,
        inode_key,
    });
    wd as i64
}

/// `sys_inotify_rm_watch(fd, wd)` â€” Linux syscall 255.
pub unsafe fn sys_inotify_rm_watch(fd: i32, wd: i32) -> i64 {
    let token = match inotify_token(fd) {
        Ok(token) => token,
        Err(errno) => return -(errno as i64),
    };
    let mut table = INOTIFIES.lock();
    let Some(instance) = table.get_mut(&token) else {
        return -(EBADF as i64);
    };
    let mut watches = instance.watches.lock();
    let before = watches.len();
    watches.retain(|watch| watch.wd != wd);
    if watches.len() == before {
        -(EINVAL as i64)
    } else {
        0
    }
}

unsafe fn copy_user_path(pathname: *const i8) -> Result<String, i32> {
    if pathname.is_null() {
        return Err(EFAULT);
    }
    const PATH_MAX: usize = 4096;
    let mut buf = alloc::vec![0u8; PATH_MAX];
    let n =
        unsafe { uaccess::strncpy_from_user(buf.as_mut_ptr(), pathname as *const u8, buf.len()) };
    if n < 0 {
        return Err((-n) as i32);
    }
    if n == 0 {
        return Err(ENOENT);
    }
    core::str::from_utf8(&buf[..n as usize])
        .map(String::from)
        .map_err(|_| EINVAL)
}

fn lookup_watch_inode(path: &str, mask: u32) -> Result<InodeRef, i32> {
    let effective = crate::fs::fs_struct::absolute_from_cwd(path);
    let lookup = effective.as_str();
    let follow_final = mask & IN_DONT_FOLLOW == 0;
    let (_, dentry) = if follow_final {
        crate::fs::mount::resolve_path_follow(lookup)?
    } else {
        crate::fs::mount::resolve_path_nofollow(lookup)?
    };
    let inode = dentry.inode().ok_or(ENOENT)?;
    if mask & IN_ONLYDIR != 0 && inode.kind != InodeKind::Directory {
        return Err(ENOTDIR);
    }
    Ok(inode)
}

fn inode_key(inode: &InodeRef) -> usize {
    alloc::sync::Arc::as_ptr(inode) as usize
}

fn event_mask(event: &[u8]) -> u32 {
    if event.len() < core::mem::size_of::<InotifyEvent>() {
        return 0;
    }
    u32::from_ne_bytes(event[4..8].try_into().unwrap_or([0; 4]))
}

fn encode_event(wd: i32, mask: u32, name: Option<&str>) -> Vec<u8> {
    let name_len = name.map(|n| (n.len() + 1 + 3) & !3).unwrap_or(0);
    let mut out = alloc::vec![0u8; core::mem::size_of::<InotifyEvent>() + name_len];
    out[0..4].copy_from_slice(&wd.to_ne_bytes());
    out[4..8].copy_from_slice(&mask.to_ne_bytes());
    out[8..12].copy_from_slice(&0u32.to_ne_bytes());
    out[12..16].copy_from_slice(&(name_len as u32).to_ne_bytes());
    if let Some(name) = name {
        out[16..16 + name.len()].copy_from_slice(name.as_bytes());
    }
    out
}

fn enqueue_event(queue: &mut InotifyQueue, wd: i32, mask: u32, name: Option<&str>) -> bool {
    if queue.events.len() < MAX_QUEUED_EVENTS {
        queue.events.push_back(encode_event(wd, mask, name));
        return true;
    }

    if !queue.overflow_queued {
        queue.events.pop_back();
        queue
            .events
            .push_back(encode_event(-1, IN_Q_OVERFLOW, None));
        queue.overflow_queued = true;
        return true;
    }
    false
}

fn queue_event_for_inode(inode: &InodeRef, mask: u32, name: Option<&str>) {
    let key = inode_key(inode);
    let event_bits = mask & IN_ALL_EVENTS;
    if event_bits == 0 {
        return;
    }
    let table = INOTIFIES.lock();
    let mut wake_instances = Vec::new();
    for instance in table.values() {
        let events = {
            let watches = instance.watches.lock();
            watches
                .iter()
                .filter(|watch| watch.inode_key == key && watch.mask & event_bits != 0)
                .map(|watch| (watch.wd, mask & (watch.mask | IN_ISDIR)))
                .collect::<Vec<_>>()
        };
        if events.is_empty() {
            continue;
        }
        let mut queue = instance.queue.lock();
        let mut enqueued = false;
        for (wd, event_mask) in events {
            enqueued |= enqueue_event(&mut queue, wd, event_mask, name);
        }
        drop(queue);
        if enqueued {
            wake_instances.push(instance.clone());
        }
    }
    drop(table);

    // Mirrors fsnotify_insert_event(): publish under notification_lock, then
    // wake notification_waitq after dropping the queue/global locks.  The Arc
    // pins the file-private instance across a concurrent final close.
    for instance in wake_instances {
        instance.notification_waitq.wake_up_all();
    }
}

/// Queue the parent-directory `IN_CREATE` event used by Linux fsnotify when a
/// child dentry is instantiated. This is inode-keyed like
/// `vendor/linux/fs/notify/inotify/inotify_user.c`: watches attach to the
/// watched inode, and directory events carry the child name.
pub fn notify_create(parent: &DentryRef, name: &str, is_dir: bool) {
    if name.is_empty() {
        return;
    }
    let Some(parent_inode) = parent.inode() else {
        return;
    };
    let mut mask = IN_CREATE;
    if is_dir {
        mask |= IN_ISDIR;
    }
    queue_event_for_inode(&parent_inode, mask, Some(name));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::eventpoll::{
        EPOLL_CTL_ADD, EPOLLIN, EpollEvent, sys_epoll_create1, sys_epoll_ctl,
    };
    use crate::fs::fdtable::FilesStruct;
    use crate::fs::mount::{Mount, set_rootfs};
    use crate::fs::read_write::vfs_read;
    use crate::fs::super_block::mount_fs;
    use crate::kernel::{cred::INIT_CRED, files, sched, task::TaskStruct};
    use alloc::boxed::Box;

    fn setup_current_with_rootfs(pid: i32) -> (Box<TaskStruct>, *mut TaskStruct) {
        let previous = unsafe { sched::get_current() };
        crate::fs::init();
        crate::fs::mount::MOUNTS.root.lock().take();
        crate::fs::mount::MOUNTS.by_path.lock().clear();
        let sb = mount_fs("ramfs", "", 0, "").expect("ramfs mount");
        let root = sb.root().expect("root dentry");
        set_rootfs(Mount::alloc(sb, root, 0));
        crate::fs::fs_struct::set_current_cwd_path("/");

        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = pid;
        current.tgid = pid;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);
        }
        (current, previous)
    }

    unsafe fn teardown_current(mut current: Box<TaskStruct>, previous: *mut TaskStruct) {
        unsafe {
            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn event_header_size_is_16() {
        assert_eq!(core::mem::size_of::<InotifyEvent>(), 16);
    }

    #[test]
    fn event_field_offsets() {
        assert_eq!(core::mem::offset_of!(InotifyEvent, wd), 0);
        assert_eq!(core::mem::offset_of!(InotifyEvent, mask), 4);
        assert_eq!(core::mem::offset_of!(InotifyEvent, cookie), 8);
        assert_eq!(core::mem::offset_of!(InotifyEvent, len), 12);
    }

    #[test]
    fn queue_enforces_limit_and_reports_overflow_once() {
        let mut queue = InotifyQueue::new();
        for idx in 0..(MAX_QUEUED_EVENTS + 3) {
            enqueue_event(&mut queue, idx as i32, IN_CREATE, Some("child"));
        }

        assert_eq!(queue.events.len(), MAX_QUEUED_EVENTS);
        assert!(queue.overflow_queued);
        assert_eq!(
            queue
                .events
                .iter()
                .filter(|event| event_mask(event) & IN_Q_OVERFLOW != 0)
                .count(),
            1
        );
        assert_eq!(
            event_mask(queue.events.back().expect("overflow event")),
            IN_Q_OVERFLOW
        );

        while let Some(event) = queue.pop_front() {
            if event_mask(&event) & IN_Q_OVERFLOW != 0 {
                assert!(!queue.overflow_queued);
                break;
            }
        }
    }

    #[test]
    fn inotify_fd_is_pollable_for_sd_event() {
        // Serializes the global current-task swap against other fs tests.
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 294;
        current.tgid = 294;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let epfd = sys_epoll_create1(0);
            assert!(epfd >= 0);
            let ifd = sys_inotify_init1(IN_NONBLOCK | IN_CLOEXEC);
            assert!(ifd >= 0);
            let ev = EpollEvent {
                events: EPOLLIN,
                data: 0x294,
            };
            assert_eq!(
                sys_epoll_ctl(epfd as i32, EPOLL_CTL_ADD, ifd as i32, &ev),
                0
            );

            let ft = files::get_task_files(&mut *current as *mut TaskStruct).unwrap();
            let file = ft.get(ifd as i32).unwrap();
            let mut buf = [0u8; core::mem::size_of::<InotifyEvent>()];
            assert_eq!(vfs_read(&file, &mut buf), Err(EAGAIN));

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn add_watch_missing_component_returns_enoent_and_parent_create_wakes_reader() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        let (current, previous) = setup_current_with_rootfs(254);
        unsafe {
            assert_eq!(crate::fs::syscalls::sys_mkdir(b"/run\0".as_ptr(), 0o755), 0);

            let ifd = sys_inotify_init1(IN_NONBLOCK | IN_CLOEXEC);
            assert!(ifd >= 0);
            let run_wd = sys_inotify_add_watch(
                ifd as i32,
                b"/run\0".as_ptr() as *const i8,
                IN_CREATE | IN_MOVED_TO,
            );
            assert!(run_wd > 0);

            let missing = sys_inotify_add_watch(
                ifd as i32,
                b"/run/dbus\0".as_ptr() as *const i8,
                IN_CREATE
                    | IN_MOVED_TO
                    | IN_ATTRIB
                    | IN_DELETE_SELF
                    | IN_MOVE_SELF
                    | IN_DONT_FOLLOW,
            );
            assert_eq!(missing, -(ENOENT as i64));

            assert_eq!(
                crate::fs::syscalls::sys_mkdir(b"/run/dbus\0".as_ptr(), 0o755),
                0
            );

            let ft = files::get_task_files(&*current as *const TaskStruct as *mut TaskStruct)
                .expect("task files");
            let file = ft.get(ifd as i32).expect("inotify fd");
            assert_ne!(super::inotify_poll(&file, None) & EPOLLIN, 0);

            let mut buf = [0u8; 64];
            let n = vfs_read(&file, &mut buf).expect("inotify event");
            assert!(n >= core::mem::size_of::<InotifyEvent>());
            assert_eq!(
                i32::from_ne_bytes(buf[0..4].try_into().unwrap()),
                run_wd as i32
            );
            let mask = u32::from_ne_bytes(buf[4..8].try_into().unwrap());
            assert_ne!(mask & IN_CREATE, 0);
            assert_ne!(mask & IN_ISDIR, 0);
            let name_len = u32::from_ne_bytes(buf[12..16].try_into().unwrap()) as usize;
            assert!(name_len >= "dbus".len() + 1);
            assert_eq!(&buf[16..20], b"dbus");

            teardown_current(current, previous);
        }
    }

    #[test]
    fn add_watch_enforces_per_instance_limit() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        let (current, previous) = setup_current_with_rootfs(256);
        unsafe {
            let ifd = sys_inotify_init1(0);
            assert!(ifd >= 0);

            for idx in 0..MAX_USER_WATCHES {
                let path = alloc::format!("/watch{idx}");
                let path_c = alloc::format!("{path}\0");
                assert_eq!(crate::fs::syscalls::sys_mkdir(path_c.as_ptr(), 0o755), 0);
                assert!(
                    sys_inotify_add_watch(ifd as i32, path_c.as_ptr() as *const i8, IN_CREATE) > 0
                );
            }

            assert_eq!(
                crate::fs::syscalls::sys_mkdir(b"/watch-limit\0".as_ptr(), 0o755),
                0
            );
            assert_eq!(
                sys_inotify_add_watch(
                    ifd as i32,
                    b"/watch-limit\0".as_ptr() as *const i8,
                    IN_CREATE,
                ),
                -(ENOSPC as i64)
            );

            teardown_current(current, previous);
        }
    }

    #[test]
    fn add_watch_validates_linux_masks_and_onlydir() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        let (current, previous) = setup_current_with_rootfs(255);
        unsafe {
            assert_eq!(crate::fs::syscalls::sys_mkdir(b"/run\0".as_ptr(), 0o755), 0);
            let fd = crate::fs::syscalls::sys_open(
                b"/run/file\0".as_ptr(),
                crate::include::uapi::fcntl::O_CREAT as i32,
                0o644,
            );
            assert!(fd >= 0);
            let ifd = sys_inotify_init1(0);
            assert!(ifd >= 0);

            assert_eq!(
                sys_inotify_add_watch(ifd as i32, b"/run\0".as_ptr() as *const i8, 0),
                -(EINVAL as i64)
            );
            assert_eq!(
                sys_inotify_add_watch(
                    ifd as i32,
                    b"/run\0".as_ptr() as *const i8,
                    IN_CREATE | IN_MASK_ADD | IN_MASK_CREATE,
                ),
                -(EINVAL as i64)
            );
            assert_eq!(
                sys_inotify_add_watch(
                    ifd as i32,
                    b"/run/file\0".as_ptr() as *const i8,
                    IN_CREATE | IN_ONLYDIR,
                ),
                -(ENOTDIR as i64)
            );
            assert_eq!(
                sys_inotify_add_watch(fd as i32, b"/run\0".as_ptr() as *const i8, IN_CREATE),
                -(EINVAL as i64)
            );

            teardown_current(current, previous);
        }
    }
}
