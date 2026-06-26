; =============================================================================
; arch/x86/boot/header.S - Linux boot-protocol entry for lupos (32-bit -> 64-bit long mode)
; =============================================================================
;
; GRUB loads the generated bzImage with its Linux loader and jumps to the
; protected-mode compressed-stage payload. The compatibility stored bridge can
; still jump to linux32_start in 32-bit protected mode with:
;   - ESI = physical address of the Linux boot_params zeropage
;   - Paging disabled, A20 enabled, GDT with flat 4 GiB segments
;
; The real Linux boot/compressed/head_64.S path jumps to linux64_start after
; extract_kernel returns, already in long mode with:
;   - RSI = physical address of the Linux boot_params zeropage
;   - Identity mappings covering the low physical kernel image
;
; This stub then transitions the CPU to 64-bit long mode before calling
; kernel_main.  The transition follows the same sequence Linux uses in
; arch/x86/boot/compressed/head_64.S (startup_32):
;
;   1. Save the boot_params pointer from ESI
;   2. Build identity-mapped page tables (PML4 → PDPT → PD with 2 MiB pages)
;   3. Enable PAE  (set CR4 bit 5)
;   4. Load PML4 into CR3
;   5. Set EFER.LME  (MSR 0xC0000080 bit 8) — enable Long Mode
;   6. Enable paging  (set CR0 bit 31) — activates long mode
;   7. Load a 64-bit GDT
;   8. Far jump to 64-bit code segment → long mode entry
;
; Once in 64-bit mode we reload segment registers and call kernel_main using
; the System V AMD64 ABI (first two args in RDI, RSI).
;
; Ref: Linux Documentation/arch/x86/boot.rst
;      https://wiki.osdev.org/Setting_Up_Long_Mode
;      Intel SDM Vol. 3A §9.8 — Software Initialization for Protected-Mode Operation
;      Intel SDM Vol. 3A §9.8.5 — Initializing IA-32e Mode
; =============================================================================

bits 32

%define KERNEL_VIRT_BASE 0xffffffff80000000
%define KERNEL_PHYS_BASE 0x00200000
%define PAGE_OFFSET_BASE 0xffff888000000000
%define VMALLOC_BASE     0xffffc90000000000
%define BOOT_STACK_SIZE  131072
%define BOOT_STACK_GUARD_SIZE 32768

%define PHYS(x) (x)

; ---------------------------------------------------------------------------
; No bootloader-discovery header is emitted in this ELF object.
;
; External bootloaders must enter through the generated bzImage's Linux
; boot-protocol path.  The linker still needs an ELF entry symbol, so `_start`
; is kept as a fail-closed placeholder and does not construct boot state.
; ---------------------------------------------------------------------------

; ---------------------------------------------------------------------------
; Page tables — identity map the first 4 GiB of physical memory, map the
; Linux-style direct map window, and map the kernel image into the higher-half
; kernel base at 0xffffffff80000000.
;
; Identity/direct map:
;   PML4[0]   → identity PDPT  →  PD0..PD3 (512 entries each, 2 MiB pages)
;   PML4[273] → direct map PDPT →  PD0..PD3 (same physical coverage)
;
; Kernel image:
;   PML4[511] → kernel PDPT    →  kernel PD (512 entries, 2 MiB pages)
;
; The kernel image is linked at 0xffffffff80000000 and loaded physically at
; 2 MiB so that the higher-half mapping can use 2 MiB leaf entries.
;
; Each table must be 4096-byte (page) aligned per the Intel SDM.
; Ref: Intel SDM Vol. 3A §4.5 — 4-Level Paging
;      Linux head_64.S lines ~199-231 — "Build early 4G boot pagetable"
;      https://wiki.osdev.org/Setting_Up_Long_Mode#Setting_up_the_Paging
; ---------------------------------------------------------------------------
section .bss
align 4096
pml4_table:
    resb 4096                       ; Page Map Level 4
identity_pdpt_table:
    resb 4096                       ; Identity map PDPT
directmap_pdpt_table:
    resb 4096                       ; Linux-style direct map PDPT
kernel_pdpt_table:
    resb 4096                       ; Kernel-image PDPT
