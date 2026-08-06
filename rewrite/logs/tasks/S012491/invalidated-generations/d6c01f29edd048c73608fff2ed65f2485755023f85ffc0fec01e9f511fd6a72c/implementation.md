# S012491 implementation

Oracle: `vendor/linux/include/acpi/platform/acgccex.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The complete 24-line header contains only its include guard and a GCC
preprocessor workaround: it conditionally `#undef`s `strchr`.  It declares no
functions, types, variables, constants, layouts, linkage, or callable ABI.
The selected x86_64 and aarch64 conditional records are therefore represented
by an item-free Rust module. Rust's macro namespace has no imported C `strchr`
macro to undefine, and this module introduces no binding with that name.

Direct source context: `acenvex.h` includes this header for `__GNUC__`; no
other direct include is present. The frozen header-closure records select it
for both architectures.

No compiler, formatter, test, or runtime command was run.
