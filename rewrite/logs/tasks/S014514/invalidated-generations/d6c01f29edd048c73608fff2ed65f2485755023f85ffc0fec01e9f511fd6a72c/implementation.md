# Implementation — S014514

Translated `include/linux/nfs_iostat.h` to `src/include/linux/nfs_iostat.rs`.

- Preserved `NFS_IOSTAT_VERS` as immutable `b"1.1\0"` storage with a named
  thin `*const core::ffi::c_char` view for its selected C `%s` use.
- Preserved both C enum tags as `core::ffi::c_int` aliases.  The frozen Clang
  19 x86_64 and AArch64 commands omit `-fshort-enums`, and each enum's values
  fit C `int`; each enumerator and terminal array-bound sentinel retains its
  original sequential value.
- The C include guard has no Rust runtime or ABI equivalent; this destination
  file is the one module-level definition of the declarations.
- The selected x86_64 and AArch64 configurations introduce no conditional
  declarations in this header.

Source and consumer context examined: `vendor/linux/include/linux/nfs_iostat.h`,
`vendor/linux/fs/nfs/iostat.h`, `vendor/linux/include/linux/nfs_fs_sb.h`, and
the NFS counter call sites under `vendor/linux/fs/nfs/`.
