//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/platform/efi/efi.c
//! test-origin: linux:vendor/linux/arch/x86/platform/efi/efi.c
//! x86 EFI runtime-service discovery.
//!
//! Mirrors the early Linux x86 EFI path far enough to bridge
//! `boot_params.efi_info` to the firmware runtime-services table.
//! The runtime-map builder and `GetVariable` wrapper mirror the parts Linux
//! keeps in `efi.c`, `efi_64.c`, and `runtime-wrappers.c`; real firmware calls
//! are exposed only after the EFI runtime map can translate physical pointers.

extern crate alloc;

use alloc::vec::Vec;
use spin::Mutex;

use crate::arch::x86::boot::compressed::efi::{
    EfiType, efi_get_memmap, efi_get_system_table, efi_get_type,
};
use crate::arch::x86::include::uapi::asm::bootparam::BootParams;
use crate::arch::x86::kernel::fpu::{KFPU_387, KFPU_MXCSR, kernel_fpu_begin_mask, kernel_fpu_end};
use crate::efi::vars::{self, GetVariableProvider, Guid};
use crate::include::uapi::errno::{EBADMSG, EINVAL, EIO, ENODEV, ENOMEM, EOPNOTSUPP, EOVERFLOW};
use crate::kernel::locking::irqflags::{
    IrqFlags, X86_EFLAGS_IF, arch_local_save_flags, local_irq_restore, local_irq_save,
};
use crate::kernel::locking::preempt::{preempt_disable, preempt_enable};

use super::EfiMode;

pub const EFI_SYSTEM_TABLE_SIGNATURE: u64 = 0x5453_5953_2049_4249;
pub const EFI_RUNTIME_SERVICES_SIGNATURE: u64 = 0x5652_4553_544e_5552;
pub const EFI_MEMORY_DESCRIPTOR_VERSION: u32 = 1;
pub const EFI_BOOT_SERVICES_CODE: u32 = 3;
pub const EFI_RUNTIME_SERVICES_CODE: u32 = 5;
pub const EFI_RUNTIME_SERVICES_DATA: u32 = 6;
pub const EFI_MEMORY_WB: u64 = 1 << 3;
pub const EFI_MEMORY_RUNTIME: u64 = 1 << 63;
pub const EFI_PAGE_SIZE: u64 = 4096;
pub const EFI_PMD_SIZE: u64 = 2 * 1024 * 1024;
pub const EFI_VA_START: u64 = 0xffff_ffff_0000_0000;
pub const EFI_VA_END: u64 = 0xffff_ffef_0000_0000;
const EFI_PMD_MASK: u64 = !(EFI_PMD_SIZE - 1);
const EFI_MAX_VARIABLE_NAME_U16: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EfiGetVariableBackend {
    pub mode: EfiMode,
    pub system_table_phys: u64,
    pub runtime_services_phys: u64,
    pub get_variable_phys: u64,
    pub set_virtual_address_map_phys: u64,
}

static OVMF_RUNTIME: Mutex<Option<EfiGetVariableBackend>> = Mutex::new(None);
static OVMF_RUNTIME_MAP: Mutex<Option<EfiRuntimeMap>> = Mutex::new(None);
static OVMF_VIRTUAL_MODE: Mutex<Option<EfiRuntimeVirtualMode>> = Mutex::new(None);
static OVMF_EFI_MM: Mutex<Option<EfiRuntimePageTables>> = Mutex::new(None);
static EFI_RUNTIME_CALL_LOCK: Mutex<()> = Mutex::new(());
const ARCH_EFI_IRQ_FLAGS_MASK: IrqFlags = X86_EFLAGS_IF;
const EFI_RUNTIME_FPU_MASK: u32 = KFPU_387 | KFPU_MXCSR;

pub fn init_from_boot_params(bp: &BootParams) -> Result<EfiGetVariableBackend, i32> {
    let state = unsafe { discover_efi_get_variable_backend_from_boot_params(bp) }?;
    install_efi_runtime_state(state, runtime_map_from_boot_params(bp, state.mode))
}

fn install_efi_runtime_state(
    state: EfiGetVariableBackend,
    runtime_map: Result<EfiRuntimeMap, i32>,
) -> Result<EfiGetVariableBackend, i32> {
    *OVMF_RUNTIME.lock() = Some(state);
    if let Ok(runtime_map) = runtime_map {
        crate::kernel::printk::log_info!(
            "efi",
            "EFI runtime memory map: {} regions desc_size={} desc_version={}",
            runtime_map.entries.len(),
            runtime_map.desc_size,
            runtime_map.desc_version
        );
        if state.set_virtual_address_map_phys != 0 {
            match unsafe { enter_efi_virtual_mode(state, &runtime_map) } {
                Ok(virtual_mode) => {
                    crate::kernel::printk::log_info!(
                        "efi",
                        "EFI runtime virtual mode: map_size={} runtime={:#x}",
                        virtual_mode.memory_map_size,
                        virtual_mode.runtime_services_virt
                    );
                    *OVMF_VIRTUAL_MODE.lock() = Some(virtual_mode);
                }
                Err(err) => {
                    crate::kernel::printk::log_warn!(
                        "efi",
                        "EFI SetVirtualAddressMap unavailable: errno={}",
                        -err
                    );
                }
            }
        }
        *OVMF_RUNTIME_MAP.lock() = Some(runtime_map);
    }
    crate::kernel::printk::log_info!(
        "efi",
        "EFI runtime services: GetVariable available systab={:#x} runtime={:#x}",
        state.system_table_phys,
        state.runtime_services_phys
    );
    Ok(state)
}

pub fn snapshot() -> Option<EfiGetVariableBackend> {
    *OVMF_RUNTIME.lock()
}

pub fn runtime_map_snapshot() -> Option<EfiRuntimeMap> {
    OVMF_RUNTIME_MAP.lock().clone()
}

pub fn runtime_virtual_mode_snapshot() -> Option<EfiRuntimeVirtualMode> {
    *OVMF_VIRTUAL_MODE.lock()
}

pub fn runtime_page_tables_snapshot() -> Option<EfiRuntimePageTables> {
    *OVMF_EFI_MM.lock()
}

pub fn firmware_get_variable_provider() -> Result<EfiRuntimeGetVariableProvider, i32> {
    let backend = snapshot().ok_or(-ENODEV)?;
    let runtime_map = runtime_map_snapshot().ok_or(-ENODEV)?;
    runtime_virtual_mode_snapshot().ok_or(-ENODEV)?;
    EfiRuntimeGetVariableProvider::new(backend, runtime_map)
}

pub fn register_secure_boot_variables_from_firmware() -> Result<usize, i32> {
    let mut provider = firmware_get_variable_provider()?;
    vars::register_runtime_variables_from_get_variable_provider(
        &mut provider,
        vars::SECURE_BOOT_VARIABLE_REQUESTS,
    )
}

pub unsafe fn discover_efi_get_variable_backend_from_boot_params(
    bp: &BootParams,
) -> Result<EfiGetVariableBackend, i32> {
    let efi_info = bp.efi_info();
    let system_table = efi_get_system_table(&efi_info);

    match efi_get_type(&efi_info) {
        EfiType::Efi64 => unsafe { discover_efi64_get_variable_backend(system_table) },
        EfiType::Efi32 => {
            let system_table = u32::try_from(system_table).map_err(|_| -EOVERFLOW)?;
            unsafe { discover_efi32_get_variable_backend(system_table) }
        }
        EfiType::None => Err(-ENODEV),
        EfiType::EfiMixed => Err(-EOPNOTSUPP),
    }
}

unsafe fn discover_efi64_get_variable_backend(
    system_table_phys: u64,
) -> Result<EfiGetVariableBackend, i32> {
    let system_table =
        unsafe { read_phys::<EfiSystemTable64>(system_table_phys) }.ok_or(-ENODEV)?;
    if system_table.hdr.signature != EFI_SYSTEM_TABLE_SIGNATURE {
        return Err(-EBADMSG);
    }
    if system_table.runtime_services == 0 {
        return Err(-ENODEV);
    }

    let runtime = unsafe { read_phys::<EfiRuntimeServices64>(system_table.runtime_services) }
        .ok_or(-ENODEV)?;
    if runtime.hdr.signature != EFI_RUNTIME_SERVICES_SIGNATURE || runtime.get_variable == 0 {
        return Err(-EBADMSG);
    }

    Ok(EfiGetVariableBackend {
        mode: EfiMode::Efi64,
        system_table_phys,
        runtime_services_phys: system_table.runtime_services,
        get_variable_phys: runtime.get_variable,
        set_virtual_address_map_phys: runtime.set_virtual_address_map,
    })
}

