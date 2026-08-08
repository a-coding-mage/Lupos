# Parity review — S016386 — slot 1

Result: **FINDINGS**.  This review used only `vendor/linux/include/uapi/linux/socket.h`, the candidate snapshot, frozen task manifests, and direct pinned UAPI consumers.  No compiler, formatter, analyzer, test, or historical Lupos source was used.

## Findings

1. **P1 — `struct __kernel_sockaddr_storage`: anonymous-member ABI/interface is not preserved.**
   Upstream `socket.h:16-25` defines one anonymous union whose anonymous struct promotes `ss_family` and `__data` to direct members of `struct __kernel_sockaddr_storage`, while `__align` is likewise a direct member of that outer structure.  The candidate instead exposes a named `__storage` field containing a named `__kernel_sockaddr_storage_union`, and puts `ss_family`/`__data` one further level inside a named `__data` field.  Thus the direct UAPI member namespace and member-access contract differ (`storage.ss_family`/`storage.__align` in C versus nested Rust paths).  Direct consumers embed this type in externally visible UAPI records, including `include/uapi/linux/in.h:211-246`, `include/uapi/linux/mptcp.h:96-112`, and `include/uapi/rdma/rdma_user_cm.h:120-143`.  The frozen symbol inventory includes the outer structure and both anonymous aggregate records for both architectures.  The candidate has no source-proven compatibility mechanism for those promoted members.

2. **P1 — `__kernel_sockaddr_storage.__data`: `char[126]` has been changed to `[u8; 126]`.**
   The upstream member is explicitly `char __data[_K_SS_MAXSIZE - sizeof(unsigned short)]` at `socket.h:21`; its element type and language-level signedness/conversion behavior are part of the declared UAPI surface.  The candidate snapshot substitutes `[u8; ...]`.  Even if both representations occupy one byte, this is not the same declared C-facing element type and changes signed interpretation for consumers.  Neither the frozen ABI rows nor the direct UAPI context supplies an accepted source-level rule that permits this replacement for both selected targets.

3. **P1 — operative macro and include-guard contract is replaced with typed Rust items.**
   Upstream has the `_UAPI_LINUX_SOCKET_H` conditional/guard (`socket.h:2-3,38`) and operative preprocessor macros `_K_SS_MAXSIZE`, `SOCK_SNDBUF_LOCK`, `SOCK_RCVBUF_LOCK`, `SOCK_BUF_LOCK_MASK`, and all three `SOCK_TXREHASH_*` definitions (`socket.h:8,29-36`).  The frozen inventory selects those conditionals and macros for both architectures.  The candidate has no guard representation and replaces token-substitution macros with `usize`/`u32` constants.  That changes macro expansion, preprocessor-condition usability, and the type selected by a C expression’s normal integer rules.  No frozen translation mechanism or ABI rule establishes exact equivalence, so the selected conditional and macro semantic records cannot be closed as parity-preserving.

## Required disposition

Do not accept this candidate as a complete UAPI translation.  An applier needs source-proven, frozen guidance for preserving the anonymous aggregate/member interface, the `char` element contract, and the selected preprocessor macro/guard interface; absent that evidence, the task must remain blocked rather than treating the Rust wrapper as equivalent.
