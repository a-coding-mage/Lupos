//! linux-parity: partial
//! linux-source: vendor/linux/drivers/dma-buf/dma-buf.c
//! DMA-BUF core ABI used by DRM and virtio dma-buf modules.

extern crate alloc;

use alloc::collections::BTreeSet;
use core::ffi::{c_char, c_void};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::{EINVAL, ENODEV, ENOMEM};
use crate::kernel::dma::resv::linux_dma_resv_init;
use crate::kernel::module::{export_symbol, find_symbol};
use crate::mm::page_flags::{__GFP_ZERO, GFP_KERNEL};

const LINUX_DMA_RESV_SIZE: usize = 64;
const LINUX_DMA_BUF_SIZE: usize = 256;
const LINUX_DMA_BUF_SIZE_OFFSET: usize = 0;
const LINUX_DMA_BUF_ATTACHMENTS_OFFSET: usize = 16;
const LINUX_DMA_BUF_OPS_OFFSET: usize = 32;
const LINUX_DMA_BUF_EXP_NAME_OFFSET: usize = 64;
const LINUX_DMA_BUF_OWNER_OFFSET: usize = 88;
const LINUX_DMA_BUF_LIST_NODE_OFFSET: usize = 96;
const LINUX_DMA_BUF_PRIV_OFFSET: usize = 112;
const LINUX_DMA_BUF_RESV_OFFSET: usize = 120;

const LINUX_DMA_BUF_ATTACHMENT_SIZE: usize = 64;
const LINUX_DMA_BUF_ATTACHMENT_DMABUF_OFFSET: usize = 0;
const LINUX_DMA_BUF_ATTACHMENT_DEV_OFFSET: usize = 8;
const LINUX_DMA_BUF_ATTACHMENT_NODE_OFFSET: usize = 16;
const LINUX_DMA_BUF_ATTACHMENT_IMPORTER_OPS_OFFSET: usize = 40;
const LINUX_DMA_BUF_ATTACHMENT_IMPORTER_PRIV_OFFSET: usize = 48;

#[repr(C)]
struct LinuxDmaBufExportInfo {
    exp_name: *const c_char,
    owner: *mut c_void,
    ops: *const LinuxDmaBufOps,
    size: usize,
    flags: i32,
    _pad: i32,
    resv: *mut c_void,
    priv_data: *mut c_void,
}

type DmaBufAttachFn = unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32;
type DmaBufDetachFn = unsafe extern "C" fn(*mut c_void, *mut c_void);
type DmaBufPinFn = unsafe extern "C" fn(*mut c_void) -> i32;
type DmaBufUnpinFn = unsafe extern "C" fn(*mut c_void);
type DmaBufMapFn = unsafe extern "C" fn(*mut c_void, u32) -> *mut c_void;
type DmaBufUnmapFn = unsafe extern "C" fn(*mut c_void, *mut c_void, u32);
type DmaBufReleaseFn = unsafe extern "C" fn(*mut c_void);
type DmaBufCpuAccessFn = unsafe extern "C" fn(*mut c_void, u32) -> i32;
type DmaBufMmapFn = unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32;
type DmaBufVmapFn = unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32;
type DmaBufVunmapFn = unsafe extern "C" fn(*mut c_void, *mut c_void);

#[repr(C)]
struct LinuxDmaBufOps {
    attach: Option<DmaBufAttachFn>,
    detach: Option<DmaBufDetachFn>,
    pin: Option<DmaBufPinFn>,
    unpin: Option<DmaBufUnpinFn>,
    map_dma_buf: Option<DmaBufMapFn>,
    unmap_dma_buf: Option<DmaBufUnmapFn>,
    release: Option<DmaBufReleaseFn>,
    begin_cpu_access: Option<DmaBufCpuAccessFn>,
    end_cpu_access: Option<DmaBufCpuAccessFn>,
    mmap: Option<DmaBufMmapFn>,
    vmap: Option<DmaBufVmapFn>,
    vunmap: Option<DmaBufVunmapFn>,
}

