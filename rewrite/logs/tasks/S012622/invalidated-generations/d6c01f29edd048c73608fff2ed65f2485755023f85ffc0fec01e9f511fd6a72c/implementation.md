# S012622 implementation

Translated `include/crypto/ecdh.h` to `src/include/crypto/ecdh.rs` from pinned
Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df` for `aarch64`.

The translation retains the four C `int` curve-ID macro values, the C-layout
`struct ecdh` field order (`char *key`, `unsigned short key_size`), and the
three exported C function declarations.  Raw pointers retain C ownership and
aliasing: `ecdh` owns neither `key` nor packet data, and successful decoding
sets `key` to packet-buffer storage.  The adjacent pinned helper source
(`crypto/ecdh_helper.c`) establishes that encode requires an exact packet
length and that decode performs no private-key allocation or copy.

No build, formatter, test, or runtime command was run.
