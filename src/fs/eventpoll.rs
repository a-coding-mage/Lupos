//! linux-parity: partial
//! linux-source: vendor/linux/fs/eventpoll.c
//! test-origin: linux:vendor/linux/fs/eventpoll.c
//! epoll — event multiplexer.
//!
//! ABI parity with vendor/linux/fs/eventpoll.c and uapi/linux/eventpoll.h.
//! The ready-list and persistent poll-wakeup path mirror Linux eventpoll.  The
//! remaining gaps are the bounded reverse-path accounting, EPOLLEXCLUSIVE, and
//! the POLLFREE/RCU teardown protocol used by the fully concurrent C code.

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use lazy_static::lazy_static;

use crate::fs::anon_inode::alloc_anon_file;
use crate::fs::fdtable::FilesStruct;
use crate::fs::file::{fget, fput};
use crate::fs::ops::FileOps;
use crate::fs::select;
use crate::fs::types::FileRef;
use crate::include::uapi::errno::{EBADF, EEXIST, EFAULT, EINTR, EINVAL, ENOENT, EPERM};
use crate::kernel::locking::{Mutex, SpinLock};
use crate::kernel::sched::wait::WaitQueueHead;
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
pub const EPOLLRDNORM: u32 = 0x0040;
pub const EPOLLWRNORM: u32 = 0x0100;
pub const EPOLLWAKEUP: u32 = 1 << 29;
pub const EPOLLEXCLUSIVE: u32 = 1 << 28;
pub const EPOLLET: u32 = 1 << 31;
pub const EPOLLONESHOT: u32 = 1 << 30;

const EP_PRIVATE_BITS: u32 = EPOLLWAKEUP | EPOLLONESHOT | EPOLLET | EPOLLEXCLUSIVE;
const EP_MAX_NESTS: usize = 4;

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
pub struct EpItem {
    id: usize,
    pub fd: i32,
    file: Option<FileRef>,
    events: AtomicU32,
    data: AtomicU64,
    /// Only process-side eventpoll operations take this mutex.  In particular,
    /// `ep_poll_callback()` never touches it from IRQ context.
    poll_table: Mutex<select::PollTable>,
    /// Only used by the one-jiffy compatibility scan for poll implementations
    /// which expose no waitqueue.  Real epoll edges are callback-driven.
    fallback_last_ready: AtomicU32,
    callback_driven: AtomicBool,
}

impl EpItem {
    fn file(&self) -> &FileRef {
        self.file.as_ref().expect("live epitem has a file")
    }

    fn events(&self) -> u32 {
        self.events.load(Ordering::Acquire)
    }

    fn data(&self) -> u64 {
        self.data.load(Ordering::Acquire)
    }
}

impl Drop for EpItem {
    fn drop(&mut self) {
        // Linux unregisters every poll hook before dropping the watched-file
        // relationship.  Keeping the FileRef in an Option lets Drop transfer
        // the actual reference to fput rather than bypassing the release hook.
        self.poll_table.lock().finish();
        if let Some(file) = self.file.take() {
            fput(file);
        }
    }
}

struct EpItemSlot {
    item: Arc<EpItem>,
    queued: bool,
    /// Embedded ready-list links, equivalent to Linux `epitem.rdllink`.
    /// IDs replace C pointers while keeping callback linkage allocation-free.
    ready_prev: Option<usize>,
    ready_next: Option<usize>,
}

struct EventPollState {
    items: Vec<EpItemSlot>,
    ready_head: Option<usize>,
    ready_tail: Option<usize>,
    ready_len: usize,
}

/// In-kernel state for one EventPoll instance.
pub struct EventPoll {
    token: usize,
    /// Linux `eventpoll.mtx`: serializes ADD/MOD/DEL and ready scans while
    /// allowing f_op->poll(), allocation and user access to sleep/fault.
    mtx: Mutex<()>,
    state: SpinLock<EventPollState>,
    /// Linux `eventpoll.wq`: sleepers in epoll_wait().
    wait_queue: WaitQueueHead,
    /// Linux `eventpoll.poll_wait`: poll/select and enclosing epolls.
    poll_wait: WaitQueueHead,
    /// Pins callbacks which already resolved this epoll before release removed
    /// it from EPOLLS.  Release drains these pins in task context so an IRQ-side
    /// Arc drop can never perform final EventPoll/Vec destruction.
    active_callbacks: AtomicUsize,
}

static EPOLL_TOKEN: AtomicUsize = AtomicUsize::new(1);
static EPITEM_TOKEN: AtomicUsize = AtomicUsize::new(1);
static EPOLL_NEST_LOCK: Mutex<()> = Mutex::new(());
static EPOLL_REGISTRY_LOCK: Mutex<()> = Mutex::new(());

struct EpollRegistry {
    entries: Vec<(usize, Arc<EventPoll>)>,
}

impl EpollRegistry {
    const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    fn get(&self, token: &usize) -> Option<&Arc<EventPoll>> {
        let idx = self
            .entries
            .binary_search_by_key(token, |(entry_token, _)| *entry_token)
            .ok()?;
        Some(&self.entries[idx].1)
    }

