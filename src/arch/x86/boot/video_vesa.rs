//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/boot/video-vesa.c
//! test-origin: linux:vendor/linux/arch/x86/boot/video-vesa.c
//! VESA (VBE) text and linear-framebuffer mode driver.
//!
//! Ports / mirrors (1:1, no simplification):
//! - vendor/linux/arch/x86/boot/video-vesa.c
//! - vendor/linux/arch/x86/boot/vesa.h (via `super::vesa`)
//!
//! `vesa_probe` issues INT 10h AX=4F00h, validates the `VESA` signature and
//! version, then walks the mode list at `vginfo.video_mode_ptr` calling
//! AX=4F01h per mode and registering text and (when enabled) linear-framebuffer
//! graphics modes. `vesa_set_mode` re-queries the chosen mode and programs it.
//! The graphics-mode parameter capture (`vesa_store_mode_params_graphics`),
//! the DAC-width and protected-mode-info queries, and EDID retrieval are all
//! ported. INT 10h goes through [`BiosCaller`]; the mode-list walk and exact
//! `ES:DI` addresses of `&vginfo/&vminfo` thread the [`VesaMem`] seam.
//!
//! `CONFIG_BOOT_VESA_SUPPORT`, `CONFIG_FIRMWARE_EDID` and the `_WAKEUP` build
//! switches are represented by the `vesa_support`/`firmware_edid`/`wakeup`
//! booleans so the same source covers every Linux build configuration.

use super::biosregs::{BiosCaller, BiosRegs};
use super::regs::initregs;
use super::vesa::{FarPtr, VESA_MAGIC, VesaGeneralInfo, VesaModeInfo};
use super::video::{ModeInfo, ScreenInfo, VIDEO_FIRST_VESA, VIDEO_TYPE_VLFB, VideoState};

/// `video_vesa` card metadata (video-vesa.c:272-279).
pub const VESA_CARD_NAME: &str = "VESA";
/// `.xmode_first = VIDEO_FIRST_VESA`.
pub const VESA_XMODE_FIRST: u16 = VIDEO_FIRST_VESA;
/// `.xmode_n = 0x200`.
pub const VESA_XMODE_N: u16 = 0x200;

/// Build-time switches the C file selects with the preprocessor.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VesaConfig {
    /// `CONFIG_BOOT_VESA_SUPPORT` — register linear-framebuffer graphics modes.
    pub vesa_support: bool,
    /// `CONFIG_FIRMWARE_EDID` — fetch EDID via VBE DDC.
    pub firmware_edid: bool,
    /// `_WAKEUP` — the ACPI-wakeup build, which compiles the graphics path out.
    pub wakeup: bool,
}

impl Default for VesaConfig {
    fn default() -> Self {
        // The default kernel config builds with VESA + firmware EDID and is not
        // the wakeup variant.
        VesaConfig {
            vesa_support: true,
            firmware_edid: true,
            wakeup: false,
        }
    }
}

/// Seam for the far-segment memory the VESA probe touches: `set_fs`, the
/// 16-bit mode-list walk (`rdfs16`), and the `ES:DI` addresses of the
/// `&vginfo`/`&vminfo` buffers the BIOS fills. Production wiring backs these
/// with real buffers; tests stub it.
pub trait VesaMem {
    /// `set_fs(seg)`.
    fn set_fs(&mut self, seg: u16);
    /// `rdfs16(addr)`.
    fn rdfs16(&self, addr: u32) -> u16;
    /// The real-mode `ES:DI` address the BIOS should fill `&vginfo` at.
    fn vginfo_ptr(&self) -> FarPtr;
    /// The real-mode `ES:DI` address the BIOS should fill `&vminfo` at.
    fn vminfo_ptr(&self) -> FarPtr;
}

