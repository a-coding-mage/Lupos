//! linux-parity: partial
//! linux-source: vendor/linux/kernel/printk
//! test-origin: linux:vendor/linux/kernel/printk
//! Structured kernel logging with static storage only.
//!
//! Features:
//! - timestamps sourced from `jiffies`
//! - 256-entry ring buffer
//! - global and per-subsystem level filters
//! - per-call-site rate limiting
//! - log and subsystem dump helpers
//!
//! Everything in this module is `no_std` and heap-free.

use core::fmt;
use core::sync::atomic::{AtomicU8, AtomicU32, AtomicU64, Ordering};

use spin::Mutex;

pub const RING_SIZE: usize = 256;
pub const MODULE_CAP: usize = 24;
// Linux's console formatting buffer is PRINTK_MESSAGE_MAX (2048), so a
// single normal record can carry long boot lines such as the full cmdline.
// Ref: vendor/linux/kernel/printk/internal.h.
pub const MSG_CAP: usize = 2048;
const MAX_SUBSYSTEM_FILTERS: usize = 32;
const MAX_DUMP_HANDLERS: usize = 32;
const ANSI_RESET: &str = "\x1b[0m";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Level {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

impl Level {
    pub const fn label(self) -> &'static str {
        match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN ",
            Level::Info => "INFO ",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        }
    }

    pub const fn ansi_color(self) -> &'static str {
        match self {
            Level::Error => "\x1b[31m",
            Level::Warn => "\x1b[33m",
            Level::Info => "\x1b[32m",
            Level::Debug => "\x1b[36m",
            Level::Trace => "\x1b[90m",
        }
    }

    const fn from_u8(value: u8) -> Self {
        match value {
            0 => Level::Error,
            1 => Level::Warn,
            2 => Level::Info,
            3 => Level::Debug,
            _ => Level::Trace,
        }
    }
}

static LOG_LEVEL: AtomicU8 = AtomicU8::new(Level::Info as u8);

pub fn set_level(level: Level) {
    LOG_LEVEL.store(level as u8, Ordering::Relaxed);
}

pub fn level() -> Level {
    Level::from_u8(LOG_LEVEL.load(Ordering::Relaxed))
}

pub fn is_enabled(msg_level: Level) -> bool {
    (msg_level as u8) <= LOG_LEVEL.load(Ordering::Relaxed)
}

#[derive(Clone, Copy)]
struct SubsystemFilter {
    prefix: &'static str,
    level: Level,
}

struct FilterTable {
    entries: [Option<SubsystemFilter>; MAX_SUBSYSTEM_FILTERS],
    count: usize,
}

impl FilterTable {
    const fn empty() -> Self {
        Self {
            entries: [None; MAX_SUBSYSTEM_FILTERS],
            count: 0,
        }
    }

    fn insert(&mut self, prefix: &'static str, level: Level) -> bool {
        for index in 0..self.count {
            if let Some(entry) = self.entries[index].as_mut() {
                if entry.prefix == prefix {
                    entry.level = level;
                    return true;
                }
            }
        }

        if self.count == MAX_SUBSYSTEM_FILTERS {
            return false;
        }

        self.entries[self.count] = Some(SubsystemFilter { prefix, level });
        self.count += 1;
        true
    }

    fn lookup(&self, module: &str) -> Option<Level> {
        let mut best: Option<(usize, Level)> = None;
        for index in 0..self.count {
            if let Some(entry) = self.entries[index] {
                if module.starts_with(entry.prefix) {
                    let len = entry.prefix.len();
                    match best {
                        Some((best_len, _)) if best_len >= len => {}
                        _ => best = Some((len, entry.level)),
                    }
                }
            }
        }
        best.map(|(_, level)| level)
    }
}

static FILTER_TABLE: Mutex<FilterTable> = Mutex::new(FilterTable::empty());

pub fn register_subsystem_filter(prefix: &'static str, level: Level) -> bool {
    FILTER_TABLE.lock().insert(prefix, level)
}

pub fn effective_level(module: &str) -> Level {
    FILTER_TABLE.lock().lookup(module).unwrap_or_else(level)
}

pub fn is_enabled_for(msg_level: Level, module: &str) -> bool {
    (msg_level as u8) <= (effective_level(module) as u8)
}

#[derive(Clone, Copy)]
pub struct LogRecord {
    pub seq: u64,
    pub jiffies: u64,
    pub level: Level,
    pub module: [u8; MODULE_CAP],
    pub msg: [u8; MSG_CAP],
    pub mod_len: u8,
    pub msg_len: u16,
}

