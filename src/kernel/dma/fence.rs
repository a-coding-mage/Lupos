//! linux-parity: partial
//! linux-source: vendor/linux/drivers/dma-buf/{dma-fence,dma-fence-array,dma-fence-chain,dma-fence-unwrap}.c
//! DMA fence core ABI used by DRM and dma-buf modules.

use core::ffi::{c_char, c_void};
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};

use crate::include::uapi::errno::{EINVAL, ENOENT, ENOMEM};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::mm::page_flags::{__GFP_ZERO, GFP_KERNEL};

const DMA_FENCE_FLAG_INITIALIZED_BIT: usize = 0;
const DMA_FENCE_FLAG_INLINE_LOCK_BIT: usize = 1;
const DMA_FENCE_FLAG_SEQNO64_BIT: usize = 2;
const DMA_FENCE_FLAG_SIGNALED_BIT: usize = 3;
const DMA_FENCE_FLAG_TIMESTAMP_BIT: usize = 4;
const DMA_FENCE_FLAG_ENABLE_SIGNAL_BIT: usize = 5;
const DMA_FENCE_ARRAY_PENDING_ERROR: i32 = 1;

const fn bit(bit: usize) -> usize {
    1usize << bit
}

type FenceNameFn = unsafe extern "C" fn(*mut LinuxDmaFence) -> *const c_char;
type FenceEnableFn = unsafe extern "C" fn(*mut LinuxDmaFence) -> bool;
type FenceSignaledFn = unsafe extern "C" fn(*mut LinuxDmaFence) -> bool;
type FenceWaitFn = unsafe extern "C" fn(*mut LinuxDmaFence, bool, isize) -> isize;
type FenceReleaseFn = unsafe extern "C" fn(*mut LinuxDmaFence);
type FenceDeadlineFn = unsafe extern "C" fn(*mut LinuxDmaFence, i64);
type FenceCallbackFn = unsafe extern "C" fn(*mut LinuxDmaFence, *mut LinuxDmaFenceCb);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxListHead {
    pub next: *mut LinuxListHead,
    pub prev: *mut LinuxListHead,
}

#[repr(C)]
pub struct LinuxKRef {
    pub refcount: AtomicI32,
}

#[repr(C)]
pub struct LinuxDmaFence {
    pub lock: usize,
    pub ops: *const LinuxDmaFenceOps,
    pub cb_list: LinuxListHead,
    pub context: u64,
    pub seqno: u64,
    pub flags: usize,
    pub refcount: LinuxKRef,
    pub error: i32,
}

#[repr(C)]
pub struct LinuxDmaFenceCb {
    pub node: LinuxListHead,
    pub func: Option<FenceCallbackFn>,
}

#[repr(C)]
pub struct LinuxDmaFenceOps {
    pub get_driver_name: Option<FenceNameFn>,
    pub get_timeline_name: Option<FenceNameFn>,
    pub enable_signaling: Option<FenceEnableFn>,
    pub signaled: Option<FenceSignaledFn>,
    pub wait: Option<FenceWaitFn>,
    pub release: Option<FenceReleaseFn>,
    pub set_deadline: Option<FenceDeadlineFn>,
}

#[repr(C)]
pub struct LinuxDmaFenceChain {
    pub base: LinuxDmaFence,
    pub prev: *mut LinuxDmaFence,
    pub prev_seqno: u64,
    pub fence: *mut LinuxDmaFence,
    pub cb_or_work: [usize; 4],
}

#[repr(C)]
pub struct LinuxCallSingleNode {
    pub llist_next: *mut c_void,
    pub flags: u32,
    pub src: u16,
    pub dst: u16,
}

#[repr(C)]
pub struct LinuxRcuWait {
    pub task: *mut c_void,
}

#[repr(C)]
pub struct LinuxIrqWork {
    pub node: LinuxCallSingleNode,
    pub func: Option<unsafe extern "C" fn(*mut LinuxIrqWork)>,
    pub irqwait: LinuxRcuWait,
}

#[repr(C)]
pub struct LinuxDmaFenceArray {
    pub base: LinuxDmaFence,
    pub num_fences: u32,
    pub num_pending: AtomicI32,
    pub fences: *mut *mut LinuxDmaFence,
    pub work: LinuxIrqWork,
}

#[repr(C)]
pub struct LinuxDmaFenceArrayCb {
    pub cb: LinuxDmaFenceCb,
    pub array: *mut LinuxDmaFenceArray,
}

#[repr(C)]
pub struct LinuxDmaFenceUnwrap {
    pub chain: *mut LinuxDmaFence,
    pub array: *mut LinuxDmaFence,
    pub index: u32,
}

static DMA_FENCE_CONTEXT_COUNTER: AtomicU64 = AtomicU64::new(1);
static DMA_FENCE_STUB_READY: AtomicBool = AtomicBool::new(false);

static DMA_FENCE_STUB_DRIVER: &[u8] = b"stub\0";
static DMA_FENCE_ARRAY_DRIVER: &[u8] = b"dma_fence_array\0";
static DMA_FENCE_CHAIN_DRIVER: &[u8] = b"dma_fence_chain\0";
static DMA_FENCE_UNBOUND_TIMELINE: &[u8] = b"unbound\0";
static DMA_FENCE_DETACHED_DRIVER: &[u8] = b"detached-driver\0";
static DMA_FENCE_SIGNALED_TIMELINE: &[u8] = b"signaled-timeline\0";

static LINUX_DMA_FENCE_STUB_OPS: LinuxDmaFenceOps = LinuxDmaFenceOps {
    get_driver_name: Some(stub_fence_name),
    get_timeline_name: Some(stub_fence_name),
    enable_signaling: None,
    signaled: Some(always_signaled),
    wait: None,
    release: None,
    set_deadline: None,
};

static LINUX_DMA_FENCE_ARRAY_OPS: LinuxDmaFenceOps = LinuxDmaFenceOps {
    get_driver_name: Some(array_driver_name),
    get_timeline_name: Some(array_timeline_name),
    enable_signaling: Some(array_enable_signaling),
    signaled: Some(array_signaled),
    wait: None,
    release: Some(array_release),
    set_deadline: Some(array_set_deadline),
};

static LINUX_DMA_FENCE_CHAIN_OPS: LinuxDmaFenceOps = LinuxDmaFenceOps {
    get_driver_name: Some(chain_driver_name),
    get_timeline_name: Some(chain_timeline_name),
    enable_signaling: Some(chain_enable_signaling),
    signaled: Some(chain_signaled),
    wait: None,
    release: Some(chain_release),
    set_deadline: Some(chain_set_deadline),
};