unsafe fn discover_efi32_get_variable_backend(
    system_table_phys: u32,
) -> Result<EfiGetVariableBackend, i32> {
    let system_table =
        unsafe { read_phys::<EfiSystemTable32>(system_table_phys as u64) }.ok_or(-ENODEV)?;
    if system_table.hdr.signature != EFI_SYSTEM_TABLE_SIGNATURE {
        return Err(-EBADMSG);
    }
    if system_table.runtime_services == 0 {
        return Err(-ENODEV);
    }

    let runtime =
        unsafe { read_phys::<EfiRuntimeServices32>(system_table.runtime_services as u64) }
            .ok_or(-ENODEV)?;
    if runtime.hdr.signature != EFI_RUNTIME_SERVICES_SIGNATURE || runtime.get_variable == 0 {
        return Err(-EBADMSG);
    }

    Ok(EfiGetVariableBackend {
        mode: EfiMode::Efi32,
        system_table_phys: system_table_phys as u64,
        runtime_services_phys: system_table.runtime_services as u64,
        get_variable_phys: runtime.get_variable as u64,
        set_virtual_address_map_phys: runtime.set_virtual_address_map as u64,
    })
}

pub fn runtime_map_from_boot_params(bp: &BootParams, mode: EfiMode) -> Result<EfiRuntimeMap, i32> {
    let efi_info = bp.efi_info();
    let map_phys = efi_get_memmap(&efi_info);
    let map_size = efi_info.efi_memmap_size as usize;
    if map_phys == 0 || map_size == 0 {
        return Err(-ENODEV);
    }

    let map = phys_to_ptr::<u8>(map_phys);
    if map.is_null() {
        return Err(-ENODEV);
    }

    unsafe {
        runtime_map_from_descriptor_bytes(
            map,
            map_size,
            efi_info.efi_memdesc_size as usize,
            efi_info.efi_memdesc_version,
            mode,
        )
    }
}

unsafe fn runtime_map_from_descriptor_bytes(
    map: *const u8,
    map_size: usize,
    desc_size: usize,
    desc_version: u32,
    mode: EfiMode,
) -> Result<EfiRuntimeMap, i32> {
    if desc_version != EFI_MEMORY_DESCRIPTOR_VERSION {
        return Err(-EBADMSG);
    }

    if desc_size < core::mem::size_of::<EfiMemoryDescriptor>() {
        return Err(-EINVAL);
    }

    let mut entries = Vec::new();
    let mut next_va = EFI_VA_START;
    let mut offset = 0usize;
    while offset.saturating_add(desc_size) <= map_size {
        let mut desc =
            unsafe { core::ptr::read_unaligned(map.add(offset) as *const EfiMemoryDescriptor) };
        if !efi_descriptor_requires_runtime_mapping(&desc) {
            offset += desc_size;
            continue;
        }
        assign_efi_runtime_virtual_address(&mut desc, mode, &mut next_va)?;
        entries.push(desc);
        offset += desc_size;
    }

    if entries.is_empty() {
        return Err(-ENODEV);
    }

    Ok(EfiRuntimeMap {
        desc_size,
        desc_version,
        entries,
    })
}

pub const fn efi_descriptor_requires_runtime_mapping(desc: &EfiMemoryDescriptor) -> bool {
    desc.attribute & EFI_MEMORY_RUNTIME != 0
}

fn assign_efi_runtime_virtual_address(
    desc: &mut EfiMemoryDescriptor,
    mode: EfiMode,
    next_va: &mut u64,
) -> Result<(), i32> {
    if desc.num_pages == 0 {
        return Err(-EBADMSG);
    }

    if matches!(mode, EfiMode::Efi32) {
        desc.virt_addr = desc.phys_addr;
        return Ok(());
    }

    let size = desc
        .num_pages
        .checked_mul(EFI_PAGE_SIZE)
        .ok_or(-EOVERFLOW)?;
    let mut va = next_va.checked_sub(size).ok_or(-EOVERFLOW)?;
    let pa_offset = desc.phys_addr & (EFI_PMD_SIZE - 1);
    if pa_offset == 0 {
        va &= EFI_PMD_MASK;
    } else {
        let prev_va = va;
        va = (va & EFI_PMD_MASK) + pa_offset;
        if va > prev_va {
            va = va.checked_sub(EFI_PMD_SIZE).ok_or(-EOVERFLOW)?;
        }
    }

    if va < EFI_VA_END {
        return Err(-EOVERFLOW);
    }

    desc.virt_addr = va;
    *next_va = va;
    Ok(())
}

unsafe fn read_phys<T: Copy>(phys: u64) -> Option<T> {
    if phys == 0 {
        return None;
    }
    let ptr = phys_to_ptr::<T>(phys);
    if ptr.is_null() {
        return None;
    }
    Some(unsafe { core::ptr::read_unaligned(ptr) })
}

#[cfg(not(test))]
fn phys_to_ptr<T>(phys: u64) -> *const T {
    crate::arch::x86::mm::paging::phys_to_virt(phys) as *const T
}

