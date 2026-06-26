//! linux-parity: complete
//! linux-source: vendor/linux/lib/vdso/getrandom.c
//! test-origin: linux:vendor/linux/lib/vdso/getrandom.c
//! Generic vDSO getrandom implementation model.

use crate::include::uapi::errno::EFAULT;

pub const CONFIG_PAGE_SHIFT: usize = 12;
pub const PAGE_SIZE: usize = 1usize << CONFIG_PAGE_SHIFT;
pub const PAGE_MASK: usize = !(PAGE_SIZE - 1);
pub const MAX_RW_COUNT: usize = i32::MAX as usize & PAGE_MASK;

pub const CHACHA_KEY_SIZE: usize = 32;
pub const CHACHA_BLOCK_SIZE: usize = 64;
pub const VGETRANDOM_BATCH_SIZE: usize = CHACHA_BLOCK_SIZE * 3 / 2;
pub const VGETRANDOM_BATCH_KEY_SIZE: usize = CHACHA_BLOCK_SIZE * 2;

pub const PROT_READ: u32 = 0x1;
pub const PROT_WRITE: u32 = 0x2;
pub const MAP_DROPPABLE: u32 = 0x08;
pub const MAP_ANONYMOUS: u32 = 0x20;

pub const GRND_NONBLOCK: u32 = 0x0001;
pub const GRND_RANDOM: u32 = 0x0002;
pub const GRND_INSECURE: u32 = 0x0004;
pub const GRND_ALLOWED: u32 = GRND_NONBLOCK | GRND_RANDOM | GRND_INSECURE;

#[repr(C)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VgetrandomState {
    pub batch_key: [u8; VGETRANDOM_BATCH_KEY_SIZE],
    pub generation: u64,
    pub pos: u8,
    pub in_use: bool,
}