static mut LINUX_DMA_FENCE_STUB: LinuxDmaFence = LinuxDmaFence {
    lock: 0,
    ops: core::ptr::null(),
    cb_list: LinuxListHead {
        next: core::ptr::null_mut(),
        prev: core::ptr::null_mut(),
    },
    context: 0,
    seqno: 0,
    flags: 0,
    refcount: LinuxKRef {
        refcount: AtomicI32::new(0),
    },
    error: 0,
};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("dma_fence_get_stub", dma_fence_get_stub as usize, false);
    export_symbol_once(
        "dma_fence_allocate_private_stub",
        dma_fence_allocate_private_stub as usize,
        false,
    );
    export_symbol_once(
        "dma_fence_context_alloc",
        dma_fence_context_alloc as usize,
        false,
    );
    export_symbol_once(
        "dma_fence_match_context",
        dma_fence_match_context as usize,
        false,
    );
    export_symbol_once(
        "dma_fence_signal_timestamp_locked",
        dma_fence_signal_timestamp_locked as usize,
        false,
    );
    export_symbol_once(
        "dma_fence_signal_timestamp",
        dma_fence_signal_timestamp as usize,
        false,
    );
    export_symbol_once(
        "dma_fence_signal_locked",
        dma_fence_signal_locked as usize,
        false,
    );
    export_symbol_once(
        "dma_fence_check_and_signal_locked",
        dma_fence_check_and_signal_locked as usize,
        false,
    );
    export_symbol_once(
        "dma_fence_check_and_signal",
        dma_fence_check_and_signal as usize,
        false,
    );
    export_symbol_once("dma_fence_signal", dma_fence_signal as usize, false);
    export_symbol_once(
        "dma_fence_wait_timeout",
        dma_fence_wait_timeout as usize,
        false,
    );
    export_symbol_once("dma_fence_release", dma_fence_release as usize, false);
    export_symbol_once("dma_fence_free", dma_fence_free as usize, false);
    export_symbol_once(
        "dma_fence_enable_sw_signaling",
        dma_fence_enable_sw_signaling as usize,
        false,
    );
    export_symbol_once(
        "dma_fence_add_callback",
        dma_fence_add_callback as usize,
        false,
    );
    export_symbol_once("dma_fence_get_status", dma_fence_get_status as usize, false);
    export_symbol_once(
        "dma_fence_remove_callback",
        dma_fence_remove_callback as usize,
        false,
    );
    export_symbol_once(
        "dma_fence_default_wait",
        dma_fence_default_wait as usize,
        false,
    );
    export_symbol_once(
        "dma_fence_wait_any_timeout",
        dma_fence_wait_any_timeout as usize,
        false,
    );
    export_symbol_once(
        "dma_fence_set_deadline",
        dma_fence_set_deadline as usize,
        false,
    );
    export_symbol_once("dma_fence_describe", dma_fence_describe as usize, false);
    export_symbol_once("dma_fence_init", dma_fence_init as usize, false);
    export_symbol_once("dma_fence_init64", dma_fence_init64 as usize, false);
    export_symbol_once(
        "dma_fence_driver_name",
        dma_fence_driver_name as usize,
        false,
    );
    export_symbol_once(
        "dma_fence_timeline_name",
        dma_fence_timeline_name as usize,
        false,
    );
    export_symbol_once(
        "dma_fence_array_alloc",
        dma_fence_array_alloc as usize,
        false,
    );
    export_symbol_once("dma_fence_array_init", dma_fence_array_init as usize, false);
    export_symbol_once(
        "dma_fence_array_create",
        dma_fence_array_create as usize,
        false,
    );
    export_symbol_once(
        "dma_fence_array_first",
        dma_fence_array_first as usize,
        false,
    );
    export_symbol_once("dma_fence_array_next", dma_fence_array_next as usize, false);
    export_symbol_once(
        "dma_fence_array_ops",
        core::ptr::addr_of!(LINUX_DMA_FENCE_ARRAY_OPS) as usize,
        false,
    );
    export_symbol_once("dma_fence_chain_walk", dma_fence_chain_walk as usize, false);
    export_symbol_once(
        "dma_fence_chain_find_seqno",
        dma_fence_chain_find_seqno as usize,
        false,
    );
    export_symbol_once("dma_fence_chain_init", dma_fence_chain_init as usize, false);
    export_symbol_once(
        "dma_fence_chain_ops",
        core::ptr::addr_of!(LINUX_DMA_FENCE_CHAIN_OPS) as usize,
        false,
    );
    export_symbol_once(
        "dma_fence_unwrap_first",
        dma_fence_unwrap_first as usize,
        true,
    );
    export_symbol_once(
        "dma_fence_unwrap_next",
        dma_fence_unwrap_next as usize,
        true,
    );
    export_symbol_once(
        "__dma_fence_unwrap_merge",
        __dma_fence_unwrap_merge as usize,
        true,
    );
}

fn current_ktime() -> i64 {
    crate::kernel::time::ktime_get().min(i64::MAX as u64) as i64
}

unsafe fn init_list_head(head: *mut LinuxListHead) {
    unsafe {
        (*head).next = head;
        (*head).prev = head;
    }
}

unsafe fn list_empty(head: *const LinuxListHead) -> bool {
    unsafe { (*head).next == head.cast_mut() }
}

unsafe fn list_add_tail(node: *mut LinuxListHead, head: *mut LinuxListHead) {
    unsafe {
        let prev = (*head).prev;
        (*node).next = head;
        (*node).prev = prev;
        (*prev).next = node;
        (*head).prev = node;
    }
}

unsafe fn list_del_init(node: *mut LinuxListHead) {
    unsafe {
        let next = (*node).next;
        let prev = (*node).prev;
        if !next.is_null() && !prev.is_null() {
            (*prev).next = next;
            (*next).prev = prev;
        }
        init_list_head(node);
    }
}

unsafe fn fence_set_flag(fence: *mut LinuxDmaFence, bitnr: usize) {
    unsafe {
        (*fence).flags |= bit(bitnr);
    }
}

unsafe fn fence_test_flag(fence: *const LinuxDmaFence, bitnr: usize) -> bool {
    unsafe { (*fence).flags & bit(bitnr) != 0 }
}

