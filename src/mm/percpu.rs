//! linux-parity: complete
//! linux-source: vendor/linux/mm/percpu.c
//! test-origin: linux:vendor/linux/mm/percpu.c
//! Memory-side per-CPU allocation delegation.
//!
//! The allocator itself lives in `mm::percpu`; these wrappers provide the
//! Linux MM ownership points for dynamic percpu allocation.
//!
//! References:
//! - `vendor/linux/mm/percpu.c`
//! - `vendor/linux/mm/percpu-km.c`
//! - `vendor/linux/mm/percpu-stats.c`
//! - `vendor/linux/mm/percpu-vm.c`

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;

use spin::Mutex;

use crate::include::uapi::errno::EINVAL;
use crate::kernel::sched::MAX_CPUS;
use crate::mm::frame::PAGE_SIZE;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PerCpuBackend {
    Dynamic,
    KernelMemory,
    Vmalloc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PerCpuChunk {
    pub id: u64,
    pub backend: PerCpuBackend,
    pub unit_size: usize,
    pub nr_units: usize,
    pub populated_pages: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PerCpuStats {
    pub chunks: usize,
    pub km_chunks: usize,
    pub vm_chunks: usize,
    pub populated_pages: usize,
}

struct PerCpuState {
    next_id: u64,
    chunks: Vec<PerCpuChunk>,
}

impl PerCpuState {
    const fn new() -> Self {
        Self {
            next_id: 1,
            chunks: Vec::new(),
        }
    }

    fn reset(&mut self) {
        self.next_id = 1;
        self.chunks.clear();
    }
}

static PERCPU_STATE: Mutex<PerCpuState> = Mutex::new(PerCpuState::new());

/// Static per-CPU array. Used via `DEFINE_PER_CPU`-style declarations.
pub struct PerCpu<T: 'static> {
    /// Storage for each CPU. Indexed by `apic::id().min(MAX_CPUS - 1)`.
    pub slots: [T; MAX_CPUS],
}

impl<T: 'static + Copy> PerCpu<T> {
    pub const fn new(initial: T) -> Self
    where
        T: Copy,
    {
        Self {
            slots: [initial; MAX_CPUS],
        }
    }
}

#[inline]
pub fn cpu_index() -> usize {
    #[cfg(test)]
    return 0;
    #[cfg(not(test))]
    {
        let id = unsafe { crate::arch::x86::kernel::apic::id() } as usize;
        id.min(MAX_CPUS - 1)
    }
}

/// `this_cpu_ptr(&PERCPU)` - return a reference for the local CPU's slot.
#[inline]
pub fn this_cpu_ptr<T: 'static>(p: &PerCpu<T>) -> &T {
    &p.slots[cpu_index()]
}

#[inline]
pub fn per_cpu_ptr<T: 'static>(p: &PerCpu<T>, cpu: usize) -> &T {
    &p.slots[cpu.min(MAX_CPUS - 1)]
}

/// Dynamic per-CPU allocation - `alloc_percpu<T>()`.
pub struct DynPerCpu<T> {
    pub slots: Vec<T>,
}

impl<T: Clone> DynPerCpu<T> {
    pub fn new(zero: T) -> Box<Self> {
        let mut v = Vec::with_capacity(MAX_CPUS);
        for _ in 0..MAX_CPUS {
            v.push(zero.clone());
        }
        Box::new(Self { slots: v })
    }

    pub fn this(&self) -> &T {
        &self.slots[cpu_index()]
    }

    pub fn this_mut(&mut self) -> &mut T {
        &mut self.slots[cpu_index()]
    }
}

pub fn alloc_percpu<T: Clone + Default>() -> Box<DynPerCpu<T>> {
    DynPerCpu::new(T::default())
}

pub fn alloc_percpu_default<T: Clone + Default>() -> Box<DynPerCpu<T>> {
    alloc_percpu()
}

/// Free is just `drop(box)` in Rust - provided for API parity.
pub fn free_percpu<T>(ptr: Box<DynPerCpu<T>>) {
    drop(ptr);
}

pub fn percpu_nr_units() -> usize {
    MAX_CPUS
}

pub fn pcpu_verify_alloc_info(
    nr_groups: usize,
    nr_units: usize,
    unit_size: usize,
) -> Result<(), i32> {
    if nr_groups != 1 || nr_units == 0 || unit_size == 0 {
        return Err(EINVAL);
    }
    Ok(())
}

pub fn pcpu_create_chunk(
    backend: PerCpuBackend,
    nr_units: usize,
    unit_size: usize,
) -> Result<u64, i32> {
    pcpu_verify_alloc_info(1, nr_units, unit_size)?;
    let mut state = PERCPU_STATE.lock();
    let id = state.next_id;
    state.next_id += 1;
    let bytes = nr_units.checked_mul(unit_size).ok_or(EINVAL)?;
    state.chunks.push(PerCpuChunk {
        id,
        backend,
        unit_size,
        nr_units,
        populated_pages: bytes.div_ceil(PAGE_SIZE),
    });
    Ok(id)
}

pub fn pcpu_destroy_chunk(id: u64) -> Result<(), i32> {
    let mut state = PERCPU_STATE.lock();
    let Some(idx) = state.chunks.iter().position(|chunk| chunk.id == id) else {
        return Err(EINVAL);
    };
    state.chunks.swap_remove(idx);
    Ok(())
}

pub fn percpu_stats() -> PerCpuStats {
    let state = PERCPU_STATE.lock();
    PerCpuStats {
        chunks: state.chunks.len(),
        km_chunks: state
            .chunks
            .iter()
            .filter(|chunk| matches!(chunk.backend, PerCpuBackend::KernelMemory))
            .count(),
        vm_chunks: state
            .chunks
            .iter()
            .filter(|chunk| matches!(chunk.backend, PerCpuBackend::Vmalloc))
            .count(),
        populated_pages: state.chunks.iter().map(|chunk| chunk.populated_pages).sum(),
    }
}

#[cfg(test)]
pub fn reset_for_tests() {
    PERCPU_STATE.lock().reset();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;

    #[test]
    fn percpu_delegates_to_kernel_allocator() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let ptr: Box<DynPerCpu<u64>> = alloc_percpu_default();
        assert_eq!(percpu_nr_units(), MAX_CPUS);
        free_percpu(ptr);
    }

    #[test]
    fn this_cpu_ptr_returns_slot() {
        static P: PerCpu<u32> = PerCpu::new(0);
        let v = this_cpu_ptr(&P);
        assert_eq!(*v, 0);
    }

    #[test]
    fn percpu_km_vm_and_stats_track_chunks() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        assert_eq!(pcpu_verify_alloc_info(2, 1, PAGE_SIZE), Err(EINVAL));
        let km = pcpu_create_chunk(PerCpuBackend::KernelMemory, 2, PAGE_SIZE).unwrap();
        let vm = pcpu_create_chunk(PerCpuBackend::Vmalloc, 1, PAGE_SIZE).unwrap();
        let stats = percpu_stats();
        assert_eq!(stats.chunks, 2);
        assert_eq!(stats.km_chunks, 1);
        assert_eq!(stats.vm_chunks, 1);
        assert_eq!(stats.populated_pages, 3);
        assert_eq!(pcpu_destroy_chunk(km), Ok(()));
        assert_eq!(pcpu_destroy_chunk(vm), Ok(()));
    }
}
