//! linux-parity: partial
//! linux-source: vendor/linux/kernel/trace/trace_events.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_events.c
//! `events/<subsystem>/<event>/enable` framework — registers static
//! tracepoint event classes.
//!
//! Ref: vendor/linux/kernel/trace/trace_events.c

extern crate alloc;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use core::cell::UnsafeCell;
use core::ffi::c_void;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use spin::Mutex;

use crate::kernel::module::{export_symbol, find_symbol};

#[derive(Clone, Debug)]
pub struct TraceEventClass {
    pub subsystem: String,
    pub name: String,
    pub enabled: bool,
}

static CLASSES: Mutex<Vec<TraceEventClass>> = Mutex::new(Vec::new());

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleTraceEvent {
    pub owner: usize,
    pub call: usize,
    pub event_type: u32,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleTraceEvalMap {
    pub owner: usize,
    pub map: usize,
}

pub const TRACE_EVENT_CALL_CLASS_OFFSET: usize = 16;
pub const TRACE_EVENT_CALL_TP_OFFSET: usize = 24;
pub const TRACE_EVENT_CALL_EVENT_TYPE_OFFSET: usize = 48;
pub const TRACE_EVENT_CALL_FLAGS_OFFSET: usize = 88;
pub const TRACE_EVENT_CALL_SIZE: usize = 120;
pub const TRACE_EVENT_CLASS_PROBE_OFFSET: usize = 8;
pub const TRACE_EVENT_CLASS_PERF_PROBE_OFFSET: usize = 16;
pub const TRACE_EVENT_CLASS_REG_OFFSET: usize = 24;
pub const TRACE_EVENT_CLASS_RAW_INIT_OFFSET: usize = 64;
pub const TRACE_EVENT_CLASS_SIZE: usize = 72;
const TRACE_EVENT_FL_TRACEPOINT: i32 = 1 << 3;
const EVENT_FILE_FL_ENABLED: usize = 1 << 0;
const EVENT_FILE_FL_SOFT_DISABLED: usize = 1 << 5;
const EVENT_FILE_FL_WAS_ENABLED: usize = 1 << 9;

#[repr(C)]
struct LinuxTraceEventFile {
    list: [usize; 2],
    event_call: usize,
    filter: usize,
    eventfs_inode: usize,
    trace_array: usize,
    system: usize,
    triggers: [usize; 2],
    flags: AtomicUsize,
    reference: AtomicU32,
    soft_mode_ref: AtomicU32,
    trigger_mode_ref: AtomicU32,
}

impl LinuxTraceEventFile {
    fn new(event_call: usize) -> Self {
        Self {
            list: [0; 2],
            event_call,
            filter: 0,
            eventfs_inode: 0,
            trace_array: 0,
            system: 0,
            triggers: [0; 2],
            flags: AtomicUsize::new(0),
            reference: AtomicU32::new(1),
            soft_mode_ref: AtomicU32::new(0),
            trigger_mode_ref: AtomicU32::new(0),
        }
    }
}

struct RegisteredModuleTraceEvent {
    owner: usize,
    call: usize,
    event_type: u32,
    file: Box<LinuxTraceEventFile>,
    enabled: bool,
}

static MODULE_EVENTS: Mutex<Vec<RegisteredModuleTraceEvent>> = Mutex::new(Vec::new());
static MODULE_EVAL_MAPS: Mutex<Vec<ModuleTraceEvalMap>> = Mutex::new(Vec::new());

static NEXT_TRACE_EVENT_TYPE: AtomicU32 = AtomicU32::new(0x200);

pub const GENERATED_TRACE_MAX_PAYLOAD: usize = 8192;
const GENERATED_TRACE_RING_SIZE: usize = 32;

struct GeneratedTraceReservation {
    active: AtomicBool,
    len: AtomicUsize,
    event_type: AtomicU32,
    bytes: UnsafeCell<[u8; GENERATED_TRACE_MAX_PAYLOAD]>,
}

unsafe impl Sync for GeneratedTraceReservation {}

impl GeneratedTraceReservation {
    const fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
            len: AtomicUsize::new(0),
            event_type: AtomicU32::new(0),
            bytes: UnsafeCell::new([0; GENERATED_TRACE_MAX_PAYLOAD]),
        }
    }
}