    fn remove(&mut self, token: &usize) -> Option<Arc<EventPoll>> {
        let idx = self
            .entries
            .binary_search_by_key(token, |(entry_token, _)| *entry_token)
            .ok()?;
        Some(self.entries.remove(idx).1)
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

lazy_static! {
    static ref EPOLLS: SpinLock<EpollRegistry> = SpinLock::new(EpollRegistry::new());
}

/// The callback path resolves its `EventPoll` through this registry, so every
/// registry access uses the same irqsave discipline as Linux's `ep->lock`.
fn with_epolls_irqsave<R>(f: impl FnOnce(&mut EpollRegistry) -> R) -> R {
    let (mut epolls, irqflags) = EPOLLS.lock_irqsave();
    let result = f(&mut epolls);
    SpinLock::unlock_irqrestore(epolls, irqflags);
    result
}

fn insert_epoll_registry(token: usize, ep: Arc<EventPoll>) {
    let _registry = EPOLL_REGISTRY_LOCK.lock();
    let count = with_epolls_irqsave(|epolls| epolls.entries.len());
    let mut replacement = Vec::with_capacity(count.saturating_add(1));
    let old_storage = with_epolls_irqsave(|epolls| {
        debug_assert_eq!(epolls.entries.len(), count);
        replacement.append(&mut epolls.entries);
        replacement.push((token, ep));
        core::mem::replace(&mut epolls.entries, replacement)
    });
    drop(old_storage);
}

fn remove_epoll_registry(token: usize) -> Option<Arc<EventPoll>> {
    let _registry = EPOLL_REGISTRY_LOCK.lock();
    with_epolls_irqsave(|epolls| epolls.remove(&token))
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
        Self::new_with_token(0)
    }

    fn new_with_token(token: usize) -> Self {
        Self {
            token,
            mtx: Mutex::new(()),
            state: SpinLock::new(EventPollState {
                items: Vec::new(),
                ready_head: None,
                ready_tail: None,
                ready_len: 0,
            }),
            wait_queue: WaitQueueHead::new(),
            poll_wait: WaitQueueHead::new(),
            active_callbacks: AtomicUsize::new(0),
        }
    }

    /// `spin_lock_irqsave(&ep->lock, flags)` around item publication and the
    /// ready list.  No allocation, f_op->poll(), user access or sleeping lock
    /// is permitted inside this closure.
    fn with_state_irqsave<R>(&self, f: impl FnOnce(&mut EventPollState) -> R) -> R {
        let (mut state, irqflags) = self.state.lock_irqsave();
        let result = f(&mut state);
        SpinLock::unlock_irqrestore(state, irqflags);
        result
    }

    fn item_index_locked(state: &EventPollState, id: usize) -> Option<usize> {
        // EPITEM_TOKEN is monotonic and process-side insertion appends while
        // removal preserves order, so IRQ callbacks need only O(log n) lookup.
        state
            .items
            .binary_search_by_key(&id, |slot| slot.item.id)
            .ok()
    }

    fn enqueue_locked(state: &mut EventPollState, id: usize) -> bool {
        let Some(idx) = Self::item_index_locked(state, id) else {
            return false;
        };
        if state.items[idx].queued {
            return false;
        }

        let previous = state.ready_tail;
        if let Some(previous_id) = previous {
            let Some(previous_idx) = Self::item_index_locked(state, previous_id) else {
                return false;
            };
            state.items[previous_idx].ready_next = Some(id);
        } else {
            state.ready_head = Some(id);
        }

        state.items[idx].queued = true;
        state.items[idx].ready_prev = previous;
        state.items[idx].ready_next = None;
        state.ready_tail = Some(id);
        state.ready_len += 1;
        true
    }

    fn enqueue_front_locked(state: &mut EventPollState, id: usize) -> bool {
        let Some(idx) = Self::item_index_locked(state, id) else {
            return false;
        };
        if state.items[idx].queued {
            return false;
        }

        let next = state.ready_head;
        if let Some(next_id) = next {
            let Some(next_idx) = Self::item_index_locked(state, next_id) else {
                return false;
            };
            state.items[next_idx].ready_prev = Some(id);
        } else {
            state.ready_tail = Some(id);
        }

        state.items[idx].queued = true;
        state.items[idx].ready_prev = None;
        state.items[idx].ready_next = next;
        state.ready_head = Some(id);
        state.ready_len += 1;
        true
    }

    fn unlink_ready_locked(state: &mut EventPollState, id: usize) -> bool {
        let Some(idx) = Self::item_index_locked(state, id) else {
            return false;
        };
        if !state.items[idx].queued {
            return false;
        }

        let previous = state.items[idx].ready_prev;
        let next = state.items[idx].ready_next;
        if let Some(previous_id) = previous {
            if let Some(previous_idx) = Self::item_index_locked(state, previous_id) {
                state.items[previous_idx].ready_next = next;
            }
        } else {
            state.ready_head = next;
        }
        if let Some(next_id) = next {
            if let Some(next_idx) = Self::item_index_locked(state, next_id) {
                state.items[next_idx].ready_prev = previous;
            }
        } else {
            state.ready_tail = previous;
        }

        state.items[idx].queued = false;
        state.items[idx].ready_prev = None;
        state.items[idx].ready_next = None;
        state.ready_len -= 1;
        true
    }

    fn pop_ready_front_locked(state: &mut EventPollState) -> Option<usize> {
        let id = state.ready_head?;
        Self::unlink_ready_locked(state, id).then_some(id)
    }

    fn wake_ready_waiters(&self) {
        self.wait_queue.wake_up_all();
        self.poll_wait.wake_up_all();
    }

    fn synchronize_callbacks(&self) {
        while self.active_callbacks.load(Ordering::Acquire) != 0 {
            #[cfg(not(test))]
            unsafe {
                sched::schedule_with_irqs_enabled();
            }
            #[cfg(test)]
            core::hint::spin_loop();
        }
    }

    /// Snapshot stable epitem references without allocating under `ep->lock`.
    /// The caller holds `mtx`, so callbacks may alter ready links but no item
    /// can be inserted or removed between the capacity and copy phases.
    fn items_snapshot_locked(&self) -> Vec<Arc<EpItem>> {
        let count = self.with_state_irqsave(|state| state.items.len());
        let mut items = Vec::with_capacity(count);
        self.with_state_irqsave(|state| {
            debug_assert!(state.items.len() <= items.capacity());
            items.extend(state.items.iter().map(|slot| slot.item.clone()));
        });
        items
    }

    /// Publish a fully allocated epitem without growing or freeing a Vec while
    /// interrupts are disabled.  IDs make moving slots transparent to ready
    /// links and callbacks.
    fn insert_item_locked(&self, item: Arc<EpItem>) {
        let count = self.with_state_irqsave(|state| state.items.len());
        let mut replacement = Vec::with_capacity(count.saturating_add(1));
        let old_storage = self.with_state_irqsave(|state| {
            debug_assert_eq!(state.items.len(), count);
            replacement.append(&mut state.items);
            replacement.push(EpItemSlot {
                item,
                queued: false,
                ready_prev: None,
                ready_next: None,
            });
            core::mem::replace(&mut state.items, replacement)
        });
        drop(old_storage);
    }

    /// Re-poll only legacy sources which failed to expose a waitqueue during
    /// ADD.  The one-jiffy fallback in epoll_wait bounds their latency; normal
    /// pipe/socket/nested-epoll readiness never takes this path.
    fn queue_unregistered_ready_locked(&self) {
        let has_fallback = self.with_state_irqsave(|state| {
            state.items.iter().any(|slot| {
                !slot.item.callback_driven.load(Ordering::Acquire)
                    && slot.item.events() & !EP_PRIVATE_BITS != 0
            })
        });
        if !has_fallback {
            return;
        }
        for item in self.items_snapshot_locked() {
            let events = item.events();
            if item.callback_driven.load(Ordering::Acquire) || events & !EP_PRIVATE_BITS == 0 {
                continue;
            }
            let ready = events & select::poll_mask(item.file());
            let previous = item.fallback_last_ready.swap(ready, Ordering::AcqRel);
            let should_queue = if events & EPOLLET != 0 {
                ready & !previous != 0
            } else {
                ready != 0
            };
            if should_queue {
                self.with_state_irqsave(|state| {
                    Self::enqueue_locked(state, item.id);
                });
            }
        }
    }

    pub fn add(&self, fd: i32, file: FileRef, ev: EpollEvent) -> Result<(), i32> {
        let _mtx = self.mtx.lock();
        let exists = self.with_state_irqsave(|state| {
            state
                .items
                .iter()
                .any(|slot| slot.item.fd == fd && Arc::ptr_eq(slot.item.file(), &file))
        });
        if exists {
            return Err(EEXIST);
        }

        let id = EPITEM_TOKEN.fetch_add(1, Ordering::AcqRel);
        let events = ev.events | EPOLLERR | EPOLLHUP;
        let data = ev.data;
        let item = Arc::new(EpItem {
            id,
            fd,
            file: Some(fget(&file)),
            events: AtomicU32::new(events),
            data: AtomicU64::new(data),
            poll_table: Mutex::new(select::PollTable::new_callback(
                id,
                ep_poll_callback,
                self.token,
                id,
            )),
            fallback_last_ready: AtomicU32::new(0),
            callback_driven: AtomicBool::new(false),
        });
        self.insert_item_locked(item.clone());

        // Register before sampling readiness, as ep_insert()/ep_item_poll() do.
        // The item is already published, so a racing callback either queues it
        // or the readiness sample below observes the event.
        let mask = if self.token == 0 {
            select::poll_mask(item.file())
        } else {
            let mut table = item.poll_table.lock();
            let mask = select::poll_mask_with_table(item.file(), Some(&mut table));
            item.callback_driven
                .store(table.has_registrations(), Ordering::Release);
            mask
        };
        let ready = events & mask;
        item.fallback_last_ready.store(ready, Ordering::Release);
        let queued =
            self.with_state_irqsave(|state| ready != 0 && Self::enqueue_locked(state, item.id));
        if queued {
            self.wake_ready_waiters();
        }
        Ok(())
    }

    fn remove_matching(&self, mut matches: impl FnMut(&EpItem) -> bool) -> usize {
        let _mtx = self.mtx.lock();
        let mut count = 0;
        loop {
            let removed = self.with_state_irqsave(|state| {
                let idx = state
                    .items
                    .iter()
                    .position(|slot| matches(slot.item.as_ref()))?;
                let id = state.items[idx].item.id;
                Self::unlink_ready_locked(state, id);
                Some(state.items.remove(idx))
            });
            let Some(removed) = removed else {
                break;
            };
            let item = removed.item;
            // Unhook every persistent callback before dropping the epitem's
            // watched-file reference.  This is Linux ep_unregister_pollwait()
            // followed by the epitem fput ordering.
            item.poll_table.lock().finish();
            drop(item);
            count += 1;
        }
        count
    }

    pub fn del(&self, fd: i32, file: &FileRef) -> Result<(), i32> {
        if self.remove_matching(|item| item.fd == fd && Arc::ptr_eq(item.file(), file)) == 0 {
            return Err(ENOENT);
        }
        Ok(())
    }

    pub fn modify(&self, fd: i32, file: &FileRef, ev: EpollEvent) -> Result<(), i32> {
        let _mtx = self.mtx.lock();
        let item = self.with_state_irqsave(|state| {
            state
                .items
                .iter()
                .find(|slot| slot.item.fd == fd && Arc::ptr_eq(slot.item.file(), file))
                .map(|slot| slot.item.clone())
        });
        let Some(item) = item else {
            return Err(ENOENT);
        };

        let events = ev.events | EPOLLERR | EPOLLHUP;
        item.events.store(events, Ordering::SeqCst);
        item.data.store(ev.data, Ordering::Release);
        item.fallback_last_ready.store(0, Ordering::Release);
        let mask = select::poll_mask(item.file());
        let ready = events & mask;
        item.fallback_last_ready.store(ready, Ordering::Release);
        let queued =
            self.with_state_irqsave(|state| ready != 0 && Self::enqueue_locked(state, item.id));
        if queued {
            self.wake_ready_waiters();
        }
        Ok(())
    }

    pub fn remove_closed_file(&self, fd: i32, file: &FileRef) {
        self.remove_matching(|item| item.fd == fd && Arc::ptr_eq(item.file(), file));
    }

    fn remove_file(&self, file: &FileRef) {
        self.remove_matching(|item| Arc::ptr_eq(item.file(), file));
    }

    pub fn clear(&self) {
        self.remove_matching(|_| true);
    }

    fn collect_ready_with(
        &self,
        maxevents: usize,
        consume: bool,
        mut deliver: impl FnMut(usize, EpollEvent) -> Result<(), i32>,
    ) -> Result<usize, i32> {
        let _mtx = self.mtx.lock();
        self.queue_unregistered_ready_locked();
        let scan_count = self.with_state_irqsave(|state| state.ready_len);
        let mut n = 0usize;

        for _ in 0..scan_count {
            if n >= maxevents {
                break;
            }
            let item = self.with_state_irqsave(|state| {
                let id = Self::pop_ready_front_locked(state)?;
                let idx = Self::item_index_locked(state, id)?;
                Some(state.items[idx].item.clone())
            });
            let Some(item) = item else {
                continue;
            };

            let events = item.events();
            let mask = select::poll_mask(item.file());
            let ready = events & mask;
            if !item.callback_driven.load(Ordering::Acquire) {
                item.fallback_last_ready.store(ready, Ordering::Release);
            }
            if ready == 0 {
                continue;
            }

            let event = EpollEvent {
                events: ready,
                data: item.data(),
            };
            trace_epoll_ready(
                item.fd,
                item.file().fops.name,
                events,
                mask,
                ready,
                event.data,
            );
            if let Err(errno) = deliver(n, event) {
                self.with_state_irqsave(|state| {
                    Self::enqueue_front_locked(state, item.id);
                });
                return if n == 0 { Err(errno) } else { Ok(n) };
            }

            if events & EPOLLONESHOT != 0 {
                item.events.fetch_and(EP_PRIVATE_BITS, Ordering::AcqRel);
            } else if events & EPOLLET == 0 {
                self.with_state_irqsave(|state| {
                    Self::enqueue_locked(state, item.id);
                });
            }
            if consume {
                crate::fs::kernfs::consume_poll_event(item.file());
            }
            n += 1;
        }
        Ok(n)
    }

    /// Collect currently ready events by polling the watched files.
    pub fn wait_ready(&self, _files: &FilesStruct, out: &mut [EpollEvent]) -> Result<usize, i32> {
        self.collect_ready_with(out.len(), true, |idx, event| {
            out[idx] = event;
            Ok(())
        })
    }

    fn wait_ready_user(&self, out: *mut EpollEvent, maxevents: usize) -> Result<usize, i32> {
        self.collect_ready_with(maxevents, true, |idx, event| {
            let offset = idx
                .checked_mul(core::mem::size_of::<EpollEvent>())
                .ok_or(EFAULT)?;
            let destination = out.cast::<u8>().wrapping_add(offset);
            let not_copied = unsafe {
                crate::arch::x86::kernel::uaccess::copy_to_user(
                    destination,
                    (&event as *const EpollEvent).cast::<u8>(),
                    core::mem::size_of::<EpollEvent>(),
                )
            };
            if not_copied == 0 { Ok(()) } else { Err(EFAULT) }
        })
    }

    fn has_ready(&self) -> bool {
        let _mtx = self.mtx.lock();
        self.queue_unregistered_ready_locked();
        let scan_count = self.with_state_irqsave(|state| state.ready_len);
        for _ in 0..scan_count {
            let item = self.with_state_irqsave(|state| {
                let id = Self::pop_ready_front_locked(state)?;
                let idx = Self::item_index_locked(state, id)?;
                Some(state.items[idx].item.clone())
            });
            let Some(item) = item else {
                continue;
            };
            let events = item.events();
            let ready = events & select::poll_mask(item.file());
            if !item.callback_driven.load(Ordering::Acquire) {
                item.fallback_last_ready.store(ready, Ordering::Release);
            }
            if ready != 0 {
                self.with_state_irqsave(|state| {
                    Self::enqueue_front_locked(state, item.id);
                });
                return true;
            }
        }
        false
    }

    fn needs_fallback_scan(&self) -> bool {
        self.with_state_irqsave(|state| {
            state.items.iter().any(|slot| {
                !slot.item.callback_driven.load(Ordering::Acquire)
                    && slot.item.events() & !EP_PRIVATE_BITS != 0
            })
        })
    }

    #[cfg(not(test))]
    fn prepare_to_wait(&self, current: *mut crate::kernel::task::TaskStruct) -> bool {
        // Install first, then test the ready list. A callback before insertion
        // leaves a visible ready item; a callback after insertion either makes
        // that test fail or wakes the installed task. This avoids growing the
        // waitqueue Vec while ep->lock has IRQs disabled.
        unsafe {
            self.wait_queue
                .prepare_to_wait(current, crate::kernel::task::task_state::TASK_INTERRUPTIBLE);
        }
        self.with_state_irqsave(|state| state.ready_head.is_none())
    }
}

/// Persistent equivalent of Linux `ep_poll_callback()`.  Tokens, rather than
/// raw epitem pointers, make callbacks which raced DEL/release safely no-op.
struct ActiveEpCallback {
    ep: Option<Arc<EventPoll>>,
    active_callbacks: *const AtomicUsize,
}

impl Drop for ActiveEpCallback {
    fn drop(&mut self) {
        // Release the callback's Arc while the active pin is still visible.
        // A concurrent epoll_release therefore continues to own/wait on its
        // Arc; only after this drop is incapable of being final do we publish
        // the zero which lets task-side destruction proceed.
        drop(self.ep.take());
        unsafe {
            (*self.active_callbacks).fetch_sub(1, Ordering::Release);
        }
    }
}

fn ep_poll_callback(ep_token: usize, item_id: usize) {
    let Some(active) = with_epolls_irqsave(|epolls| {
        let ep = epolls.get(&ep_token)?.clone();
        ep.active_callbacks.fetch_add(1, Ordering::AcqRel);
        let active_callbacks = &ep.active_callbacks as *const AtomicUsize;
        Some(ActiveEpCallback {
            ep: Some(ep),
            active_callbacks,
        })
    }) else {
        return;
    };
    let ep = active.ep.as_ref().expect("active callback owns epoll");
    let enabled = ep.with_state_irqsave(|state| {
        let Some(idx) = EventPoll::item_index_locked(state, item_id) else {
            return false;
        };
        let item = state.items[idx].item.as_ref();
        if item.events() & !EP_PRIVATE_BITS == 0 {
            return false;
        }
        EventPoll::enqueue_locked(state, item_id);
        true
    });
    if enabled {
        // Linux wakes both epoll_wait sleepers and pollers of this epoll fd,
        // even when the epitem was already linked on the ready list.
        ep.wake_ready_waiters();
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
    let ep = with_epolls_irqsave(|epolls| epolls.get(&token).cloned()).ok_or(EBADF)?;
    Ok((token, ep))
}

fn epoll_from_fd(fd: i32) -> Result<Arc<EventPoll>, i32> {
    let file = current_files()?.get(fd)?;
    epoll_from_file(&file).map(|(_, ep)| ep)
}

/// Snapshot the registry without letting Vec growth invoke GFP_KERNEL while
/// the IRQ-safe registry lock is held.  Creation/release may change its length
/// between passes, in which case the process-side caller simply retries.
fn epolls_snapshot() -> Vec<(usize, Arc<EventPoll>)> {
    loop {
        let count = with_epolls_irqsave(|epolls| epolls.len());
        let mut snapshot = Vec::with_capacity(count);
        let complete = with_epolls_irqsave(|epolls| {
            if epolls.len() > snapshot.capacity() {
                return false;
            }
            snapshot.extend(
                epolls
                    .entries
                    .iter()
                    .map(|(token, ep)| (*token, ep.clone())),
            );
            true
        });
        if complete {
            return snapshot;
        }
    }
}

fn nested_targets_snapshot(ep: &EventPoll) -> Vec<usize> {
    let _mtx = ep.mtx.lock();
    ep.items_snapshot_locked()
        .into_iter()
        .filter(|item| item.file().fops.name == EPOLL_FILE_OPS.name)
        .map(|item| *item.file().private.lock())
        .collect()
}

fn nested_path_valid(
    graph: &BTreeMap<usize, Vec<usize>>,
    token: usize,
    depth: usize,
    visiting: &mut BTreeSet<usize>,
) -> bool {
    if depth > EP_MAX_NESTS || !visiting.insert(token) {
        return false;
    }
    let valid = graph.get(&token).is_none_or(|targets| {
        targets
            .iter()
            .all(|target| nested_path_valid(graph, *target, depth + 1, visiting))
    });
    visiting.remove(&token);
    valid
}

/// Linux serializes full nested checks with epnested_mutex and rejects both
/// loops and paths deeper than EP_MAX_NESTS.  Checking every root also covers
/// an existing outer epoll chain above `source_token`, not only the new edge's
/// downward subtree.
fn nested_graph_accepts(source_token: usize, target_token: usize) -> bool {
    let mut graph = BTreeMap::new();
    for (token, ep) in epolls_snapshot() {
        graph.insert(token, nested_targets_snapshot(&ep));
    }
    graph.entry(source_token).or_default().push(target_token);

    let roots: Vec<_> = graph.keys().copied().collect();
    roots
        .into_iter()
        .all(|root| nested_path_valid(&graph, root, 0, &mut BTreeSet::new()))
}

pub fn notify_fd_closed(file: &FileRef) {
    if file.fops.poll.is_none() {
        return;
    }
    // File::f_count is Lupos' logical Linux file reference count: dup/fork and
    // SCM_RIGHTS use fget(), while FilesStruct::get() Arc clones are temporary
    // Rust lifetime pins and deliberately do not change it. Each epitem owns
    // one artificial fget, and each PollTableEntry owns one more to keep its
    // raw waitqueue pointer alive. Discount exactly those implementation pins;
    // the closing fd's not-yet-fput reference must then be the sole remainder.
    let epolls: Vec<_> = epolls_snapshot().into_iter().map(|(_, ep)| ep).collect();
    let internal_refs = epolls
        .iter()
        .map(|ep| {
            let _mtx = ep.mtx.lock();
            ep.items_snapshot_locked()
                .into_iter()
                .filter(|item| Arc::ptr_eq(item.file(), file))
                .map(|item| 1usize.saturating_add(item.poll_table.lock().registration_count()))
                .sum::<usize>()
        })
        .sum::<usize>();
    if file.f_count.load(Ordering::Acquire) != internal_refs.saturating_add(1) {
        return;
    }

    // Linux eventpoll_release_file() walks file->f_ep and removes this open-file
    // description from every watching epoll, including inherited epoll objects
    // which are not present in the closing task's FilesStruct.
    for ep in epolls {
        ep.remove_file(file);
    }
}

fn epoll_release(file: FileRef) {
    let token = *file.private.lock();
    let ep = remove_epoll_registry(token);
    if let Some(ep) = ep {
        // Removal prevents new callback pins. Drain callbacks which cloned the
        // registry Arc before removal, then tear down poll hooks and storage in
        // this task context (the POLLFREE/RCU lifetime guarantee in Linux).
        ep.synchronize_callbacks();
        ep.clear();
    }
}

fn epoll_poll(file: &FileRef, table: Option<&mut select::PollTable>) -> u32 {
    let token = *file.private.lock();
    let Some(ep) = with_epolls_irqsave(|epolls| epolls.get(&token).cloned()) else {
        return EPOLLERR;
    };
    // Linux ep_eventpoll_poll() registers on poll_wait before checking the
    // ready list, closing the nested-epoll/poll check-to-sleep race.
    select::poll_wait(file, &ep.poll_wait, table);
    if ep.has_ready() {
        EPOLLIN | EPOLLRDNORM
    } else {
        0
    }
}

/// `sys_epoll_create1(flags)` — Linux syscall 291.
pub unsafe fn sys_epoll_create1(flags: i32) -> i64 {
    if flags & !EPOLL_CLOEXEC != 0 {
        return -(EINVAL as i64);
    }
    let token = EPOLL_TOKEN.fetch_add(1, Ordering::AcqRel);
    let ep = Arc::new(EventPoll::new_with_token(token));
    insert_epoll_registry(token, ep);
    let file = alloc_anon_file("eventpoll", &EPOLL_FILE_OPS, token);
    match current_files().and_then(|ft| ft.install(file, flags & EPOLL_CLOEXEC != 0)) {
        Ok(fd) => fd as i64,
        Err(errno) => {
            remove_epoll_registry(token);
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
    // Linux's epnested_mutex covers the complete check-and-insert interval.
    // Without this serialization, concurrent A->B and B->A additions can both
    // pass their independent checks and create a callback-recursion cycle.
    let nest_guard = if op == EPOLL_CTL_ADD && target.fops.name == EPOLL_FILE_OPS.name {
        let guard = EPOLL_NEST_LOCK.lock();
        let (target_token, _) = match epoll_from_file(&target) {
            Ok(ep) => ep,
            Err(errno) => return -(errno as i64),
        };
        if !nested_graph_accepts(ep_token, target_token) {
            return -(EINVAL as i64);
        }
        Some(guard)
    } else {
        None
    };
    let ev = match op {
        EPOLL_CTL_DEL => EpollEvent { events: 0, data: 0 },
        EPOLL_CTL_ADD | EPOLL_CTL_MOD => {
            if event.is_null() {
                return -(EFAULT as i64);
            }
            let mut ev = core::mem::MaybeUninit::<EpollEvent>::uninit();
            let not_copied = unsafe {
                crate::arch::x86::kernel::uaccess::copy_from_user(
                    ev.as_mut_ptr().cast::<u8>(),
                    event.cast::<u8>(),
                    core::mem::size_of::<EpollEvent>(),
                )
            };
            if not_copied != 0 {
                return -(EFAULT as i64);
            }
            unsafe { ev.assume_init() }
        }
        _ => return -(EINVAL as i64),
    };
    trace_epoll_ctl(epfd, op, fd, target.fops.name, ev.events, ev.data);
    let result = match op {
        EPOLL_CTL_ADD => ep.add(fd, target.clone(), ev),
        EPOLL_CTL_DEL => ep.del(fd, &target),
        EPOLL_CTL_MOD => ep.modify(fd, &target, ev),
        _ => Err(EINVAL),
    };
    drop(nest_guard);
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
    let _ = (&epfd, &op, &fd, &file_ops, &events, &data);
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
    let _ = (&fd, &file_ops, &events, &mask, &ready, &data);
}

/// `sys_epoll_wait(epfd, events, maxevents, timeout)` — Linux syscall 232.
pub unsafe fn sys_epoll_wait(
    epfd: i32,
    events: *mut EpollEvent,
    maxevents: i32,
    timeout: i32,
) -> i64 {
    const EP_MAX_EVENTS: usize = i32::MAX as usize / core::mem::size_of::<EpollEvent>();
    if maxevents <= 0 || maxevents as usize > EP_MAX_EVENTS {
        return -(EINVAL as i64);
    }
    if events.is_null() {
        return -(EFAULT as i64);
    }
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
    #[cfg(not(test))]
    let current = unsafe { sched::get_current() };

    loop {
        #[cfg(not(test))]
        let _ = crate::linux_driver_abi::poll_driver_abi_events_for_wait();
        match ep.wait_ready_user(events, maxevents as usize) {
            Ok(n) if n != 0 => {
                return n as i64;
            }
            Ok(_) if timeout == 0 => {
                return 0;
            }
            Ok(_) => {}
            Err(errno) => return -(errno as i64),
        }
        if crate::kernel::signal::current_has_unblocked_pending_signals() {
            // epoll_wait is not restartable. Return through the syscall frame
            // so the EventPoll Arc is dropped before syscall-exit delivers a
            // default-fatal signal.
            return -(EINTR as i64);
        }
        if let Some(deadline_ns) = deadline_ns {
            if crate::kernel::time::ktime_get() >= deadline_ns {
                return 0;
            }
        }

        #[cfg(not(test))]
        {
            wait_state.maintenance();
            let task = current as usize;
            // prepare_to_wait() installs the waiter before the final locked
            // ready-list check, closing the callback/check/schedule race.
            if ep.prepare_to_wait(current) {
                let deadline_timeout = deadline_ns.map(|deadline| {
                    let remaining = deadline.saturating_sub(crate::kernel::time::ktime_get());
                    crate::kernel::time::timeconv::nsecs_to_jiffies64(remaining).max(1)
                });
                let timeout = if ep.needs_fallback_scan() {
                    Some(1)
                } else {
                    deadline_timeout
                };
                if let Some(timeout) = timeout {
                    let wake_at = crate::kernel::time::jiffies::jiffies().saturating_add(timeout);
                    crate::kernel::time::sleep_timeout::arm_wakeup(task, wake_at);
                }
                unsafe {
                    sched::schedule_with_irqs_enabled();
                }
                if timeout.is_some() {
                    crate::kernel::time::sleep_timeout::cancel_wakeup(task);
                }
            }
            unsafe {
                ep.wait_queue.finish_wait(current);
            }
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
    sigmask: *const u8,
    sigsetsize: usize,
) -> i64 {
    let error = unsafe {
        crate::kernel::signal::set_user_sigmask(
            sigmask.cast::<crate::kernel::signal::SigSet>(),
            sigsetsize,
        )
    };
    if error != 0 {
        return error;
    }

    let result = unsafe { sys_epoll_wait(epfd, events, maxevents, timeout) };
    crate::kernel::signal::restore_saved_sigmask_unless(result == -(EINTR as i64));
    result
}

pub unsafe fn sys_epoll_pwait2(
    epfd: i32,
    events: *mut EpollEvent,
    maxevents: i32,
    timeout: *const crate::kernel::time::Timespec64,
    sigmask: *const u8,
    sigsetsize: usize,
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
    let error = unsafe {
        crate::kernel::signal::set_user_sigmask(
            sigmask.cast::<crate::kernel::signal::SigSet>(),
            sigsetsize,
        )
    };
    if error != 0 {
        return error;
    }

    let result = unsafe { sys_epoll_wait(epfd, events, maxevents, timeout_ms) };
    crate::kernel::signal::restore_saved_sigmask_unless(result == -(EINTR as i64));
    result
}

#[cfg(not(test))]
#[derive(Default)]
struct EventWaitState;

#[cfg(not(test))]
impl EventWaitState {
    fn maintenance(&mut self) {
        crate::init::rootfs::drain_console_control_bytes();
        crate::linux_driver_abi::video::fbdev::core::refresh_cursor_blink();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::dcache::d_alloc;
    use crate::fs::file::{alloc_file, fput};
    use crate::fs::ops::FileOps;
    use crate::kernel::{cred::INIT_CRED, files, sched, task::TaskStruct};
    use alloc::boxed::Box;
    use core::sync::atomic::AtomicUsize;

    static READABLE_OPS: FileOps = FileOps {
        name: "epoll-readable",
        read: None,
        write: None,
        llseek: None,
        fsync: None,
        poll: Some(|_, _| EPOLLIN),
        ioctl: None,
        mmap: None,
        release: None,
        readdir: None,
    };

    static RELEASED_WATCHED_FILES: AtomicUsize = AtomicUsize::new(0);

    fn release_watched_file(_file: FileRef) {
        RELEASED_WATCHED_FILES.fetch_add(1, Ordering::AcqRel);
    }

    static RELEASE_COUNT_OPS: FileOps = FileOps {
        name: "epoll-release-count",
        read: None,
        write: None,
        llseek: None,
        fsync: None,
        poll: Some(|_, _| EPOLLIN),
        ioctl: None,
        mmap: None,
        release: Some(release_watched_file),
        readdir: None,
    };

    fn test_poll_mask(file: &FileRef, _table: Option<&mut select::PollTable>) -> u32 {
        *file.private.lock() as u32
    }

    static MASK_OPS: FileOps = FileOps {
        name: "epoll-mask",
        read: None,
        write: None,
        llseek: None,
        fsync: None,
        poll: Some(test_poll_mask),
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
    fn fatal_signal_interrupts_epoll_without_consuming_signal_or_leaking_arcs() {
        let _signal_guard = crate::kernel::signal::SIGNAL_TEST_LOCK.lock();
        crate::kernel::signal::reset_for_tests();
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 31_102;
        current.tgid = 31_102;
        current.cred = &raw const INIT_CRED;

        unsafe {
            let ft = FilesStruct::new();
            let ft_weak = Arc::downgrade(&ft);
            files::set_task_files(&mut *current as *mut TaskStruct, ft.clone());
            sched::set_current(&mut *current as *mut TaskStruct);

            let epfd = sys_epoll_create1(0);
            assert!(epfd >= 0);
            let ep = epoll_from_fd(epfd as i32).expect("created epoll");
            let ep_weak = Arc::downgrade(&ep);
            assert!(ep.wait_queue.is_empty());
            drop(ep);
            drop(ft);

            assert_eq!(
                crate::kernel::signal::send_signal_to_task(
                    &mut *current as *mut TaskStruct,
                    crate::kernel::signal::SIGTERM,
                ),
                0
            );

            let mut out = [EpollEvent { events: 0, data: 0 }; 1];
            assert_eq!(
                sys_epoll_wait(epfd as i32, out.as_mut_ptr(), 1, -1),
                -(EINTR as i64)
            );
            assert_ne!(
                crate::kernel::signal::current_pending_signal_bits()
                    & (1u64 << (crate::kernel::signal::SIGTERM - 1)),
                0,
                "epoll_wait must leave the fatal signal queued for syscall-exit"
            );
            let ep = epoll_from_fd(epfd as i32).expect("epoll survives interrupted wait");
            assert!(
                ep.wait_queue.is_empty(),
                "interrupted epoll_wait left a task wait entry"
            );
            drop(ep);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            assert!(
                ft_weak.upgrade().is_none(),
                "epoll_wait syscall-local files_struct Arc leaked"
            );
            assert!(
                ep_weak.upgrade().is_none(),
                "epoll_wait syscall-local EventPoll Arc leaked"
            );
            sched::set_current(previous);
        }
        crate::kernel::signal::reset_for_tests();
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
    fn del_fputs_watched_file_reference() {
        RELEASED_WATCHED_FILES.store(0, Ordering::Release);
        let ep = EventPoll::new();
        let file = alloc_file(d_alloc("watched-release"), 0, 0, &RELEASE_COUNT_OPS);
        let ev = EpollEvent {
            events: EPOLLIN,
            data: 0x99,
        };

        ep.add(3, file.clone(), ev).unwrap();
        ep.del(3, &file).unwrap();
        assert_eq!(RELEASED_WATCHED_FILES.load(Ordering::Acquire), 0);

        fput(file);
        assert_eq!(RELEASED_WATCHED_FILES.load(Ordering::Acquire), 1);
    }

    #[test]
    fn clear_fputs_last_watched_reference_after_fd_put() {
        RELEASED_WATCHED_FILES.store(0, Ordering::Release);
        let ep = EventPoll::new();
        let file = alloc_file(d_alloc("watched-clear-release"), 0, 0, &RELEASE_COUNT_OPS);
        let file_for_add = file.clone();
        let ev = EpollEvent {
            events: EPOLLIN,
            data: 0x100,
        };

        ep.add(4, file_for_add, ev).unwrap();
        fput(file);
        assert_eq!(RELEASED_WATCHED_FILES.load(Ordering::Acquire), 0);

        ep.clear();
        assert_eq!(RELEASED_WATCHED_FILES.load(Ordering::Acquire), 1);
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
        assert_eq!(ep.with_state_irqsave(|state| state.items.len()), 2);

        ep.remove_closed_file(4, &old_file);
        let (item_count, retained_new_file) = ep.with_state_irqsave(|state| {
            (
                state.items.len(),
                Arc::ptr_eq(state.items[0].item.file(), &new_file),
            )
        });
        assert_eq!(item_count, 1);
        assert!(retained_new_file);
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
        let (events, data) = ep
            .with_state_irqsave(|state| (state.items[0].item.events(), state.items[0].item.data()));
        assert_eq!(events, EPOLLIN | EPOLLOUT | EPOLLERR | EPOLLHUP);
        assert_eq!(data, 2);
    }

    #[test]
    fn oneshot_interest_is_disabled_until_modified() {
        let files = FilesStruct::new();
        let file = alloc_file(d_alloc("oneshot"), 0, 0, &MASK_OPS);
        *file.private.lock() = EPOLLIN as usize;
        let fd = files.install(file.clone(), false).unwrap();
        let ep = EventPoll::new();
        ep.add(
            fd,
            file.clone(),
            EpollEvent {
                events: EPOLLIN | EPOLLONESHOT,
                data: 0x10,
            },
        )
        .unwrap();

        let mut buf = [EpollEvent { events: 0, data: 0 }; 1];
        assert_eq!(ep.wait_ready(&files, &mut buf).unwrap(), 1);
        assert_eq!(ep.wait_ready(&files, &mut buf).unwrap(), 0);

        ep.modify(
            fd,
            &file,
            EpollEvent {
                events: EPOLLIN | EPOLLONESHOT,
                data: 0x11,
            },
        )
        .unwrap();
        assert_eq!(ep.wait_ready(&files, &mut buf).unwrap(), 1);
        let data = buf[0].data;
        assert_eq!(data, 0x11);
    }

    #[test]
    fn edge_triggered_interest_waits_for_new_ready_edge() {
        let files = FilesStruct::new();
        let file = alloc_file(d_alloc("edge"), 0, 0, &MASK_OPS);
        *file.private.lock() = EPOLLIN as usize;
        let fd = files.install(file.clone(), false).unwrap();
        let ep = EventPoll::new();
        ep.add(
            fd,
            file.clone(),
            EpollEvent {
                events: EPOLLIN | EPOLLET,
                data: 0x20,
            },
        )
        .unwrap();

        let mut buf = [EpollEvent { events: 0, data: 0 }; 1];
        assert_eq!(ep.wait_ready(&files, &mut buf).unwrap(), 1);
        assert_eq!(ep.wait_ready(&files, &mut buf).unwrap(), 0);

        *file.private.lock() = 0;
        assert_eq!(ep.wait_ready(&files, &mut buf).unwrap(), 0);
        *file.private.lock() = EPOLLIN as usize;
        assert_eq!(ep.wait_ready(&files, &mut buf).unwrap(), 1);
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