impl LogRecord {
    const fn empty() -> Self {
        Self {
            seq: 0,
            jiffies: 0,
            level: Level::Info,
            module: [0; MODULE_CAP],
            msg: [0; MSG_CAP],
            mod_len: 0,
            msg_len: 0,
        }
    }

    pub fn module_str(&self) -> &str {
        let len = (self.mod_len as usize).min(MODULE_CAP);
        core::str::from_utf8(&self.module[..len]).unwrap_or("?")
    }

    pub fn msg_str(&self) -> &str {
        let len = (self.msg_len as usize).min(MSG_CAP);
        core::str::from_utf8(&self.msg[..len]).unwrap_or("?")
    }
}

struct RingBuffer {
    slots: [LogRecord; RING_SIZE],
    head: usize,
    seq: u64,
    len: usize,
}

impl RingBuffer {
    const fn new() -> Self {
        Self {
            slots: [LogRecord::empty(); RING_SIZE],
            head: 0,
            seq: 0,
            len: 0,
        }
    }

    fn push(&mut self, level: Level, module: &str, message: &[u8], jiffies: u64) {
        let mut record = LogRecord::empty();
        record.seq = self.seq;
        record.jiffies = jiffies;
        record.level = level;

        let mod_len = module.len().min(MODULE_CAP);
        record.module[..mod_len].copy_from_slice(&module.as_bytes()[..mod_len]);
        record.mod_len = mod_len as u8;

        let msg_len = message.len().min(MSG_CAP);
        record.msg[..msg_len].copy_from_slice(&message[..msg_len]);
        record.msg_len = msg_len as u16;

        self.slots[self.head] = record;
        self.head = (self.head + 1) & (RING_SIZE - 1);
        self.seq = self.seq.wrapping_add(1);
        if self.len < RING_SIZE {
            self.len += 1;
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn total_count(&self) -> u64 {
        self.seq
    }

    fn oldest_index(&self) -> usize {
        if self.len == RING_SIZE { self.head } else { 0 }
    }

    fn get_ordered(&self, index: usize) -> Option<&LogRecord> {
        if index >= self.len {
            return None;
        }
        let slot = (self.oldest_index() + index) & (RING_SIZE - 1);
        Some(&self.slots[slot])
    }
}

static RING: Mutex<RingBuffer> = Mutex::new(RingBuffer::new());

struct WriteBuf<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> WriteBuf<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn len(&self) -> usize {
        self.pos
    }
}

impl fmt::Write for WriteBuf<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.pos >= self.buf.len() {
            return Ok(());
        }

        let available = self.buf.len() - self.pos;
        let bytes = s.as_bytes();
        let copy_len = bytes.len().min(available);
        self.buf[self.pos..self.pos + copy_len].copy_from_slice(&bytes[..copy_len]);
        self.pos += copy_len;
        Ok(())
    }
}

#[inline]
fn now_jiffies() -> u64 {
    crate::kernel::time::jiffies::jiffies()
}

#[inline]
fn now_msecs() -> u64 {
    crate::kernel::time::jiffies::jiffies_to_msecs(now_jiffies())
}

/// Microseconds since boot.
///
/// Uses the calibrated TSC frequency from
/// [`crate::arch::x86::kernel::tsc::calibrate`] when available, else falls
/// back to a jiffies-derived count so timestamps never sit at 0.
///
/// The `jiffies` field on [`LogRecord`] stores the result and is named for
/// historical reasons — it is the µs value the emitter formats.
#[inline]
fn now_tsc_usecs() -> u64 {
    let tsc = crate::kernel::time::clocksource::read_tsc();
    if tsc == 0 {
        return now_msecs().saturating_mul(1000);
    }
    let usec = crate::arch::x86::kernel::tsc::cycles_to_usec(tsc);
    if usec != 0 {
        return usec;
    }
    // TSC is ticking but uncalibrated — treat raw cycles as ns under a
    // 1 GHz nominal assumption so printk timestamps still monotonically
    // advance.  Switches to true µs as soon as `tsc::calibrate()` runs.
    tsc / 1000
}

pub fn timestamp_msecs() -> u64 {
    now_tsc_usecs() / 1000
}

