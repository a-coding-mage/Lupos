; arch/x86/realmode/trampoline.S — AP (Application Processor) startup trampoline
;
; This flat binary is copied to physical address 0x8000 by the BSP before
; sending SIPI.  The SIPI vector byte (0x08) tells each AP to begin execution
; at 0x08 << 12 = 0x8000 in 16-bit real mode.
;
; Flow:
;   16-bit real mode  →  32-bit protected mode  →  64-bit long mode
;   → call ap_main (Rust, in kernel image above 1 MiB)
;
; The BSP writes several values into the "data area" at fixed offsets within
; this page before sending the SIPI.  Those offsets must match the OFF_*
; constants defined in smp.rs.
;
; References:
;   Intel SDM Vol. 3A §10.6.7 "Broadcast/Self-Directed IPIs"
;   Intel SDM Vol. 3A §9.9.3  "MP Initialization Example"
;   https://wiki.osdev.org/Symmetric_Multiprocessing#AP_startup
;   https://wiki.osdev.org/Setting_Up_Long_Mode

bits 16
org  0x8000

; ── 16-bit real-mode entry ────────────────────────────────────────────────────
;
; The CPU starts here in real mode.  CS = 0x0800 (SIPI pushed far pointer
; 0x0800:0x0000), EIP = 0x0000.  Physical address = CS*16 + EIP = 0x8000.
;
; Note: NASM's org makes all label addresses relative to 0x8000, which is the
; physical address we actually execute from.

_ap_start:
    cli                         ; disable interrupts until we have an IDT
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    xor sp, sp                  ; temporary 16-bit stack at 0x0000 (grows down from 0)

    ; DEBUG: write '1' to serial port to prove 16-bit entry
    mov dx, 0x3F8
    mov al, '1'
    out dx, al

    ; ── Load a minimal GDT embedded in this page ───────────────────────────
    ; We cannot use the BSP's full GDT yet (it requires 64-bit lgdt operand).
    ; Instead, load a local 5-entry GDT sufficient for the 16→32→64 transition.
    ;
    ; The descriptor is in the rodata area below the data area (see label below).
    lgdt [ap_local_gdt_ptr]     ; 6-byte descriptor: limit(u16) + base(u32)

    ; ── Enter 32-bit protected mode ────────────────────────────────────────
    mov eax, cr0
    or  eax, 1                  ; set CR0.PE (Protection Enable)
    mov cr0, eax

    ; Far jump to 32-bit code segment (selector 0x08 = second GDT entry).
    ; This flushes the prefetch queue and loads CS with the 32-bit descriptor.
    jmp dword 0x08:ap_32bit     ; 32-bit far jump using local GDT

; ── 32-bit protected mode ─────────────────────────────────────────────────────
;
; CS = 0x08 (32-bit code), running at physical address 0x8000+.
; We can now use 32-bit instructions and address the full 4 GiB.

