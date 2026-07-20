//! linux-parity: partial
//! linux-source: vendor/linux/fs/char_dev.c
//! Character-device number registration for Linux-built modules.

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};
use core::sync::atomic::Ordering;

use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::file::alloc_file;
use crate::fs::ops::FileOps;
use crate::fs::types::{DentryRef, FileRef, InodeKind};
use crate::include::uapi::errno::{EBADF, EBUSY, EINVAL, ENODEV, ENOSYS, ENOTTY, ENXIO};
use crate::include::uapi::fcntl::{O_ACCMODE, O_PATH, O_RDONLY, O_RDWR, O_WRONLY};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::mm::page_flags::{__GFP_ZERO, GFP_KERNEL};

const MINORBITS: u32 = 20;
const MINORMASK: u32 = (1 << MINORBITS) - 1;
const FIRST_DYNAMIC_MAJOR: u32 = 234;
const LAST_DYNAMIC_MAJOR: u32 = 511;

#[derive(Clone)]
struct CharDeviceRegion {
    major: u32,
    baseminor: u32,
    count: u32,
    name: String,
    fops: usize,
}

lazy_static! {
    static ref CHRDEVS: Mutex<Vec<CharDeviceRegion>> = Mutex::new(Vec::new());
    static ref DYNAMIC_CDEVS: Mutex<Vec<usize>> = Mutex::new(Vec::new());
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "alloc_chrdev_region",
        linux_alloc_chrdev_region as usize,
        false,
    );
    export_symbol_once(
        "register_chrdev_region",
        linux_register_chrdev_region as usize,
        false,
    );
    export_symbol_once(
        "unregister_chrdev_region",
        linux_unregister_chrdev_region as usize,
        false,
    );
    export_symbol_once("__register_chrdev", linux___register_chrdev as usize, false);
    export_symbol_once(
        "__unregister_chrdev",
        linux___unregister_chrdev as usize,
        false,
    );
    export_symbol_once("cdev_init", linux_cdev_init as usize, false);
    export_symbol_once("cdev_alloc", linux_cdev_alloc as usize, false);
    export_symbol_once("cdev_add", linux_cdev_add as usize, false);
    export_symbol_once("cdev_del", linux_cdev_del as usize, false);
    export_symbol_once("cdev_set_parent", linux_cdev_set_parent as usize, false);
    export_symbol_once("cdev_device_add", linux_cdev_device_add as usize, false);
    export_symbol_once("cdev_device_del", linux_cdev_device_del as usize, false);
}

pub fn registered_chrdev_fops(major: u32) -> Option<usize> {
    CHRDEVS
        .lock()
        .iter()
        .find(|region| region.major == major)
        .map(|region| region.fops)
}

fn mkdev(major: u32, minor: u32) -> u32 {
    (major << MINORBITS) | (minor & MINORMASK)
}

const LINUX_CDEV_OPS_OFFSET: usize = 72;
const LINUX_CDEV_DEV_OFFSET: usize = 96;
const LINUX_CDEV_COUNT_OFFSET: usize = 100;
const LINUX_CDEV_SIZE: usize = 104;
const LINUX_DEVICE_DEVT_OFFSET: usize = 668;

// Configured vendor ABI.  These values are pinned by
// xtask/vendor_abi_probe/lupos_abi_layout_probe.c.
const LINUX_FILE_F_MODE_OFFSET: usize = 4;
const LINUX_FILE_F_OP_OFFSET: usize = 8;
const LINUX_FILE_F_MAPPING_OFFSET: usize = 16;
const LINUX_FILE_PRIVATE_DATA_OFFSET: usize = 24;
const LINUX_FILE_F_INODE_OFFSET: usize = 32;
const LINUX_FILE_F_FLAGS_OFFSET: usize = 40;
const LINUX_FILE_F_POS_OFFSET: usize = 104;
const LINUX_FILE_SIZE: usize = 176;
const LINUX_INODE_I_MODE_OFFSET: usize = 0;
const LINUX_INODE_I_MAPPING_OFFSET: usize = 48;
const LINUX_INODE_I_RDEV_OFFSET: usize = 76;
const LINUX_INODE_SIZE: usize = 544;
const LINUX_ADDRESS_SPACE_HOST_OFFSET: usize = 0;
const LINUX_ADDRESS_SPACE_SIZE: usize = 152;

const LINUX_FOPS_READ_OFFSET: usize = 24;
const LINUX_FOPS_WRITE_OFFSET: usize = 32;
const LINUX_FOPS_OWNER_OFFSET: usize = 0;
const LINUX_FOPS_POLL_OFFSET: usize = 72;
const LINUX_FOPS_UNLOCKED_IOCTL_OFFSET: usize = 80;
const LINUX_FOPS_MMAP_OFFSET: usize = 96;
const LINUX_FOPS_OPEN_OFFSET: usize = 104;
const LINUX_FOPS_RELEASE_OFFSET: usize = 120;

