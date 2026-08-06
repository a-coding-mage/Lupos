# S013730 implementation — attempt 3

Model: gpt-5.6-terra, medium reasoning effort.

Translated the pinned `include/linux/device-id/rpmsg.h` header at Linux
revision `425f94c2954b1fe80ebdbf9b29854e89750355df`. The candidate preserves
the C macros, kernel-only `kernel_ulong_t` alias for both frozen 64-bit
targets, and C-layout copyable `rpmsg_device_id` field order.

No compiler, formatter, linker, test, or runtime tooling was run.
