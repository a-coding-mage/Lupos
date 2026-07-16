//! linux-parity: partial
//! linux-source: vendor/linux/kernel/trace/bpf_trace.c
//! test-origin: linux:vendor/linux/tools/testing/selftests/bpf
//! Module ownership for `__bpf_raw_tp_map`.

extern crate alloc;

use alloc::vec::Vec;

use spin::Mutex;

/// `struct bpf_raw_event_map` is 32-byte aligned and 32 bytes wide on the
/// pinned x86-64 vendor configuration.
pub const BPF_RAW_EVENT_MAP_SIZE: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RawTracepointMap {
    pub owner: usize,
    pub address: usize,
}

#[derive(Clone, Copy, Debug)]
struct ModuleRawTracepoints {
    owner: usize,
    start: usize,
    count: usize,
    references: usize,
    accepting: bool,
}

static MODULES: Mutex<Vec<ModuleRawTracepoints>> = Mutex::new(Vec::new());

/// BPF module notifier `MODULE_STATE_COMING` path.
pub fn module_coming(owner: usize, section_address: usize, section_len: usize) -> Result<(), i32> {
    if section_len % BPF_RAW_EVENT_MAP_SIZE != 0
        || (section_len != 0 && section_address % BPF_RAW_EVENT_MAP_SIZE != 0)
    {
        return Err(-8); // ENOEXEC
    }

    let mut modules = MODULES.lock();
    if modules.iter().any(|module| module.owner == owner) {
        return Err(-17); // EEXIST
    }
    if section_len != 0 {
        modules.push(ModuleRawTracepoints {
            owner,
            start: section_address,
            count: section_len / BPF_RAW_EVENT_MAP_SIZE,
            references: 0,
            accepting: true,
        });
    }
    Ok(())
}

unsafe fn tracepoint_name_matches(map_address: usize, name: &str) -> bool {
    // `struct bpf_raw_event_map.tp` is its first field.
    let tracepoint = unsafe { (map_address as *const usize).read_unaligned() };
    if tracepoint == 0 {
        return false;
    }
    // `struct tracepoint.name` is its first field.
    let name_address = unsafe { (tracepoint as *const usize).read_unaligned() };
    if name_address == 0 {
        return false;
    }
    let bytes = name.as_bytes();
    let pointer = name_address as *const u8;
    for (index, expected) in bytes.iter().copied().enumerate() {
        if unsafe { pointer.add(index).read() } != expected {
            return false;
        }
    }
    unsafe { pointer.add(bytes.len()).read() == 0 }
}

/// `bpf_get_raw_tracepoint_module()`.  A successful get pins the owner until
/// the matching `put()`.
///
/// # Safety
/// Registered maps and their tracepoint/name pointers must still refer to
/// relocated module memory.  The COMING/GOING lifecycle guarantees that when
/// callers pair this operation with `put()`.
pub unsafe fn get(name: &str) -> Option<RawTracepointMap> {
    let mut modules = MODULES.lock();
    for module in modules.iter_mut() {
        if !module.accepting {
            continue;
        }
        for index in 0..module.count {
            let address = module
                .start
                .checked_add(index.checked_mul(BPF_RAW_EVENT_MAP_SIZE)?)?;
            if unsafe { tracepoint_name_matches(address, name) } {
                module.references = module.references.checked_add(1)?;
                return Some(RawTracepointMap {
                    owner: module.owner,
                    address,
                });
            }
        }
    }
    None
}

/// Freeze new raw-tracepoint references before `cleanup_module()` begins.
///
/// Linux obtains the same exclusion by changing `mod->state` under
/// `module_mutex` after its base reference has been removed.  The raw-map
/// registry has its own lock in Lupos, so this explicit freeze closes the
/// check-to-GOING race without dismantling notifier metadata before the
/// module's exit function has run.
pub fn module_begin_going(owner: usize) -> Result<(), i32> {
    let mut modules = MODULES.lock();
    let Some(module) = modules.iter_mut().find(|module| module.owner == owner) else {
        return Ok(());
    };
    if module.references != 0 {
        return Err(-16); // EBUSY
    }
    module.accepting = false;
    Ok(())
}

pub fn put(map: RawTracepointMap) {
    if let Some(module) = MODULES
        .lock()
        .iter_mut()
        .find(|module| module.owner == map.owner)
    {
        module.references = module.references.saturating_sub(1);
    }
}

/// BPF module notifier `MODULE_STATE_GOING` path.  Linux's module refcount
/// prevents this state while a raw tracepoint map is held.
pub fn module_going(owner: usize) -> Result<(), i32> {
    let mut modules = MODULES.lock();
    let Some(index) = modules.iter().position(|module| module.owner == owner) else {
        return Ok(());
    };
    if modules[index].references != 0 {
        return Err(-16); // EBUSY
    }
    modules.remove(index);
    Ok(())
}

pub fn module_maps(owner: usize) -> Vec<RawTracepointMap> {
    let modules = MODULES.lock();
    let Some(module) = modules.iter().find(|module| module.owner == owner) else {
        return Vec::new();
    };
    (0..module.count)
        .filter_map(|index| {
            Some(RawTracepointMap {
                owner,
                address: module
                    .start
                    .checked_add(index.checked_mul(BPF_RAW_EVENT_MAP_SIZE)?)?,
            })
        })
        .collect()
}
