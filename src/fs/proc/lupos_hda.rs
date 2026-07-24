//! test-origin: lupos-specific:no Linux counterpart, see rationale below
//!
//! Rust-only source: there is no `vendor/linux` file this mirrors, so it
//! carries no `linux-parity`/`linux-source` header and the layout audit
//! classifies it as an exception rather than a mapped translation.
//! `/proc/lupos_hda` — live Intel HD Audio controller register dump.
//!
//! # Why this exists (Lupos-specific, no Linux counterpart)
//!
//! Linux exposes this state through the sound core's own procfs tree
//! (`/proc/asound/card0/...`, built by `snd_info_*`). On Lupos that tree does
//! not exist: the `proc_create*`/`proc_mkdir*` symbols exported to modules in
//! `src/fs/procfs_abi.rs` only push a record onto a private vector and return
//! non-NULL, so `snd.ko` is told its nodes were created while userspace can
//! never see them. Until that bridge is written, there is no way to read the
//! controller state from inside the guest.
//!
//! That gap blocks a specific, measured question. The HDA stream drains its
//! ring at exactly 48 kHz, yet across a ~7 s playback the shared `irq 11`
//! advanced only 67-69 counts while period completions should have added
//! several hundred. Two explanations remain and they need opposite fixes:
//!
//! * `INTSTS`/`SDnSTS` show a pending completion while `irq 11` stays flat →
//!   the device raises the interrupt and the INTx assertion never reaches the
//!   I/O APIC (a Lupos routing problem);
//! * they are clear, and `INTCTL`'s GIE/SIE or `SDnCTL`'s IOCE are not set →
//!   the controller was never armed to interrupt at all.
//!
//! This file reads those registers directly off the device's BAR0 so the two
//! can be told apart. It follows the existing `/proc/lupos_boot_trace`
//! precedent for a Lupos-only diagnostic node.
//!
//! Register offsets are from the Intel High Definition Audio specification and
//! match `vendor/linux/sound/hda/core/hdac_controller.c` / `include/sound/hdaudio.h`.

extern crate alloc;

use alloc::sync::Arc;

use crate::fs::kernfs::KernfsNode;

/// PCI class/subclass for an HD Audio controller (`PCI_CLASS_MULTIMEDIA_HD_AUDIO`).
const PCI_CLASS_MULTIMEDIA: u8 = 0x04;
const PCI_SUBCLASS_HDA: u8 = 0x03;

// Global controller registers.
const GCAP: u64 = 0x00;
const GCTL: u64 = 0x08;
const INTCTL: u64 = 0x20;
const INTSTS: u64 = 0x24;
const SSYNC: u64 = 0x38;

/// First stream descriptor and its stride, per the HDA specification.
const SD_BASE: u64 = 0x80;
const SD_STRIDE: u64 = 0x20;
const SD_CTL: u64 = 0x00;
const SD_STS: u64 = 0x03;
const SD_LPIB: u64 = 0x04;

/// Bytes of BAR0 to map: global registers plus eight stream descriptors.
const MAP_SIZE: u64 = SD_BASE + 8 * SD_STRIDE;

fn hda_bar0() -> Option<(u64, alloc::string::String)> {
    for dev in crate::linux_driver_abi::pci::enumerate::pci_devices() {
        if dev.class != PCI_CLASS_MULTIMEDIA || dev.subclass != PCI_SUBCLASS_HDA {
            continue;
        }
        let bar = dev.bars[0].as_ref()?;
        if !bar.is_mmio || bar.base == 0 {
            continue;
        }
        return Some((
            bar.base,
            alloc::format!(
                "{:04x}:{:02x}:{:02x}.{} vendor={:#06x} device={:#06x}",
                dev.seg,
                dev.bus,
                dev.dev,
                dev.func,
                dev.vendor,
                dev.device
            ),
        ));
    }
    None
}

pub fn show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    use core::fmt::Write as _;

    let mut out = alloc::string::String::new();

    let Some((base, ident)) = hda_bar0() else {
        return super::util::copy_into(buf, "no HD Audio controller found\n");
    };
    let _ = writeln!(out, "hda {ident} bar0={base:#018x}");

    // Uncached: these are device registers, and a cached view could report
    // stale status bits, which is exactly what this file exists to rule out.
    let mapping = match unsafe { crate::arch::x86::mm::ioremap::ioremap_uc(base, MAP_SIZE) } {
        Ok(mapping) => mapping,
        Err(err) => {
            let _ = writeln!(out, "ioremap_uc failed: {err:?}");
            return super::util::copy_into(buf, out.as_str());
        }
    };

    let read32 = |off: u64| -> u32 {
        unsafe { core::ptr::read_volatile((mapping.virt + off) as *const u32) }
    };
    let read16 = |off: u64| -> u16 {
        unsafe { core::ptr::read_volatile((mapping.virt + off) as *const u16) }
    };
    let read8 =
        |off: u64| -> u8 { unsafe { core::ptr::read_volatile((mapping.virt + off) as *const u8) } };

    let gcap = read16(GCAP);
    let intctl = read32(INTCTL);
    let intsts = read32(INTSTS);
    let _ = writeln!(
        out,
        "GCAP={gcap:#06x} iss={} oss={} bss={}",
        (gcap >> 8) & 0xF,
        (gcap >> 12) & 0xF,
        (gcap >> 3) & 0x1F
    );
    let _ = writeln!(out, "GCTL={:#010x}", read32(GCTL));
    let _ = writeln!(
        out,
        "INTCTL={intctl:#010x} GIE={} CIE={} SIE={:#010x}",
        (intctl >> 31) & 1,
        (intctl >> 30) & 1,
        intctl & 0x3FFF_FFFF
    );
    let _ = writeln!(
        out,
        "INTSTS={intsts:#010x} GIS={} CIS={} SIS={:#010x}",
        (intsts >> 31) & 1,
        (intsts >> 30) & 1,
        intsts & 0x3FFF_FFFF
    );
    let _ = writeln!(out, "SSYNC={:#010x}", read32(SSYNC));

    for stream in 0..8u64 {
        let sd = SD_BASE + stream * SD_STRIDE;
        let ctl = read32(sd + SD_CTL) & 0x00FF_FFFF;
        let sts = read8(sd + SD_STS);
        let _ = writeln!(
            out,
            "SD{stream} CTL={ctl:#08x} RUN={} IOCE={} FEIE={} DEIE={} STS={sts:#04x} BCIS={} FIFOE={} DESE={} LPIB={:#010x}",
            (ctl >> 1) & 1,
            (ctl >> 2) & 1,
            (ctl >> 3) & 1,
            (ctl >> 4) & 1,
            (sts >> 2) & 1,
            (sts >> 3) & 1,
            (sts >> 4) & 1,
            read32(sd + SD_LPIB)
        );
    }

    super::util::copy_into(buf, out.as_str())
}
