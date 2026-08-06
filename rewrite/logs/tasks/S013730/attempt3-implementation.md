# S013730 implementation — attempt 3

Model: gpt-5.6-terra, medium reasoning effort.

Translated `include/linux/device-id/rpmsg.h` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`. The candidate preserves the two
macros, the kernel-only `kernel_ulong_t` alias for the two frozen 64-bit
targets, and the C-layout, copyable `rpmsg_device_id` field sequence.

No compiler, formatter, linker, test, or runtime tooling was run.
