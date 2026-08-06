# Implementation — S016271

Actual implementer: `gpt-5.6-terra`, medium reasoning effort (fallback from the advisory Luna assignment).

Read the complete pinned UAPI header, the in-kernel wrapper header, and both selected FTP conntrack/NAT consumers. The source declares one userspace-visible C enum with four implicit consecutive `int` values and no configuration-dependent branches, storage, or executable behavior.

The fresh destination uses a `#[repr(transparent)]` `c_int` wrapper and four typed constants, preserving the C enum tag's ABI representation while allowing all C `int` payloads rather than making Rust's closed enum validity requirements stronger. No historical Rust source, compilation, formatting, tests, or runtime tools were used.
