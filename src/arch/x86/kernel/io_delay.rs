//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/io_delay.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/io_delay.c
//! Configurable I/O port delay strategy for `inb_p`/`outb_p`.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/io_delay.c
//!
//! Linux supports four delay strategies, selectable at boot via the
//! `io_delay=` command line:
//! - `0x80`: write to port 0x80 (the canonical legacy delay; ~1 us)
//! - `0xed`: write to port 0xed (some HP/Compaq laptops lock up on 0x80)
//! - `udelay`: use `udelay(2)`
//! - `none`: no delay
//!
//! Lupos targets modern hardware (port 0x80 works); the module mostly
//! exists so the DMI quirk table and cmdline parser are faithful to
//! Linux.

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

use crate::include::uapi::errno::EINVAL;

/// `io_delay_type` enumeration — mirror the `IO_DELAY_TYPE_*` macros.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u32)]
pub enum IoDelayType {
    Port0x80 = 0,
    Port0xed = 1,
    Udelay = 2,
    None = 3,
}

impl IoDelayType {
    pub const fn as_u32(self) -> u32 {
        self as u32
    }

    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(Self::Port0x80),
            1 => Some(Self::Port0xed),
            2 => Some(Self::Udelay),
            3 => Some(Self::None),
            _ => None,
        }
    }
}

/// Live `io_delay_type`; defaults to `Port0x80` (matches Linux's
/// `CONFIG_IO_DELAY_0X80=y` default).
pub static IO_DELAY_TYPE: AtomicU32 = AtomicU32::new(IoDelayType::Port0x80 as u32);

/// Whether `io_delay_override` is set — toggled by the cmdline parser.
pub static IO_DELAY_OVERRIDE: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

/// Trait seam for `outb` and `udelay(2)`.
pub trait IoDelayBackend {
    fn outb(&self, port: u16, value: u8);
    fn udelay(&self, micros: u32);
}

/// Linux's `native_io_delay`: dispatch on the active strategy.
pub fn native_io_delay<B: IoDelayBackend>(backend: &B) {
    let ty = IoDelayType::from_u32(IO_DELAY_TYPE.load(Ordering::Relaxed))
        .unwrap_or(IoDelayType::Port0x80);
    match ty {
        IoDelayType::Port0x80 => backend.outb(0x80, 0),
        IoDelayType::Port0xed => backend.outb(0xed, 0),
        IoDelayType::Udelay => backend.udelay(2),
        IoDelayType::None => {}
    }
}

/// Linux's `io_delay_param("0x80"|"0xed"|"udelay"|"none")`.
pub fn io_delay_param(s: &str) -> Result<(), i32> {
    let ty = match s {
        "0x80" => IoDelayType::Port0x80,
        "0xed" => IoDelayType::Port0xed,
        "udelay" => IoDelayType::Udelay,
        "none" => IoDelayType::None,
        _ => return Err(EINVAL),
    };
    IO_DELAY_TYPE.store(ty.as_u32(), Ordering::Relaxed);
    IO_DELAY_OVERRIDE.store(true, Ordering::Relaxed);
    Ok(())
}

/// DMI quirk record — list of (board-vendor, board-name) pairs that
/// require switching `Port0x80` → `Port0xed` to avoid lockups.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DmiQuirk {
    pub ident: &'static str,
    pub board_vendor: &'static str,
    pub board_name: &'static str,
}

pub const IO_DELAY_0XED_DMI_TABLE: &[DmiQuirk] = &[
    DmiQuirk {
        ident: "Compaq Presario V6000",
        board_vendor: "Quanta",
        board_name: "30B7",
    },
    DmiQuirk {
        ident: "HP Pavilion dv9000z",
        board_vendor: "Quanta",
        board_name: "30B9",
    },
    DmiQuirk {
        ident: "HP Pavilion dv6000",
        board_vendor: "Quanta",
        board_name: "30B8",
    },
    DmiQuirk {
        ident: "HP Pavilion tx1000",
        board_vendor: "Quanta",
        board_name: "30BF",
    },
    DmiQuirk {
        ident: "Presario F700",
        board_vendor: "Quanta",
        board_name: "30D3",
    },
];

/// Linux's `dmi_io_delay_0xed_port` callback: switch the strategy to
/// `Port0xed` *only* if the current value is `Port0x80`.
pub fn dmi_io_delay_0xed_port() {
    let cur = IoDelayType::from_u32(IO_DELAY_TYPE.load(Ordering::Relaxed))
        .unwrap_or(IoDelayType::Port0x80);
    if cur == IoDelayType::Port0x80 {
        IO_DELAY_TYPE.store(IoDelayType::Port0xed.as_u32(), Ordering::Relaxed);
    }
}

/// Linux's `io_delay_init`: if the override hasn't been set on the
/// cmdline, walk the DMI table and apply the matching quirk.
pub fn io_delay_init(matched_quirks: &[&DmiQuirk]) {
    if IO_DELAY_OVERRIDE.load(Ordering::Relaxed) {
        return;
    }
    if !matched_quirks.is_empty() {
        dmi_io_delay_0xed_port();
    }
}

/// Helper: find the first DMI quirk in `IO_DELAY_0XED_DMI_TABLE` whose
/// board-vendor and board-name match the live DMI strings.
pub fn match_dmi_quirk<'a>(
    table: &'a [DmiQuirk],
    board_vendor: &str,
    board_name: &str,
) -> Option<&'a DmiQuirk> {
    table
        .iter()
        .find(|q| q.board_vendor == board_vendor && q.board_name == board_name)
}

/// Accumulator helper for callers that want a `Vec` of matched quirks.
pub fn collect_dmi_matches<'a>(
    table: &'a [DmiQuirk],
    board_vendor: &str,
    board_name: &str,
) -> Vec<&'a DmiQuirk> {
    table
        .iter()
        .filter(|q| q.board_vendor == board_vendor && q.board_name == board_name)
        .collect()
}