bits 32
ap_32bit:
    ; DEBUG: write '2' to serial port to prove 32-bit entry
    mov dx, 0x3F8
    mov al, '2'
    out dx, al

    mov ax, 0x10                ; data segment selector (32-bit data, entry 2)
    mov ds, ax
    mov es, ax
    mov ss, ax

    ; ── Load the BSP's pre-paging CR4 state ────────────────────────────────
    ; PAE is required for long mode, but APs also need to inherit BSP CR4
    ; feature bits that affect page-table interpretation.  The BSP writes a
    ; sanitized copy here with PCIDE clear, because PCIDE can only be enabled
    ; after paging and long mode are active.
    ; Reference: Intel SDM Vol. 3A §4.1 "Paging Modes and Control Bits"
    mov eax, [0x8000 + 0x238]   ; bsp_cr4_pre_paging (low u32) at offset 0x238
    or  eax, (1 << 5)           ; force CR4.PAE = bit 5
    mov cr4, eax

    ; ── Load BSP's PML4 into CR3 ──────────────────────────────────────────
    ; The BSP writes its current CR3 value to ap_pml4_addr (at offset 0x200).
    ; Since the BSP has already set up a 4 GiB identity map, the AP can share
    ; these page tables — both physical and virtual addresses are identical.
    ; Reference: Intel SDM Vol. 3A §4.5 "4-Level Paging"
    mov eax, [0x8000 + 0x200]   ; ap_pml4_addr (u32) at offset 0x200 in this page
    mov cr3, eax

    ; ── Enable Long Mode with BSP EFER state ──────────────────────────────
    ; Write IA32_EFER MSR (0xC000_0080).  The BSP-provided value preserves
    ; writable bits such as SCE/NXE and has LME set, with read-only LMA clear.
    ; Reference: Intel SDM Vol. 3A §2.2.1 "IA32_EFER MSR"
    mov ecx, 0xC0000080
    mov eax, [0x8000 + 0x248]   ; bsp_efer low u32 at offset 0x248
    mov edx, [0x8000 + 0x24C]   ; bsp_efer high u32
    wrmsr

    ; ── Enable paging (activates long mode) ───────────────────────────────
    ; Setting CR0.PG while EFER.LME=1 and CR4.PAE=1 transitions to long mode.
    ; CR0.PE must also be set (already is from real mode transition above).
    ; Preserve the BSP CR0 state so APs match the boot CPU's paging semantics.
    ; Reference: Intel SDM Vol. 3A §9.9.4 "Switching to Long Mode"
    mov eax, [0x8000 + 0x230]   ; bsp_cr0 low u32 at offset 0x230
    or  eax, (1 << 31) | 1      ; force PG | PE
    mov cr0, eax

    ; ── Far jump to 64-bit code segment ───────────────────────────────────
    ; Selector 0x18 = entry 3 in the local GDT (64-bit code, L=1).
    ; This flushes the pipeline and switches the decoder to 64-bit mode.
    jmp dword 0x18:ap_64bit

; ── 64-bit long mode ──────────────────────────────────────────────────────────
;
; CS = 0x18 (64-bit code from local GDT), running with BSP's page tables.
; We now have full 64-bit addressing and can reach the kernel image above 1 MiB.