static GENERATED_RESERVATIONS: [GeneratedTraceReservation; crate::kernel::sched::MAX_CPUS] =
    [const { GeneratedTraceReservation::new() }; crate::kernel::sched::MAX_CPUS];

#[derive(Clone, Copy)]
struct GeneratedTraceSlot {
    event_type: u32,
    len: usize,
    bytes: [u8; GENERATED_TRACE_MAX_PAYLOAD],
}

impl GeneratedTraceSlot {
    const fn empty() -> Self {
        Self {
            event_type: 0,
            len: 0,
            bytes: [0; GENERATED_TRACE_MAX_PAYLOAD],
        }
    }
}

static GENERATED_TRACE_RING: Mutex<[GeneratedTraceSlot; GENERATED_TRACE_RING_SIZE]> =
    Mutex::new([GeneratedTraceSlot::empty(); GENERATED_TRACE_RING_SIZE]);
static GENERATED_TRACE_HEAD: AtomicU64 = AtomicU64::new(0);
static GENERATED_TRACE_TAIL: AtomicU64 = AtomicU64::new(0);

#[repr(C)]
struct LinuxTraceEventBuffer {
    buffer: usize,
    event: usize,
    trace_file: usize,
    entry: usize,
    trace_ctx: u32,
    _pad: u32,
    regs: usize,
}

fn read_word(address: usize, offset: usize) -> Option<usize> {
    if address == 0
        || address
            .checked_add(offset + core::mem::size_of::<usize>())
            .is_none()
    {
        return None;
    }
    Some(unsafe { ((address + offset) as *const usize).read_volatile() })
}

fn read_i32_at(address: usize, offset: usize) -> Option<i32> {
    if address == 0 || address.checked_add(offset + 4).is_none() {
        return None;
    }
    Some(unsafe { ((address + offset) as *const i32).read_volatile() })
}

fn event_type(call: usize) -> Option<u32> {
    if call == 0 {
        return None;
    }
    Some(unsafe {
        ((call + TRACE_EVENT_CALL_EVENT_TYPE_OFFSET) as *const i32).read_volatile() as u32
    })
}

fn commit_generated(reservation: &GeneratedTraceReservation) {
    let len = reservation
        .len
        .load(Ordering::Acquire)
        .min(GENERATED_TRACE_MAX_PAYLOAD);
    let event_type = reservation.event_type.load(Ordering::Acquire);
    if len == 0 || event_type == 0 {
        reservation.active.store(false, Ordering::Release);
        return;
    }
    let head = GENERATED_TRACE_HEAD.fetch_add(1, Ordering::AcqRel);
    let mut ring = GENERATED_TRACE_RING.lock();
    let slot = &mut ring[head as usize % GENERATED_TRACE_RING_SIZE];
    slot.event_type = event_type;
    slot.len = len;
    unsafe {
        let bytes = &*reservation.bytes.get();
        slot.bytes[..len].copy_from_slice(&bytes[..len]);
    }
    let new_head = head + 1;
    let tail = GENERATED_TRACE_TAIL.load(Ordering::Acquire);
    if new_head.saturating_sub(tail) > GENERATED_TRACE_RING_SIZE as u64 {
        GENERATED_TRACE_TAIL.store(
            new_head - GENERATED_TRACE_RING_SIZE as u64,
            Ordering::Release,
        );
    }
    reservation.active.store(false, Ordering::Release);
}

pub fn generated_event_count(event_type: u32) -> usize {
    let head = GENERATED_TRACE_HEAD.load(Ordering::Acquire);
    let tail = GENERATED_TRACE_TAIL.load(Ordering::Acquire);
    let ring = GENERATED_TRACE_RING.lock();
    (tail..head)
        .filter(|sequence| {
            ring[*sequence as usize % GENERATED_TRACE_RING_SIZE].event_type == event_type
        })
        .count()
}

pub fn latest_generated_payload(event_type: u32, output: &mut [u8]) -> Option<usize> {
    let head = GENERATED_TRACE_HEAD.load(Ordering::Acquire);
    let tail = GENERATED_TRACE_TAIL.load(Ordering::Acquire);
    let ring = GENERATED_TRACE_RING.lock();
    for sequence in (tail..head).rev() {
        let slot = &ring[sequence as usize % GENERATED_TRACE_RING_SIZE];
        if slot.event_type != event_type {
            continue;
        }
        let len = slot.len.min(output.len());
        output[..len].copy_from_slice(&slot.bytes[..len]);
        return Some(slot.len);
    }
    None
}

