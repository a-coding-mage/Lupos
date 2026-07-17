//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/kprobes/core.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/kprobes/core.c
//! x86 kprobes instruction preparation and trap handling.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/kprobes/core.c
//!
//! Live text mutation remains behind a trait seam, but the x86 rules Linux
//! relies on here are real: probeable instruction decoding, RIP-relative
//! displacement repair, relative branch synthesis, boostability checks, and
//! the INT3 trap bridge into the generic kprobe registry.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::arch::x86::kernel::alternative::{CALL_INSN_OPCODE, JMP32_INSN_OPCODE};
use crate::arch::x86::kernel::idt::ExceptionFrame;
use crate::arch::x86::kernel::jump_label::text_gen_insn;
use crate::arch::x86::lib::insn::{Insn, MAX_INSN_SIZE};
use crate::include::uapi::errno::{EFAULT, EINVAL, ENOMEM, EOPNOTSUPP};

pub const INT3_INSN_OPCODE: u8 = 0xcc;
pub const INT3_INSN_SIZE: usize = 1;
pub const RELATIVEJUMP_OPCODE: u8 = JMP32_INSN_OPCODE;
pub const RELATIVECALL_OPCODE: u8 = CALL_INSN_OPCODE;
pub const RELATIVE_ADDR_SIZE: usize = 4;
pub const RELATIVE_INSN_SIZE: usize = 1 + RELATIVE_ADDR_SIZE;
pub const X86_EFLAGS_IF: u64 = 1 << 9;

pub trait KernelText {
    fn read(&self, ip: u64, len: usize) -> Result<Vec<u8>, i32>;
}

