//! linux-parity: partial
//! linux-source: vendor/linux/lib/crypto/mpi
//! Multiple-precision integer helpers.

pub mod generic_mpih_add1;
pub mod generic_mpih_lshift;
pub mod generic_mpih_mul1;
pub mod generic_mpih_mul2;
pub mod generic_mpih_mul3;
pub mod generic_mpih_rshift;
pub mod generic_mpih_sub1;
pub mod mpi_bit;
pub mod mpi_cmp;
pub mod mpi_mod;
pub mod mpi_mul;
pub mod mpi_sub_ui;
pub mod mpih_cmp;

pub fn register_module_exports() {
    mpi_bit::register_module_exports();
    mpi_cmp::register_module_exports();
    mpi_mul::register_module_exports();
    mpi_sub_ui::register_module_exports();
}