identity_pd_tables:
    resb 4096 * 64                  ; 64 Page Directories for identity map (1 GiB each)
; Direct-map gets its own PD tables so W^X splits to the identity map do not
; make direct-map pages non-writable.  The two PDPT chains use separate physical
; PMD tables even though they start with identical 2 MiB R/W huge-page entries.
directmap_pd_tables:
    resb 4096 * 64                  ; 64 Page Directories for Linux direct map (1 GiB each)
kernel_pd_table:
    resb 4096                       ; Kernel image PD (1 GiB window, 2 MiB pages)
page_tables_end:

; Linux boot_params (zeropage) buffer handed to kernel_main
boot_params_page:
    resb 4096

; ---------------------------------------------------------------------------
; Initial stack used from 32-bit entry through early Rust initialization.
; Keep a guard cushion below it so deep early boot call chains cannot corrupt
; the static boot page tables if they outgrow the nominal stack.
; ---------------------------------------------------------------------------
align 4096
boot_stack_guard:
    resb BOOT_STACK_GUARD_SIZE
align 16                            ; 16-byte alignment for System V ABI
stack_bottom:
    resb BOOT_STACK_SIZE
stack_top:

; ---------------------------------------------------------------------------
; Saved bootloader registers — we stash EAX/EBX here before building page
; tables, then restore them into RDI/RSI in 64-bit mode.
; ---------------------------------------------------------------------------
saved_magic:
    resd 1                          ; unused legacy bootloader magic scratch
saved_info_ptr:
    resq 1                          ; unused legacy bootloader info scratch
saved_boot_params_ptr:
    resq 1                          ; ESI from Linux boot protocol entry, 64-bit

; ---------------------------------------------------------------------------
; 64-bit GDT (Global Descriptor Table)
;
; Minimal GDT for long mode — only three entries are needed:
;   0x00: Null descriptor (required by the CPU)
;   0x08: 64-bit code segment (L=1, D=0, P=1, S=1, type=Execute/Read)
;   0x10: Data segment (P=1, S=1, type=Read/Write)
;
; In long mode, the CPU largely ignores the base and limit fields of segment
; descriptors — segmentation is effectively flat.  Only the L (Long), D, and
; DPL bits matter for code segments.
;
; Ref: Intel SDM Vol. 3A §3.4.5 — Segment Descriptors
;      Intel SDM Vol. 3A §5.2.1 — Code-Segment Descriptor in 64-Bit Mode
;      https://wiki.osdev.org/GDT_Tutorial
; ---------------------------------------------------------------------------
section .rodata
align 16

gdt64:
    ; Null descriptor (index 0) — required, must be all zeros
    dq 0x0000000000000000

.code64:
    ; 64-bit code segment (selector 0x08)
    ;
    ; Bits breakdown (from low to high in the 8-byte descriptor):
    ;   Limit 0:15  = 0xFFFF (ignored in long mode)
    ;   Base  0:15  = 0x0000
    ;   Base 16:23  = 0x00
    ;   Access byte = 0x9A
    ;     P=1 (present), DPL=00 (ring 0), S=1 (code/data), Type=1010 (exec/read)
    ;   Flags/Limit = 0xAF
    ;     G=1 (4K granularity), D=0 (must be 0 for 64-bit), L=1 (long mode), Limit 16:19=F
    ;   Base 24:31  = 0x00
    dq 0x00AF9A000000FFFF

.data64:
    ; Data segment (selector 0x10)
    ;
    ;   Access byte = 0x92
    ;     P=1 (present), DPL=00, S=1, Type=0010 (read/write)
    ;   Flags = 0xCF
    ;     G=1, D/B=1, L=0, Limit 16:19=F
    dq 0x00CF92000000FFFF

.pointer:
    ; GDT pseudo-descriptor: loaded by LGDT instruction
    dw $ - gdt64 - 1               ; limit = size of GDT - 1
    dq gdt64                        ; base address of GDT

; ---------------------------------------------------------------------------
; Entry point — called by GRUB in 32-bit protected mode
; ---------------------------------------------------------------------------
section .text
global _start
global linux32_start
global linux64_start
extern kernel_main                  ; defined in Rust (src/main.rs)
extern boot_marker                  ; Rust helper: early 64-bit boot trace