// `DEFINE_SRCU_FAST(tracepoint_srcu)` starts with srcu_ctrp and sda.  Module
// tracepoint fast paths only consume the first pointer and then update its
// adjacent lock/unlock counters through %gs.  The remaining words retain the
// configured x86-64 prefix of struct srcu_struct and stay zero until a fuller
// grace-period implementation needs them.
static TRACEPOINT_SRCU: [AtomicUsize; 4] = [const { AtomicUsize::new(0) }; 4];
static EMPTY_TRACE_TEXT: [u8; 1] = [0];

fn export_symbol_once(name: &'static str, address: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, address, gpl_only);
    }
}

/// Install the generated-trace-event ABI referenced by unchanged Kbuild
/// modules.  Event/perf recording paths which do not yet have a Linux
/// `trace_event_file` return a disabled result instead of fabricating one;
/// ordinary disabled static tracepoints therefore retain Linux's zero-cost
/// behavior while their metadata remains loadable and discoverable.
pub fn register_module_exports() {
    crate::kernel::trace::blktrace::register_module_exports();
    crate::kernel::trace::trace_seq::register_module_exports();
    crate::kernel::trace::tracepoint::register_builtin_module_exports();
    let counters = crate::arch::x86::kernel::setup_percpu::tracepoint_srcu_counters_symbol();
    TRACEPOINT_SRCU[0].store(counters, Ordering::Release);
    TRACEPOINT_SRCU[1].store(counters, Ordering::Release);
    export_symbol_once("tracepoint_srcu", TRACEPOINT_SRCU.as_ptr() as usize, true);
    export_symbol_once(
        "trace_raw_output_prep",
        linux_trace_raw_output_prep as usize,
        false,
    );
    export_symbol_once(
        "trace_event_printf",
        linux_trace_event_printf as usize,
        false,
    );
    export_symbol_once(
        "trace_handle_return",
        linux_trace_handle_return as usize,
        true,
    );
    export_symbol_once(
        "trace_print_symbols_seq",
        linux_trace_print_symbols_seq as usize,
        false,
    );
    export_symbol_once(
        "trace_event_raw_init",
        linux_trace_event_raw_init as usize,
        true,
    );
    export_symbol_once("trace_event_reg", linux_trace_event_reg as usize, true);
    export_symbol_once(
        "trace_event_buffer_reserve",
        linux_trace_event_buffer_reserve as usize,
        true,
    );
    export_symbol_once(
        "trace_event_buffer_commit",
        linux_trace_event_buffer_commit as usize,
        true,
    );
    export_symbol_once(
        "perf_trace_buf_alloc",
        linux_perf_trace_buf_alloc as usize,
        true,
    );
    export_symbol_once(
        "perf_trace_run_bpf_submit",
        linux_perf_trace_run_bpf_submit as usize,
        true,
    );
    export_symbol_once(
        "__trace_trigger_soft_disabled",
        linux_trace_trigger_soft_disabled as usize,
        true,
    );
}

#[unsafe(export_name = "trace_raw_output_prep")]
unsafe extern "C" fn linux_trace_raw_output_prep(_iter: *mut c_void, _event: *mut c_void) -> i32 {
    2 // TRACE_TYPE_UNHANDLED
}

// The actual C function is variadic.  SysV AMD64 permits this fixed-prefix
// entry point to ignore all additional register/stack arguments safely.
#[unsafe(export_name = "trace_event_printf")]
unsafe extern "C" fn linux_trace_event_printf(_iter: *mut c_void, _fmt: *const u8) {}

#[unsafe(export_name = "trace_handle_return")]
unsafe extern "C" fn linux_trace_handle_return(_seq: *mut c_void) -> i32 {
    1 // TRACE_TYPE_HANDLED
}

#[unsafe(export_name = "trace_print_symbols_seq")]
unsafe extern "C" fn linux_trace_print_symbols_seq(
    _seq: *mut c_void,
    _value: usize,
    _symbols: *const c_void,
    _count: usize,
) -> *const u8 {
    EMPTY_TRACE_TEXT.as_ptr()
}