const LINUX_VMA_VM_START_OFFSET: usize = 0;
const LINUX_VMA_VM_END_OFFSET: usize = 8;
const LINUX_VMA_VM_MM_OFFSET: usize = 16;
const LINUX_VMA_VM_PAGE_PROT_OFFSET: usize = 24;
const LINUX_VMA_VM_FLAGS_OFFSET: usize = 32;
const LINUX_VMA_VM_LOCK_SEQ_OFFSET: usize = 40;
const LINUX_VMA_VM_OPS_OFFSET: usize = 72;
const LINUX_VMA_VM_PGOFF_OFFSET: usize = 80;
const LINUX_VMA_VM_FILE_OFFSET: usize = 88;
const LINUX_VMA_SIZE: usize = 192;

const LINUX_MM_MMAP_LOCK_COUNT_OFFSET: usize = 464;
const LINUX_MM_MM_LOCK_SEQ_OFFSET: usize = 520;
const LINUX_MM_SIZE: usize = 1664;
const LINUX_MMAP_WRITE_SEQUENCE: u32 = 1;

const LINUX_VMOPS_OPEN_OFFSET: usize = 0;
const LINUX_VMOPS_CLOSE_OFFSET: usize = 8;
const LINUX_VMOPS_FAULT_OFFSET: usize = 48;

const LINUX_VMF_VMA_OFFSET: usize = 0;
const LINUX_VMF_GFP_MASK_OFFSET: usize = 8;
const LINUX_VMF_PGOFF_OFFSET: usize = 16;
const LINUX_VMF_ADDRESS_OFFSET: usize = 24;
const LINUX_VMF_REAL_ADDRESS_OFFSET: usize = 32;
const LINUX_VMF_FLAGS_OFFSET: usize = 40;
const LINUX_VMF_PMD_OFFSET: usize = 48;
const LINUX_VMF_PUD_OFFSET: usize = 56;
const LINUX_VMF_ORIG_PTE_OFFSET: usize = 64;
const LINUX_VMF_COW_PAGE_OFFSET: usize = 72;
const LINUX_VMF_PAGE_OFFSET: usize = 80;
const LINUX_VMF_PTE_OFFSET: usize = 88;
const LINUX_VMF_SIZE: usize = 112;

const SNDRV_PCM_MMAP_OFFSET_STATUS_OLD_PGOFF: u64 = 0x80000;
const SNDRV_PCM_MMAP_OFFSET_CONTROL_OLD_PGOFF: u64 = 0x81000;
const SNDRV_PCM_MMAP_OFFSET_STATUS_NEW_PGOFF: u64 = 0x82000;
const SNDRV_PCM_MMAP_OFFSET_CONTROL_NEW_PGOFF: u64 = 0x83000;

const FMODE_OPENED: u32 = 1 << 19;
const FMODE_READ: u32 = 1 << 0;
const FMODE_WRITE: u32 = 1 << 1;
const FMODE_CAN_READ: u32 = 1 << 17;
const FMODE_CAN_WRITE: u32 = 1 << 18;

const LINUX_CHAR_FILE_ALLOCATION_SIZE: usize =
    LINUX_FILE_SIZE + LINUX_INODE_SIZE + LINUX_ADDRESS_SPACE_SIZE;

#[inline]
fn linux_decode_major(encoded: u64) -> u32 {
    ((encoded & 0x000f_ff00) >> 8) as u32
}

#[inline]
fn linux_decode_minor(encoded: u64) -> u32 {
    ((encoded & 0xff) | ((encoded >> 12) & 0x000f_ff00)) as u32
}

#[inline]
fn linux_open_fmode(flags: u32) -> u32 {
    let access = (flags.wrapping_add(1)) & O_ACCMODE;
    let mut mode = access;
    if access & FMODE_READ != 0 {
        mode |= FMODE_CAN_READ;
    }
    if access & FMODE_WRITE != 0 {
        mode |= FMODE_CAN_WRITE;
    }
    mode
}

unsafe fn raw_read_usize(base: *const u8, offset: usize) -> usize {
    unsafe { base.add(offset).cast::<usize>().read() }
}

unsafe fn raw_write_usize(base: *mut u8, offset: usize, value: usize) {
    unsafe { base.add(offset).cast::<usize>().write(value) };
}

unsafe fn raw_write_u32(base: *mut u8, offset: usize, value: u32) {
    unsafe { base.add(offset).cast::<u32>().write(value) };
}

unsafe fn raw_write_u16(base: *mut u8, offset: usize, value: u16) {
    unsafe { base.add(offset).cast::<u16>().write(value) };
}

unsafe fn raw_read_u64(base: *const u8, offset: usize) -> u64 {
    unsafe { base.add(offset).cast::<u64>().read() }
}

unsafe fn raw_write_u64(base: *mut u8, offset: usize, value: u64) {
    unsafe { base.add(offset).cast::<u64>().write(value) };
}

unsafe fn linux_char_raw_file(file: &FileRef) -> Option<*mut u8> {
    if !core::ptr::eq(file.fops, &LINUX_CHAR_FILE_OPS) {
        return None;
    }
    let raw = *file.private.lock() as *mut u8;
    (!raw.is_null()).then_some(raw)
}

unsafe fn linux_char_current_fops(raw_file: *mut u8) -> Option<*const u8> {
    let fops =
        unsafe { raw_read_usize(raw_file.cast_const(), LINUX_FILE_F_OP_OFFSET) } as *const u8;
    (!fops.is_null()).then_some(fops)
}