/// `vesa_probe()` (video-vesa.c:31-102) — query the VBE controller info,
/// validate it, and walk the mode list registering text (and, when
/// `vesa_support`, linear-framebuffer) modes.
///
/// The C code stores `vginfo`/`vminfo` in file-scope statics that the BIOS
/// fills; here the caller owns them and supplies a `query_mode` closure that
/// runs AX=4F01h into `vminfo` at the supplied `ES:DI` address. The closure
/// returns the BIOS AX and lets the probe inspect the freshly filled `vminfo`.
/// `heap_bytes` is the setup-heap space available at the initial GET_HEAP.
pub fn vesa_probe<B, M, Q>(
    bios: &B,
    mem: &mut M,
    vginfo: &mut VesaGeneralInfo,
    vminfo: &mut VesaModeInfo,
    cfg: &VesaConfig,
    mut heap_bytes: usize,
    mut query_mode: Q,
) -> alloc::vec::Vec<ModeInfo>
where
    B: BiosCaller,
    M: VesaMem,
    // Returns BIOS AX for AX=4F01h; fills *vminfo at the supplied ES:DI.
    Q: FnMut(u16, FarPtr, &mut VesaModeInfo) -> u16,
{
    let mut out = alloc::vec::Vec::new();

    let mut ireg = BiosRegs::default();
    let mut oreg = BiosRegs::default();
    initregs(&mut ireg);
    ireg.set_ax(0x4f00);
    let vginfo_ptr = mem.vginfo_ptr();
    ireg.es = vginfo_ptr.seg;
    set_di(&mut ireg, vginfo_ptr.off);
    bios.intcall(0x10, &ireg, Some(&mut oreg));

    if oreg.ax() != 0x004f
        || read_packed_u32(vginfo, 0) != VESA_MAGIC // vginfo.signature
        || read_packed_u16(vginfo, 4) < 0x0102
    // vginfo.version
    {
        return out; // Not present.
    }

    // set_fs(vginfo.video_mode_ptr.seg); mode_ptr = vginfo.video_mode_ptr.off;
    let seg = read_packed_u16(vginfo, 16); // video_mode_ptr.seg (off at 14, seg at 16)
    mem.set_fs(seg);
    let mut mode_ptr = read_packed_u16(vginfo, 14) as u32; // .off

    loop {
        let mode = mem.rdfs16(mode_ptr);
        if mode == 0xffff {
            break;
        }
        mode_ptr += 2;

        let mode_bytes = core::mem::size_of::<ModeInfo>();
        if heap_bytes < mode_bytes {
            break; // Heap full, can't save mode info.
        }

        if mode & !0x1ff != 0 {
            continue;
        }

        // memset(&vminfo, 0, ...).
        *vminfo = VesaModeInfo::default();

        let ax = query_mode(mode, mem.vminfo_ptr(), vminfo);
        if ax != 0x004f {
            continue;
        }

        let mode_attr = read_packed_u16(vminfo, 0);
        let mut accepted = None;
        if (mode_attr & 0x15) == 0x05 {
            // Text Mode, TTY BIOS supported, supported by hardware.
            accepted = Some(ModeInfo {
                mode: mode + VIDEO_FIRST_VESA,
                x: read_packed_u16(vminfo, 18), // h_res
                y: read_packed_u16(vminfo, 20), // v_res
                depth: 0,                       // text
            });
        } else if (mode_attr & 0x99) == 0x99
            && (read_packed_u8(vminfo, 27) == 4 || read_packed_u8(vminfo, 27) == 6) // memory_layout
            && read_packed_u8(vminfo, 24) == 1
        // memory_planes
        {
            if cfg.vesa_support {
                // Graphics mode, color, linear frame buffer supported. Only
                // register if the framebuffer is configured.
                accepted = Some(ModeInfo {
                    mode: mode + VIDEO_FIRST_VESA,
                    x: read_packed_u16(vminfo, 18),           // h_res
                    y: read_packed_u16(vminfo, 20),           // v_res
                    depth: read_packed_u8(vminfo, 25) as u16, // bpp (cast to u16)
                });
            }
        }

        if let Some(mi) = accepted {
            if out.try_reserve_exact(1).is_err() {
                break;
            }
            out.push(mi);
            heap_bytes -= mode_bytes;
        }
    }

    out
}

/// `vesa_set_mode(mode)` (video-vesa.c:104-155) — re-query the chosen mode,
/// classify it as text or linear-framebuffer graphics, set it via AX=4F02h, and
/// either capture text rows/cols (`force_x/force_y`) or the graphics params.
///
/// `query_mode` runs AX=4F01h into `vminfo` at `vminfo_destination` (returns
/// BIOS AX), and `store_graphics` is invoked for the graphics path (the C
/// `vesa_store_mode_params_graphics`, which is a no-op under `_WAKEUP`).
pub fn vesa_set_mode<B, Q, G>(
    bios: &B,
    vminfo: &mut VesaModeInfo,
    vminfo_destination: FarPtr,
    st: &mut VideoState,
    cfg: &VesaConfig,
    mi: &ModeInfo,
    mut query_mode: Q,
    mut store_graphics: G,
) -> i32
where
    B: BiosCaller,
    Q: FnMut(u16, FarPtr, &mut VesaModeInfo) -> u16,
    G: FnMut(&mut VideoState, &VesaModeInfo),
{
    let mut vesa_mode = mi.mode - VIDEO_FIRST_VESA;

    // memset(&vminfo, 0, ...).
    *vminfo = VesaModeInfo::default();

    // AX=4F01h Get Mode Info.
    let ax = query_mode(vesa_mode, vminfo_destination, vminfo);
    if ax != 0x004f {
        return -1;
    }

    let mode_attr = read_packed_u16(vminfo, 0);
    let is_graphic;
    if (mode_attr & 0x15) == 0x05 {
        // It's a supported text mode.
        is_graphic = false;
    } else if cfg.vesa_support && (mode_attr & 0x99) == 0x99 {
        // It's a graphics mode with linear frame buffer.
        is_graphic = true;
        vesa_mode |= 0x4000; // Request linear frame buffer.
    } else {
        return -1; // Invalid mode.
    }

    // AX=4F02h Set Mode.
    let mut ireg = BiosRegs::default();
    let mut oreg = BiosRegs::default();
    initregs(&mut ireg);
    ireg.set_ax(0x4f02);
    set_bx(&mut ireg, vesa_mode);
    bios.intcall(0x10, &ireg, Some(&mut oreg));

    if oreg.ax() != 0x004f {
        return -1;
    }

    st.graphic_mode = is_graphic as i32;
    if !is_graphic {
        // Text mode.
        st.force_x = mi.x as i32;
        st.force_y = mi.y as i32;
        st.do_restore = 1;
    } else {
        // Graphics mode. (no-op under _WAKEUP)
        if !cfg.wakeup {
            store_graphics(st, vminfo);
        }
    }

    0
}

