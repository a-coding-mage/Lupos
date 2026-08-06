# S013677 implementation

Pinned source: `vendor/linux/include/linux/decompress/generic.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The header's sole selected type, `decompress_fn`, is represented as a nullable
C-ABI function pointer.  Each C callback parameter is independently nullable,
and every buffer and output-position argument remains a raw pointer so this
declaration neither owns nor shortens any Linux-controlled storage lifetime.
`long`, `unsigned long`, `unsigned char`, and `char` use their matching C FFI
types.  The `decompress_method` declaration retains its const input buffer,
nullable output-name pointer, and nullable function-pointer result.

No build, formatting, test, or runtime command was run.
