//! linux-parity: complete
//! linux-source: vendor/linux/security/selinux/ss/context.c
//! test-origin: linux:vendor/linux/security/selinux/ss/context.c
//! SELinux security context hashing.

pub const EBITMAP_UNIT_NUMS: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EbitmapNode {
    pub startbit: u32,
    pub maps: [u64; EBITMAP_UNIT_NUMS],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ebitmap<'a> {
    pub highbit: u32,
    pub nodes: &'a [EbitmapNode],
}

impl Ebitmap<'_> {
    pub const fn empty() -> Self {
        Self {
            highbit: 0,
            nodes: &[],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MlsLevel<'a> {
    pub sens: u32,
    pub cat: Ebitmap<'a>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MlsRange<'a> {
    pub low: MlsLevel<'a>,
    pub high: MlsLevel<'a>,
}

impl MlsRange<'_> {
    pub const fn empty() -> Self {
        Self {
            low: MlsLevel {
                sens: 0,
                cat: Ebitmap::empty(),
            },
            high: MlsLevel {
                sens: 0,
                cat: Ebitmap::empty(),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SecurityContext<'a> {
    pub user: u32,
    pub role: u32,
    pub type_id: u32,
    pub len: u32,
    pub str_bytes: &'a [u8],
    pub range: MlsRange<'a>,
}

impl<'a> SecurityContext<'a> {
    pub const fn valid(user: u32, role: u32, type_id: u32, range: MlsRange<'a>) -> Self {
        Self {
            user,
            role,
            type_id,
            len: 0,
            str_bytes: &[],
            range,
        }
    }

    pub fn invalid(str_bytes: &'a [u8]) -> Self {
        Self {
            user: 0,
            role: 0,
            type_id: 0,
            len: str_bytes.len() as u32,
            str_bytes,
            range: MlsRange::empty(),
        }
    }
}

pub fn context_compute_hash(context: &SecurityContext<'_>) -> u32 {
    if context.len != 0 {
        let len = (context.len as usize).min(context.str_bytes.len());
        return full_name_hash(&context.str_bytes[..len]);
    }

    let mut hash = jhash_3words(context.user, context.role, context.type_id, 0);
    hash = mls_range_hash(&context.range, hash);
    hash
}

pub fn mls_range_hash(range: &MlsRange<'_>, hash: u32) -> u32 {
    let mut hash = jhash_2words(range.low.sens, range.high.sens, hash);
    hash = ebitmap_hash(&range.low.cat, hash);
    ebitmap_hash(&range.high.cat, hash)
}

pub fn ebitmap_hash(ebitmap: &Ebitmap<'_>, hash: u32) -> u32 {
    let mut hash = jhash_1word(ebitmap.highbit, hash);
    for node in ebitmap.nodes {
        hash = jhash_1word(node.startbit, hash);
        let mut bytes = [0u8; EBITMAP_UNIT_NUMS * core::mem::size_of::<u64>()];
        for (index, map) in node.maps.iter().enumerate() {
            let start = index * core::mem::size_of::<u64>();
            bytes[start..start + core::mem::size_of::<u64>()].copy_from_slice(&map.to_ne_bytes());
        }
        hash = jhash(&bytes, hash);
    }
    hash
}

pub fn full_name_hash(name: &[u8]) -> u32 {
    let mut hash = 0u64;
    for byte in name {
        hash = partial_name_hash(*byte as u64, hash);
    }
    hash_64(hash, 32)
}

fn partial_name_hash(c: u64, prev_hash: u64) -> u64 {
    prev_hash
        .wrapping_add(c.wrapping_shl(4).wrapping_add(c.wrapping_shr(4)))
        .wrapping_mul(11)
}

fn hash_64(value: u64, bits: u32) -> u32 {
    const GOLDEN_RATIO_64: u64 = 0x61c8_8646_80b5_83eb;
    value.wrapping_mul(GOLDEN_RATIO_64).wrapping_shr(64 - bits) as u32
}

pub fn jhash_3words(a: u32, b: u32, c: u32, initval: u32) -> u32 {
    jhash_nwords(
        a,
        b,
        c,
        initval.wrapping_add(JHASH_INITVAL).wrapping_add(3 << 2),
    )
}

pub fn jhash_2words(a: u32, b: u32, initval: u32) -> u32 {
    jhash_nwords(
        a,
        b,
        0,
        initval.wrapping_add(JHASH_INITVAL).wrapping_add(2 << 2),
    )
}

pub fn jhash_1word(a: u32, initval: u32) -> u32 {
    jhash_nwords(
        a,
        0,
        0,
        initval.wrapping_add(JHASH_INITVAL).wrapping_add(1 << 2),
    )
}

pub fn jhash(key: &[u8], initval: u32) -> u32 {
    let mut length = key.len();
    let mut offset = 0usize;
    let mut a = JHASH_INITVAL
        .wrapping_add(length as u32)
        .wrapping_add(initval);
    let mut b = a;
    let mut c = a;

    while length > 12 {
        a = a.wrapping_add(read_le_u32(key, offset));
        b = b.wrapping_add(read_le_u32(key, offset + 4));
        c = c.wrapping_add(read_le_u32(key, offset + 8));
        (a, b, c) = jhash_mix(a, b, c);
        offset += 12;
        length -= 12;
    }

    let tail = &key[offset..];
    if length >= 12 {
        c = c.wrapping_add((tail[11] as u32) << 24);
    }
    if length >= 11 {
        c = c.wrapping_add((tail[10] as u32) << 16);
    }
    if length >= 10 {
        c = c.wrapping_add((tail[9] as u32) << 8);
    }
    if length >= 9 {
        c = c.wrapping_add(tail[8] as u32);
    }
    if length >= 8 {
        b = b.wrapping_add((tail[7] as u32) << 24);
    }
    if length >= 7 {
        b = b.wrapping_add((tail[6] as u32) << 16);
    }
    if length >= 6 {
        b = b.wrapping_add((tail[5] as u32) << 8);
    }
    if length >= 5 {
        b = b.wrapping_add(tail[4] as u32);
    }
    if length >= 4 {
        a = a.wrapping_add((tail[3] as u32) << 24);
    }
    if length >= 3 {
        a = a.wrapping_add((tail[2] as u32) << 16);
    }
    if length >= 2 {
        a = a.wrapping_add((tail[1] as u32) << 8);
    }
    if length >= 1 {
        a = a.wrapping_add(tail[0] as u32);
        (_, _, c) = jhash_final(a, b, c);
    }

    c
}

const JHASH_INITVAL: u32 = 0xdead_beef;

fn read_le_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn jhash_nwords(a: u32, b: u32, c: u32, initval: u32) -> u32 {
    let a = a.wrapping_add(initval);
    let b = b.wrapping_add(initval);
    let c = c.wrapping_add(initval);
    let (_, _, c) = jhash_final(a, b, c);
    c
}

fn jhash_mix(mut a: u32, mut b: u32, mut c: u32) -> (u32, u32, u32) {
    a = a.wrapping_sub(c);
    a ^= c.rotate_left(4);
    c = c.wrapping_add(b);
    b = b.wrapping_sub(a);
    b ^= a.rotate_left(6);
    a = a.wrapping_add(c);
    c = c.wrapping_sub(b);
    c ^= b.rotate_left(8);
    b = b.wrapping_add(a);
    a = a.wrapping_sub(c);
    a ^= c.rotate_left(16);
    c = c.wrapping_add(b);
    b = b.wrapping_sub(a);
    b ^= a.rotate_left(19);
    a = a.wrapping_add(c);
    c = c.wrapping_sub(b);
    c ^= b.rotate_left(4);
    b = b.wrapping_add(a);
    (a, b, c)
}

fn jhash_final(mut a: u32, mut b: u32, mut c: u32) -> (u32, u32, u32) {
    c ^= b;
    c = c.wrapping_sub(b.rotate_left(14));
    a ^= c;
    a = a.wrapping_sub(c.rotate_left(11));
    b ^= a;
    b = b.wrapping_sub(a.rotate_left(25));
    c ^= b;
    c = c.wrapping_sub(b.rotate_left(16));
    a ^= c;
    a = a.wrapping_sub(c.rotate_left(4));
    b ^= a;
    b = b.wrapping_sub(a.rotate_left(14));
    c ^= b;
    c = c.wrapping_sub(b.rotate_left(24));
    (a, b, c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_compute_hash_matches_linux_source_shape() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/selinux/ss/context.c"
        ));
        assert!(source.contains("u32 context_compute_hash(const struct context *c)"));
        assert!(source.contains("return full_name_hash(NULL, c->str, c->len);"));
        assert!(source.contains("hash = jhash_3words(c->user, c->role, c->type, hash);"));
        assert!(source.contains("hash = mls_range_hash(&c->range, hash);"));

        let valid = SecurityContext::valid(1, 2, 3, MlsRange::empty());
        let expected = mls_range_hash(&MlsRange::empty(), jhash_3words(1, 2, 3, 0));
        assert_eq!(context_compute_hash(&valid), expected);

        let invalid = SecurityContext::invalid(b"user_u:role_r:type_t:s0");
        assert_eq!(
            context_compute_hash(&invalid),
            full_name_hash(b"user_u:role_r:type_t:s0")
        );
        assert_ne!(context_compute_hash(&invalid), context_compute_hash(&valid));
    }

    #[test]
    fn mls_range_hash_includes_sensitivity_and_category_nodes() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let node = EbitmapNode {
            startbit: 64,
            maps: [0b101, 0, 0, 0, 0, 0],
        };
        let nodes = [node];
        let categories = Ebitmap {
            highbit: 129,
            nodes: &nodes,
        };
        let range = MlsRange {
            low: MlsLevel {
                sens: 7,
                cat: categories,
            },
            high: MlsLevel {
                sens: 9,
                cat: Ebitmap::empty(),
            },
        };
        assert_ne!(
            mls_range_hash(&range, 0),
            mls_range_hash(&MlsRange::empty(), 0)
        );
    }
}