/// `fops_get()` / `fops_put()` from `vendor/linux/include/linux/fs.h`.
///
/// A successful get pins the module that owns the operations table. This is
/// essential before invoking a top-level character-device `->open()`, because
/// callbacks such as ALSA's `snd_open()` use `replace_fops()`: they drop that
/// top-level reference while installing an already-pinned concrete table.
unsafe fn linux_fops_get(fops: *const u8) -> bool {
    if fops.is_null() {
        return false;
    }
    let owner = unsafe { raw_read_usize(fops, LINUX_FOPS_OWNER_OFFSET) } as *mut u8;
    unsafe { crate::kernel::module::loader::linux_try_module_get(owner) }
}

unsafe fn linux_fops_put(fops: *const u8) {
    if fops.is_null() {
        return;
    }
    let owner = unsafe { raw_read_usize(fops, LINUX_FOPS_OWNER_OFFSET) } as *mut u8;
    unsafe { crate::kernel::module::loader::linux_module_put(owner) };
}

fn linux_char_poll(file: &FileRef, _table: Option<&mut crate::fs::select::PollTable>) -> u32 {
    let Some(raw_file) = (unsafe { linux_char_raw_file(file) }) else {
        return 0;
    };
    let Some(fops) = (unsafe { linux_char_current_fops(raw_file) }) else {
        return 0;
    };
    let poll = unsafe { raw_read_usize(fops, LINUX_FOPS_POLL_OFFSET) };
    if poll == 0 {
        return 0;
    }
    let callback: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u32 =
        unsafe { core::mem::transmute(poll) };
    unsafe { callback(raw_file.cast(), core::ptr::null_mut()) }
}

fn linux_char_ioctl(file: &FileRef, cmd: u32, arg: u64) -> Result<i64, i32> {
    let raw_file = unsafe { linux_char_raw_file(file) }.ok_or(EBADF)?;
    let fops = unsafe { linux_char_current_fops(raw_file) }.ok_or(ENODEV)?;
    let ioctl = unsafe { raw_read_usize(fops, LINUX_FOPS_UNLOCKED_IOCTL_OFFSET) };
    if ioctl == 0 {
        return Err(ENOTTY);
    }
    let callback: unsafe extern "C" fn(*mut c_void, u32, usize) -> isize =
        unsafe { core::mem::transmute(ioctl) };
    let result = unsafe { callback(raw_file.cast(), cmd, arg as usize) };
    if result < 0 {
        Err((-result) as i32)
    } else {
        Ok(result as i64)
    }
}

#[repr(align(16))]
struct LinuxCharVmaBridge {
    raw_vma: [u8; LINUX_VMA_SIZE],
    raw_mm: [u8; LINUX_MM_SIZE],
}

impl LinuxCharVmaBridge {
    fn new() -> Self {
        let mut bridge = Self {
            raw_vma: [0; LINUX_VMA_SIZE],
            raw_mm: [0; LINUX_MM_SIZE],
        };
        unsafe {
            // `do_mmap()` invokes ->mmap with mmap_lock write-held and the
            // current VMA sequence, so expose that exact observable state.
            raw_write_u64(
                bridge.raw_mm.as_mut_ptr(),
                LINUX_MM_MMAP_LOCK_COUNT_OFFSET,
                1,
            );
            raw_write_u32(
                bridge.raw_mm.as_mut_ptr(),
                LINUX_MM_MM_LOCK_SEQ_OFFSET,
                LINUX_MMAP_WRITE_SEQUENCE,
            );
        }
        bridge
    }
}

#[repr(align(16))]
struct LinuxRawVmFault([u8; LINUX_VMF_SIZE]);

unsafe fn linux_char_vma_bridge(
    vma: *mut crate::mm::mm_types::VmAreaStruct,
) -> Option<*const LinuxCharVmaBridge> {
    if vma.is_null() || unsafe { (*vma).vm_private_data } == 0 {
        return None;
    }
    Some(unsafe { (*vma).vm_private_data as *const LinuxCharVmaBridge })
}

unsafe fn linux_char_sync_raw_vma(
    state: *const LinuxCharVmaBridge,
    vma: *mut crate::mm::mm_types::VmAreaStruct,
) -> *mut u8 {
    let raw = unsafe { (*state).raw_vma.as_ptr().cast_mut() };
    unsafe {
        raw_write_u64(raw, LINUX_VMA_VM_START_OFFSET, (*vma).vm_start);
        raw_write_u64(raw, LINUX_VMA_VM_END_OFFSET, (*vma).vm_end);
        // The vendor inline vm_flags_{set,mod} helpers dereference vm_mm to
        // assert mmap_lock ownership and compare the per-VMA lock sequence.
        // Native MmStruct is intentionally not Linux-layout-compatible, so
        // expose the configured vendor layout for this bounded callback.
        raw_write_usize(
            raw,
            LINUX_VMA_VM_MM_OFFSET,
            (*state).raw_mm.as_ptr() as usize,
        );
        raw_write_u64(raw, LINUX_VMA_VM_PAGE_PROT_OFFSET, (*vma).vm_page_prot);
        raw_write_u64(raw, LINUX_VMA_VM_FLAGS_OFFSET, (*vma).vm_flags);
        raw_write_u32(raw, LINUX_VMA_VM_LOCK_SEQ_OFFSET, LINUX_MMAP_WRITE_SEQUENCE);
        raw_write_u64(raw, LINUX_VMA_VM_PGOFF_OFFSET, (*vma).vm_pgoff);
    }
    raw
}