unsafe fn fence_ref_get(fence: *mut LinuxDmaFence) -> *mut LinuxDmaFence {
    if !fence.is_null() {
        unsafe {
            (*fence).refcount.refcount.fetch_add(1, Ordering::AcqRel);
        }
    }
    fence
}

unsafe fn fence_ref_put(fence: *mut LinuxDmaFence) {
    if fence.is_null() {
        return;
    }

    let old = unsafe { (*fence).refcount.refcount.fetch_sub(1, Ordering::AcqRel) };
    if old == 1 {
        unsafe {
            dma_fence_release(core::ptr::addr_of_mut!((*fence).refcount));
        }
    }
}

unsafe fn fence_init_raw(
    fence: *mut LinuxDmaFence,
    ops: *const LinuxDmaFenceOps,
    lock: *mut c_void,
    context: u64,
    seqno: u64,
    flags: usize,
) {
    if fence.is_null() || ops.is_null() {
        return;
    }

    unsafe {
        (*fence).refcount.refcount.store(1, Ordering::Release);
        (*fence).ops = ops;
        init_list_head(core::ptr::addr_of_mut!((*fence).cb_list));
        (*fence).context = context;
        (*fence).seqno = seqno;
        (*fence).flags = flags | bit(DMA_FENCE_FLAG_INITIALIZED_BIT);
        (*fence).lock = lock as usize;
        if lock.is_null() {
            (*fence).lock = 0;
            (*fence).flags |= bit(DMA_FENCE_FLAG_INLINE_LOCK_BIT);
        }
        (*fence).error = 0;
    }
}

fn ensure_stub_initialized() -> *mut LinuxDmaFence {
    let stub = core::ptr::addr_of_mut!(LINUX_DMA_FENCE_STUB);
    if DMA_FENCE_STUB_READY
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        unsafe {
            fence_init_raw(
                stub,
                core::ptr::addr_of!(LINUX_DMA_FENCE_STUB_OPS),
                core::ptr::null_mut(),
                0,
                0,
                0,
            );
            fence_set_flag(stub, DMA_FENCE_FLAG_ENABLE_SIGNAL_BIT);
            dma_fence_signal_timestamp(stub, current_ktime());
            (*stub).refcount.refcount.store(1, Ordering::Release);
        }
    }
    stub
}

unsafe extern "C" fn stub_fence_name(_fence: *mut LinuxDmaFence) -> *const c_char {
    DMA_FENCE_STUB_DRIVER.as_ptr().cast()
}

unsafe extern "C" fn array_driver_name(_fence: *mut LinuxDmaFence) -> *const c_char {
    DMA_FENCE_ARRAY_DRIVER.as_ptr().cast()
}

unsafe extern "C" fn array_timeline_name(_fence: *mut LinuxDmaFence) -> *const c_char {
    DMA_FENCE_UNBOUND_TIMELINE.as_ptr().cast()
}

unsafe extern "C" fn chain_driver_name(_fence: *mut LinuxDmaFence) -> *const c_char {
    DMA_FENCE_CHAIN_DRIVER.as_ptr().cast()
}

unsafe extern "C" fn chain_timeline_name(_fence: *mut LinuxDmaFence) -> *const c_char {
    DMA_FENCE_UNBOUND_TIMELINE.as_ptr().cast()
}

unsafe extern "C" fn always_signaled(_fence: *mut LinuxDmaFence) -> bool {
    true
}

unsafe fn to_array(fence: *mut LinuxDmaFence) -> *mut LinuxDmaFenceArray {
    if fence.is_null() {
        return core::ptr::null_mut();
    }
    let is_array = unsafe { (*fence).ops == core::ptr::addr_of!(LINUX_DMA_FENCE_ARRAY_OPS) };
    if is_array {
        fence.cast::<LinuxDmaFenceArray>()
    } else {
        core::ptr::null_mut()
    }
}

unsafe fn dma_fence_array_callbacks(array: *mut LinuxDmaFenceArray) -> *mut LinuxDmaFenceArrayCb {
    unsafe {
        array
            .cast::<u8>()
            .add(core::mem::size_of::<LinuxDmaFenceArray>())
            .cast::<LinuxDmaFenceArrayCb>()
    }
}

unsafe fn dma_fence_array_set_pending_error(array: *mut LinuxDmaFenceArray, error: i32) {
    if array.is_null() || error == 0 {
        return;
    }

    unsafe {
        if (*array).base.error == DMA_FENCE_ARRAY_PENDING_ERROR {
            (*array).base.error = error;
        }
    }
}

unsafe fn dma_fence_array_clear_pending_error(array: *mut LinuxDmaFenceArray) {
    if array.is_null() {
        return;
    }

    unsafe {
        if (*array).base.error == DMA_FENCE_ARRAY_PENDING_ERROR {
            (*array).base.error = 0;
        }
    }
}

unsafe extern "C" fn irq_dma_fence_array_work(work: *mut LinuxIrqWork) {
    if work.is_null() {
        return;
    }

    let array = (work.cast::<u8>() as usize - core::mem::offset_of!(LinuxDmaFenceArray, work))
        as *mut LinuxDmaFenceArray;
    unsafe {
        dma_fence_array_clear_pending_error(array);
        dma_fence_signal(core::ptr::addr_of_mut!((*array).base));
        fence_ref_put(core::ptr::addr_of_mut!((*array).base));
    }
}

unsafe extern "C" fn dma_fence_array_cb_func(fence: *mut LinuxDmaFence, cb: *mut LinuxDmaFenceCb) {
    if cb.is_null() {
        return;
    }

    let array_cb = cb.cast::<LinuxDmaFenceArrayCb>();
    let array = unsafe { (*array_cb).array };
    if array.is_null() {
        return;
    }

    let error = if fence.is_null() {
        0
    } else {
        unsafe { (*fence).error }
    };
    unsafe {
        dma_fence_array_set_pending_error(array, error);
        let old = (*array).num_pending.fetch_sub(1, Ordering::AcqRel);
        if old == 1 {
            dma_fence_array_clear_pending_error(array);
            dma_fence_signal(core::ptr::addr_of_mut!((*array).base));
        }
        fence_ref_put(core::ptr::addr_of_mut!((*array).base));
    }
}

