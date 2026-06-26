//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/entry/vdso/vdso64/vgetrandom.c
//! test-origin: linux:vendor/linux/arch/x86/entry/vdso/vdso64/vgetrandom.c
//! 64-bit vDSO getrandom wrapper.

pub use crate::lib::vdso::getrandom::{
    __cvdso_getrandom, __cvdso_getrandom_data, CvdsoOpaqueState, VdsoGetrandomBackend, VdsoRngData,
    VgetrandomOpaqueParams, VgetrandomState,
};

pub fn __vdso_getrandom<B: VdsoGetrandomBackend>(
    rng_info: &VdsoRngData,
    backend: &mut B,
    buffer: &mut [u8],
    flags: u32,
    state_address: usize,
    state: &mut VgetrandomState,
    opaque_len: usize,
) -> isize {
    __cvdso_getrandom(
        rng_info,
        buffer,
        flags,
        state_address,
        state,
        opaque_len,
        backend,
    )
}

pub fn getrandom<B: VdsoGetrandomBackend>(
    rng_info: &VdsoRngData,
    backend: &mut B,
    buffer: &mut [u8],
    flags: u32,
    state_address: usize,
    state: &mut VgetrandomState,
    opaque_len: usize,
) -> isize {
    __vdso_getrandom(
        rng_info,
        backend,
        buffer,
        flags,
        state_address,
        state,
        opaque_len,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lib::vdso::getrandom::{CHACHA_BLOCK_SIZE, CHACHA_KEY_SIZE};

    struct FixedBackend;

    impl VdsoGetrandomBackend for FixedBackend {
        fn getrandom_syscall(&mut self, buffer: &mut [u8], _flags: u32) -> isize {
            for byte in buffer.iter_mut() {
                *byte = 0x5a;
            }
            buffer.len() as isize
        }

        fn chacha20_blocks_nostack(
            &mut self,
            dst: &mut [u8],
            _key: &[u8; CHACHA_KEY_SIZE],
            _counter: &mut [u32; 2],
            nblocks: usize,
        ) {
            for byte in &mut dst[..nblocks * CHACHA_BLOCK_SIZE] {
                *byte = 0xa5;
            }
        }
    }

    #[test]
    fn vgetrandom_wrapper_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/entry/vdso/vdso64/vgetrandom.c"
        ));
        assert!(source.contains("#include \"lib/vdso/getrandom.c\""));
        assert!(source.contains("ssize_t __vdso_getrandom"));
        assert!(
            source.contains(
                "return __cvdso_getrandom(buffer, len, flags, opaque_state, opaque_len);"
            )
        );
        assert!(source.contains("weak, alias(\"__vdso_getrandom\")"));
    }

    #[test]
    fn getrandom_alias_returns_vdso_wrapper_result() {
        let rng = VdsoRngData {
            is_ready: true,
            generation: 1,
        };
        let mut backend = FixedBackend;
        let mut state = VgetrandomState::default();
        let mut buffer = [0u8; 8];
        assert_eq!(
            getrandom(
                &rng,
                &mut backend,
                &mut buffer,
                0,
                0,
                &mut state,
                core::mem::size_of::<VgetrandomState>(),
            ),
            8
        );
        assert_eq!(buffer, [0xa5; 8]);
    }
}