/// `vesa_dac_set_8bits()` (video-vesa.c:161-186) — switch the DAC to 8-bit mode
/// when the controller advertises it, then record the color component sizes.
pub fn vesa_dac_set_8bits<B: BiosCaller>(bios: &B, vginfo: &VesaGeneralInfo, si: &mut ScreenInfo) {
    let mut dac_size: u8 = 6;

    // If possible, switch the DAC to 8-bit mode.
    if read_packed_u32(vginfo, 10) & 1 != 0 {
        // vginfo.capabilities
        let mut ireg = BiosRegs::default();
        let mut oreg = BiosRegs::default();
        initregs(&mut ireg);
        ireg.set_ax(0x4f08);
        set_bh(&mut ireg, 0x08);
        bios.intcall(0x10, &ireg, Some(&mut oreg));
        if oreg.ax() == 0x004f {
            dac_size = bh(&oreg);
        }
    }

    // Set the color sizes to the DAC size, and offsets to 0.
    si.red_size = dac_size;
    si.green_size = dac_size;
    si.blue_size = dac_size;
    si.rsvd_size = dac_size;

    si.red_pos = 0;
    si.green_pos = 0;
    si.blue_pos = 0;
    si.rsvd_pos = 0;
}

/// `vesa_store_pm_info()` (video-vesa.c:189-202) — query the VBE protected-mode
/// interface and record its seg/off.
pub fn vesa_store_pm_info<B: BiosCaller>(bios: &B, si: &mut ScreenInfo) {
    let mut ireg = BiosRegs::default();
    let mut oreg = BiosRegs::default();
    initregs(&mut ireg);
    ireg.set_ax(0x4f0a);
    bios.intcall(0x10, &ireg, Some(&mut oreg));

    if oreg.ax() != 0x004f {
        return;
    }

    si.vesapm_seg = oreg.es; // oreg.es
    si.vesapm_off = oreg.di();
}

/// `vesa_store_mode_params_graphics()` (video-vesa.c:204-230) — capture the
/// linear-framebuffer parameters for the kernel, optionally switching the DAC
/// to 8-bit and saving the protected-mode info.
pub fn vesa_store_mode_params_graphics<B: BiosCaller>(
    bios: &B,
    vginfo: &VesaGeneralInfo,
    vminfo: &VesaModeInfo,
    st: &mut VideoState,
) {
    // Tell the kernel we're in VESA graphics mode.
    st.screen_info.orig_video_isvga = VIDEO_TYPE_VLFB;

    // Mode parameters.
    st.screen_info.vesa_attributes = read_packed_u16(vminfo, 0); // mode_attr
    st.screen_info.lfb_linelength = read_packed_u16(vminfo, 16); // logical_scan
    st.screen_info.lfb_width = read_packed_u16(vminfo, 18); // h_res
    st.screen_info.lfb_height = read_packed_u16(vminfo, 20); // v_res
    st.screen_info.lfb_depth = read_packed_u8(vminfo, 25) as u16; // bpp
    st.screen_info.pages = read_packed_u8(vminfo, 29) as u16; // image_planes
    st.screen_info.lfb_base = read_packed_u32(vminfo, 40); // lfb_ptr

    // memcpy(&screen_info.red_size, &vminfo.rmask, 8): copy the 8 direct-color
    // mask/position bytes starting at vminfo offset 31 (rmask).
    st.screen_info.red_size = read_packed_u8(vminfo, 31); // rmask
    st.screen_info.red_pos = read_packed_u8(vminfo, 32); // rpos
    st.screen_info.green_size = read_packed_u8(vminfo, 33); // gmask
    st.screen_info.green_pos = read_packed_u8(vminfo, 34); // gpos
    st.screen_info.blue_size = read_packed_u8(vminfo, 35); // bmask
    st.screen_info.blue_pos = read_packed_u8(vminfo, 36); // bpos
    st.screen_info.rsvd_size = read_packed_u8(vminfo, 37); // resv_mask
    st.screen_info.rsvd_pos = read_packed_u8(vminfo, 38); // resv_pos

    // General parameters.
    st.screen_info.lfb_size = read_packed_u16(vginfo, 18) as u32; // total_memory

    if read_packed_u8(vminfo, 25) <= 8 {
        // bpp
        let vg = *vginfo;
        vesa_dac_set_8bits(bios, &vg, &mut st.screen_info);
    }

    vesa_store_pm_info(bios, &mut st.screen_info);
}