bits 32
_start:
    ; Linker entry placeholder only.  The runnable external entry is the
    ; generated bzImage's Linux boot-protocol compressed-stage handoff into
    ; linux32_start.
    cli
.unsupported_elf_entry:
    hlt
    jmp .unsupported_elf_entry

linux32_start:
    ; Linux/x86 boot protocol 2.15 32-bit entry. GRUB's `linux` command
    ; enters protected mode with ESI pointing at the boot_params zeropage.
    xor eax, eax
    mov [PHYS(saved_magic)], eax
    mov dword [PHYS(saved_info_ptr)], 0
    mov dword [PHYS(saved_info_ptr) + 4], 0
    mov [PHYS(saved_boot_params_ptr)], esi
    mov dword [PHYS(saved_boot_params_ptr) + 4], 0

startup32_common:

    ; Set up the stack pointer.  The stack grows downward on x86.
    mov esp, PHYS(stack_top)

    ; =====================================================================
    ; Phase 2: Build identity-mapped page tables (4 GiB)
    ;
    ; We build a PML4 → PDPT → PD hierarchy that identity-maps the first
    ; 4 GiB of physical memory using 2 MiB huge pages.  4 GiB is needed
    ; because PCI device memory (including the GPU framebuffer) is typically
    ; mapped above 3 GiB (e.g., QEMU's framebuffer at 0xFD000000).
    ;
    ; Structure:
    ;   PML4[0] → PDPT
    ;   PDPT[0] → PD0  (maps 0x00000000 – 0x3FFFFFFF, 0-1 GiB)
    ;   PDPT[1] → PD1  (maps 0x40000000 – 0x7FFFFFFF, 1-2 GiB)
    ;   PDPT[2] → PD2  (maps 0x80000000 – 0xBFFFFFFF, 2-3 GiB)
    ;   PDPT[3] → PD3  (maps 0xC0000000 – 0xFFFFFFFF, 3-4 GiB)
    ;
    ; This matches Linux head_64.S: "Build early 4G boot pagetable".
    ;
    ; Ref: Intel SDM Vol. 3A §4.5.4 — PML4E, PDPTE, PDE formats
    ;      Linux head_64.S lines ~199-231 ("Build early 4G boot pagetable")
    ; =====================================================================

    ; --- PML4[0] → identity PDPT ---
    mov eax, PHYS(identity_pdpt_table)
    or eax, 0x03                    ; Present + Read/Write
    mov [PHYS(pml4_table) + 0*8], eax

    ; --- PML4[273] → Linux-style direct map PDPT ---
    mov eax, PHYS(directmap_pdpt_table)
    or eax, 0x03
    mov [PHYS(pml4_table) + 273*8], eax

    ; --- PML4[511] → kernel-image PDPT ---
    mov eax, PHYS(kernel_pdpt_table)
    or eax, 0x03
    mov [PHYS(pml4_table) + 511*8], eax

    ; Identity PDPT entries 0..63 -> identity_pd_tables (may be W^X-split later).
    mov ecx, 0
.fill_identity_pdpt:
    mov eax, ecx
    shl eax, 12                     ; eax = ecx * 4096
    add eax, PHYS(identity_pd_tables)
    or eax, 0x03
    mov [PHYS(identity_pdpt_table) + ecx*8], eax
    inc ecx
    cmp ecx, 64
    jne .fill_identity_pdpt

    ; Direct-map PDPT entries 0..63 -> directmap_pd_tables (always R/W, never split).
    mov ecx, 0
.fill_directmap_pdpt:
    mov eax, ecx
    shl eax, 12                     ; eax = ecx * 4096
    add eax, PHYS(directmap_pd_tables)
    or eax, 0x03
    mov [PHYS(directmap_pdpt_table) + ecx*8], eax
    inc ecx
    cmp ecx, 64
    jne .fill_directmap_pdpt

    ; Kernel-image PDPT entry 510 -> kernel PD table.
    mov eax, PHYS(kernel_pd_table)
    or eax, 0x03
    mov [PHYS(kernel_pdpt_table) + 510*8], eax

    ; Fill the kernel PD with 2 MiB leaf pages.
    ; This maps 1 GiB of higher-half kernel space starting at
    ; 0xffffffff80000000 to physical 0x00200000.
    mov ecx, 0
.fill_kernel_pd:
    mov eax, ecx
    mov edx, eax
    shl eax, 21                     ; low dword: index × 2 MiB
    add eax, KERNEL_PHYS_BASE
    shr edx, 11                     ; high dword of phys addr
    or eax, 0x83                    ; Present + R/W + Page Size (2 MiB huge page)
    mov [PHYS(kernel_pd_table) + ecx * 8], eax
    mov [PHYS(kernel_pd_table) + ecx * 8 + 4], edx
    inc ecx
    cmp ecx, 512
    jne .fill_kernel_pd

    ; Fill 64 identity PDs (32768 entries) with 2 MiB R/W identity pages.
    mov ecx, 0                      ; loop counter (0..32767)
.fill_pd:
    mov eax, ecx
    mov edx, eax
    shl eax, 21                     ; low dword: index × 2 MiB
    shr edx, 11                     ; high dword of phys addr
    or eax, 0x83                    ; Present + R/W + Page Size (2 MiB huge page)
    mov [PHYS(identity_pd_tables) + ecx * 8], eax
    mov [PHYS(identity_pd_tables) + ecx * 8 + 4], edx
    inc ecx
    cmp ecx, 32768                  ; 64 PDs × 512 entries = 32768
    jne .fill_pd

    ; Fill 64 direct-map PDs (32768 entries) with 2 MiB R/W+NX pages.
    ; NXE (bit 63 of PDE) is set so the direct map is never W+X.
    ; The identity-map W^X pass does not touch the direct-map tables, so NX
    ; must be set here rather than relying on protect_kernel_image_mappings.
    mov ecx, 0
.fill_directmap_pd:
    mov eax, ecx
    mov edx, eax
    shl eax, 21                     ; low dword: index × 2 MiB
    shr edx, 11                     ; high bits of phys addr (≡ 0 for <4GiB)
    or eax, 0x83                    ; Present + R/W + Page Size (2 MiB huge page)
    or edx, 0x80000000              ; NXE = bit 63 (high 32-bit word bit 31)
    mov [PHYS(directmap_pd_tables) + ecx * 8], eax
    mov [PHYS(directmap_pd_tables) + ecx * 8 + 4], edx
    inc ecx
    cmp ecx, 32768                  ; 64 PDs × 512 entries = 32768
    jne .fill_directmap_pd

    ; =====================================================================
    ; Phase 3: Enable PAE (Physical Address Extension)
    ;
    ; PAE is a prerequisite for long mode.  It extends page table entries
    ; to 64 bits, supporting physical addresses wider than 32 bits.
    ;
    ; Ref: Intel SDM Vol. 3A §4.1.1 — Four Paging Modes
    ;      Intel SDM Vol. 3A §2.5 — Control Registers (CR4.PAE = bit 5)
    ; =====================================================================
    mov eax, cr4
    or eax, (1 << 5)               ; CR4.PAE = bit 5
    mov cr4, eax

    ; =====================================================================
    ; Phase 4: Load PML4 into CR3
    ;
    ; CR3 holds the physical address of the top-level page table (PML4).
    ; The CPU reads this register during every address translation.
    ;
    ; Ref: Intel SDM Vol. 3A §2.5 — Control Registers (CR3)
    ; =====================================================================
    mov eax, PHYS(pml4_table)
    mov cr3, eax

    ; =====================================================================
    ; Phase 5: Enable Long Mode via EFER MSR
    ;
    ; The IA32_EFER MSR (Model-Specific Register) at address 0xC0000080
    ; controls long mode.  Bit 8 (LME — Long Mode Enable) tells the CPU
    ; to enter 64-bit mode once paging is activated.  Bit 11 (NXE) enables
    ; the no-execute bit in page-table entries, which our kernel uses for
    ; writable data mappings such as the kmap window.
    ;
    ; Note: Long mode is NOT active yet — it only activates when CR0.PG
    ; is set (Phase 6).  Setting LME alone is a "compatibility" step.
    ;
    ; Ref: Intel SDM Vol. 3A §9.8.5 — Initializing IA-32e Mode
    ;      AMD64 Architecture Programmer's Manual Vol. 2 §14.8
    ; =====================================================================
    mov ecx, 0xC0000080             ; IA32_EFER MSR address
    rdmsr                           ; read current EFER into EDX:EAX
    or eax, (1 << 8) | (1 << 11)    ; set LME + NXE
    wrmsr                           ; write back

    ; =====================================================================
    ; Phase 6: Enable paging (activates long mode)
    ;
    ; Setting CR0.PG (bit 31) while EFER.LME=1 and CR4.PAE=1 causes the
    ; CPU to enter IA-32e mode (long mode).  The next instruction fetch
    ; already goes through the new page tables.
    ;
    ; We also keep CR0.PE (bit 0) set — it was already set by GRUB, but
    ; we OR it in explicitly to be safe.
    ;
    ; Ref: Intel SDM Vol. 3A §9.8.5 step 5
    ; =====================================================================
    mov eax, cr0
    or eax, (1 << 31) | (1 << 16) | (1 << 0)   ; CR0.PG | CR0.WP | CR0.PE
    mov cr0, eax

    ; =====================================================================
    ; Phase 7: Load 64-bit GDT and far-jump to long mode
    ;
    ; Although paging is now active and the CPU is technically in long mode,
    ; we are still executing 32-bit code (the CS descriptor still has L=0).
    ; We must:
    ;   1. Load the 64-bit GDT (which has a code segment with L=1)
    ;   2. Far jump to that code segment — this reloads CS and switches
    ;      the CPU to 64-bit instruction decoding.
    ;
    ; Ref: Intel SDM Vol. 3A §9.8.5 step 6 — "Execute a far jump"
    ;      https://wiki.osdev.org/Setting_Up_Long_Mode#Entering_Long_Mode
    ; =====================================================================
    lgdt [PHYS(gdt64.pointer)]
    jmp 0x08:PHYS(long_mode_entry)  ; far jump to 64-bit code segment

; ===========================================================================
; 64-bit long mode code
; ===========================================================================
bits 64

linux64_start:
    ; Linux boot/compressed/head_64.S has already entered long mode and jumps
    ; here with RSI still carrying the boot_params pointer. Install Lupos'
    ; early page-table shape, then use the shared kernel_main call path below.
    mov rbx, PHYS(saved_boot_params_ptr)
    mov [rbx], rsi

    ; Clear the static page-table arena before rebuilding it.
    mov rdi, PHYS(pml4_table)
    xor rax, rax
    mov rcx, (page_tables_end - pml4_table) / 8
    rep stosq

    ; PML4[0] -> identity map, PML4[273] -> Linux-style direct map,
    ; PML4[511] -> higher-half kernel image mapping.
    mov rdi, PHYS(pml4_table)
    mov rax, PHYS(identity_pdpt_table)
    or rax, 0x03
    mov [rdi + 0*8], rax
    mov rax, PHYS(directmap_pdpt_table)
    or rax, 0x03
    mov [rdi + 273*8], rax
    mov rax, PHYS(kernel_pdpt_table)
    or rax, 0x03
    mov [rdi + 511*8], rax

    ; Identity PDPT entries 0..63 -> identity_pd_tables.
    xor rcx, rcx
    mov r8, PHYS(identity_pdpt_table)
.fill_identity_pdpt64:
    mov rax, rcx
    shl rax, 12
    add rax, PHYS(identity_pd_tables)
    or rax, 0x03
    mov [r8 + rcx*8], rax
    inc rcx
    cmp rcx, 64
    jne .fill_identity_pdpt64

    ; Direct-map PDPT entries 0..63 -> directmap_pd_tables (separate from identity).
    xor rcx, rcx
    mov r9, PHYS(directmap_pdpt_table)
.fill_directmap_pdpt64:
    mov rax, rcx
    shl rax, 12
    add rax, PHYS(directmap_pd_tables)
    or rax, 0x03
    mov [r9 + rcx*8], rax
    inc rcx
    cmp rcx, 64
    jne .fill_directmap_pdpt64

    ; Kernel-image PDPT entry 510 -> kernel PD table.
    mov rdi, PHYS(kernel_pdpt_table)
    mov rax, PHYS(kernel_pd_table)
    or rax, 0x03
    mov [rdi + 510*8], rax

    ; Map 1 GiB at 0xffffffff80000000 to physical 0x00200000.
    mov rdi, PHYS(kernel_pd_table)
    xor rcx, rcx
.fill_kernel_pd64:
    mov rax, rcx
    shl rax, 21
    add rax, KERNEL_PHYS_BASE
    or rax, 0x83
    mov [rdi + rcx*8], rax
    inc rcx
    cmp rcx, 512
    jne .fill_kernel_pd64

    ; Fill identity PD tables (32768 entries) with 2 MiB identity pages.
    mov rdi, PHYS(identity_pd_tables)
    xor rcx, rcx
.fill_pd64:
    mov rax, rcx
    shl rax, 21
    or rax, 0x83
    mov [rdi + rcx*8], rax
    inc rcx
    cmp rcx, 32768
    jne .fill_pd64

    ; Fill direct-map PD tables (32768 entries) with 2 MiB R/W+NX pages.
    mov rdi, PHYS(directmap_pd_tables)
    xor rcx, rcx
    mov r10, 0x8000000000000083    ; P|RW|PSE|NX
.fill_directmap_pd64:
    mov rax, rcx
    shl rax, 21
    or rax, r10
    mov [rdi + rcx*8], rax
    inc rcx
    cmp rcx, 32768
    jne .fill_directmap_pd64

    ; Linux's compressed head enabled long mode; Lupos also expects NXE.
    mov ecx, 0xC0000080
    rdmsr
    or eax, (1 << 11)
    wrmsr

    mov rax, PHYS(pml4_table)
    mov cr3, rax
    jmp linux64_long_mode_entry

linux64_long_mode_entry:
    mov bx, 0x18                    ; Linux compressed-head GDT data selector
    jmp reload_segments_common

long_mode_entry:
    mov bx, 0x10                    ; Lupos compatibility GDT data selector

reload_segments_common:
    ; =====================================================================
    ; Reload data segment registers
    ;
    ; After the far jump, CS is loaded with the 64-bit code segment (0x08).
    ; We must also reload the data segment registers (DS, ES, SS, FS, GS)
    ; with the data segment selector (0x10), or null (0x00).
    ;
    ; In 64-bit mode, DS/ES/SS bases are forced to 0 regardless of the
    ; descriptor, but the CPU still checks that a valid selector is loaded.
    ; FS and GS are special — they can have non-zero bases set via MSRs
    ; (used later for per-CPU data, like Linux's percpu mechanism).
    ;
    ; Ref: Intel SDM Vol. 3A §5.2.1 — Code-Segment Descriptor in 64-Bit Mode
    ;      Intel SDM Vol. 3A §3.4.4 — Segment Loading Instructions in IA-32e Mode
    ; =====================================================================
    mov ax, bx                      ; data segment selector
    mov ds, ax
    mov es, ax
    mov ss, ax
    xor ax, ax                      ; null selector for FS/GS (no percpu yet)
    mov fs, ax
    mov gs, ax

    ; Reload the stack pointer in 64-bit mode (same physical address).
    mov rsp, PHYS(stack_top)

    ; =====================================================================
    ; Call kernel_main using the System V AMD64 ABI
    ;
    ; kernel_main(boot_params: *const BootParams)
    ; =====================================================================
    call boot_marker
    mov ebx, PHYS(saved_boot_params_ptr)
    mov rax, [rbx]
    test rax, rax
    jz .missing_boot_params
    mov rdi, rax
    jmp .call_kernel_main

.missing_boot_params:
    cli
    hlt
    jmp .missing_boot_params

.call_kernel_main:
    call kernel_main

    ; kernel_main diverges (-> !), but if it ever returns, halt forever.
.hang:
    cli                             ; disable interrupts
    hlt                             ; halt the CPU until next interrupt (none)
    jmp .hang                       ; safety net — should never execute