lazy_static! {
    static ref DMA_BUFS: Mutex<BTreeSet<usize>> = Mutex::new(BTreeSet::new());
    static ref DMA_BUF_ATTACHMENTS: Mutex<BTreeSet<usize>> = Mutex::new(BTreeSet::new());
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("dma_buf_export", linux_dma_buf_export as usize, true);
    export_symbol_once("dma_buf_fd", linux_dma_buf_fd as usize, true);
    export_symbol_once("dma_buf_get", linux_dma_buf_get as usize, true);
    export_symbol_once("dma_buf_put", linux_dma_buf_put as usize, true);
    export_symbol_once("dma_buf_attach", linux_dma_buf_attach as usize, true);
    export_symbol_once(
        "dma_buf_dynamic_attach",
        linux_dma_buf_dynamic_attach as usize,
        true,
    );
    export_symbol_once("dma_buf_detach", linux_dma_buf_detach as usize, true);
    export_symbol_once("dma_buf_pin", linux_dma_buf_pin as usize, true);
    export_symbol_once("dma_buf_unpin", linux_dma_buf_unpin as usize, true);
    export_symbol_once(
        "dma_buf_map_attachment",
        linux_dma_buf_map_attachment as usize,
        true,
    );
    export_symbol_once(
        "dma_buf_map_attachment_unlocked",
        linux_dma_buf_map_attachment_unlocked as usize,
        true,
    );
    export_symbol_once(
        "dma_buf_unmap_attachment",
        linux_dma_buf_unmap_attachment as usize,
        true,
    );
    export_symbol_once(
        "dma_buf_unmap_attachment_unlocked",
        linux_dma_buf_unmap_attachment_unlocked as usize,
        true,
    );
    export_symbol_once(
        "dma_buf_begin_cpu_access",
        linux_dma_buf_begin_cpu_access as usize,
        true,
    );
    export_symbol_once(
        "dma_buf_end_cpu_access",
        linux_dma_buf_end_cpu_access as usize,
        true,
    );
    export_symbol_once("dma_buf_mmap", linux_dma_buf_mmap as usize, true);
    export_symbol_once("dma_buf_vmap", linux_dma_buf_vmap as usize, true);
    export_symbol_once(
        "dma_buf_vmap_unlocked",
        linux_dma_buf_vmap_unlocked as usize,
        true,
    );
    export_symbol_once("dma_buf_vunmap", linux_dma_buf_vunmap as usize, true);
    export_symbol_once(
        "dma_buf_vunmap_unlocked",
        linux_dma_buf_vunmap_unlocked as usize,
        true,
    );
    export_symbol_once(
        "dma_buf_invalidate_mappings",
        linux_dma_buf_invalidate_mappings as usize,
        true,
    );
    export_symbol_once(
        "dma_buf_attach_revocable",
        linux_dma_buf_attach_revocable as usize,
        true,
    );
}

fn err_ptr(errno: i32) -> *mut c_void {
    (-(errno as isize)) as *mut c_void
}

unsafe fn kzalloc(size: usize) -> *mut u8 {
    unsafe { crate::mm::slab::kmalloc(size, GFP_KERNEL | __GFP_ZERO) }
}

unsafe fn write_usize(base: *mut c_void, offset: usize, value: usize) {
    unsafe { base.cast::<u8>().add(offset).cast::<usize>().write(value) };
}

unsafe fn read_usize(base: *mut c_void, offset: usize) -> usize {
    unsafe { base.cast::<u8>().add(offset).cast::<usize>().read() }
}

unsafe fn init_list_head(base: *mut c_void, offset: usize) {
    let node = unsafe { base.cast::<u8>().add(offset) } as usize;
    unsafe {
        write_usize(base, offset, node);
        write_usize(base, offset + core::mem::size_of::<usize>(), node);
    }
}

unsafe fn dma_buf_ops(dmabuf: *mut c_void) -> *const LinuxDmaBufOps {
    unsafe { read_usize(dmabuf, LINUX_DMA_BUF_OPS_OFFSET) as *const LinuxDmaBufOps }
}

unsafe fn dma_buf_resv(dmabuf: *mut c_void) -> *mut c_void {
    unsafe { read_usize(dmabuf, LINUX_DMA_BUF_RESV_OFFSET) as *mut c_void }
}

unsafe fn attachment_dmabuf(attach: *mut c_void) -> *mut c_void {
    unsafe { read_usize(attach, LINUX_DMA_BUF_ATTACHMENT_DMABUF_OFFSET) as *mut c_void }
}

