//! linux-parity: partial
//! linux-source: vendor/linux/drivers/virtio
//! test-origin: linux:vendor/linux/drivers/virtio
//! VirtIO core — M57.
//!
//! Mirrors `include/linux/virtio.h`, `drivers/virtio/virtio.c`, and
//! `drivers/virtio/virtio_ring.c`.
//!
//! Linux's VirtIO transport abstraction allows a single driver (virtio-blk,
//! virtio-net, virtio-console) to work over PCI-modern, PCI-legacy, or MMIO.
//! This file exposes the core module/device-model ABI those Linux-built
//! drivers bind to: `virtio.c` bus registration and `virtio_ring.c` helper
//! symbols. PCI/MMIO transport behavior stays in Linux-built modules, and
//! missing ABI helpers fail closed.
//!
//! Runtime virtio-pci transport code must come from Linux-built
//! `virtio_pci*.ko` payloads staged from `vendor/linux/drivers/virtio/`.
//!
//! References:
//!   - `include/linux/virtio.h:168`              — `struct virtio_device`
//!   - `include/linux/virtio.h:247`              — `struct virtio_driver`
//!   - `include/uapi/linux/virtio_ring.h`        — vring ABI
//!   - `drivers/virtio/virtio.c:449`             — `__register_virtio_driver`
//!   - `drivers/virtio/virtio_ring.c`            — split-ring core helpers

extern crate alloc;

pub mod linux_sources;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicI32, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::arch::x86::mm::paging::pfn_to_virt;
use crate::block::bio::{BIO_OP_READ, BIO_OP_WRITE, BioRef};
use crate::include::uapi::errno::{E2BIG, EINVAL, EIO, ENODEV, ENOSPC, EOPNOTSUPP};
use crate::kernel::dma::{
    DmaAddr, DmaDirection, dma_alloc_coherent, dma_free_coherent, dma_map_single,
};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::linux_driver_abi::base::{
    LinuxBusType, LinuxDevice, LinuxDeviceDriver, LinuxListHead, linux_device_add,
    linux_device_initialize, linux_device_set_name_index, linux_device_unregister,
    linux_driver_register, linux_driver_unregister, register_linux_bus_type,
};
use crate::linux_driver_abi::block::{
    LINUX_REQUEST_PDU_OFFSET, LinuxBlockBackendHooks, LinuxRequest, LinuxRequestQueue,
    blk_mq_complete_request, linux_disk_name, register_linux_block_backend_hooks,
};
use crate::mm::buddy::page_to_pfn;
use crate::mm::page::Page;

// ── VirtIO device IDs (subset) ────────────────────────────────────────────────
// From `include/uapi/linux/virtio_ids.h`.
pub const VIRTIO_ID_NET: u32 = 1;
pub const VIRTIO_ID_BLOCK: u32 = 2;
pub const VIRTIO_ID_CONSOLE: u32 = 3;
pub const VIRTIO_ID_RNG: u32 = 4;
pub const VIRTIO_ID_INPUT: u32 = 18;
pub const VIRTIO_DEV_ANY_ID: u32 = 0xffff_ffff;
#[cfg(any(
    test,
    feature = "test-initramfs-rootfs",
    feature = "test-disk-root-remount"
))]
pub const PCI_VENDOR_ID_VIRTIO: u16 = 0x1af4;
#[cfg(any(
    test,
    feature = "test-initramfs-rootfs",
    feature = "test-disk-root-remount"
))]
pub const VIRTIO_PCI_LEGACY_DEVICE_ID_MIN: u16 = 0x1000;
#[cfg(any(
    test,
    feature = "test-initramfs-rootfs",
    feature = "test-disk-root-remount"
))]
pub const VIRTIO_PCI_MODERN_DEVICE_ID_BASE: u16 = 0x1040;
#[cfg(any(
    test,
    feature = "test-initramfs-rootfs",
    feature = "test-disk-root-remount"
))]
pub const VIRTIO_PCI_DEVICE_ID_MAX: u16 = 0x107f;

/// Decode the VirtIO device id from a PCI vendor/device/subsystem tuple for
/// test/boot-gate diagnostics only.
///
/// Mirrors Linux virtio-pci ID matching without creating or binding a device:
/// runtime transport and block driver behavior must still come from
/// `vendor/linux/drivers/virtio/virtio_pci_common.c` and
/// `vendor/linux/drivers/block/virtio_blk.c` module payloads.
#[cfg(any(
    test,
    feature = "test-initramfs-rootfs",
    feature = "test-disk-root-remount"
))]
pub fn virtio_device_id_from_pci_ids(
    vendor: u16,
    device: u16,
    subsystem_device: u16,
) -> Option<u32> {
    if vendor != PCI_VENDOR_ID_VIRTIO {
        return None;
    }
    if (VIRTIO_PCI_MODERN_DEVICE_ID_BASE..=VIRTIO_PCI_DEVICE_ID_MAX).contains(&device) {
        return Some((device - VIRTIO_PCI_MODERN_DEVICE_ID_BASE) as u32);
    }
    if device >= VIRTIO_PCI_LEGACY_DEVICE_ID_MIN && device < VIRTIO_PCI_MODERN_DEVICE_ID_BASE {
        return Some(subsystem_device as u32);
    }
    None
}

// ── VirtIO feature bits ───────────────────────────────────────────────────────
// From `include/uapi/linux/virtio_config.h`.
pub const VIRTIO_F_VERSION_1: u64 = 1 << 32;
pub const VIRTIO_F_RING_PACKED: u64 = 1 << 34;
pub const VIRTIO_BLK_F_RO: u64 = 1 << 5;
pub const VIRTIO_NET_F_MAC: u64 = 1 << 5;
pub const VIRTIO_CONFIG_S_ACKNOWLEDGE: u8 = 1;
pub const VIRTIO_CONFIG_S_DRIVER: u8 = 2;
pub const VIRTIO_CONFIG_S_DRIVER_OK: u8 = 4;
pub const VIRTIO_CONFIG_S_FEATURES_OK: u8 = 8;
pub const VIRTIO_CONFIG_S_FAILED: u8 = 0x80;
pub const VIRTIO_TRANSPORT_F_START: u32 = 28;
pub const VIRTIO_TRANSPORT_F_END: u32 = 42;
pub const VIRTIO_FEATURES_U64S: usize = 2;
pub const VRING_DESC_F_NEXT: u16 = 1;
pub const VRING_DESC_F_WRITE: u16 = 2;
pub const VRING_DESC_F_INDIRECT: u16 = 4;
pub const VRING_DESC_ALIGN_SIZE: usize = 16;
pub const VRING_AVAIL_ALIGN_SIZE: usize = 2;
pub const VRING_USED_ALIGN_SIZE: usize = 4;

/// `struct virtio_config_ops` — `vendor/linux/include/linux/virtio_config.h:112`.
#[repr(C)]
pub struct LinuxVirtioConfigOps {
    pub get:
        Option<unsafe extern "C" fn(vdev: *mut c_void, offset: u32, buf: *mut c_void, len: u32)>,
    pub set:
        Option<unsafe extern "C" fn(vdev: *mut c_void, offset: u32, buf: *const c_void, len: u32)>,
    pub generation: Option<unsafe extern "C" fn(vdev: *mut c_void) -> u32>,
    pub get_status: Option<unsafe extern "C" fn(vdev: *mut c_void) -> u8>,
    pub set_status: Option<unsafe extern "C" fn(vdev: *mut c_void, status: u8)>,
    pub reset: Option<unsafe extern "C" fn(vdev: *mut c_void)>,
    pub find_vqs: Option<
        unsafe extern "C" fn(
            vdev: *mut c_void,
            nvqs: u32,
            vqs: *mut *mut c_void,
            vqs_info: *mut c_void,
            desc: *mut c_void,
        ) -> i32,
    >,
    pub del_vqs: Option<unsafe extern "C" fn(vdev: *mut c_void)>,
    pub synchronize_cbs: Option<unsafe extern "C" fn(vdev: *mut c_void)>,
    pub get_features: Option<unsafe extern "C" fn(vdev: *mut c_void) -> u64>,
    pub get_extended_features: Option<unsafe extern "C" fn(vdev: *mut c_void, features: *mut u64)>,
    pub finalize_features: Option<unsafe extern "C" fn(vdev: *mut c_void) -> i32>,
    pub bus_name: Option<unsafe extern "C" fn(vdev: *mut c_void) -> *const c_char>,
    pub set_vq_affinity:
        Option<unsafe extern "C" fn(vq: *mut c_void, cpu_mask: *const c_void) -> i32>,
    pub get_vq_affinity:
        Option<unsafe extern "C" fn(vdev: *mut c_void, index: i32) -> *const c_void>,
    pub get_shm_region:
        Option<unsafe extern "C" fn(vdev: *mut c_void, region: *mut c_void, id: u8) -> bool>,
    pub disable_vq_and_reset: Option<unsafe extern "C" fn(vq: *mut c_void) -> i32>,
    pub enable_vq_after_reset: Option<unsafe extern "C" fn(vq: *mut c_void) -> i32>,
}

pub type LinuxVirtqueueCallback = Option<unsafe extern "C" fn(vq: *mut c_void)>;
pub type LinuxVirtqueueNotify = Option<unsafe extern "C" fn(vq: *mut LinuxVirtqueue) -> bool>;
pub type LinuxVirtqueueRecycle =
    Option<unsafe extern "C" fn(vq: *mut LinuxVirtqueue, buf: *mut c_void)>;
pub type LinuxVirtqueueRecycleDone = Option<unsafe extern "C" fn(vq: *mut LinuxVirtqueue)>;

/// `struct virtqueue_info` — `vendor/linux/include/linux/virtio_config.h:22`.
#[repr(C)]
pub struct LinuxVirtqueueInfo {
    pub name: *const c_char,
    pub callback: LinuxVirtqueueCallback,
    pub ctx: bool,
}

/// Public prefix of `struct virtqueue` — `vendor/linux/include/linux/virtio.h:34`.
#[repr(C)]
pub struct LinuxVirtqueue {
    pub list: LinuxListHead,
    pub callback: LinuxVirtqueueCallback,
    pub name: *const c_char,
    pub vdev: *mut LinuxVirtioDevice,
    pub index: u32,
    pub num_free: u32,
    pub num_max: u32,
    pub reset: bool,
    pub priv_: *mut c_void,
}

/// `struct vring_desc` - `vendor/linux/include/uapi/linux/virtio_ring.h`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LinuxVringDesc {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
    pub next: u16,
}

/// `struct vring_used_elem` - `vendor/linux/include/uapi/linux/virtio_ring.h`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LinuxVringUsedElem {
    pub id: u32,
    pub len: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LinuxVringSplitLayout {
    desc_offset: usize,
    avail_offset: usize,
    used_offset: usize,
    total_size: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LinuxVirtqueueToken {
    data: usize,
    len: u32,
    ctx: usize,
    head: u16,
    descriptors: Vec<u16>,
}

struct LinuxVirtqueueBackend {
    vq: usize,
    notify: LinuxVirtqueueNotify,
    ring_cpu: usize,
    ring_dma: u64,
    ring_len: usize,
    layout: LinuxVringSplitLayout,
    ring_ready: bool,
    callbacks_enabled: bool,
    pending_notify: bool,
    last_used_idx: u32,
    avail_idx_shadow: u16,
    free_list: Vec<u16>,
    submitted: Vec<LinuxVirtqueueToken>,
}

lazy_static! {
    static ref LINUX_VIRTQUEUE_BACKENDS: Mutex<Vec<LinuxVirtqueueBackend>> = Mutex::new(Vec::new());
}

/// `struct virtio_device_id` — `vendor/linux/include/linux/mod_devicetable.h`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LinuxVirtioDeviceId {
    pub device: u32,
    pub vendor: u32,
}

/// `struct virtio_driver` — `vendor/linux/include/linux/virtio.h:247`.
#[repr(C)]
pub struct LinuxVirtioDriver {
    pub driver: LinuxDeviceDriver,
    pub id_table: *const LinuxVirtioDeviceId,
    pub feature_table: *const u32,
    pub feature_table_size: u32,
    pub feature_table_legacy: *const u32,
    pub feature_table_size_legacy: u32,
    pub validate: Option<unsafe extern "C" fn(dev: *mut c_void) -> i32>,
    pub probe: Option<unsafe extern "C" fn(dev: *mut c_void) -> i32>,
    pub scan: Option<unsafe extern "C" fn(dev: *mut c_void)>,
    pub remove: Option<unsafe extern "C" fn(dev: *mut c_void)>,
    pub config_changed: Option<unsafe extern "C" fn(dev: *mut c_void)>,
    pub freeze: Option<unsafe extern "C" fn(dev: *mut c_void) -> i32>,
    pub restore: Option<unsafe extern "C" fn(dev: *mut c_void) -> i32>,
    pub reset_prepare: Option<unsafe extern "C" fn(dev: *mut c_void) -> i32>,
    pub reset_done: Option<unsafe extern "C" fn(dev: *mut c_void) -> i32>,
    pub shutdown: Option<unsafe extern "C" fn(dev: *mut c_void)>,
}

/// Prefix of `struct virtio_device` before the embedded `struct device`.
///
/// Source: `vendor/linux/include/linux/virtio.h:168`.  The full embedded
/// device layout is intentionally not modeled here yet.
#[repr(C)]
pub struct LinuxVirtioDevicePrefix {
    pub index: i32,
    pub failed: bool,
    pub config_core_enabled: bool,
    pub config_driver_disabled: bool,
    pub config_change_pending: bool,
}

/// Prefix of `struct virtio_device` through `priv`.
///
/// Source: `vendor/linux/include/linux/virtio.h:168`.  This is the transport
/// object Linux-built virtio modules pass to `register_virtio_device()`.
#[repr(C)]
pub struct LinuxVirtioDevice {
    pub prefix: LinuxVirtioDevicePrefix,
    pub dev: LinuxDevice,
    pub _pad_after_device_prefix: [u8; LINUX_VIRTIO_DEVICE_ID_OFFSET
        - (LINUX_VIRTIO_DEVICE_DEV_OFFSET + core::mem::size_of::<LinuxDevice>())],
    pub id: LinuxVirtioDeviceId,
    pub config: *const LinuxVirtioConfigOps,
    pub vringh_config: *const c_void,
    pub map: *const c_void,
    pub vqs: LinuxListHead,
    pub features: [u64; VIRTIO_FEATURES_U64S],
    pub priv_: *mut c_void,
}

pub const LINUX_VIRTIO_DEVICE_DEV_OFFSET: usize = 0x8;
pub const LINUX_VIRTIO_DEVICE_ID_OFFSET: usize = 0x1b8;
pub const LINUX_VIRTIO_DEVICE_CONFIG_OFFSET: usize = 0x1c0;
pub const LINUX_VIRTIO_DEVICE_PRIV_OFFSET: usize = 0x1f8;

// ── Virtqueue ─────────────────────────────────────────────────────────────────

// ── VirtIO device and driver ──────────────────────────────────────────────────

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

static VIRTIO_BUS_NAME: [u8; 7] = *b"virtio\0";
static NEXT_LINUX_VIRTIO_INDEX: AtomicI32 = AtomicI32::new(0);

fn linux_virtio_config_ops(dev: *mut c_void) -> Option<*const LinuxVirtioConfigOps> {
    if dev.is_null() {
        return None;
    }
    let config = unsafe { (*dev.cast::<LinuxVirtioDevice>()).config };
    if config.is_null() { None } else { Some(config) }
}

fn linux_virtio_device_id(dev: *mut c_void) -> Option<LinuxVirtioDeviceId> {
    if dev.is_null() {
        return None;
    }
    let id = unsafe { (*dev.cast::<LinuxVirtioDevice>()).id };
    if id.device == 0 { None } else { Some(id) }
}