fn emit_record(record: &LogRecord) {
    // Linux print_time() — vendor/linux/kernel/printk/printk.c:1355.
    //   printf("[%5lu.%06lu] ", seconds, microseconds)
    let us = record.jiffies;
    let secs = us / 1_000_000;
    let micros = us % 1_000_000;
    let module = record.module_str();
    let msg = record.msg_str();

    if module.is_empty() {
        crate::linux_driver_abi::tty::serial::_print(format_args!(
            "[{:>5}.{:06}] {}\n",
            secs, micros, msg,
        ));
        #[cfg(not(test))]
        crate::linux_driver_abi::video::console::vgacon::_print(format_args!(
            "[{:>5}.{:06}] {}\n",
            secs, micros, msg,
        ));
    } else {
        crate::linux_driver_abi::tty::serial::_print(format_args!(
            "[{:>5}.{:06}] {}: {}\n",
            secs, micros, module, msg,
        ));
        #[cfg(not(test))]
        crate::linux_driver_abi::video::console::vgacon::_print(format_args!(
            "[{:>5}.{:06}] {}: {}\n",
            secs, micros, module, msg,
        ));
    }
}

#[doc(hidden)]
pub fn _log(msg_level: Level, module: &str, args: fmt::Arguments<'_>) {
    if !is_enabled_for(msg_level, module) {
        return;
    }

    let mut message_buf = [0u8; MSG_CAP];
    let mut writer = WriteBuf::new(&mut message_buf);
    let _ = fmt::write(&mut writer, args);
    let message_len = writer.len();
    let message = &message_buf[..message_len];

    let ts_us = now_tsc_usecs();

    if let Some(mut ring) = RING.try_lock() {
        ring.push(msg_level, module, message, ts_us);
    }

    let mut record = LogRecord::empty();
    record.level = msg_level;
    record.jiffies = ts_us;

    let mod_len = module.len().min(MODULE_CAP);
    record.module[..mod_len].copy_from_slice(&module.as_bytes()[..mod_len]);
    record.mod_len = mod_len as u8;

    let msg_len = message.len().min(MSG_CAP);
    record.msg[..msg_len].copy_from_slice(&message[..msg_len]);
    record.msg_len = msg_len as u16;

    emit_record(&record);
}

fn dump_record(record: &LogRecord) {
    // record.jiffies is µs since boot, stored by `_log` via `now_tsc_usecs`.
    let us = record.jiffies;
    crate::linux_driver_abi::tty::serial::_print(format_args!(
        "  #{:<5} t={:>6}.{:06}s  {}[{}]{} {:>16}: {}\n",
        record.seq,
        us / 1_000_000,
        us % 1_000_000,
        record.level.ansi_color(),
        record.level.label(),
        ANSI_RESET,
        record.module_str(),
        record.msg_str(),
    ));
}

pub fn dump_log() {
    crate::linux_driver_abi::tty::serial::_print(format_args!(
        "\x1b[1m=== lupos log dump (last {} entries) ===\x1b[0m\n",
        RING_SIZE
    ));

    let ring = RING.lock();
    for index in 0..ring.len() {
        if let Some(record) = ring.get_ordered(index) {
            dump_record(record);
        }
    }

    crate::linux_driver_abi::tty::serial::_print(format_args!(
        "=== end ({} records) ===\n\n",
        ring.len()
    ));
}

pub fn dump_log_for_module(prefix: &str) {
    crate::linux_driver_abi::tty::serial::_print(format_args!(
        "\x1b[1m=== lupos log dump for '{}' ===\x1b[0m\n",
        prefix
    ));

    let ring = RING.lock();
    let mut dumped = 0usize;
    for index in 0..ring.len() {
        if let Some(record) = ring.get_ordered(index) {
            if record.module_str().starts_with(prefix) {
                dump_record(record);
                dumped += 1;
            }
        }
    }

    crate::linux_driver_abi::tty::serial::_print(format_args!(
        "=== end ({} records) ===\n\n",
        dumped
    ));
}

pub fn log_count() -> u64 {
    RING.lock().total_count()
}

pub fn ring_fill() -> usize {
    RING.lock().len()
}

pub struct RateLimiter {
    last_jiffies: AtomicU64,
    count: AtomicU32,
    pub interval: u64,
    pub burst: u32,
}

impl RateLimiter {
    pub const fn new(interval_jiffies: u64, burst: u32) -> Self {
        Self {
            last_jiffies: AtomicU64::new(0),
            count: AtomicU32::new(0),
            interval: interval_jiffies,
            burst,
        }
    }