unsafe fn alloc_attachment(
    dmabuf: *mut c_void,
    dev: *mut c_void,
    importer_ops: *const c_void,
    importer_priv: *mut c_void,
) -> *mut c_void {
    let attach = unsafe { kzalloc(LINUX_DMA_BUF_ATTACHMENT_SIZE) }.cast::<c_void>();
    if attach.is_null() {
        return err_ptr(ENOMEM);
    }
    unsafe {
        write_usize(
            attach,
            LINUX_DMA_BUF_ATTACHMENT_DMABUF_OFFSET,
            dmabuf as usize,
        );
        write_usize(attach, LINUX_DMA_BUF_ATTACHMENT_DEV_OFFSET, dev as usize);
        init_list_head(attach, LINUX_DMA_BUF_ATTACHMENT_NODE_OFFSET);
        write_usize(
            attach,
            LINUX_DMA_BUF_ATTACHMENT_IMPORTER_OPS_OFFSET,
            importer_ops as usize,
        );
        write_usize(
            attach,
            LINUX_DMA_BUF_ATTACHMENT_IMPORTER_PRIV_OFFSET,
            importer_priv as usize,
        );
    }
    DMA_BUF_ATTACHMENTS.lock().insert(attach as usize);
    attach
}

/// `dma_buf_export` - `vendor/linux/drivers/dma-buf/dma-buf.c:708`.
pub unsafe extern "C" fn linux_dma_buf_export(
    exp_info: *const LinuxDmaBufExportInfo,
) -> *mut c_void {
    if exp_info.is_null() {
        return err_ptr(EINVAL);
    }
    let info = unsafe { &*exp_info };
    if info.priv_data.is_null()
        || info.ops.is_null()
        || unsafe { (*info.ops).map_dma_buf.is_none() }
        || unsafe { (*info.ops).unmap_dma_buf.is_none() }
        || unsafe { (*info.ops).release.is_none() }
    {
        return err_ptr(EINVAL);
    }

    let uses_internal_resv = info.resv.is_null();
    let alloc_size = LINUX_DMA_BUF_SIZE
        + if uses_internal_resv {
            LINUX_DMA_RESV_SIZE
        } else {
            0
        };
    let dmabuf = unsafe { kzalloc(alloc_size) }.cast::<c_void>();
    if dmabuf.is_null() {
        return err_ptr(ENOMEM);
    }

    let resv = if uses_internal_resv {
        unsafe { dmabuf.cast::<u8>().add(LINUX_DMA_BUF_SIZE).cast::<c_void>() }
    } else {
        info.resv
    };

    unsafe {
        write_usize(dmabuf, LINUX_DMA_BUF_SIZE_OFFSET, info.size);
        init_list_head(dmabuf, LINUX_DMA_BUF_ATTACHMENTS_OFFSET);
        write_usize(dmabuf, LINUX_DMA_BUF_OPS_OFFSET, info.ops as usize);
        write_usize(
            dmabuf,
            LINUX_DMA_BUF_EXP_NAME_OFFSET,
            info.exp_name as usize,
        );
        write_usize(dmabuf, LINUX_DMA_BUF_OWNER_OFFSET, info.owner as usize);
        init_list_head(dmabuf, LINUX_DMA_BUF_LIST_NODE_OFFSET);
        write_usize(dmabuf, LINUX_DMA_BUF_PRIV_OFFSET, info.priv_data as usize);
        write_usize(dmabuf, LINUX_DMA_BUF_RESV_OFFSET, resv as usize);
        if uses_internal_resv {
            linux_dma_resv_init(resv);
        }
    }
    DMA_BUFS.lock().insert(dmabuf as usize);
    dmabuf
}

/// `dma_buf_fd` - fd export is not wired to Lupos fd tables yet.
pub unsafe extern "C" fn linux_dma_buf_fd(_dmabuf: *mut c_void, _flags: i32) -> i32 {
    -ENODEV
}

/// `dma_buf_get` - fd import is not wired to Lupos fd tables yet.
pub unsafe extern "C" fn linux_dma_buf_get(_fd: i32) -> *mut c_void {
    err_ptr(ENODEV)
}