unsafe extern "C" fn array_enable_signaling(fence: *mut LinuxDmaFence) -> bool {
    let array = unsafe { to_array(fence) };
    if array.is_null() {
        return false;
    }

    let num_fences = unsafe { (*array).num_fences };
    if num_fences == 0 || unsafe { (*array).fences.is_null() } {
        return false;
    }

    let callbacks = unsafe { dma_fence_array_callbacks(array) };
    for i in 0..num_fences {
        let child = unsafe { (*array).fences.add(i as usize).read() };
        let array_cb = unsafe { callbacks.add(i as usize) };

        unsafe {
            (*array_cb).array = array;
            fence_ref_get(core::ptr::addr_of_mut!((*array).base));
        }

        let added = unsafe {
            dma_fence_add_callback(
                child,
                core::ptr::addr_of_mut!((*array_cb).cb),
                Some(dma_fence_array_cb_func),
            )
        };
        if added != 0 {
            let error = if child.is_null() {
                0
            } else {
                unsafe { (*child).error }
            };
            unsafe {
                dma_fence_array_set_pending_error(array, error);
                fence_ref_put(core::ptr::addr_of_mut!((*array).base));
                if (*array).num_pending.fetch_sub(1, Ordering::AcqRel) == 1 {
                    dma_fence_array_clear_pending_error(array);
                    return false;
                }
            }
        }
    }

    true
}

unsafe extern "C" fn array_signaled(fence: *mut LinuxDmaFence) -> bool {
    let array = unsafe { to_array(fence) };
    if array.is_null() {
        return true;
    }

    let mut num_pending = unsafe { (*array).num_pending.load(Ordering::Acquire) };
    if unsafe {
        fence_test_flag(
            core::ptr::addr_of!((*array).base),
            DMA_FENCE_FLAG_ENABLE_SIGNAL_BIT,
        )
    } {
        if num_pending <= 0 {
            unsafe { dma_fence_array_clear_pending_error(array) };
            return true;
        }
        return false;
    }

    let num_fences = unsafe { (*array).num_fences };
    for i in 0..num_fences {
        let child = unsafe { (*array).fences.add(i as usize).read() };
        if unsafe { dma_fence_is_signaled(child) } {
            num_pending -= 1;
            if num_pending <= 0 {
                unsafe { dma_fence_array_clear_pending_error(array) };
                return true;
            }
        }
    }
    false
}

unsafe extern "C" fn array_release(fence: *mut LinuxDmaFence) {
    let array = unsafe { to_array(fence) };
    if array.is_null() {
        unsafe { dma_fence_free(fence) };
        return;
    }

    unsafe {
        for i in 0..(*array).num_fences {
            fence_ref_put((*array).fences.add(i as usize).read());
        }
        crate::mm::slab::linux_kfree((*array).fences.cast::<u8>());
        dma_fence_free(fence);
    }
}

unsafe extern "C" fn array_set_deadline(fence: *mut LinuxDmaFence, deadline: i64) {
    let array = unsafe { to_array(fence) };
    if array.is_null() {
        return;
    }

    unsafe {
        for i in 0..(*array).num_fences {
            dma_fence_set_deadline((*array).fences.add(i as usize).read(), deadline);
        }
    }
}

unsafe extern "C" fn chain_enable_signaling(fence: *mut LinuxDmaFence) -> bool {
    !unsafe { chain_signaled(fence) }
}

unsafe extern "C" fn chain_signaled(fence: *mut LinuxDmaFence) -> bool {
    if fence.is_null() {
        return true;
    }

    let chain = fence.cast::<LinuxDmaFenceChain>();
    unsafe {
        let contained = (*chain).fence;
        let prev = (*chain).prev;
        (contained.is_null() || dma_fence_is_signaled(contained))
            && (prev.is_null() || dma_fence_is_signaled(prev))
    }
}

unsafe extern "C" fn chain_release(fence: *mut LinuxDmaFence) {
    if fence.is_null() {
        return;
    }
    let chain = fence.cast::<LinuxDmaFenceChain>();
    unsafe {
        fence_ref_put((*chain).prev);
        fence_ref_put((*chain).fence);
        dma_fence_free(fence);
    }
}

unsafe extern "C" fn chain_set_deadline(fence: *mut LinuxDmaFence, deadline: i64) {
    if fence.is_null() {
        return;
    }
    let chain = fence.cast::<LinuxDmaFenceChain>();
    unsafe {
        if !(*chain).fence.is_null() {
            dma_fence_set_deadline((*chain).fence, deadline);
        }
        if !(*chain).prev.is_null() {
            dma_fence_set_deadline((*chain).prev, deadline);
        }
    }
}

unsafe fn dma_fence_is_signaled(fence: *mut LinuxDmaFence) -> bool {
    if fence.is_null() {
        return true;
    }
    if unsafe { fence_test_flag(fence, DMA_FENCE_FLAG_SIGNALED_BIT) } {
        return true;
    }

    let ops = unsafe { (*fence).ops };
    if ops.is_null() {
        return false;
    }
    if let Some(signaled) = unsafe { (*ops).signaled }
        && unsafe { signaled(fence) }
    {
        unsafe {
            dma_fence_signal(fence);
        }
        return true;
    }
    false
}

#[unsafe(export_name = "dma_fence_get_stub")]
pub unsafe extern "C" fn dma_fence_get_stub() -> *mut LinuxDmaFence {
    let stub = ensure_stub_initialized();
    unsafe { fence_ref_get(stub) }
}

#[unsafe(export_name = "dma_fence_allocate_private_stub")]
pub unsafe extern "C" fn dma_fence_allocate_private_stub(timestamp: i64) -> *mut LinuxDmaFence {
    let ptr = unsafe {
        crate::mm::slab::linux___kmalloc_noprof(
            core::mem::size_of::<LinuxDmaFence>(),
            GFP_KERNEL | __GFP_ZERO,
        )
    }
    .cast::<LinuxDmaFence>();
    if ptr.is_null() {
        return core::ptr::null_mut();
    }

    unsafe {
        fence_init_raw(
            ptr,
            core::ptr::addr_of!(LINUX_DMA_FENCE_STUB_OPS),
            core::ptr::null_mut(),
            0,
            0,
            0,
        );
        fence_set_flag(ptr, DMA_FENCE_FLAG_ENABLE_SIGNAL_BIT);
        dma_fence_signal_timestamp(ptr, timestamp);
    }
    ptr
}