bits 64
DEFAULT ABS             ; explicit absolute addressing (suppress NASM deprecation warning)
ap_64bit:
    ; Restore the full BSP CR4 now that paging and long mode are active.  This
    ; includes bits that were unsafe before paging, such as PCIDE if present.
    mov rax, [0x8000 + 0x240]   ; bsp_cr4_long (u64) at offset 0x240
    mov cr4, rax

    ; DEBUG: write '3' to serial port to prove 64-bit entry
    mov dx, 0x3F8
    mov al, '3'
    out dx, al

    mov ax, 0x20                ; 64-bit data segment from local GDT (entry 4)
    mov ds, ax
    mov es, ax
    mov ss, ax
    xor ax, ax
    mov fs, ax
    mov gs, ax

    ; ── Load the BSP's GDT (full kernel GDT with TSS descriptor) ──────────
    ; The 64-bit lgdt instruction reads a 10-byte descriptor: u16 limit + u64 base.
    ; The BSP wrote its GDTR value to ap_bsp_gdt_desc (at offset 0x210).
    ; After this, the kernel's real GDT segments are active.
    ;
    ; We reload CS after loading the GDT via a 64-bit far return (retfq).
    ; The new CS must be the BSP's Linux KERNEL_CS = 0x10.
    ; Reference: Intel SDM Vol. 3A §9.9.4 "Switching to Long Mode"
    ; DEBUG: 'a' before lgdt
    mov dx, 0x3F8
    mov al, 'a'
    out dx, al

    lgdt [0x8000 + 0x210]       ; 10-byte BSP GDT descriptor at offset 0x210

    ; DEBUG: 'b' after lgdt, before retfq
    mov dx, 0x3F8
    mov al, 'b'
    out dx, al

    ; Reload CS to 0x10 (BSP's KERNEL_CS) using 64-bit far return trick.
    ; Push RIP (= address of .after_cs_reload) and CS (0x10), then retfq.
    ; retfq pops RIP then CS, completing the far jump to the new segment.
    ; Reload DS/ES/SS with BSP's KERNEL_DS = 0x18 before the far return.
    mov ax, 0x18
    mov ds, ax
    mov es, ax
    mov ss, ax
    xor ax, ax
    mov fs, ax
    mov gs, ax

    ; DEBUG: 'c' after BSP segment reload
    mov dx, 0x3F8
    mov al, 'c'
    out dx, al

    ; Load the BSP's IDT before the far return, mirroring Linux's bring-up path.
    lidt [0x8000 + 0x21A]       ; 10-byte BSP IDT descriptor at offset 0x21A

    ; DEBUG: 'd' after lidt
    mov dx, 0x3F8
    mov al, 'd'
    out dx, al

    ; Set up the AP's private stack before the far return so the push/retfq
    ; sequence uses a valid 64-bit stack instead of wrapping RSP=0.
    mov rsp, [0x8000 + 0x208]   ; ap_stack_top (u64) at offset 0x208

    push qword 0x10                         ; new CS = BSP KERNEL_CS
    lea  rax, [rel .after_cs_reload]
    push rax                                ; new RIP
    o64 retf                                ; 64-bit far return

.after_cs_reload:
    ; DEBUG: 'e' after retfq
    mov dx, 0x3F8
    mov al, 'e'
    out dx, al

    ; Reload DS/ES/SS with BSP's KERNEL_DS = 0x18.
    mov ax, 0x18
    mov ds, ax
    mov es, ax
    mov ss, ax
    xor ax, ax
    mov fs, ax
    mov gs, ax

    ; DEBUG: 'f' after post-retfq segment reload
    mov dx, 0x3F8
    mov al, 'f'
    out dx, al

    ; ── Load the BSP's IDT ────────────────────────────────────────────────
    ; The 64-bit lidt reads 10 bytes: u16 limit + u64 base.
    ; The BSP wrote its IDTR to ap_bsp_idt_desc (at offset 0x21A).
    lidt [0x8000 + 0x21A]       ; 10-byte BSP IDT descriptor at offset 0x21A

    ; DEBUG: 'g' after post-retfq lidt
    mov dx, 0x3F8
    mov al, 'g'
    out dx, al

    ; ── Set up the AP's private stack ─────────────────────────────────────
    ; The BSP allocated a 64 KiB scheduler-compatible stack per AP and wrote
    ; its top address to ap_stack_top (at offset 0x208).  x86 stacks grow
    ; downward, so RSP points to the highest address in the stack region.
    mov rsp, [0x8000 + 0x208]   ; ap_stack_top (u64) at offset 0x208
    ; DEBUG: 'h' after stack setup
    mov dx, 0x3F8
    mov al, 'h'
    out dx, al

    ; ── Read this AP's APIC ID from the LAPIC MMIO register ───────────────
    ; LAPIC ID register is at physical address 0xFEE00020; bits 31:24 = xAPIC ID.
    ; We pass it as the first argument (RDI) to ap_main.
    ;
    ; We load the address into a 32-bit register first (zero-extends to 64 bits)
    ; to avoid the sign-extension that would occur if encoded as a 32-bit
    ; displacement (0xFEE00020 has bit 31 set → sign-extends to 0xFFFFFFFF_FEE00020).
    ;
    ; Reference: Intel SDM Vol. 3A §10.4.6 "Local APIC ID"
    mov  eax, 0xFEE00020        ; EAX = 0xFEE00020; RAX = 0x0000_0000_FEE0_0020
    mov  eax, [rax]             ; read LAPIC ID register (bits 31:24 = xAPIC ID)
    shr  eax, 24
    and  eax, 0xFF
    movzx rdi, al               ; RDI = apic_id (u64, zero-extended)

    ; DEBUG: 'i' after APIC ID read
    mov dx, 0x3F8
    mov al, 'i'
    out dx, al

    ; DEBUG: write '4' to serial port before calling ap_main
    mov dx, 0x3F8
    mov al, '4'
    out dx, al

    ; ── Call the Rust ap_main function ────────────────────────────────────
    ; ap_main is defined in smp.rs with #[no_mangle] extern "C".
    ; Its address is stored by the BSP at ap_main_ptr (offset 0x228).
    ; We must use an indirect call because the kernel image is above 1 MiB,
    ; beyond the 32-bit relative range of a direct `call` from 0x8000.
    mov  rax, [0x8000 + 0x228]  ; ap_main_ptr (u64) at offset 0x228
    call rax                    ; ap_main(apic_id: u64) -> !

    ; ap_main is diverging (!); should never return.  Halt if it does.
.hang:
    cli
    hlt
    jmp .hang

; ── Minimal GDT (embedded in trampoline page) ─────────────────────────────────
;
; This GDT is only used during the 16→32→64 transition.  After the AP enters
; 64-bit mode and calls lgdt to load the BSP's full GDT, this table is no
; longer referenced.
;
; Layout (each entry is 8 bytes, selector = index × 8):
;   0x00  null descriptor    (required by x86 architecture)
;   0x08  32-bit code        (CS for the 16→32 far jump)
;   0x10  32-bit data        (DS/ES/SS in 32-bit mode)
;   0x18  64-bit code        (CS for the 32→64 far jump, L=1)
;   0x20  64-bit data        (DS/ES/SS in 64-bit mode, before BSP GDT load)

align 8
ap_local_gdt:
    dq 0x0000000000000000   ; [0x00] null descriptor
    dq 0x00CF9A000000FFFF   ; [0x08] 32-bit code: P=1 DPL=0 S=1 Type=A(code+exec) G=1 D=1
    dq 0x00CF92000000FFFF   ; [0x10] 32-bit data: P=1 DPL=0 S=1 Type=2(data+rw)   G=1 D=1
    dq 0x00AF9A000000FFFF   ; [0x18] 64-bit code: P=1 DPL=0 S=1 Type=A G=1 L=1 D=0
    dq 0x00CF92000000FFFF   ; [0x20] 64-bit data: same as 32-bit data (ignored in 64-bit mode)
ap_local_gdt_end:

ap_local_gdt_ptr:
    dw ap_local_gdt_end - ap_local_gdt - 1   ; limit = size - 1
    dd ap_local_gdt                           ; base (32-bit, fine for physical 0x8000)

; ── Data area ─────────────────────────────────────────────────────────────────
;
; The BSP writes these fields before sending the SIPI.  Offsets from 0x8000
; MUST match the OFF_* constants in smp.rs.
;
;   OFF_PML4_ADDR = 0x200   BSP's CR3 value (u32)           [+4-byte pad]
;   OFF_STACK_TOP = 0x208   AP stack top address (u64)
;   OFF_BSP_GDT   = 0x210   BSP's GDTR: limit(u16) + base(u64) = 10 bytes
;   OFF_BSP_IDT   = 0x21A   BSP's IDTR: limit(u16) + base(u64) = 10 bytes
;   OFF_AP_MAIN   = 0x228   address of ap_main Rust function (u64)
;   OFF_BSP_CR0   = 0x230   BSP CR0 for paging transition (u64)
;   OFF_BSP_CR4_PRE_PAGING = 0x238   BSP CR4 with PCIDE masked (u64)
;   OFF_BSP_CR4_LONG       = 0x240   full BSP CR4 restored in long mode (u64)
;   OFF_BSP_EFER           = 0x248   BSP EFER with LME set, LMA clear (u64)
;
; Data starts at offset 0x200 (512 bytes from page start), leaving 512 - <code_size>
; bytes of slack for code growth.  Using `$ - $$` (section-relative current offset)
; avoids the NASM "non-constant TIMES argument" error caused by forward references.

times 0x200 - ($ - $$) db 0    ; pad code area to exactly 0x200 bytes from page start

; Offset 0x200: BSP writes CR3 (PML4 physical address) here.
ap_pml4_addr: dd 0              ; u32 (4 bytes)
              dd 0              ; 4-byte pad → next field at 0x118 (8-byte aligned)

; Offset 0x208: BSP writes the AP's stack top (u64) here.
ap_stack_top: dq 0

; Offset 0x210: BSP writes GDTR (limit u16 + base u64 = 10 bytes) here.
ap_bsp_gdt_desc: dw 0, 0, 0, 0, 0   ; 10 bytes (5 × u16)

; Offset 0x21A: BSP writes IDTR (limit u16 + base u64 = 10 bytes) here.
ap_bsp_idt_desc: dw 0, 0, 0, 0, 0   ; 10 bytes (5 × u16)

; 4-byte pad at 0x224 to align ap_main_ptr to an 8-byte boundary (0x228).
dd 0

; Offset 0x228: BSP writes address of ap_main (u64) here.
ap_main_ptr: dq 0

; Offset 0x230: BSP writes CR0 state for the AP paging transition.
ap_bsp_cr0: dq 0

; Offset 0x238: BSP writes CR4 state safe before paging is active.
ap_bsp_cr4_pre_paging: dq 0

; Offset 0x240: BSP writes full CR4 state to restore after long mode is active.
ap_bsp_cr4_long: dq 0

; Offset 0x248: BSP writes EFER state with LME set and LMA clear.
ap_bsp_efer: dq 0