/// `dma_buf_put` - `vendor/linux/drivers/dma-buf/dma-buf.c:850`.
pub unsafe extern "C" fn linux_dma_buf_put(dmabuf: *mut c_void) {
    if dmabuf.is_null() {
        return;
    }
    if !DMA_BUFS.lock().remove(&(dmabuf as usize)) {
        return;
    }
    let ops = unsafe { dma_buf_ops(dmabuf) };
    if !ops.is_null() {
        if let Some(release) = unsafe { (*ops).release } {
            unsafe { release(dmabuf) };
        }
    }
    unsafe { crate::mm::slab::kfree(dmabuf.cast()) };
}

pub unsafe extern "C" fn linux_dma_buf_attach(
    dmabuf: *mut c_void,
    dev: *mut c_void,
) -> *mut c_void {
    unsafe { linux_dma_buf_dynamic_attach(dmabuf, dev, core::ptr::null(), core::ptr::null_mut()) }
}

/// `dma_buf_dynamic_attach` - `vendor/linux/drivers/dma-buf/dma-buf.c`.
pub unsafe extern "C" fn linux_dma_buf_dynamic_attach(
    dmabuf: *mut c_void,
    dev: *mut c_void,
    importer_ops: *const c_void,
    importer_priv: *mut c_void,
) -> *mut c_void {
    if dmabuf.is_null() {
        return err_ptr(EINVAL);
    }
    let attach = unsafe { alloc_attachment(dmabuf, dev, importer_ops, importer_priv) };
    if (attach as isize) < 0 {
        return attach;
    }
    let ops = unsafe { dma_buf_ops(dmabuf) };
    if !ops.is_null() {
        if let Some(attach_fn) = unsafe { (*ops).attach } {
            let ret = unsafe { attach_fn(dmabuf, attach) };
            if ret != 0 {
                DMA_BUF_ATTACHMENTS.lock().remove(&(attach as usize));
                unsafe { crate::mm::slab::kfree(attach.cast()) };
                return err_ptr(-ret);
            }
        }
    }
    attach
}

/// `dma_buf_detach` - `vendor/linux/drivers/dma-buf/dma-buf.c`.
pub unsafe extern "C" fn linux_dma_buf_detach(dmabuf: *mut c_void, attach: *mut c_void) {
    if dmabuf.is_null() || attach.is_null() {
        return;
    }
    let ops = unsafe { dma_buf_ops(dmabuf) };
    if !ops.is_null() {
        if let Some(detach) = unsafe { (*ops).detach } {
            unsafe { detach(dmabuf, attach) };
        }
    }
    if DMA_BUF_ATTACHMENTS.lock().remove(&(attach as usize)) {
        unsafe { crate::mm::slab::kfree(attach.cast()) };
    }
}

pub unsafe extern "C" fn linux_dma_buf_pin(attach: *mut c_void) -> i32 {
    if attach.is_null() {
        return -EINVAL;
    }
    let dmabuf = unsafe { attachment_dmabuf(attach) };
    let ops = unsafe { dma_buf_ops(dmabuf) };
    if !ops.is_null() {
        if let Some(pin) = unsafe { (*ops).pin } {
            return unsafe { pin(attach) };
        }
    }
    0
}

pub unsafe extern "C" fn linux_dma_buf_unpin(attach: *mut c_void) {
    if attach.is_null() {
        return;
    }
    let dmabuf = unsafe { attachment_dmabuf(attach) };
    let ops = unsafe { dma_buf_ops(dmabuf) };
    if !ops.is_null() {
        if let Some(unpin) = unsafe { (*ops).unpin } {
            unsafe { unpin(attach) };
        }
    }
}

/// `dma_buf_map_attachment` - `vendor/linux/drivers/dma-buf/dma-buf.c:1168`.
pub unsafe extern "C" fn linux_dma_buf_map_attachment(
    attach: *mut c_void,
    direction: u32,
) -> *mut c_void {
    if attach.is_null() {
        return err_ptr(EINVAL);
    }
    let dmabuf = unsafe { attachment_dmabuf(attach) };
    if dmabuf.is_null() {
        return err_ptr(EINVAL);
    }
    let ops = unsafe { dma_buf_ops(dmabuf) };
    if ops.is_null() {
        return err_ptr(EINVAL);
    }
    let Some(map_dma_buf) = (unsafe { (*ops).map_dma_buf }) else {
        return err_ptr(EINVAL);
    };
    let sgt = unsafe { map_dma_buf(attach, direction) };
    if sgt.is_null() { err_ptr(ENOMEM) } else { sgt }
}