#[unsafe(export_name = "dma_fence_context_alloc")]
pub unsafe extern "C" fn dma_fence_context_alloc(num: u32) -> u64 {
    DMA_FENCE_CONTEXT_COUNTER.fetch_add(u64::from(num.max(1)), Ordering::AcqRel)
}

/// `dma_fence_match_context` - `vendor/linux/drivers/dma-buf/dma-fence-array.c:287`.
#[unsafe(export_name = "dma_fence_match_context")]
pub unsafe extern "C" fn dma_fence_match_context(fence: *mut LinuxDmaFence, context: u64) -> bool {
    if fence.is_null() {
        return false;
    }

    let array = unsafe { to_array(fence) };
    if array.is_null() {
        return unsafe { (*fence).context == context };
    }

    unsafe {
        for i in 0..(*array).num_fences {
            let child = (*array).fences.add(i as usize).read();
            if child.is_null() || (*child).context != context {
                return false;
            }
        }
    }
    true
}

#[unsafe(export_name = "dma_fence_signal_timestamp_locked")]
pub unsafe extern "C" fn dma_fence_signal_timestamp_locked(
    fence: *mut LinuxDmaFence,
    timestamp: i64,
) {
    if fence.is_null() {
        return;
    }
    if unsafe { fence_test_flag(fence, DMA_FENCE_FLAG_SIGNALED_BIT) } {
        return;
    }

    unsafe {
        fence_set_flag(fence, DMA_FENCE_FLAG_SIGNALED_BIT);

        let head = core::ptr::addr_of_mut!((*fence).cb_list);
        let mut node = (*head).next;
        while !node.is_null() && node != head {
            let next = (*node).next;
            list_del_init(node);
            let cb = node.cast::<LinuxDmaFenceCb>();
            if let Some(func) = (*cb).func {
                func(fence, cb);
            }
            node = next;
        }

        head.cast::<i64>().write(timestamp);
        fence_set_flag(fence, DMA_FENCE_FLAG_TIMESTAMP_BIT);
    }
}

#[unsafe(export_name = "dma_fence_signal_timestamp")]
pub unsafe extern "C" fn dma_fence_signal_timestamp(fence: *mut LinuxDmaFence, timestamp: i64) {
    unsafe { dma_fence_signal_timestamp_locked(fence, timestamp) };
}

#[unsafe(export_name = "dma_fence_signal_locked")]
pub unsafe extern "C" fn dma_fence_signal_locked(fence: *mut LinuxDmaFence) {
    unsafe { dma_fence_signal_timestamp_locked(fence, current_ktime()) };
}

#[unsafe(export_name = "dma_fence_check_and_signal_locked")]
pub unsafe extern "C" fn dma_fence_check_and_signal_locked(fence: *mut LinuxDmaFence) -> bool {
    let was_signaled = unsafe { dma_fence_is_signaled(fence) };
    unsafe { dma_fence_signal_locked(fence) };
    was_signaled
}

#[unsafe(export_name = "dma_fence_check_and_signal")]
pub unsafe extern "C" fn dma_fence_check_and_signal(fence: *mut LinuxDmaFence) -> bool {
    unsafe { dma_fence_check_and_signal_locked(fence) }
}

#[unsafe(export_name = "dma_fence_signal")]
pub unsafe extern "C" fn dma_fence_signal(fence: *mut LinuxDmaFence) {
    unsafe { dma_fence_signal_timestamp_locked(fence, current_ktime()) };
}

#[unsafe(export_name = "dma_fence_wait_timeout")]
pub unsafe extern "C" fn dma_fence_wait_timeout(
    fence: *mut LinuxDmaFence,
    _intr: bool,
    timeout: isize,
) -> isize {
    if fence.is_null() || timeout < 0 {
        return -(EINVAL as isize);
    }
    if timeout == 0 {
        return if unsafe { dma_fence_is_signaled(fence) } {
            1
        } else {
            0
        };
    }
    unsafe {
        dma_fence_enable_sw_signaling(fence);
        if !dma_fence_is_signaled(fence) {
            dma_fence_signal(fence);
        }
    }
    timeout.max(1)
}

#[unsafe(export_name = "dma_fence_release")]
pub unsafe extern "C" fn dma_fence_release(kref: *mut LinuxKRef) {
    if kref.is_null() {
        return;
    }

    let fence = (kref.cast::<u8>() as usize - core::mem::offset_of!(LinuxDmaFence, refcount))
        as *mut LinuxDmaFence;
    if fence == core::ptr::addr_of_mut!(LINUX_DMA_FENCE_STUB) {
        unsafe {
            (*fence).refcount.refcount.store(1, Ordering::Release);
        }
        return;
    }

    let ops = unsafe { (*fence).ops };
    if !ops.is_null()
        && let Some(release) = unsafe { (*ops).release }
    {
        unsafe { release(fence) };
    } else {
        unsafe { dma_fence_free(fence) };
    }
}

#[unsafe(export_name = "dma_fence_free")]
pub unsafe extern "C" fn dma_fence_free(fence: *mut LinuxDmaFence) {
    if fence.is_null() || fence == core::ptr::addr_of_mut!(LINUX_DMA_FENCE_STUB) {
        return;
    }
    unsafe { crate::mm::slab::linux_kfree(fence.cast()) };
}

#[unsafe(export_name = "dma_fence_enable_sw_signaling")]
pub unsafe extern "C" fn dma_fence_enable_sw_signaling(fence: *mut LinuxDmaFence) {
    if fence.is_null() || unsafe { fence_test_flag(fence, DMA_FENCE_FLAG_SIGNALED_BIT) } {
        return;
    }

    unsafe {
        let was_enabled = fence_test_flag(fence, DMA_FENCE_FLAG_ENABLE_SIGNAL_BIT);
        fence_set_flag(fence, DMA_FENCE_FLAG_ENABLE_SIGNAL_BIT);
        let ops = (*fence).ops;
        if !was_enabled
            && !ops.is_null()
            && let Some(enable) = (*ops).enable_signaling
            && !enable(fence)
        {
            dma_fence_signal_locked(fence);
        }
    }
}

