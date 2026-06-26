//! linux-parity: complete
//! linux-source: vendor/linux/lib/math
//! Linux math helper exports.

pub mod cordic;
pub mod gcd;
pub mod int_log;
pub mod int_pow;
pub mod int_sqrt;
pub mod lcm;
pub mod polynomial;
pub mod prime_numbers;
pub mod rational;
pub mod reciprocal_div;
pub mod test_div64;
pub mod test_mul_u64_u64_div_u64;
pub mod tests;

pub fn register_module_exports() {
    cordic::register_module_exports();
    gcd::register_module_exports();
    int_log::register_module_exports();
    int_pow::register_module_exports();
    int_sqrt::register_module_exports();
    lcm::register_module_exports();
    polynomial::register_module_exports();
    prime_numbers::register_module_exports();
    rational::register_module_exports();
    reciprocal_div::register_module_exports();
}