unsafe fn linux_virtio_driver_for_device(dev: *mut c_void) -> Option<*mut LinuxVirtioDriver> {
    if dev.is_null() {
        return None;
    }
    let embedded = unsafe { linux_virtio_embedded_device(dev) };
    let driver = unsafe { (*embedded).driver };
    if driver.is_null() {
        None
    } else {
        Some(driver.cast::<LinuxVirtioDriver>())
    }
}

fn linux_virtqueue_err_ptr(err: i32) -> *mut LinuxVirtqueue {
    (err as isize) as usize as *mut LinuxVirtqueue
}

fn linux_virtio_id_matches(dev: LinuxVirtioDeviceId, id: LinuxVirtioDeviceId) -> bool {
    (id.device == dev.device || id.device == VIRTIO_DEV_ANY_ID)
        && (id.vendor == dev.vendor || id.vendor == VIRTIO_DEV_ANY_ID)
}

fn linux_virtio_driver_offered_feature(driver: &LinuxVirtioDriver, fbit: u32) -> bool {
    if !driver.feature_table.is_null() {
        let mut idx = 0usize;
        while idx < driver.feature_table_size as usize {
            if unsafe { *driver.feature_table.add(idx) } == fbit {
                return true;
            }
            idx += 1;
        }
    }

    if !driver.feature_table_legacy.is_null() {
        let mut idx = 0usize;
        while idx < driver.feature_table_size_legacy as usize {
            if unsafe { *driver.feature_table_legacy.add(idx) } == fbit {
                return true;
            }
            idx += 1;
        }
    }

    false
}

fn linux_virtio_feature_word(fbit: u32) -> Option<(usize, u64)> {
    let word = (fbit / 64) as usize;
    if word >= VIRTIO_FEATURES_U64S {
        return None;
    }
    Some((word, 1u64 << (fbit % 64)))
}

fn linux_virtio_features_has(features: &[u64; VIRTIO_FEATURES_U64S], fbit: u32) -> bool {
    let Some((word, bit)) = linux_virtio_feature_word(fbit) else {
        return false;
    };
    features[word] & bit != 0
}

fn linux_virtio_features_set(features: &mut [u64; VIRTIO_FEATURES_U64S], fbit: u32) {
    if let Some((word, bit)) = linux_virtio_feature_word(fbit) {
        features[word] |= bit;
    }
}

fn linux_align_up(value: usize, align: usize) -> Option<usize> {
    if align == 0 || !align.is_power_of_two() {
        return None;
    }
    value.checked_add(align - 1).map(|v| v & !(align - 1))
}

fn linux_vring_split_layout(num: u32, align: u32) -> Option<LinuxVringSplitLayout> {
    if num == 0 || !num.is_power_of_two() {
        return None;
    }
    let num = usize::try_from(num).ok()?;
    let align = usize::try_from(align).ok()?;
    let desc_size = core::mem::size_of::<LinuxVringDesc>().checked_mul(num)?;
    let avail_size = core::mem::size_of::<u16>().checked_mul(3usize.checked_add(num)?)?;
    let used_offset = linux_align_up(desc_size.checked_add(avail_size)?, align)?;
    let used_size = core::mem::size_of::<u16>()
        .checked_mul(3)?
        .checked_add(core::mem::size_of::<LinuxVringUsedElem>().checked_mul(num)?)?;
    let total_size = used_offset.checked_add(used_size)?;
    Some(LinuxVringSplitLayout {
        desc_offset: 0,
        avail_offset: desc_size,
        used_offset,
        total_size,
    })
}

fn linux_vring_desc_dma(backend: &LinuxVirtqueueBackend) -> u64 {
    backend.ring_dma + backend.layout.desc_offset as u64
}

fn linux_vring_avail_dma(backend: &LinuxVirtqueueBackend) -> u64 {
    backend.ring_dma + backend.layout.avail_offset as u64
}

fn linux_vring_used_dma(backend: &LinuxVirtqueueBackend) -> u64 {
    backend.ring_dma + backend.layout.used_offset as u64
}

unsafe fn linux_vring_ptr<T>(backend: &LinuxVirtqueueBackend, offset: usize) -> *mut T {
    unsafe { (backend.ring_cpu as *mut u8).add(offset).cast::<T>() }
}

unsafe fn linux_vring_desc_ptr(backend: &LinuxVirtqueueBackend, idx: u16) -> *mut LinuxVringDesc {
    let offset = backend.layout.desc_offset + idx as usize * core::mem::size_of::<LinuxVringDesc>();
    unsafe { linux_vring_ptr::<LinuxVringDesc>(backend, offset) }
}

unsafe fn linux_vring_write_desc(backend: &LinuxVirtqueueBackend, idx: u16, desc: LinuxVringDesc) {
    unsafe {
        write_volatile(linux_vring_desc_ptr(backend, idx), desc);
    }
}

unsafe fn linux_vring_read_desc(backend: &LinuxVirtqueueBackend, idx: u16) -> LinuxVringDesc {
    unsafe { read_volatile(linux_vring_desc_ptr(backend, idx)) }
}

unsafe fn linux_vring_avail_idx(backend: &LinuxVirtqueueBackend) -> u16 {
    unsafe {
        read_volatile(linux_vring_ptr::<u16>(
            backend,
            backend.layout.avail_offset + 2,
        ))
    }
}

unsafe fn linux_vring_write_avail_idx(backend: &LinuxVirtqueueBackend, value: u16) {
    unsafe {
        write_volatile(
            linux_vring_ptr::<u16>(backend, backend.layout.avail_offset + 2),
            value,
        );
    }
}

unsafe fn linux_vring_write_avail_ring(backend: &LinuxVirtqueueBackend, slot: usize, head: u16) {
    let offset = backend.layout.avail_offset + 4 + slot * core::mem::size_of::<u16>();
    unsafe {
        write_volatile(linux_vring_ptr::<u16>(backend, offset), head);
    }
}

unsafe fn linux_vring_used_idx(backend: &LinuxVirtqueueBackend) -> u16 {
    unsafe {
        read_volatile(linux_vring_ptr::<u16>(
            backend,
            backend.layout.used_offset + 2,
        ))
    }
}

unsafe fn linux_vring_set_used_idx(backend: &LinuxVirtqueueBackend, value: u16) {
    unsafe {
        write_volatile(
            linux_vring_ptr::<u16>(backend, backend.layout.used_offset + 2),
            value,
        );
    }
}

unsafe fn linux_vring_used_elem(
    backend: &LinuxVirtqueueBackend,
    slot: usize,
) -> LinuxVringUsedElem {
    let offset = backend.layout.used_offset + 4 + slot * core::mem::size_of::<LinuxVringUsedElem>();
    unsafe { read_volatile(linux_vring_ptr::<LinuxVringUsedElem>(backend, offset)) }
}

fn linux_virtqueue_register_backend(
    vq: *mut LinuxVirtqueue,
    notify: LinuxVirtqueueNotify,
    ring_cpu: *mut u8,
    ring_dma: u64,
    ring_len: usize,
    layout: LinuxVringSplitLayout,
) {
    if vq.is_null() {
        return;
    }
    let num = unsafe { (*vq).num_max as u16 };
    let mut free_list = Vec::with_capacity(num as usize);
    for idx in (0..num).rev() {
        free_list.push(idx);
    }
    let mut backends = LINUX_VIRTQUEUE_BACKENDS.lock();
    backends.retain(|backend| backend.vq != vq as usize);
    backends.push(LinuxVirtqueueBackend {
        vq: vq as usize,
        notify,
        ring_cpu: ring_cpu as usize,
        ring_dma,
        ring_len,
        layout,
        ring_ready: true,
        callbacks_enabled: true,
        pending_notify: false,
        last_used_idx: 0,
        avail_idx_shadow: 0,
        free_list,
        submitted: Vec::new(),
    });
}

fn linux_virtqueue_remove_backend(vq: *mut LinuxVirtqueue) {
    let mut backends = LINUX_VIRTQUEUE_BACKENDS.lock();
    if let Some(pos) = backends
        .iter()
        .position(|backend| backend.vq == vq as usize)
    {
        let backend = backends.remove(pos);
        unsafe {
            dma_free_coherent(backend.ring_cpu as *mut u8, backend.ring_len);
        }
    }
}

fn linux_virtqueue_with_backend_mut<R>(
    vq: *mut LinuxVirtqueue,
    f: impl FnOnce(&mut LinuxVirtqueueBackend) -> R,
) -> Option<R> {
    if vq.is_null() {
        return None;
    }
    LINUX_VIRTQUEUE_BACKENDS
        .lock()
        .iter_mut()
        .find(|backend| backend.vq == vq as usize)
        .map(f)
}

fn poll_virtqueues() -> usize {
    let ready = {
        let mut backends = LINUX_VIRTQUEUE_BACKENDS.lock();
        backends
            .iter_mut()
            .filter_map(|backend| {
                if !backend.ring_ready {
                    return None;
                }
                let vq = backend.vq as *mut LinuxVirtqueue;
                if vq.is_null() || unsafe { (*vq).reset } {
                    return None;
                }
                let used_idx = unsafe { linux_vring_used_idx(backend) } as u32;
                (used_idx != backend.last_used_idx).then_some(backend.vq)
            })
            .collect::<Vec<_>>()
    };
    for vq in ready.iter().copied() {
        unsafe {
            vring_interrupt(0, vq as *mut c_void);
        }
    }
    ready.len()
}

pub(crate) fn take_used_buffer_for_token(data: *mut c_void) -> bool {
    if data.is_null() {
        return false;
    }
    let data = data as usize;
    let mut backends = LINUX_VIRTQUEUE_BACKENDS.lock();
    for backend in backends.iter_mut() {
        if !backend.ring_ready {
            continue;
        }
        let vq = backend.vq as *mut LinuxVirtqueue;
        if vq.is_null() || unsafe { (*vq).reset } {
            continue;
        }
        let used_idx = unsafe { linux_vring_used_idx(backend) };
        if backend.last_used_idx as u16 == used_idx {
            continue;
        }
        core::sync::atomic::fence(Ordering::SeqCst);
        let slot = backend.last_used_idx as usize % unsafe { (*vq).num_max as usize };
        let elem = unsafe { linux_vring_used_elem(backend, slot) };
        let Some(pos) = backend
            .submitted
            .iter()
            .position(|token| token.head as u32 == elem.id)
        else {
            unsafe {
                (*vq).reset = true;
            }
            return false;
        };
        if backend.submitted[pos].data != data {
            continue;
        }
        let token = backend.submitted.remove(pos);
        backend.last_used_idx = backend.last_used_idx.wrapping_add(1);
        for desc in token.descriptors.iter().copied() {
            unsafe {
                linux_vring_write_desc(backend, desc, LinuxVringDesc::default());
            }
            backend.free_list.push(desc);
        }
        unsafe {
            (*vq).num_free = (*vq)
                .num_free
                .saturating_add(token.descriptors.len() as u32)
                .min((*vq).num_max);
        }
        return true;
    }
    false
}

pub(crate) fn has_virtqueue_backend(vq: *mut LinuxVirtqueue) -> bool {
    if vq.is_null() {
        return false;
    }
    LINUX_VIRTQUEUE_BACKENDS
        .lock()
        .iter()
        .any(|backend| backend.vq == vq as usize && backend.ring_ready)
}

const LINUX_VIRTIO_BLK_NUM_VQS_OFFSET: usize = 0x16c;
const LINUX_VIRTIO_BLK_VQS_OFFSET: usize = 0x180;
const LINUX_VIRTIO_BLK_VQ_STRIDE: usize = 0x18;
const VIRTIO_BLK_T_IN: u32 = 0;
const VIRTIO_BLK_T_OUT: u32 = 1;
const VIRTIO_BLK_S_OK: u8 = 0;
const VIRTIO_BLK_S_IOERR: u8 = 1;
const VIRTIO_BLK_COMPLETION_SPINS: usize = 1_000_000;

#[repr(C)]
struct LinuxVirtioBlkOutHdr {
    type_: u32,
    ioprio: u32,
    sector: u64,
}

fn linux_virtio_blk_try_complete_request(rq: *mut LinuxRequest) -> bool {
    if rq.is_null() {
        return false;
    }
    let pdu = unsafe {
        (rq.cast::<u8>())
            .add(LINUX_REQUEST_PDU_OFFSET)
            .cast::<c_void>()
    };
    if !take_used_buffer_for_token(pdu) {
        return false;
    }
    unsafe {
        blk_mq_complete_request(rq);
    }
    true
}

fn linux_virtio_blk_vq_for_queue(q: *mut LinuxRequestQueue) -> Option<*mut LinuxVirtqueue> {
    if q.is_null() {
        return None;
    }
    let disk = unsafe { (*q).disk };
    let name = linux_disk_name(disk).ok()?;
    if !name.starts_with("vd") {
        return None;
    }
    let vblk = unsafe { (*q).queuedata }.cast::<u8>();
    if vblk.is_null() {
        return None;
    }
    let num_vqs = unsafe { *vblk.add(LINUX_VIRTIO_BLK_NUM_VQS_OFFSET).cast::<i32>() };
    if num_vqs <= 0 {
        return None;
    }
    let hctx = if q.is_null() {
        core::ptr::null_mut()
    } else {
        let table = unsafe {
            (*q).queue_hw_ctx as *mut *mut crate::linux_driver_abi::block::LinuxBlkMqHwCtx
        };
        if table.is_null() {
            core::ptr::null_mut()
        } else {
            unsafe { *table }
        }
    };
    let queue_num = if hctx.is_null() {
        0
    } else {
        unsafe { (*hctx).queue_num as usize }
    };
    if queue_num >= num_vqs as usize {
        return None;
    }
    let vqs = unsafe { *vblk.add(LINUX_VIRTIO_BLK_VQS_OFFSET).cast::<*mut u8>() };
    if vqs.is_null() {
        return None;
    }
    let vq = unsafe {
        *vqs.add(queue_num.checked_mul(LINUX_VIRTIO_BLK_VQ_STRIDE)?)
            .cast::<*mut LinuxVirtqueue>()
    };
    has_virtqueue_backend(vq).then_some(vq)
}

fn linux_virtio_status_result(status: u8) -> Result<(), i32> {
    match status {
        VIRTIO_BLK_S_OK => Ok(()),
        VIRTIO_BLK_S_IOERR => Err(EIO),
        _ => Err(EIO),
    }
}

fn linux_sg_entry() -> crate::lib::scatterlist::LinuxScatterList {
    crate::lib::scatterlist::LinuxScatterList {
        page_link: 0,
        offset: 0,
        length: 0,
        dma_address: 0,
        dma_length: 0,
    }
}

