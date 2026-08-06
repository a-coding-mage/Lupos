# Parity review — S016271 (slot 1)

Verdict: **ACCEPT** — no actionable source-parity finding.

Reviewed only from source and frozen Phase 0 metadata; no compiler, formatter,
linker, test, or runtime command was used.

## Evidence examined

- Pinned source: `vendor/linux/include/uapi/linux/netfilter/nf_conntrack_ftp.h`
  at `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Candidate: `src/include/uapi/linux/netfilter/nf_conntrack_ftp.rs`.
- Frozen x86_64 configuration and metadata:
  `rewrite/configs/x86_64/frozen.config`,
  `rewrite/metadata/header_closure.tsv`, and
  `rewrite/metadata/x86_64/{object_inventory.tsv,compile_commands.json}`.
- Direct selected consumer context:
  `include/linux/netfilter/nf_conntrack_ftp.h`,
  `net/netfilter/nf_conntrack_ftp.c`, and `net/netfilter/nf_nat_ftp.c`.

## Comparison

The UAPI header is unconditional and defines exactly one public enum tag,
`nf_ct_ftp_type`, with four implicit consecutive values.  The frozen x86_64
configuration selects `CONFIG_NF_CONNTRACK_FTP=y`; the header closure records
exactly the two built-in Rust-translate consumers, `nf_conntrack_ftp.o` and
`nf_nat_ftp.o`.  Neither the header nor those selection conditions add a
configuration-dependent branch to this declaration.

The candidate retains the immutable provenance and exact source SPDX text.  Its
transparent `c_int` wrapper preserves the x86_64 C enum storage ABI while
avoiding a Rust closed-enum validity restriction that would reject values a C
enum object may carry after an integer conversion.  `#[repr(transparent)]`
makes the wrapper's layout and ABI those of its sole `c_int` field: no added
padding, alignment change, packing rule, or bitfield interpretation is
introduced.  The four public values retain their source order and values:
`PORT = 0`, `PASV = 1`, `EPRT = 2`, and `EPSV = 3`.

There are no macros (other than the C include guard), masks, structures,
unions, fields, packing directives, linkage declarations, functions, storage,
or cleanup/locking behavior in the source header to translate.  The C include
guard has no Rust runtime or ABI counterpart.  The consumers use this type only
as the FTP command discriminator and switch over exactly these four values;
the candidate provides each discriminator with the same integer payload.

No unauthorized branding, omitted selected declaration, changed value, or
ABI/layout discrepancy was found.
