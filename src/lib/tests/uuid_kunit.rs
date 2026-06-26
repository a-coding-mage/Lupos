//! linux-parity: complete
//! linux-source: vendor/linux/lib/tests/uuid_kunit.c
//! test-origin: linux:vendor/linux/lib/tests/uuid_kunit.c
//! Source-backed UUID/GUID parse KUnit vectors.

use crate::include::uapi::errno::EINVAL;

pub type UuidBytes = [u8; 16];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UuidTestData {
    pub uuid: &'static str,
    pub guid_le: UuidBytes,
    pub uuid_be: UuidBytes,
}

pub const UUID_TEST_DATA: &[UuidTestData] = &[
    UuidTestData {
        uuid: "c33f4995-3701-450e-9fbf-206a2e98e576",
        guid_le: [
            0x95, 0x49, 0x3f, 0xc3, 0x01, 0x37, 0x0e, 0x45, 0x9f, 0xbf, 0x20, 0x6a, 0x2e, 0x98,
            0xe5, 0x76,
        ],
        uuid_be: [
            0xc3, 0x3f, 0x49, 0x95, 0x37, 0x01, 0x45, 0x0e, 0x9f, 0xbf, 0x20, 0x6a, 0x2e, 0x98,
            0xe5, 0x76,
        ],
    },
    UuidTestData {
        uuid: "64b4371c-77c1-48f9-8221-29f054fc023b",
        guid_le: [
            0x1c, 0x37, 0xb4, 0x64, 0xc1, 0x77, 0xf9, 0x48, 0x82, 0x21, 0x29, 0xf0, 0x54, 0xfc,
            0x02, 0x3b,
        ],
        uuid_be: [
            0x64, 0xb4, 0x37, 0x1c, 0x77, 0xc1, 0x48, 0xf9, 0x82, 0x21, 0x29, 0xf0, 0x54, 0xfc,
            0x02, 0x3b,
        ],
    },
    UuidTestData {
        uuid: "0cb4ddff-a545-4401-9d06-688af53e7f84",
        guid_le: [
            0xff, 0xdd, 0xb4, 0x0c, 0x45, 0xa5, 0x01, 0x44, 0x9d, 0x06, 0x68, 0x8a, 0xf5, 0x3e,
            0x7f, 0x84,
        ],
        uuid_be: [
            0x0c, 0xb4, 0xdd, 0xff, 0xa5, 0x45, 0x44, 0x01, 0x9d, 0x06, 0x68, 0x8a, 0xf5, 0x3e,
            0x7f, 0x84,
        ],
    },
];

pub const UUID_WRONG_DATA: &[&str] = &[
    "c33f4995-3701-450e-9fbf206a2e98e576 ",
    "64b4371c-77c1-48f9-8221-29f054XX023b",
    "0cb4ddff-a545-4401-9d06-688af53e",
];

pub fn uuid_parse(input: &str) -> Result<UuidBytes, i32> {
    parse_uuid_be(input)
}

pub fn guid_parse(input: &str) -> Result<UuidBytes, i32> {
    let be = parse_uuid_be(input)?;
    Ok([
        be[3], be[2], be[1], be[0], be[5], be[4], be[7], be[6], be[8], be[9], be[10], be[11],
        be[12], be[13], be[14], be[15],
    ])
}

fn parse_uuid_be(input: &str) -> Result<UuidBytes, i32> {
    let bytes = input.as_bytes();
    if bytes.len() != 36 {
        return Err(-EINVAL);
    }
    for index in [8usize, 13, 18, 23] {
        if bytes[index] != b'-' {
            return Err(-EINVAL);
        }
    }

    let mut out = [0u8; 16];
    let mut src = 0usize;
    let mut dst = 0usize;
    while src < bytes.len() {
        if bytes[src] == b'-' {
            src += 1;
            continue;
        }
        let hi = hex_value(bytes[src])?;
        let lo = hex_value(bytes[src + 1])?;
        out[dst] = (hi << 4) | lo;
        dst += 1;
        src += 2;
    }
    if dst == 16 { Ok(out) } else { Err(-EINVAL) }
}

fn hex_value(byte: u8) -> Result<u8, i32> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(-EINVAL),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_kunit_vectors_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/tests/uuid_kunit.c"
        ));
        assert!(source.contains("struct test_uuid_data"));
        assert!(source.contains("GUID_INIT(0xc33f4995, 0x3701, 0x450e"));
        assert!(source.contains("UUID_INIT(0x64b4371c, 0x77c1, 0x48f9"));
        assert!(source.contains("test_uuid_wrong_data[]"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, guid_parse(uuid, &le), -EINVAL);"));
        assert!(source.contains("KUNIT_CASE(uuid_test_uuid_invalid)"));
        assert!(source.contains(".name = \"uuid\""));
        assert_eq!(UUID_TEST_DATA.len(), 3);
        assert_eq!(UUID_WRONG_DATA.len(), 3);

        for data in UUID_TEST_DATA {
            assert_eq!(guid_parse(data.uuid), Ok(data.guid_le));
            assert_eq!(uuid_parse(data.uuid), Ok(data.uuid_be));
        }
        for uuid in UUID_WRONG_DATA {
            assert_eq!(guid_parse(uuid), Err(-EINVAL));
            assert_eq!(uuid_parse(uuid), Err(-EINVAL));
        }
    }
}