/// `vesa_store_edid()` (video-vesa.c:236-268) — fetch EDID via VBE DDC into
/// `boot_params.edid_info`. Returns the captured EDID block (128 bytes filled
/// with the 0x13 nonsense token first, then overwritten on success), or `None`
/// when EDID is unsupported by the build. The C function writes into
/// `boot_params.edid_info`; here we return the block so the caller stores it.
pub fn vesa_store_edid<B, R>(
    bios: &B,
    vginfo: &VesaGeneralInfo,
    cfg: &VesaConfig,
    destination: FarPtr,
    mut read_edid: R,
) -> Option<[u8; 128]>
where
    B: BiosCaller,
    // Reads the 128-byte EDID block the BIOS just filled at ES:DI.
    R: FnMut() -> [u8; 128],
{
    if !cfg.firmware_edid {
        return None;
    }

    // Apparently used as a nonsense token...
    let mut edid = [0x13u8; 128];

    if read_packed_u16(vginfo, 4) < 0x0200 {
        // vginfo.version: EDID requires VBE 2.0+.
        return Some(edid);
    }

    let mut ireg = BiosRegs::default();
    let mut oreg = BiosRegs::default();
    initregs(&mut ireg);
    ireg.set_ax(0x4f15); // VBE DDC, report capabilities.
    ireg.es = 0; // ES:DI must be 0 by spec.
    bios.intcall(0x10, &ireg, Some(&mut oreg));

    if oreg.ax() != 0x004f {
        return Some(edid); // No EDID.
    }

    ireg.set_ax(0x4f15); // VBE DDC.
    set_bx(&mut ireg, 0x0001); // Read EDID.
    // Linux passes ds():&boot_params.edid_info as the real-mode destination.
    // The caller owns that boot-parameter storage in this translation and
    // supplies its exact segment:offset address.
    ireg.es = destination.seg;
    set_di(&mut ireg, destination.off);
    bios.intcall(0x10, &ireg, Some(&mut oreg));

    edid = read_edid();
    Some(edid)
}

// --- packed-field readers for vesa.rs structures ----------------------
// vesa.h structs are #[repr(C, packed)] so fields are byte-addressable; we read
// them by offset to mirror the C field accesses without taking references to
// unaligned fields.

fn vesa_general_bytes(g: &VesaGeneralInfo) -> &[u8] {
    // SAFETY: VesaGeneralInfo is repr(C, packed) and exactly 256 bytes; viewing
    // it as a byte slice is sound and lets us read fields by their C offsets.
    unsafe {
        core::slice::from_raw_parts(
            g as *const VesaGeneralInfo as *const u8,
            core::mem::size_of::<VesaGeneralInfo>(),
        )
    }
}
fn vesa_mode_bytes(m: &VesaModeInfo) -> &[u8] {
    // SAFETY: VesaModeInfo is repr(C, packed) and exactly 256 bytes.
    unsafe {
        core::slice::from_raw_parts(
            m as *const VesaModeInfo as *const u8,
            core::mem::size_of::<VesaModeInfo>(),
        )
    }
}

trait PackedRead {
    fn bytes(&self) -> &[u8];
}
impl PackedRead for VesaGeneralInfo {
    fn bytes(&self) -> &[u8] {
        vesa_general_bytes(self)
    }
}
impl PackedRead for VesaModeInfo {
    fn bytes(&self) -> &[u8] {
        vesa_mode_bytes(self)
    }
}

