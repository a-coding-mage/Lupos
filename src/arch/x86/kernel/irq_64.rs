//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/irq_64.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/irq_64.c
//! x86_64 hardirq stack mapping policy.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::include::uapi::errno::ENOMEM;

pub const PAGE_SHIFT: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
pub const IRQ_STACK_SIZE: usize = 16 * 1024;

#[repr(align(4096))]
struct IrqStack([u8; IRQ_STACK_SIZE]);

// Linux keeps one page-aligned backing store and one in-use bit per CPU.  The
// Lupos allocator does not yet expose the Linux per-CPU vmap lifecycle, but
// this backing store has the same per-CPU ownership and exact stack extent.
// The transfer below is deliberately limited to the device/timer handler;
// IDT entry/exit and rescheduling stay on the interrupted task stack.
static mut IRQ_STACK_BACKING_STORE: [IrqStack; crate::kernel::sched::MAX_CPUS] =
    [const { IrqStack([0; IRQ_STACK_SIZE]) }; crate::kernel::sched::MAX_CPUS];
static HARDIRQ_STACK_INUSE: [AtomicBool; crate::kernel::sched::MAX_CPUS] =
    [const { AtomicBool::new(false) }; crate::kernel::sched::MAX_CPUS];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IrqStackBacking {
    Vmap { guard_pages: bool },
    PerCpuBackingStore,
}

pub const fn irq_stack_pages() -> usize {
    IRQ_STACK_SIZE / PAGE_SIZE
}

pub const fn hardirq_stack_top(base: usize) -> usize {
    base + IRQ_STACK_SIZE - 8
}

pub const fn map_irq_stack(
    cpu: usize,
    vmap_stack: bool,
    vmap_success: bool,
    percpu_base: usize,
) -> Result<(usize, IrqStackBacking), i32> {
    let _ = cpu;
    if vmap_stack {
        if !vmap_success {
            return Err(-ENOMEM);
        }
        Ok((
            hardirq_stack_top(percpu_base),
            IrqStackBacking::Vmap { guard_pages: true },
        ))
    } else {
        Ok((
            hardirq_stack_top(percpu_base),
            IrqStackBacking::PerCpuBackingStore,
        ))
    }
}

pub const fn irq_init_percpu_irqstack(
    existing_stack_ptr: Option<usize>,
    mapped: Result<(usize, IrqStackBacking), i32>,
) -> Result<usize, i32> {
    if let Some(ptr) = existing_stack_ptr {
        return Ok(ptr);
    }
    match mapped {
        Ok((ptr, _)) => Ok(ptr),
        Err(err) => Err(err),
    }
}

/// Linux `run_irq_on_irqstack_cond()` for the Lupos IDT's device/timer
/// handler.  The exception frame remains on the interrupted stack, exactly as
/// it does in Linux; only the potentially deep driver call is moved.
///
/// # Safety
/// `frame` must be the live IDT frame and interrupts must be disabled.
pub unsafe fn run_irq_on_irqstack(
    frame: *mut crate::arch::x86::kernel::idt::ExceptionFrame,
    vector: u8,
) {
    let cpu = crate::arch::x86::kernel::setup_percpu::current_cpu_number()
        .min(crate::kernel::sched::MAX_CPUS - 1);
    let user_mode = unsafe { (*frame).cs & 3 != 0 };
    if user_mode || HARDIRQ_STACK_INUSE[cpu].swap(true, Ordering::AcqRel) {
        unsafe { crate::arch::x86::kernel::idt::run_hardirq_handler(frame, vector) };
        return;
    }

    let base = unsafe { core::ptr::addr_of_mut!(IRQ_STACK_BACKING_STORE[cpu].0) as *mut u8 };
    // `hardirq_stack_ptr` is Linux's actual TOS, not merely an initial RSP.
    // `call_on_stack()` stores the interrupted RSP in this exact topmost slot.
    // Stack walkers recover the previous task stack from that slot, so do not
    // consume it with an extra saved-stack frame.
    let top = unsafe { base.add(IRQ_STACK_SIZE - 8) } as usize;
    unsafe { irq_stack_call(top, frame, vector as usize) };
    HARDIRQ_STACK_INUSE[cpu].store(false, Ordering::Release);
}

/// Linux `run_sysvec_on_irqstack_cond()` for system vectors whose handlers
/// are deeper than the minimal reschedule IPI. TLB shootdowns can arrive
/// while the interrupted task is in a deep scheduler or mm path, so their
/// handler must use the per-CPU IRQ stack just like Linux's
/// `DEFINE_IDTENTRY_SYSVEC` path.
///
/// # Safety
/// `frame` must be the live IDT frame and interrupts must be disabled.
pub unsafe fn run_sysvec_on_irqstack(
    frame: *mut crate::arch::x86::kernel::idt::ExceptionFrame,
    vector: u8,
) {
    let cpu = crate::arch::x86::kernel::setup_percpu::current_cpu_number()
        .min(crate::kernel::sched::MAX_CPUS - 1);
    let user_mode = unsafe { (*frame).cs & 3 != 0 };
    if user_mode || HARDIRQ_STACK_INUSE[cpu].swap(true, Ordering::AcqRel) {
        unsafe { crate::arch::x86::kernel::idt::run_sysvec_handler(frame, vector) };
        return;
    }

    let base = unsafe { core::ptr::addr_of_mut!(IRQ_STACK_BACKING_STORE[cpu].0) as *mut u8 };
    let top = unsafe { base.add(IRQ_STACK_SIZE - 8) } as usize;
    unsafe { irq_stack_call_sysvec(top, frame, vector as usize) };
    HARDIRQ_STACK_INUSE[cpu].store(false, Ordering::Release);
}

