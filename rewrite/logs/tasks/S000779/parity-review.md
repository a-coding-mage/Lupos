# Parity review — S000779, attempt 1, P02

Reviewer: parity reviewer (`gpt-5.6-terra`, high)

Reviewed only the pinned source, task candidate snapshot, and direct frozen records. No compiler, formatter, analyzer, test, or historical Lupos source was used.

## Findings

### P1 — `struct user_desc` ABI and bit-field layout were asserted, not established

`arch/x86/include/uapi/asm/ldt.h:21-41` defines the user-visible `struct user_desc` as three `unsigned int` members followed by seven one-bit/two-bit `unsigned int` bit-fields and the x86_64-only `lm` bit. The candidate replaces those declared fields with a fourth named `u32` member, `flags`, and manually assumes least-significant-bit allocation order.

The direct ABI record for this exact type (`rewrite/ABI.tsv`, S000779) remains `PENDING_REVIEW` for layout, alignment, and export kind; it contains no pinned layout/bit-field-allocation evidence that proves the candidate's assumed 16-byte/bit-0-through-bit-7 projection. `#[repr(C)]` only governs Rust's ordinary-field layout; it does not establish the frozen C compiler's unsigned-int bit-field allocation contract. Therefore the candidate cannot close the `struct user_desc` ABI record from the supplied source evidence.

This is observable at the syscall ABI: pinned `arch/x86/kernel/tls.c:119-123,217-237` and `arch/x86/kernel/ldt.c:583-600` copy `sizeof(struct user_desc)` directly to/from userspace and then interpret the bit-fields. A wrong allocation unit, bit ordering, size, or alignment changes the bytes accepted and returned by `modify_ldt`, `set_thread_area`, and `get_thread_area`.

Affected semantic records: `SC1-5463f93831a07b738772ddea3cf73bbd3ccf028e90eff96748d8ada5bc823af8`, `SC1-8655eb0964f26c453ddae4812fb5445d0ee7e8bf4e7c15def729fa8fdf2bf4c7`, `SC1-0d37fac5188ba4328a6b44822aca714b3d412f146d4f82c6c358b8524cd8ded9`, `SC1-018e330573a032ad63590eaa3d89ae8c82feee1d96b4ddc9884973b4eb71b315`, `SC1-68a4c058f17c04256cfb6a8d8ef223a5fa95ba6665237b6b3a9c35e860fb37e7`, `SC1-77b2744a7b0a1f2caa7ae90bff78b460bef486f01255b8afe9b89ce95ecbe7d8`.

### P1 — public field read/write semantics were removed

The source exposes the named fields `seg_32bit`, `contents`, `read_exec_only`, `limit_in_pages`, `seg_not_present`, `useable`, and (on this selected x86_64 target) `lm`. Pinned `arch/x86/include/asm/desc.h:16-42` reads all of the relevant bit-fields when constructing an LDT descriptor. Pinned `arch/x86/kernel/tls.c:198-215` assigns every field, including `lm`, before `copy_to_user`; `arch/x86/kernel/tls.c:53-80` and `arch/x86/kernel/ldt.c:595-610` make decisions from the same fields.

The candidate removes those public members and provides read-only methods over a differently named raw field. It supplies no named write operations and no source-backed cross-file mapping for direct field assignment, address-taking, or the C aggregate's ordinary copy semantics. In particular, the candidate does not derive `Copy` for the C plain-data aggregate, despite pinned consumers using stack aggregates and bytewise transfer. This is not an equivalent header/API translation.

Affected semantic records: `SC1-5463f93831a07b738772ddea3cf73bbd3ccf028e90eff96748d8ada5bc823af8`, `SC1-8655eb0964f26c453ddae4812fb5445d0ee7e8bf4e7c15def729fa8fdf2bf4c7`, `SC1-68a4c058f17c04256cfb6a8d8ef223a5fa95ba6665237b6b3a9c35e860fb37e7`, `SC1-9e58c1a8da3e3437358247ce5f6b99d02d92dd90f52914ee01977b7e8eab8593`, `SC1-d4fde54277313a869f114481dd51288d7c59d271f0fbb7afda7fb6f0f1432123`.

### P1 — C/assembly macro contract and expression types are not represented

`LDT_ENTRIES` and `LDT_ENTRY_SIZE` occur outside the source's `#ifndef __ASSEMBLER__` block (`ldt.h:11-15`), so they are deliberately available to both C and assembly consumers. The candidate makes them Rust-only `u32` constants and omits any documented generation/bridge that supplies the UAPI macro contract to non-Rust consumers. It likewise turns the unsuffixed C integer macro replacements for all five numeric macros into `u32`, without evidence that this preserves their C integer-promotion and expression behavior at pinned consumers such as `arch/x86/kernel/ldt.c:515-520` and `arch/x86/xen/enlighten_pv.c:500,520`.

The candidate also omits an equivalent mapping for the selected `__ASSEMBLER__`, header-guard, and `__x86_64__` conditional behavior. A Rust module alone cannot perform those C preprocessor roles. The direct SYMBOLS rows remain `PENDING_REVIEW`; marking them complete has no source-backed basis.

Affected semantic records: `SC1-0e171631b0e3a66312ad9301bf41277811af4233699fc82a852a9ae96c1d28c0`, `SC1-24abe370ff799f25675f75d5ba1b5a87cd69f27aec81c766cfe179de5c6fa568`, `SC1-40223b5dc6dad75dc4e0ccc0c1b822578a288ea37c192a8de78300f0c5916c23`, `SC1-b1fd82ed4a565fbb32911df03e8d00459883007ac597145d335fbd9525750ad0`, `SC1-ef3f1dd191376f21f327ad6c132d24dd116347a726294694061f2562df9dd7eb`, `SC1-b708e35e1381016a83f168ec4b3a1518915b746cad2920db2b66e65d1797bccc`, `SC1-c55dc368b4e818f1f0c7dd2365c8fe91cebd8af95598246f1f17196feaf09638`, `SC1-74ff04f8e426b841bc3ea99d9da31029992b3e44ed03734de7708232593c0c22`, `SC1-f008baa1168242bd5126b6dc743d552ad5726c4998b11da9688e6c5199e209bb`.

## Result

FINDINGS. The source-level evidence cannot establish exact UAPI, C-preprocessor, and bit-field ABI parity for the candidate. These are blocking P1 findings, not compiler questions.