fn linux_virtio_blk_submit_bio(q: *mut LinuxRequestQueue, bio: &BioRef) -> Option<Result<(), i32>> {
    // If a Linux-built block driver installed mq_ops, let its queue_rq path own
    // the request PDU token. The direct backend below uses a status-byte token,
    // which is only valid when no vendor virtio_blk completion callback will
    // interpret used-buffer tokens as `struct virtblk_req *`.
    if !q.is_null() && !unsafe { (*q).mq_ops }.is_null() {
        return None;
    }
    if bio.op.0 != BIO_OP_READ && bio.op.0 != BIO_OP_WRITE {
        return None;
    }
    let vq = linux_virtio_blk_vq_for_queue(q)?;
    let vecs = bio.vecs.lock();
    if vecs.is_empty() || bio.total_size() == 0 {
        return Some(Ok(()));
    }
    let mut guards = Vec::with_capacity(vecs.len());
    for vec in vecs.iter() {
        let guard = vec.data.lock();
        if vec.off > guard.len() || vec.len > guard.len().saturating_sub(vec.off) {
            return Some(Err(EINVAL));
        }
        if u32::try_from(vec.len).is_err() {
            return Some(Err(EINVAL));
        }
        guards.push((guard, vec.off, vec.len));
    }

    let mut hdr = LinuxVirtioBlkOutHdr {
        type_: match bio.op.0 {
            BIO_OP_READ => VIRTIO_BLK_T_IN.to_le(),
            BIO_OP_WRITE => VIRTIO_BLK_T_OUT.to_le(),
            _ => return None,
        },
        ioprio: 0,
        sector: bio.sector.to_le(),
    };
    let mut status = 0xffu8;
    let mut sg_entries = Vec::with_capacity(guards.len() + 2);
    let mut hdr_sg = linux_sg_entry();
    unsafe {
        crate::lib::scatterlist::linux_sg_init_one(
            &mut hdr_sg,
            (&mut hdr as *mut LinuxVirtioBlkOutHdr).cast::<c_void>(),
            core::mem::size_of::<LinuxVirtioBlkOutHdr>() as u32,
        );
    }
    sg_entries.push(hdr_sg);
    for (guard, off, len) in guards.iter_mut() {
        let mut sg = linux_sg_entry();
        let ptr = unsafe { guard.as_mut_ptr().add(*off).cast::<c_void>() };
        unsafe {
            crate::lib::scatterlist::linux_sg_init_one(&mut sg, ptr, *len as u32);
        }
        sg_entries.push(sg);
    }
    let mut status_sg = linux_sg_entry();
    unsafe {
        crate::lib::scatterlist::linux_sg_init_one(
            &mut status_sg,
            (&mut status as *mut u8).cast::<c_void>(),
            core::mem::size_of::<u8>() as u32,
        );
    }
    sg_entries.push(status_sg);

    let mut sg_ptrs = Vec::with_capacity(sg_entries.len());
    for sg in sg_entries.iter_mut() {
        sg_ptrs.push((sg as *mut crate::lib::scatterlist::LinuxScatterList).cast::<c_void>());
    }
    let data_sgs = guards.len() as u32;
    let (out_sgs, in_sgs) = if bio.op.0 == BIO_OP_WRITE {
        (1 + data_sgs, 1)
    } else {
        (1, data_sgs + 1)
    };
    let token = (&mut status as *mut u8).cast::<c_void>();
    let add = unsafe { virtqueue_add_sgs(vq, sg_ptrs.as_mut_ptr(), out_sgs, in_sgs, token, 0) };
    if add < 0 {
        return Some(Err((-add) as i32));
    }
    if !unsafe { virtqueue_kick(vq) } {
        return Some(Err(EIO));
    }
    for _ in 0..VIRTIO_BLK_COMPLETION_SPINS {
        if take_used_buffer_for_token(token) {
            return Some(linux_virtio_status_result(status));
        }
        core::hint::spin_loop();
    }
    crate::log_warn!(
        "virtio",
        "linux_virtio_blk_submit_bio: timeout op={} sector={} bytes={} segments={}",
        bio.op.0,
        bio.sector,
        bio.total_size(),
        guards.len()
    );
    Some(Err(EIO))
}

fn linux_scatterlist_count(mut sg: *const crate::lib::scatterlist::LinuxScatterList) -> u32 {
    let mut count = 0u32;
    while !sg.is_null() && count < 1024 {
        count = count.saturating_add(1);
        let flags = unsafe { (*sg).page_link & crate::lib::scatterlist::SG_PAGE_LINK_MASK };
        if flags & crate::lib::scatterlist::SG_END != 0 {
            break;
        }
        if flags & crate::lib::scatterlist::SG_CHAIN != 0 {
            let next = unsafe { (*sg).page_link & !crate::lib::scatterlist::SG_PAGE_LINK_MASK };
            sg = next as *const crate::lib::scatterlist::LinuxScatterList;
        } else {
            sg = unsafe { sg.add(1) };
        }
    }
    count
}

unsafe fn linux_list_head_init(head: *mut LinuxListHead) {
    unsafe {
        (*head).next = head.cast::<c_void>();
        (*head).prev = head.cast::<c_void>();
    }
}

unsafe fn linux_list_add_tail(node: *mut LinuxListHead, head: *mut LinuxListHead) {
    unsafe {
        let prev = (*head).prev.cast::<LinuxListHead>();
        (*node).next = head.cast::<c_void>();
        (*node).prev = prev.cast::<c_void>();
        (*prev).next = node.cast::<c_void>();
        (*head).prev = node.cast::<c_void>();
    }
}

unsafe fn linux_list_del_init(node: *mut LinuxListHead) {
    unsafe {
        let next = (*node).next.cast::<LinuxListHead>();
        let prev = (*node).prev.cast::<LinuxListHead>();
        if !next.is_null() && !prev.is_null() {
            (*next).prev = prev.cast::<c_void>();
            (*prev).next = next.cast::<c_void>();
        }
        linux_list_head_init(node);
    }
}

unsafe fn linux_virtio_get_features(
    dev: *mut c_void,
    config: &LinuxVirtioConfigOps,
) -> Result<[u64; VIRTIO_FEATURES_U64S], i32> {
    let mut features = [0u64; VIRTIO_FEATURES_U64S];
    if let Some(get_extended_features) = config.get_extended_features {
        unsafe {
            get_extended_features(dev, features.as_mut_ptr());
        }
        return Ok(features);
    }
    let Some(get_features) = config.get_features else {
        return Err(EINVAL);
    };
    features[0] = unsafe { get_features(dev) };
    Ok(features)
}

unsafe fn linux_virtio_driver_features(
    driver: &LinuxVirtioDriver,
) -> ([u64; VIRTIO_FEATURES_U64S], u64) {
    let mut driver_features = [0u64; VIRTIO_FEATURES_U64S];
    if !driver.feature_table.is_null() {
        let mut idx = 0usize;
        while idx < driver.feature_table_size as usize {
            linux_virtio_features_set(&mut driver_features, unsafe {
                *driver.feature_table.add(idx)
            });
            idx += 1;
        }
    }

    let mut legacy = 0u64;
    if !driver.feature_table_legacy.is_null() {
        let mut idx = 0usize;
        while idx < driver.feature_table_size_legacy as usize {
            let fbit = unsafe { *driver.feature_table_legacy.add(idx) };
            if fbit < 64 {
                legacy |= 1u64 << fbit;
            }
            idx += 1;
        }
    } else {
        legacy = driver_features[0];
    }

    (driver_features, legacy)
}

unsafe fn linux_virtio_features_ok(dev: *mut c_void, config: &LinuxVirtioConfigOps) -> i32 {
    let features = unsafe { &(*dev.cast::<LinuxVirtioDevice>()).features };
    if !linux_virtio_features_has(features, 32) {
        return 0;
    }

    let Some(get_status) = config.get_status else {
        return -EINVAL;
    };
    unsafe {
        virtio_add_status(dev, VIRTIO_CONFIG_S_FEATURES_OK as u32);
        if get_status(dev) & VIRTIO_CONFIG_S_FEATURES_OK == 0 {
            return -ENODEV;
        }
    }
    0
}

unsafe fn linux_virtio_config_changed_inner(dev: *mut c_void) {
    if dev.is_null() {
        return;
    }

    let prefix = dev.cast::<LinuxVirtioDevicePrefix>();
    unsafe {
        if !(*prefix).config_core_enabled || (*prefix).config_driver_disabled {
            (*prefix).config_change_pending = true;
            return;
        }
    }

    let Some(driver) = (unsafe { linux_virtio_driver_for_device(dev) }) else {
        return;
    };
    let Some(config_changed) = (unsafe { (*driver).config_changed }) else {
        return;
    };

    unsafe {
        config_changed(dev);
        (*prefix).config_change_pending = false;
    }
}

/// `virtio_dev_match` — `vendor/linux/drivers/virtio/virtio.c:85`.
unsafe extern "C" fn linux_virtio_dev_match(dev: *mut c_void, driver: *const c_void) -> i32 {
    if dev.is_null() || driver.is_null() {
        return 0;
    }

    let vdev = unsafe {
        dev.cast::<u8>()
            .sub(LINUX_VIRTIO_DEVICE_DEV_OFFSET)
            .cast::<c_void>()
    };
    let Some(dev_id) = linux_virtio_device_id(vdev) else {
        return 0;
    };

    let driver = driver.cast::<LinuxVirtioDriver>();
    let ids = unsafe { (*driver).id_table };
    if ids.is_null() {
        return 0;
    }

    let mut idx = 0usize;
    while idx < 4096 {
        let id = unsafe { *ids.add(idx) };
        if id.device == 0 {
            return 0;
        }
        if linux_virtio_id_matches(dev_id, id) {
            return 1;
        }
        idx += 1;
    }
    0
}

/// `virtio_dev_probe` — `vendor/linux/drivers/virtio/virtio.c:270`.
unsafe extern "C" fn linux_virtio_dev_probe(dev: *mut c_void) -> i32 {
    if dev.is_null() {
        return -EINVAL;
    }

    let vdev = unsafe {
        dev.cast::<u8>()
            .sub(LINUX_VIRTIO_DEVICE_DEV_OFFSET)
            .cast::<c_void>()
    };
    let config = match linux_virtio_config_ops(vdev) {
        Some(config) => unsafe { &*config },
        None => return -EINVAL,
    };
    let driver = match unsafe { (*dev.cast::<LinuxDevice>()).driver } {
        driver if !driver.is_null() => driver.cast::<LinuxVirtioDriver>(),
        _ => return -EINVAL,
    };
    let Some(finalize_features) = config.finalize_features else {
        unsafe {
            virtio_add_status(vdev, VIRTIO_CONFIG_S_FAILED as u32);
        }
        return -EINVAL;
    };
    let Some(probe) = (unsafe { (*driver).probe }) else {
        unsafe {
            virtio_add_status(vdev, VIRTIO_CONFIG_S_FAILED as u32);
        }
        return -EINVAL;
    };

    unsafe {
        virtio_add_status(vdev, VIRTIO_CONFIG_S_DRIVER as u32);
    }

    let device_features = match unsafe { linux_virtio_get_features(vdev, config) } {
        Ok(features) => features,
        Err(err) => {
            unsafe {
                virtio_add_status(vdev, VIRTIO_CONFIG_S_FAILED as u32);
            }
            return -err;
        }
    };
    let (driver_features, driver_features_legacy) =
        unsafe { linux_virtio_driver_features(&*driver) };

    let mut negotiated = [0u64; VIRTIO_FEATURES_U64S];
    if linux_virtio_features_has(&device_features, 32) {
        let mut idx = 0usize;
        while idx < VIRTIO_FEATURES_U64S {
            negotiated[idx] = driver_features[idx] & device_features[idx];
            idx += 1;
        }
    } else {
        negotiated[0] = driver_features_legacy & device_features[0];
    }

    let mut transport_bit = VIRTIO_TRANSPORT_F_START;
    while transport_bit < VIRTIO_TRANSPORT_F_END {
        if linux_virtio_features_has(&device_features, transport_bit) {
            linux_virtio_features_set(&mut negotiated, transport_bit);
        }
        transport_bit += 1;
    }

    unsafe {
        (*vdev.cast::<LinuxVirtioDevice>()).features = negotiated;
    }

    let mut err = unsafe { finalize_features(vdev) };
    if err != 0 {
        unsafe {
            virtio_add_status(vdev, VIRTIO_CONFIG_S_FAILED as u32);
        }
        return err;
    }

    if let Some(validate) = unsafe { (*driver).validate } {
        let before = unsafe { (*vdev.cast::<LinuxVirtioDevice>()).features };
        err = unsafe { validate(vdev) };
        if err != 0 {
            unsafe {
                virtio_add_status(vdev, VIRTIO_CONFIG_S_FAILED as u32);
            }
            return err;
        }
        if before != unsafe { (*vdev.cast::<LinuxVirtioDevice>()).features } {
            err = unsafe { finalize_features(vdev) };
            if err != 0 {
                unsafe {
                    virtio_add_status(vdev, VIRTIO_CONFIG_S_FAILED as u32);
                }
                return err;
            }
        }
    }

    err = unsafe { linux_virtio_features_ok(vdev, config) };
    if err != 0 {
        unsafe {
            virtio_add_status(vdev, VIRTIO_CONFIG_S_FAILED as u32);
        }
        return err;
    }

    err = unsafe { probe(vdev) };
    if err != 0 {
        unsafe {
            virtio_add_status(vdev, VIRTIO_CONFIG_S_FAILED as u32);
        }
        return err;
    }

    if let Some(get_status) = config.get_status {
        if unsafe { get_status(vdev) } & VIRTIO_CONFIG_S_DRIVER_OK == 0 {
            unsafe {
                virtio_add_status(vdev, VIRTIO_CONFIG_S_DRIVER_OK as u32);
            }
        }
    }
    if let Some(scan) = unsafe { (*driver).scan } {
        unsafe {
            scan(vdev);
        }
    }

    unsafe {
        (*vdev.cast::<LinuxVirtioDevice>())
            .prefix
            .config_core_enabled = true;
        if (*vdev.cast::<LinuxVirtioDevice>())
            .prefix
            .config_change_pending
        {
            linux_virtio_config_changed_inner(vdev);
        }
    }

    0
}

static LINUX_VIRTIO_BUS: LinuxBusType = LinuxBusType {
    name: VIRTIO_BUS_NAME.as_ptr().cast::<c_char>(),
    dev_name: core::ptr::null(),
    bus_groups: core::ptr::null(),
    dev_groups: core::ptr::null(),
    drv_groups: core::ptr::null(),
    match_fn: Some(linux_virtio_dev_match),
    uevent: None,
    probe: Some(linux_virtio_dev_probe),
    sync_state: None,
    remove: None,
    shutdown: None,
    irq_get_affinity: None,
    online: None,
    offline: None,
    suspend: None,
    resume: None,
    num_vf: None,
    dma_configure: None,
    dma_cleanup: None,
    pm: core::ptr::null(),
    driver_override: false,
    need_parent_lock: false,
};

fn linux_virtio_bus_ptr() -> *const LinuxBusType {
    core::ptr::addr_of!(LINUX_VIRTIO_BUS)
}

unsafe fn linux_virtio_embedded_device(dev: *mut c_void) -> *mut LinuxDevice {
    unsafe { core::ptr::addr_of_mut!((*dev.cast::<LinuxVirtioDevice>()).dev) }
}

