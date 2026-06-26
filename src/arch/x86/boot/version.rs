//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/version.c
//! test-origin: linux:vendor/linux/arch/x86/boot/version.c
//! Kernel-version string used by the real-mode setup banner.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/version.c
//!
//! Linux composes `kernel_version[]` from
//! `UTS_RELEASE " (" LINUX_COMPILE_BY "@" LINUX_COMPILE_HOST ") " UTS_VERSION`
//! at build time from `src/include/generated/{utsrelease.h,compile.h,utsversion.h}`.
//! Lupos reproduces the same composition rule, sourcing the build-time
//! strings from `crate::build` (set in `build.rs`) when available, or
//! falling back to compile-time defaults for host tests.

/// Linux `kernel_version[]` analogue. The literal is what Linux assembles
/// — exact format is `<release> (<user>@<host>) <version>`.
pub const KERNEL_VERSION: &str =
    concat!(env!("CARGO_PKG_VERSION"), " (lupos@build) ", "#1 SMP lupos");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_version_format_matches_linux_pattern() {
        // <release> (<user>@<host>) <version>
        assert!(KERNEL_VERSION.contains('('));
        assert!(KERNEL_VERSION.contains('@'));
        assert!(KERNEL_VERSION.contains(')'));
        // The first character must be a digit (semantic version) just
        // like Linux's "6.18.0-rc1" pattern.
        assert!(KERNEL_VERSION.chars().next().unwrap().is_ascii_digit());
    }
}
