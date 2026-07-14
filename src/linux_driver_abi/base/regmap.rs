//! linux-parity: partial
//! linux-source: vendor/linux/drivers/base/regmap
//! Minimal `regmap` core for vendor HDA modules.

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};

use spin::Mutex;

use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::{export_symbol, find_symbol};

const MAX_ERRNO: usize = 4095;
const REGCACHE_NONE: i32 = 0;
const REGMAP_MAGIC: u64 = 0x7265_676d_6170_0001;

type RegRead = unsafe extern "C" fn(*mut c_void, u32, *mut u32) -> i32;
type RegWrite = unsafe extern "C" fn(*mut c_void, u32, u32) -> i32;
type RegUpdateBits = unsafe extern "C" fn(*mut c_void, u32, u32, u32) -> i32;
type RegBool = unsafe extern "C" fn(*mut c_void, u32) -> u8;
type RegLock = unsafe extern "C" fn(*mut c_void);

#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxRegDefault {
    reg: u32,
    def: u32,
}

#[repr(C)]
struct LinuxRegmapConfig {
    name: *const c_char,
    reg_bits: i32,
    reg_stride: i32,
    reg_shift: i32,
    reg_base: u32,
    pad_bits: i32,
    val_bits: i32,
    writeable_reg: Option<RegBool>,
    readable_reg: Option<RegBool>,
    volatile_reg: Option<RegBool>,
    precious_reg: Option<RegBool>,
    writeable_noinc_reg: Option<RegBool>,
    readable_noinc_reg: Option<RegBool>,
    reg_read: Option<RegRead>,
    reg_write: Option<RegWrite>,
    reg_update_bits: Option<RegUpdateBits>,
    read: *const c_void,
    write: *const c_void,
    max_raw_read: usize,
    max_raw_write: usize,
    can_sleep: u8,
    fast_io: u8,
    io_port: u8,
    disable_locking: u8,
    lock: Option<RegLock>,
    unlock: Option<RegLock>,
    lock_arg: *mut c_void,
    max_register: u32,
    max_register_is_0: u8,
    wr_table: *const c_void,
    rd_table: *const c_void,
    volatile_table: *const c_void,
    precious_table: *const c_void,
    wr_noinc_table: *const c_void,
    rd_noinc_table: *const c_void,
    reg_defaults: *const LinuxRegDefault,
    num_reg_defaults: u32,
    reg_default_cb: *const c_void,
    cache_type: i32,
}

#[repr(C)]
struct LinuxRegmapBus {
    fast_io: u8,
    free_on_exit: u8,
    write: *const c_void,
    gather_write: *const c_void,
    async_write: *const c_void,
    reg_write: Option<RegWrite>,
    reg_noinc_write: *const c_void,
    reg_update_bits: Option<RegUpdateBits>,
    read: *const c_void,
    reg_read: Option<RegRead>,
    reg_noinc_read: *const c_void,
    free_context: *const c_void,
    async_alloc: *const c_void,
    read_flag_mask: u8,
    reg_format_endian_default: i32,
    val_format_endian_default: i32,
    max_raw_read: usize,
    max_raw_write: usize,
}

#[derive(Clone, Copy)]
struct RegCacheEntry {
    reg: u32,
    val: u32,
}

struct LinuxRegmap {
    magic: u64,
    dev: *mut c_void,
    context: *mut c_void,
    reg_read: Option<RegRead>,
    reg_write: Option<RegWrite>,
    reg_update_bits: Option<RegUpdateBits>,
    cache_type: i32,
    cache_dirty: Mutex<bool>,
    cache_only: Mutex<bool>,
    cache_bypass: Mutex<bool>,
    cache: Mutex<Vec<RegCacheEntry>>,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("__regmap_init", linux___regmap_init as usize, true);
    export_symbol_once("regmap_exit", linux_regmap_exit as usize, true);
    export_symbol_once("regmap_read", linux_regmap_read as usize, true);
    export_symbol_once("regmap_write", linux_regmap_write as usize, true);
    export_symbol_once(
        "regmap_update_bits_base",
        linux_regmap_update_bits_base as usize,
        true,
    );
    export_symbol_once("regcache_sync", linux_regcache_sync as usize, true);
    export_symbol_once(
        "regcache_reg_cached",
        linux_regcache_reg_cached as usize,
        true,
    );
    export_symbol_once(
        "regcache_mark_dirty",
        linux_regcache_mark_dirty as usize,
        true,
    );
    export_symbol_once(
        "regcache_cache_only",
        linux_regcache_cache_only as usize,
        true,
    );
    export_symbol_once(
        "regcache_cache_bypass",
        linux_regcache_cache_bypass as usize,
        true,
    );
}

fn err_ptr<T>(errno: i32) -> *mut T {
    (-(errno as isize)) as *mut T
}