#[unsafe(export_name = "trace_event_raw_init")]
unsafe extern "C" fn linux_trace_event_raw_init(call: *mut c_void) -> i32 {
    if call.is_null() {
        return -22; // EINVAL
    }
    let call = call as usize;
    let event_type = event_type(call).unwrap_or(0);
    if event_type != 0 {
        return 0;
    }
    let event_type = NEXT_TRACE_EVENT_TYPE.fetch_add(1, Ordering::AcqRel);
    if event_type == 0 || event_type > u16::MAX as u32 {
        return -19; // ENODEV
    }
    unsafe {
        ((call + TRACE_EVENT_CALL_EVENT_TYPE_OFFSET) as *mut i32).write_volatile(event_type as i32);
    }
    0
}

#[unsafe(export_name = "trace_event_reg")]
unsafe extern "C" fn linux_trace_event_reg(
    call: *mut c_void,
    registration_type: i32,
    data: *mut c_void,
) -> i32 {
    if call.is_null() {
        return -22;
    }
    let call = call as usize;
    if read_i32_at(call, TRACE_EVENT_CALL_FLAGS_OFFSET)
        .is_none_or(|flags| flags & TRACE_EVENT_FL_TRACEPOINT == 0)
    {
        return -22;
    }
    let Some(tracepoint) = read_word(call, TRACE_EVENT_CALL_TP_OFFSET) else {
        return -22;
    };
    let Some(class) = read_word(call, TRACE_EVENT_CALL_CLASS_OFFSET) else {
        return -22;
    };
    match registration_type {
        0 => {
            let Some(probe) = read_word(class, TRACE_EVENT_CLASS_PROBE_OFFSET) else {
                return -22;
            };
            match unsafe {
                crate::kernel::trace::tracepoint::register_module_probe(
                    tracepoint,
                    probe,
                    data as usize,
                )
            } {
                Ok(()) => 0,
                Err(error) => error,
            }
        }
        1 => {
            let Some(probe) = read_word(class, TRACE_EVENT_CLASS_PROBE_OFFSET) else {
                return -22;
            };
            match unsafe {
                crate::kernel::trace::tracepoint::unregister_module_probe(
                    tracepoint,
                    probe,
                    data as usize,
                )
            } {
                Ok(()) => 0,
                Err(error) => error,
            }
        }
        2 => {
            let Some(probe) = read_word(class, TRACE_EVENT_CLASS_PERF_PROBE_OFFSET) else {
                return -19;
            };
            if probe == 0 {
                return -19;
            }
            match unsafe {
                crate::kernel::trace::tracepoint::register_module_probe(tracepoint, probe, call)
            } {
                Ok(()) => 0,
                Err(error) => error,
            }
        }
        3 => {
            let Some(probe) = read_word(class, TRACE_EVENT_CLASS_PERF_PROBE_OFFSET) else {
                return -22;
            };
            match unsafe {
                crate::kernel::trace::tracepoint::unregister_module_probe(tracepoint, probe, call)
            } {
                Ok(()) => 0,
                Err(error) => error,
            }
        }
        4..=7 => 0,
        _ => -22,
    }
}

#[unsafe(export_name = "trace_event_buffer_reserve")]
unsafe extern "C" fn linux_trace_event_buffer_reserve(
    buffer: *mut c_void,
    file: *mut c_void,
    len: usize,
) -> *mut c_void {
    if buffer.is_null() || file.is_null() || !(8..=GENERATED_TRACE_MAX_PAYLOAD).contains(&len) {
        return core::ptr::null_mut();
    }
    let file = unsafe { &*(file as *const LinuxTraceEventFile) };
    let flags = file.flags.load(Ordering::Acquire);
    if flags & EVENT_FILE_FL_ENABLED == 0 || flags & EVENT_FILE_FL_SOFT_DISABLED != 0 {
        return core::ptr::null_mut();
    }
    let Some(event_type) = event_type(file.event_call) else {
        return core::ptr::null_mut();
    };
    let cpu = crate::kernel::sched::current_cpu() as usize;
    let reservation = &GENERATED_RESERVATIONS[cpu.min(GENERATED_RESERVATIONS.len() - 1)];
    if reservation
        .active
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return core::ptr::null_mut();
    }
    reservation.len.store(len, Ordering::Release);
    reservation.event_type.store(event_type, Ordering::Release);
    let bytes = unsafe { &mut *reservation.bytes.get() };
    bytes[..len].fill(0);
    bytes[0..2].copy_from_slice(&(event_type as u16).to_le_bytes());
    let current = unsafe { crate::kernel::sched::get_current() };
    let pid = if current.is_null() {
        0
    } else {
        unsafe { (*current).pid }
    };
    bytes[4..8].copy_from_slice(&pid.to_le_bytes());

    let fbuffer = unsafe { &mut *(buffer as *mut LinuxTraceEventBuffer) };
    *fbuffer = LinuxTraceEventBuffer {
        buffer: 0,
        event: reservation as *const _ as usize,
        trace_file: file as *const _ as usize,
        entry: bytes.as_mut_ptr() as usize,
        trace_ctx: 0,
        _pad: 0,
        regs: 0,
    };
    bytes.as_mut_ptr().cast()
}