/// Register Linux virtio core symbols needed by vendor-built virtio modules.
///
/// The exported entry points mirror `drivers/virtio/virtio.c` and
/// `drivers/virtio/virtio_ring.c`. Driver and device registration flow through
/// the C driver-core ABI; local Rust virtio-pci and virtio-blk driver paths
/// are retired.
pub fn register_module_exports() {
    crate::linux_driver_abi::register_driver_abi_poller("virtio", poll_virtqueues);
    register_linux_block_backend_hooks(LinuxBlockBackendHooks {
        try_complete_request: linux_virtio_blk_try_complete_request,
        submit_bio: linux_virtio_blk_submit_bio,
    });
    export_symbol_once(
        "__register_virtio_driver",
        __register_virtio_driver as usize,
        true,
    );
    export_symbol_once(
        "unregister_virtio_driver",
        unregister_virtio_driver as usize,
        true,
    );
    export_symbol_once(
        "virtio_check_driver_offered_feature",
        virtio_check_driver_offered_feature as usize,
        true,
    );
    export_symbol_once(
        "virtio_config_changed",
        virtio_config_changed as usize,
        true,
    );
    export_symbol_once(
        "virtio_config_driver_disable",
        virtio_config_driver_disable as usize,
        true,
    );
    export_symbol_once(
        "virtio_config_driver_enable",
        virtio_config_driver_enable as usize,
        true,
    );
    export_symbol_once("virtio_add_status", virtio_add_status as usize, true);
    export_symbol_once("virtio_reset_device", virtio_reset_device as usize, true);
    export_symbol_once(
        "virtio_device_reset_prepare",
        virtio_device_reset_prepare as usize,
        true,
    );
    export_symbol_once(
        "virtio_device_reset_done",
        virtio_device_reset_done as usize,
        true,
    );
    export_symbol_once("virtio_break_device", virtio_break_device as usize, true);
    export_symbol_once(
        "__virtio_unbreak_device",
        __virtio_unbreak_device as usize,
        true,
    );
    export_symbol_once("virtio_find_vqs", virtio_find_vqs as usize, true);
    export_symbol_once(
        "virtio_find_single_vq",
        virtio_find_single_vq as usize,
        true,
    );
    export_symbol_once("virtio_device_ready", virtio_device_ready as usize, true);
    export_symbol_once("virtio_max_dma_size", virtio_max_dma_size as usize, true);
    export_symbol_once(
        "vring_create_virtqueue",
        vring_create_virtqueue as usize,
        true,
    );
    export_symbol_once("vring_del_virtqueue", vring_del_virtqueue as usize, true);
    export_symbol_once("vring_interrupt", vring_interrupt as usize, true);
    export_symbol_once(
        "vring_notification_data",
        vring_notification_data as usize,
        true,
    );
    export_symbol_once(
        "vring_transport_features",
        vring_transport_features as usize,
        true,
    );
    export_symbol_once("virtqueue_add_sgs", virtqueue_add_sgs as usize, true);
    export_symbol_once("virtqueue_add_inbuf", virtqueue_add_inbuf as usize, true);
    export_symbol_once(
        "virtqueue_add_inbuf_cache_clean",
        virtqueue_add_inbuf_cache_clean as usize,
        true,
    );
    export_symbol_once(
        "virtqueue_add_inbuf_ctx",
        virtqueue_add_inbuf_ctx as usize,
        true,
    );
    export_symbol_once(
        "virtqueue_add_inbuf_premapped",
        virtqueue_add_inbuf_premapped as usize,
        true,
    );
    export_symbol_once("virtqueue_add_outbuf", virtqueue_add_outbuf as usize, true);
    export_symbol_once(
        "virtqueue_add_outbuf_premapped",
        virtqueue_add_outbuf_premapped as usize,
        true,
    );
    export_symbol_once(
        "virtqueue_kick_prepare",
        virtqueue_kick_prepare as usize,
        true,
    );
    export_symbol_once("virtqueue_notify", virtqueue_notify as usize, true);
    export_symbol_once("virtqueue_kick", virtqueue_kick as usize, true);
    export_symbol_once(
        "virtqueue_get_buf_ctx",
        virtqueue_get_buf_ctx as usize,
        true,
    );
    export_symbol_once("virtqueue_get_buf", virtqueue_get_buf as usize, true);
    export_symbol_once("virtqueue_disable_cb", virtqueue_disable_cb as usize, true);
    export_symbol_once(
        "virtqueue_enable_cb_prepare",
        virtqueue_enable_cb_prepare as usize,
        true,
    );
    export_symbol_once("virtqueue_poll", virtqueue_poll as usize, true);
    export_symbol_once("virtqueue_enable_cb", virtqueue_enable_cb as usize, true);
    export_symbol_once(
        "virtqueue_enable_cb_delayed",
        virtqueue_enable_cb_delayed as usize,
        true,
    );
    export_symbol_once(
        "virtqueue_detach_unused_buf",
        virtqueue_detach_unused_buf as usize,
        true,
    );
    export_symbol_once("virtqueue_resize", virtqueue_resize as usize, true);
    export_symbol_once("virtqueue_reset", virtqueue_reset as usize, true);
    export_symbol_once(
        "virtqueue_get_vring_size",
        virtqueue_get_vring_size as usize,
        true,
    );
    export_symbol_once("virtqueue_is_broken", virtqueue_is_broken as usize, true);
    export_symbol_once("virtqueue_dma_dev", virtqueue_dma_dev as usize, true);
    export_symbol_once(
        "virtqueue_get_desc_addr",
        virtqueue_get_desc_addr as usize,
        true,
    );
    export_symbol_once(
        "virtqueue_get_avail_addr",
        virtqueue_get_avail_addr as usize,
        true,
    );
    export_symbol_once(
        "virtqueue_get_used_addr",
        virtqueue_get_used_addr as usize,
        true,
    );
    export_symbol_once("__virtqueue_break", __virtqueue_break as usize, true);
    export_symbol_once("__virtqueue_unbreak", __virtqueue_unbreak as usize, true);
    export_symbol_once(
        "virtqueue_map_page_attrs",
        virtqueue_map_page_attrs as usize,
        true,
    );
    export_symbol_once(
        "virtqueue_unmap_page_attrs",
        virtqueue_unmap_page_attrs as usize,
        true,
    );
    export_symbol_once(
        "virtqueue_map_single_attrs",
        virtqueue_map_single_attrs as usize,
        true,
    );
    export_symbol_once(
        "virtqueue_unmap_single_attrs",
        virtqueue_unmap_single_attrs as usize,
        true,
    );
    export_symbol_once(
        "virtqueue_map_mapping_error",
        virtqueue_map_mapping_error as usize,
        true,
    );
    export_symbol_once(
        "virtqueue_map_need_sync",
        virtqueue_map_need_sync as usize,
        true,
    );
    export_symbol_once(
        "virtqueue_map_sync_single_range_for_cpu",
        virtqueue_map_sync_single_range_for_cpu as usize,
        true,
    );
    export_symbol_once(
        "virtqueue_map_sync_single_range_for_device",
        virtqueue_map_sync_single_range_for_device as usize,
        true,
    );
    export_symbol_once(
        "register_virtio_device",
        register_virtio_device as usize,
        true,
    );
    export_symbol_once(
        "unregister_virtio_device",
        unregister_virtio_device as usize,
        true,
    );
    export_symbol_once("is_virtio_device", is_virtio_device as usize, true);
}

/// `__register_virtio_driver` — `drivers/virtio/virtio.c:449`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __register_virtio_driver(driver: *mut c_void, _owner: *mut c_void) -> i32 {
    if driver.is_null() {
        return -EINVAL;
    }

    let driver = driver.cast::<LinuxVirtioDriver>();
    unsafe {
        if (*driver).feature_table_size != 0 && (*driver).feature_table.is_null() {
            return -EINVAL;
        }
        (*driver).driver.owner = _owner;
        (*driver).driver.bus = linux_virtio_bus_ptr();
    }
    register_linux_bus_type(linux_virtio_bus_ptr());

    unsafe { linux_driver_register(core::ptr::addr_of_mut!((*driver).driver)) }
}

/// `unregister_virtio_driver` — `drivers/virtio/virtio.c:460`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unregister_virtio_driver(driver: *mut c_void) {
    if driver.is_null() {
        return;
    }

    let driver = driver.cast::<LinuxVirtioDriver>();
    unsafe {
        linux_driver_unregister(core::ptr::addr_of_mut!((*driver).driver));
    }
}

/// `virtio_check_driver_offered_feature` — `vendor/linux/drivers/virtio/virtio.c:106`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtio_check_driver_offered_feature(dev: *const c_void, fbit: u32) {
    if dev.is_null() {
        return;
    }

    let Some(driver) = (unsafe { linux_virtio_driver_for_device(dev.cast_mut()) }) else {
        return;
    };
    if unsafe { !linux_virtio_driver_offered_feature(&*driver, fbit) } {
        unsafe {
            virtio_add_status(dev.cast_mut(), VIRTIO_CONFIG_S_FAILED as u32);
        }
    }
}

/// `virtio_config_changed` — `vendor/linux/drivers/virtio/virtio.c:138`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtio_config_changed(dev: *mut c_void) {
    unsafe {
        linux_virtio_config_changed_inner(dev);
    }
}

/// `virtio_config_driver_disable` — `vendor/linux/drivers/virtio/virtio.c:155`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtio_config_driver_disable(dev: *mut c_void) {
    if dev.is_null() {
        return;
    }
    unsafe {
        (*dev.cast::<LinuxVirtioDevicePrefix>()).config_driver_disabled = true;
    }
}

/// `virtio_config_driver_enable` — `vendor/linux/drivers/virtio/virtio.c:170`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtio_config_driver_enable(dev: *mut c_void) {
    if dev.is_null() {
        return;
    }
    let prefix = dev.cast::<LinuxVirtioDevicePrefix>();
    let pending = unsafe {
        (*prefix).config_driver_disabled = false;
        (*prefix).config_change_pending
    };
    if pending {
        unsafe {
            linux_virtio_config_changed_inner(dev);
        }
    }
}

/// `virtio_add_status` — `vendor/linux/drivers/virtio/virtio.c:196`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtio_add_status(dev: *mut c_void, status: u32) {
    let Some(config) = linux_virtio_config_ops(dev) else {
        return;
    };
    unsafe {
        let config = &*config;
        let (Some(get_status), Some(set_status)) = (config.get_status, config.set_status) else {
            return;
        };
        set_status(dev, get_status(dev) | status as u8);
    }
}

/// `virtio_reset_device` — `vendor/linux/drivers/virtio/virtio.c:253`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtio_reset_device(dev: *mut c_void) {
    let Some(config) = linux_virtio_config_ops(dev) else {
        return;
    };
    unsafe {
        let Some(reset) = (*config).reset else {
            return;
        };
        reset(dev);
    }
}

/// `virtio_device_reset_prepare` - `vendor/linux/drivers/virtio/virtio.c:676`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtio_device_reset_prepare(_dev: *mut c_void) -> i32 {
    -EOPNOTSUPP
}

/// `virtio_device_reset_done` - `vendor/linux/drivers/virtio/virtio.c:698`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtio_device_reset_done(_dev: *mut c_void) -> i32 {
    -EOPNOTSUPP
}

/// `virtio_find_vqs` — `vendor/linux/include/linux/virtio_config.h:293`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtio_find_vqs(
    dev: *mut c_void,
    nvqs: u32,
    vqs: *mut *mut LinuxVirtqueue,
    vqs_info: *mut LinuxVirtqueueInfo,
    desc: *mut c_void,
) -> i32 {
    if dev.is_null() || (nvqs != 0 && vqs.is_null()) {
        return -EINVAL;
    }
    let Some(config) = linux_virtio_config_ops(dev) else {
        return -EINVAL;
    };
    let Some(find_vqs) = (unsafe { (*config).find_vqs }) else {
        return -EINVAL;
    };

    let ret = unsafe { find_vqs(dev, nvqs, vqs.cast(), vqs_info.cast(), desc) };
    if ret == 0 {
        let mut idx = 0u32;
        while idx < nvqs {
            let vq = unsafe { *vqs.add(idx as usize) };
            if !vq.is_null() {
                unsafe {
                    if (*vq).vdev.is_null() {
                        (*vq).vdev = dev.cast::<LinuxVirtioDevice>();
                    }
                    (*vq).index = idx;
                }
            }
            idx += 1;
        }
    }
    ret
}

/// `virtio_find_single_vq` — `vendor/linux/include/linux/virtio_config.h:302`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtio_find_single_vq(
    dev: *mut c_void,
    callback: LinuxVirtqueueCallback,
    name: *const c_char,
) -> *mut LinuxVirtqueue {
    let mut vq = core::ptr::null_mut::<LinuxVirtqueue>();
    let mut info = LinuxVirtqueueInfo {
        name,
        callback,
        ctx: false,
    };
    let err = unsafe { virtio_find_vqs(dev, 1, &mut vq, &mut info, core::ptr::null_mut()) };
    if err < 0 {
        linux_virtqueue_err_ptr(err)
    } else {
        vq
    }
}

/// `virtio_device_ready` — `vendor/linux/include/linux/virtio_config.h:344`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtio_device_ready(dev: *mut c_void) {
    let Some(config) = linux_virtio_config_ops(dev) else {
        return;
    };
    unsafe {
        let config = &*config;
        let (Some(get_status), Some(set_status)) = (config.get_status, config.set_status) else {
            return;
        };
        set_status(dev, get_status(dev) | VIRTIO_CONFIG_S_DRIVER_OK);
    }
}

/// `virtio_max_dma_size` — `vendor/linux/drivers/virtio/virtio_ring.c:359`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtio_max_dma_size(_dev: *const c_void) -> usize {
    usize::MAX
}

/// `vring_create_virtqueue` - `vendor/linux/drivers/virtio/virtio_ring.c:3260`.
/// This is Linux `virtio_ring.c` core ABI glue. The actual PCI/MMIO transport
/// driver is still a Linux-built module; this entry point only materializes the
/// vring object after that transport calls into the exported Linux helper.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vring_create_virtqueue(
    index: u32,
    num: u32,
    _vring_align: u32,
    vdev: *mut LinuxVirtioDevice,
    _weak_barriers: bool,
    _may_reduce_num: bool,
    _context: bool,
    notify: LinuxVirtqueueNotify,
    callback: LinuxVirtqueueCallback,
    name: *const c_char,
) -> *mut LinuxVirtqueue {
    if vdev.is_null() || num == 0 {
        crate::log_warn!(
            "virtio",
            "vring_create_virtqueue rejected index={} num={} vdev={:p}",
            index,
            num,
            vdev
        );
        return core::ptr::null_mut();
    }
    let Some(layout) = linux_vring_split_layout(num, _vring_align) else {
        crate::log_warn!(
            "virtio",
            "vring_create_virtqueue invalid layout index={} num={} align={}",
            index,
            num,
            _vring_align
        );
        return core::ptr::null_mut();
    };
    let Some((ring_cpu, ring_dma)) = dma_alloc_coherent(layout.total_size) else {
        crate::log_warn!(
            "virtio",
            "vring_create_virtqueue dma alloc failed index={} bytes={}",
            index,
            layout.total_size
        );
        return core::ptr::null_mut();
    };
    let num_u16 = match u16::try_from(num) {
        Ok(num) => num,
        Err(_) => {
            unsafe {
                dma_free_coherent(ring_cpu, layout.total_size);
            }
            crate::log_warn!(
                "virtio",
                "vring_create_virtqueue queue too large index={} num={}",
                index,
                num
            );
            return core::ptr::null_mut();
        }
    };

    let vq = Box::new(LinuxVirtqueue {
        list: LinuxListHead {
            next: core::ptr::null_mut(),
            prev: core::ptr::null_mut(),
        },
        callback,
        name,
        vdev,
        index,
        num_free: num_u16 as u32,
        num_max: num_u16 as u32,
        reset: false,
        priv_: core::ptr::null_mut(),
    });
    let vq = Box::into_raw(vq);
    unsafe {
        linux_list_add_tail(
            core::ptr::addr_of_mut!((*vq).list),
            core::ptr::addr_of_mut!((*vdev).vqs),
        );
    }
    linux_virtqueue_register_backend(vq, notify, ring_cpu, ring_dma, layout.total_size, layout);
    vq
}

/// `vring_del_virtqueue` - `vendor/linux/drivers/virtio/virtio_ring.c:3473`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vring_del_virtqueue(vq: *mut LinuxVirtqueue) {
    if !vq.is_null() {
        unsafe {
            linux_list_del_init(core::ptr::addr_of_mut!((*vq).list));
            linux_virtqueue_remove_backend(vq);
            drop(Box::from_raw(vq));
        }
    }
}

