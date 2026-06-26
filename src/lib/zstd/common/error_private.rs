//! linux-parity: complete
//! linux-source: vendor/linux/lib/zstd/common/error_private.c
//! test-origin: linux:vendor/linux/lib/zstd/common/error_private.c
//! Zstd private error string table.

use super::zstd_common::ZstdErrorCode;

pub const STRIPPED_ERROR_STRING: &str = "Error strings stripped";
pub const NOT_ERROR_CODE: &str = "Unspecified error code";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ZstdErrorString {
    pub code: ZstdErrorCode,
    pub linux_case: &'static str,
    pub message: &'static str,
    pub stable: bool,
}

pub const ERR_ERROR_STRINGS: &[ZstdErrorString] = &[
    ZstdErrorString {
        code: ZstdErrorCode::NoError,
        linux_case: "PREFIX(no_error)",
        message: "No error detected",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::Generic,
        linux_case: "PREFIX(GENERIC)",
        message: "Error (generic)",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::PrefixUnknown,
        linux_case: "PREFIX(prefix_unknown)",
        message: "Unknown frame descriptor",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::VersionUnsupported,
        linux_case: "PREFIX(version_unsupported)",
        message: "Version not supported",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::FrameParameterUnsupported,
        linux_case: "PREFIX(frameParameter_unsupported)",
        message: "Unsupported frame parameter",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::FrameParameterWindowTooLarge,
        linux_case: "PREFIX(frameParameter_windowTooLarge)",
        message: "Frame requires too much memory for decoding",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::CorruptionDetected,
        linux_case: "PREFIX(corruption_detected)",
        message: "Data corruption detected",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::ChecksumWrong,
        linux_case: "PREFIX(checksum_wrong)",
        message: "Restored data doesn't match checksum",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::LiteralsHeaderWrong,
        linux_case: "PREFIX(literals_headerWrong)",
        message: "Header of Literals' block doesn't respect format specification",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::ParameterUnsupported,
        linux_case: "PREFIX(parameter_unsupported)",
        message: "Unsupported parameter",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::ParameterCombinationUnsupported,
        linux_case: "PREFIX(parameter_combination_unsupported)",
        message: "Unsupported combination of parameters",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::ParameterOutOfBound,
        linux_case: "PREFIX(parameter_outOfBound)",
        message: "Parameter is out of bound",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::InitMissing,
        linux_case: "PREFIX(init_missing)",
        message: "Context should be init first",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::MemoryAllocation,
        linux_case: "PREFIX(memory_allocation)",
        message: "Allocation error : not enough memory",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::WorkSpaceTooSmall,
        linux_case: "PREFIX(workSpace_tooSmall)",
        message: "workSpace buffer is not large enough",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::StageWrong,
        linux_case: "PREFIX(stage_wrong)",
        message: "Operation not authorized at current processing stage",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::TableLogTooLarge,
        linux_case: "PREFIX(tableLog_tooLarge)",
        message: "tableLog requires too much memory : unsupported",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::MaxSymbolValueTooLarge,
        linux_case: "PREFIX(maxSymbolValue_tooLarge)",
        message: "Unsupported max Symbol Value : too large",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::MaxSymbolValueTooSmall,
        linux_case: "PREFIX(maxSymbolValue_tooSmall)",
        message: "Specified maxSymbolValue is too small",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::CannotProduceUncompressedBlock,
        linux_case: "PREFIX(cannotProduce_uncompressedBlock)",
        message: "This mode cannot generate an uncompressed block",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::StabilityConditionNotRespected,
        linux_case: "PREFIX(stabilityCondition_notRespected)",
        message: "pledged buffer stability condition is not respected",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::DictionaryCorrupted,
        linux_case: "PREFIX(dictionary_corrupted)",
        message: "Dictionary is corrupted",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::DictionaryWrong,
        linux_case: "PREFIX(dictionary_wrong)",
        message: "Dictionary mismatch",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::DictionaryCreationFailed,
        linux_case: "PREFIX(dictionaryCreation_failed)",
        message: "Cannot create Dictionary from provided samples",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::DstSizeTooSmall,
        linux_case: "PREFIX(dstSize_tooSmall)",
        message: "Destination buffer is too small",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::SrcSizeWrong,
        linux_case: "PREFIX(srcSize_wrong)",
        message: "Src size is incorrect",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::DstBufferNull,
        linux_case: "PREFIX(dstBuffer_null)",
        message: "Operation on NULL destination buffer",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::NoForwardProgressDestFull,
        linux_case: "PREFIX(noForwardProgress_destFull)",
        message: "Operation made no progress over multiple calls, due to output buffer being full",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::NoForwardProgressInputEmpty,
        linux_case: "PREFIX(noForwardProgress_inputEmpty)",
        message: "Operation made no progress over multiple calls, due to input being empty",
        stable: true,
    },
    ZstdErrorString {
        code: ZstdErrorCode::FrameIndexTooLarge,
        linux_case: "PREFIX(frameIndex_tooLarge)",
        message: "Frame index is too large",
        stable: false,
    },
    ZstdErrorString {
        code: ZstdErrorCode::SeekableIo,
        linux_case: "PREFIX(seekableIO)",
        message: "An I/O error occurred when reading/seeking",
        stable: false,
    },
    ZstdErrorString {
        code: ZstdErrorCode::DstBufferWrong,
        linux_case: "PREFIX(dstBuffer_wrong)",
        message: "Destination buffer is wrong",
        stable: false,
    },
    ZstdErrorString {
        code: ZstdErrorCode::SrcBufferWrong,
        linux_case: "PREFIX(srcBuffer_wrong)",
        message: "Source buffer is wrong",
        stable: false,
    },
    ZstdErrorString {
        code: ZstdErrorCode::SequenceProducerFailed,
        linux_case: "PREFIX(sequenceProducer_failed)",
        message: "Block-level external sequence producer returned an error code",
        stable: false,
    },
    ZstdErrorString {
        code: ZstdErrorCode::ExternalSequencesInvalid,
        linux_case: "PREFIX(externalSequences_invalid)",
        message: "External sequences are not valid",
        stable: false,
    },
];

