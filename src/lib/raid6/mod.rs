//! linux-parity: partial
//! linux-source: vendor/linux/lib/raid6
//! RAID6 implementation source coverage.

pub mod neon;
pub mod recov;
pub mod recov_neon;
pub mod recov_neon_inner;
pub mod recov_s390xc;