unsafe extern "C" fn linux_char_vma_open(vma: *mut crate::mm::mm_types::VmAreaStruct) {
    let Some(state) = (unsafe { linux_char_vma_bridge(vma) }) else {
        return;
    };
    unsafe {
        Arc::increment_strong_count(state);
    }
    let raw = unsafe { linux_char_sync_raw_vma(state, vma) };
    let vm_ops = unsafe { raw_read_usize(raw.cast_const(), LINUX_VMA_VM_OPS_OFFSET) };
    if vm_ops != 0 {
        let open = unsafe { raw_read_usize(vm_ops as *const u8, LINUX_VMOPS_OPEN_OFFSET) };
        if open != 0 {
            let callback: unsafe extern "C" fn(*mut c_void) = unsafe { core::mem::transmute(open) };
            unsafe { callback(raw.cast()) };
        }
    }
}

unsafe extern "C" fn linux_char_vma_close(vma: *mut crate::mm::mm_types::VmAreaStruct) {
    let Some(state) = (unsafe { linux_char_vma_bridge(vma) }) else {
        return;
    };
    let raw = unsafe { linux_char_sync_raw_vma(state, vma) };
    let vm_ops = unsafe { raw_read_usize(raw.cast_const(), LINUX_VMA_VM_OPS_OFFSET) };
    if vm_ops != 0 {
        let close = unsafe { raw_read_usize(vm_ops as *const u8, LINUX_VMOPS_CLOSE_OFFSET) };
        if close != 0 {
            let callback: unsafe extern "C" fn(*mut c_void) =
                unsafe { core::mem::transmute(close) };
            unsafe { callback(raw.cast()) };
        }
    }
    unsafe {
        (*vma).vm_private_data = 0;
        drop(Arc::from_raw(state));
    }
}

unsafe extern "C" fn linux_char_vma_fault(
    vmf: *mut crate::mm::fault::VmFault,
) -> crate::mm::fault::VmFaultFlags {
    if vmf.is_null() {
        return crate::mm::fault::VM_FAULT_SIGBUS;
    }
    let native_vma = unsafe { (*vmf).vma };
    let Some(state) = (unsafe { linux_char_vma_bridge(native_vma) }) else {
        return crate::mm::fault::VM_FAULT_SIGBUS;
    };
    let raw_vma = unsafe { linux_char_sync_raw_vma(state, native_vma) };
    let vm_ops = unsafe { raw_read_usize(raw_vma.cast_const(), LINUX_VMA_VM_OPS_OFFSET) };
    if vm_ops == 0 {
        return crate::mm::fault::VM_FAULT_SIGBUS;
    }
    let fault = unsafe { raw_read_usize(vm_ops as *const u8, LINUX_VMOPS_FAULT_OFFSET) };
    if fault == 0 {
        return crate::mm::fault::VM_FAULT_SIGBUS;
    }

    let mut raw_vmf = LinuxRawVmFault([0; LINUX_VMF_SIZE]);
    let raw = raw_vmf.0.as_mut_ptr();
    unsafe {
        raw_write_usize(raw, LINUX_VMF_VMA_OFFSET, raw_vma as usize);
        raw_write_u32(raw, LINUX_VMF_GFP_MASK_OFFSET, (*vmf).gfp_mask);
        raw_write_u64(raw, LINUX_VMF_PGOFF_OFFSET, (*vmf).pgoff);
        raw_write_u64(raw, LINUX_VMF_ADDRESS_OFFSET, (*vmf).address);
        raw_write_u64(raw, LINUX_VMF_REAL_ADDRESS_OFFSET, (*vmf).real_address);
        raw_write_u32(raw, LINUX_VMF_FLAGS_OFFSET, (*vmf).flags);
        raw_write_usize(raw, LINUX_VMF_PMD_OFFSET, (*vmf).pmd as usize);
        raw_write_usize(raw, LINUX_VMF_PUD_OFFSET, (*vmf).pud as usize);
        raw_write_u64(raw, LINUX_VMF_ORIG_PTE_OFFSET, (*vmf).orig_pte.0);
        raw_write_usize(raw, LINUX_VMF_COW_PAGE_OFFSET, (*vmf).cow_page as usize);
        raw_write_usize(raw, LINUX_VMF_PAGE_OFFSET, (*vmf).page as usize);
        raw_write_usize(raw, LINUX_VMF_PTE_OFFSET, (*vmf).pte as usize);
    }

    let callback: unsafe extern "C" fn(*mut c_void) -> u32 = unsafe { core::mem::transmute(fault) };
    let result = unsafe { callback(raw.cast()) };
    let page = unsafe { raw_read_usize(raw.cast_const(), LINUX_VMF_PAGE_OFFSET) }
        as *mut crate::mm::page::Page;
    unsafe { crate::mm::fault::finish_linux_module_page_fault(vmf, page, result) }
}

static LINUX_CHAR_VM_OPS: crate::mm::fault::VmOperationsStruct =
    crate::mm::fault::VmOperationsStruct {
        open: Some(linux_char_vma_open),
        close: Some(linux_char_vma_close),
        fault: Some(linux_char_vma_fault),
        map_pages: None,
        pfn_mkwrite: None,
        access: None,
    };

