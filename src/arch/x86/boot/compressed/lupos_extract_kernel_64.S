/* SPDX-License-Identifier: GPL-2.0 */
/*
 * Temporary Lupos extract_kernel shim for the real Linux x86_64 compressed
 * head. Linux's boot/compressed/head_64.S calls:
 *
 *   extract_kernel(struct boot_params *rmode, unsigned char *output)
 *
 * and jumps to the returned 64-bit entry with RSI restored to boot_params.
 * xtask stores a gzip stream in input_data using Linux's mkpiggy convention.
 * This shim preserves the Linux decompressor staging shape:
 *
 *   extract_kernel -> decompress_kernel -> parse_elf -> handle_relocations
 *
 * decompress_kernel calls Linux's preboot gzip __decompress() implementation
 * from vendor/linux/lib/decompress_inflate.c. parse_elf then reads that output
 * staging buffer and places PT_LOAD payloads at the compact ELF's physical
 * addresses. The bss zeroing remains because the compact ELF omits
 * p_memsz - p_filesz bytes.
 *
 * Keep the program-header walk in ELF order. Larger feature builds can place
 * the decompressed ELF staging buffer above the runtime image, and zeroing the
 * high BSS segment before lower file-backed segments are copied can erase
 * source bytes that parse_elf still needs.
 */

#define LUPOS_LOAD_PHYSICAL_ADDR 0x01000000
#define LUPOS_BOOT_HEAP_SIZE 0x10000

	.code64
	.text

	.globl	extract_kernel
	.type	extract_kernel, @function
extract_kernel:
	pushq	%rbx
	movq	%rsi, %rbx		/* Linux outbuf / decompression target */
	movq	%rbx, %rdi
	call	decompress_kernel	/* returns compact ELF e_entry */
	popq	%rbx
	ret
	.size	extract_kernel, .-extract_kernel

	.type	decompress_kernel, @function
decompress_kernel:
	pushq	%rbx
	pushq	%r12
	pushq	%r13
	pushq	%r14
	pushq	%r15
	movq	%rdi, %rbx		/* outbuf */

	/*
	 * Linux decompress_kernel() initializes the compressed-stage heap before
	 * calling __decompress(input_data, input_len, NULL, NULL, outbuf,
	 * output_len, NULL, error).
	 */
	movq	free_mem_ptr(%rip), %rax
	testq	%rax, %rax
	jnz	.Lheap_ready
	leaq	boot_heap(%rip), %rax
	movq	%rax, free_mem_ptr(%rip)
	addq	$LUPOS_BOOT_HEAP_SIZE, %rax
	movq	%rax, free_mem_end_ptr(%rip)

.Lheap_ready:
	leaq	input_data(%rip), %rdi
	movl	input_len(%rip), %esi
	xorl	%edx, %edx		/* fill = NULL */
	xorl	%ecx, %ecx		/* flush = NULL */
	movq	%rbx, %r8		/* outbuf */
	movl	output_len(%rip), %r9d
	leaq	lupos_decompress_error(%rip), %rax
	pushq	%rax			/* error callback, arg 8 */
	pushq	$0			/* pos = NULL, arg 7 */
	call	__decompress
	addq	$16, %rsp
	testl	%eax, %eax
	js	.Lbad_payload

	movq	%rbx, %rdi
	call	parse_elf
	movq	entry_addr(%rip), %r12

	movq	%rbx, %rdi
	movl	output_len(%rip), %esi
	movl	$LUPOS_LOAD_PHYSICAL_ADDR, %edx
	call	handle_relocations

	movq	%r12, %rax
	popq	%r15
	popq	%r14
	popq	%r13
	popq	%r12
	popq	%rbx
	ret
	.size	decompress_kernel, .-decompress_kernel

	.type	parse_elf, @function
