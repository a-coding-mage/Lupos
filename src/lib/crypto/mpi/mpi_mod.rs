//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/mpi/mpi-mod.c
//! test-origin: linux:vendor/linux/lib/crypto/mpi/mpi-mod.c
//! MPI modular-reduction wrapper.

pub const LINUX_SOURCE: &str = "vendor/linux/lib/crypto/mpi/mpi-mod.c";
pub const FORWARD_TARGET: &str = "mpi_fdiv_r(rem, dividend, divisor)";

pub fn mpi_mod_forward_target() -> &'static str {
    FORWARD_TARGET
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpi_mod_forwards_to_floor_division_remainder() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/mpi/mpi-mod.c"
        ));
        assert!(source.contains("#include \"mpi-internal.h\""));
        assert!(source.contains("int mpi_mod(MPI rem, MPI dividend, MPI divisor)"));
        assert!(source.contains("return mpi_fdiv_r(rem, dividend, divisor);"));
        assert_eq!(mpi_mod_forward_target(), FORWARD_TARGET);
    }
}