#[unsafe(export_name = "trace_event_buffer_commit")]
unsafe extern "C" fn linux_trace_event_buffer_commit(buffer: *mut c_void) {
    if buffer.is_null() {
        return;
    }
    let fbuffer = unsafe { &mut *(buffer as *mut LinuxTraceEventBuffer) };
    if fbuffer.event == 0 {
        return;
    }
    let reservation = unsafe { &*(fbuffer.event as *const GeneratedTraceReservation) };
    commit_generated(reservation);
    fbuffer.event = 0;
    fbuffer.entry = 0;
}

#[unsafe(export_name = "perf_trace_buf_alloc")]
unsafe extern "C" fn linux_perf_trace_buf_alloc(
    size: i32,
    regs: *mut *mut c_void,
    recursion_context: *mut i32,
) -> *mut c_void {
    if size < 8 || size as usize > GENERATED_TRACE_MAX_PAYLOAD {
        return core::ptr::null_mut();
    }
    let cpu = crate::kernel::sched::current_cpu() as usize;
    let reservation = &GENERATED_RESERVATIONS[cpu.min(GENERATED_RESERVATIONS.len() - 1)];
    if reservation
        .active
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return core::ptr::null_mut();
    }
    reservation.len.store(size as usize, Ordering::Release);
    reservation.event_type.store(0, Ordering::Release);
    unsafe { (&mut *reservation.bytes.get())[..size as usize].fill(0) };
    if !regs.is_null() {
        unsafe { regs.write(core::ptr::null_mut()) };
    }
    if !recursion_context.is_null() {
        unsafe { recursion_context.write(cpu as i32) };
    }
    unsafe { (&mut *reservation.bytes.get()).as_mut_ptr().cast() }
}

#[unsafe(export_name = "perf_trace_run_bpf_submit")]
unsafe extern "C" fn linux_perf_trace_run_bpf_submit(
    entry: *mut c_void,
    size: u32,
    recursion_context: i32,
    call: *mut c_void,
    _count: u64,
    _regs: *mut c_void,
    _head: *mut c_void,
    _task: *mut c_void,
) {
    if entry.is_null() || call.is_null() || recursion_context < 0 {
        return;
    }
    let cpu = recursion_context as usize;
    let Some(reservation) = GENERATED_RESERVATIONS.get(cpu) else {
        return;
    };
    if reservation.bytes.get() as *mut u8 != entry.cast::<u8>() {
        reservation.active.store(false, Ordering::Release);
        return;
    }
    reservation.len.store(
        (size as usize).min(GENERATED_TRACE_MAX_PAYLOAD),
        Ordering::Release,
    );
    reservation
        .event_type
        .store(event_type(call as usize).unwrap_or(0), Ordering::Release);
    commit_generated(reservation);
}

#[unsafe(export_name = "__trace_trigger_soft_disabled")]
unsafe extern "C" fn linux_trace_trigger_soft_disabled(file: *mut c_void) -> bool {
    if file.is_null() {
        return true;
    }
    unsafe { &*(file as *const LinuxTraceEventFile) }
        .flags
        .load(Ordering::Acquire)
        & EVENT_FILE_FL_SOFT_DISABLED
        != 0
}