fn is_err_or_null<T>(ptr: *const T) -> bool {
    ptr.is_null() || (ptr as usize) >= usize::MAX - MAX_ERRNO
}

unsafe fn regmap_ref<'a>(map: *mut LinuxRegmap) -> Result<&'a LinuxRegmap, i32> {
    if is_err_or_null(map) {
        return Err(EINVAL);
    }
    let map = unsafe { &*map };
    if map.magic != REGMAP_MAGIC {
        return Err(EINVAL);
    }
    Ok(map)
}

fn cache_get(map: &LinuxRegmap, reg: u32) -> Option<u32> {
    map.cache
        .lock()
        .iter()
        .find(|entry| entry.reg == reg)
        .map(|entry| entry.val)
}

fn cache_put(map: &LinuxRegmap, reg: u32, val: u32) {
    let mut cache = map.cache.lock();
    if let Some(entry) = cache.iter_mut().find(|entry| entry.reg == reg) {
        entry.val = val;
    } else {
        cache.push(RegCacheEntry { reg, val });
    }
}

fn cache_snapshot(map: &LinuxRegmap) -> Vec<RegCacheEntry> {
    map.cache.lock().clone()
}

/// `__regmap_init` - `vendor/linux/drivers/base/regmap/regmap.c`.
#[unsafe(export_name = "__regmap_init")]
unsafe extern "C" fn linux___regmap_init(
    dev: *mut c_void,
    bus: *const LinuxRegmapBus,
    bus_context: *mut c_void,
    config: *const LinuxRegmapConfig,
    _lock_key: *mut c_void,
    _lock_name: *const c_char,
) -> *mut LinuxRegmap {
    if config.is_null() {
        return err_ptr(EINVAL);
    }
    let config = unsafe { &*config };
    let (bus_reg_read, bus_reg_write, bus_reg_update_bits) = if bus.is_null() {
        (None, None, None)
    } else {
        let bus = unsafe { &*bus };
        (bus.reg_read, bus.reg_write, bus.reg_update_bits)
    };
    let reg_read = config.reg_read.or(bus_reg_read);
    let reg_write = config.reg_write.or(bus_reg_write);
    let reg_update_bits = config.reg_update_bits.or(bus_reg_update_bits);
    if reg_read.is_none() && reg_write.is_none() && reg_update_bits.is_none() {
        return err_ptr(EINVAL);
    }

    let map = Box::new(LinuxRegmap {
        magic: REGMAP_MAGIC,
        dev,
        context: bus_context,
        reg_read,
        reg_write,
        reg_update_bits,
        cache_type: config.cache_type,
        cache_dirty: Mutex::new(false),
        cache_only: Mutex::new(false),
        cache_bypass: Mutex::new(false),
        cache: Mutex::new(Vec::new()),
    });
    if config.cache_type != REGCACHE_NONE
        && !config.reg_defaults.is_null()
        && config.num_reg_defaults > 0
    {
        for index in 0..config.num_reg_defaults as usize {
            let default = unsafe { *config.reg_defaults.add(index) };
            cache_put(&map, default.reg, default.def);
        }
    }
    Box::into_raw(map)
}

/// `regmap_exit` - `vendor/linux/drivers/base/regmap/regmap.c`.
#[unsafe(export_name = "regmap_exit")]
unsafe extern "C" fn linux_regmap_exit(map: *mut LinuxRegmap) {
    if is_err_or_null(map) {
        return;
    }
    unsafe {
        drop(Box::from_raw(map));
    }
}

/// `regmap_read` - `vendor/linux/drivers/base/regmap/regmap.c`.
#[unsafe(export_name = "regmap_read")]
unsafe extern "C" fn linux_regmap_read(map: *mut LinuxRegmap, reg: u32, val: *mut u32) -> i32 {
    if val.is_null() {
        return -EINVAL;
    }
    let map = match unsafe { regmap_ref(map) } {
        Ok(map) => map,
        Err(errno) => return -errno,
    };
    if map.cache_type != REGCACHE_NONE && !*map.cache_bypass.lock() {
        if let Some(cached) = cache_get(map, reg) {
            unsafe {
                *val = cached;
            }
            return 0;
        }
    }
    let Some(read) = map.reg_read else {
        return -EINVAL;
    };
    let mut read_val = 0u32;
    let ret = unsafe { read(map.context, reg, &mut read_val) };
    if ret == 0 {
        unsafe {
            *val = read_val;
        }
        if map.cache_type != REGCACHE_NONE && !*map.cache_bypass.lock() {
            cache_put(map, reg, read_val);
        }
    }
    ret
}

