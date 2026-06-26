//! linux-parity: complete
//! linux-source: vendor/linux/certs/extract-cert.c
//! test-origin: linux:vendor/linux/certs/extract-cert.c
//! Build-time X.509 extraction source selection.

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CertSourceKind {
    EmptyInput,
    Pkcs11,
    PemFile,
}

pub const USAGE: &str = "Usage: extract-cert <source> <dest>";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Pkcs11Mode {
    Provider,
    Engine,
    Unavailable,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExtractCertEnv {
    pub kbuild_verbose: Option<String>,
    pub kbuild_sign_pin: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct X509Certificate {
    pub subject: String,
    pub der: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExtractCertAction {
    OpenSslInit,
    FormatUsage,
    CreateEmptyOutput { dest: String },
    OpenOutput { dest: String },
    WriteDer { subject: String, len: usize },
    VerboseExtracted { subject: String },
    ProviderTryLoad { name: &'static str },
    StoreOpen { uri: String },
    StoreLoadLoop,
    EngineLoadBuiltin,
    EngineById { id: &'static str },
    EngineInit,
    EngineSetPin,
    EngineLoadCertCtrl,
    OpenPemInput { source: String },
    PemReadLoop,
    BioFreeOutput,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExtractCertError {
    Usage { code: i32 },
    NoPkcs11Support { code: i32 },
    Pkcs11LoadFailed,
    PemReadFailed { source: String },
    OutputOpenFailed { dest: String },
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExtractCertRun {
    pub actions: Vec<ExtractCertAction>,
    pub der_writes: Vec<Vec<u8>>,
}

pub fn classify_cert_source(source: &str) -> CertSourceKind {
    if source.is_empty() {
        CertSourceKind::EmptyInput
    } else if source.starts_with("pkcs11:") {
        CertSourceKind::Pkcs11
    } else {
        CertSourceKind::PemFile
    }
}

pub fn kbuild_verbose_enabled(value: Option<&str>) -> bool {
    value.is_some_and(|v| v.as_bytes().iter().any(|b| *b == b'1'))
}

pub fn format_usage_action() -> ExtractCertError {
    ExtractCertError::Usage { code: 2 }
}

pub fn write_cert(
    run: &mut ExtractCertRun,
    dest: &str,
    verbose: bool,
    output_open: &mut bool,
    cert: &X509Certificate,
) -> Result<(), ExtractCertError> {
    if dest.is_empty() {
        return Err(ExtractCertError::OutputOpenFailed {
            dest: dest.to_string(),
        });
    }
    if !*output_open {
        run.actions.push(ExtractCertAction::OpenOutput {
            dest: dest.to_string(),
        });
        *output_open = true;
    }
    run.actions.push(ExtractCertAction::WriteDer {
        subject: cert.subject.clone(),
        len: cert.der.len(),
    });
    run.der_writes.push(cert.der.clone());
    if verbose {
        run.actions.push(ExtractCertAction::VerboseExtracted {
            subject: cert.subject.clone(),
        });
    }
    Ok(())
}

pub fn load_cert_pkcs11_plan(
    run: &mut ExtractCertRun,
    cert_src: &str,
    mode: Pkcs11Mode,
    key_pass: Option<&str>,
) -> Result<(), ExtractCertError> {
    match mode {
        Pkcs11Mode::Provider => {
            run.actions
                .push(ExtractCertAction::ProviderTryLoad { name: "pkcs11" });
            run.actions
                .push(ExtractCertAction::ProviderTryLoad { name: "default" });
            run.actions.push(ExtractCertAction::StoreOpen {
                uri: cert_src.to_string(),
            });
            run.actions.push(ExtractCertAction::StoreLoadLoop);
            Ok(())
        }
        Pkcs11Mode::Engine => {
            run.actions.push(ExtractCertAction::EngineLoadBuiltin);
            run.actions
                .push(ExtractCertAction::EngineById { id: "pkcs11" });
            run.actions.push(ExtractCertAction::EngineInit);
            if key_pass.is_some() {
                run.actions.push(ExtractCertAction::EngineSetPin);
            }
            run.actions.push(ExtractCertAction::EngineLoadCertCtrl);
            Ok(())
        }
        Pkcs11Mode::Unavailable => Err(ExtractCertError::NoPkcs11Support { code: 1 }),
    }
}

pub fn extract_cert_main(
    args: &[&str],
    env: &ExtractCertEnv,
    pkcs11_mode: Pkcs11Mode,
    pkcs11_cert: Option<X509Certificate>,
    pem_certs: &[X509Certificate],
) -> Result<ExtractCertRun, ExtractCertError> {
    let mut run = ExtractCertRun::default();
    run.actions.push(ExtractCertAction::OpenSslInit);

    let verbose = kbuild_verbose_enabled(env.kbuild_verbose.as_deref());
    if args.len() != 3 {
        run.actions.push(ExtractCertAction::FormatUsage);
        return Err(format_usage_action());
    }

    let cert_src = args[1];
    let cert_dst = args[2];
    match classify_cert_source(cert_src) {
        CertSourceKind::EmptyInput => {
            run.actions.push(ExtractCertAction::CreateEmptyOutput {
                dest: cert_dst.to_string(),
            });
            Ok(run)
        }
        CertSourceKind::Pkcs11 => {
            load_cert_pkcs11_plan(
                &mut run,
                cert_src,
                pkcs11_mode,
                env.kbuild_sign_pin.as_deref(),
            )?;
            let cert = pkcs11_cert.ok_or(ExtractCertError::Pkcs11LoadFailed)?;
            let mut output_open = false;
            write_cert(&mut run, cert_dst, verbose, &mut output_open, &cert)?;
            run.actions.push(ExtractCertAction::BioFreeOutput);
            Ok(run)
        }
        CertSourceKind::PemFile => {
            run.actions.push(ExtractCertAction::OpenPemInput {
                source: cert_src.to_string(),
            });
            run.actions.push(ExtractCertAction::PemReadLoop);
            if pem_certs.is_empty() {
                return Err(ExtractCertError::PemReadFailed {
                    source: cert_src.to_string(),
                });
            }
            let mut output_open = false;
            for cert in pem_certs {
                write_cert(&mut run, cert_dst, verbose, &mut output_open, cert)?;
            }
            run.actions.push(ExtractCertAction::BioFreeOutput);
            Ok(run)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn extract_cert_flow_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/certs/extract-cert.c"
        ));
        assert!(source.contains("Usage: extract-cert <source> <dest>"));
        assert!(source.contains("verbose_env = getenv(\"KBUILD_VERBOSE\")"));
        assert!(source.contains("strchr(verbose_env, '1')"));
        assert!(source.contains("key_pass = getenv(\"KBUILD_SIGN_PIN\")"));
        assert!(source.contains("if (argc != 3)"));
        assert!(source.contains("if (!cert_src[0])"));
        assert!(source.contains("!strncmp(cert_src, \"pkcs11:\", 7)"));
        assert!(source.contains("OSSL_PROVIDER_try_load(NULL, \"pkcs11\", true)"));
        assert!(source.contains("OSSL_STORE_open(cert_src, NULL, NULL, NULL, NULL)"));
        assert!(source.contains("ENGINE_ctrl_cmd_string(e, \"PIN\", key_pass, 0)"));
        assert!(source.contains("ENGINE_ctrl_cmd(e, \"LOAD_CERT_CTRL\""));
        assert!(source.contains("PEM_read_bio_X509"));
        assert!(source.contains("ERR_GET_REASON(err) == PEM_R_NO_START_LINE"));
        assert!(source.contains("i2d_X509_bio(wb, x509)"));
        assert!(source.contains("BIO_free(wb);"));

        assert_eq!(USAGE, "Usage: extract-cert <source> <dest>");
        assert_eq!(classify_cert_source(""), CertSourceKind::EmptyInput);
        assert_eq!(
            classify_cert_source("pkcs11:id=%01"),
            CertSourceKind::Pkcs11
        );
        assert_eq!(classify_cert_source("cert.pem"), CertSourceKind::PemFile);
        assert!(kbuild_verbose_enabled(Some("1")));
        assert!(kbuild_verbose_enabled(Some("V=1")));
        assert!(!kbuild_verbose_enabled(Some("0")));
        assert!(!kbuild_verbose_enabled(None));
    }

    #[test]
    fn empty_input_creates_empty_destination_and_exits_zero() {
        let run = extract_cert_main(
            &["extract-cert", "", "out.der"],
            &ExtractCertEnv::default(),
            Pkcs11Mode::Unavailable,
            None,
            &[],
        )
        .unwrap();
        assert_eq!(
            run.actions,
            [
                ExtractCertAction::OpenSslInit,
                ExtractCertAction::CreateEmptyOutput {
                    dest: "out.der".to_string()
                },
            ]
        );
        assert!(run.der_writes.is_empty());
    }

    #[test]
    fn pkcs11_provider_path_loads_store_and_writes_one_cert() {
        let cert = X509Certificate {
            subject: "/CN=test".to_string(),
            der: vec![1, 2, 3],
        };
        let env = ExtractCertEnv {
            kbuild_verbose: Some("1".to_string()),
            kbuild_sign_pin: None,
        };
        let run = extract_cert_main(
            &["extract-cert", "pkcs11:id=%01", "out.der"],
            &env,
            Pkcs11Mode::Provider,
            Some(cert),
            &[],
        )
        .unwrap();
        assert!(
            run.actions
                .contains(&ExtractCertAction::ProviderTryLoad { name: "pkcs11" })
        );
        assert!(
            run.actions
                .contains(&ExtractCertAction::ProviderTryLoad { name: "default" })
        );
        assert!(run.actions.contains(&ExtractCertAction::StoreOpen {
            uri: "pkcs11:id=%01".to_string()
        }));
        assert!(run.actions.contains(&ExtractCertAction::VerboseExtracted {
            subject: "/CN=test".to_string()
        }));
        assert_eq!(run.der_writes, [vec![1, 2, 3]]);
    }

    #[test]
    fn pkcs11_engine_path_sets_pin_when_present() {
        let cert = X509Certificate {
            subject: "/CN=engine".to_string(),
            der: vec![9],
        };
        let env = ExtractCertEnv {
            kbuild_verbose: None,
            kbuild_sign_pin: Some("1234".to_string()),
        };
        let run = extract_cert_main(
            &["extract-cert", "pkcs11:token=kernel", "out.der"],
            &env,
            Pkcs11Mode::Engine,
            Some(cert),
            &[],
        )
        .unwrap();
        assert!(run.actions.contains(&ExtractCertAction::EngineLoadBuiltin));
        assert!(
            run.actions
                .contains(&ExtractCertAction::EngineById { id: "pkcs11" })
        );
        assert!(run.actions.contains(&ExtractCertAction::EngineSetPin));
        assert!(run.actions.contains(&ExtractCertAction::EngineLoadCertCtrl));
    }

    #[test]
    fn pem_path_writes_all_certificates_and_rejects_empty_pem() {
        let certs = [
            X509Certificate {
                subject: "/CN=one".to_string(),
                der: vec![1],
            },
            X509Certificate {
                subject: "/CN=two".to_string(),
                der: vec![2, 2],
            },
        ];
        let run = extract_cert_main(
            &["extract-cert", "certs.pem", "out.der"],
            &ExtractCertEnv::default(),
            Pkcs11Mode::Unavailable,
            None,
            &certs,
        )
        .unwrap();
        assert!(run.actions.contains(&ExtractCertAction::OpenPemInput {
            source: "certs.pem".to_string()
        }));
        assert_eq!(run.der_writes, [vec![1], vec![2, 2]]);

        assert_eq!(
            extract_cert_main(
                &["extract-cert", "empty.pem", "out.der"],
                &ExtractCertEnv::default(),
                Pkcs11Mode::Unavailable,
                None,
                &[],
            ),
            Err(ExtractCertError::PemReadFailed {
                source: "empty.pem".to_string()
            })
        );
    }

    #[test]
    fn usage_and_missing_pkcs11_support_match_exit_paths() {
        assert_eq!(
            extract_cert_main(
                &["extract-cert"],
                &ExtractCertEnv::default(),
                Pkcs11Mode::Unavailable,
                None,
                &[],
            ),
            Err(ExtractCertError::Usage { code: 2 })
        );
        assert_eq!(
            extract_cert_main(
                &["extract-cert", "pkcs11:id=1", "out.der"],
                &ExtractCertEnv::default(),
                Pkcs11Mode::Unavailable,
                None,
                &[],
            ),
            Err(ExtractCertError::NoPkcs11Support { code: 1 })
        );
    }
}