fn relocated_pointer_table(section: &[u8]) -> Result<Vec<usize>, i32> {
    if section.len() % core::mem::size_of::<usize>() != 0 {
        return Err(-8); // ENOEXEC
    }
    let mut pointers = Vec::with_capacity(section.len() / core::mem::size_of::<usize>());
    for bytes in section.chunks_exact(core::mem::size_of::<usize>()) {
        let pointer = usize::from_le_bytes(bytes.try_into().map_err(|_| -8)?);
        if pointer == 0 {
            return Err(-8);
        }
        pointers.push(pointer);
    }
    Ok(pointers)
}

/// `trace_module_add_events()` plus `trace_module_add_evals()` at
/// `MODULE_STATE_COMING`.  The section contents are relocated pointer arrays;
/// event objects and eval maps remain owned by module memory.
pub fn module_coming(owner: usize, events: &[u8], eval_maps: &[u8]) -> Result<(), i32> {
    let events = relocated_pointer_table(events)?;
    let eval_maps = relocated_pointer_table(eval_maps)?;

    let mut registered_events = MODULE_EVENTS.lock();
    let mut registered_evals = MODULE_EVAL_MAPS.lock();
    if registered_events.iter().any(|event| event.owner == owner)
        || registered_evals.iter().any(|map| map.owner == owner)
    {
        return Err(-17); // EEXIST
    }

    let mut initialized = Vec::with_capacity(events.len());
    for call in events {
        let flags = read_i32_at(call, TRACE_EVENT_CALL_FLAGS_OFFSET).ok_or(-8)?;
        if flags & TRACE_EVENT_FL_TRACEPOINT == 0
            || read_word(call, TRACE_EVENT_CALL_CLASS_OFFSET).is_none_or(|value| value == 0)
            || read_word(call, TRACE_EVENT_CALL_TP_OFFSET).is_none_or(|value| value == 0)
        {
            return Err(-8); // ENOEXEC
        }
        let init = unsafe { linux_trace_event_raw_init(call as *mut c_void) };
        if init != 0 {
            return Err(init);
        }
        let event_type = event_type(call).ok_or(-8)?;
        initialized.push(RegisteredModuleTraceEvent {
            owner,
            call,
            event_type,
            file: Box::new(LinuxTraceEventFile::new(call)),
            enabled: false,
        });
    }
    registered_events.extend(initialized);
    registered_evals.extend(
        eval_maps
            .into_iter()
            .map(|map| ModuleTraceEvalMap { owner, map }),
    );
    Ok(())
}

/// `trace_module_remove_events()` / `trace_module_remove_evals()` at
/// `MODULE_STATE_GOING`.
pub fn module_going(owner: usize) {
    let mut events = MODULE_EVENTS.lock();
    for event in events
        .iter_mut()
        .filter(|event| event.owner == owner && event.enabled)
    {
        event
            .file
            .flags
            .fetch_and(!EVENT_FILE_FL_ENABLED, Ordering::AcqRel);
        let result = unsafe {
            linux_trace_event_reg(
                event.call as *mut c_void,
                1,
                (&mut *event.file) as *mut LinuxTraceEventFile as *mut c_void,
            )
        };
        debug_assert_eq!(result, 0);
        event.enabled = false;
    }
    events.retain(|event| event.owner != owner);
    MODULE_EVAL_MAPS.lock().retain(|map| map.owner != owner);
}

pub fn module_events(owner: usize) -> Vec<ModuleTraceEvent> {
    MODULE_EVENTS
        .lock()
        .iter()
        .filter(|event| event.owner == owner)
        .map(|event| ModuleTraceEvent {
            owner: event.owner,
            call: event.call,
            event_type: event.event_type,
            enabled: event.enabled,
        })
        .collect()
}

/// Enable or disable one generated module trace event using its original
/// `struct trace_event_call`.  This runs the class registration callback,
/// publishes a Linux-layout tracepoint probe array, updates its static call,
/// and toggles the associated jump label.
pub fn set_module_event_enabled(call: usize, enable: bool) -> Result<(), i32> {
    let mut events = MODULE_EVENTS.lock();
    let event = events
        .iter_mut()
        .find(|event| event.call == call)
        .ok_or(-2)?;
    if event.enabled == enable {
        return Ok(());
    }
    let file = (&mut *event.file) as *mut LinuxTraceEventFile as *mut c_void;
    if enable {
        let result = unsafe { linux_trace_event_reg(call as *mut c_void, 0, file) };
        if result != 0 {
            return Err(result);
        }
        event.file.flags.fetch_or(
            EVENT_FILE_FL_ENABLED | EVENT_FILE_FL_WAS_ENABLED,
            Ordering::Release,
        );
        event.enabled = true;
    } else {
        event
            .file
            .flags
            .fetch_and(!EVENT_FILE_FL_ENABLED, Ordering::AcqRel);
        let result = unsafe { linux_trace_event_reg(call as *mut c_void, 1, file) };
        if result != 0 {
            event
                .file
                .flags
                .fetch_or(EVENT_FILE_FL_ENABLED, Ordering::Release);
            return Err(result);
        }
        event.enabled = false;
    }
    Ok(())
}