#[cfg(test)]
fn phys_to_ptr<T>(phys: u64) -> *const T {
    phys as *const T
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct EfiTableHeader {
    signature: u64,
    revision: u32,
    header_size: u32,
    crc32: u32,
    reserved: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct EfiSystemTable64 {
    hdr: EfiTableHeader,
    firmware_vendor: u64,
    firmware_revision: u32,
    _pad0: u32,
    console_in_handle: u64,
    con_in: u64,
    console_out_handle: u64,
    con_out: u64,
    standard_error_handle: u64,
    std_err: u64,
    runtime_services: u64,
    boot_services: u64,
    number_of_table_entries: u64,
    configuration_table: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct EfiSystemTable32 {
    hdr: EfiTableHeader,
    firmware_vendor: u32,
    firmware_revision: u32,
    console_in_handle: u32,
    con_in: u32,
    console_out_handle: u32,
    con_out: u32,
    standard_error_handle: u32,
    std_err: u32,
    runtime_services: u32,
    boot_services: u32,
    number_of_table_entries: u32,
    configuration_table: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct EfiRuntimeServices64 {
    hdr: EfiTableHeader,
    get_time: u64,
    set_time: u64,
    get_wakeup_time: u64,
    set_wakeup_time: u64,
    set_virtual_address_map: u64,
    convert_pointer: u64,
    get_variable: u64,
    get_next_variable_name: u64,
    set_variable: u64,
    get_next_high_monotonic_count: u64,
    reset_system: u64,
    update_capsule: u64,
    query_capsule_capabilities: u64,
    query_variable_info: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct EfiRuntimeServices32 {
    hdr: EfiTableHeader,
    get_time: u32,
    set_time: u32,
    get_wakeup_time: u32,
    set_wakeup_time: u32,
    set_virtual_address_map: u32,
    convert_pointer: u32,
    get_variable: u32,
    get_next_variable_name: u32,
    set_variable: u32,
    get_next_high_monotonic_count: u32,
    reset_system: u32,
    update_capsule: u32,
    query_capsule_capabilities: u32,
    query_variable_info: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EfiMemoryDescriptor {
    pub ty: u32,
    pub pad: u32,
    pub phys_addr: u64,
    pub virt_addr: u64,
    pub num_pages: u64,
    pub attribute: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EfiRuntimeMap {
    pub desc_size: usize,
    pub desc_version: u32,
    pub entries: Vec<EfiMemoryDescriptor>,
}

impl EfiRuntimeMap {
    pub fn virtualize_phys(&self, phys: u64) -> Option<u64> {
        self.entries.iter().find_map(|entry| {
            let size = entry.num_pages.checked_mul(EFI_PAGE_SIZE)?;
            let end = entry.phys_addr.checked_add(size)?;
            if phys >= entry.phys_addr && phys < end {
                Some(entry.virt_addr + (phys - entry.phys_addr))
            } else {
                None
            }
        })
    }

    pub fn memory_map_size(&self) -> usize {
        self.desc_size.saturating_mul(self.entries.len())
    }

    pub fn serialized_descriptors(&self) -> Result<Vec<u8>, i32> {
        let desc_len = core::mem::size_of::<EfiMemoryDescriptor>();
        if self.desc_size < desc_len {
            return Err(-EINVAL);
        }
        let mut bytes = Vec::new();
        bytes.resize(self.memory_map_size(), 0);
        for (idx, entry) in self.entries.iter().enumerate() {
            let start = idx * self.desc_size;
            let raw = unsafe {
                core::slice::from_raw_parts(
                    (entry as *const EfiMemoryDescriptor).cast::<u8>(),
                    desc_len,
                )
            };
            bytes[start..start + desc_len].copy_from_slice(raw);
        }
        Ok(bytes)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EfiRuntimeVirtualMapCall {
    pub memory_map_size: usize,
    pub desc_size: usize,
    pub desc_version: u32,
    pub set_virtual_address_map_phys: u64,
    pub set_virtual_address_map_virt: u64,
    pub runtime_services_virt: u64,
    pub get_variable_virt: u64,
    pub efi_mm_pgd_phys: u64,
    pub synced_kernel_entries: usize,
    pub mapped_pages: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EfiRuntimeVirtualMode {
    pub memory_map_size: usize,
    pub desc_size: usize,
    pub desc_version: u32,
    pub runtime_services_virt: u64,
    pub get_variable_virt: u64,
    pub set_virtual_address_map_virt: u64,
    pub efi_mm_pgd_phys: u64,
    pub synced_kernel_entries: usize,
    pub mapped_pages: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EfiRuntimePageTables {
    pub pgd_phys: u64,
    pub synced_kernel_entries: usize,
    pub mapped_pages: usize,
}

unsafe fn allocate_efi_runtime_page_tables() -> Result<EfiRuntimePageTables, i32> {
    let pgd_phys =
        unsafe { crate::arch::x86::mm::paging::alloc_kernel_page_table_root() }.ok_or(-ENOMEM)?;
    let synced_kernel_entries = unsafe {
        crate::arch::x86::mm::paging::sync_kernel_mappings_around_window(
            pgd_phys,
            EFI_VA_END,
            EFI_VA_START,
        )
    }
    .ok_or(-EINVAL)?;
    Ok(EfiRuntimePageTables {
        pgd_phys,
        synced_kernel_entries,
        mapped_pages: 0,
    })
}

pub fn enter_efi_virtual_mode_with_caller<F>(
    backend: EfiGetVariableBackend,
    runtime_map: &EfiRuntimeMap,
    mut caller: F,
) -> Result<EfiRuntimeVirtualMode, i32>
where
    F: FnMut(&EfiRuntimeVirtualMapCall, *const EfiMemoryDescriptor) -> vars::EfiStatus,
{
    if !matches!(backend.mode, EfiMode::Efi64) {
        return Err(-EOPNOTSUPP);
    }
    if backend.set_virtual_address_map_phys == 0 {
        return Err(-ENODEV);
    }

    let set_virtual_address_map_virt = runtime_map
        .virtualize_phys(backend.set_virtual_address_map_phys)
        .ok_or(-ENODEV)?;
    let runtime_services_virt = runtime_map
        .virtualize_phys(backend.runtime_services_phys)
        .ok_or(-ENODEV)?;
    let get_variable_virt = runtime_map
        .virtualize_phys(backend.get_variable_phys)
        .ok_or(-ENODEV)?;

    let mut page_tables = unsafe { allocate_efi_runtime_page_tables()? };
    let mapped_pages =
        unsafe { map_efi_runtime_regions_in_pgd(page_tables.pgd_phys, runtime_map)? };
    page_tables.mapped_pages = mapped_pages;
    *OVMF_EFI_MM.lock() = Some(page_tables);

    let descriptors = runtime_map.serialized_descriptors()?;
    #[cfg(not(test))]
    let (descriptor_ptr, memory_map_buffer_pages) = unsafe {
        map_efi_memory_map_buffer_in_pgd(
            page_tables.pgd_phys,
            descriptors.as_ptr() as u64,
            descriptors.len(),
        )?
    };
    #[cfg(test)]
    let (descriptor_ptr, memory_map_buffer_pages) =
        (descriptors.as_ptr().cast::<EfiMemoryDescriptor>(), 0usize);

    let call = EfiRuntimeVirtualMapCall {
        memory_map_size: descriptors.len(),
        desc_size: runtime_map.desc_size,
        desc_version: runtime_map.desc_version,
        set_virtual_address_map_phys: backend.set_virtual_address_map_phys,
        set_virtual_address_map_virt,
        runtime_services_virt,
        get_variable_virt,
        efi_mm_pgd_phys: page_tables.pgd_phys,
        synced_kernel_entries: page_tables.synced_kernel_entries,
        mapped_pages: mapped_pages.saturating_add(memory_map_buffer_pages),
    };
    let status = caller(&call, descriptor_ptr);
    if status != vars::EFI_SUCCESS {
        *OVMF_EFI_MM.lock() = None;
        return Err(-EIO);
    }

    Ok(EfiRuntimeVirtualMode {
        memory_map_size: call.memory_map_size,
        desc_size: call.desc_size,
        desc_version: call.desc_version,
        runtime_services_virt: call.runtime_services_virt,
        get_variable_virt: call.get_variable_virt,
        set_virtual_address_map_virt: call.set_virtual_address_map_virt,
        efi_mm_pgd_phys: call.efi_mm_pgd_phys,
        synced_kernel_entries: call.synced_kernel_entries,
        mapped_pages,
    })
}

/// Switch EFI runtime services to the Linux-style virtual map.
///
/// Source shape:
/// - `vendor/linux/arch/x86/platform/efi/efi.c::__efi_enter_virtual_mode`
/// - `vendor/linux/arch/x86/platform/efi/efi_64.c::efi_set_virtual_address_map`
///
/// # Safety
/// Firmware execution is entered through the runtime-services table. The caller
/// must only invoke this after normal boot services are gone and the EFI
/// runtime descriptors have been validated from the firmware memory map.
pub unsafe fn enter_efi_virtual_mode(
    backend: EfiGetVariableBackend,
    runtime_map: &EfiRuntimeMap,
) -> Result<EfiRuntimeVirtualMode, i32> {
    enter_efi_virtual_mode_with_caller(backend, runtime_map, |call, map| unsafe {
        call_efi_set_virtual_address_map64(
            call.set_virtual_address_map_phys,
            call.memory_map_size,
            call.desc_size,
            call.desc_version,
            map,
        )
    })
}

pub unsafe fn map_efi_runtime_regions(runtime_map: &EfiRuntimeMap) -> Result<usize, i32> {
    let mut mapped = 0usize;
    for entry in runtime_map.entries.iter() {
        let offset = entry.phys_addr & (EFI_PAGE_SIZE - 1);
        let bytes = entry
            .num_pages
            .checked_mul(EFI_PAGE_SIZE)
            .and_then(|size| size.checked_add(offset))
            .ok_or(-EOVERFLOW)?;
        let pages = bytes.div_ceil(EFI_PAGE_SIZE);
        let phys_base = entry.phys_addr & !(EFI_PAGE_SIZE - 1);
        let virt_base = entry.virt_addr & !(EFI_PAGE_SIZE - 1);
        let prot = efi_initial_runtime_pgprot(entry);

        for idx in 0..pages {
            let phys = phys_base
                .checked_add(idx.checked_mul(EFI_PAGE_SIZE).ok_or(-EOVERFLOW)?)
                .ok_or(-EOVERFLOW)?;
            unsafe {
                crate::arch::x86::mm::paging::map_kernel_page(phys, phys, prot);
            }
            mapped = mapped.saturating_add(1);

            let virt = virt_base
                .checked_add(idx.checked_mul(EFI_PAGE_SIZE).ok_or(-EOVERFLOW)?)
                .ok_or(-EOVERFLOW)?;
            if virt != phys {
                unsafe {
                    crate::arch::x86::mm::paging::map_kernel_page(virt, phys, prot);
                }
                mapped = mapped.saturating_add(1);
            }
        }
    }
    Ok(mapped)
}

#[cfg(not(test))]
unsafe fn map_efi_memory_map_buffer_in_pgd(
    pgd_phys: u64,
    ptr: u64,
    len: usize,
) -> Result<(*const EfiMemoryDescriptor, usize), i32> {
    if len == 0 {
        return Err(-EINVAL);
    }
    let phys = crate::arch::x86::mm::paging::virt_to_phys(ptr).ok_or(-ENOMEM)?;
    let page_offset = phys & (EFI_PAGE_SIZE - 1);
    let bytes = page_offset.checked_add(len as u64).ok_or(-EOVERFLOW)?;
    let pages = bytes.div_ceil(EFI_PAGE_SIZE);
    let phys_base = phys & !(EFI_PAGE_SIZE - 1);
    let prot = crate::arch::x86::mm::paging::__pgprot(
        crate::arch::x86::mm::paging::_PAGE_PRESENT
            | crate::arch::x86::mm::paging::_PAGE_RW
            | crate::arch::x86::mm::paging::_PAGE_ACCESSED
            | crate::arch::x86::mm::paging::_PAGE_DIRTY
            | crate::arch::x86::mm::paging::_PAGE_NX,
    );

    for idx in 0..pages {
        let page = phys_base
            .checked_add(idx.checked_mul(EFI_PAGE_SIZE).ok_or(-EOVERFLOW)?)
            .ok_or(-EOVERFLOW)?;
        unsafe {
            crate::arch::x86::mm::paging::map_kernel_page_in_pgd(pgd_phys, page, page, prot)
                .ok_or(-ENOMEM)?;
        }
    }

    Ok((phys as *const EfiMemoryDescriptor, pages as usize))
}

pub unsafe fn map_efi_runtime_regions_in_pgd(
    pgd_phys: u64,
    runtime_map: &EfiRuntimeMap,
) -> Result<usize, i32> {
    let mut mapped = 0usize;
    for entry in runtime_map.entries.iter() {
        let offset = entry.phys_addr & (EFI_PAGE_SIZE - 1);
        let bytes = entry
            .num_pages
            .checked_mul(EFI_PAGE_SIZE)
            .and_then(|size| size.checked_add(offset))
            .ok_or(-EOVERFLOW)?;
        let pages = bytes.div_ceil(EFI_PAGE_SIZE);
        let phys_base = entry.phys_addr & !(EFI_PAGE_SIZE - 1);
        let virt_base = entry.virt_addr & !(EFI_PAGE_SIZE - 1);
        let prot = efi_initial_runtime_pgprot(entry);

        for idx in 0..pages {
            let phys = phys_base
                .checked_add(idx.checked_mul(EFI_PAGE_SIZE).ok_or(-EOVERFLOW)?)
                .ok_or(-EOVERFLOW)?;
            unsafe {
                crate::arch::x86::mm::paging::map_kernel_page_in_pgd(pgd_phys, phys, phys, prot)
                    .ok_or(-ENOMEM)?;
            }
            mapped = mapped.saturating_add(1);

            let virt = virt_base
                .checked_add(idx.checked_mul(EFI_PAGE_SIZE).ok_or(-EOVERFLOW)?)
                .ok_or(-EOVERFLOW)?;
            if virt != phys {
                unsafe {
                    crate::arch::x86::mm::paging::map_kernel_page_in_pgd(
                        pgd_phys, virt, phys, prot,
                    )
                    .ok_or(-ENOMEM)?;
                }
                mapped = mapped.saturating_add(1);
            }
        }
    }
    Ok(mapped)
}

fn efi_initial_runtime_pgprot(
    desc: &EfiMemoryDescriptor,
) -> crate::arch::x86::mm::paging::pgprot_t {
    use crate::arch::x86::mm::paging::{
        __pgprot, _PAGE_ACCESSED, _PAGE_DIRTY, _PAGE_GLOBAL, _PAGE_NX, _PAGE_PCD, _PAGE_PRESENT,
        _PAGE_RW,
    };

    let mut flags = _PAGE_PRESENT | _PAGE_RW | _PAGE_ACCESSED | _PAGE_DIRTY | _PAGE_GLOBAL;
    if desc.ty != EFI_BOOT_SERVICES_CODE && desc.ty != EFI_RUNTIME_SERVICES_CODE {
        flags |= _PAGE_NX;
    }
    if desc.attribute & EFI_MEMORY_WB == 0 {
        flags |= _PAGE_PCD;
    }
    __pgprot(flags)
}

pub struct EfiRuntimeGetVariableProvider {
    backend: EfiGetVariableBackend,
    runtime_map: EfiRuntimeMap,
    get_variable_virt: u64,
}

impl EfiRuntimeGetVariableProvider {
    pub fn new(backend: EfiGetVariableBackend, runtime_map: EfiRuntimeMap) -> Result<Self, i32> {
        if !matches!(backend.mode, EfiMode::Efi64) {
            return Err(-EOPNOTSUPP);
        }
        let get_variable_virt = runtime_map
            .virtualize_phys(backend.get_variable_phys)
            .ok_or(-ENODEV)?;
        Ok(Self {
            backend,
            runtime_map,
            get_variable_virt,
        })
    }

    pub fn backend(&self) -> EfiGetVariableBackend {
        self.backend
    }

    pub fn runtime_map(&self) -> &EfiRuntimeMap {
        &self.runtime_map
    }
}

impl GetVariableProvider for EfiRuntimeGetVariableProvider {
    fn get_variable(
        &mut self,
        name: &str,
        vendor: Guid,
        attributes: &mut u32,
        data_size: &mut usize,
        data: Option<&mut [u8]>,
    ) -> vars::EfiStatus {
        let mut name_buf = [0u16; EFI_MAX_VARIABLE_NAME_U16];
        let mut pos = 0usize;
        for unit in name.encode_utf16() {
            if pos + 1 >= name_buf.len() {
                return vars::EFI_INVALID_PARAMETER;
            }
            name_buf[pos] = unit;
            pos += 1;
        }

        let data_ptr = data
            .map(|buf| buf.as_mut_ptr())
            .unwrap_or(core::ptr::null_mut());
        unsafe {
            call_efi_get_variable64(
                self.get_variable_virt,
                name_buf.as_ptr(),
                &vendor,
                attributes,
                data_size,
                data_ptr,
            )
        }
    }
}

type EfiGetVariableFn64 = unsafe extern "win64" fn(
    *const u16,
    *const Guid,
    *mut u32,
    *mut usize,
    *mut u8,
) -> vars::EfiStatus;

type EfiSetVirtualAddressMapFn64 =
    unsafe extern "win64" fn(usize, usize, u32, *const EfiMemoryDescriptor) -> vars::EfiStatus;

/// Linux-shaped guard around EFI runtime firmware calls.
///
/// Source shape:
/// - `vendor/linux/arch/x86/platform/efi/efi_64.c::arch_efi_call_virt_setup`
/// - `vendor/linux/arch/x86/platform/efi/efi_64.c::efi_set_virtual_address_map`
/// - `vendor/linux/drivers/firmware/efi/runtime-wrappers.c::efi_call_virt_check_flags`
///
/// Lupos borrows a dedicated EFI runtime page-table root for firmware calls,
/// matching Linux's `efi_enter_mm()`/`efi_leave_mm()` shape, and still wraps
/// the call boundary with x87/SSE preservation and IRQ flag policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EfiRuntimeCallKind {
    RuntimeService,
    SetVirtualAddressMap,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EfiRuntimeCallOrder {
    enter_mm_before_fpu: bool,
    fpu_end_before_leave_mm: bool,
}

const fn efi_runtime_call_order(kind: EfiRuntimeCallKind) -> EfiRuntimeCallOrder {
    match kind {
        // arch_efi_call_virt_setup()/teardown()
        EfiRuntimeCallKind::RuntimeService => EfiRuntimeCallOrder {
            enter_mm_before_fpu: false,
            fpu_end_before_leave_mm: false,
        },
        // efi_64.c::efi_set_virtual_address_map()
        EfiRuntimeCallKind::SetVirtualAddressMap => EfiRuntimeCallOrder {
            enter_mm_before_fpu: true,
            fpu_end_before_leave_mm: true,
        },
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EfiRuntimeCallGuardAudit {
    pub kind: EfiRuntimeCallKind,
    pub flags_before: IrqFlags,
    pub flags_after: IrqFlags,
    pub fpu_guarded: bool,
    pub interrupts_disabled: bool,
    pub flags_restored: bool,
    pub temporary_mm: bool,
    pub efi_mm_pgd_phys: Option<u64>,
}

fn efi_runtime_call_guard_audit(
    kind: EfiRuntimeCallKind,
    flags_before: IrqFlags,
    flags_after: IrqFlags,
    efi_mm_pgd_phys: Option<u64>,
) -> EfiRuntimeCallGuardAudit {
    let interrupts_disabled = matches!(kind, EfiRuntimeCallKind::SetVirtualAddressMap);
    let flags_restored = matches!(kind, EfiRuntimeCallKind::RuntimeService)
        && ((flags_before ^ flags_after) & ARCH_EFI_IRQ_FLAGS_MASK) != 0;
    EfiRuntimeCallGuardAudit {
        kind,
        flags_before,
        flags_after,
        fpu_guarded: true,
        interrupts_disabled,
        flags_restored,
        temporary_mm: efi_mm_pgd_phys.is_some(),
        efi_mm_pgd_phys,
    }
}

struct EfiRuntimeCallGuard {
    kind: EfiRuntimeCallKind,
    flags_before: IrqFlags,
    irq_restore: Option<IrqFlags>,
    temporary_pgd: Option<crate::arch::x86::mm::paging::TemporaryKernelPgdState>,
    efi_mm_pgd_phys: Option<u64>,
}

impl EfiRuntimeCallGuard {
    fn enter(kind: EfiRuntimeCallKind) -> Self {
        let efi_mm_pgd_phys = runtime_page_tables_snapshot().map(|tables| tables.pgd_phys);
        if let Some(pgd_phys) = efi_mm_pgd_phys {
            let _ = unsafe {
                crate::arch::x86::mm::paging::sync_kernel_mappings_around_window(
                    pgd_phys,
                    EFI_VA_END,
                    EFI_VA_START,
                )
            };
        }

        let order = efi_runtime_call_order(kind);
        let temporary_pgd = if order.enter_mm_before_fpu {
            // Linux reaches SetVirtualAddressMap's early path already pinned;
            // Lupos makes that external precondition explicit here.
            preempt_disable();
            let temporary = efi_mm_pgd_phys.map(|pgd_phys| unsafe {
                crate::arch::x86::mm::paging::use_temporary_kernel_pgd(pgd_phys)
            });
            kernel_fpu_begin_mask(EFI_RUNTIME_FPU_MASK);
            temporary
        } else {
            kernel_fpu_begin_mask(EFI_RUNTIME_FPU_MASK);
            let temporary = efi_mm_pgd_phys.map(|pgd_phys| unsafe {
                crate::arch::x86::mm::paging::use_temporary_kernel_pgd(pgd_phys)
            });
            temporary
        };
        let (flags_before, irq_restore) = match kind {
            EfiRuntimeCallKind::RuntimeService => (arch_local_save_flags(), None),
            EfiRuntimeCallKind::SetVirtualAddressMap => {
                let flags = local_irq_save();
                (flags, Some(flags))
            }
        };
        Self {
            kind,
            flags_before,
            irq_restore,
            temporary_pgd,
            efi_mm_pgd_phys,
        }
    }

    fn exit(self) -> EfiRuntimeCallGuardAudit {
        let flags_after = arch_local_save_flags();
        let audit = efi_runtime_call_guard_audit(
            self.kind,
            self.flags_before,
            flags_after,
            self.efi_mm_pgd_phys,
        );
        match self.irq_restore {
            Some(flags) => local_irq_restore(flags),
            None if audit.flags_restored => {
                crate::kernel::printk::log_warn!(
                    "efi",
                    "EFI runtime call corrupted IRQ flags {:#x}->{:#x}; restoring",
                    self.flags_before,
                    flags_after
                );
                local_irq_restore(self.flags_before);
            }
            None => {}
        }
        let order = efi_runtime_call_order(self.kind);
        if order.fpu_end_before_leave_mm {
            // The explicit outer preempt pin keeps the temporary-mm interval
            // valid after kernel_fpu_end() releases its own BH/preempt guard.
            kernel_fpu_end();
            if let Some(previous) = self.temporary_pgd {
                unsafe {
                    crate::arch::x86::mm::paging::unuse_temporary_kernel_pgd(previous);
                }
            }
            preempt_enable();
        } else {
            if let Some(previous) = self.temporary_pgd {
                unsafe {
                    crate::arch::x86::mm::paging::unuse_temporary_kernel_pgd(previous);
                }
            }
            kernel_fpu_end();
        }
        audit
    }
}

unsafe fn call_efi_set_virtual_address_map64(
    func_addr: u64,
    memory_map_size: usize,
    desc_size: usize,
    desc_version: u32,
    map: *const EfiMemoryDescriptor,
) -> vars::EfiStatus {
    let _runtime_lock = EFI_RUNTIME_CALL_LOCK.lock();
    let guard = EfiRuntimeCallGuard::enter(EfiRuntimeCallKind::SetVirtualAddressMap);
    let func: EfiSetVirtualAddressMapFn64 = unsafe { core::mem::transmute(func_addr as usize) };
    let status = unsafe { func(memory_map_size, desc_size, desc_version, map) };
    let _ = guard.exit();
    status
}

unsafe fn call_efi_get_variable64(
    func_addr: u64,
    name: *const u16,
    vendor: *const Guid,
    attributes: *mut u32,
    data_size: *mut usize,
    data: *mut u8,
) -> vars::EfiStatus {
    let _runtime_lock = EFI_RUNTIME_CALL_LOCK.lock();
    let guard = EfiRuntimeCallGuard::enter(EfiRuntimeCallKind::RuntimeService);
    let func: EfiGetVariableFn64 = unsafe { core::mem::transmute(func_addr as usize) };
    let status = unsafe { func(name, vendor, attributes, data_size, data) };
    let _ = guard.exit();
    status
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use crate::arch::x86::boot::compressed::efi::{
        EFI32_LOADER_SIGNATURE, EFI64_LOADER_SIGNATURE, EfiInfo,
    };
    use crate::arch::x86::include::uapi::asm::bootparam::BootParams;
    use std::sync::Mutex;

    static EFI_TEST_LOCK: Mutex<()> = Mutex::new(());

    macro_rules! efi_test_guard {
        () => {
            let _efi_test_guard = EFI_TEST_LOCK.lock().unwrap();
            *OVMF_RUNTIME.lock() = None;
            *OVMF_RUNTIME_MAP.lock() = None;
            *OVMF_VIRTUAL_MODE.lock() = None;
            *OVMF_EFI_MM.lock() = None;
            vars::unregister_runtime_variables();
        };
    }

    fn boot_params_with_efi_info(info: EfiInfo) -> BootParams {
        let mut bp = BootParams::new();
        bp.set_efi_info(info);
        bp
    }

    fn efi_info64(system_table: u64) -> EfiInfo {
        EfiInfo {
            efi_loader_signature: *EFI64_LOADER_SIGNATURE,
            efi_systab: system_table as u32,
            efi_systab_hi: (system_table >> 32) as u32,
            ..EfiInfo::default()
        }
    }

    #[test]
    fn efi_table_signatures_match_linux_constants() {
        assert_eq!(EFI_SYSTEM_TABLE_SIGNATURE, 0x5453_5953_2049_4249);
        assert_eq!(EFI_RUNTIME_SERVICES_SIGNATURE, 0x5652_4553_544e_5552);
    }

    #[test]
    fn efi64_struct_offsets_match_uefi_layout() {
        assert_eq!(core::mem::size_of::<EfiTableHeader>(), 24);
        assert_eq!(
            core::mem::offset_of!(EfiSystemTable64, runtime_services),
            88
        );
        assert_eq!(
            core::mem::offset_of!(EfiRuntimeServices64, get_variable),
            72
        );
    }

    #[test]
    fn efi_memory_descriptor_layout_matches_linux() {
        assert_eq!(core::mem::size_of::<EfiMemoryDescriptor>(), 40);
        assert_eq!(core::mem::offset_of!(EfiMemoryDescriptor, ty), 0);
        assert_eq!(core::mem::offset_of!(EfiMemoryDescriptor, phys_addr), 8);
        assert_eq!(core::mem::offset_of!(EfiMemoryDescriptor, virt_addr), 16);
        assert_eq!(core::mem::offset_of!(EfiMemoryDescriptor, num_pages), 24);
        assert_eq!(core::mem::offset_of!(EfiMemoryDescriptor, attribute), 32);
        assert_eq!(EFI_RUNTIME_SERVICES_CODE, 5);
        assert_eq!(EFI_RUNTIME_SERVICES_DATA, 6);
        assert_eq!(EFI_MEMORY_DESCRIPTOR_VERSION, 1);
        assert_eq!(EFI_MEMORY_RUNTIME, 1u64 << 63);
        assert_eq!(EFI_VA_START, 0xffff_ffff_0000_0000);
        assert_eq!(EFI_VA_END, 0xffff_ffef_0000_0000);
    }

    #[test]
    fn discovers_efi64_get_variable_from_boot_params_efi_info() {
        let runtime = EfiRuntimeServices64 {
            hdr: EfiTableHeader {
                signature: EFI_RUNTIME_SERVICES_SIGNATURE,
                ..EfiTableHeader::default()
            },
            get_variable: 0xfeed_cafe_dead_beefu64,
            ..EfiRuntimeServices64::default()
        };
        let systab = EfiSystemTable64 {
            hdr: EfiTableHeader {
                signature: EFI_SYSTEM_TABLE_SIGNATURE,
                ..EfiTableHeader::default()
            },
            runtime_services: &runtime as *const _ as u64,
            ..EfiSystemTable64::default()
        };
        let bp = boot_params_with_efi_info(efi_info64(&systab as *const _ as u64));

        let state = unsafe { discover_efi_get_variable_backend_from_boot_params(&bp) }
            .expect("efi backend");

        assert_eq!(state.mode, EfiMode::Efi64);
        assert_eq!(state.system_table_phys, &systab as *const _ as u64);
        assert_eq!(state.runtime_services_phys, &runtime as *const _ as u64);
        assert_eq!(state.get_variable_phys, 0xfeed_cafe_dead_beef);
    }

    #[test]
    fn discovers_efi32_get_variable_from_boot_params_efi_info() {
        let runtime = EfiRuntimeServices32 {
            hdr: EfiTableHeader {
                signature: EFI_RUNTIME_SERVICES_SIGNATURE,
                ..EfiTableHeader::default()
            },
            get_variable: 0x1234_5678,
            ..EfiRuntimeServices32::default()
        };
        let runtime_ptr = &runtime as *const _ as usize;
        if runtime_ptr > u32::MAX as usize {
            return;
        }
        let systab = EfiSystemTable32 {
            hdr: EfiTableHeader {
                signature: EFI_SYSTEM_TABLE_SIGNATURE,
                ..EfiTableHeader::default()
            },
            runtime_services: runtime_ptr as u32,
            ..EfiSystemTable32::default()
        };
        let systab_ptr = &systab as *const _ as usize;
        if systab_ptr > u32::MAX as usize {
            return;
        }
        let bp = boot_params_with_efi_info(EfiInfo {
            efi_loader_signature: *EFI32_LOADER_SIGNATURE,
            efi_systab: systab_ptr as u32,
            ..EfiInfo::default()
        });

        let state = unsafe { discover_efi_get_variable_backend_from_boot_params(&bp) }
            .expect("efi backend");

        assert_eq!(state.mode, EfiMode::Efi32);
        assert_eq!(state.get_variable_phys, 0x1234_5678);
    }

    #[test]
    fn rejects_bad_efi_system_table_signature() {
        let runtime = EfiRuntimeServices64 {
            hdr: EfiTableHeader {
                signature: EFI_RUNTIME_SERVICES_SIGNATURE,
                ..EfiTableHeader::default()
            },
            get_variable: 1,
            ..EfiRuntimeServices64::default()
        };
        let systab = EfiSystemTable64 {
            hdr: EfiTableHeader {
                signature: 0,
                ..EfiTableHeader::default()
            },
            runtime_services: &runtime as *const _ as u64,
            ..EfiSystemTable64::default()
        };
        let bp = boot_params_with_efi_info(efi_info64(&systab as *const _ as u64));

        assert_eq!(
            unsafe { discover_efi_get_variable_backend_from_boot_params(&bp) },
            Err(-EBADMSG)
        );
    }

    #[test]
    fn runtime_map_uses_linux_top_down_virtual_addresses() {
        let entries = [
            EfiMemoryDescriptor {
                ty: EFI_RUNTIME_SERVICES_CODE,
                phys_addr: 0x20_0000,
                num_pages: 2,
                attribute: EFI_MEMORY_RUNTIME | EFI_MEMORY_WB,
                ..EfiMemoryDescriptor::default()
            },
            EfiMemoryDescriptor {
                ty: EFI_RUNTIME_SERVICES_DATA,
                phys_addr: 0x30_1000,
                num_pages: 1,
                attribute: EFI_MEMORY_RUNTIME | EFI_MEMORY_WB,
                ..EfiMemoryDescriptor::default()
            },
            EfiMemoryDescriptor {
                ty: 7,
                phys_addr: 0x40_0000,
                num_pages: 1,
                attribute: EFI_MEMORY_WB,
                ..EfiMemoryDescriptor::default()
            },
        ];
        let map_phys = entries.as_ptr() as u64;
        let bp = boot_params_with_efi_info(EfiInfo {
            efi_loader_signature: *EFI64_LOADER_SIGNATURE,
            efi_memdesc_size: core::mem::size_of::<EfiMemoryDescriptor>() as u32,
            efi_memdesc_version: EFI_MEMORY_DESCRIPTOR_VERSION,
            efi_memmap: map_phys as u32,
            efi_memmap_hi: (map_phys >> 32) as u32,
            efi_memmap_size: core::mem::size_of_val(&entries) as u32,
            ..EfiInfo::default()
        });

        let map = runtime_map_from_boot_params(&bp, EfiMode::Efi64).expect("runtime map");

        assert_eq!(map.desc_version, EFI_MEMORY_DESCRIPTOR_VERSION);
        assert_eq!(map.entries.len(), 2);
        assert_eq!(map.entries[0].phys_addr, 0x20_0000);
        assert_eq!(map.entries[0].virt_addr & (EFI_PMD_SIZE - 1), 0);
        assert!(map.entries[0].virt_addr < EFI_VA_START);
        assert!(map.entries[0].virt_addr >= EFI_VA_END);
        assert_eq!(
            map.entries[1].virt_addr & (EFI_PMD_SIZE - 1),
            entries[1].phys_addr & (EFI_PMD_SIZE - 1)
        );
        assert_eq!(
            map.virtualize_phys(entries[1].phys_addr + 0x80),
            Some(map.entries[1].virt_addr + 0x80)
        );
    }

    #[test]
    fn runtime_map_uses_boot_params_efi_memory_map() {
        let entries = [
            EfiMemoryDescriptor {
                ty: EFI_RUNTIME_SERVICES_CODE,
                phys_addr: 0x20_0000,
                num_pages: 2,
                attribute: EFI_MEMORY_RUNTIME | EFI_MEMORY_WB,
                ..EfiMemoryDescriptor::default()
            },
            EfiMemoryDescriptor {
                ty: 7,
                phys_addr: 0x40_0000,
                num_pages: 1,
                attribute: EFI_MEMORY_WB,
                ..EfiMemoryDescriptor::default()
            },
        ];
        let map_phys = entries.as_ptr() as u64;
        let bp = boot_params_with_efi_info(EfiInfo {
            efi_loader_signature: *EFI64_LOADER_SIGNATURE,
            efi_memdesc_size: core::mem::size_of::<EfiMemoryDescriptor>() as u32,
            efi_memdesc_version: EFI_MEMORY_DESCRIPTOR_VERSION,
            efi_memmap: map_phys as u32,
            efi_memmap_hi: (map_phys >> 32) as u32,
            efi_memmap_size: core::mem::size_of_val(&entries) as u32,
            ..EfiInfo::default()
        });

        let map = runtime_map_from_boot_params(&bp, EfiMode::Efi64).expect("runtime map");

        assert_eq!(map.desc_version, EFI_MEMORY_DESCRIPTOR_VERSION);
        assert_eq!(map.entries.len(), 1);
        assert_eq!(map.entries[0].phys_addr, 0x20_0000);
        assert_eq!(map.entries[0].virt_addr & (EFI_PMD_SIZE - 1), 0);
        assert!(map.entries[0].virt_addr < EFI_VA_START);
        assert!(map.entries[0].virt_addr >= EFI_VA_END);
    }

    #[test]
    fn runtime_map_serializes_descriptor_stride_for_set_virtual_address_map() {
        let desc_len = core::mem::size_of::<EfiMemoryDescriptor>();
        let map = EfiRuntimeMap {
            desc_size: desc_len + 8,
            desc_version: EFI_MEMORY_DESCRIPTOR_VERSION,
            entries: alloc::vec![
                EfiMemoryDescriptor {
                    ty: EFI_RUNTIME_SERVICES_CODE,
                    phys_addr: 0x2000,
                    virt_addr: 0xffff_ffff_0000_0000,
                    num_pages: 1,
                    attribute: EFI_MEMORY_RUNTIME | EFI_MEMORY_WB,
                    ..EfiMemoryDescriptor::default()
                },
                EfiMemoryDescriptor {
                    ty: EFI_RUNTIME_SERVICES_DATA,
                    phys_addr: 0x4000,
                    virt_addr: 0xffff_ffff_0020_0000,
                    num_pages: 2,
                    attribute: EFI_MEMORY_RUNTIME,
                    ..EfiMemoryDescriptor::default()
                },
            ],
        };

        let bytes = map.serialized_descriptors().expect("serialized map");

        assert_eq!(bytes.len(), map.desc_size * map.entries.len());
        let first =
            unsafe { core::ptr::read_unaligned(bytes.as_ptr().cast::<EfiMemoryDescriptor>()) };
        let second = unsafe {
            core::ptr::read_unaligned(
                bytes
                    .as_ptr()
                    .add(map.desc_size)
                    .cast::<EfiMemoryDescriptor>(),
            )
        };
        assert_eq!(first, map.entries[0]);
        assert_eq!(second, map.entries[1]);
        assert!(bytes[desc_len..map.desc_size].iter().all(|byte| *byte == 0));
        assert!(
            bytes[map.desc_size + desc_len..map.desc_size * 2]
                .iter()
                .all(|byte| *byte == 0)
        );

        let bad = EfiRuntimeMap {
            desc_size: desc_len - 1,
            desc_version: EFI_MEMORY_DESCRIPTOR_VERSION,
            entries: map.entries.clone(),
        };
        assert_eq!(bad.serialized_descriptors(), Err(-EINVAL));
    }

    #[test]
    fn efi_runtime_mapping_covers_identity_and_virtual_aliases() {
        efi_test_guard!();
        unsafe {
            crate::arch::x86::mm::paging::reset_test_pool();
        }
        let code = EfiMemoryDescriptor {
            ty: EFI_RUNTIME_SERVICES_CODE,
            phys_addr: 0x0200_0000,
            virt_addr: 0xffff_ffff_0000_0000,
            num_pages: 1,
            attribute: EFI_MEMORY_RUNTIME | EFI_MEMORY_WB,
            ..EfiMemoryDescriptor::default()
        };
        let data = EfiMemoryDescriptor {
            ty: EFI_RUNTIME_SERVICES_DATA,
            phys_addr: 0x0200_1000,
            virt_addr: 0xffff_ffff_0020_1000,
            num_pages: 1,
            attribute: EFI_MEMORY_RUNTIME,
            ..EfiMemoryDescriptor::default()
        };
        let map = EfiRuntimeMap {
            desc_size: core::mem::size_of::<EfiMemoryDescriptor>(),
            desc_version: EFI_MEMORY_DESCRIPTOR_VERSION,
            entries: alloc::vec![code, data],
        };

        let mapped = unsafe { map_efi_runtime_regions(&map) }.expect("map runtime regions");

        assert_eq!(mapped, 4);
        assert_eq!(
            crate::arch::x86::mm::paging::virt_to_phys(code.phys_addr),
            Some(code.phys_addr)
        );
        assert_eq!(
            crate::arch::x86::mm::paging::virt_to_phys(code.virt_addr),
            Some(code.phys_addr)
        );
        assert_eq!(
            crate::arch::x86::mm::paging::virt_to_phys(data.phys_addr),
            Some(data.phys_addr)
        );
        assert_eq!(
            crate::arch::x86::mm::paging::virt_to_phys(data.virt_addr),
            Some(data.phys_addr)
        );

        let code_prot = efi_initial_runtime_pgprot(&code).0;
        let data_prot = efi_initial_runtime_pgprot(&data).0;
        assert_eq!(code_prot & crate::arch::x86::mm::paging::_PAGE_NX, 0);
        assert_eq!(code_prot & crate::arch::x86::mm::paging::_PAGE_PCD, 0);
        assert_ne!(data_prot & crate::arch::x86::mm::paging::_PAGE_NX, 0);
        assert_ne!(data_prot & crate::arch::x86::mm::paging::_PAGE_PCD, 0);
    }

    #[test]
    fn enter_efi_virtual_mode_passes_linux_set_virtual_address_map_shape() {
        efi_test_guard!();
        unsafe {
            crate::arch::x86::mm::paging::reset_test_pool();
        }
        let phys_base = 0x0300_0000;
        let virt_base = 0xffff_ffff_0040_0000;
        let map = EfiRuntimeMap {
            desc_size: core::mem::size_of::<EfiMemoryDescriptor>() + 8,
            desc_version: EFI_MEMORY_DESCRIPTOR_VERSION,
            entries: alloc::vec![EfiMemoryDescriptor {
                ty: EFI_RUNTIME_SERVICES_CODE,
                phys_addr: phys_base,
                virt_addr: virt_base,
                num_pages: 1,
                attribute: EFI_MEMORY_RUNTIME | EFI_MEMORY_WB,
                ..EfiMemoryDescriptor::default()
            }],
        };
        let backend = EfiGetVariableBackend {
            mode: EfiMode::Efi64,
            system_table_phys: 0x1000,
            runtime_services_phys: phys_base + 0x100,
            get_variable_phys: phys_base + 0x200,
            set_virtual_address_map_phys: phys_base + 0x300,
        };
        let mut saw_call = false;

        let virtual_mode =
            enter_efi_virtual_mode_with_caller(backend, &map, |call, descriptors| {
                saw_call = true;
                assert_eq!(call.memory_map_size, map.desc_size);
                assert_eq!(call.desc_size, map.desc_size);
                assert_eq!(call.desc_version, EFI_MEMORY_DESCRIPTOR_VERSION);
                assert_eq!(call.runtime_services_virt, virt_base + 0x100);
                assert_eq!(call.get_variable_virt, virt_base + 0x200);
                assert_eq!(call.set_virtual_address_map_phys, phys_base + 0x300);
                assert_eq!(call.set_virtual_address_map_virt, virt_base + 0x300);
                assert_ne!(call.efi_mm_pgd_phys, 0);
                assert_eq!(call.mapped_pages, 2);
                assert_eq!(
                    crate::arch::x86::mm::paging::virt_to_phys_in_pgd(
                        call.efi_mm_pgd_phys,
                        virt_base
                    ),
                    Some(phys_base)
                );
                assert_eq!(
                    crate::arch::x86::mm::paging::virt_to_phys_in_pgd(
                        call.efi_mm_pgd_phys,
                        phys_base
                    ),
                    Some(phys_base)
                );
                let desc = unsafe { core::ptr::read_unaligned(descriptors) };
                assert_eq!(desc, map.entries[0]);
                vars::EFI_SUCCESS
            })
            .expect("enter efi virtual mode");

        assert!(saw_call);
        assert_eq!(virtual_mode.memory_map_size, map.desc_size);
        assert_eq!(virtual_mode.runtime_services_virt, virt_base + 0x100);
        assert_eq!(virtual_mode.get_variable_virt, virt_base + 0x200);
        assert_eq!(virtual_mode.set_virtual_address_map_virt, virt_base + 0x300);
        assert_ne!(virtual_mode.efi_mm_pgd_phys, 0);
        assert_eq!(virtual_mode.mapped_pages, 2);
    }

    #[test]
    fn efi_mm_syncs_kernel_mappings_but_excludes_runtime_va_window() {
        efi_test_guard!();
        unsafe {
            crate::arch::x86::mm::paging::reset_test_pool();
            crate::arch::x86::mm::paging::map_kernel_page(
                crate::arch::x86::mm::paging::PAGE_OFFSET,
                0x0400_0000,
                crate::arch::x86::mm::paging::PAGE_KERNEL,
            );
            crate::arch::x86::mm::paging::map_kernel_page(
                EFI_VA_START - crate::arch::x86::mm::paging::PUD_SIZE,
                0x0410_0000,
                crate::arch::x86::mm::paging::PAGE_KERNEL,
            );
            crate::arch::x86::mm::paging::map_kernel_page(
                EFI_VA_START,
                0x0420_0000,
                crate::arch::x86::mm::paging::PAGE_KERNEL,
            );
        }

        let page_tables =
            unsafe { allocate_efi_runtime_page_tables() }.expect("allocate efi_mm pgd");

        assert_ne!(page_tables.pgd_phys, 0);
        assert!(page_tables.synced_kernel_entries > 0);
        assert_eq!(
            crate::arch::x86::mm::paging::virt_to_phys_in_pgd(
                page_tables.pgd_phys,
                crate::arch::x86::mm::paging::PAGE_OFFSET
            ),
            Some(0x0400_0000)
        );
        assert_eq!(
            crate::arch::x86::mm::paging::virt_to_phys_in_pgd(page_tables.pgd_phys, EFI_VA_START),
            Some(0x0420_0000)
        );
        assert_eq!(
            crate::arch::x86::mm::paging::virt_to_phys_in_pgd(
                page_tables.pgd_phys,
                EFI_VA_START - crate::arch::x86::mm::paging::PUD_SIZE
            ),
            None
        );
    }

    #[test]
    fn enter_efi_virtual_mode_reports_firmware_failure() {
        efi_test_guard!();
        unsafe {
            crate::arch::x86::mm::paging::reset_test_pool();
        }
        let phys_base = 0x0310_0000;
        let virt_base = 0xffff_ffff_0060_0000;
        let map = EfiRuntimeMap {
            desc_size: core::mem::size_of::<EfiMemoryDescriptor>(),
            desc_version: EFI_MEMORY_DESCRIPTOR_VERSION,
            entries: alloc::vec![EfiMemoryDescriptor {
                ty: EFI_RUNTIME_SERVICES_CODE,
                phys_addr: phys_base,
                virt_addr: virt_base,
                num_pages: 1,
                attribute: EFI_MEMORY_RUNTIME | EFI_MEMORY_WB,
                ..EfiMemoryDescriptor::default()
            }],
        };
        let backend = EfiGetVariableBackend {
            mode: EfiMode::Efi64,
            system_table_phys: 0x1000,
            runtime_services_phys: phys_base,
            get_variable_phys: phys_base,
            set_virtual_address_map_phys: phys_base,
        };

        let err = enter_efi_virtual_mode_with_caller(backend, &map, |_call, _descriptors| {
            vars::EFI_INVALID_PARAMETER
        })
        .expect_err("firmware error should fail virtual mode entry");

        assert_eq!(err, -EIO);
    }

    #[test]
    fn runtime_call_guard_matches_linux_irq_and_fpu_policy() {
        // Origin: vendor/linux/arch/x86/include/asm/efi.h::efi_fpu_begin.
        // UEFI requires both the x87 control word and MXCSR to be initialized.
        assert_eq!(EFI_RUNTIME_FPU_MASK, KFPU_387 | KFPU_MXCSR);

        // Origin: vendor/linux/arch/x86/platform/efi/efi_64.c.
        let runtime_order = efi_runtime_call_order(EfiRuntimeCallKind::RuntimeService);
        assert!(!runtime_order.enter_mm_before_fpu);
        assert!(!runtime_order.fpu_end_before_leave_mm);
        let set_va_order = efi_runtime_call_order(EfiRuntimeCallKind::SetVirtualAddressMap);
        assert!(set_va_order.enter_mm_before_fpu);
        assert!(set_va_order.fpu_end_before_leave_mm);

        let runtime_ok = efi_runtime_call_guard_audit(
            EfiRuntimeCallKind::RuntimeService,
            X86_EFLAGS_IF,
            X86_EFLAGS_IF,
            None,
        );
        assert!(runtime_ok.fpu_guarded);
        assert!(!runtime_ok.interrupts_disabled);
        assert!(!runtime_ok.flags_restored);
        assert!(!runtime_ok.temporary_mm);

        let runtime_corrupted = efi_runtime_call_guard_audit(
            EfiRuntimeCallKind::RuntimeService,
            X86_EFLAGS_IF,
            0,
            None,
        );
        assert!(runtime_corrupted.fpu_guarded);
        assert!(!runtime_corrupted.interrupts_disabled);
        assert!(runtime_corrupted.flags_restored);
        assert!(!runtime_corrupted.temporary_mm);

        let set_va = efi_runtime_call_guard_audit(
            EfiRuntimeCallKind::SetVirtualAddressMap,
            X86_EFLAGS_IF,
            0,
            Some(0x2000),
        );
        assert!(set_va.fpu_guarded);
        assert!(set_va.interrupts_disabled);
        assert!(!set_va.flags_restored);
        assert!(set_va.temporary_mm);
        assert_eq!(set_va.efi_mm_pgd_phys, Some(0x2000));
    }

    unsafe extern "win64" fn fake_set_virtual_address_map(
        memory_map_size: usize,
        desc_size: usize,
        desc_version: u32,
        map: *const EfiMemoryDescriptor,
    ) -> vars::EfiStatus {
        if memory_map_size != desc_size || desc_version != EFI_MEMORY_DESCRIPTOR_VERSION {
            return vars::EFI_INVALID_PARAMETER;
        }
        if map.is_null() {
            return vars::EFI_INVALID_PARAMETER;
        }
        let desc = unsafe { core::ptr::read_unaligned(map) };
        if desc.virt_addr == 0 {
            return vars::EFI_INVALID_PARAMETER;
        }
        vars::EFI_SUCCESS
    }

    #[test]
    fn set_virtual_address_map_call_uses_runtime_guard_wrapper() {
        efi_test_guard!();
        *OVMF_EFI_MM.lock() = None;
        let desc = EfiMemoryDescriptor {
            ty: EFI_RUNTIME_SERVICES_CODE,
            phys_addr: 0x1000,
            virt_addr: 0xffff_ffff_0000_0000,
            num_pages: 1,
            attribute: EFI_MEMORY_RUNTIME | EFI_MEMORY_WB,
            ..EfiMemoryDescriptor::default()
        };

        let status = unsafe {
            call_efi_set_virtual_address_map64(
                fake_set_virtual_address_map as usize as u64,
                core::mem::size_of::<EfiMemoryDescriptor>(),
                core::mem::size_of::<EfiMemoryDescriptor>(),
                EFI_MEMORY_DESCRIPTOR_VERSION,
                &desc,
            )
        };

        assert_eq!(status, vars::EFI_SUCCESS);
    }

    unsafe extern "win64" fn fake_get_variable(
        name: *const u16,
        vendor: *const Guid,
        attributes: *mut u32,
        data_size: *mut usize,
        data: *mut u8,
    ) -> vars::EfiStatus {
        if name.is_null() || vendor.is_null() || attributes.is_null() || data_size.is_null() {
            return vars::EFI_INVALID_PARAMETER;
        }

        let mut units = [0u16; 8];
        let mut len = 0usize;
        while len < units.len() {
            let unit = unsafe { *name.add(len) };
            if unit == 0 {
                break;
            }
            units[len] = unit;
            len += 1;
        }

        if len != 2
            || units[0] != b'd' as u16
            || units[1] != b'b' as u16
            || unsafe { *vendor } != vars::EFI_IMAGE_SECURITY_DATABASE_GUID
        {
            return vars::EFI_NOT_FOUND;
        }

        const VALUE: &[u8] = b"efi-db";
        unsafe {
            *attributes = vars::EFI_VARIABLE_SECURE_BOOT_IMPORT_ATTRS;
            if data.is_null() {
                *data_size = VALUE.len();
                return vars::EFI_BUFFER_TOO_SMALL;
            }
            if *data_size < VALUE.len() {
                *data_size = VALUE.len();
                return vars::EFI_BUFFER_TOO_SMALL;
            }
            core::ptr::copy_nonoverlapping(VALUE.as_ptr(), data, VALUE.len());
            *data_size = VALUE.len();
        }
        vars::EFI_SUCCESS
    }

    #[test]
    fn runtime_get_variable_provider_calls_virtualized_efi_function() {
        efi_test_guard!();
        *OVMF_EFI_MM.lock() = None;
        vars::unregister_runtime_variables();
        let func = fake_get_variable as usize as u64;
        let page_base = func & !(EFI_PAGE_SIZE - 1);
        let runtime_map = EfiRuntimeMap {
            desc_size: core::mem::size_of::<EfiMemoryDescriptor>(),
            desc_version: EFI_MEMORY_DESCRIPTOR_VERSION,
            entries: alloc::vec![EfiMemoryDescriptor {
                ty: EFI_RUNTIME_SERVICES_CODE,
                phys_addr: page_base,
                virt_addr: page_base,
                num_pages: 1,
                attribute: EFI_MEMORY_RUNTIME | EFI_MEMORY_WB,
                ..EfiMemoryDescriptor::default()
            }],
        };
        let backend = EfiGetVariableBackend {
            mode: EfiMode::Efi64,
            system_table_phys: 0x1000,
            runtime_services_phys: 0x2000,
            get_variable_phys: func,
            set_virtual_address_map_phys: 0,
        };
        let mut provider =
            EfiRuntimeGetVariableProvider::new(backend, runtime_map).expect("provider");

        let loaded = vars::register_runtime_variables_from_get_variable_provider(
            &mut provider,
            vars::SECURE_BOOT_VARIABLE_REQUESTS,
        )
        .expect("firmware vars");

        assert_eq!(loaded, 1);
        let db = vars::get_variable("db", vars::EFI_IMAGE_SECURITY_DATABASE_GUID).expect("db");
        assert_eq!(db.data, b"efi-db");
        assert_eq!(db.attributes, vars::EFI_VARIABLE_SECURE_BOOT_IMPORT_ATTRS);
        vars::unregister_runtime_variables();
    }
}