impl Default for VgetrandomState {
    fn default() -> Self {
        Self {
            batch_key: [0; VGETRANDOM_BATCH_KEY_SIZE],
            generation: 0,
            pos: VGETRANDOM_BATCH_SIZE as u8,
            in_use: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VdsoRngData {
    pub is_ready: bool,
    pub generation: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VgetrandomOpaqueParams {
    pub size_of_opaque_state: u32,
    pub mmap_prot: u32,
    pub mmap_flags: u32,
    pub reserved: [u32; 13],
}

pub enum CvdsoOpaqueState<'a> {
    Params(&'a mut VgetrandomOpaqueParams),
    State {
        address: usize,
        state: &'a mut VgetrandomState,
    },
}

pub trait VdsoGetrandomBackend {
    fn getrandom_syscall(&mut self, buffer: &mut [u8], flags: u32) -> isize;

    fn chacha20_blocks_nostack(
        &mut self,
        dst: &mut [u8],
        key: &[u8; CHACHA_KEY_SIZE],
        counter: &mut [u32; 2],
        nblocks: usize,
    );
}

pub fn memcpy_and_zero_src(dst: &mut [u8], src: &mut [u8]) {
    let len = core::cmp::min(dst.len(), src.len());
    for index in 0..len {
        dst[index] = src[index];
        src[index] = 0;
    }
}

pub fn vgetrandom_state_straddles_page(address: usize) -> bool {
    (address & !PAGE_MASK) + core::mem::size_of::<VgetrandomState>() > PAGE_SIZE
}

fn fallback_syscall<B: VdsoGetrandomBackend>(
    buffer: &mut [u8],
    flags: u32,
    state: Option<&mut VgetrandomState>,
    backend: &mut B,
) -> isize {
    if let Some(state) = state {
        state.in_use = false;
    }
    backend.getrandom_syscall(buffer, flags)
}

pub fn __cvdso_getrandom_data<B: VdsoGetrandomBackend>(
    rng_info: &VdsoRngData,
    mut buffer: Option<&mut [u8]>,
    flags: u32,
    opaque_state: CvdsoOpaqueState<'_>,
    opaque_len: usize,
    backend: &mut B,
) -> isize {
    if opaque_len == usize::MAX && buffer.as_ref().is_none_or(|buf| buf.is_empty()) && flags == 0 {
        if let CvdsoOpaqueState::Params(params) = opaque_state {
            params.size_of_opaque_state = core::mem::size_of::<VgetrandomState>() as u32;
            params.mmap_prot = PROT_READ | PROT_WRITE;
            params.mmap_flags = MAP_DROPPABLE | MAP_ANONYMOUS;
            params.reserved = [0; 13];
            return 0;
        }
    }

    let Some(orig_buffer) = buffer.as_mut() else {
        let mut empty = [];
        return backend.getrandom_syscall(&mut empty, flags);
    };
    let CvdsoOpaqueState::State { address, state } = opaque_state else {
        return backend.getrandom_syscall(orig_buffer, flags);
    };

    let ret = core::cmp::min(MAX_RW_COUNT, orig_buffer.len());

    if vgetrandom_state_straddles_page(address) {
        return -(EFAULT as isize);
    }
    if flags & !GRND_ALLOWED != 0 {
        return fallback_syscall(orig_buffer, flags, None, backend);
    }
    if opaque_len != core::mem::size_of::<VgetrandomState>() {
        return fallback_syscall(orig_buffer, flags, None, backend);
    }
    if !rng_info.is_ready {
        return fallback_syscall(orig_buffer, flags, None, backend);
    }
    if ret == 0 {
        return 0;
    }
    if state.in_use {
        return fallback_syscall(orig_buffer, flags, None, backend);
    }
    state.in_use = true;

    let mut have_retried = false;
    let mut counter = [0u32; 2];

    'retry_generation: loop {
        let current_generation = rng_info.generation;
        if state.generation != current_generation {
            state.generation = current_generation;
            let key_range = VGETRANDOM_BATCH_SIZE..VGETRANDOM_BATCH_KEY_SIZE;
            if backend.getrandom_syscall(&mut state.batch_key[key_range], 0)
                != CHACHA_KEY_SIZE as isize
            {
                state.generation = 0;
                return fallback_syscall(orig_buffer, flags, Some(state), backend);
            }
            state.pos = VGETRANDOM_BATCH_SIZE as u8;
        }

        let mut out_pos = 0usize;
        let mut remaining = ret;
        loop {
            let pos = core::cmp::min(state.pos as usize, VGETRANDOM_BATCH_SIZE);
            let batch_len = core::cmp::min(VGETRANDOM_BATCH_SIZE - pos, remaining);
            if batch_len != 0 {
                memcpy_and_zero_src(
                    &mut orig_buffer[out_pos..out_pos + batch_len],
                    &mut state.batch_key[pos..pos + batch_len],
                );
                state.pos = (pos + batch_len) as u8;
                out_pos += batch_len;
                remaining -= batch_len;
            }

            if remaining == 0 {
                if state.generation != rng_info.generation {
                    if have_retried {
                        return fallback_syscall(orig_buffer, flags, Some(state), backend);
                    }
                    have_retried = true;
                    continue 'retry_generation;
                }
                state.in_use = false;
                return ret as isize;
            }

            let nblocks = remaining / CHACHA_BLOCK_SIZE;
            if nblocks != 0 {
                let byte_len = nblocks * CHACHA_BLOCK_SIZE;
                let mut key = [0u8; CHACHA_KEY_SIZE];
                key.copy_from_slice(&state.batch_key[VGETRANDOM_BATCH_SIZE..]);
                backend.chacha20_blocks_nostack(
                    &mut orig_buffer[out_pos..out_pos + byte_len],
                    &key,
                    &mut counter,
                    nblocks,
                );
                out_pos += byte_len;
                remaining -= byte_len;
            }

            let mut key = [0u8; CHACHA_KEY_SIZE];
            key.copy_from_slice(&state.batch_key[VGETRANDOM_BATCH_SIZE..]);
            backend.chacha20_blocks_nostack(
                &mut state.batch_key,
                &key,
                &mut counter,
                VGETRANDOM_BATCH_KEY_SIZE / CHACHA_BLOCK_SIZE,
            );
            state.pos = 0;
        }
    }
}

pub fn __cvdso_getrandom<B: VdsoGetrandomBackend>(
    rng_info: &VdsoRngData,
    buffer: &mut [u8],
    flags: u32,
    state_address: usize,
    state: &mut VgetrandomState,
    opaque_len: usize,
    backend: &mut B,
) -> isize {
    __cvdso_getrandom_data(
        rng_info,
        Some(buffer),
        flags,
        CvdsoOpaqueState::State {
            address: state_address,
            state,
        },
        opaque_len,
        backend,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DeterministicBackend {
        syscall_calls: usize,
        block_calls: usize,
    }

    impl DeterministicBackend {
        fn new() -> Self {
            Self {
                syscall_calls: 0,
                block_calls: 0,
            }
        }
    }

    impl VdsoGetrandomBackend for DeterministicBackend {
        fn getrandom_syscall(&mut self, buffer: &mut [u8], _flags: u32) -> isize {
            self.syscall_calls += 1;
            for (index, byte) in buffer.iter_mut().enumerate() {
                *byte = 0xa0u8.wrapping_add(index as u8);
            }
            buffer.len() as isize
        }

        fn chacha20_blocks_nostack(
            &mut self,
            dst: &mut [u8],
            key: &[u8; CHACHA_KEY_SIZE],
            counter: &mut [u32; 2],
            nblocks: usize,
        ) {
            self.block_calls += 1;
            for index in 0..(nblocks * CHACHA_BLOCK_SIZE) {
                dst[index] = key[index % key.len()]
                    .wrapping_add(counter[0] as u8)
                    .wrapping_add((index / CHACHA_BLOCK_SIZE) as u8);
            }
            counter[0] = counter[0].wrapping_add(nblocks as u32);
        }
    }

    #[test]
    fn vdso_getrandom_matches_linux_source_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/vdso/getrandom.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/vdso/getrandom.h"
        ));
        assert!(source.contains("MEMCPY_AND_ZERO_SRC"));
        assert!(source.contains("__cvdso_getrandom_data"));
        assert!(source.contains("opaque_len == ~0UL && !buffer && !len && !flags"));
        assert!(source.contains("return -EFAULT;"));
        assert!(source.contains("flags & ~(GRND_NONBLOCK | GRND_RANDOM | GRND_INSECURE)"));
        assert!(source.contains("getrandom_syscall(state->key, sizeof(state->key), 0)"));
        assert!(source.contains("__arch_chacha20_blocks_nostack"));
        assert!(source.contains("return getrandom_syscall(orig_buffer, orig_len, flags);"));
        assert!(header.contains("struct vgetrandom_state"));
        assert!(header.contains("u8\tbatch[CHACHA_BLOCK_SIZE * 3 / 2]"));
    }

    #[test]
    fn query_params_match_linux_mmap_contract() {
        let rng = VdsoRngData {
            is_ready: true,
            generation: 1,
        };
        let mut params = VgetrandomOpaqueParams {
            reserved: [7; 13],
            ..Default::default()
        };
        let mut backend = DeterministicBackend::new();
        assert_eq!(
            __cvdso_getrandom_data(
                &rng,
                None,
                0,
                CvdsoOpaqueState::Params(&mut params),
                usize::MAX,
                &mut backend
            ),
            0
        );
        assert_eq!(
            params.size_of_opaque_state,
            core::mem::size_of::<VgetrandomState>() as u32
        );
        assert_eq!(params.mmap_prot, PROT_READ | PROT_WRITE);
        assert_eq!(params.mmap_flags, MAP_DROPPABLE | MAP_ANONYMOUS);
        assert_eq!(params.reserved, [0; 13]);
    }

    #[test]
    fn state_guards_and_fallbacks_follow_cvdso_order() {
        let rng = VdsoRngData {
            is_ready: true,
            generation: 9,
        };
        let mut backend = DeterministicBackend::new();
        let mut state = VgetrandomState::default();
        let mut out = [0u8; 16];
        assert_eq!(
            __cvdso_getrandom(
                &rng,
                &mut out,
                GRND_ALLOWED << 1,
                0,
                &mut state,
                core::mem::size_of::<VgetrandomState>(),
                &mut backend
            ),
            16
        );
        assert_eq!(backend.syscall_calls, 1);

        let mut backend = DeterministicBackend::new();
        assert_eq!(
            __cvdso_getrandom(
                &rng,
                &mut out,
                0,
                PAGE_SIZE - core::mem::size_of::<VgetrandomState>() + 1,
                &mut state,
                core::mem::size_of::<VgetrandomState>(),
                &mut backend
            ),
            -(EFAULT as isize)
        );
    }

    #[test]
    fn ready_state_uses_batch_and_erases_copied_bytes() {
        let rng = VdsoRngData {
            is_ready: true,
            generation: 42,
        };
        let mut backend = DeterministicBackend::new();
        let mut state = VgetrandomState::default();
        let mut out = [0u8; 24];
        assert_eq!(
            __cvdso_getrandom(
                &rng,
                &mut out,
                0,
                0,
                &mut state,
                core::mem::size_of::<VgetrandomState>(),
                &mut backend
            ),
            24
        );
        assert_eq!(state.generation, 42);
        assert!(!state.in_use);
        assert_eq!(state.pos, 24);
        assert_eq!(backend.syscall_calls, 1);
        assert!(backend.block_calls >= 1);
        assert!(out.iter().any(|byte| *byte != 0));
        assert!(state.batch_key[..24].iter().all(|byte| *byte == 0));
    }
}
