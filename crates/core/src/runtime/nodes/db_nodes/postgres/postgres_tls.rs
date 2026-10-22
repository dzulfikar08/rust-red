//! TLS connector for the PostgreSQL pool (`ssl` of the config node).

use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, ring};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme};
use tokio_postgres_rustls::MakeRustlsConnect;

/// Accepts any server certificate. Used only for `ssl: {"rejectUnauthorized": false}`,
/// the same opt-out node-postgres offers for self-signed servers.
#[derive(Debug)]
struct AcceptAnyCertificate(Arc<CryptoProvider>);

impl ServerCertVerifier for AcceptAnyCertificate {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

/// Build a TLS connector. With `accept_invalid_certs` the server certificate is not verified.
pub(super) fn connector(accept_invalid_certs: bool) -> crate::Result<MakeRustlsConnect> {
    let provider = Arc::new(ring::default_provider());
    let builder = ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .map_err(|e| anyhow::anyhow!("TLS configuration error: {e}"))?;

    let config = if accept_invalid_certs {
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAnyCertificate(provider)))
            .with_no_client_auth()
    } else {
        let mut roots = RootCertStore::empty();
        let native = rustls_native_certs::load_native_certs();
        for error in &native.errors {
            log::warn!("Skipping a system root certificate: {error}");
        }
        let (added, ignored) = roots.add_parsable_certificates(native.certs);
        if added == 0 {
            return Err(anyhow::anyhow!(
                "No usable system root certificates found for PostgreSQL TLS ({ignored} ignored)"
            ));
        }
        builder.with_root_certificates(roots).with_no_client_auth()
    };

    Ok(MakeRustlsConnect::new(config))
}