pub fn module_eval_maps(owner: usize) -> Vec<ModuleTraceEvalMap> {
    MODULE_EVAL_MAPS
        .lock()
        .iter()
        .filter(|map| map.owner == owner)
        .copied()
        .collect()
}

pub fn register(subsystem: &str, name: &str) {
    let mut g = CLASSES.lock();
    if !g.iter().any(|c| c.subsystem == subsystem && c.name == name) {
        g.push(TraceEventClass {
            subsystem: subsystem.into(),
            name: name.into(),
            enabled: false,
        });
    }
}

pub fn enable(subsystem: &str, name: &str) -> Result<(), i32> {
    let mut g = CLASSES.lock();
    g.iter_mut()
        .find(|c| c.subsystem == subsystem && c.name == name)
        .map(|c| c.enabled = true)
        .ok_or(-2)
}

pub fn count() -> usize {
    CLASSES.lock().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_abi_layout_matches_vendor_x86_64() {
        assert_eq!(core::mem::size_of::<LinuxTraceEventFile>(), 96);
        assert_eq!(core::mem::offset_of!(LinuxTraceEventFile, event_call), 16);
        assert_eq!(core::mem::offset_of!(LinuxTraceEventFile, flags), 72);
        assert_eq!(core::mem::size_of::<LinuxTraceEventBuffer>(), 48);
        assert_eq!(core::mem::offset_of!(LinuxTraceEventBuffer, entry), 24);
        assert_eq!(core::mem::offset_of!(LinuxTraceEventBuffer, regs), 40);
    }

    #[test]
    fn generated_reserve_and_commit_preserve_vendor_payload() {
        let mut call = [0usize; TRACE_EVENT_CALL_SIZE / core::mem::size_of::<usize>()];
        let call_address = call.as_mut_ptr() as usize;
        unsafe {
            ((call_address + TRACE_EVENT_CALL_EVENT_TYPE_OFFSET) as *mut i32).write(0x7ffe);
        }
        let mut file = LinuxTraceEventFile::new(call_address);
        file.flags.store(EVENT_FILE_FL_ENABLED, Ordering::Release);
        let mut buffer = LinuxTraceEventBuffer {
            buffer: 0,
            event: 0,
            trace_file: 0,
            entry: 0,
            trace_ctx: 0,
            _pad: 0,
            regs: 0,
        };
        let before = generated_event_count(0x7ffe);
        let entry = unsafe {
            linux_trace_event_buffer_reserve(
                (&mut buffer as *mut LinuxTraceEventBuffer).cast(),
                (&mut file as *mut LinuxTraceEventFile).cast(),
                20,
            )
        };
        assert!(!entry.is_null());
        unsafe {
            entry
                .cast::<u8>()
                .add(8)
                .copy_from_nonoverlapping(0x1234_5678u32.to_le_bytes().as_ptr(), 4);
            linux_trace_event_buffer_commit((&mut buffer as *mut LinuxTraceEventBuffer).cast());
        }
        assert_eq!(generated_event_count(0x7ffe), before + 1);
        let mut payload = [0u8; 20];
        assert_eq!(latest_generated_payload(0x7ffe, &mut payload), Some(20));
        assert_eq!(
            u32::from_le_bytes(payload[8..12].try_into().unwrap()),
            0x1234_5678
        );
    }

    #[test]
    fn register_then_enable() {
        register("sched", "sched_switch");
        enable("sched", "sched_switch").unwrap();
        let g = CLASSES.lock();
        let c = g.iter().find(|c| c.name == "sched_switch").unwrap();
        assert!(c.enabled);
    }

    #[test]
    fn enable_missing_is_enoent() {
        assert_eq!(enable("none", "none").unwrap_err(), -2);
    }
}
