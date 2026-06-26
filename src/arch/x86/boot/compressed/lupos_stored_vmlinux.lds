/* SPDX-License-Identifier: GPL-2.0 */
/* Temporary linker script for the Lupos uncompressed-ELF boot/compressed bridge.
 *
 * Keep this structurally close to arch/x86/boot/compressed/vmlinux.lds.S so
 * the extract_kernel shim can be replaced by the full decompressor with fewer
 * linker-layout surprises. The C-preprocessor-only values used by Linux are
 * spelled out here for the current x86_64 bridge: L1_CACHE_BYTES=64 and
 * PAGE_SIZE=4096.
 */

OUTPUT_FORMAT(elf64-x86-64)
OUTPUT_ARCH(i386:x86-64)
ENTRY(startup_64)

SECTIONS
{
	. = 0;
	.head.text : {
		_head = .;
		*(.head.text)
		_ehead = .;
	}
	.rodata..compressed : {
		*(.rodata..compressed)
	}
	.text : {
		_text = .;
		*(.text)
		*(.text.*)
		*(.noinstr.text)
		_etext = .;
	}
	.rodata : {
		_rodata = .;
		*(.rodata)
		*(.rodata.*)
		_erodata = .;
	}
	.data : ALIGN(0x1000) {
		_data = .;
		*(.data)
		*(.data.*)

		/* Linux reserves four bytes for the obsolete CRC-32 checksum. */
		. = ALIGN(. + 4, 0x200);
		_edata = .;
	}
	. = ALIGN(64);
	.bss : {
		_bss = .;
		*(.bss)
		*(.bss.*)
		*(COMMON)
		. = ALIGN(8);
		_ebss = .;
	}
	. = ALIGN(0x1000);
	.pgtable : {
		_pgtable = .;
		*(.pgtable)
		_epgtable = .;
	}
	. = ALIGN(0x1000);
	_end = .;

	/DISCARD/ : {
		*(.dynamic)
		*(.dynsym)
		*(.dynstr)
		*(.dynbss)
		*(.hash)
		*(.gnu.hash)
		*(.comment)
		*(.note*)
		*(.modinfo)
		*(.eh_frame)
	}

	.got.plt (INFO) : {
		*(.got.plt)
	}
	ASSERT(SIZEOF(.got.plt) == 0 || SIZEOF(.got.plt) == 0x18,
	       "Unexpected GOT/PLT entries detected!")

	.got : {
		*(.got)
	}
	ASSERT(SIZEOF(.got) == 0, "Unexpected GOT entries detected!")

	.plt : {
		*(.plt)
		*(.plt.*)
	}
	ASSERT(SIZEOF(.plt) == 0, "Unexpected run-time procedure linkages detected!")

	.rel.dyn : {
		*(.rel.*)
		*(.rel_*)
	}
	ASSERT(SIZEOF(.rel.dyn) == 0, "Unexpected run-time relocations (.rel) detected!")

	.rela.dyn : {
		*(.rela.*)
		*(.rela_*)
	}
	ASSERT(SIZEOF(.rela.dyn) == 0, "Unexpected run-time relocations (.rela) detected!")
}
