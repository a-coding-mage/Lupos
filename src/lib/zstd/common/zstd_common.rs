//! linux-parity: complete
//! linux-source: vendor/linux/lib/zstd/common/zstd_common.c
//! test-origin: linux:vendor/linux/lib/zstd/common/zstd_common.c
//! Zstd common version and error-code helpers.

pub const ZSTD_VERSION_MAJOR: u32 = 1;
pub const ZSTD_VERSION_MINOR: u32 = 5;
pub const ZSTD_VERSION_RELEASE: u32 = 7;
pub const ZSTD_VERSION_NUMBER: u32 =
    ZSTD_VERSION_MAJOR * 100 * 100 + ZSTD_VERSION_MINOR * 100 + ZSTD_VERSION_RELEASE;
pub const ZSTD_VERSION_STRING: &str = "1.5.7";

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZstdErrorCode {
    NoError = 0,
    Generic = 1,
    PrefixUnknown = 10,
    VersionUnsupported = 12,
    FrameParameterUnsupported = 14,
    FrameParameterWindowTooLarge = 16,
    CorruptionDetected = 20,
    ChecksumWrong = 22,
    LiteralsHeaderWrong = 24,
    DictionaryCorrupted = 30,
    DictionaryWrong = 32,
    DictionaryCreationFailed = 34,
    ParameterUnsupported = 40,
    ParameterCombinationUnsupported = 41,
    ParameterOutOfBound = 42,
    TableLogTooLarge = 44,
    MaxSymbolValueTooLarge = 46,
    MaxSymbolValueTooSmall = 48,
    CannotProduceUncompressedBlock = 49,
    StabilityConditionNotRespected = 50,
    StageWrong = 60,
    InitMissing = 62,
    MemoryAllocation = 64,
    WorkSpaceTooSmall = 66,
    DstSizeTooSmall = 70,
    SrcSizeWrong = 72,
    DstBufferNull = 74,
    NoForwardProgressDestFull = 80,
    NoForwardProgressInputEmpty = 82,
    FrameIndexTooLarge = 100,
    SeekableIo = 102,
    DstBufferWrong = 104,
    SrcBufferWrong = 105,
    SequenceProducerFailed = 106,
    ExternalSequencesInvalid = 107,
    MaxCode = 120,
}

pub const fn zstd_error(code: ZstdErrorCode) -> usize {
    0usize.wrapping_sub(code as usize)
}

pub const fn zstd_version_number() -> u32 {
    ZSTD_VERSION_NUMBER
}

pub const fn zstd_version_string() -> &'static str {
    ZSTD_VERSION_STRING
}

pub const fn zstd_is_error(code: usize) -> u32 {
    (code > zstd_error(ZstdErrorCode::MaxCode)) as u32
}

pub const fn zstd_get_error_code(code: usize) -> ZstdErrorCode {
    if zstd_is_error(code) == 0 {
        return ZstdErrorCode::NoError;
    }

    match 0usize.wrapping_sub(code) as u32 {
        1 => ZstdErrorCode::Generic,
        10 => ZstdErrorCode::PrefixUnknown,
        12 => ZstdErrorCode::VersionUnsupported,
        14 => ZstdErrorCode::FrameParameterUnsupported,
        16 => ZstdErrorCode::FrameParameterWindowTooLarge,
        20 => ZstdErrorCode::CorruptionDetected,
        22 => ZstdErrorCode::ChecksumWrong,
        24 => ZstdErrorCode::LiteralsHeaderWrong,
        30 => ZstdErrorCode::DictionaryCorrupted,
        32 => ZstdErrorCode::DictionaryWrong,
        34 => ZstdErrorCode::DictionaryCreationFailed,
        40 => ZstdErrorCode::ParameterUnsupported,
        41 => ZstdErrorCode::ParameterCombinationUnsupported,
        42 => ZstdErrorCode::ParameterOutOfBound,
        44 => ZstdErrorCode::TableLogTooLarge,
        46 => ZstdErrorCode::MaxSymbolValueTooLarge,
        48 => ZstdErrorCode::MaxSymbolValueTooSmall,
        49 => ZstdErrorCode::CannotProduceUncompressedBlock,
        50 => ZstdErrorCode::StabilityConditionNotRespected,
        60 => ZstdErrorCode::StageWrong,
        62 => ZstdErrorCode::InitMissing,
        64 => ZstdErrorCode::MemoryAllocation,
        66 => ZstdErrorCode::WorkSpaceTooSmall,
        70 => ZstdErrorCode::DstSizeTooSmall,
        72 => ZstdErrorCode::SrcSizeWrong,
        74 => ZstdErrorCode::DstBufferNull,
        80 => ZstdErrorCode::NoForwardProgressDestFull,
        82 => ZstdErrorCode::NoForwardProgressInputEmpty,
        100 => ZstdErrorCode::FrameIndexTooLarge,
        102 => ZstdErrorCode::SeekableIo,
        104 => ZstdErrorCode::DstBufferWrong,
        105 => ZstdErrorCode::SrcBufferWrong,
        106 => ZstdErrorCode::SequenceProducerFailed,
        107 => ZstdErrorCode::ExternalSequencesInvalid,
        _ => ZstdErrorCode::MaxCode,
    }
}