#[unsafe(export_name = "dma_fence_add_callback")]
pub unsafe extern "C" fn dma_fence_add_callback(
    fence: *mut LinuxDmaFence,
    cb: *mut LinuxDmaFenceCb,
    func: Option<FenceCallbackFn>,
) -> i32 {
    if fence.is_null() || cb.is_null() || func.is_none() {
        return -EINVAL;
    }
    if unsafe { dma_fence_is_signaled(fence) } {
        unsafe { init_list_head(core::ptr::addr_of_mut!((*cb).node)) };
        return -ENOENT;
    }

    unsafe {
        dma_fence_enable_sw_signaling(fence);
        if dma_fence_is_signaled(fence) {
            init_list_head(core::ptr::addr_of_mut!((*cb).node));
            return -ENOENT;
        }
        (*cb).func = func;
        list_add_tail(
            core::ptr::addr_of_mut!((*cb).node),
            core::ptr::addr_of_mut!((*fence).cb_list),
        );
    }
    0
}

#[unsafe(export_name = "dma_fence_get_status")]
pub unsafe extern "C" fn dma_fence_get_status(fence: *mut LinuxDmaFence) -> i32 {
    if fence.is_null() {
        return -EINVAL;
    }
    if unsafe { dma_fence_is_signaled(fence) } {
        let error = unsafe { (*fence).error };
        if error < 0 { error } else { 1 }
    } else {
        0
    }
}

#[unsafe(export_name = "dma_fence_remove_callback")]
pub unsafe extern "C" fn dma_fence_remove_callback(
    _fence: *mut LinuxDmaFence,
    cb: *mut LinuxDmaFenceCb,
) -> bool {
    if cb.is_null() {
        return false;
    }
    unsafe {
        let node = core::ptr::addr_of_mut!((*cb).node);
        if list_empty(node) {
            false
        } else {
            list_del_init(node);
            true
        }
    }
}

#[unsafe(export_name = "dma_fence_default_wait")]
pub unsafe extern "C" fn dma_fence_default_wait(
    fence: *mut LinuxDmaFence,
    intr: bool,
    timeout: isize,
) -> isize {
    unsafe { dma_fence_wait_timeout(fence, intr, timeout) }
}

#[unsafe(export_name = "dma_fence_wait_any_timeout")]
pub unsafe extern "C" fn dma_fence_wait_any_timeout(
    fences: *mut *mut LinuxDmaFence,
    count: u32,
    intr: bool,
    timeout: isize,
    idx: *mut u32,
) -> isize {
    if fences.is_null() || count == 0 || timeout < 0 {
        return -(EINVAL as isize);
    }

    for i in 0..count {
        let fence = unsafe { fences.add(i as usize).read() };
        if unsafe { dma_fence_is_signaled(fence) } {
            if !idx.is_null() {
                unsafe { idx.write(i) };
            }
            return timeout.max(1);
        }
    }

    if timeout == 0 {
        return 0;
    }

    let fence = unsafe { fences.read() };
    unsafe {
        dma_fence_wait_timeout(fence, intr, timeout);
    }
    if !idx.is_null() {
        unsafe { idx.write(0) };
    }
    timeout.max(1)
}

#[unsafe(export_name = "dma_fence_set_deadline")]
pub unsafe extern "C" fn dma_fence_set_deadline(fence: *mut LinuxDmaFence, deadline: i64) {
    if fence.is_null() || unsafe { dma_fence_is_signaled(fence) } {
        return;
    }
    let ops = unsafe { (*fence).ops };
    if !ops.is_null()
        && let Some(set_deadline) = unsafe { (*ops).set_deadline }
    {
        unsafe { set_deadline(fence, deadline) };
    }
}

#[unsafe(export_name = "dma_fence_describe")]
pub unsafe extern "C" fn dma_fence_describe(_fence: *mut LinuxDmaFence, _seq: *mut c_void) {}

#[unsafe(export_name = "dma_fence_init")]
pub unsafe extern "C" fn dma_fence_init(
    fence: *mut LinuxDmaFence,
    ops: *const LinuxDmaFenceOps,
    lock: *mut c_void,
    context: u64,
    seqno: u64,
) {
    unsafe { fence_init_raw(fence, ops, lock, context, seqno, 0) };
}

#[unsafe(export_name = "dma_fence_init64")]
pub unsafe extern "C" fn dma_fence_init64(
    fence: *mut LinuxDmaFence,
    ops: *const LinuxDmaFenceOps,
    lock: *mut c_void,
    context: u64,
    seqno: u64,
) {
    unsafe {
        fence_init_raw(
            fence,
            ops,
            lock,
            context,
            seqno,
            bit(DMA_FENCE_FLAG_SEQNO64_BIT),
        )
    };
}

#[unsafe(export_name = "dma_fence_driver_name")]
pub unsafe extern "C" fn dma_fence_driver_name(fence: *mut LinuxDmaFence) -> *const c_char {
    if fence.is_null() || unsafe { fence_test_flag(fence, DMA_FENCE_FLAG_SIGNALED_BIT) } {
        return DMA_FENCE_DETACHED_DRIVER.as_ptr().cast();
    }

    let ops = unsafe { (*fence).ops };
    if !ops.is_null()
        && let Some(name) = unsafe { (*ops).get_driver_name }
    {
        unsafe { name(fence) }
    } else {
        DMA_FENCE_DETACHED_DRIVER.as_ptr().cast()
    }
}

#[unsafe(export_name = "dma_fence_timeline_name")]
pub unsafe extern "C" fn dma_fence_timeline_name(fence: *mut LinuxDmaFence) -> *const c_char {
    if fence.is_null() || unsafe { fence_test_flag(fence, DMA_FENCE_FLAG_SIGNALED_BIT) } {
        return DMA_FENCE_SIGNALED_TIMELINE.as_ptr().cast();
    }

    let ops = unsafe { (*fence).ops };
    if !ops.is_null()
        && let Some(name) = unsafe { (*ops).get_timeline_name }
    {
        unsafe { name(fence) }
    } else {
        DMA_FENCE_SIGNALED_TIMELINE.as_ptr().cast()
    }
}

/// `dma_fence_array_alloc` - `vendor/linux/drivers/dma-buf/dma-fence-array.c:178`.
#[unsafe(export_name = "dma_fence_array_alloc")]
pub unsafe extern "C" fn dma_fence_array_alloc(num_fences: i32) -> *mut LinuxDmaFenceArray {
    if num_fences <= 0 {
        return core::ptr::null_mut();
    }
    let callbacks = num_fences as usize;
    let Some(callback_bytes) = callbacks.checked_mul(core::mem::size_of::<LinuxDmaFenceArrayCb>())
    else {
        return core::ptr::null_mut();
    };
    let Some(size) = core::mem::size_of::<LinuxDmaFenceArray>().checked_add(callback_bytes) else {
        return core::ptr::null_mut();
    };

    unsafe { crate::mm::slab::linux___kmalloc_noprof(size, GFP_KERNEL | __GFP_ZERO) }
        .cast::<LinuxDmaFenceArray>()
}