/// `vring_interrupt` - `vendor/linux/drivers/virtio/virtio_ring.c:3234`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vring_interrupt(_irq: i32, vq: *mut c_void) -> i32 {
    if vq.is_null() {
        return 0;
    }
    let vq = vq.cast::<LinuxVirtqueue>();
    let Some(callback) = (unsafe { (*vq).callback }) else {
        return 0;
    };
    unsafe {
        callback(vq.cast::<c_void>());
    }
    1
}

/// `vring_notification_data` - `vendor/linux/drivers/virtio/virtio_ring.c:3487`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vring_notification_data(vq: *mut LinuxVirtqueue) -> u32 {
    if vq.is_null() {
        0
    } else {
        unsafe { (*vq).index }
    }
}

/// `vring_transport_features` - `vendor/linux/drivers/virtio/virtio_ring.c:3505`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vring_transport_features(_vdev: *mut LinuxVirtioDevice) {}

/// `virtqueue_add_sgs` — `vendor/linux/drivers/virtio/virtio_ring.c:2819`.
/// Exposes descriptors for a virtqueue created by Linux transport code.
///
/// Linux-built function drivers normally own request construction. The block
/// ABI also uses this for a guarded synchronous virtio-blk fast path after the
/// Linux driver has discovered the device and registered the gendisk.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_add_sgs(
    vq: *mut LinuxVirtqueue,
    sgs: *mut *mut c_void,
    out_sgs: u32,
    in_sgs: u32,
    data: *mut c_void,
    _gfp: u32,
) -> i32 {
    if vq.is_null() || data.is_null() {
        crate::log_warn!(
            "virtio",
            "virtqueue_add_sgs rejected null pointer vq={:p} data={:p}",
            vq,
            data
        );
        return -EINVAL;
    }
    if unsafe { (*vq).reset } {
        crate::log_warn!(
            "virtio",
            "virtqueue_add_sgs rejected broken queue index={}",
            unsafe { (*vq).index }
        );
        return -EIO;
    }
    if linux_virtqueue_with_backend_mut(vq, |_| ()).is_none() {
        crate::log_warn!(
            "virtio",
            "virtqueue_add_sgs missing backend for queue index={}",
            unsafe { (*vq).index }
        );
        return -ENODEV;
    }
    let total_lists = out_sgs.saturating_add(in_sgs);
    if total_lists > 0 && sgs.is_null() {
        crate::log_warn!(
            "virtio",
            "virtqueue_add_sgs rejected null sg array queue index={} out={} in={}",
            unsafe { (*vq).index },
            out_sgs,
            in_sgs
        );
        return -EINVAL;
    }
    let mut entries = Vec::new();
    for list_idx in 0..total_lists {
        let writable = list_idx >= out_sgs;
        let mut sg = unsafe { *sgs.add(list_idx as usize) }
            .cast::<crate::lib::scatterlist::LinuxScatterList>();
        let mut seen = 0u32;
        while !sg.is_null() && seen < 1024 {
            let len = unsafe {
                if (*sg).dma_length != 0 {
                    (*sg).dma_length
                } else {
                    (*sg).length
                }
            };
            let cpu_addr = unsafe { (*sg).dma_address as *const u8 };
            if len == 0 || cpu_addr.is_null() {
                crate::log_warn!(
                    "virtio",
                    "virtqueue_add_sgs rejected sg list={} len={} cpu={:p}",
                    list_idx,
                    len,
                    cpu_addr
                );
                return -EINVAL;
            }
            let dma = dma_map_single(
                cpu_addr,
                len as usize,
                if writable {
                    DmaDirection::FromDevice
                } else {
                    DmaDirection::ToDevice
                },
            );
            let dma = if dma == 0 {
                unsafe { (*sg).dma_address as u64 }
            } else {
                dma
            };
            if dma == 0 {
                crate::log_warn!(
                    "virtio",
                    "virtqueue_add_sgs dma map failed list={} len={} cpu={:p}",
                    list_idx,
                    len,
                    cpu_addr
                );
                return -EINVAL;
            }
            entries.push((dma, len, writable));

            let flags = unsafe { (*sg).page_link & crate::lib::scatterlist::SG_PAGE_LINK_MASK };
            seen = seen.saturating_add(1);
            if flags & crate::lib::scatterlist::SG_END != 0 {
                break;
            }
            if flags & crate::lib::scatterlist::SG_CHAIN != 0 {
                let next = unsafe { (*sg).page_link & !crate::lib::scatterlist::SG_PAGE_LINK_MASK };
                sg = next as *mut crate::lib::scatterlist::LinuxScatterList;
            } else {
                sg = unsafe { sg.add(1) };
            }
        }
    }
    if entries.is_empty() {
        crate::log_warn!(
            "virtio",
            "virtqueue_add_sgs rejected empty sg set queue index={} out={} in={}",
            unsafe { (*vq).index },
            out_sgs,
            in_sgs
        );
        return -EINVAL;
    }

    linux_virtqueue_with_backend_mut(vq, |backend| {
        if !backend.ring_ready {
            crate::log_warn!(
                "virtio",
                "virtqueue_add_sgs queue index={} ring backing not ready",
                unsafe { (*vq).index }
            );
            return -ENODEV;
        }
        if entries.len() > backend.free_list.len() {
            crate::log_warn!(
                "virtio",
                "virtqueue_add_sgs queue index={} needs {} descriptors, free {}",
                unsafe { (*vq).index },
                entries.len(),
                backend.free_list.len()
            );
            return -ENOSPC;
        }
        let mut descriptors = Vec::with_capacity(entries.len());
        for _ in 0..entries.len() {
            descriptors.push(
                backend
                    .free_list
                    .pop()
                    .expect("free_list length checked before descriptor allocation"),
            );
        }
        for (pos, (dma, len, writable)) in entries.iter().enumerate() {
            let mut flags = if *writable { VRING_DESC_F_WRITE } else { 0 };
            let next = if pos + 1 < descriptors.len() {
                flags |= VRING_DESC_F_NEXT;
                descriptors[pos + 1]
            } else {
                0
            };
            unsafe {
                linux_vring_write_desc(
                    backend,
                    descriptors[pos],
                    LinuxVringDesc {
                        addr: *dma,
                        len: *len,
                        flags,
                        next,
                    },
                );
            }
        }
        let head = descriptors[0];
        let slot = backend.avail_idx_shadow as usize % unsafe { (*vq).num_max as usize };
        unsafe {
            linux_vring_write_avail_ring(backend, slot, head);
        }
        core::sync::atomic::fence(Ordering::SeqCst);
        backend.avail_idx_shadow = backend.avail_idx_shadow.wrapping_add(1);
        unsafe {
            linux_vring_write_avail_idx(backend, backend.avail_idx_shadow);
        }
        unsafe {
            (*vq).num_free -= descriptors.len() as u32;
        }
        backend.submitted.push(LinuxVirtqueueToken {
            data: data as usize,
            len: 0,
            ctx: 0,
            head,
            descriptors,
        });
        backend.pending_notify = true;
        0
    })
    .unwrap_or(-ENODEV)
}

/// `virtqueue_add_inbuf` — `vendor/linux/drivers/virtio/virtio_ring.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_add_inbuf(
    vq: *mut LinuxVirtqueue,
    sg: *mut c_void,
    _num: u32,
    data: *mut c_void,
    gfp: u32,
) -> i32 {
    let mut sg = sg;
    unsafe { virtqueue_add_sgs(vq, core::ptr::addr_of_mut!(sg), 0, 1, data, gfp) }
}

/// `virtqueue_add_inbuf_cache_clean` — `vendor/linux/drivers/virtio/virtio_ring.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_add_inbuf_cache_clean(
    vq: *mut LinuxVirtqueue,
    sg: *mut c_void,
    num: u32,
    data: *mut c_void,
    gfp: u32,
) -> i32 {
    unsafe { virtqueue_add_inbuf(vq, sg, num, data, gfp) }
}

/// `virtqueue_add_inbuf_ctx` — `vendor/linux/drivers/virtio/virtio_ring.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_add_inbuf_ctx(
    vq: *mut LinuxVirtqueue,
    sg: *mut c_void,
    num: u32,
    data: *mut c_void,
    _ctx: *mut c_void,
    gfp: u32,
) -> i32 {
    unsafe { virtqueue_add_inbuf(vq, sg, num, data, gfp) }
}

/// `virtqueue_add_inbuf_premapped` — `vendor/linux/drivers/virtio/virtio_ring.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_add_inbuf_premapped(
    vq: *mut LinuxVirtqueue,
    sg: *mut c_void,
    num: u32,
    data: *mut c_void,
    _ctx: *mut c_void,
    gfp: u32,
) -> i32 {
    unsafe { virtqueue_add_inbuf(vq, sg, num, data, gfp) }
}

/// `virtqueue_add_outbuf` — `vendor/linux/drivers/virtio/virtio_ring.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_add_outbuf(
    vq: *mut LinuxVirtqueue,
    sg: *mut c_void,
    _num: u32,
    data: *mut c_void,
    gfp: u32,
) -> i32 {
    let mut sg = sg;
    unsafe { virtqueue_add_sgs(vq, core::ptr::addr_of_mut!(sg), 1, 0, data, gfp) }
}

/// `virtqueue_add_outbuf_premapped` — `vendor/linux/drivers/virtio/virtio_ring.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_add_outbuf_premapped(
    vq: *mut LinuxVirtqueue,
    sg: *mut c_void,
    num: u32,
    data: *mut c_void,
    gfp: u32,
) -> i32 {
    unsafe { virtqueue_add_outbuf(vq, sg, num, data, gfp) }
}

/// `virtqueue_map_page_attrs` — `vendor/linux/drivers/virtio/virtio_ring.c:3776`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_map_page_attrs(
    _vq: *const LinuxVirtqueue,
    page: *mut Page,
    offset: usize,
    size: usize,
    dir: DmaDirection,
    _attrs: usize,
) -> DmaAddr {
    if page.is_null() || size == 0 {
        return 0;
    }
    let cpu = unsafe { pfn_to_virt(page_to_pfn(page)).add(offset) };
    dma_map_single(cpu, size, dir)
}

/// `virtqueue_unmap_page_attrs` — `vendor/linux/drivers/virtio/virtio_ring.c:3794`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_unmap_page_attrs(
    _vq: *const LinuxVirtqueue,
    _addr: DmaAddr,
    _size: usize,
    _dir: DmaDirection,
    _attrs: usize,
) {
}

/// `virtqueue_map_single_attrs` — `vendor/linux/drivers/virtio/virtio_ring.c:3819`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_map_single_attrs(
    _vq: *const LinuxVirtqueue,
    ptr: *mut c_void,
    size: usize,
    dir: DmaDirection,
    _attrs: usize,
) -> DmaAddr {
    dma_map_single(ptr.cast::<u8>(), size, dir)
}

/// `virtqueue_unmap_single_attrs` — `vendor/linux/drivers/virtio/virtio_ring.c:3854`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_unmap_single_attrs(
    _vq: *const LinuxVirtqueue,
    _addr: DmaAddr,
    _size: usize,
    _dir: DmaDirection,
    _attrs: usize,
) {
}

/// `virtqueue_map_mapping_error` — `vendor/linux/drivers/virtio/virtio_ring.c:3882`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_map_mapping_error(
    _vq: *const LinuxVirtqueue,
    addr: DmaAddr,
) -> i32 {
    i32::from(addr == 0)
}

/// `virtqueue_map_need_sync` — `vendor/linux/drivers/virtio/virtio_ring.c:3896`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_map_need_sync(
    _vq: *const LinuxVirtqueue,
    _addr: DmaAddr,
) -> bool {
    false
}

/// `virtqueue_map_sync_single_range_for_cpu` — `vendor/linux/drivers/virtio/virtio_ring.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_map_sync_single_range_for_cpu(
    _vq: *const LinuxVirtqueue,
    _addr: DmaAddr,
    _offset: usize,
    _size: usize,
    _dir: DmaDirection,
) {
}

/// `virtqueue_map_sync_single_range_for_device` — `vendor/linux/drivers/virtio/virtio_ring.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_map_sync_single_range_for_device(
    _vq: *const LinuxVirtqueue,
    _addr: DmaAddr,
    _offset: usize,
    _size: usize,
    _dir: DmaDirection,
) {
}

/// `virtqueue_kick_prepare` — `vendor/linux/drivers/virtio/virtio_ring.c:3012`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_kick_prepare(vq: *mut LinuxVirtqueue) -> bool {
    linux_virtqueue_with_backend_mut(vq, |backend| {
        if !backend.ring_ready || backend.submitted.is_empty() {
            return false;
        }
        core::sync::atomic::fence(Ordering::SeqCst);
        let pending = backend.pending_notify;
        backend.pending_notify = false;
        pending
    })
    .unwrap_or(false)
}

/// `virtqueue_notify` — `vendor/linux/drivers/virtio/virtio_ring.c:3028`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_notify(vq: *mut LinuxVirtqueue) -> bool {
    if vq.is_null() || unsafe { (*vq).reset } {
        return false;
    }
    let notify = linux_virtqueue_with_backend_mut(vq, |backend| {
        if backend.ring_ready {
            backend.notify
        } else {
            None
        }
    })
    .flatten();
    let Some(notify) = notify else {
        return false;
    };
    let ok = unsafe { notify(vq) };
    if !ok {
        unsafe {
            (*vq).reset = true;
        }
    }
    ok
}

/// `virtqueue_kick` — `vendor/linux/drivers/virtio/virtio_ring.c:3056`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_kick(vq: *mut LinuxVirtqueue) -> bool {
    if unsafe { virtqueue_kick_prepare(vq) } {
        unsafe { virtqueue_notify(vq) }
    } else {
        linux_virtqueue_with_backend_mut(vq, |backend| backend.ring_ready).unwrap_or(false)
            && !unsafe { (*vq).reset }
    }
}

/// `virtqueue_get_buf_ctx` — `vendor/linux/drivers/virtio/virtio_ring.c:3081`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_get_buf_ctx(
    vq: *mut LinuxVirtqueue,
    len: *mut u32,
    ctx: *mut *mut c_void,
) -> *mut c_void {
    if let Some(token) = linux_virtqueue_with_backend_mut(vq, |backend| {
        if !backend.ring_ready {
            return None;
        }
        let used_idx = unsafe { linux_vring_used_idx(backend) };
        if backend.last_used_idx as u16 == used_idx {
            return None;
        }
        core::sync::atomic::fence(Ordering::SeqCst);
        let slot = backend.last_used_idx as usize % unsafe { (*vq).num_max as usize };
        let elem = unsafe { linux_vring_used_elem(backend, slot) };
        let Some(pos) = backend
            .submitted
            .iter()
            .position(|token| token.head as u32 == elem.id)
        else {
            unsafe {
                (*vq).reset = true;
            }
            return None;
        };
        let mut token = backend.submitted.remove(pos);
        token.len = elem.len;
        backend.last_used_idx = backend.last_used_idx.wrapping_add(1);
        for desc in token.descriptors.iter().copied() {
            unsafe {
                linux_vring_write_desc(backend, desc, LinuxVringDesc::default());
            }
            backend.free_list.push(desc);
        }
        Some(token)
    })
    .flatten()
    {
        unsafe {
            let new_free = (*vq)
                .num_free
                .saturating_add(token.descriptors.len() as u32)
                .min((*vq).num_max);
            (*vq).num_free = new_free;
            if !len.is_null() {
                *len = token.len;
            }
            if !ctx.is_null() {
                *ctx = token.ctx as *mut c_void;
            }
        }
        return token.data as *mut c_void;
    }
    unsafe {
        if !len.is_null() {
            *len = 0;
        }
        if !ctx.is_null() {
            *ctx = core::ptr::null_mut();
        }
    }
    core::ptr::null_mut()
}