pub unsafe extern "C" fn linux_dma_buf_map_attachment_unlocked(
    attach: *mut c_void,
    direction: u32,
) -> *mut c_void {
    unsafe { linux_dma_buf_map_attachment(attach, direction) }
}

/// `dma_buf_unmap_attachment` - `vendor/linux/drivers/dma-buf/dma-buf.c:1284`.
pub unsafe extern "C" fn linux_dma_buf_unmap_attachment(
    attach: *mut c_void,
    sg_table: *mut c_void,
    direction: u32,
) {
    if attach.is_null() || sg_table.is_null() {
        return;
    }
    let dmabuf = unsafe { attachment_dmabuf(attach) };
    let ops = unsafe { dma_buf_ops(dmabuf) };
    if !ops.is_null() {
        if let Some(unmap) = unsafe { (*ops).unmap_dma_buf } {
            unsafe { unmap(attach, sg_table, direction) };
        }
    }
}

pub unsafe extern "C" fn linux_dma_buf_unmap_attachment_unlocked(
    attach: *mut c_void,
    sg_table: *mut c_void,
    direction: u32,
) {
    unsafe { linux_dma_buf_unmap_attachment(attach, sg_table, direction) };
}

pub unsafe extern "C" fn linux_dma_buf_begin_cpu_access(
    dmabuf: *mut c_void,
    direction: u32,
) -> i32 {
    if dmabuf.is_null() {
        return -EINVAL;
    }
    let ops = unsafe { dma_buf_ops(dmabuf) };
    if !ops.is_null() {
        if let Some(begin) = unsafe { (*ops).begin_cpu_access } {
            return unsafe { begin(dmabuf, direction) };
        }
    }
    0
}

pub unsafe extern "C" fn linux_dma_buf_end_cpu_access(dmabuf: *mut c_void, direction: u32) -> i32 {
    if dmabuf.is_null() {
        return -EINVAL;
    }
    let ops = unsafe { dma_buf_ops(dmabuf) };
    if !ops.is_null() {
        if let Some(end) = unsafe { (*ops).end_cpu_access } {
            return unsafe { end(dmabuf, direction) };
        }
    }
    0
}

pub unsafe extern "C" fn linux_dma_buf_mmap(dmabuf: *mut c_void, vma: *mut c_void) -> i32 {
    if dmabuf.is_null() {
        return -EINVAL;
    }
    let ops = unsafe { dma_buf_ops(dmabuf) };
    if !ops.is_null() {
        if let Some(mmap) = unsafe { (*ops).mmap } {
            return unsafe { mmap(dmabuf, vma) };
        }
    }
    -ENODEV
}

pub unsafe extern "C" fn linux_dma_buf_vmap(dmabuf: *mut c_void, map: *mut c_void) -> i32 {
    if dmabuf.is_null() || map.is_null() {
        return -EINVAL;
    }
    let ops = unsafe { dma_buf_ops(dmabuf) };
    if !ops.is_null() {
        if let Some(vmap) = unsafe { (*ops).vmap } {
            return unsafe { vmap(dmabuf, map) };
        }
    }
    -ENODEV
}

pub unsafe extern "C" fn linux_dma_buf_vmap_unlocked(dmabuf: *mut c_void, map: *mut c_void) -> i32 {
    unsafe { linux_dma_buf_vmap(dmabuf, map) }
}

pub unsafe extern "C" fn linux_dma_buf_vunmap(dmabuf: *mut c_void, map: *mut c_void) {
    if dmabuf.is_null() || map.is_null() {
        return;
    }
    let ops = unsafe { dma_buf_ops(dmabuf) };
    if !ops.is_null() {
        if let Some(vunmap) = unsafe { (*ops).vunmap } {
            unsafe { vunmap(dmabuf, map) };
        }
    }
}

pub unsafe extern "C" fn linux_dma_buf_vunmap_unlocked(dmabuf: *mut c_void, map: *mut c_void) {
    unsafe { linux_dma_buf_vunmap(dmabuf, map) };
}

pub unsafe extern "C" fn linux_dma_buf_invalidate_mappings(_dmabuf: *mut c_void) {}

pub unsafe extern "C" fn linux_dma_buf_attach_revocable(_attach: *mut c_void) -> bool {
    false
}