pub fn linux_char_vm_ops_tag() -> usize {
    &LINUX_CHAR_VM_OPS as *const crate::mm::fault::VmOperationsStruct as usize
}

fn linux_char_mmap(file: &FileRef, vma: &mut crate::mm::mm_types::VmAreaStruct) -> Result<(), i32> {
    let raw_file = unsafe { linux_char_raw_file(file) }.ok_or(EBADF)?;
    if !matches!(
        vma.vm_pgoff,
        SNDRV_PCM_MMAP_OFFSET_STATUS_OLD_PGOFF
            | SNDRV_PCM_MMAP_OFFSET_CONTROL_OLD_PGOFF
            | SNDRV_PCM_MMAP_OFFSET_STATUS_NEW_PGOFF
            | SNDRV_PCM_MMAP_OFFSET_CONTROL_NEW_PGOFF
    ) {
        // Audio-buffer mmap remains disabled in the shipped WirePlumber
        // policy. Status/control pages are mandatory even for RW_INTERLEAVED.
        return Err(ENODEV);
    }
    let fops = unsafe { linux_char_current_fops(raw_file) }.ok_or(ENODEV)?;
    let mmap = unsafe { raw_read_usize(fops, LINUX_FOPS_MMAP_OFFSET) };
    if mmap == 0 {
        return Err(ENODEV);
    }

    let state = Arc::new(LinuxCharVmaBridge::new());
    let state_ptr = Arc::as_ptr(&state);
    let raw_vma = unsafe { linux_char_sync_raw_vma(state_ptr, vma) };
    unsafe {
        raw_write_usize(raw_vma, LINUX_VMA_VM_FILE_OFFSET, raw_file as usize);
    }
    let callback: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32 =
        unsafe { core::mem::transmute(mmap) };
    let result = unsafe { callback(raw_file.cast(), raw_vma.cast()) };
    if result < 0 {
        return Err(-result);
    }

    let raw_vm_ops = unsafe { raw_read_usize(raw_vma.cast_const(), LINUX_VMA_VM_OPS_OFFSET) };
    if raw_vm_ops == 0 {
        return Err(ENODEV);
    }
    vma.vm_page_prot = unsafe { raw_read_u64(raw_vma.cast_const(), LINUX_VMA_VM_PAGE_PROT_OFFSET) };
    vma.vm_flags = unsafe { raw_read_u64(raw_vma.cast_const(), LINUX_VMA_VM_FLAGS_OFFSET) };
    // The bridge owns the vendor VMA and translates its fault/open/close
    // callbacks. The native VMA's vm_file continues to own the FileRef.
    vma.vm_ops = linux_char_vm_ops_tag();
    vma.vm_private_data = Arc::into_raw(state) as usize;
    Ok(())
}

fn linux_char_release(file: FileRef) {
    let Some(raw_file) = (unsafe { linux_char_raw_file(&file) }) else {
        return;
    };
    if let Some(fops) = unsafe { linux_char_current_fops(raw_file) } {
        let release = unsafe { raw_read_usize(fops, LINUX_FOPS_RELEASE_OFFSET) };
        if release != 0 {
            let callback: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32 =
                unsafe { core::mem::transmute(release) };
            let raw_inode =
                unsafe { raw_read_usize(raw_file.cast_const(), LINUX_FILE_F_INODE_OFFSET) }
                    as *mut c_void;
            let _ = unsafe { callback(raw_inode, raw_file.cast()) };
        }
        unsafe { linux_fops_put(fops) };
    }
    *file.private.lock() = 0;
    unsafe { crate::mm::slab::kfree(raw_file) };
}

pub static LINUX_CHAR_FILE_OPS: FileOps = FileOps {
    name: "linux-module-chardev",
    read: None,
    write: None,
    llseek: None,
    fsync: None,
    poll: Some(linux_char_poll),
    ioctl: Some(linux_char_ioctl),
    mmap: Some(linux_char_mmap),
    release: Some(linux_char_release),
    readdir: None,
};