/// `virtqueue_get_buf` — `vendor/linux/drivers/virtio/virtio_ring.c:3090`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_get_buf(vq: *mut LinuxVirtqueue, len: *mut u32) -> *mut c_void {
    unsafe { virtqueue_get_buf_ctx(vq, len, core::ptr::null_mut()) }
}

/// `virtqueue_disable_cb` — `vendor/linux/drivers/virtio/virtio_ring.c:3104`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_disable_cb(vq: *mut LinuxVirtqueue) {
    let _ = linux_virtqueue_with_backend_mut(vq, |backend| {
        backend.callbacks_enabled = false;
    });
}

/// `virtqueue_enable_cb_prepare` — `vendor/linux/drivers/virtio/virtio_ring.c:3124`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_enable_cb_prepare(vq: *mut LinuxVirtqueue) -> u32 {
    linux_virtqueue_with_backend_mut(vq, |backend| {
        backend.callbacks_enabled = true;
        backend.last_used_idx
    })
    .unwrap_or(0)
}

/// `virtqueue_poll` — `vendor/linux/drivers/virtio/virtio_ring.c:3144`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_poll(vq: *mut LinuxVirtqueue, last_used_idx: u32) -> bool {
    linux_virtqueue_with_backend_mut(vq, |backend| {
        if !backend.ring_ready {
            return false;
        }
        unsafe { linux_vring_used_idx(backend) as u32 != last_used_idx }
    })
    .unwrap_or(false)
}

/// `virtqueue_enable_cb` — `vendor/linux/drivers/virtio/virtio_ring.c:3168`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_enable_cb(vq: *mut LinuxVirtqueue) -> bool {
    linux_virtqueue_with_backend_mut(vq, |backend| {
        backend.callbacks_enabled = true;
        if !backend.ring_ready {
            return true;
        }
        unsafe { linux_vring_used_idx(backend) as u32 == backend.last_used_idx }
    })
    .unwrap_or(true)
}

/// `virtqueue_enable_cb_delayed` — `vendor/linux/drivers/virtio/virtio_ring.c:3189`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_enable_cb_delayed(vq: *mut LinuxVirtqueue) -> bool {
    unsafe { virtqueue_enable_cb(vq) }
}

/// `virtqueue_detach_unused_buf` — `vendor/linux/drivers/virtio/virtio_ring.c:3208`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_detach_unused_buf(vq: *mut LinuxVirtqueue) -> *mut c_void {
    linux_virtqueue_with_backend_mut(vq, |backend| backend.submitted.pop())
        .flatten()
        .map(|token| {
            for desc in token.descriptors.iter().copied() {
                let _ = linux_virtqueue_with_backend_mut(vq, |backend| {
                    unsafe {
                        linux_vring_write_desc(backend, desc, LinuxVringDesc::default());
                    }
                    backend.free_list.push(desc);
                });
            }
            unsafe {
                if !vq.is_null() {
                    (*vq).num_free = (*vq)
                        .num_free
                        .saturating_add(token.descriptors.len() as u32)
                        .min((*vq).num_max);
                }
            }
            token.data as *mut c_void
        })
        .unwrap_or(core::ptr::null_mut())
}

/// `virtqueue_resize` — `vendor/linux/drivers/virtio/virtio_ring.c:3379`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_resize(
    vq: *mut LinuxVirtqueue,
    num: u32,
    _recycle: LinuxVirtqueueRecycle,
    _recycle_done: LinuxVirtqueueRecycleDone,
) -> i32 {
    if vq.is_null() || num == 0 {
        return -EINVAL;
    }
    if num > unsafe { (*vq).num_max } {
        return -E2BIG;
    }
    if !has_virtqueue_backend(vq) {
        return -ENODEV;
    }
    0
}

/// `virtqueue_reset` — `vendor/linux/drivers/virtio/virtio_ring.c:3411`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_reset(
    vq: *mut LinuxVirtqueue,
    recycle: LinuxVirtqueueRecycle,
    recycle_done: LinuxVirtqueueRecycleDone,
) -> i32 {
    if vq.is_null() {
        return -EINVAL;
    }

    let Some(tokens) = linux_virtqueue_with_backend_mut(vq, |backend| {
        if !backend.ring_ready {
            return Err(-ENODEV);
        }
        let tokens = core::mem::take(&mut backend.submitted);
        let num = unsafe { (*vq).num_max as u16 };
        backend.free_list.clear();
        for idx in (0..num).rev() {
            backend.free_list.push(idx);
        }
        for idx in 0..num {
            unsafe {
                linux_vring_write_desc(backend, idx, LinuxVringDesc::default());
            }
        }
        unsafe {
            linux_vring_write_avail_idx(backend, 0);
            linux_vring_set_used_idx(backend, 0);
            (*vq).num_free = (*vq).num_max;
            (*vq).reset = false;
        }
        backend.avail_idx_shadow = 0;
        backend.last_used_idx = 0;
        backend.callbacks_enabled = true;
        backend.pending_notify = false;
        Ok(tokens)
    }) else {
        return -ENODEV;
    };
    let tokens = match tokens {
        Ok(tokens) => tokens,
        Err(err) => return err,
    };

    if let Some(recycle) = recycle {
        for token in tokens {
            unsafe {
                recycle(vq, token.data as *mut c_void);
            }
        }
    }
    if let Some(recycle_done) = recycle_done {
        unsafe {
            recycle_done(vq);
        }
    }
    0
}

/// `virtqueue_get_vring_size` — `vendor/linux/drivers/virtio/virtio_ring.c:3542`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_get_vring_size(vq: *const LinuxVirtqueue) -> u32 {
    if vq.is_null() {
        0
    } else {
        unsafe { (*vq).num_max }
    }
}

/// `virtqueue_is_broken` — `vendor/linux/drivers/virtio/virtio_ring.c:3576`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_is_broken(vq: *const LinuxVirtqueue) -> bool {
    !vq.is_null() && unsafe { (*vq).reset }
}

/// `virtqueue_dma_dev` — `vendor/linux/drivers/virtio/virtio_ring.c:2990`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_dma_dev(_vq: *mut LinuxVirtqueue) -> *mut LinuxDevice {
    core::ptr::null_mut()
}

/// `__virtqueue_break` — `vendor/linux/drivers/virtio/virtio_ring.c:3555`.
/// `virtqueue_get_desc_addr` - `vendor/linux/drivers/virtio/virtio_ring.c:3625`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_get_desc_addr(vq: *const LinuxVirtqueue) -> u64 {
    linux_virtqueue_with_backend_mut(vq.cast_mut(), |backend| {
        if backend.ring_ready {
            linux_vring_desc_dma(backend)
        } else {
            0
        }
    })
    .unwrap_or(0)
}

/// `virtqueue_get_avail_addr` - `vendor/linux/drivers/virtio/virtio_ring.c:3639`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_get_avail_addr(vq: *const LinuxVirtqueue) -> u64 {
    linux_virtqueue_with_backend_mut(vq.cast_mut(), |backend| {
        if backend.ring_ready {
            linux_vring_avail_dma(backend)
        } else {
            0
        }
    })
    .unwrap_or(0)
}

/// `virtqueue_get_used_addr` - `vendor/linux/drivers/virtio/virtio_ring.c:3653`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtqueue_get_used_addr(vq: *const LinuxVirtqueue) -> u64 {
    linux_virtqueue_with_backend_mut(vq.cast_mut(), |backend| {
        if backend.ring_ready {
            linux_vring_used_dma(backend)
        } else {
            0
        }
    })
    .unwrap_or(0)
}

/// `virtio_break_device` - `vendor/linux/drivers/virtio/virtio_ring.c:3588`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn virtio_break_device(_dev: *mut c_void) {}

/// `__virtio_unbreak_device` - `vendor/linux/drivers/virtio/virtio_ring.c:3610`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __virtio_unbreak_device(_dev: *mut c_void) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __virtqueue_break(vq: *mut LinuxVirtqueue) {
    if !vq.is_null() {
        unsafe {
            (*vq).reset = true;
        }
    }
}

/// `__virtqueue_unbreak` — `vendor/linux/drivers/virtio/virtio_ring.c:3567`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __virtqueue_unbreak(vq: *mut LinuxVirtqueue) {
    if !vq.is_null() {
        unsafe {
            (*vq).reset = false;
        }
    }
}

/// `register_virtio_device` — `vendor/linux/drivers/virtio/virtio.c:517`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn register_virtio_device(dev: *mut c_void) -> i32 {
    if dev.is_null() {
        return -EINVAL;
    }

    let prefix = dev.cast::<LinuxVirtioDevicePrefix>();
    let embedded = unsafe { linux_virtio_embedded_device(dev) };
    if linux_virtio_config_ops(dev).is_none() {
        unsafe {
            (*prefix).failed = true;
        }
        return -EINVAL;
    }
    if linux_virtio_device_id(dev).is_none() {
        unsafe {
            (*prefix).failed = true;
        }
        return -EINVAL;
    }
    let index = NEXT_LINUX_VIRTIO_INDEX.fetch_add(1, Ordering::AcqRel);

    unsafe {
        (*prefix).index = index;
        (*prefix).failed = false;
        (*prefix).config_core_enabled = false;
        (*prefix).config_driver_disabled = false;
        (*prefix).config_change_pending = false;
        linux_list_head_init(core::ptr::addr_of_mut!(
            (*dev.cast::<LinuxVirtioDevice>()).vqs
        ));
        (*embedded).bus = linux_virtio_bus_ptr();
        linux_device_initialize(embedded);
        if linux_device_set_name_index(embedded, b"virtio", index).is_err() {
            (*prefix).failed = true;
            return -EINVAL;
        }
    }
    register_linux_bus_type(linux_virtio_bus_ptr());

    unsafe {
        virtio_reset_device(dev);
        virtio_add_status(dev, VIRTIO_CONFIG_S_ACKNOWLEDGE as u32);
    }

    let ret = unsafe { linux_device_add(embedded) };
    if ret != 0 {
        unsafe {
            (*prefix).failed = true;
            virtio_add_status(dev, VIRTIO_CONFIG_S_FAILED as u32);
        }
    }
    ret
}

/// `unregister_virtio_device` — `vendor/linux/drivers/virtio/virtio.c:581`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unregister_virtio_device(dev: *mut c_void) {
    if dev.is_null() {
        return;
    }

    let embedded = unsafe { linux_virtio_embedded_device(dev) };
    unsafe {
        linux_device_unregister(embedded);
    }
}

/// `is_virtio_device` — `vendor/linux/drivers/virtio/virtio.c:575`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn is_virtio_device(dev: *mut c_void) -> bool {
    if dev.is_null() {
        return false;
    }

    unsafe { (*dev.cast::<LinuxDevice>()).bus == linux_virtio_bus_ptr() }
}