/// `regmap_write` - `vendor/linux/drivers/base/regmap/regmap.c`.
#[unsafe(export_name = "regmap_write")]
unsafe extern "C" fn linux_regmap_write(map: *mut LinuxRegmap, reg: u32, val: u32) -> i32 {
    let map = match unsafe { regmap_ref(map) } {
        Ok(map) => map,
        Err(errno) => return -errno,
    };
    if map.cache_type != REGCACHE_NONE && *map.cache_only.lock() {
        cache_put(map, reg, val);
        *map.cache_dirty.lock() = true;
        return 0;
    }
    let ret = if *map.cache_bypass.lock() {
        if let Some(write) = map.reg_write {
            unsafe { write(map.context, reg, val) }
        } else {
            -EINVAL
        }
    } else if let Some(write) = map.reg_write {
        unsafe { write(map.context, reg, val) }
    } else {
        -EINVAL
    };
    if ret == 0 && map.cache_type != REGCACHE_NONE && !*map.cache_bypass.lock() {
        cache_put(map, reg, val);
    }
    ret
}

/// `regmap_update_bits_base` - `vendor/linux/drivers/base/regmap/regmap.c`.
#[unsafe(export_name = "regmap_update_bits_base")]
unsafe extern "C" fn linux_regmap_update_bits_base(
    map: *mut LinuxRegmap,
    reg: u32,
    mask: u32,
    val: u32,
    change: *mut u8,
    _async: u8,
    force: u8,
) -> i32 {
    let map_ref = match unsafe { regmap_ref(map) } {
        Ok(map) => map,
        Err(errno) => return -errno,
    };
    if *map_ref.cache_bypass.lock() {
        if let Some(update_bits) = map_ref.reg_update_bits {
            let ret = unsafe { update_bits(map_ref.context, reg, mask, val) };
            if !change.is_null() {
                unsafe {
                    *change = (ret == 0) as u8;
                }
            }
            return ret;
        }
    }

    let mut orig = 0u32;
    let ret = unsafe { linux_regmap_read(map, reg, &mut orig) };
    if ret != 0 {
        return ret;
    }
    let new = (orig & !mask) | (val & mask);
    let changed = new != orig;
    if !change.is_null() {
        unsafe {
            *change = changed as u8;
        }
    }
    if changed || force != 0 {
        unsafe { linux_regmap_write(map, reg, new) }
    } else {
        0
    }
}

/// `regcache_sync` - `vendor/linux/drivers/base/regmap/regcache.c`.
#[unsafe(export_name = "regcache_sync")]
unsafe extern "C" fn linux_regcache_sync(map: *mut LinuxRegmap) -> i32 {
    let map = match unsafe { regmap_ref(map) } {
        Ok(map) => map,
        Err(errno) => return -errno,
    };
    if map.cache_type == REGCACHE_NONE {
        return -EINVAL;
    }
    if !*map.cache_dirty.lock() {
        return 0;
    }
    let Some(write) = map.reg_write else {
        return -EINVAL;
    };
    let previous_bypass = *map.cache_bypass.lock();
    *map.cache_bypass.lock() = true;
    let entries = cache_snapshot(map);
    for entry in entries {
        let ret = unsafe { write(map.context, entry.reg, entry.val) };
        if ret != 0 {
            *map.cache_bypass.lock() = previous_bypass;
            return ret;
        }
    }
    *map.cache_bypass.lock() = previous_bypass;
    *map.cache_dirty.lock() = false;
    0
}

/// `regcache_reg_cached` - `vendor/linux/drivers/base/regmap/regcache.c`.
#[unsafe(export_name = "regcache_reg_cached")]
unsafe extern "C" fn linux_regcache_reg_cached(map: *mut LinuxRegmap, reg: u32) -> u8 {
    let Ok(map) = (unsafe { regmap_ref(map) }) else {
        return 0;
    };
    cache_get(map, reg).is_some() as u8
}

/// `regcache_mark_dirty` - `vendor/linux/drivers/base/regmap/regcache.c`.
#[unsafe(export_name = "regcache_mark_dirty")]
unsafe extern "C" fn linux_regcache_mark_dirty(map: *mut LinuxRegmap) {
    if let Ok(map) = unsafe { regmap_ref(map) } {
        *map.cache_dirty.lock() = true;
    }
}

/// `regcache_cache_only` - `vendor/linux/drivers/base/regmap/regcache.c`.
#[unsafe(export_name = "regcache_cache_only")]
unsafe extern "C" fn linux_regcache_cache_only(map: *mut LinuxRegmap, enable: u8) {
    if let Ok(map) = unsafe { regmap_ref(map) } {
        *map.cache_only.lock() = enable != 0;
    }
}

/// `regcache_cache_bypass` - `vendor/linux/drivers/base/regmap/regcache.c`.
#[unsafe(export_name = "regcache_cache_bypass")]
unsafe extern "C" fn linux_regcache_cache_bypass(map: *mut LinuxRegmap, enable: u8) {
    if let Ok(map) = unsafe { regmap_ref(map) } {
        *map.cache_bypass.lock() = enable != 0;
    }
}