fn read_packed_u8<T: PackedRead>(t: &T, off: usize) -> u8 {
    t.bytes()[off]
}
fn read_packed_u16<T: PackedRead>(t: &T, off: usize) -> u16 {
    let b = t.bytes();
    u16::from_le_bytes([b[off], b[off + 1]])
}
fn read_packed_u32<T: PackedRead>(t: &T, off: usize) -> u32 {
    let b = t.bytes();
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

// --- BiosRegs byte/word accessors not provided by biosregs.rs ---------

#[inline]
fn set_bx(r: &mut BiosRegs, v: u16) {
    r.ebx = (r.ebx & 0xffff_0000) | v as u32;
}
#[inline]
fn set_bh(r: &mut BiosRegs, v: u8) {
    r.ebx = (r.ebx & 0xffff_00ff) | ((v as u32) << 8);
}
#[inline]
fn bh(r: &BiosRegs) -> u8 {
    (r.ebx >> 8) as u8
}
#[inline]
fn set_di(r: &mut BiosRegs, v: u16) {
    r.edi = (r.edi & 0xffff_0000) | v as u32;
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::vec::Vec;
    use core::cell::RefCell;

    // Helpers to populate the packed vesa structs by offset for tests.
    fn write_u16(buf: &mut [u8], off: usize, v: u16) {
        buf[off..off + 2].copy_from_slice(&v.to_le_bytes());
    }
    fn write_u32(buf: &mut [u8], off: usize, v: u32) {
        buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn vginfo_from(bytes: &[u8; 256]) -> VesaGeneralInfo {
        // SAFETY: VesaGeneralInfo is repr(C, packed), exactly 256 bytes.
        unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const VesaGeneralInfo) }
    }
    fn vminfo_from(bytes: &[u8; 256]) -> VesaModeInfo {
        unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const VesaModeInfo) }
    }

    struct StubBios {
        replies: RefCell<alloc::collections::VecDeque<BiosRegs>>,
        // (ax, bx, es, di, cx, dx)
        calls: RefCell<Vec<(u16, u16, u16, u16, u16, u16)>>,
    }
    impl StubBios {
        fn new() -> Self {
            StubBios {
                replies: RefCell::new(alloc::collections::VecDeque::new()),
                calls: RefCell::new(Vec::new()),
            }
        }
        fn push(&self, r: BiosRegs) {
            self.replies.borrow_mut().push_back(r);
        }
        fn push_ax(&self, ax: u16) {
            let mut r = BiosRegs::default();
            r.set_ax(ax);
            self.push(r);
        }
    }
    impl BiosCaller for StubBios {
        fn intcall(&self, _int_no: u8, ireg: &BiosRegs, oreg: Option<&mut BiosRegs>) {
            self.calls.borrow_mut().push((
                ireg.ax(),
                ireg.bx(),
                ireg.es,
                ireg.di(),
                ireg.cx(),
                ireg.dx(),
            ));
            if let Some(o) = oreg {
                *o = self.replies.borrow_mut().pop_front().unwrap_or_default();
            }
        }
    }

    struct StubMem {
        // The mode-id stream rdfs16 returns, terminated by 0xffff.
        mode_words: RefCell<alloc::collections::VecDeque<u16>>,
    }
    impl VesaMem for StubMem {
        fn set_fs(&mut self, _seg: u16) {}
        fn rdfs16(&self, _addr: u32) -> u16 {
            self.mode_words.borrow_mut().pop_front().unwrap_or(0xffff)
        }
        fn vginfo_ptr(&self) -> FarPtr {
            FarPtr {
                off: 0x1000,
                seg: 0x9000,
            }
        }
        fn vminfo_ptr(&self) -> FarPtr {
            FarPtr {
                off: 0x2000,
                seg: 0x9000,
            }
        }
    }

    // ---- card metadata (video-vesa.c:272-279) ------------------------

    #[test]
    fn vesa_card_metadata_matches_video_vesa_c() {
        assert_eq!(VESA_CARD_NAME, "VESA");
        assert_eq!(VESA_XMODE_FIRST, VIDEO_FIRST_VESA);
        assert_eq!(VESA_XMODE_N, 0x200);
    }

    #[test]
    fn vesa_magic_matches_vesa_h() {
        // vesa.h: VESA_MAGIC = 'V' + 'E'<<8 + 'S'<<16 + 'A'<<24.
        assert_eq!(VESA_MAGIC, 0x4153_4556);
    }

    // ---- vesa_probe --------------------------------------------------

    #[test]
    fn vesa_probe_rejects_missing_controller() {
        // AX != 0x004f => not present.
        let bios = StubBios::new();
        bios.push_ax(0xffff);
        let mut mem = StubMem {
            mode_words: RefCell::new(alloc::collections::VecDeque::new()),
        };
        let mut vginfo = VesaGeneralInfo::default();
        let mut vminfo = VesaModeInfo::default();
        let cfg = VesaConfig::default();
        let modes = vesa_probe(
            &bios,
            &mut mem,
            &mut vginfo,
            &mut vminfo,
            &cfg,
            usize::MAX,
            |_, _, _| 0x004f,
        );
        assert!(modes.is_empty());
    }

    #[test]
    fn vesa_probe_rejects_bad_signature_or_old_version() {
        let bios = StubBios::new();
        bios.push_ax(0x004f);
        let mut mem = StubMem {
            mode_words: RefCell::new(alloc::collections::VecDeque::new()),
        };
        // Signature wrong.
        let mut g = [0u8; 256];
        write_u32(&mut g, 0, 0xDEAD_BEEF);
        write_u16(&mut g, 4, 0x0200);
        let mut vginfo = vginfo_from(&g);
        let mut vminfo = VesaModeInfo::default();
        let cfg = VesaConfig::default();
        let modes = vesa_probe(
            &bios,
            &mut mem,
            &mut vginfo,
            &mut vminfo,
            &cfg,
            usize::MAX,
            |_, _, _| 0x004f,
        );
        assert!(modes.is_empty());
    }

    #[test]
    fn vesa_probe_registers_text_mode() {
        let bios = StubBios::new();
        bios.push_ax(0x004f); // AX=4F00 controller info OK
        let mut mem = StubMem {
            mode_words: RefCell::new([0x0108u16, 0xffff].into_iter().collect()),
        };
        // Valid controller info: signature, version 0x0200, mode_ptr seg/off.
        let mut g = [0u8; 256];
        write_u32(&mut g, 0, VESA_MAGIC);
        write_u16(&mut g, 4, 0x0200);
        write_u16(&mut g, 14, 0x0000); // video_mode_ptr.off
        write_u16(&mut g, 16, 0xc000); // video_mode_ptr.seg
        let mut vginfo = vginfo_from(&g);
        let mut vminfo = VesaModeInfo::default();
        let cfg = VesaConfig::default();

        let modes = vesa_probe(
            &bios,
            &mut mem,
            &mut vginfo,
            &mut vminfo,
            &cfg,
            usize::MAX,
            |_mode, destination, vm| {
                let destination = (destination.seg, destination.off);
                assert_eq!(destination, (0x9000, 0x2000));
                // Fill vminfo: text mode attr 0x05, h_res 132, v_res 60.
                let mut b = [0u8; 256];
                write_u16(&mut b, 0, 0x05); // mode_attr text+hw
                write_u16(&mut b, 18, 132); // h_res
                write_u16(&mut b, 20, 60); // v_res
                *vm = vminfo_from(&b);
                0x004f
            },
        );

        assert_eq!(modes.len(), 1);
        assert_eq!(
            modes[0],
            ModeInfo {
                mode: 0x0108 + VIDEO_FIRST_VESA,
                x: 132,
                y: 60,
                depth: 0
            }
        );
        assert_eq!(
            bios.calls.borrow()[0],
            (0x4f00, 0, 0x9000, 0x1000, 0, 0),
            "controller query must address vginfo through ES:DI"
        );
    }

    #[test]
    fn vesa_probe_registers_graphics_mode_with_depth() {
        let bios = StubBios::new();
        bios.push_ax(0x004f);
        let mut mem = StubMem {
            mode_words: RefCell::new([0x0117u16, 0xffff].into_iter().collect()),
        };
        let mut g = [0u8; 256];
        write_u32(&mut g, 0, VESA_MAGIC);
        write_u16(&mut g, 4, 0x0200);
        let mut vginfo = vginfo_from(&g);
        let mut vminfo = VesaModeInfo::default();
        let cfg = VesaConfig::default();

        let modes = vesa_probe(
            &bios,
            &mut mem,
            &mut vginfo,
            &mut vminfo,
            &cfg,
            usize::MAX,
            |_mode, _destination, vm| {
                // attr 0x99, memory_layout 6, memory_planes 1, bpp 16, 1024x768.
                let mut b = [0u8; 256];
                write_u16(&mut b, 0, 0x99); // mode_attr
                write_u16(&mut b, 18, 1024); // h_res
                write_u16(&mut b, 20, 768); // v_res
                b[24] = 1; // memory_planes
                b[25] = 16; // bpp
                b[27] = 6; // memory_layout
                *vm = vminfo_from(&b);
                0x004f
            },
        );

        assert_eq!(modes.len(), 1);
        assert_eq!(
            modes[0],
            ModeInfo {
                mode: 0x0117 + VIDEO_FIRST_VESA,
                x: 1024,
                y: 768,
                depth: 16
            }
        );
    }

    #[test]
    fn vesa_probe_skips_modes_with_high_bits() {
        let bios = StubBios::new();
        bios.push_ax(0x004f);
        // 0x4108 has bits outside 0x1ff set => skipped; then terminator.
        let mut mem = StubMem {
            mode_words: RefCell::new([0x4108u16, 0xffff].into_iter().collect()),
        };
        let mut g = [0u8; 256];
        write_u32(&mut g, 0, VESA_MAGIC);
        write_u16(&mut g, 4, 0x0200);
        let mut vginfo = vginfo_from(&g);
        let mut vminfo = VesaModeInfo::default();
        let cfg = VesaConfig::default();
        let mut queried = false;
        let modes = vesa_probe(
            &bios,
            &mut mem,
            &mut vginfo,
            &mut vminfo,
            &cfg,
            usize::MAX,
            |_, _, _| {
                queried = true;
                0x004f
            },
        );
        assert!(modes.is_empty());
        assert!(!queried, "mode with high bits must be skipped before query");
    }

    #[test]
    fn vesa_probe_omits_graphics_when_support_disabled() {
        let bios = StubBios::new();
        bios.push_ax(0x004f);
        let mut mem = StubMem {
            mode_words: RefCell::new([0x0117u16, 0xffff].into_iter().collect()),
        };
        let mut g = [0u8; 256];
        write_u32(&mut g, 0, VESA_MAGIC);
        write_u16(&mut g, 4, 0x0200);
        let mut vginfo = vginfo_from(&g);
        let mut vminfo = VesaModeInfo::default();
        let cfg = VesaConfig {
            vesa_support: false,
            ..Default::default()
        };
        let modes = vesa_probe(
            &bios,
            &mut mem,
            &mut vginfo,
            &mut vminfo,
            &cfg,
            usize::MAX,
            |_mode, _destination, vm| {
                let mut b = [0u8; 256];
                write_u16(&mut b, 0, 0x99);
                b[24] = 1;
                b[25] = 32;
                b[27] = 6;
                *vm = vminfo_from(&b);
                0x004f
            },
        );
        assert!(modes.is_empty());
    }

    #[test]
    fn vesa_probe_breaks_before_query_when_heap_cannot_fit_mode_info() {
        let bios = StubBios::new();
        bios.push_ax(0x004f);
        let mut mem = StubMem {
            mode_words: RefCell::new([0x0108u16, 0xffff].into_iter().collect()),
        };
        let mut g = [0u8; 256];
        write_u32(&mut g, 0, VESA_MAGIC);
        write_u16(&mut g, 4, 0x0200);
        let mut vginfo = vginfo_from(&g);
        let mut vminfo = VesaModeInfo::default();
        let mut queried = false;

        let modes = vesa_probe(
            &bios,
            &mut mem,
            &mut vginfo,
            &mut vminfo,
            &VesaConfig::default(),
            core::mem::size_of::<ModeInfo>() - 1,
            |_, _, _| {
                queried = true;
                0x004f
            },
        );

        assert!(modes.is_empty());
        assert!(!queried);
    }

    // ---- vesa_set_mode -----------------------------------------------

    #[test]
    fn vesa_set_mode_text_sets_force_xy_and_restore() {
        let bios = StubBios::new();
        // AX=4F02 reply OK.
        bios.push_ax(0x004f);
        let mut vminfo = VesaModeInfo::default();
        let mut st = VideoState::default();
        let cfg = VesaConfig::default();
        let mi = ModeInfo {
            mode: 0x0108 + VIDEO_FIRST_VESA,
            x: 132,
            y: 60,
            depth: 0,
        };
        let rv = vesa_set_mode(
            &bios,
            &mut vminfo,
            FarPtr {
                off: 0x2000,
                seg: 0x9000,
            },
            &mut st,
            &cfg,
            &mi,
            |_m, _destination, vm| {
                let mut b = [0u8; 256];
                write_u16(&mut b, 0, 0x05); // text
                *vm = vminfo_from(&b);
                0x004f
            },
            |_, _| panic!("graphics store must not run for text mode"),
        );
        assert_eq!(rv, 0);
        assert_eq!(st.graphic_mode, 0);
        assert_eq!(st.force_x, 132);
        assert_eq!(st.force_y, 60);
        assert_eq!(st.do_restore, 1);
        // The AX=4F02 set call carried BX = raw vesa mode (no 0x4000 for text).
        let set_call = bios.calls.borrow().iter().find(|c| c.0 == 0x4f02).copied();
        assert_eq!(set_call.unwrap().1, 0x0108);
    }

    #[test]
    fn vesa_set_mode_graphics_requests_linear_fb_and_stores_params() {
        let bios = StubBios::new();
        bios.push_ax(0x004f); // AX=4F02 reply OK
        let mut vminfo = VesaModeInfo::default();
        let mut st = VideoState::default();
        let cfg = VesaConfig::default();
        let mi = ModeInfo {
            mode: 0x0117 + VIDEO_FIRST_VESA,
            x: 1024,
            y: 768,
            depth: 16,
        };
        let mut stored = false;
        let rv = vesa_set_mode(
            &bios,
            &mut vminfo,
            FarPtr {
                off: 0x2000,
                seg: 0x9000,
            },
            &mut st,
            &cfg,
            &mi,
            |_m, _destination, vm| {
                let mut b = [0u8; 256];
                write_u16(&mut b, 0, 0x99); // graphics+lfb
                *vm = vminfo_from(&b);
                0x004f
            },
            |_st, _vm| stored = true,
        );
        assert_eq!(rv, 0);
        assert_eq!(st.graphic_mode, 1);
        assert!(stored);
        // BX carried the 0x4000 linear-framebuffer request bit.
        let set_call = bios.calls.borrow().iter().find(|c| c.0 == 0x4f02).copied();
        assert_eq!(set_call.unwrap().1, 0x0117 | 0x4000);
    }

    #[test]
    fn vesa_set_mode_rejects_unsupported_attrs() {
        let bios = StubBios::new();
        let mut vminfo = VesaModeInfo::default();
        let mut st = VideoState::default();
        let cfg = VesaConfig::default();
        let mi = ModeInfo {
            mode: 0x0200 + VIDEO_FIRST_VESA,
            x: 0,
            y: 0,
            depth: 0,
        };
        let rv = vesa_set_mode(
            &bios,
            &mut vminfo,
            FarPtr {
                off: 0x2000,
                seg: 0x9000,
            },
            &mut st,
            &cfg,
            &mi,
            |_m, _destination, vm| {
                let mut b = [0u8; 256];
                write_u16(&mut b, 0, 0x00); // neither text nor graphics
                *vm = vminfo_from(&b);
                0x004f
            },
            |_, _| {},
        );
        assert_eq!(rv, -1);
    }

    #[test]
    fn vesa_set_mode_returns_minus_one_when_query_fails() {
        let bios = StubBios::new();
        let mut vminfo = VesaModeInfo::default();
        let mut st = VideoState::default();
        let cfg = VesaConfig::default();
        let mi = ModeInfo {
            mode: VIDEO_FIRST_VESA + 0x100,
            x: 0,
            y: 0,
            depth: 0,
        };
        let rv = vesa_set_mode(
            &bios,
            &mut vminfo,
            FarPtr {
                off: 0x2000,
                seg: 0x9000,
            },
            &mut st,
            &cfg,
            &mi,
            |_, _, _| 0xffff,
            |_, _| {},
        );
        assert_eq!(rv, -1);
    }

    // ---- vesa_dac_set_8bits / pm info / graphics params --------------

    #[test]
    fn vesa_dac_set_8bits_uses_returned_size_when_capable() {
        // capabilities bit 0 set; AX=4F08 reply OK with BH=8.
        let bios = StubBios::new();
        let mut r = BiosRegs::default();
        r.set_ax(0x004f);
        r.ebx = 0x0800; // BH=8
        bios.push(r);
        let mut g = [0u8; 256];
        write_u32(&mut g, 10, 0x0000_0001); // capabilities bit 0
        let vginfo = vginfo_from(&g);
        let mut si = ScreenInfo::default();
        vesa_dac_set_8bits(&bios, &vginfo, &mut si);
        assert_eq!(si.red_size, 8);
        assert_eq!(si.green_size, 8);
        assert_eq!(si.blue_size, 8);
        assert_eq!(si.rsvd_size, 8);
        assert_eq!(si.red_pos, 0);
    }

    #[test]
    fn vesa_dac_set_8bits_defaults_to_6_when_not_capable() {
        let bios = StubBios::new();
        let vginfo = VesaGeneralInfo::default(); // capabilities = 0
        let mut si = ScreenInfo::default();
        vesa_dac_set_8bits(&bios, &vginfo, &mut si);
        assert_eq!(si.red_size, 6);
        // No BIOS call should have been issued.
        assert!(bios.calls.borrow().is_empty());
    }

    #[test]
    fn vesa_store_pm_info_records_seg_and_off() {
        let bios = StubBios::new();
        let mut r = BiosRegs::default();
        r.set_ax(0x004f);
        r.es = 0xc000;
        r.edi = 0x1234;
        bios.push(r);
        let mut si = ScreenInfo::default();
        vesa_store_pm_info(&bios, &mut si);
        assert_eq!(si.vesapm_seg, 0xc000);
        assert_eq!(si.vesapm_off, 0x1234);
    }

    #[test]
    fn vesa_store_mode_params_graphics_captures_lfb_fields() {
        let bios = StubBios::new();
        // bpp > 8 so no DAC call; pm-info reply OK.
        bios.push_ax(0x004f); // vesa_store_pm_info reply
        let mut g = [0u8; 256];
        write_u16(&mut g, 18, 256); // total_memory (64KiB blocks)
        let vginfo = vginfo_from(&g);

        let mut m = [0u8; 256];
        write_u16(&mut m, 0, 0x99); // mode_attr
        write_u16(&mut m, 16, 4096); // logical_scan
        write_u16(&mut m, 18, 1024); // h_res
        write_u16(&mut m, 20, 768); // v_res
        m[25] = 32; // bpp
        m[29] = 1; // image_planes
        write_u32(&mut m, 40, 0xE000_0000); // lfb_ptr
        m[31] = 8; // rmask
        m[32] = 16; // rpos
        let vminfo = vminfo_from(&m);

        let mut st = VideoState::default();
        vesa_store_mode_params_graphics(&bios, &vginfo, &vminfo, &mut st);

        assert_eq!(st.screen_info.orig_video_isvga, VIDEO_TYPE_VLFB);
        assert_eq!(st.screen_info.vesa_attributes, 0x99);
        assert_eq!(st.screen_info.lfb_linelength, 4096);
        assert_eq!(st.screen_info.lfb_width, 1024);
        assert_eq!(st.screen_info.lfb_height, 768);
        assert_eq!(st.screen_info.lfb_depth, 32);
        assert_eq!(st.screen_info.pages, 1);
        assert_eq!(st.screen_info.lfb_base, 0xE000_0000);
        assert_eq!(st.screen_info.lfb_size, 256);
        assert_eq!(st.screen_info.red_size, 8);
        assert_eq!(st.screen_info.red_pos, 16);
    }

    // ---- vesa_store_edid ---------------------------------------------

    #[test]
    fn vesa_store_edid_returns_none_when_firmware_edid_disabled() {
        let bios = StubBios::new();
        let vginfo = VesaGeneralInfo::default();
        let cfg = VesaConfig {
            firmware_edid: false,
            ..Default::default()
        };
        assert!(
            vesa_store_edid(
                &bios,
                &vginfo,
                &cfg,
                FarPtr {
                    off: 0x1234,
                    seg: 0x9000,
                },
                || [0u8; 128],
            )
            .is_none()
        );
    }

    #[test]
    fn vesa_store_edid_returns_token_block_for_old_vbe() {
        let bios = StubBios::new();
        // version < 0x0200 => returns the 0x13-filled token, no BIOS DDC call.
        let mut g = [0u8; 256];
        write_u16(&mut g, 4, 0x0102);
        let vginfo = vginfo_from(&g);
        let cfg = VesaConfig::default();
        let block = vesa_store_edid(
            &bios,
            &vginfo,
            &cfg,
            FarPtr {
                off: 0x1234,
                seg: 0x9000,
            },
            || [0xAB; 128],
        )
        .unwrap();
        assert!(block.iter().all(|&b| b == 0x13));
        assert!(bios.calls.borrow().is_empty());
    }

    #[test]
    fn vesa_store_edid_reads_block_on_success() {
        let bios = StubBios::new();
        bios.push_ax(0x004f); // DDC capabilities OK
        bios.push_ax(0x004f); // Read EDID OK
        let mut g = [0u8; 256];
        write_u16(&mut g, 4, 0x0200);
        let vginfo = vginfo_from(&g);
        let cfg = VesaConfig::default();
        let block = vesa_store_edid(
            &bios,
            &vginfo,
            &cfg,
            FarPtr {
                off: 0x2468,
                seg: 0x9000,
            },
            || [0x42; 128],
        )
        .unwrap();
        assert!(block.iter().all(|&b| b == 0x42));
        // Two DDC calls were issued.
        let calls = bios.calls.borrow();
        assert_eq!(calls.len(), 2);
        // Capability query: BX=CX=DX=0 and ES:DI=0000:0000.
        assert_eq!(calls[0], (0x4f15, 0, 0, 0, 0, 0));
        // Read: BX=1, controller/block remain zero, and ES:DI is the caller's
        // exact boot_params.edid_info real-mode address.
        assert_eq!(calls[1], (0x4f15, 1, 0x9000, 0x2468, 0, 0));
    }
}