/// `__register_virtio_driver` — `drivers/virtio/virtio.c:449`.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtio_pci_id_decode_matches_linux_modern_and_legacy_rules() {
        assert_eq!(
            virtio_device_id_from_pci_ids(
                PCI_VENDOR_ID_VIRTIO,
                VIRTIO_PCI_MODERN_DEVICE_ID_BASE + VIRTIO_ID_BLOCK as u16,
                0,
            ),
            Some(VIRTIO_ID_BLOCK)
        );
        assert_eq!(
            virtio_device_id_from_pci_ids(PCI_VENDOR_ID_VIRTIO, 0x1001, VIRTIO_ID_NET as u16),
            Some(VIRTIO_ID_NET)
        );
        assert_eq!(
            virtio_device_id_from_pci_ids(0x8086, VIRTIO_PCI_MODERN_DEVICE_ID_BASE, 0),
            None
        );
    }

    #[test]
    fn linux_virtio_core_exports_register_for_modules() {
        register_module_exports();

        assert_eq!(
            crate::kernel::module::find_symbol("__register_virtio_driver"),
            Some(__register_virtio_driver as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("unregister_virtio_driver"),
            Some(unregister_virtio_driver as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("virtio_check_driver_offered_feature"),
            Some(virtio_check_driver_offered_feature as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("virtio_config_changed"),
            Some(virtio_config_changed as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("virtio_config_driver_disable"),
            Some(virtio_config_driver_disable as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("virtio_config_driver_enable"),
            Some(virtio_config_driver_enable as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("virtio_add_status"),
            Some(virtio_add_status as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("virtio_reset_device"),
            Some(virtio_reset_device as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("virtio_device_reset_prepare"),
            Some(virtio_device_reset_prepare as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("virtio_device_reset_done"),
            Some(virtio_device_reset_done as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("virtio_break_device"),
            Some(virtio_break_device as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("__virtio_unbreak_device"),
            Some(__virtio_unbreak_device as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("virtio_find_vqs"),
            Some(virtio_find_vqs as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("virtio_find_single_vq"),
            Some(virtio_find_single_vq as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("virtio_device_ready"),
            Some(virtio_device_ready as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("virtio_max_dma_size"),
            Some(virtio_max_dma_size as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("vring_create_virtqueue"),
            Some(vring_create_virtqueue as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("vring_del_virtqueue"),
            Some(vring_del_virtqueue as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("vring_interrupt"),
            Some(vring_interrupt as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("vring_notification_data"),
            Some(vring_notification_data as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("vring_transport_features"),
            Some(vring_transport_features as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("virtqueue_add_sgs"),
            Some(virtqueue_add_sgs as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("virtqueue_get_buf"),
            Some(virtqueue_get_buf as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("virtqueue_kick"),
            Some(virtqueue_kick as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("virtqueue_get_desc_addr"),
            Some(virtqueue_get_desc_addr as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("virtqueue_get_avail_addr"),
            Some(virtqueue_get_avail_addr as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("virtqueue_get_used_addr"),
            Some(virtqueue_get_used_addr as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("register_virtio_device"),
            Some(register_virtio_device as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("unregister_virtio_device"),
            Some(unregister_virtio_device as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("is_virtio_device"),
            Some(is_virtio_device as usize)
        );
    }

    #[test]
    fn linux_virtio_driver_c_layout_matches_vendor_headers() {
        use core::mem::{offset_of, size_of};

        assert_eq!(offset_of!(LinuxVirtioConfigOps, get), 0);
        assert_eq!(offset_of!(LinuxVirtioConfigOps, set), 8);
        assert_eq!(offset_of!(LinuxVirtioConfigOps, generation), 16);
        assert_eq!(offset_of!(LinuxVirtioConfigOps, get_status), 24);
        assert_eq!(offset_of!(LinuxVirtioConfigOps, set_status), 32);
        assert_eq!(offset_of!(LinuxVirtioConfigOps, reset), 40);
        assert_eq!(offset_of!(LinuxVirtioConfigOps, find_vqs), 48);
        assert_eq!(offset_of!(LinuxVirtioConfigOps, del_vqs), 56);
        assert_eq!(offset_of!(LinuxVirtioConfigOps, synchronize_cbs), 64);
        assert_eq!(offset_of!(LinuxVirtioConfigOps, get_features), 72);
        assert_eq!(offset_of!(LinuxVirtioConfigOps, get_extended_features), 80);
        assert_eq!(offset_of!(LinuxVirtioConfigOps, finalize_features), 88);
        assert_eq!(offset_of!(LinuxVirtioConfigOps, bus_name), 96);
        assert_eq!(offset_of!(LinuxVirtioConfigOps, set_vq_affinity), 104);
        assert_eq!(offset_of!(LinuxVirtioConfigOps, get_vq_affinity), 112);
        assert_eq!(offset_of!(LinuxVirtioConfigOps, get_shm_region), 120);
        assert_eq!(offset_of!(LinuxVirtioConfigOps, disable_vq_and_reset), 128);
        assert_eq!(offset_of!(LinuxVirtioConfigOps, enable_vq_after_reset), 136);
        assert_eq!(size_of::<LinuxVirtioConfigOps>(), 144);

        assert_eq!(offset_of!(LinuxVirtqueueInfo, name), 0);
        assert_eq!(offset_of!(LinuxVirtqueueInfo, callback), 8);
        assert_eq!(offset_of!(LinuxVirtqueueInfo, ctx), 16);
        assert_eq!(size_of::<LinuxVirtqueueInfo>(), 24);

        assert_eq!(offset_of!(LinuxVirtqueue, list), 0);
        assert_eq!(offset_of!(LinuxVirtqueue, callback), 16);
        assert_eq!(offset_of!(LinuxVirtqueue, name), 24);
        assert_eq!(offset_of!(LinuxVirtqueue, vdev), 32);
        assert_eq!(offset_of!(LinuxVirtqueue, index), 40);
        assert_eq!(offset_of!(LinuxVirtqueue, num_free), 44);
        assert_eq!(offset_of!(LinuxVirtqueue, num_max), 48);
        assert_eq!(offset_of!(LinuxVirtqueue, reset), 52);
        assert_eq!(offset_of!(LinuxVirtqueue, priv_), 56);
        assert_eq!(size_of::<LinuxVirtqueue>(), 64);

        assert_eq!(size_of::<LinuxVirtioDeviceId>(), 8);
        assert_eq!(offset_of!(LinuxVirtioDeviceId, device), 0);
        assert_eq!(offset_of!(LinuxVirtioDeviceId, vendor), 4);
        assert_eq!(VIRTIO_DEV_ANY_ID, 0xffff_ffff);

        assert_eq!(offset_of!(LinuxVirtioDevicePrefix, index), 0);
        assert_eq!(offset_of!(LinuxVirtioDevicePrefix, failed), 4);
        assert_eq!(offset_of!(LinuxVirtioDevicePrefix, config_core_enabled), 5);
        assert_eq!(
            offset_of!(LinuxVirtioDevicePrefix, config_driver_disabled),
            6
        );
        assert_eq!(
            offset_of!(LinuxVirtioDevicePrefix, config_change_pending),
            7
        );
        assert_eq!(size_of::<LinuxVirtioDevicePrefix>(), 8);
        assert_eq!(LINUX_VIRTIO_DEVICE_DEV_OFFSET, 8);

        assert_eq!(offset_of!(LinuxVirtioDevice, prefix), 0);
        assert_eq!(
            offset_of!(LinuxVirtioDevice, dev),
            LINUX_VIRTIO_DEVICE_DEV_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxVirtioDevice, id),
            LINUX_VIRTIO_DEVICE_ID_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxVirtioDevice, config),
            LINUX_VIRTIO_DEVICE_CONFIG_OFFSET
        );
        assert_eq!(offset_of!(LinuxVirtioDevice, vringh_config), 0x1c8);
        assert_eq!(offset_of!(LinuxVirtioDevice, map), 0x1d0);
        assert_eq!(offset_of!(LinuxVirtioDevice, vqs), 0x1d8);
        assert_eq!(offset_of!(LinuxVirtioDevice, features), 0x1e8);
        assert_eq!(
            offset_of!(LinuxVirtioDevice, priv_),
            LINUX_VIRTIO_DEVICE_PRIV_OFFSET
        );
        assert_eq!(size_of::<LinuxVirtioDevice>(), 0x200);

        assert_eq!(offset_of!(LinuxDeviceDriver, name), 0);
        assert_eq!(offset_of!(LinuxDeviceDriver, bus), 8);
        assert_eq!(offset_of!(LinuxDeviceDriver, owner), 16);
        assert_eq!(offset_of!(LinuxDeviceDriver, mod_name), 24);
        assert_eq!(offset_of!(LinuxDeviceDriver, suppress_bind_attrs), 32);
        assert_eq!(offset_of!(LinuxDeviceDriver, probe_type), 36);
        assert_eq!(offset_of!(LinuxDeviceDriver, of_match_table), 40);
        assert_eq!(offset_of!(LinuxDeviceDriver, acpi_match_table), 48);
        assert_eq!(offset_of!(LinuxDeviceDriver, probe), 56);
        assert_eq!(offset_of!(LinuxDeviceDriver, sync_state), 64);
        assert_eq!(offset_of!(LinuxDeviceDriver, remove), 72);
        assert_eq!(offset_of!(LinuxDeviceDriver, shutdown), 80);
        assert_eq!(offset_of!(LinuxDeviceDriver, suspend), 88);
        assert_eq!(offset_of!(LinuxDeviceDriver, resume), 96);
        assert_eq!(offset_of!(LinuxDeviceDriver, groups), 104);
        assert_eq!(offset_of!(LinuxDeviceDriver, dev_groups), 112);
        assert_eq!(offset_of!(LinuxDeviceDriver, pm), 120);
        assert_eq!(offset_of!(LinuxDeviceDriver, coredump), 128);
        assert_eq!(offset_of!(LinuxDeviceDriver, p), 136);
        assert_eq!(offset_of!(LinuxDeviceDriver, p_cb), 144);
        assert_eq!(size_of::<LinuxDeviceDriver>(), 152);

        assert_eq!(offset_of!(LinuxVirtioDriver, driver), 0);
        assert_eq!(offset_of!(LinuxVirtioDriver, id_table), 152);
        assert_eq!(offset_of!(LinuxVirtioDriver, feature_table), 160);
        assert_eq!(offset_of!(LinuxVirtioDriver, feature_table_size), 168);
        assert_eq!(offset_of!(LinuxVirtioDriver, feature_table_legacy), 176);
        assert_eq!(
            offset_of!(LinuxVirtioDriver, feature_table_size_legacy),
            184
        );
        assert_eq!(offset_of!(LinuxVirtioDriver, validate), 192);
        assert_eq!(offset_of!(LinuxVirtioDriver, probe), 200);
        assert_eq!(offset_of!(LinuxVirtioDriver, scan), 208);
        assert_eq!(offset_of!(LinuxVirtioDriver, remove), 216);
        assert_eq!(offset_of!(LinuxVirtioDriver, config_changed), 224);
        assert_eq!(offset_of!(LinuxVirtioDriver, freeze), 232);
        assert_eq!(offset_of!(LinuxVirtioDriver, restore), 240);
        assert_eq!(offset_of!(LinuxVirtioDriver, reset_prepare), 248);
        assert_eq!(offset_of!(LinuxVirtioDriver, reset_done), 256);
        assert_eq!(offset_of!(LinuxVirtioDriver, shutdown), 264);
        assert_eq!(size_of::<LinuxVirtioDriver>(), 272);
    }

    #[test]
    fn linux_virtio_device_entrypoint_registers_embedded_device_without_probe() {
        use core::sync::atomic::{AtomicU8, AtomicU32, Ordering};

        static STATUS: AtomicU8 = AtomicU8::new(0);
        static RESET_COUNT: AtomicU32 = AtomicU32::new(0);

        unsafe extern "C" fn get_status(_vdev: *mut c_void) -> u8 {
            STATUS.load(Ordering::Acquire)
        }

        unsafe extern "C" fn set_status(_vdev: *mut c_void, status: u8) {
            STATUS.store(status, Ordering::Release);
        }

        unsafe extern "C" fn reset(_vdev: *mut c_void) {
            RESET_COUNT.fetch_add(1, Ordering::AcqRel);
            STATUS.store(0, Ordering::Release);
        }

        unsafe {
            assert_eq!(register_virtio_device(core::ptr::null_mut()), -EINVAL);

            let mut device = core::mem::zeroed::<LinuxVirtioDevice>();
            let before = crate::linux_driver_abi::base::registered_linux_device_count();
            let vdev = (&mut device as *mut LinuxVirtioDevice).cast::<c_void>();

            assert_eq!(register_virtio_device(vdev), -EINVAL);
            assert!(device.prefix.failed);

            STATUS.store(0, Ordering::Release);
            RESET_COUNT.store(0, Ordering::Release);
            let config = LinuxVirtioConfigOps {
                get: None,
                set: None,
                generation: None,
                get_status: Some(get_status),
                set_status: Some(set_status),
                reset: Some(reset),
                find_vqs: None,
                del_vqs: None,
                synchronize_cbs: None,
                get_features: None,
                get_extended_features: None,
                finalize_features: None,
                bus_name: None,
                set_vq_affinity: None,
                get_vq_affinity: None,
                get_shm_region: None,
                disable_vq_and_reset: None,
                enable_vq_after_reset: None,
            };
            device.config = &config;

            assert_eq!(register_virtio_device(vdev), -EINVAL);
            assert!(device.prefix.failed);

            device.id = LinuxVirtioDeviceId {
                device: VIRTIO_ID_BLOCK,
                vendor: VIRTIO_DEV_ANY_ID,
            };

            assert_eq!(register_virtio_device(vdev), 0);
            assert!(device.prefix.index >= 0);
            assert!(!device.prefix.failed);
            assert!(!device.prefix.config_core_enabled);
            assert!(!device.prefix.config_driver_disabled);
            assert!(!device.prefix.config_change_pending);
            assert_eq!(device.dev.bus, linux_virtio_bus_ptr());
            assert!(!device.dev.p.is_null());
            assert!(is_virtio_device(
                (&mut device.dev as *mut LinuxDevice).cast()
            ));
            assert!(crate::linux_driver_abi::base::linux_device_registered(
                &device.dev
            ));
            assert_eq!(
                crate::linux_driver_abi::base::registered_linux_device_count(),
                before + 1
            );
            assert_eq!(RESET_COUNT.load(Ordering::Acquire), 1);
            assert_eq!(STATUS.load(Ordering::Acquire), VIRTIO_CONFIG_S_ACKNOWLEDGE);

            virtio_add_status(vdev, VIRTIO_CONFIG_S_DRIVER as u32);
            assert_eq!(
                STATUS.load(Ordering::Acquire),
                VIRTIO_CONFIG_S_ACKNOWLEDGE | VIRTIO_CONFIG_S_DRIVER
            );
            virtio_reset_device(vdev);
            assert_eq!(RESET_COUNT.load(Ordering::Acquire), 2);
            assert_eq!(STATUS.load(Ordering::Acquire), 0);

            unregister_virtio_device(vdev);
            assert!(device.dev.p.is_null());
            // Linux's is_virtio_device() checks dev->bus == &virtio_bus; the
            // bus pointer remains part of the device identity after unregister.
            assert!(is_virtio_device(
                (&mut device.dev as *mut LinuxDevice).cast()
            ));
            assert!(!crate::linux_driver_abi::base::linux_device_registered(
                &device.dev
            ));
            assert_eq!(
                crate::linux_driver_abi::base::registered_linux_device_count(),
                before
            );
        }
    }

    #[test]
    fn linux_virtio_bus_match_uses_vendor_id_table() {
        unsafe {
            let mut device = core::mem::zeroed::<LinuxVirtioDevice>();
            let dev = (&mut device.dev as *mut LinuxDevice).cast::<c_void>();
            let mut driver = core::mem::zeroed::<LinuxVirtioDriver>();

            static BLOCK_IDS: [LinuxVirtioDeviceId; 2] = [
                LinuxVirtioDeviceId {
                    device: VIRTIO_ID_BLOCK,
                    vendor: VIRTIO_DEV_ANY_ID,
                },
                LinuxVirtioDeviceId {
                    device: 0,
                    vendor: 0,
                },
            ];
            static NET_IDS: [LinuxVirtioDeviceId; 2] = [
                LinuxVirtioDeviceId {
                    device: VIRTIO_ID_NET,
                    vendor: VIRTIO_DEV_ANY_ID,
                },
                LinuxVirtioDeviceId {
                    device: 0,
                    vendor: 0,
                },
            ];

            assert_eq!(
                linux_virtio_dev_match(dev, (&driver as *const LinuxVirtioDriver).cast()),
                0
            );

            device.id = LinuxVirtioDeviceId {
                device: VIRTIO_ID_BLOCK,
                vendor: 0x1af4,
            };

            driver.id_table = NET_IDS.as_ptr();
            assert_eq!(
                linux_virtio_dev_match(dev, (&driver as *const LinuxVirtioDriver).cast()),
                0
            );

            driver.id_table = BLOCK_IDS.as_ptr();
            assert_eq!(
                linux_virtio_dev_match(dev, (&driver as *const LinuxVirtioDriver).cast()),
                1
            );
            device.id = LinuxVirtioDeviceId {
                device: 0,
                vendor: 0,
            };
            assert_eq!(
                linux_virtio_dev_match(dev, (&driver as *const LinuxVirtioDriver).cast()),
                0
            );
        }
    }

    #[test]
    fn linux_virtio_bus_probe_negotiates_features_and_calls_linux_probe() {
        use core::sync::atomic::{AtomicU8, AtomicU32, AtomicU64, Ordering};

        static STATUS: AtomicU8 = AtomicU8::new(0);
        static FINALIZE_COUNT: AtomicU32 = AtomicU32::new(0);
        static PROBE_COUNT: AtomicU32 = AtomicU32::new(0);
        static NEGOTIATED: AtomicU64 = AtomicU64::new(0);

        unsafe extern "C" fn get_status(_vdev: *mut c_void) -> u8 {
            STATUS.load(Ordering::Acquire)
        }

        unsafe extern "C" fn set_status(_vdev: *mut c_void, status: u8) {
            STATUS.store(status, Ordering::Release);
        }

        unsafe extern "C" fn get_features(_vdev: *mut c_void) -> u64 {
            VIRTIO_F_VERSION_1 | VIRTIO_BLK_F_RO
        }

        unsafe extern "C" fn finalize_features(_vdev: *mut c_void) -> i32 {
            FINALIZE_COUNT.fetch_add(1, Ordering::AcqRel);
            0
        }

        unsafe extern "C" fn probe(vdev: *mut c_void) -> i32 {
            PROBE_COUNT.fetch_add(1, Ordering::AcqRel);
            NEGOTIATED.store(
                unsafe { (*vdev.cast::<LinuxVirtioDevice>()).features[0] },
                Ordering::Release,
            );
            0
        }

        unsafe {
            STATUS.store(0, Ordering::Release);
            FINALIZE_COUNT.store(0, Ordering::Release);
            PROBE_COUNT.store(0, Ordering::Release);
            NEGOTIATED.store(0, Ordering::Release);

            static IDS: [LinuxVirtioDeviceId; 2] = [
                LinuxVirtioDeviceId {
                    device: VIRTIO_ID_BLOCK,
                    vendor: VIRTIO_DEV_ANY_ID,
                },
                LinuxVirtioDeviceId {
                    device: 0,
                    vendor: 0,
                },
            ];
            static FEATURES: [u32; 1] = [5];

            let mut driver = core::mem::zeroed::<LinuxVirtioDriver>();
            let name = b"virtio-probe-test\0";
            driver.driver.name = name.as_ptr().cast::<c_char>();
            driver.id_table = IDS.as_ptr();
            driver.feature_table = FEATURES.as_ptr();
            driver.feature_table_size = FEATURES.len() as u32;
            driver.probe = Some(probe);
            assert_eq!(
                __register_virtio_driver(
                    (&mut driver as *mut LinuxVirtioDriver).cast(),
                    core::ptr::null_mut()
                ),
                0
            );

            let config = LinuxVirtioConfigOps {
                get: None,
                set: None,
                generation: None,
                get_status: Some(get_status),
                set_status: Some(set_status),
                reset: None,
                find_vqs: None,
                del_vqs: None,
                synchronize_cbs: None,
                get_features: Some(get_features),
                get_extended_features: None,
                finalize_features: Some(finalize_features),
                bus_name: None,
                set_vq_affinity: None,
                get_vq_affinity: None,
                get_shm_region: None,
                disable_vq_and_reset: None,
                enable_vq_after_reset: None,
            };
            let mut device = core::mem::zeroed::<LinuxVirtioDevice>();
            device.id = LinuxVirtioDeviceId {
                device: VIRTIO_ID_BLOCK,
                vendor: 0x1af4,
            };
            device.config = &config;
            let vdev = (&mut device as *mut LinuxVirtioDevice).cast::<c_void>();

            assert_eq!(register_virtio_device(vdev), 0);
            assert_eq!(FINALIZE_COUNT.load(Ordering::Acquire), 1);
            assert_eq!(PROBE_COUNT.load(Ordering::Acquire), 1);
            assert_eq!(
                STATUS.load(Ordering::Acquire),
                VIRTIO_CONFIG_S_ACKNOWLEDGE
                    | VIRTIO_CONFIG_S_DRIVER
                    | VIRTIO_CONFIG_S_FEATURES_OK
                    | VIRTIO_CONFIG_S_DRIVER_OK
            );
            assert_eq!(
                NEGOTIATED.load(Ordering::Acquire),
                VIRTIO_F_VERSION_1 | VIRTIO_BLK_F_RO
            );
            assert!(device.prefix.config_core_enabled);

            unregister_virtio_device(vdev);
            unregister_virtio_driver((&mut driver as *mut LinuxVirtioDriver).cast());
        }
    }

    #[test]
    fn linux_virtio_bus_probe_fails_closed_without_finalize_features() {
        use core::sync::atomic::{AtomicU8, Ordering};

        static STATUS: AtomicU8 = AtomicU8::new(0);

        unsafe extern "C" fn get_status(_vdev: *mut c_void) -> u8 {
            STATUS.load(Ordering::Acquire)
        }

        unsafe extern "C" fn set_status(_vdev: *mut c_void, status: u8) {
            STATUS.store(status, Ordering::Release);
        }

        unsafe extern "C" fn get_features(_vdev: *mut c_void) -> u64 {
            VIRTIO_F_VERSION_1
        }

        unsafe extern "C" fn probe(_vdev: *mut c_void) -> i32 {
            0
        }

        unsafe {
            STATUS.store(0, Ordering::Release);
            let config = LinuxVirtioConfigOps {
                get: None,
                set: None,
                generation: None,
                get_status: Some(get_status),
                set_status: Some(set_status),
                reset: None,
                find_vqs: None,
                del_vqs: None,
                synchronize_cbs: None,
                get_features: Some(get_features),
                get_extended_features: None,
                finalize_features: None,
                bus_name: None,
                set_vq_affinity: None,
                get_vq_affinity: None,
                get_shm_region: None,
                disable_vq_and_reset: None,
                enable_vq_after_reset: None,
            };
            let mut device = core::mem::zeroed::<LinuxVirtioDevice>();
            let mut driver = core::mem::zeroed::<LinuxVirtioDriver>();
            device.config = &config;
            device.dev.driver = (&mut driver.driver as *mut LinuxDeviceDriver).cast();
            driver.probe = Some(probe);

            assert_eq!(
                linux_virtio_dev_probe((&mut device.dev as *mut LinuxDevice).cast()),
                -EINVAL
            );
            assert_eq!(STATUS.load(Ordering::Acquire), VIRTIO_CONFIG_S_FAILED);
        }
    }

    #[test]
    fn linux_virtio_find_vqs_dispatches_transport_callback() {
        use core::sync::atomic::{AtomicU32, Ordering};

        static FIND_COUNT: AtomicU32 = AtomicU32::new(0);

        unsafe extern "C" fn find_vqs(
            vdev: *mut c_void,
            nvqs: u32,
            vqs: *mut *mut c_void,
            vqs_info: *mut c_void,
            _desc: *mut c_void,
        ) -> i32 {
            FIND_COUNT.fetch_add(1, Ordering::AcqRel);
            assert_eq!(nvqs, 1);
            let info = vqs_info.cast::<LinuxVirtqueueInfo>();
            unsafe {
                assert!(!(*info).name.is_null());
                let queue = Box::into_raw(Box::new(LinuxVirtqueue {
                    list: LinuxListHead {
                        next: core::ptr::null_mut(),
                        prev: core::ptr::null_mut(),
                    },
                    callback: None,
                    name: core::ptr::null(),
                    vdev: core::ptr::null_mut(),
                    index: 99,
                    num_free: 8,
                    num_max: 8,
                    reset: false,
                    priv_: core::ptr::null_mut(),
                }));
                (*queue).callback = (*info).callback;
                (*queue).name = (*info).name;
                (*queue).vdev = vdev.cast();
                *vqs = queue.cast();
            }
            0
        }

        unsafe {
            FIND_COUNT.store(0, Ordering::Release);
            let config = LinuxVirtioConfigOps {
                get: None,
                set: None,
                generation: None,
                get_status: None,
                set_status: None,
                reset: None,
                find_vqs: Some(find_vqs),
                del_vqs: None,
                synchronize_cbs: None,
                get_features: None,
                get_extended_features: None,
                finalize_features: None,
                bus_name: None,
                set_vq_affinity: None,
                get_vq_affinity: None,
                get_shm_region: None,
                disable_vq_and_reset: None,
                enable_vq_after_reset: None,
            };
            let mut device = core::mem::zeroed::<LinuxVirtioDevice>();
            device.config = &config;
            let mut vq = core::ptr::null_mut::<LinuxVirtqueue>();
            let name = b"requests\0";
            let mut info = LinuxVirtqueueInfo {
                name: name.as_ptr().cast(),
                callback: None,
                ctx: false,
            };

            assert_eq!(
                virtio_find_vqs(
                    (&mut device as *mut LinuxVirtioDevice).cast(),
                    1,
                    &mut vq,
                    &mut info,
                    core::ptr::null_mut(),
                ),
                0
            );
            assert_eq!(FIND_COUNT.load(Ordering::Acquire), 1);
            assert!(!vq.is_null());
            assert_eq!((*vq).vdev, &mut device as *mut LinuxVirtioDevice);
            assert_eq!((*vq).index, 0);
        }
    }

    #[test]
    fn linux_virtqueue_helpers_fail_closed_without_vring_backend() {
        unsafe {
            let mut vq = core::mem::zeroed::<LinuxVirtqueue>();
            vq.num_free = 4;
            vq.num_max = 4;
            let token = 0x1234usize as *mut c_void;
            let mut sg = core::ptr::null_mut::<c_void>();

            assert_eq!(virtqueue_add_sgs(&mut vq, &mut sg, 1, 1, token, 0), -ENODEV);
            assert_eq!(vq.num_free, 4);
            assert!(!virtqueue_kick_prepare(&mut vq));
            assert!(!virtqueue_kick(&mut vq));

            let mut len = 0u32;
            assert!(virtqueue_get_buf(&mut vq, &mut len).is_null());
            assert_eq!(len, 0);
            assert!(virtqueue_enable_cb(&mut vq));
            assert!(virtqueue_get_buf(&mut vq, &mut len).is_null());
            assert_eq!(vq.num_free, 4);

            __virtqueue_break(&mut vq);
            assert!(virtqueue_is_broken(&vq));
            assert_eq!(virtqueue_add_sgs(&mut vq, &mut sg, 1, 0, token, 0), -EIO);
            __virtqueue_unbreak(&mut vq);
            assert!(!virtqueue_is_broken(&vq));
            assert_eq!(virtqueue_get_vring_size(&vq), 4);
            assert!(virtqueue_dma_dev(&mut vq).is_null());
            assert_eq!(virtqueue_get_desc_addr(&vq), 0);
            assert_eq!(virtqueue_get_avail_addr(&vq), 0);
            assert_eq!(virtqueue_get_used_addr(&vq), 0);
            assert_eq!(vring_notification_data(&mut vq), 0);
            assert_eq!(vring_interrupt(0, core::ptr::null_mut()), 0);
            vring_transport_features(core::ptr::null_mut());
            assert_eq!(
                virtio_device_reset_prepare(core::ptr::null_mut()),
                -EOPNOTSUPP
            );
            assert_eq!(virtio_device_reset_done(core::ptr::null_mut()), -EOPNOTSUPP);
            virtio_break_device(core::ptr::null_mut());
            __virtio_unbreak_device(core::ptr::null_mut());
        }
    }

    #[test]
    fn vring_create_virtqueue_allocates_split_ring_backing() {
        unsafe {
            let mut device = core::mem::zeroed::<LinuxVirtioDevice>();
            let head = core::ptr::addr_of_mut!(device.vqs).cast::<c_void>();
            linux_list_head_init(core::ptr::addr_of_mut!(device.vqs));
            let name = b"requests\0";
            let vq = vring_create_virtqueue(
                7,
                16,
                4096,
                &mut device,
                false,
                false,
                false,
                None,
                None,
                name.as_ptr().cast(),
            );
            assert!(!vq.is_null());
            assert_eq!((*vq).index, 7);
            assert_eq!((*vq).num_free, 16);
            assert_eq!((*vq).num_max, 16);
            assert_eq!((*vq).vdev, &mut device as *mut LinuxVirtioDevice);
            assert_eq!(device.vqs.next, core::ptr::addr_of_mut!((*vq).list).cast());
            assert_eq!(device.vqs.prev, core::ptr::addr_of_mut!((*vq).list).cast());
            assert_eq!(vring_notification_data(vq), 7);
            let desc = virtqueue_get_desc_addr(vq);
            let avail = virtqueue_get_avail_addr(vq);
            let used = virtqueue_get_used_addr(vq);
            assert_ne!(desc, 0);
            assert_eq!(desc % VRING_DESC_ALIGN_SIZE as u64, 0);
            assert!(avail > desc);
            assert_eq!(avail % VRING_AVAIL_ALIGN_SIZE as u64, 0);
            assert!(used > avail);
            assert_eq!(used % VRING_USED_ALIGN_SIZE as u64, 0);
            vring_del_virtqueue(vq);
            assert_eq!(device.vqs.next, head);
            assert_eq!(device.vqs.prev, head);
        }
    }

    #[test]
    fn linux_virtio_config_change_helpers_track_driver_state() {
        use core::sync::atomic::{AtomicU32, Ordering};

        static CHANGE_COUNT: AtomicU32 = AtomicU32::new(0);

        unsafe extern "C" fn config_changed(_vdev: *mut c_void) {
            CHANGE_COUNT.fetch_add(1, Ordering::AcqRel);
        }

        unsafe {
            CHANGE_COUNT.store(0, Ordering::Release);
            let mut device = core::mem::zeroed::<LinuxVirtioDevice>();
            let mut driver = core::mem::zeroed::<LinuxVirtioDriver>();
            let vdev = (&mut device as *mut LinuxVirtioDevice).cast::<c_void>();
            driver.config_changed = Some(config_changed);
            device.dev.driver = (&mut driver.driver as *mut LinuxDeviceDriver).cast();

            virtio_config_changed(vdev);
            assert!(device.prefix.config_change_pending);
            assert_eq!(CHANGE_COUNT.load(Ordering::Acquire), 0);

            device.prefix.config_core_enabled = true;
            virtio_config_driver_enable(vdev);
            assert!(!device.prefix.config_change_pending);
            assert_eq!(CHANGE_COUNT.load(Ordering::Acquire), 1);

            virtio_config_driver_disable(vdev);
            assert!(device.prefix.config_driver_disabled);
            virtio_config_changed(vdev);
            assert!(device.prefix.config_change_pending);
            assert_eq!(CHANGE_COUNT.load(Ordering::Acquire), 1);

            virtio_config_driver_enable(vdev);
            assert!(!device.prefix.config_driver_disabled);
            assert!(!device.prefix.config_change_pending);
            assert_eq!(CHANGE_COUNT.load(Ordering::Acquire), 2);
        }
    }

    #[test]
    fn linux_virtio_feature_check_uses_driver_tables() {
        use core::sync::atomic::{AtomicU8, Ordering};

        static STATUS: AtomicU8 = AtomicU8::new(0);

        unsafe extern "C" fn get_status(_vdev: *mut c_void) -> u8 {
            STATUS.load(Ordering::Acquire)
        }

        unsafe extern "C" fn set_status(_vdev: *mut c_void, status: u8) {
            STATUS.store(status, Ordering::Release);
        }

        unsafe {
            STATUS.store(0, Ordering::Release);
            let mut device = core::mem::zeroed::<LinuxVirtioDevice>();
            let mut driver = core::mem::zeroed::<LinuxVirtioDriver>();
            let vdev = (&mut device as *mut LinuxVirtioDevice).cast::<c_void>();
            let config = LinuxVirtioConfigOps {
                get: None,
                set: None,
                generation: None,
                get_status: Some(get_status),
                set_status: Some(set_status),
                reset: None,
                find_vqs: None,
                del_vqs: None,
                synchronize_cbs: None,
                get_features: None,
                get_extended_features: None,
                finalize_features: None,
                bus_name: None,
                set_vq_affinity: None,
                get_vq_affinity: None,
                get_shm_region: None,
                disable_vq_and_reset: None,
                enable_vq_after_reset: None,
            };
            static FEATURES: [u32; 2] = [5, 32];
            driver.feature_table = FEATURES.as_ptr();
            driver.feature_table_size = FEATURES.len() as u32;
            device.dev.driver = (&mut driver.driver as *mut LinuxDeviceDriver).cast();
            device.config = &config;

            virtio_check_driver_offered_feature(vdev, 5);
            assert_eq!(STATUS.load(Ordering::Acquire), 0);

            virtio_check_driver_offered_feature(vdev, 9);
            assert_eq!(STATUS.load(Ordering::Acquire), VIRTIO_CONFIG_S_FAILED);

            device.config = core::ptr::null();
        }
    }

    #[test]
    fn linux_virtio_driver_entrypoint_registers_through_driver_core_without_probe() {
        unsafe {
            assert_eq!(
                __register_virtio_driver(core::ptr::null_mut(), core::ptr::null_mut()),
                -EINVAL
            );

            let mut driver = core::mem::zeroed::<LinuxVirtioDriver>();
            let name = b"virtio-test-driver\0";
            let owner = 0xfeed_cafeusize as *mut c_void;
            driver.driver.name = name.as_ptr().cast::<c_char>();
            let before = crate::linux_driver_abi::base::registered_linux_device_driver_count();
            assert_eq!(
                __register_virtio_driver((&mut driver as *mut LinuxVirtioDriver).cast(), owner,),
                0
            );
            assert_eq!(driver.driver.owner, owner);
            assert_eq!(driver.driver.bus, linux_virtio_bus_ptr());
            assert!(!driver.driver.p.is_null());
            assert!(crate::linux_driver_abi::base::linux_bus_type_registered(
                linux_virtio_bus_ptr()
            ));
            assert!(crate::linux_driver_abi::base::linux_device_driver_registered(&driver.driver));
            assert_eq!(
                crate::linux_driver_abi::base::registered_linux_device_driver_count(),
                before + 1
            );

            unregister_virtio_driver((&mut driver as *mut LinuxVirtioDriver).cast());
            assert!(driver.driver.p.is_null());
            assert!(!crate::linux_driver_abi::base::linux_device_driver_registered(&driver.driver));
            assert_eq!(
                crate::linux_driver_abi::base::registered_linux_device_driver_count(),
                before
            );

            driver.feature_table_size = 1;
            driver.feature_table = core::ptr::null();
            assert_eq!(
                __register_virtio_driver(
                    (&mut driver as *mut LinuxVirtioDriver).cast(),
                    core::ptr::null_mut(),
                ),
                -EINVAL
            );
        }
    }
}