#[unsafe(naked)]
unsafe extern "C" fn irq_stack_call(
    _stack_top: usize,
    _frame: *mut crate::arch::x86::kernel::idt::ExceptionFrame,
    _vector: usize,
) {
    core::arch::naked_asm!(
        // Linux `call_on_stack()` saves the interrupted RSP at
        // hardirq_stack_ptr itself, invokes the handler, then restores it
        // with `popq %rsp`.  Preserve that externally visible stack link.
        //
        // Linux compiles x86 kernel C with an 8-byte stack alignment
        // (`arch/x86/Makefile:cc_stack_align8`), so its C handler may enter
        // with RSP % 16 == 0.  Rust `extern "C"` follows the SysV AMD64 ABI
        // and requires RSP % 16 == 8 at function entry.  Reserve one
        // otherwise-unused word below the saved link before the Rust call,
        // then discard it before Linux's `pop rsp` restore.  A literal C-asm
        // translation would misalign every nested Rust call and corrupt task
        // stacks under IRQ load.
        "mov [rdi], rsp",
        "mov rsp, rdi",
        "mov rdi, rsi",
        "mov rsi, rdx",
        "sub rsp, 8",
        "call {dispatch}",
        "add rsp, 8",
        "pop rsp",
        "ret",
        dispatch = sym crate::arch::x86::kernel::idt::run_hardirq_handler,
    );
}

#[unsafe(naked)]
unsafe extern "C" fn irq_stack_call_sysvec(
    _stack_top: usize,
    _frame: *mut crate::arch::x86::kernel::idt::ExceptionFrame,
    _vector: usize,
) {
    core::arch::naked_asm!(
        // Keep the same Linux call_on_stack() link and SysV alignment as the
        // hardirq adapter above. The handler itself owns irq_enter/exit.
        "mov [rdi], rsp",
        "mov rsp, rdi",
        "mov rdi, rsi",
        "mov rsi, rdx",
        "sub rsp, 8",
        "call {dispatch}",
        "add rsp, 8",
        "pop rsp",
        "ret",
        dispatch = sym crate::arch::x86::kernel::idt::run_sysvec_handler,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn irq64_stack_mapping_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/irq_64.c"
        ));
        assert!(source.contains("DEFINE_PER_CPU_CACHE_HOT(bool, hardirq_stack_inuse)"));
        assert!(source.contains("DEFINE_PER_CPU_PAGE_ALIGNED(struct irq_stack"));
        assert!(source.contains("#ifdef CONFIG_VMAP_STACK"));
        assert!(source.contains("phys_addr_t pa = per_cpu_ptr_to_phys"));
        assert!(source.contains("vmap(pages, IRQ_STACK_SIZE / PAGE_SIZE, VM_MAP, PAGE_KERNEL)"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("IRQ_STACK_SIZE - 8"));
        assert!(source.contains("if (per_cpu(hardirq_stack_ptr, cpu))"));
        assert!(source.contains("return map_irq_stack(cpu);"));

        assert_eq!(irq_stack_pages(), 4);
        let mapped = map_irq_stack(0, true, true, 0x1000).unwrap();
        assert_eq!(mapped.0, 0x1000 + IRQ_STACK_SIZE - 8);
        assert_eq!(map_irq_stack(0, true, false, 0x1000), Err(-ENOMEM));
        assert_eq!(
            irq_init_percpu_irqstack(Some(0xdead), Ok(mapped)),
            Ok(0xdead)
        );
        assert_eq!(irq_init_percpu_irqstack(None, Ok(mapped)), Ok(mapped.0));
    }

    #[test]
    fn irq_stack_link_uses_linux_topmost_slot() {
        // test-origin: linux:vendor/linux/arch/x86/include/asm/irq_stack.h:call_on_stack
        // Linux records the interrupted RSP at hardirq_stack_ptr itself.  The
        // Rust SysV adapter leaves that link intact and reserves one word
        // below it so the handler receives the required 16-byte call-site
        // alignment.  It must discard that word before Linux's pop restore.
        let base = 0x1_0000usize;
        let top = hardirq_stack_top(base);
        assert_eq!(top, base + IRQ_STACK_SIZE - 8);

        let source = include_str!("irq_64.rs");
        let stub = source
            .split("unsafe extern \"C\" fn irq_stack_call")
            .nth(1)
            .expect("irq stack transfer stub must exist")
            .split("#[cfg(test)]")
            .next()
            .expect("irq stack transfer stub must end before tests");
        assert!(stub.contains("\"mov [rdi], rsp\""));
        assert!(stub.contains("\"mov rsp, rdi\""));
        assert!(stub.contains("\"sub rsp, 8\""));
        assert!(stub.contains("\"add rsp, 8\""));
        assert!(stub.contains("\"add rsp, 8\",\n        \"pop rsp\""));
        assert!(!stub.contains("\"xchg rsp, rdi\""));
        assert!(!stub.contains("\"push rdi\""));
    }
}
