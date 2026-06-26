//! linux-parity: complete
//! linux-source: vendor/linux/security/integrity/platform_certs/efi_parser.c
//! test-origin: linux:vendor/linux/security/integrity/platform_certs/efi_parser.c
//! EFI signature-list parser.

extern crate alloc;

use alloc::vec::Vec;

pub const EBADMSG: i32 = 74;
pub const EFI_SIGNATURE_LIST_SIZE: usize = 28;
pub const EFI_SIGNATURE_DATA_HEADER_SIZE: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EfiSignatureListHeader {
    pub signature_type: [u8; 16],
    pub signature_list_size: u32,
    pub signature_header_size: u32,
    pub signature_size: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EfiSignatureElement<'a> {
    pub signature_type: [u8; 16],
    pub owner: [u8; 16],
    pub data: &'a [u8],
}

pub fn parse_efi_signature_list<'a>(
    mut data: &'a [u8],
    mut handler_accepts_guid: impl FnMut(&[u8; 16]) -> bool,
) -> Result<Vec<EfiSignatureElement<'a>>, i32> {
    let mut out = Vec::new();

    while !data.is_empty() {
        if data.len() < EFI_SIGNATURE_LIST_SIZE {
            return Err(-EBADMSG);
        }
        let list = read_header(data).ok_or(-EBADMSG)?;
        let lsize = list.signature_list_size as usize;
        let hsize = list.signature_header_size as usize;
        let esize = list.signature_size as usize;

        if lsize > data.len()
            || lsize < EFI_SIGNATURE_LIST_SIZE
            || lsize - EFI_SIGNATURE_LIST_SIZE < hsize
            || esize < EFI_SIGNATURE_DATA_HEADER_SIZE
        {
            return Err(-EBADMSG);
        }
        let elsize = lsize - EFI_SIGNATURE_LIST_SIZE - hsize;
        if elsize < esize || elsize % esize != 0 {
            return Err(-EBADMSG);
        }

        if handler_accepts_guid(&list.signature_type) {
            let mut elements = &data[EFI_SIGNATURE_LIST_SIZE + hsize..lsize];
            while !elements.is_empty() {
                let owner: [u8; 16] = elements[..16].try_into().map_err(|_| -EBADMSG)?;
                out.push(EfiSignatureElement {
                    signature_type: list.signature_type,
                    owner,
                    data: &elements[16..esize],
                });
                elements = &elements[esize..];
            }
        }

        data = &data[lsize..];
    }

    Ok(out)
}

fn read_header(data: &[u8]) -> Option<EfiSignatureListHeader> {
    Some(EfiSignatureListHeader {
        signature_type: data.get(0..16)?.try_into().ok()?,
        signature_list_size: u32::from_le_bytes(data.get(16..20)?.try_into().ok()?),
        signature_header_size: u32::from_le_bytes(data.get(20..24)?.try_into().ok()?),
        signature_size: u32::from_le_bytes(data.get(24..28)?.try_into().ok()?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list_blob(guid: [u8; 16], header_size: u32, elem_payload: &[u8]) -> Vec<u8> {
        let signature_size = (EFI_SIGNATURE_DATA_HEADER_SIZE + elem_payload.len()) as u32;
        let list_size = EFI_SIGNATURE_LIST_SIZE as u32 + header_size + signature_size;
        let mut data = Vec::new();
        data.extend_from_slice(&guid);
        data.extend_from_slice(&list_size.to_le_bytes());
        data.extend_from_slice(&header_size.to_le_bytes());
        data.extend_from_slice(&signature_size.to_le_bytes());
        data.resize(data.len() + header_size as usize, 0);
        data.extend_from_slice(&[0x5a; 16]);
        data.extend_from_slice(elem_payload);
        data
    }

    #[test]
    fn efi_signature_list_parser_matches_linux_size_checks_and_handler_flow() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/integrity/platform_certs/efi_parser.c"
        ));
        assert!(source.contains("parse_efi_signature_list"));
        assert!(source.contains("if (size < sizeof(list))"));
        assert!(source.contains("return -EBADMSG;"));
        assert!(source.contains("memcpy(&list, data, sizeof(list));"));
        assert!(source.contains("lsize = list.signature_list_size;"));
        assert!(source.contains("hsize = list.signature_header_size;"));
        assert!(source.contains("esize = list.signature_size;"));
        assert!(source.contains("elsize = lsize - sizeof(list) - hsize;"));
        assert!(source.contains("lsize > size"));
        assert!(source.contains("elsize % esize != 0"));
        assert!(source.contains("handler = get_handler_for_guid(&list.signature_type);"));
        assert!(source.contains("handler(source,"));
        assert!(source.contains("esize - sizeof(*elem)"));

        let guid = [1u8; 16];
        let blob = list_blob(guid, 4, &[9, 8, 7]);
        let parsed = parse_efi_signature_list(&blob, |candidate| candidate == &guid)
            .expect("valid signature list");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].signature_type, guid);
        assert_eq!(parsed[0].owner, [0x5a; 16]);
        assert_eq!(parsed[0].data, &[9, 8, 7]);
        assert!(
            parse_efi_signature_list(&blob, |_| false)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            parse_efi_signature_list(&blob[..8], |_| true),
            Err(-EBADMSG)
        );
    }
}