    pub fn allow(&self) -> bool {
        let now = now_jiffies();
        let last = self.last_jiffies.load(Ordering::Acquire);

        if now.wrapping_sub(last) >= self.interval {
            self.last_jiffies.store(now, Ordering::Release);
            self.count.store(1, Ordering::Release);
            return true;
        }

        let previous = self.count.fetch_add(1, Ordering::AcqRel);
        previous < self.burst
    }
}

type DumpFn = fn();

#[derive(Clone, Copy)]
struct DumpEntry {
    name: &'static str,
    dump: DumpFn,
}

struct DumpTable {
    entries: [Option<DumpEntry>; MAX_DUMP_HANDLERS],
    count: usize,
}

impl DumpTable {
    const fn empty() -> Self {
        Self {
            entries: [None; MAX_DUMP_HANDLERS],
            count: 0,
        }
    }

    fn register(&mut self, name: &'static str, dump: DumpFn) -> bool {
        for index in 0..self.count {
            if let Some(entry) = self.entries[index] {
                if entry.name == name {
                    self.entries[index] = Some(DumpEntry { name, dump });
                    return true;
                }
            }
        }

        if self.count == MAX_DUMP_HANDLERS {
            return false;
        }

        self.entries[self.count] = Some(DumpEntry { name, dump });
        self.count += 1;
        true
    }

    fn find(&self, name: &str) -> Option<DumpFn> {
        for index in 0..self.count {
            if let Some(entry) = self.entries[index] {
                if entry.name == name {
                    return Some(entry.dump);
                }
            }
        }
        None
    }

    fn get(&self, index: usize) -> Option<DumpFn> {
        if index >= self.count {
            return None;
        }
        self.entries[index].map(|entry| entry.dump)
    }

    fn len(&self) -> usize {
        self.count
    }
}

static DUMP_TABLE: Mutex<DumpTable> = Mutex::new(DumpTable::empty());

pub fn register_dump(name: &'static str, dump: DumpFn) -> bool {
    DUMP_TABLE.lock().register(name, dump)
}

pub fn dump_subsystem(name: &str) {
    let dump = DUMP_TABLE.lock().find(name);
    match dump {
        Some(handler) => handler(),
        None => {
            crate::kernel::printk::log_warn!("log", "dump_subsystem: no handler for '{}'", name)
        }
    }
}

pub fn dump_all_subsystems() {
    let count = DUMP_TABLE.lock().len();
    for index in 0..count {
        let dump = DUMP_TABLE.lock().get(index);
        if let Some(handler) = dump {
            handler();
        }
    }
}

pub fn dump_on_panic() {
    crate::linux_driver_abi::tty::serial::_print(format_args!(
        "\x1b[31m[PANIC] log replay:\x1b[0m\n"
    ));
    dump_log();
    dump_all_subsystems();
}

pub fn uptime_msecs() -> u64 {
    now_msecs()
}