/// `dma_fence_array_init` - `vendor/linux/drivers/dma-buf/dma-fence-array.c:197`.
#[unsafe(export_name = "dma_fence_array_init")]
pub unsafe extern "C" fn dma_fence_array_init(
    array: *mut LinuxDmaFenceArray,
    num_fences: i32,
    fences: *mut *mut LinuxDmaFence,
    context: u64,
    seqno: u32,
) {
    if array.is_null() {
        return;
    }

    let num_fences = if num_fences > 0 { num_fences as u32 } else { 0 };
    unsafe {
        (*array).num_fences = num_fences;
        (*array)
            .num_pending
            .store(num_fences as i32, Ordering::Release);
        (*array).fences = fences;
        (*array).work = LinuxIrqWork {
            node: LinuxCallSingleNode {
                llist_next: core::ptr::null_mut(),
                flags: 0,
                src: 0,
                dst: 0,
            },
            func: Some(irq_dma_fence_array_work),
            irqwait: LinuxRcuWait {
                task: core::ptr::null_mut(),
            },
        };
        fence_init_raw(
            core::ptr::addr_of_mut!((*array).base),
            core::ptr::addr_of!(LINUX_DMA_FENCE_ARRAY_OPS),
            core::ptr::null_mut(),
            context,
            u64::from(seqno),
            0,
        );
        (*array).base.error = DMA_FENCE_ARRAY_PENDING_ERROR;
    }
}

/// `dma_fence_array_create` - `vendor/linux/drivers/dma-buf/dma-fence-array.c:262`.
#[unsafe(export_name = "dma_fence_array_create")]
pub unsafe extern "C" fn dma_fence_array_create(
    num_fences: i32,
    fences: *mut *mut LinuxDmaFence,
    context: u64,
    seqno: u32,
) -> *mut LinuxDmaFenceArray {
    if num_fences <= 0 || fences.is_null() {
        return core::ptr::null_mut();
    }

    let array = unsafe { dma_fence_array_alloc(num_fences) };
    if array.is_null() {
        return core::ptr::null_mut();
    }

    unsafe { dma_fence_array_init(array, num_fences, fences, context, seqno) };
    array
}

/// `dma_fence_array_first` - `vendor/linux/drivers/dma-buf/dma-fence-array.c:304`.
#[unsafe(export_name = "dma_fence_array_first")]
pub unsafe extern "C" fn dma_fence_array_first(head: *mut LinuxDmaFence) -> *mut LinuxDmaFence {
    if head.is_null() {
        return core::ptr::null_mut();
    }

    let array = unsafe { to_array(head) };
    if array.is_null() {
        return head;
    }
    if unsafe { (*array).num_fences == 0 || (*array).fences.is_null() } {
        return core::ptr::null_mut();
    }

    unsafe { (*array).fences.read() }
}

/// `dma_fence_array_next` - `vendor/linux/drivers/dma-buf/dma-fence-array.c:322`.
#[unsafe(export_name = "dma_fence_array_next")]
pub unsafe extern "C" fn dma_fence_array_next(
    head: *mut LinuxDmaFence,
    index: u32,
) -> *mut LinuxDmaFence {
    let array = unsafe { to_array(head) };
    if array.is_null()
        || unsafe { (*array).fences.is_null() }
        || index >= unsafe { (*array).num_fences }
    {
        return core::ptr::null_mut();
    }

    unsafe { (*array).fences.add(index as usize).read() }
}

unsafe fn to_chain(fence: *mut LinuxDmaFence) -> *mut LinuxDmaFenceChain {
    if fence.is_null() {
        return core::ptr::null_mut();
    }
    let is_chain = unsafe { (*fence).ops == core::ptr::addr_of!(LINUX_DMA_FENCE_CHAIN_OPS) };
    if is_chain {
        fence.cast::<LinuxDmaFenceChain>()
    } else {
        core::ptr::null_mut()
    }
}

#[unsafe(export_name = "dma_fence_chain_walk")]
pub unsafe extern "C" fn dma_fence_chain_walk(fence: *mut LinuxDmaFence) -> *mut LinuxDmaFence {
    let chain = unsafe { to_chain(fence) };
    if chain.is_null() {
        unsafe { fence_ref_put(fence) };
        return core::ptr::null_mut();
    }

    let prev = unsafe { (*chain).prev };
    if !prev.is_null() {
        unsafe { fence_ref_get(prev) };
    }
    unsafe { fence_ref_put(fence) };
    prev
}

#[unsafe(export_name = "dma_fence_chain_find_seqno")]
pub unsafe extern "C" fn dma_fence_chain_find_seqno(
    pfence: *mut *mut LinuxDmaFence,
    seqno: u64,
) -> i32 {
    if pfence.is_null() || seqno == 0 {
        return 0;
    }
    let fence = unsafe { pfence.read() };
    let chain = unsafe { to_chain(fence) };
    if chain.is_null() || unsafe { (*chain).base.seqno < seqno } {
        return -EINVAL;
    }
    0
}

#[unsafe(export_name = "dma_fence_chain_init")]
pub unsafe extern "C" fn dma_fence_chain_init(
    chain: *mut LinuxDmaFenceChain,
    prev: *mut LinuxDmaFence,
    fence: *mut LinuxDmaFence,
    seqno: u64,
) {
    if chain.is_null() {
        return;
    }

    unsafe {
        (*chain).prev = prev;
        (*chain).fence = fence;
        (*chain).prev_seqno = if !prev.is_null() { (*prev).seqno } else { 0 };
        fence_init_raw(
            core::ptr::addr_of_mut!((*chain).base),
            core::ptr::addr_of!(LINUX_DMA_FENCE_CHAIN_OPS),
            core::ptr::null_mut(),
            if !prev.is_null() {
                (*prev).context
            } else {
                dma_fence_context_alloc(1)
            },
            seqno,
            bit(DMA_FENCE_FLAG_SEQNO64_BIT),
        );
    }
}