/// Open a devtmpfs character node backed by a vendor Linux module.
///
/// `snd_open()` replaces the top-level major fops with the concrete ALSA
/// control/PCM table, exactly as `vendor/linux/fs/char_dev.c::chrdev_open`
/// and `vendor/linux/sound/core/sound.c::snd_open` do.
pub fn open_linux_module_chardev(
    dentry: DentryRef,
    flags: u32,
    mode: u32,
) -> Option<Result<FileRef, i32>> {
    let inode = dentry.inode()?;
    if inode.kind != InodeKind::Chardev {
        return None;
    }
    let encoded = inode.rdev.load(Ordering::Acquire);
    let major = linux_decode_major(encoded);
    let minor = linux_decode_minor(encoded);
    let top_fops = registered_chrdev_fops(major)?;
    if top_fops == 0 {
        return Some(Err(ENODEV));
    }

    Some((|| {
        if flags & O_PATH != 0 {
            return Err(EBADF);
        }
        let raw_file = unsafe {
            crate::mm::slab::kmalloc(LINUX_CHAR_FILE_ALLOCATION_SIZE, GFP_KERNEL | __GFP_ZERO)
        };
        if raw_file.is_null() {
            return Err(crate::include::uapi::errno::ENOMEM);
        }
        let raw_inode = unsafe { raw_file.add(LINUX_FILE_SIZE) };
        let raw_mapping = unsafe { raw_inode.add(LINUX_INODE_SIZE) };
        let internal_devt = mkdev(major, minor);

        unsafe {
            raw_write_u32(raw_file, LINUX_FILE_F_MODE_OFFSET, linux_open_fmode(flags));
            raw_write_usize(raw_file, LINUX_FILE_F_OP_OFFSET, top_fops);
            raw_write_usize(raw_file, LINUX_FILE_F_MAPPING_OFFSET, raw_mapping as usize);
            raw_write_usize(raw_file, LINUX_FILE_F_INODE_OFFSET, raw_inode as usize);
            raw_write_u32(raw_file, LINUX_FILE_F_FLAGS_OFFSET, flags);
            raw_write_u16(
                raw_inode,
                LINUX_INODE_I_MODE_OFFSET,
                inode.mode.load(Ordering::Acquire) as u16,
            );
            raw_write_usize(
                raw_inode,
                LINUX_INODE_I_MAPPING_OFFSET,
                raw_mapping as usize,
            );
            raw_write_u32(raw_inode, LINUX_INODE_I_RDEV_OFFSET, internal_devt);
            raw_write_usize(
                raw_mapping,
                LINUX_ADDRESS_SPACE_HOST_OFFSET,
                raw_inode as usize,
            );
        }

        let top_fops = top_fops as *const u8;
        if !unsafe { linux_fops_get(top_fops) } {
            unsafe { crate::mm::slab::kfree(raw_file) };
            return Err(ENXIO);
        }
        let open = unsafe { raw_read_usize(top_fops, LINUX_FOPS_OPEN_OFFSET) };
        if open != 0 {
            let callback: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32 =
                unsafe { core::mem::transmute(open) };
            let result = unsafe { callback(raw_inode.cast(), raw_file.cast()) };
            if result < 0 {
                if let Some(fops) = unsafe { linux_char_current_fops(raw_file) } {
                    unsafe { linux_fops_put(fops) };
                }
                unsafe { crate::mm::slab::kfree(raw_file) };
                return Err(-result);
            }
        }
        let opened_mode = unsafe { raw_file.add(LINUX_FILE_F_MODE_OFFSET).cast::<u32>().read() };
        unsafe {
            raw_write_u32(
                raw_file,
                LINUX_FILE_F_MODE_OFFSET,
                opened_mode | FMODE_OPENED,
            )
        };

        let file = alloc_file(dentry, flags, mode, &LINUX_CHAR_FILE_OPS);
        *file.private.lock() = raw_file as usize;
        Ok(file)
    })())
}

/// Dispatch read(2) with the original userspace pointer. Vendor ALSA's
/// callbacks perform their own `copy_to_user()` and therefore must not receive
/// the native VFS bounce buffer.
pub unsafe fn linux_module_chardev_read(
    file: &FileRef,
    user: *mut u8,
    count: usize,
) -> Option<i64> {
    let raw_file = unsafe { linux_char_raw_file(file) }?;
    let flags = file.flags.load(Ordering::Acquire);
    if flags & O_PATH != 0 || !matches!(flags & O_ACCMODE, O_RDONLY | O_RDWR) {
        return Some(-(EBADF as i64));
    }
    let fops = unsafe { linux_char_current_fops(raw_file) }?;
    let read = unsafe { raw_read_usize(fops, LINUX_FOPS_READ_OFFSET) };
    if read == 0 {
        return Some(-(ENOSYS as i64));
    }
    let callback: unsafe extern "C" fn(*mut c_void, *mut u8, usize, *mut i64) -> isize =
        unsafe { core::mem::transmute(read) };
    let pos = unsafe { raw_file.add(LINUX_FILE_F_POS_OFFSET).cast::<i64>() };
    Some(unsafe { callback(raw_file.cast(), user, count, pos) } as i64)
}

/// Dispatch write(2) with the original userspace pointer.
pub unsafe fn linux_module_chardev_write(
    file: &FileRef,
    user: *const u8,
    count: usize,
) -> Option<i64> {
    let raw_file = unsafe { linux_char_raw_file(file) }?;
    let flags = file.flags.load(Ordering::Acquire);
    if flags & O_PATH != 0 || !matches!(flags & O_ACCMODE, O_WRONLY | O_RDWR) {
        return Some(-(EBADF as i64));
    }
    let fops = unsafe { linux_char_current_fops(raw_file) }?;
    let write = unsafe { raw_read_usize(fops, LINUX_FOPS_WRITE_OFFSET) };
    if write == 0 {
        return Some(-(ENOSYS as i64));
    }
    let callback: unsafe extern "C" fn(*mut c_void, *const u8, usize, *mut i64) -> isize =
        unsafe { core::mem::transmute(write) };
    let pos = unsafe { raw_file.add(LINUX_FILE_F_POS_OFFSET).cast::<i64>() };
    Some(unsafe { callback(raw_file.cast(), user, count, pos) } as i64)
}

