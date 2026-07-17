/*
 * arch/x86/kernel/vmlinux.lds.S - Linker script for the 64-bit Lupos kernel
 *
 * Places the kernel at the 1 MiB mark, which is the conventional load
 * address for x86 kernels above the legacy BIOS/real-mode region.
 *
 * The generated bzImage uses the Linux boot protocol; this ELF carries no
 * bootloader-discovery header.
 *
 * _kernel_start and _kernel_end symbols mark the physical extent of the
 * kernel image — used by the memory manager to reserve the kernel's own
 * pages from the physical frame allocator (similar to Linux's __start_rodata
 * and _end symbols in vmlinux.lds.S).
 *
 * Ref: Linux: arch/x86/kernel/vmlinux.lds.S
 */

OUTPUT_FORMAT(elf64-x86-64)
ENTRY(_start)

KERNEL_VIRT_BASE = 0xffffffff80000000;
KERNEL_PHYS_BASE = 0x00200000;
LOAD_OFFSET = KERNEL_VIRT_BASE - KERNEL_PHYS_BASE;

SECTIONS
{
    /* Load the image at a low physical address so GRUB can enter it. */
    . = KERNEL_PHYS_BASE;

    /* Virtual and physical bounds of the kernel image. */
    _kernel_start = .;
    _kernel_phys_start = KERNEL_PHYS_BASE;

    /* Executable code. */
    __text_start = .;
    .text : ALIGN(4K)
    {
        *(.text .text.*)
    }
    __text_end = .;

    /* Read-only data (string literals, constants, GDT, etc.). */
    . = ALIGN(2M);
    __start_rodata = .;
    .rodata : ALIGN(4K)
    {
        *(.rodata .rodata.*)
        *(.eh_frame_hdr)

        /* Trampoline-to-key relations emitted by ARCH_ADD_TRAMP_KEY().
         * Keep the Linux PREL32 layout so static-call module metadata can
         * discover non-exported keys without symbol-specific registration.
         */
        . = ALIGN(8);
        __start_static_call_tramp_key = .;
        KEEP(*(.static_call_tramp_key))
        __stop_static_call_tramp_key = .;
    }

    /* Exception table — pairs of (fault_ip_off, fixup_ip_off) i32 PC-relative
     * offsets emitted by the `.pushsection __ex_table` directives in the
     * usercopy asm blocks.  Searched from the page-fault handler so that a
     * faulted user access can be redirected to its fixup label.
     * Mirrors Linux's EX_TABLE in include/asm-generic/vmlinux.lds.h.
     */
    __ex_table : ALIGN(8)
    {
        __start___ex_table = .;
        KEEP(*(__ex_table))
        __stop___ex_table = .;
    }
    . = ALIGN(4K);
    __end_rodata = .;

    /* Initialized read-write data. */
    . = ALIGN(2M);
    __data_start = .;
    .data : ALIGN(4K)
    {
        *(.data .data.*)
    }
    __data_end = .;

    /*
     * Boot-only sections, released by free_initmem() once the kernel is ready
     * to hand off to PID 1. Linux brackets these with __init_begin/__init_end
     * and returns the aligned pages to the buddy allocator.
     */
    . = ALIGN(4K);
    __init_begin = .;
    __init_text_start = .;
    .init.text : ALIGN(4K)
    {
        KEEP(*(.init.text .init.text.*))
    }
    . = ALIGN(4K);
    __init_text_end = .;
    __init_data_start = .;
    .init.data : ALIGN(4K)
    {
        KEEP(*(.init.data .init.data.*))
        . = ALIGN(4K);
        . += 4K;
    }
    . = ALIGN(4K);
    __init_data_end = .;
    __init_end = .;

    /* Zero-initialized data (BSS) — does not occupy space on disk. */
    __bss_start = .;
    .bss : ALIGN(4K)
    {
        *(COMMON)
        *(.bss .bss.*)
    }
    __bss_stop = .;

    /*
     * AMD SME/SEV decrypted-BSS brackets. Linux frees
     * __start_bss_decrypted_unused..__end_bss_decrypted from free_initmem()
     * after re-encrypting it when memory encryption is active. Lupos has no
     * decrypted payload today, but the symbols keep the handoff explicit.
     */
    . = ALIGN(4K);
    __start_bss_decrypted = .;
    __start_bss_decrypted_unused = .;
    __end_bss_decrypted = .;

    /* Virtual and physical end markers for the kernel image. */
    _kernel_end = .;
    _kernel_phys_end = KERNEL_PHYS_BASE + (_kernel_end - _kernel_start);

    /* Discard unwanted sections to keep the binary lean. */
    /DISCARD/ :
    {
        *(.eh_frame)
        *(.comment)
        *(.note .note.*)
    }
}
