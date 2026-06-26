//! linux-parity: partial
//! linux-source: vendor/linux/lib/crypto
//! Generic crypto library helpers.

pub mod aes;
pub mod aescfb;
pub mod aesgcm;
pub mod arc4;
pub mod blake2b;
pub mod blake2s;
pub mod chacha;
pub mod chacha20poly1305;
pub mod chacha_block_generic;
pub mod curve25519;
pub mod gf128mul;
pub mod hash_info;
pub mod md5;
pub mod memneq;
pub mod mpi;
pub mod nh;
pub mod poly1305;
pub mod sha1;
pub mod sha256;
pub mod sha512;
pub mod simd;
pub mod sm3;
pub mod tests;
pub mod utils;

pub fn register_module_exports() {
    arc4::register_module_exports();
    aes::register_module_exports();
    aescfb::register_module_exports();
    aesgcm::register_module_exports();
    blake2b::register_module_exports();
    blake2s::register_module_exports();
    chacha::register_module_exports();
    chacha_block_generic::register_module_exports();
    chacha20poly1305::register_module_exports();
    curve25519::register_module_exports();
    gf128mul::register_module_exports();
    hash_info::register_module_exports();
    memneq::register_module_exports();
    md5::register_module_exports();
    mpi::register_module_exports();
    nh::register_module_exports();
    poly1305::register_module_exports();
    sha1::register_module_exports();
    sha256::register_module_exports();
    sha512::register_module_exports();
    utils::register_module_exports();
    sm3::register_module_exports();
}