pub trait KprobeTextPoke {
    fn poke(&self, ip: u64, bytes: &[u8]) -> Result<(), i32>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CopiedInsn {
    pub original_ip: u64,
    pub slot_ip: u64,
    pub bytes: [u8; MAX_INSN_SIZE],
    pub len: usize,
    pub rip_relative_fixup: Option<i32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArchKprobe {
    pub addr: u64,
    pub opcode: u8,
    pub copied: CopiedInsn,
    pub boostable: bool,
    pub armed: bool,
}

/// Production ownership for one executable displaced-instruction slot.
pub struct LiveKprobe {
    pub arch: ArchKprobe,
    slot: usize,
}

unsafe impl Send for LiveKprobe {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KprobeExecution {
    pub original_ip: u64,
    pub slot_ip: u64,
    pub instruction_len: usize,
    pub bytes: [u8; MAX_INSN_SIZE],
    pub behavior: KprobeExecutionBehavior,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KprobeExecutionBehavior {
    /// Branches and IF-only instructions can be completed in the #BP frame;
    /// they must not execute from a displaced address.
    pub emulate_in_breakpoint: bool,
    /// A displaced CALL pushes a slot-relative return address.  Rewrite it to
    /// the original instruction's fall-through address before resuming.
    pub repair_call_return: bool,
    /// PUSHF observes the kprobe's private TF/IF state.  Replace the pushed
    /// word with the flags which existed at the probed instruction.
    pub repair_pushed_flags: bool,
    /// POPF supplies the architectural post-instruction flags.  Preserve
    /// those flags instead of restoring the kprobe's saved IF and clearing TF.
    pub keep_result_flags: bool,
}

impl LiveKprobe {
    pub fn execution(&self) -> KprobeExecution {
        KprobeExecution {
            original_ip: self.arch.addr,
            slot_ip: self.arch.copied.slot_ip,
            instruction_len: self.arch.copied.len,
            bytes: self.arch.copied.bytes,
            behavior: classify_execution_behavior(&self.arch.copied.bytes, self.arch.copied.len),
        }
    }
}

pub(super) struct ProductionText;

impl KernelText for ProductionText {
    fn read(&self, ip: u64, len: usize) -> Result<Vec<u8>, i32> {
        crate::arch::x86::kernel::alternative::text_poke_read(ip as usize, len)
    }
}

impl KprobeTextPoke for ProductionText {
    fn poke(&self, ip: u64, bytes: &[u8]) -> Result<(), i32> {
        crate::arch::x86::kernel::alternative::text_poke_live(ip as usize, bytes)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmulatedRegs {
    pub ip: u64,
    pub sp: u64,
    pub flags: u64,
    pub cx: u64,
    pub stack: Vec<u64>,
}

impl EmulatedRegs {
    pub fn new(ip: u64) -> Self {
        Self {
            ip,
            sp: 0,
            flags: 0,
            cx: 0,
            stack: Vec::new(),
        }
    }
}

pub fn synthesize_reljump(from: u64, to: u64) -> [u8; RELATIVE_INSN_SIZE] {
    let v = text_gen_insn(RELATIVEJUMP_OPCODE, RELATIVE_INSN_SIZE, from, to);
    [v[0], v[1], v[2], v[3], v[4]]
}

pub fn synthesize_relcall(from: u64, to: u64) -> [u8; RELATIVE_INSN_SIZE] {
    let v = text_gen_insn(RELATIVECALL_OPCODE, RELATIVE_INSN_SIZE, from, to);
    [v[0], v[1], v[2], v[3], v[4]]
}

pub fn decode_instruction(bytes: &[u8]) -> Result<Insn, i32> {
    let mut insn = Insn::init(bytes, true);
    let len = insn.get_length() as usize;
    if len == 0 || len > bytes.len() {
        Err(EFAULT)
    } else {
        Ok(insn)
    }
}

pub fn can_probe(bytes: &[u8]) -> bool {
    decode_instruction(bytes)
        .map(|insn| insn.length != 0 && bytes.first().copied() != Some(INT3_INSN_OPCODE))
        .unwrap_or(false)
}

pub fn can_boost(bytes: &[u8]) -> bool {
    if !can_probe(bytes) {
        return false;
    }
    let op = first_opcode(bytes);
    !matches!(
        op,
        0xc2 | 0xc3 | 0xca | 0xcb | 0xcf | 0xe0..=0xe3 | 0xe8 | 0xe9 | 0xeb | 0xfa | 0xfb | 0x9d
    ) && !is_jcc(bytes)
}

pub fn first_opcode(bytes: &[u8]) -> u8 {
    bytes.get(opcode_offset(bytes)).copied().unwrap_or(0)
}

fn opcode_offset(bytes: &[u8]) -> usize {
    let mut i = 0usize;
    while i < bytes.len()
        && matches!(
            bytes[i],
            0x26 | 0x2e | 0x36 | 0x3e | 0x64 | 0x65 | 0x66 | 0x67 | 0xf0 | 0xf2 | 0xf3
        )
    {
        i += 1;
    }
    if i < bytes.len() && (0x40..=0x4f).contains(&bytes[i]) {
        i += 1;
    }
    i
}

pub fn is_jcc(bytes: &[u8]) -> bool {
    let offset = opcode_offset(bytes);
    let op = first_opcode(bytes);
    (0x70..=0x7f).contains(&op)
        || (op == 0x0f
            && bytes
                .get(offset + 1)
                .is_some_and(|b| (0x80..=0x8f).contains(b)))
}

fn encode_rel32(from_next_ip: u64, target: u64) -> Result<i32, i32> {
    let displacement = target.wrapping_sub(from_next_ip) as u32 as i32;
    if from_next_ip.wrapping_add_signed(displacement as i64) == target {
        Ok(displacement)
    } else {
        Err(EINVAL)
    }
}

pub fn copy_instruction(
    original_ip: u64,
    slot_ip: u64,
    original: &[u8],
) -> Result<CopiedInsn, i32> {
    let insn = decode_instruction(original)?;
    let len = insn.length as usize;
    let mut bytes = [0u8; MAX_INSN_SIZE];
    bytes[..len].copy_from_slice(&original[..len]);

    let mut rip_relative_fixup = None;
    if insn.modrm.got != 0 {
        let modrm = insn.modrm.value as u8;
        let mode = (modrm >> 6) & 0x3;
        let rm = modrm & 0x7;
        if mode == 0 && rm == 5 && insn.displacement.nbytes == 4 {
            let disp_off = insn.prefixes.nbytes as usize
                + insn.rex_prefix.nbytes as usize
                + insn.opcode.nbytes as usize
                + insn.modrm.nbytes as usize
                + insn.sib.nbytes as usize;
            let old_disp = i32::from_le_bytes(bytes[disp_off..disp_off + 4].try_into().unwrap());
            let target = original_ip
                .wrapping_add(len as u64)
                .wrapping_add(old_disp as i64 as u64);
            let new_disp = encode_rel32(slot_ip.wrapping_add(len as u64), target)?;
            bytes[disp_off..disp_off + 4].copy_from_slice(&(new_disp as i32).to_le_bytes());
            rip_relative_fixup = Some(new_disp);
        }
    }

    // Relative CALL is executed from the displaced slot so the processor
    // performs the real stack update.  Keep its destination unchanged; the
    // resulting slot-relative return address is repaired in the #DB handler.
    let opcode_offset = opcode_offset(&bytes[..len]);
    if bytes.get(opcode_offset).copied() == Some(RELATIVECALL_OPCODE) {
        let immediate_offset = len.checked_sub(4).ok_or(EINVAL)?;
        if immediate_offset <= opcode_offset {
            return Err(EINVAL);
        }
        let old = i32::from_le_bytes(
            bytes[immediate_offset..immediate_offset + 4]
                .try_into()
                .unwrap(),
        );
        let target = original_ip
            .wrapping_add(len as u64)
            .wrapping_add(old as i64 as u64);
        let replacement = encode_rel32(slot_ip.wrapping_add(len as u64), target)?;
        bytes[immediate_offset..immediate_offset + 4].copy_from_slice(&replacement.to_le_bytes());
    }

    Ok(CopiedInsn {
        original_ip,
        slot_ip,
        bytes,
        len,
        rip_relative_fixup,
    })
}

pub fn recover_probed_instruction(copied: &CopiedInsn) -> Vec<u8> {
    copied.bytes[..copied.len].to_vec()
}

pub fn arch_prepare_kprobe<T: KernelText>(
    text: &T,
    addr: u64,
    slot_ip: u64,
) -> Result<ArchKprobe, i32> {
    let bytes = text.read(addr, MAX_INSN_SIZE)?;
    let copied = copy_instruction(addr, slot_ip, &bytes)?;
    Ok(ArchKprobe {
        addr,
        opcode: bytes[0],
        copied,
        boostable: can_boost(&bytes),
        armed: false,
    })
}

pub fn arch_arm_kprobe<P: KprobeTextPoke>(poker: &P, kp: &mut ArchKprobe) -> Result<(), i32> {
    poker.poke(kp.addr, &[INT3_INSN_OPCODE])?;
    kp.armed = true;
    Ok(())
}

pub fn arch_disarm_kprobe<P: KprobeTextPoke>(poker: &P, kp: &mut ArchKprobe) -> Result<(), i32> {
    poker.poke(kp.addr, &[kp.opcode])?;
    kp.armed = false;
    Ok(())
}

pub fn arch_remove_kprobe<P: KprobeTextPoke>(poker: &P, kp: &mut ArchKprobe) -> Result<(), i32> {
    if kp.armed {
        arch_disarm_kprobe(poker, kp)?;
    }
    Ok(())
}

/// Linux x86 `arch_adjust_kprobe_addr()`: a probe requested at a function
/// entry starts after ENDBR so CET continues to recognize the function as a
/// valid indirect target while the probe is armed.
#[cfg(not(test))]
pub(super) fn adjust_live_kprobe_addr(addr: u64) -> Result<u64, i32> {
    let prefix = ProductionText.read(addr, super::super::alternative::ENDBR_INSN_SIZE)?;
    if prefix == super::super::alternative::ENDBR64 || prefix == super::super::alternative::ENDBR32
    {
        Ok(addr + super::super::alternative::ENDBR_INSN_SIZE as u64)
    } else {
        Ok(addr)
    }
}

const fn empty_execution_behavior() -> KprobeExecutionBehavior {
    KprobeExecutionBehavior {
        emulate_in_breakpoint: false,
        repair_call_return: false,
        repair_pushed_flags: false,
        keep_result_flags: false,
    }
}

fn classify_execution_behavior(bytes: &[u8; MAX_INSN_SIZE], len: usize) -> KprobeExecutionBehavior {
    let bytes = &bytes[..len];
    let opcode = first_opcode(bytes);
    let mut behavior = empty_execution_behavior();
    behavior.emulate_in_breakpoint =
        matches!(opcode, 0xe0..=0xe3 | 0xe9 | 0xeb | 0xfa | 0xfb) || is_jcc(bytes);
    behavior.repair_call_return = opcode == 0xe8
        || (opcode == 0xff
            && decode_instruction(bytes)
                .is_ok_and(|insn| insn.modrm.got != 0 && ((insn.modrm.value as u8 >> 3) & 7) == 2));
    behavior.repair_pushed_flags = opcode == 0x9c;
    behavior.keep_result_flags = opcode == 0x9d;
    behavior
}

fn validate_live_instruction(bytes: &[u8]) -> Result<(), i32> {
    let insn = decode_instruction(bytes)?;
    let opcode = first_opcode(bytes);
    let offset = opcode_offset(bytes);
    let second_opcode = bytes.get(offset + 1).copied();
    // Match Linux is_exception_insn(): probing an instruction which raises
    // its own INT/UD exception would attribute that exception to the displaced
    // slot and strand the active probe state.
    let exception_insn = matches!(opcode, 0xcc | 0xcd | 0xce | 0xf1)
        || (opcode == 0x0f && matches!(second_opcode, Some(0xff | 0xb9 | 0x0b)));
    // IRET and far control transfers likewise cannot use the ordinary
    // displaced-instruction completion path.  POPF may clear TF, suppressing
    // the #DB on which this implementation relies, so fail closed rather than
    // leaving a permanently active probe.
    if exception_insn || matches!(opcode, 0x9a | 0x9d | 0xcf | 0xea) {
        return Err(EOPNOTSUPP);
    }
    // Intel suppresses the single-step trap after POP SS and MOV SS.  Linux's
    // __copy_instruction() rejects both via insn_masking_exception().
    if opcode == 0x1f
        || (opcode == 0x8e && insn.modrm.got != 0 && ((insn.modrm.value as u8 >> 3) & 7) == 2)
    {
        return Err(EOPNOTSUPP);
    }
    if opcode == 0xff && insn.modrm.got != 0 {
        let modrm = insn.modrm.value as u8;
        let extension = (modrm >> 3) & 7;
        // Far CALL/JMP change CS and cannot use the ordinary displaced-slot
        // completion path. Near indirect branches, including memory and
        // RIP-relative operands, are safe: copy_instruction() relocates the
        // address and #DB observes the resolved branch target. Indirect CALL
        // return addresses are repaired by kprobe_debug_handler().
        if matches!(extension, 3 | 5) {
            return Err(EOPNOTSUPP);
        }
    }
    if opcode == 0x0f
        && bytes.get(offset + 1).copied() == Some(0x01)
        && insn.modrm.got != 0
        && ((insn.modrm.value as u8 >> 3) & 7) == 0
        && insn.modrm.value as u8 >> 6 == 3
    {
        return Err(EOPNOTSUPP);
    }
    Ok(())
}

/// Allocate and populate the executable instruction slot before the target is
/// armed. Ordinary and RIP-relative instructions use the displaced
/// single-step path. Relative branches and IF-only instructions are emulated
/// directly in the saved exception frame; stack-changing CALL/RET/PUSHF/POPF
/// instructions execute from the slot and are repaired on #DB.
#[cfg(not(test))]
pub fn prepare_live_kprobe(addr: u64) -> Result<LiveKprobe, i32> {
    let slot = crate::arch::x86::mm::init::execmem_alloc_rw(
        crate::arch::x86::mm::paging::PAGE_SIZE as usize,
    );
    if slot.is_null() {
        return Err(ENOMEM);
    }
    let result = (|| {
        let adjusted_addr = adjust_live_kprobe_addr(addr)?;
        let arch = arch_prepare_kprobe(&ProductionText, adjusted_addr, slot as u64)?;
        validate_live_instruction(&arch.copied.bytes[..arch.copied.len])?;
        unsafe {
            core::ptr::copy_nonoverlapping(arch.copied.bytes.as_ptr(), slot, arch.copied.len);
        }
        crate::arch::x86::mm::init::execmem_set_final_permissions(
            slot,
            crate::arch::x86::mm::paging::PAGE_SIZE as usize,
            false,
            true,
        )?;
        Ok(LiveKprobe {
            arch,
            slot: slot as usize,
        })
    })();
    if result.is_err() {
        crate::arch::x86::mm::init::execmem_free(slot);
    }
    result
}

#[cfg(test)]
pub fn prepare_live_kprobe(_addr: u64) -> Result<LiveKprobe, i32> {
    Err(EOPNOTSUPP)
}

pub fn arm_live_kprobe(kprobe: &mut LiveKprobe) -> Result<(), i32> {
    if kprobe.arch.armed {
        return Ok(());
    }
    arch_arm_kprobe(&ProductionText, &mut kprobe.arch)
}

pub fn disarm_live_kprobe(kprobe: &mut LiveKprobe) -> Result<(), i32> {
    if !kprobe.arch.armed {
        return Ok(());
    }
    arch_disarm_kprobe(&ProductionText, &mut kprobe.arch)
}

pub fn free_live_kprobe(mut kprobe: LiveKprobe) {
    if kprobe.arch.armed {
        let _ = disarm_live_kprobe(&mut kprobe);
    }
    #[cfg(not(test))]
    crate::arch::x86::mm::init::execmem_free(kprobe.slot as *mut u8);
}

const MAX_KPROBE_NESTING: usize = 4;
const KPROBE_STATE_SLOTS: usize = crate::kernel::sched::MAX_CPUS * MAX_KPROBE_NESTING;

static ACTIVE_DEPTH: [AtomicUsize; crate::kernel::sched::MAX_CPUS] =
    [const { AtomicUsize::new(0) }; crate::kernel::sched::MAX_CPUS];
static ACTIVE_PROBE: [AtomicU64; KPROBE_STATE_SLOTS] =
    [const { AtomicU64::new(0) }; KPROBE_STATE_SLOTS];
static ACTIVE_SLOT: [AtomicU64; KPROBE_STATE_SLOTS] =
    [const { AtomicU64::new(0) }; KPROBE_STATE_SLOTS];
static ACTIVE_LEN: [AtomicUsize; KPROBE_STATE_SLOTS] =
    [const { AtomicUsize::new(0) }; KPROBE_STATE_SLOTS];
static ACTIVE_IF: [AtomicU64; KPROBE_STATE_SLOTS] =
    [const { AtomicU64::new(0) }; KPROBE_STATE_SLOTS];
static ACTIVE_FLAGS: [AtomicU64; KPROBE_STATE_SLOTS] =
    [const { AtomicU64::new(0) }; KPROBE_STATE_SLOTS];
static ACTIVE_BEHAVIOR: [AtomicU64; KPROBE_STATE_SLOTS] =
    [const { AtomicU64::new(0) }; KPROBE_STATE_SLOTS];
static KPROBE_HANDLERS_IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);

const BEHAVIOR_REPAIR_CALL_RETURN: u64 = 1 << 0;
const BEHAVIOR_REPAIR_PUSHED_FLAGS: u64 = 1 << 1;
const BEHAVIOR_KEEP_RESULT_FLAGS: u64 = 1 << 2;

fn encode_execution_behavior(behavior: KprobeExecutionBehavior) -> u64 {
    (behavior.repair_call_return as u64) * BEHAVIOR_REPAIR_CALL_RETURN
        | (behavior.repair_pushed_flags as u64) * BEHAVIOR_REPAIR_PUSHED_FLAGS
        | (behavior.keep_result_flags as u64) * BEHAVIOR_KEEP_RESULT_FLAGS
}

fn active_cpu() -> usize {
    (crate::kernel::sched::current_cpu() as usize).min(crate::kernel::sched::MAX_CPUS - 1)
}

const fn active_slot(cpu: usize, depth: usize) -> usize {
    cpu * MAX_KPROBE_NESTING + depth
}

pub fn kprobe_active(address: u64) -> bool {
    ACTIVE_PROBE
        .iter()
        .any(|active| active.load(Ordering::Acquire) == address)
}

pub fn kprobe_handlers_active() -> bool {
    KPROBE_HANDLERS_IN_FLIGHT.load(Ordering::Acquire) != 0
}

pub fn emulate_instruction(bytes: &[u8], regs: &mut EmulatedRegs) -> Result<(), i32> {
    let insn = decode_instruction(bytes)?;
    let op = first_opcode(bytes);
    let len = match op {
        0xe8 | 0xe9 => 5,
        0xeb | 0x70..=0x7f | 0xe0..=0xe3 => 2,
        0xc2 => 3,
        0x0f if bytes.get(1).is_some_and(|b| (0x80..=0x8f).contains(b)) => 6,
        _ => insn.length as u64,
    };
    let next = regs.ip.wrapping_add(len);
    match op {
        0xe8 => {
            let rel = read_i32(bytes, 1)? as i64;
            regs.stack.push(next);
            regs.sp = regs.sp.wrapping_sub(8);
            regs.ip = (next as i64 + rel) as u64;
        }
        0xe9 => regs.ip = (next as i64 + read_i32(bytes, 1)? as i64) as u64,
        0xeb => regs.ip = (next as i64 + read_i8(bytes, 1)? as i64) as u64,
        0xc3 => {
            regs.ip = regs.stack.pop().ok_or(EFAULT)?;
            regs.sp = regs.sp.wrapping_add(8);
        }
        0xc2 => {
            regs.ip = regs.stack.pop().ok_or(EFAULT)?;
            let adj = read_u16(bytes, 1)? as u64;
            regs.sp = regs.sp.wrapping_add(8 + adj);
        }
        0xfa => {
            regs.flags &= !X86_EFLAGS_IF;
            regs.ip = next;
        }
        0xfb => {
            regs.flags |= X86_EFLAGS_IF;
            regs.ip = next;
        }
        0x9d => {
            regs.flags = regs.stack.pop().ok_or(EFAULT)?;
            regs.sp = regs.sp.wrapping_add(8);
            regs.ip = next;
        }
        0xe0..=0xe2 => {
            regs.cx = regs.cx.wrapping_sub(1);
            regs.ip = if regs.cx != 0 {
                (next as i64 + read_i8(bytes, 1)? as i64) as u64
            } else {
                next
            };
        }
        0xe3 => {
            regs.ip = if regs.cx == 0 {
                (next as i64 + read_i8(bytes, 1)? as i64) as u64
            } else {
                next
            };
        }
        op if (0x70..=0x7f).contains(&op) => {
            regs.ip = if jcc_taken(op & 0x0f, regs.flags) {
                (next as i64 + read_i8(bytes, 1)? as i64) as u64
            } else {
                next
            };
        }
        0x0f if bytes.get(1).is_some_and(|b| (0x80..=0x8f).contains(b)) => {
            regs.ip = if jcc_taken(bytes[1] & 0x0f, regs.flags) {
                (next as i64 + read_i32(bytes, 2)? as i64) as u64
            } else {
                next
            };
        }
        _ => regs.ip = next,
    }
    Ok(())
}

fn emulate_breakpoint_instruction(
    execution: &KprobeExecution,
    frame: &ExceptionFrame,
) -> Result<(), i32> {
    let bytes = &execution.bytes[..execution.instruction_len];
    let offset = opcode_offset(bytes);
    let opcode = *bytes.get(offset).ok_or(EFAULT)?;
    let next = execution
        .original_ip
        .wrapping_add(execution.instruction_len as u64);
    let mut target = next;
    let mut cx = frame.rcx;
    let mut flags = frame.rflags;

    match opcode {
        0xe9 => target = next.wrapping_add(read_i32(bytes, bytes.len() - 4)? as i64 as u64),
        0xeb => target = next.wrapping_add(read_i8(bytes, bytes.len() - 1)? as i64 as u64),
        0x70..=0x7f => {
            if jcc_taken(opcode & 0x0f, flags) {
                target = next.wrapping_add(read_i8(bytes, bytes.len() - 1)? as i64 as u64);
            }
        }
        0x0f if bytes
            .get(offset + 1)
            .is_some_and(|opcode| (0x80..=0x8f).contains(opcode)) =>
        {
            if jcc_taken(bytes[offset + 1] & 0x0f, flags) {
                target = next.wrapping_add(read_i32(bytes, bytes.len() - 4)? as i64 as u64);
            }
        }
        0xe0..=0xe2 => {
            let address32 = bytes[..offset].contains(&0x67);
            if address32 {
                let next_cx = (cx as u32).wrapping_sub(1);
                cx = next_cx as u64;
            } else {
                cx = cx.wrapping_sub(1);
            }
            let nonzero = if address32 { cx as u32 != 0 } else { cx != 0 };
            let condition = match opcode {
                0xe0 => nonzero && flags & (1 << 6) == 0,
                0xe1 => nonzero && flags & (1 << 6) != 0,
                _ => nonzero,
            };
            if condition {
                target = next.wrapping_add(read_i8(bytes, bytes.len() - 1)? as i64 as u64);
            }
        }
        0xe3 => {
            let address32 = bytes[..offset].contains(&0x67);
            let zero = if address32 { cx as u32 == 0 } else { cx == 0 };
            if zero {
                target = next.wrapping_add(read_i8(bytes, bytes.len() - 1)? as i64 as u64);
            }
        }
        0xfa => flags &= !X86_EFLAGS_IF,
        0xfb => flags |= X86_EFLAGS_IF,
        _ => return Err(EINVAL),
    }

    unsafe {
        let frame = frame as *const ExceptionFrame as *mut ExceptionFrame;
        (*frame).rip = target;
        (*frame).rcx = cx;
        (*frame).rflags = flags;
    }
    Ok(())
}

fn kernel_exception_stack_pointer(frame: &ExceptionFrame) -> Result<usize, i32> {
    if frame.cs & 3 != 0 {
        return Err(EINVAL);
    }
    Ok(core::ptr::addr_of!(frame.user_rsp) as usize)
}

fn write_stack_word(address: usize, value: u64) -> Result<(), i32> {
    unsafe {
        crate::arch::x86::mm::maccess::copy_to_kernel_nofault(
            address as *mut u8,
            core::ptr::addr_of!(value).cast(),
            core::mem::size_of::<u64>(),
        )
    }
}

pub fn kprobe_int3_handler(frame: &ExceptionFrame) -> bool {
    let probe_ip = frame.rip.wrapping_sub(INT3_INSN_SIZE as u64);
    KPROBE_HANDLERS_IN_FLIGHT.fetch_add(1, Ordering::AcqRel);
    let cpu = active_cpu();
    let depth = ACTIVE_DEPTH[cpu].load(Ordering::Acquire);
    if depth >= MAX_KPROBE_NESTING
        || ACTIVE_DEPTH[cpu]
            .compare_exchange(depth, depth + 1, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
    {
        KPROBE_HANDLERS_IN_FLIGHT.fetch_sub(1, Ordering::AcqRel);
        return false;
    }
    let active = active_slot(cpu, depth);
    ACTIVE_PROBE[active].store(probe_ip, Ordering::Release);
    let Some(execution) = crate::kernel::trace::kprobe::begin_kprobe(probe_ip) else {
        ACTIVE_PROBE[active].store(0, Ordering::Release);
        ACTIVE_DEPTH[cpu].store(depth, Ordering::Release);
        KPROBE_HANDLERS_IN_FLIGHT.fetch_sub(1, Ordering::AcqRel);
        return false;
    };
    if execution.behavior.emulate_in_breakpoint {
        let handled = emulate_breakpoint_instruction(&execution, frame).is_ok();
        crate::kernel::trace::kprobe::finish_kprobe(probe_ip);
        ACTIVE_PROBE[active].store(0, Ordering::Release);
        ACTIVE_DEPTH[cpu].store(depth, Ordering::Release);
        KPROBE_HANDLERS_IN_FLIGHT.fetch_sub(1, Ordering::AcqRel);
        return handled;
    }
    ACTIVE_SLOT[active].store(execution.slot_ip, Ordering::Release);
    ACTIVE_LEN[active].store(execution.instruction_len, Ordering::Release);
    ACTIVE_IF[active].store(frame.rflags & X86_EFLAGS_IF, Ordering::Release);
    ACTIVE_FLAGS[active].store(frame.rflags, Ordering::Release);
    ACTIVE_BEHAVIOR[active].store(
        encode_execution_behavior(execution.behavior),
        Ordering::Release,
    );
    unsafe {
        let frame_mut = frame as *const ExceptionFrame as *mut ExceptionFrame;
        (*frame_mut).rip = execution.slot_ip;
        // Single-step exactly one displaced instruction. Interrupts stay off
        // for that instruction so a nested probe cannot overwrite per-CPU
        // completion state.
        (*frame_mut).rflags = ((*frame_mut).rflags | (1 << 8)) & !X86_EFLAGS_IF;
    }
    true
}

/// Complete the displaced instruction on #DB and translate its slot-relative
/// fallthrough back to the original instruction stream.
pub fn kprobe_debug_handler(frame: &ExceptionFrame) -> bool {
    let cpu = active_cpu();
    let depth = ACTIVE_DEPTH[cpu].load(Ordering::Acquire);
    if depth == 0 || depth > MAX_KPROBE_NESTING {
        return false;
    }
    let active = active_slot(cpu, depth - 1);
    let original = ACTIVE_PROBE[active].load(Ordering::Acquire);
    if original == 0 {
        return false;
    }
    let slot = ACTIVE_SLOT[active].load(Ordering::Acquire);
    let len = ACTIVE_LEN[active].load(Ordering::Acquire);
    let behavior = ACTIVE_BEHAVIOR[active].load(Ordering::Acquire);
    let expected = slot.wrapping_add(len as u64);
    if behavior & BEHAVIOR_REPAIR_CALL_RETURN != 0 {
        let result = kernel_exception_stack_pointer(frame)
            .and_then(|sp| write_stack_word(sp, original.wrapping_add(len as u64)));
        if result.is_err() {
            return false;
        }
    }
    if behavior & BEHAVIOR_REPAIR_PUSHED_FLAGS != 0 {
        let result = kernel_exception_stack_pointer(frame)
            .and_then(|sp| write_stack_word(sp, ACTIVE_FLAGS[active].load(Ordering::Acquire)));
        if result.is_err() {
            return false;
        }
    }
    unsafe {
        let frame_mut = frame as *const ExceptionFrame as *mut ExceptionFrame;
        if behavior & BEHAVIOR_KEEP_RESULT_FLAGS == 0 {
            (*frame_mut).rflags &= !(1 << 8);
            (*frame_mut).rflags =
                ((*frame_mut).rflags & !X86_EFLAGS_IF) | ACTIVE_IF[active].load(Ordering::Acquire);
        }
        if (*frame_mut).rip == expected {
            (*frame_mut).rip = original.wrapping_add(len as u64);
        }
    }
    crate::kernel::trace::kprobe::finish_kprobe(original);
    ACTIVE_LEN[active].store(0, Ordering::Release);
    ACTIVE_SLOT[active].store(0, Ordering::Release);
    ACTIVE_IF[active].store(0, Ordering::Release);
    ACTIVE_FLAGS[active].store(0, Ordering::Release);
    ACTIVE_BEHAVIOR[active].store(0, Ordering::Release);
    ACTIVE_PROBE[active].store(0, Ordering::Release);
    ACTIVE_DEPTH[cpu].store(depth - 1, Ordering::Release);
    KPROBE_HANDLERS_IN_FLIGHT.fetch_sub(1, Ordering::AcqRel);
    true
}

pub const fn arch_populate_kprobe_blacklist() -> &'static [&'static str] {
    &["__switch_to", "do_int3", "kprobe_int3_handler"]
}

pub const fn arch_init_kprobes() -> Result<(), i32> {
    Ok(())
}

fn read_i8(bytes: &[u8], off: usize) -> Result<i8, i32> {
    bytes.get(off).copied().map(|b| b as i8).ok_or(EFAULT)
}

fn read_i32(bytes: &[u8], off: usize) -> Result<i32, i32> {
    if off + 4 > bytes.len() {
        return Err(EFAULT);
    }
    Ok(i32::from_le_bytes(bytes[off..off + 4].try_into().unwrap()))
}

fn read_u16(bytes: &[u8], off: usize) -> Result<u16, i32> {
    if off + 2 > bytes.len() {
        return Err(EFAULT);
    }
    Ok(u16::from_le_bytes(bytes[off..off + 2].try_into().unwrap()))
}

fn jcc_taken(cc: u8, flags: u64) -> bool {
    const CF: u64 = 1 << 0;
    const ZF: u64 = 1 << 6;
    const SF: u64 = 1 << 7;
    const OF: u64 = 1 << 11;
    const PF: u64 = 1 << 2;
    match cc {
        0 => flags & OF != 0,
        1 => flags & OF == 0,
        2 => flags & CF != 0,
        3 => flags & CF == 0,
        4 => flags & ZF != 0,
        5 => flags & ZF == 0,
        6 => flags & (CF | ZF) != 0,
        7 => flags & (CF | ZF) == 0,
        8 => flags & SF != 0,
        9 => flags & SF == 0,
        0xa => flags & PF != 0,
        0xb => flags & PF == 0,
        0xc => (flags & SF != 0) != (flags & OF != 0),
        0xd => (flags & SF != 0) == (flags & OF != 0),
        0xe => (flags & ZF != 0) || ((flags & SF != 0) != (flags & OF != 0)),
        0xf => (flags & ZF == 0) && ((flags & SF != 0) == (flags & OF != 0)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeMap;
    use core::cell::RefCell;

    #[derive(Default)]
    struct Mem {
        bytes: RefCell<BTreeMap<u64, u8>>,
    }

    impl Mem {
        fn seed(&self, ip: u64, bytes: &[u8]) {
            let mut m = self.bytes.borrow_mut();
            for (i, b) in bytes.iter().enumerate() {
                m.insert(ip + i as u64, *b);
            }
        }
    }

    impl KernelText for Mem {
        fn read(&self, ip: u64, len: usize) -> Result<Vec<u8>, i32> {
            let m = self.bytes.borrow();
            Ok((0..len)
                .map(|i| *m.get(&(ip + i as u64)).unwrap_or(&0x90))
                .collect())
        }
    }

    impl KprobeTextPoke for Mem {
        fn poke(&self, ip: u64, bytes: &[u8]) -> Result<(), i32> {
            self.seed(ip, bytes);
            Ok(())
        }
    }

    #[test]
    fn reljump_and_relcall_encode_rel32() {
        let j = synthesize_reljump(0x1000, 0x2000);
        assert_eq!(j[0], JMP32_INSN_OPCODE);
        assert_eq!(i32::from_le_bytes(j[1..5].try_into().unwrap()), 0xffb);
        let c = synthesize_relcall(0x1000, 0x0ff0);
        assert_eq!(c[0], CALL_INSN_OPCODE);
        assert_eq!(i32::from_le_bytes(c[1..5].try_into().unwrap()), -0x15);
    }

    #[test]
    fn rip_relative_copy_rewrites_displacement_for_slot() {
        let copied =
            copy_instruction(0x1000, 0x2000, &[0x48, 0x8b, 0x05, 0x34, 0x12, 0, 0]).expect("copy");
        assert_eq!(copied.len, 7);
        let new_disp = i32::from_le_bytes(copied.bytes[3..7].try_into().unwrap());
        assert_eq!(new_disp, 0x234);
    }

    #[test]
    fn relative_call_copy_accepts_canonical_alias_wrap() {
        let original = 0xffff_ffff_c100_0000;
        let slot = 0xffff_ffff_c100_1000;
        let target = 0x0000_0000_0050_0000;
        let original_disp = encode_rel32(original + 5, target).unwrap();
        let mut instruction = [RELATIVECALL_OPCODE, 0, 0, 0, 0];
        instruction[1..].copy_from_slice(&original_disp.to_le_bytes());

        let copied = copy_instruction(original, slot, &instruction).unwrap();
        let copied_disp = i32::from_le_bytes(copied.bytes[1..5].try_into().unwrap());
        assert_eq!(
            slot.wrapping_add(5).wrapping_add_signed(copied_disp as i64),
            target
        );
    }

    #[test]
    fn boost_rejects_control_flow_and_if_modifiers() {
        assert!(can_boost(&[0x90]));
        assert!(!can_boost(&[0xe8, 0, 0, 0, 0]));
        assert!(!can_boost(&[0xeb, 0]));
        assert!(!can_boost(&[0xc3]));
        assert!(!can_boost(&[0xfa]));
    }

    #[test]
    fn emulate_call_ret_jcc_loop_and_if() {
        let mut regs = EmulatedRegs::new(0x1000);
        regs.sp = 0x8000;
        emulate_instruction(&[0xe8, 0x05, 0, 0, 0], &mut regs).unwrap();
        assert_eq!(regs.ip, 0x100a);
        assert_eq!(regs.stack.last().copied(), Some(0x1005));
        emulate_instruction(&[0xc3], &mut regs).unwrap();
        assert_eq!(regs.ip, 0x1005);

        regs.ip = 0x2000;
        regs.flags = 1 << 6;
        emulate_instruction(&[0x74, 0x7e], &mut regs).unwrap();
        assert_eq!(regs.ip, 0x2080);

        regs.ip = 0x3000;
        regs.cx = 2;
        emulate_instruction(&[0xe2, 0xfc], &mut regs).unwrap();
        assert_eq!(regs.cx, 1);
        assert_eq!(regs.ip, 0x2ffe);

        emulate_instruction(&[0xfa], &mut regs).unwrap();
        assert_eq!(regs.flags & X86_EFLAGS_IF, 0);
        emulate_instruction(&[0xfb], &mut regs).unwrap();
        assert_ne!(regs.flags & X86_EFLAGS_IF, 0);
    }

    #[test]
    fn arm_and_disarm_patch_first_byte() {
        let mem = Mem::default();
        mem.seed(0x1000, &[0x90; MAX_INSN_SIZE]);
        let mut kp = arch_prepare_kprobe(&mem, 0x1000, 0x2000).unwrap();
        arch_arm_kprobe(&mem, &mut kp).unwrap();
        assert!(kp.armed);
        assert_eq!(mem.read(0x1000, 1).unwrap(), &[INT3_INSN_OPCODE]);
        arch_disarm_kprobe(&mem, &mut kp).unwrap();
        assert!(!kp.armed);
        assert_eq!(mem.read(0x1000, 1).unwrap(), &[0x90]);
    }

    #[test]
    fn near_indirect_memory_branches_are_probeable() {
        assert_eq!(validate_live_instruction(&[0xff, 0x10]), Ok(())); // call *(%rax)
        assert_eq!(validate_live_instruction(&[0xff, 0x20]), Ok(())); // jmp *(%rax)
        assert_eq!(validate_live_instruction(&[0xff, 0x18]), Err(EOPNOTSUPP)); // lcall
        assert_eq!(validate_live_instruction(&[0xff, 0x28]), Err(EOPNOTSUPP)); // ljmp
    }

    #[test]
    fn exception_and_single_step_masking_instructions_are_rejected() {
        for instruction in [
            &[0xcc][..],
            &[0xcd, 0x80],
            &[0xce],
            &[0xf1],
            &[0x0f, 0x0b],
            &[0x0f, 0xb9, 0xc0],
            &[0x0f, 0xff, 0xc0],
            &[0x9d],
            &[0x1f],
            &[0x8e, 0xd0],
        ] {
            assert_eq!(validate_live_instruction(instruction), Err(EOPNOTSUPP));
        }
    }
}
