use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
    server,
};
use x509_parser::{asn1_rs::FromDer, certificate::X509Certificate, nom::AsBytes, time::ASN1Time};

use crate::{config, outputln};

fn load_cert<'a>(path: &str) -> anyhow::Result<CertificateDer<'a>> {
    let content = std::fs::read_to_string(path)?;
    Ok(CertificateDer::from_pem_slice(content.as_bytes())?)
}

fn load_private_key<'a>(path: &str) -> anyhow::Result<PrivateKeyDer<'a>> {
    let content = std::fs::read_to_string(path)?;
    Ok(PrivateKeyDer::from_pem_slice(content.as_bytes())?)
}

fn calculate_certificate_expiration(cert: &[u8]) -> anyhow::Result<Option<i64>> {
    let (_, cert) = X509Certificate::from_der(cert)?;
    let not_after = cert.validity().not_after;
    let now = ASN1Time::now();

    if not_after < now {
        return Ok(None);
    }

    Ok(Some(not_after.timestamp()))
}
pub struct TlsServerCerts {
    pub not_after: i64,
    pub config: server::ServerConfig,
}

pub enum BuidlTlsServerResult {
    Expired,
    Data(Box<TlsServerCerts>),
}

pub async fn build_tls_server() -> anyhow::Result<BuidlTlsServerResult> {
    let config = config::get();

    let Some(ref tls_config) = config.server.tls else {
        anyhow::bail!("build_tls_server called when no tls configuration is available");
    };

    let cert_filename = &tls_config.cert_filename;
    let key_filename = &tls_config.key_filename;

    outputln!("loading TLS certificate and key", cert_filename: cert_filename, key_filename: key_filename);

    let cert_pem = load_cert(cert_filename)?;
    let key_pem = load_private_key(key_filename)?;

    let Some(expiration) = calculate_certificate_expiration(cert_pem.as_bytes())? else {
        return Ok(BuidlTlsServerResult::Expired);
    };

    if expiration < 0 {
        return Err(anyhow::anyhow!("TLS certificate has already expired"));
    }

    let server_config = server::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_pem], key_pem)?;

    Ok(BuidlTlsServerResult::Data(Box::new(TlsServerCerts {
        not_after: expiration,
        config: server_config,
    })))
}

pub async fn renew_spin() -> TlsServerCerts {
    loop {
        match build_tls_server().await {
            Ok(BuidlTlsServerResult::Expired) => {
                outputln!("TLS certificate expired, retry in 10 seconds");
            }
            Ok(BuidlTlsServerResult::Data(tls_server_certs)) => {
                outputln!("TLS certificate pulled");

                return TlsServerCerts {
                    not_after: tls_server_certs.not_after,
                    config: tls_server_certs.config,
                };
            }
            Err(err) => {
                outputln!("TLS certificate renewer failed, retry in 10 seconds", error: err.to_string());
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}