/// Render the active strategy as the `io_delay=...` cmdline value.
pub fn current_strategy_name() -> String {
    match IoDelayType::from_u32(IO_DELAY_TYPE.load(Ordering::Relaxed))
        .unwrap_or(IoDelayType::Port0x80)
    {
        IoDelayType::Port0x80 => String::from("0x80"),
        IoDelayType::Port0xed => String::from("0xed"),
        IoDelayType::Udelay => String::from("udelay"),
        IoDelayType::None => String::from("none"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::RefCell;

    struct RecordingBackend {
        outb: RefCell<Vec<(u16, u8)>>,
        udelay: RefCell<Vec<u32>>,
    }

    impl RecordingBackend {
        fn new() -> Self {
            Self {
                outb: RefCell::new(Vec::new()),
                udelay: RefCell::new(Vec::new()),
            }
        }
    }

    impl IoDelayBackend for RecordingBackend {
        fn outb(&self, port: u16, value: u8) {
            self.outb.borrow_mut().push((port, value));
        }
        fn udelay(&self, micros: u32) {
            self.udelay.borrow_mut().push(micros);
        }
    }

    fn reset_globals() {
        IO_DELAY_TYPE.store(IoDelayType::Port0x80.as_u32(), Ordering::Relaxed);
        IO_DELAY_OVERRIDE.store(false, Ordering::Relaxed);
    }

    #[test]
    fn enum_round_trips_via_u32() {
        for &ty in &[
            IoDelayType::Port0x80,
            IoDelayType::Port0xed,
            IoDelayType::Udelay,
            IoDelayType::None,
        ] {
            assert_eq!(IoDelayType::from_u32(ty.as_u32()), Some(ty));
        }
    }

    #[test]
    fn param_parser_accepts_known_values() {
        reset_globals();
        assert!(io_delay_param("0x80").is_ok());
        assert_eq!(
            IO_DELAY_TYPE.load(Ordering::Relaxed),
            IoDelayType::Port0x80.as_u32()
        );
        assert!(io_delay_param("udelay").is_ok());
        assert_eq!(
            IO_DELAY_TYPE.load(Ordering::Relaxed),
            IoDelayType::Udelay.as_u32()
        );
        assert!(IO_DELAY_OVERRIDE.load(Ordering::Relaxed));
    }

    #[test]
    fn param_parser_rejects_unknown_values() {
        reset_globals();
        assert_eq!(io_delay_param("xyz"), Err(EINVAL));
    }

    #[test]
    fn native_io_delay_writes_port_0x80() {
        reset_globals();
        let backend = RecordingBackend::new();
        native_io_delay(&backend);
        assert_eq!(*backend.outb.borrow(), [(0x80, 0)]);
    }

    #[test]
    fn native_io_delay_writes_port_0xed_after_quirk() {
        reset_globals();
        IO_DELAY_TYPE.store(IoDelayType::Port0xed.as_u32(), Ordering::Relaxed);
        let backend = RecordingBackend::new();
        native_io_delay(&backend);
        assert_eq!(*backend.outb.borrow(), [(0xed, 0)]);
    }

    #[test]
    fn native_io_delay_uses_udelay_with_2_micros() {
        reset_globals();
        IO_DELAY_TYPE.store(IoDelayType::Udelay.as_u32(), Ordering::Relaxed);
        let backend = RecordingBackend::new();
        native_io_delay(&backend);
        assert!(backend.outb.borrow().is_empty());
        assert_eq!(*backend.udelay.borrow(), [2]);
    }

    #[test]
    fn dmi_quirk_only_overrides_default_strategy() {
        reset_globals();
        // Quirk applies → Port0xed.
        dmi_io_delay_0xed_port();
        assert_eq!(
            IO_DELAY_TYPE.load(Ordering::Relaxed),
            IoDelayType::Port0xed.as_u32()
        );

        // Once set to Udelay by cmdline, the DMI quirk must NOT clobber it.
        IO_DELAY_TYPE.store(IoDelayType::Udelay.as_u32(), Ordering::Relaxed);
        dmi_io_delay_0xed_port();
        assert_eq!(
            IO_DELAY_TYPE.load(Ordering::Relaxed),
            IoDelayType::Udelay.as_u32()
        );
    }

    #[test]
    fn dmi_table_contains_known_compaq_and_hp_boards() {
        let names: Vec<&str> = IO_DELAY_0XED_DMI_TABLE.iter().map(|q| q.ident).collect();
        assert!(names.contains(&"Compaq Presario V6000"));
        assert!(names.contains(&"HP Pavilion dv9000z"));
        assert!(names.contains(&"HP Pavilion dv6000"));
        assert!(names.contains(&"HP Pavilion tx1000"));
        assert!(names.contains(&"Presario F700"));
    }

    #[test]
    fn dmi_match_finds_quanta_30b7() {
        let q = match_dmi_quirk(IO_DELAY_0XED_DMI_TABLE, "Quanta", "30B7").unwrap();
        assert_eq!(q.ident, "Compaq Presario V6000");
    }

    #[test]
    fn io_delay_init_skips_when_override_active() {
        reset_globals();
        IO_DELAY_OVERRIDE.store(true, Ordering::Relaxed);
        let matches: Vec<&DmiQuirk> = IO_DELAY_0XED_DMI_TABLE.iter().take(1).collect();
        io_delay_init(&matches);
        // Type should remain at the default; override prevented the change.
        assert_eq!(
            IO_DELAY_TYPE.load(Ordering::Relaxed),
            IoDelayType::Port0x80.as_u32()
        );
    }
}