pub fn sync_linux_module_chardev_flags(file: &FileRef) {
    let Some(raw_file) = (unsafe { linux_char_raw_file(file) }) else {
        return;
    };
    unsafe {
        raw_write_u32(
            raw_file,
            LINUX_FILE_F_FLAGS_OFFSET,
            file.flags.load(Ordering::Acquire),
        )
    };
}

unsafe fn cdev_write_ops(cdev: *mut c_void, fops: *const c_void) {
    unsafe {
        cdev.cast::<u8>()
            .add(LINUX_CDEV_OPS_OFFSET)
            .cast::<*const c_void>()
            .write(fops);
    }
}

unsafe fn cdev_write_dev_count(cdev: *mut c_void, dev: u32, count: u32) {
    unsafe {
        cdev.cast::<u8>()
            .add(LINUX_CDEV_DEV_OFFSET)
            .cast::<u32>()
            .write(dev);
        cdev.cast::<u8>()
            .add(LINUX_CDEV_COUNT_OFFSET)
            .cast::<u32>()
            .write(count);
    }
}

unsafe fn linux_device_devt(dev: *const c_void) -> u32 {
    if dev.is_null() {
        0
    } else {
        unsafe {
            dev.cast::<u8>()
                .add(LINUX_DEVICE_DEVT_OFFSET)
                .cast::<u32>()
                .read()
        }
    }
}

fn ranges_overlap(a_base: u32, a_count: u32, b_base: u32, b_count: u32) -> bool {
    let Some(a_end) = a_base.checked_add(a_count) else {
        return true;
    };
    let Some(b_end) = b_base.checked_add(b_count) else {
        return true;
    };
    a_base < b_end && b_base < a_end
}

unsafe fn c_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let len = unsafe { crate::lib::string::c_strlen(ptr, 256) };
    let bytes = unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len) };
    String::from(core::str::from_utf8(bytes).unwrap_or(""))
}

fn register_chrdev_region(
    requested_major: u32,
    baseminor: u32,
    count: u32,
    name: String,
    fops: usize,
) -> Result<u32, i32> {
    if count == 0 || baseminor > MINORMASK || count - 1 > MINORMASK - baseminor {
        return Err(EINVAL);
    }

    let mut regions = CHRDEVS.lock();
    let major = if requested_major != 0 {
        requested_major
    } else {
        (FIRST_DYNAMIC_MAJOR..=LAST_DYNAMIC_MAJOR)
            .find(|candidate| !regions.iter().any(|region| region.major == *candidate))
            .ok_or(EBUSY)?
    };

    if regions.iter().any(|region| {
        region.major == major && ranges_overlap(region.baseminor, region.count, baseminor, count)
    }) {
        return Err(EBUSY);
    }

    regions.push(CharDeviceRegion {
        major,
        baseminor,
        count,
        name,
        fops,
    });
    Ok(major)
}

unsafe extern "C" fn linux_register_chrdev_region(
    from: u32,
    count: u32,
    name: *const c_char,
) -> i32 {
    let major = from >> MINORBITS;
    let baseminor = from & MINORMASK;
    match register_chrdev_region(major, baseminor, count, unsafe { c_string(name) }, 0) {
        Ok(_) => 0,
        Err(err) => -err,
    }
}

unsafe extern "C" fn linux_alloc_chrdev_region(
    dev: *mut u32,
    baseminor: u32,
    count: u32,
    name: *const c_char,
) -> i32 {
    if dev.is_null() {
        return -EINVAL;
    }
    match register_chrdev_region(0, baseminor, count, unsafe { c_string(name) }, 0) {
        Ok(major) => {
            unsafe { dev.write(mkdev(major, baseminor)) };
            0
        }
        Err(err) => -err,
    }
}

unsafe extern "C" fn linux___register_chrdev(
    major: u32,
    baseminor: u32,
    count: u32,
    name: *const c_char,
    fops: *const c_void,
) -> i32 {
    match register_chrdev_region(
        major,
        baseminor,
        count,
        unsafe { c_string(name) },
        fops as usize,
    ) {
        Ok(allocated_major) if major == 0 => allocated_major as i32,
        Ok(_) => 0,
        Err(err) => -err,
    }
}

unsafe extern "C" fn linux_unregister_chrdev_region(from: u32, count: u32) {
    let major = from >> MINORBITS;
    let baseminor = from & MINORMASK;
    CHRDEVS.lock().retain(|region| {
        !(region.major == major && ranges_overlap(region.baseminor, region.count, baseminor, count))
    });
}

unsafe extern "C" fn linux___unregister_chrdev(
    major: u32,
    baseminor: u32,
    count: u32,
    name: *const c_char,
) {
    let name = unsafe { c_string(name) };
    CHRDEVS.lock().retain(|region| {
        !(region.major == major
            && region.baseminor == baseminor
            && region.count == count
            && (name.is_empty() || region.name == name))
    });
}

unsafe extern "C" fn linux_cdev_init(cdev: *mut c_void, fops: *const c_void) {
    if cdev.is_null() {
        return;
    }
    unsafe {
        core::ptr::write_bytes(cdev.cast::<u8>(), 0, LINUX_CDEV_SIZE);
        cdev_write_ops(cdev, fops);
    }
}

unsafe extern "C" fn linux_cdev_alloc() -> *mut c_void {
    let cdev = Box::into_raw(Box::new([0u8; LINUX_CDEV_SIZE])).cast::<c_void>();
    DYNAMIC_CDEVS.lock().push(cdev as usize);
    cdev
}