pub fn err_get_error_string(code: ZstdErrorCode, strip_error_strings: bool) -> &'static str {
    if strip_error_strings {
        STRIPPED_ERROR_STRING
    } else {
        ERR_ERROR_STRINGS
            .iter()
            .find(|entry| entry.code == code)
            .map(|entry| entry.message)
            .unwrap_or(NOT_ERROR_CODE)
    }
}

pub fn err_error_string_entry(code: ZstdErrorCode) -> Option<&'static ZstdErrorString> {
    ERR_ERROR_STRINGS.iter().find(|entry| entry.code == code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_private_strings_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/zstd/common/error_private.c"
        ));
        assert!(source.contains("const char* ERR_getErrorString(ERR_enum code)"));
        assert!(source.contains("single list of error strings embedded in binary"));
        assert!(source.contains("return \"Error strings stripped\";"));
        assert!(
            source.contains("static const char* const notErrorCode = \"Unspecified error code\";")
        );
        assert!(source.contains("case PREFIX(no_error): return \"No error detected\";"));
        assert!(source.contains("case PREFIX(GENERIC):  return \"Error (generic)\";"));
        assert!(
            source.contains("case PREFIX(prefix_unknown): return \"Unknown frame descriptor\";")
        );
        assert!(
            source.contains("case PREFIX(version_unsupported): return \"Version not supported\";")
        );
        assert!(
            source
                .contains("case PREFIX(dictionary_corrupted): return \"Dictionary is corrupted\";")
        );
        assert!(source.contains(
            "case PREFIX(memory_allocation): return \"Allocation error : not enough memory\";"
        ));
        assert!(source.contains("following error codes are not stable"));
        assert!(source.contains("case PREFIX(sequenceProducer_failed): return \"Block-level external sequence producer returned an error code\";"));
        assert!(source.contains("case PREFIX(maxCode):"));
        assert!(source.contains("default: return notErrorCode;"));

        assert_eq!(ERR_ERROR_STRINGS.len(), 35);
        for entry in ERR_ERROR_STRINGS {
            assert!(
                source.contains(entry.linux_case),
                "missing {}",
                entry.linux_case
            );
            assert!(source.contains(entry.message), "missing {}", entry.message);
        }
        assert_eq!(
            ERR_ERROR_STRINGS
                .iter()
                .filter(|entry| entry.stable)
                .count(),
            29
        );
        assert_eq!(
            ERR_ERROR_STRINGS
                .iter()
                .filter(|entry| !entry.stable)
                .map(|entry| entry.code as u32)
                .collect::<alloc::vec::Vec<_>>(),
            alloc::vec![100, 102, 104, 105, 106, 107]
        );

        assert_eq!(
            err_get_error_string(ZstdErrorCode::NoError, false),
            "No error detected"
        );
        assert_eq!(
            err_get_error_string(ZstdErrorCode::MemoryAllocation, false),
            "Allocation error : not enough memory"
        );
        assert_eq!(
            err_get_error_string(ZstdErrorCode::MaxCode, false),
            NOT_ERROR_CODE
        );
        assert_eq!(
            err_get_error_string(ZstdErrorCode::Generic, true),
            STRIPPED_ERROR_STRING
        );
        assert_eq!(
            err_error_string_entry(ZstdErrorCode::ExternalSequencesInvalid)
                .map(|entry| entry.message),
            Some("External sequences are not valid")
        );
    }
}