pub const fn zstd_get_error_string(code: ZstdErrorCode) -> &'static str {
    match code {
        ZstdErrorCode::NoError => "No error detected",
        ZstdErrorCode::Generic => "Error (generic)",
        ZstdErrorCode::PrefixUnknown => "Unknown frame descriptor",
        ZstdErrorCode::VersionUnsupported => "Version not supported",
        ZstdErrorCode::FrameParameterUnsupported => "Unsupported frame parameter",
        ZstdErrorCode::FrameParameterWindowTooLarge => {
            "Frame requires too much memory for decoding"
        }
        ZstdErrorCode::CorruptionDetected => "Data corruption detected",
        ZstdErrorCode::ChecksumWrong => "Restored data doesn't match checksum",
        ZstdErrorCode::LiteralsHeaderWrong => {
            "Header of Literals' block doesn't respect format specification"
        }
        ZstdErrorCode::DictionaryCorrupted => "Dictionary is corrupted",
        ZstdErrorCode::DictionaryWrong => "Dictionary mismatch",
        ZstdErrorCode::DictionaryCreationFailed => "Cannot create Dictionary from provided samples",
        ZstdErrorCode::ParameterUnsupported => "Unsupported parameter",
        ZstdErrorCode::ParameterCombinationUnsupported => "Unsupported combination of parameters",
        ZstdErrorCode::ParameterOutOfBound => "Parameter is out of bound",
        ZstdErrorCode::TableLogTooLarge => "tableLog requires too much memory : unsupported",
        ZstdErrorCode::MaxSymbolValueTooLarge => "Unsupported max Symbol Value : too large",
        ZstdErrorCode::MaxSymbolValueTooSmall => "Specified maxSymbolValue is too small",
        ZstdErrorCode::CannotProduceUncompressedBlock => {
            "This mode cannot generate an uncompressed block"
        }
        ZstdErrorCode::StabilityConditionNotRespected => {
            "pledged buffer stability condition is not respected"
        }
        ZstdErrorCode::StageWrong => "Operation not authorized at current processing stage",
        ZstdErrorCode::InitMissing => "Context should be init first",
        ZstdErrorCode::MemoryAllocation => "Allocation error : not enough memory",
        ZstdErrorCode::WorkSpaceTooSmall => "workSpace buffer is not large enough",
        ZstdErrorCode::DstSizeTooSmall => "Destination buffer is too small",
        ZstdErrorCode::SrcSizeWrong => "Src size is incorrect",
        ZstdErrorCode::DstBufferNull => "Operation on NULL destination buffer",
        ZstdErrorCode::NoForwardProgressDestFull => {
            "Operation made no progress over multiple calls, due to output buffer being full"
        }
        ZstdErrorCode::NoForwardProgressInputEmpty => {
            "Operation made no progress over multiple calls, due to input being empty"
        }
        ZstdErrorCode::FrameIndexTooLarge => "Frame index is too large",
        ZstdErrorCode::SeekableIo => "An I/O error occurred when reading/seeking",
        ZstdErrorCode::DstBufferWrong => "Destination buffer is wrong",
        ZstdErrorCode::SrcBufferWrong => "Source buffer is wrong",
        ZstdErrorCode::SequenceProducerFailed => {
            "Block-level external sequence producer returned an error code"
        }
        ZstdErrorCode::ExternalSequencesInvalid => "External sequences are not valid",
        ZstdErrorCode::MaxCode => "Unspecified error code",
    }
}

pub const fn zstd_get_error_name(code: usize) -> &'static str {
    zstd_get_error_string(zstd_get_error_code(code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zstd_common_matches_linux_version_and_error_wrappers() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/zstd/common/zstd_common.c"
        ));
        assert!(source.contains("unsigned ZSTD_versionNumber(void)"));
        assert!(source.contains("return ZSTD_VERSION_NUMBER;"));
        assert!(source.contains("const char* ZSTD_versionString(void)"));
        assert!(source.contains("return ZSTD_VERSION_STRING;"));
        assert!(source.contains("unsigned ZSTD_isError(size_t code)"));
        assert!(source.contains("return ERR_isError(code);"));
        assert!(source.contains("ZSTD_ErrorCode ZSTD_getErrorCode(size_t code)"));
        assert!(source.contains("const char* ZSTD_getErrorString(ZSTD_ErrorCode code)"));

        assert_eq!(zstd_version_number(), 10507);
        assert_eq!(zstd_version_string(), "1.5.7");
        let memory_error = zstd_error(ZstdErrorCode::MemoryAllocation);
        assert_eq!(zstd_is_error(memory_error), 1);
        assert_eq!(
            zstd_get_error_code(memory_error),
            ZstdErrorCode::MemoryAllocation
        );
        assert_eq!(
            zstd_get_error_name(memory_error),
            "Allocation error : not enough memory"
        );
        assert_eq!(zstd_is_error(123), 0);
        assert_eq!(zstd_get_error_code(123), ZstdErrorCode::NoError);
    }
}