unsafe extern "C" fn linux_cdev_add(cdev: *mut c_void, dev: u32, count: u32) -> i32 {
    if cdev.is_null() || count == 0 {
        return -EINVAL;
    }
    unsafe { cdev_write_dev_count(cdev, dev, count) };
    0
}

unsafe extern "C" fn linux_cdev_del(cdev: *mut c_void) {
    if cdev.is_null() {
        return;
    }
    let mut dynamic = DYNAMIC_CDEVS.lock();
    if let Some(pos) = dynamic.iter().position(|ptr| *ptr == cdev as usize) {
        dynamic.swap_remove(pos);
        unsafe {
            let _ = Box::from_raw(cdev.cast::<[u8; LINUX_CDEV_SIZE]>());
        }
    }
}

unsafe extern "C" fn linux_cdev_set_parent(_cdev: *mut c_void, _kobj: *mut c_void) {}

unsafe extern "C" fn linux_cdev_device_add(
    cdev: *mut c_void,
    dev: *mut crate::linux_driver_abi::base::LinuxDevice,
) -> i32 {
    if dev.is_null() {
        return -EINVAL;
    }
    let devt = unsafe { linux_device_devt(dev.cast_const().cast()) };
    if devt != 0 {
        let rc = unsafe { linux_cdev_add(cdev, devt, 1) };
        if rc != 0 {
            return rc;
        }
    }
    unsafe { crate::linux_driver_abi::base::linux_device_add(dev) }
}

unsafe extern "C" fn linux_cdev_device_del(
    cdev: *mut c_void,
    dev: *mut crate::linux_driver_abi::base::LinuxDevice,
) {
    if !dev.is_null() {
        unsafe { crate::linux_driver_abi::base::linux_device_unregister(dev) };
    }
    if !cdev.is_null() {
        unsafe { linux_cdev_del(cdev) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::mm_types::{MmStruct, VmAreaStruct};
    use crate::mm::vm_flags::VM_READ;
    use core::sync::atomic::AtomicU32;

    #[test]
    fn alloc_chrdev_region_returns_dynamic_dev_t() {
        let name = b"test\0";
        let mut dev = 0;
        let rc = unsafe { linux_alloc_chrdev_region(&mut dev, 2, 4, name.as_ptr().cast()) };
        assert_eq!(rc, 0);
        assert_eq!(dev & MINORMASK, 2);
        assert!(dev >> MINORBITS >= FIRST_DYNAMIC_MAJOR);
    }

    #[test]
    fn fops_get_pins_live_owner_until_fops_put() {
        // Configured vendor `struct module` has `state` at 0 and `refcnt` at
        // 1184. Linux `fops_get()` takes exactly one reference on a live owner.
        let mut module = [0u64; 152];
        let module_ptr = module.as_mut_ptr().cast::<u8>();
        unsafe {
            module_ptr.cast::<AtomicU32>().write(AtomicU32::new(0));
            module_ptr
                .add(1184)
                .cast::<AtomicU32>()
                .write(AtomicU32::new(1));
        }
        let mut fops = [0usize; 34];
        fops[LINUX_FOPS_OWNER_OFFSET / size_of::<usize>()] = module_ptr as usize;

        assert!(unsafe { linux_fops_get(fops.as_ptr().cast()) });
        assert_eq!(
            unsafe { &*module_ptr.add(1184).cast::<AtomicU32>() }.load(Ordering::Acquire),
            2
        );
        unsafe { linux_fops_put(fops.as_ptr().cast()) };
        assert_eq!(
            unsafe { &*module_ptr.add(1184).cast::<AtomicU32>() }.load(Ordering::Acquire),
            1
        );

        unsafe { module_ptr.cast::<AtomicU32>().write(AtomicU32::new(2)) };
        assert!(!unsafe { linux_fops_get(fops.as_ptr().cast()) });
    }

    #[test]
    fn vendor_vma_bridge_exposes_write_locked_linux_mm_layout() {
        let mut mm = MmStruct::new(0x1000);
        let mut vma = VmAreaStruct::new(0x4000, 0x5000, VM_READ);
        vma.vm_mm = &mut mm;
        let state = LinuxCharVmaBridge::new();

        let raw_vma = unsafe { linux_char_sync_raw_vma(&state, &mut vma) };
        let raw_mm = unsafe { raw_read_usize(raw_vma, LINUX_VMA_VM_MM_OFFSET) } as *const u8;

        assert_eq!(raw_mm, state.raw_mm.as_ptr());
        assert_eq!(
            unsafe { raw_read_u64(raw_mm, LINUX_MM_MMAP_LOCK_COUNT_OFFSET) },
            1
        );
        assert_eq!(
            unsafe {
                raw_vma
                    .add(LINUX_VMA_VM_LOCK_SEQ_OFFSET)
                    .cast::<u32>()
                    .read()
            },
            LINUX_MMAP_WRITE_SEQUENCE
        );
        assert_eq!(
            unsafe { raw_mm.add(LINUX_MM_MM_LOCK_SEQ_OFFSET).cast::<u32>().read() },
            LINUX_MMAP_WRITE_SEQUENCE
        );
    }
}