/// `dma_fence_unwrap_first` - `vendor/linux/drivers/dma-buf/dma-fence-unwrap.c:30`.
#[unsafe(export_name = "dma_fence_unwrap_first")]
pub unsafe extern "C" fn dma_fence_unwrap_first(
    head: *mut LinuxDmaFence,
    cursor: *mut LinuxDmaFenceUnwrap,
) -> *mut LinuxDmaFence {
    if cursor.is_null() {
        return head;
    }

    unsafe {
        (*cursor).chain = fence_ref_get(head);
        (*cursor).array = head;
        (*cursor).index = 0;
    }
    head
}

/// `dma_fence_unwrap_next` - `vendor/linux/drivers/dma-buf/dma-fence-unwrap.c:47`.
#[unsafe(export_name = "dma_fence_unwrap_next")]
pub unsafe extern "C" fn dma_fence_unwrap_next(
    cursor: *mut LinuxDmaFenceUnwrap,
) -> *mut LinuxDmaFence {
    if cursor.is_null() {
        return core::ptr::null_mut();
    }

    unsafe {
        fence_ref_put((*cursor).chain);
        (*cursor).chain = core::ptr::null_mut();
        (*cursor).array = core::ptr::null_mut();
        (*cursor).index = 0;
    }
    core::ptr::null_mut()
}

#[unsafe(export_name = "__dma_fence_unwrap_merge")]
pub unsafe extern "C" fn __dma_fence_unwrap_merge(
    num_fences: u32,
    fences: *mut *mut LinuxDmaFence,
    _iter: *mut c_void,
) -> *mut LinuxDmaFence {
    if fences.is_null() {
        return unsafe { dma_fence_allocate_private_stub(current_ktime()) };
    }

    let mut unsignaled: *mut LinuxDmaFence = core::ptr::null_mut();
    let mut unsignaled_count = 0u32;
    let mut timestamp = 0i64;

    for i in 0..num_fences {
        let fence = unsafe { fences.add(i as usize).read() };
        if fence.is_null() {
            continue;
        }
        if unsafe { dma_fence_is_signaled(fence) } {
            if unsafe { fence_test_flag(fence, DMA_FENCE_FLAG_TIMESTAMP_BIT) } {
                let fence_ts =
                    unsafe { core::ptr::addr_of!((*fence).cb_list).cast::<i64>().read() };
                timestamp = timestamp.max(fence_ts);
            }
        } else {
            unsignaled = fence;
            unsignaled_count = unsignaled_count.saturating_add(1);
        }
    }

    if unsignaled_count == 1 {
        unsafe { fence_ref_get(unsignaled) }
    } else {
        unsafe { dma_fence_allocate_private_stub(timestamp.max(current_ktime())) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{offset_of, size_of};

    #[test]
    fn dma_fence_layout_matches_configured_vendor_abi() {
        assert_eq!(offset_of!(LinuxDmaFence, ops), 8);
        assert_eq!(offset_of!(LinuxDmaFence, cb_list), 16);
        assert_eq!(offset_of!(LinuxDmaFence, context), 32);
        assert_eq!(offset_of!(LinuxDmaFence, seqno), 40);
        assert_eq!(offset_of!(LinuxDmaFence, flags), 48);
        assert_eq!(offset_of!(LinuxDmaFence, refcount), 56);
        assert_eq!(offset_of!(LinuxDmaFence, error), 60);
        assert_eq!(size_of::<LinuxDmaFence>(), 64);
        assert_eq!(size_of::<LinuxDmaFenceOps>(), 56);
        assert_eq!(size_of::<LinuxDmaFenceCb>(), 24);
        assert_eq!(size_of::<LinuxIrqWork>(), 32);
        assert_eq!(offset_of!(LinuxDmaFenceArray, num_fences), 64);
        assert_eq!(offset_of!(LinuxDmaFenceArray, num_pending), 68);
        assert_eq!(offset_of!(LinuxDmaFenceArray, fences), 72);
        assert_eq!(offset_of!(LinuxDmaFenceArray, work), 80);
        assert_eq!(size_of::<LinuxDmaFenceArray>(), 112);
        assert_eq!(offset_of!(LinuxDmaFenceArrayCb, array), 24);
        assert_eq!(size_of::<LinuxDmaFenceArrayCb>(), 32);
    }

    #[test]
    fn registers_vendor_dma_fence_symbols() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("dma_fence_chain_ops"),
            Some(core::ptr::addr_of!(LINUX_DMA_FENCE_CHAIN_OPS) as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("dma_fence_array_ops"),
            Some(core::ptr::addr_of!(LINUX_DMA_FENCE_ARRAY_OPS) as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("dma_fence_array_create"),
            Some(dma_fence_array_create as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("dma_fence_array_first"),
            Some(dma_fence_array_first as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("dma_fence_array_next"),
            Some(dma_fence_array_next as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("dma_fence_context_alloc"),
            Some(dma_fence_context_alloc as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("__dma_fence_unwrap_merge"),
            Some(__dma_fence_unwrap_merge as usize)
        );
    }

    #[test]
    fn private_stub_is_signaled() {
        let fence = unsafe { dma_fence_allocate_private_stub(123) };
        assert!(!fence.is_null());
        assert!(unsafe { dma_fence_is_signaled(fence) });
        assert_eq!(unsafe { dma_fence_wait_timeout(fence, false, 10) }, 10);
        unsafe { fence_ref_put(fence) };
    }

    #[test]
    fn array_create_iterates_children_and_releases_owned_fences() {
        let fences = unsafe {
            crate::mm::slab::linux___kmalloc_noprof(
                2 * size_of::<*mut LinuxDmaFence>(),
                GFP_KERNEL | __GFP_ZERO,
            )
        }
        .cast::<*mut LinuxDmaFence>();
        assert!(!fences.is_null());

        let f0 = unsafe { dma_fence_allocate_private_stub(1) };
        let f1 = unsafe { dma_fence_allocate_private_stub(2) };
        unsafe {
            fences.write(f0);
            fences.add(1).write(f1);
        }

        let array = unsafe { dma_fence_array_create(2, fences, 55, 7) };
        assert!(!array.is_null());
        let base = unsafe { core::ptr::addr_of_mut!((*array).base) };

        assert_eq!(unsafe { dma_fence_array_first(base) }, f0);
        assert_eq!(unsafe { dma_fence_array_next(base, 1) }, f1);
        assert!(unsafe { dma_fence_array_next(base, 2).is_null() });
        assert!(unsafe { dma_fence_match_context(base, 55) });
        assert!(unsafe { dma_fence_is_signaled(base) });
        unsafe { fence_ref_put(base) };
    }
}
