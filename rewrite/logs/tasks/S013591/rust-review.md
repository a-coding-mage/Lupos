# Rust source review — S013591

## Scope and method

Independent manual source review of the current candidate
`src/include/linux/circ_buf.rs` against pinned
`vendor/linux/include/linux/circ_buf.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen Phase-0 manifests,
and direct frozen consumer evidence.  No compiler, formatter, test,
rust-analyzer diagnostic, historical Lupos source, or implementer/private
rationale was used.

## Result: FINDINGS

### R1 — `CIRC_CNT_TO_END` and `CIRC_SPACE_TO_END` lose the header's fixed `int` temporaries

`circ_buf.h:26-35` is a GNU statement expression whose `end` and `n` are
explicitly `int`.  Consequently, the results of `(size) - (tail)` and
`(head) + end` are converted to signed 32-bit `int` at each declaration,
before the comparison and final conditional result.  The candidate instead
infers `end` and `n` from its macro arguments and performs `wrapping_*` at
that inferred width.  For an unsigned-long caller (the direct selected
consumer `kernel/events/ring_buffer.c:141-150` establishes this input family
for the header), those are different arithmetic domains and can differ when
values do not fit `int`.  It also changes the signed comparison in the
conditional to a comparison in the inferred type.

The header declares no constraint which authorizes erasing the two `int`
conversions.  This is a semantic and C-promotion/wrapping mismatch, not a
style concern.

Affected closure keys:

- `SC1-accf1c8fc2bb85670fdbb323967ab3a289bc7f8cac35e33c386aa89a84468352`
  (`CIRC_CNT_TO_END`, aarch64)
- `SC1-dd564e04d2f89b6059c2d69c3036dac48aea1b7b1a5c68b079e53a89ceb5b326`
  (`CIRC_SPACE_TO_END`, aarch64)
- `SC1-3d69afccf47ca1a7a25d9bc529498d77261b803f2dbc96f324535b700f2c8739`
  (`CIRC_CNT_TO_END`, x86_64)
- `SC1-c26c0cc900ac5e02c61d2d257f6f70a0ef37ccbdd1a6121a697f9060fa4645b4`
  (`CIRC_SPACE_TO_END`, x86_64)

### R2 — all four macros replace C usual arithmetic conversions with a same-type Rust method contract

The C macros are token substitutions: their arithmetic uses each operand's
actual C type and the usual arithmetic conversions.  The Rust replacements
call `wrapping_sub`/`wrapping_add` directly on an argument expression.  That
requires the operands to have a Rust method-compatible common type and makes
the result that receiver type.  It does not represent C's promotions of narrow
integer arguments, signed/unsigned balancing, or mixed-width arithmetic.

The direct selected `ring_buffer_has_space` consumer has `unsigned long`
`head`, `tail`, and `data_size`, and an `unsigned int size`
(`ring_buffer.c:141-150`); frozen header-closure evidence also records 42
AArch64 and 4 x86_64 consumers.  The candidate supplies no source-derived
argument-domain contract or conversion boundary for the macro interface.
The fixed left-to-right evaluation imposed by Rust also is not the C
operator's unspecified operand-evaluation order.  Although each supplied
argument remains single-evaluated (which is necessary), that does not prove
the replacement preserves side-effect ordering for every valid header use.

Affected closure keys:

- `SC1-1da806bd7b4aa2574fbfbf9db70896e43364083ce55f7294d2da2de9976396c9`,
  `SC1-eafbd95f34a87bec1271af124926deba6d11da248026db595b5b49bd367113ad`,
  `SC1-accf1c8fc2bb85670fdbb323967ab3a289bc7f8cac35e33c386aa89a84468352`,
  `SC1-dd564e04d2f89b6059c2d69c3036dac48aea1b7b1a5c68b079e53a89ceb5b326`
  (the four operative macros, aarch64)
- `SC1-43bc48e0040ebeb11b64ba2671dcd3f900bbddf8742514b23645865e774cf8da`,
  `SC1-1e2bcaf674e563772588d52d2b3e31656409f3f67d1658cc27481c5cec5e8baf`,
  `SC1-3d69afccf47ca1a7a25d9bc529498d77261b803f2dbc96f324535b700f2c8739`,
  `SC1-c26c0cc900ac5e02c61d2d257f6f70a0ef37ccbdd1a6121a697f9060fa4645b4`
  (the four operative macros, x86_64)

### R3 — `circ_buf.buf` does not bind the frozen C `char` representation

The frozen C commands use `-funsigned-char` for both approved architectures.
The C field at `circ_buf.h:10` is therefore a pointer to unsigned `char` in
this configuration.  The candidate exposes `*mut core::ffi::c_char` without
any frozen-source evidence that this target-dependent Rust alias is the
required unsigned byte representation.  Pointer size and field order alone
do not close the element-type/signedness contract for callers that dereference
the buffer.  The proposed ABI layout, ownership, lifetime, and synchronization
closures for `struct circ_buf` are therefore unproven.

Affected closure keys:

- `SC1-b74469685fabe6512d0562ddcd31d4e29565849fb4a912c4efa17ec1d5e3ac9c`,
  `SC1-26ea96ed395e835f7428b1b80596517f1fe67cbee37bf49892c0db9bc1638404`,
  `SC1-11319cde9a85b4d69e6c5f720158d62549c61cd846a39988166bbe6b448955ff`,
  `SC1-8120ebe899b2890af4b77c1c6431eb9bc9ab0cbbf600b315bec7ee16966548c8`,
  `SC1-de00a9e1bb5ec9e0112b48cf0a32d085f4fc61b431ee2b27f0b280dc2ab7c3a0`,
  `SC1-9320e259c6a3d695da2a75292d086a987d3092fd3bc0aafae18ed8f04c91f631`
  (aarch64 `struct circ_buf`)
- `SC1-c9ec39ffc554afdc99405b03ac7323dec9b6a26181aa30e99e0f45188826c30a`,
  `SC1-c93f1b54133d119733638ff3feee4de48cb13f30be5e565ac397a081b8b56455`,
  `SC1-5960ec67f42db5133c2471376b8a5b7bc49caeb1548aff0a0d841e7b1e6404f1`,
  `SC1-fbfc7ad9edf5ab579adaa1893a11b634eaaaceaf2360b2dfd24c6b7d4e6ecceb`,
  `SC1-bbfec7086321f33b46a7f047988053d37999cb61bfc0a6ce5754ad90b29e178c`,
  `SC1-f9b5aa4c8da989a2f30b6db64f12ab4b51e58aeff06634ff1837538e63f79abd`
  (x86_64 `struct circ_buf`)

The `#[repr(C)]` annotation and `i32` index fields are necessary but do not
resolve these findings.  There are no unsafe blocks, Drop implementations, or
explicit `Send`/`Sync` implementations to approve; the unresolved problem is
the public ABI/macro boundary itself.  The candidate must not be accepted or
semantically sealed as COMPLETE until an exact source-derived mapping preserves
the C arithmetic/conversion contract and the frozen unsigned-character buffer
element contract.
