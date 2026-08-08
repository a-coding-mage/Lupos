# Resolution — S016386 — attempt 1 — P01

## Outcome

**BLOCKED.** The candidate is sealed and was not edited.  The pinned source
and frozen records do not establish an exact Rust representation for the
selected C UAPI interface, so applying a guessed wrapper would violate the
zero-difference contract.

## Finding dispositions

| Finding | Disposition | Source-level basis |
| --- | --- | --- |
| P1 / R1 — anonymous aggregate and promoted-member interface | **Unresolved; blocker.** | `include/uapi/linux/socket.h:16-26` declares an anonymous union containing an anonymous struct, thereby promoting `ss_family`, `__data`, and `__align` into `struct __kernel_sockaddr_storage`.  The candidate instead adds `__storage` and named helper aggregate types.  The direct UAPI embeds in `include/uapi/linux/in.h:214-246`, `include/uapi/linux/mptcp.h:96-112`, `include/uapi/linux/tcp.h:389-470`, and `include/uapi/rdma/rdma_user_cm.h:120-143` make this public layout/interface rather than an internal implementation detail.  The corresponding x86_64/aarch64 ABI rows retain both the outer type and anonymous aggregates as `PENDING_REVIEW`; no frozen bridge supplies a compatible promoted-member mechanism. |
| P2 / R2 — `char __data[126]` value domain | **Unresolved; blocker.** | `socket.h:19-22` declares plain `char`, while the candidate declares `[u8; 126]`.  The frozen ABI rows for the enclosing storage and anonymous struct are `PENDING_REVIEW` for both approved architectures, and no frozen source record proves the C plain-`char` signedness/promotion contract is equivalent to Rust `u8`. |
| P3 / R3 — include guard and integer macro expression contract | **Unresolved; blocker.** | The selected inventory retains `_UAPI_LINUX_SOCKET_H` and the macros at `socket.h:2-8,29-36` for x86_64 and AArch64.  The candidate replaces their preprocessor/token-substitution behavior and untyped C integer-constant expressions with typed Rust constants.  Pinned use sites show expression-context dependence: `net/core/sock.c:1280` casts before comparing `SOCK_TXREHASH_DEFAULT`, and `sock.c:1651-1658,2147-2149` uses `SOCK_BUF_LOCK_MASK` in bitwise expressions.  No frozen Rust macro/guard mechanism or scalar-promotion ABI rule establishes exact preservation. |

All cited `SYMBOLS.tsv`, `ABI.tsv`, and `LIFETIMES.tsv` records remain
`PENDING_REVIEW`; therefore none may be closed for this task.  No compiler,
formatter, analyzer, linker, test, runtime tool, or historical Lupos source
was used.