#[macro_export]
macro_rules! log_error {
    ($module:expr, $($arg:tt)*) => {
        $crate::kernel::printk::log::_log($crate::kernel::printk::log::Level::Error, $module, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($module:expr, $($arg:tt)*) => {
        $crate::kernel::printk::log::_log($crate::kernel::printk::log::Level::Warn, $module, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_info {
    ($module:expr, $($arg:tt)*) => {
        $crate::kernel::printk::log::_log($crate::kernel::printk::log::Level::Info, $module, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($module:expr, $($arg:tt)*) => {
        $crate::kernel::printk::log::_log($crate::kernel::printk::log::Level::Debug, $module, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_trace {
    ($module:expr, $($arg:tt)*) => {
        $crate::kernel::printk::log::_log($crate::kernel::printk::log::Level::Trace, $module, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_ratelimited {
    ($level:expr, $module:expr, $interval_jiffies:expr, $burst:expr, $($arg:tt)*) => {{
        static RL: $crate::kernel::printk::log::RateLimiter =
            $crate::kernel::printk::log::RateLimiter::new($interval_jiffies, $burst);
        if RL.allow() {
            $crate::kernel::printk::log::_log($level, $module, format_args!($($arg)*));
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_ordering() {
        assert!(Level::Error < Level::Warn);
        assert!(Level::Warn < Level::Info);
        assert!(Level::Info < Level::Debug);
        assert!(Level::Debug < Level::Trace);
    }

    #[test]
    fn labels_are_fixed_width() {
        for level in [
            Level::Error,
            Level::Warn,
            Level::Info,
            Level::Debug,
            Level::Trace,
        ] {
            assert_eq!(level.label().len(), 5);
        }
    }

    #[test]
    fn set_and_get_level() {
        let original = LOG_LEVEL.load(Ordering::Relaxed);
        set_level(Level::Debug);
        assert_eq!(level(), Level::Debug);
        set_level(Level::Error);
        assert_eq!(level(), Level::Error);
        LOG_LEVEL.store(original, Ordering::Relaxed);
    }

    #[test]
    fn subsystem_filter_overrides_global_level() {
        let original = LOG_LEVEL.load(Ordering::Relaxed);
        set_level(Level::Error);
        assert!(register_subsystem_filter("test-subsystem", Level::Debug));
        assert!(is_enabled_for(Level::Debug, "test-subsystem/io"));
        assert!(!is_enabled_for(Level::Debug, "other"));
        LOG_LEVEL.store(original, Ordering::Relaxed);
    }

    #[test]
    fn longest_prefix_match_wins() {
        assert!(register_subsystem_filter("mm", Level::Warn));
        assert!(register_subsystem_filter("mm/fault", Level::Trace));
        assert_eq!(effective_level("mm/fault/anon"), Level::Trace);
        assert_eq!(effective_level("mm/slab"), Level::Warn);
    }

    #[test]
    fn write_buf_truncates_without_panicking() {
        let mut buf = [0u8; 8];
        let mut writer = WriteBuf::new(&mut buf);
        use core::fmt::Write;
        let _ = writer.write_str("hello world");
        assert_eq!(writer.len(), 8);
        assert_eq!(&buf, b"hello wo");
    }

    #[test]
    fn message_cap_matches_linux_printk_message_max() {
        assert_eq!(MSG_CAP, 2048);
        assert!(MSG_CAP > u8::MAX as usize);
        assert!(MSG_CAP <= u16::MAX as usize);
    }

    #[test]
    fn ring_buffer_wraps_and_preserves_order() {
        crate::kernel::time::jiffies::_reset_for_tests();
        let mut ring = RingBuffer::new();
        for i in 0..(RING_SIZE + 10) {
            let byte = b'0' + (i % 10) as u8;
            ring.push(Level::Info, "t", &[byte], i as u64);
        }

        assert_eq!(ring.len(), RING_SIZE);
        assert_eq!(ring.total_count(), (RING_SIZE + 10) as u64);
        assert_eq!(ring.get_ordered(0).unwrap().seq, 10);
        assert_eq!(
            ring.get_ordered(RING_SIZE - 1).unwrap().seq,
            (RING_SIZE + 9) as u64
        );
    }

    #[test]
    fn log_record_round_trip() {
        let mut ring = RingBuffer::new();
        ring.push(Level::Warn, "mm/fault", b"page fault", 12);
        let record = ring.get_ordered(0).unwrap();
        assert_eq!(record.level, Level::Warn);
        assert_eq!(record.module_str(), "mm/fault");
        assert_eq!(record.msg_str(), "page fault");
        assert_eq!(record.jiffies, 12);
    }

    #[test]
    fn long_log_record_round_trip_does_not_wrap_length() {
        let mut ring = RingBuffer::new();
        let message = [b'x'; 300];
        ring.push(Level::Info, "long", &message, 34);
        let record = ring.get_ordered(0).unwrap();
        assert_eq!(record.msg_len, 300);
        assert_eq!(record.msg_str().as_bytes(), &message);
    }

    #[test]
    fn rate_limiter_allows_burst_then_suppresses() {
        crate::kernel::time::jiffies::_reset_for_tests();
        let limiter = RateLimiter::new(250, 3);
        assert!(limiter.allow());
        assert!(limiter.allow());
        assert!(limiter.allow());
        assert!(!limiter.allow());

        for _ in 0..250 {
            crate::kernel::time::jiffies::tick_jiffies();
        }
        assert!(limiter.allow());
    }

    #[test]
    fn dump_table_registers_and_updates_handlers() {
        static CALLED_A: AtomicU32 = AtomicU32::new(0);
        static CALLED_B: AtomicU32 = AtomicU32::new(0);

        fn handler_a() {
            CALLED_A.fetch_add(1, Ordering::Relaxed);
        }

        fn handler_b() {
            CALLED_B.fetch_add(1, Ordering::Relaxed);
        }

        assert!(register_dump("test-dump", handler_a));
        dump_subsystem("test-dump");
        assert_eq!(CALLED_A.load(Ordering::Relaxed), 1);

        assert!(register_dump("test-dump", handler_b));
        dump_subsystem("test-dump");
        assert_eq!(CALLED_B.load(Ordering::Relaxed), 1);
    }
}