parse_elf:
	pushq	%r12
	pushq	%r13
	pushq	%r14
	pushq	%r15
	movq	%rsp, %r15
	movq	%rdi, %r11		/* decompressed ELF image in outbuf */

	/* ELF64 little-endian executable for EM_X86_64. */
	cmpl	$0x464c457f, (%r11)
	jne	.Lbad_elf
	cmpb	$2, 4(%r11)
	jne	.Lbad_elf
	cmpb	$1, 5(%r11)
	jne	.Lbad_elf
	cmpw	$0x3e, 18(%r11)
	jne	.Lbad_elf

	movzwl	54(%r11), %r9d		/* e_phentsize */
	cmpw	$56, %r9w
	jb	.Lbad_elf
	movzwl	56(%r11), %r8d		/* e_phnum */
	testl	%r8d, %r8d
	jz	.Lbad_elf

	movq	24(%r11), %r12		/* e_entry */
	movq	%r8, %r13		/* e_phnum */
	movq	%r8, %rax
	imulq	%r9, %rax		/* e_phnum * e_phentsize */
	movq	%rax, %r14
	addq	$15, %rax
	andq	$-16, %rax
	subq	%rax, %rsp

	movq	%rsp, %r10		/* stack copy of program headers */
	movq	%r10, %rdi
	movq	32(%r11), %rsi		/* e_phoff */
	addq	%r11, %rsi
	movq	%r14, %rdx
	subq	$8, %rsp
	call	memmove
	addq	$8, %rsp
	movq	%r13, %r8

.Lphdr_loop:
	cmpl	$1, (%r10)		/* PT_LOAD */
	jne	.Lnext_phdr
	movq	48(%r10), %rax		/* p_align */
	testq	$0x1fffff, %rax
	jnz	.Lbad_elf

	movq	32(%r10), %rcx		/* p_filesz */
	movq	40(%r10), %rdx		/* p_memsz */
	cmpq	%rcx, %rdx
	jb	.Lbad_elf

	movq	24(%r10), %rdi		/* p_paddr */
	movq	8(%r10), %rsi		/* p_offset */
	addq	%r11, %rsi

	pushq	%r8
	pushq	%r9
	pushq	%r10
	pushq	%r11
	pushq	%rdx
	pushq	%rcx
	pushq	%rdi
	movq	%rcx, %rdx
	call	memmove
	popq	%rdi
	popq	%rcx
	popq	%rdx
	subq	%rcx, %rdx
	jz	.Lsegment_done
	addq	%rcx, %rdi
	movq	%rdx, %rcx
	xorl	%eax, %eax
	cld
	xorl	%eax, %eax
	rep stosb

.Lsegment_done:
	popq	%r11
	popq	%r10
	popq	%r9
	popq	%r8

.Lnext_phdr:
	addq	%r9, %r10
	decl	%r8d
	jnz	.Lphdr_loop

	movq	%r15, %rsp
	movq	%r12, %rax		/* e_entry */
	popq	%r15
	popq	%r14
	popq	%r13
	popq	%r12
	ret
	.size	parse_elf, .-parse_elf

	.type	handle_relocations, @function
handle_relocations:
	ret
	.size	handle_relocations, .-handle_relocations

lupos_decompress_error:
	ret

	.globl	memcpy
	.type	memcpy, @function
memcpy:
	jmp	memmove
	.size	memcpy, .-memcpy

	.globl	memmove
	.type	memmove, @function
memmove:
	movq	%rdi, %rax
	testq	%rdx, %rdx
	jz	.Lmemmove_done
	cmpq	%rsi, %rdi
	jbe	.Lmemmove_forward
	leaq	(%rsi,%rdx), %rcx
	cmpq	%rcx, %rdi
	jae	.Lmemmove_forward
	leaq	-1(%rdi,%rdx), %rdi
	leaq	-1(%rsi,%rdx), %rsi
	movq	%rdx, %rcx
	std
	rep movsb
	cld
	ret
.Lmemmove_forward:
	movq	%rdx, %rcx
	cld
	rep movsb
.Lmemmove_done:
	ret
	.size	memmove, .-memmove

.Lbad_payload:
.Lbad_elf:
	cli
1:
	hlt
	jmp	1b

	.globl	load_stage1_idt
	.type	load_stage1_idt, @function
load_stage1_idt:
	ret
	.size	load_stage1_idt, .-load_stage1_idt

	.globl	load_stage2_idt
	.type	load_stage2_idt, @function
load_stage2_idt:
	ret
	.size	load_stage2_idt, .-load_stage2_idt

	.globl	configure_5level_paging
	.type	configure_5level_paging, @function
configure_5level_paging:
	ret
	.size	configure_5level_paging, .-configure_5level_paging

	.globl	initialize_identity_maps
	.type	initialize_identity_maps, @function
initialize_identity_maps:
	ret
	.size	initialize_identity_maps, .-initialize_identity_maps

	.bss
	.balign	8
	.globl	free_mem_ptr
free_mem_ptr:
	.quad	0
	.globl	free_mem_end_ptr
free_mem_end_ptr:
	.quad	0
	.balign	8
boot_heap:
	.skip	LUPOS_BOOT_HEAP_SIZE
